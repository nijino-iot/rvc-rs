use log::info;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

// 引入 rvc_core 库
use rvc_core::{
    config::{Config, ConfigManager},
    events::{AppEvent, AppState, EventManager, EventPublisher, EventSubscriber, RuntimeStats},
    sd,
};

/// 轻量级应用状态管理 - 避免线程安全问题
#[derive(Debug)]
struct AppStateManager {
    /// 配置管理器
    config: Arc<Mutex<ConfigManager>>,
    /// 运行时状态
    app_state: Arc<Mutex<AppState>>,
    /// 运行时统计
    stats: Arc<Mutex<RuntimeStats>>,
    /// 事件发布器
    event_publisher: Arc<Mutex<Option<EventPublisher>>>,
}

use directories::ProjectDirs;

fn get_config_path() -> PathBuf {
    let proj_dirs = ProjectDirs::from("me", "kigis", "rvc-ui").expect("无法获取系统配置路径");
    proj_dirs.config_dir().join("config.json")
}

impl AppStateManager {
    pub fn new() -> Self {
        let config_path = get_config_path();
        info!("配置文件 {}", config_path.display());
        let config = Arc::new(Mutex::new(ConfigManager::new(config_path)));
        let app_state = Arc::new(Mutex::new(AppState::Initializing));
        let stats = Arc::new(Mutex::new(RuntimeStats::default()));

        Self {
            config,
            app_state,
            stats,
            event_publisher: Arc::new(Mutex::new(None)),
        }
    }

    /// 初始化事件系统
    pub async fn initialize_events(&self) -> Result<EventManager, String> {
        let event_manager = EventManager::new(1000);
        let publisher = event_manager.publisher();
        *self.event_publisher.lock().await = Some(publisher);
        Ok(event_manager)
    }

    /// 启动事件监听器
    pub async fn start_event_listener(
        &self,
        app_handle: AppHandle,
        mut subscriber: EventSubscriber,
    ) -> Result<(), String> {
        let state_manager = self.clone();
        tokio::spawn(async move {
            loop {
                match subscriber.recv().await {
                    Ok(event) => {
                        if let Err(e) = state_manager.handle_core_event(event, &app_handle).await {
                            log::error!("处理核心事件失败: {}", e);
                        }
                    }
                    Err(e) => {
                        log::error!("事件接收失败: {}", e);
                        break;
                    }
                }
            }
        });
        Ok(())
    }

    /// 处理来自核心的事件
    async fn handle_core_event(
        &self,
        event: AppEvent,
        app_handle: &AppHandle,
    ) -> Result<(), String> {
        match event {
            AppEvent::StateChanged { new_state, .. } => {
                *self.app_state.lock().await = new_state.clone();
                app_handle
                    .emit("state-changed", &new_state)
                    .map_err(|e| e.to_string())?;
            }
            AppEvent::StatsUpdated { stats } => {
                *self.stats.lock().await = stats.clone();
                app_handle
                    .emit("stats-updated", &stats)
                    .map_err(|e| e.to_string())?;
            }
            AppEvent::DevicesUpdated {
                input_devices,
                output_devices,
            } => {
                let device_info = serde_json::json!({
                    "input_devices": input_devices,
                    "output_devices": output_devices
                });
                app_handle
                    .emit("devices-updated", device_info)
                    .map_err(|e| e.to_string())?;
            }
            AppEvent::ConfigUpdated { config } => {
                app_handle
                    .emit("config-updated", config)
                    .map_err(|e| e.to_string())?;
            }
            AppEvent::AudioProcessing {
                delay_time,
                inference_time,
                buffer_usage,
            } => {
                let audio_status = serde_json::json!({
                    "delay_time": delay_time,
                    "inference_time": inference_time,
                    "buffer_usage": buffer_usage
                });
                app_handle
                    .emit("audio-processing", audio_status)
                    .map_err(|e| e.to_string())?;
            }
            AppEvent::Error {
                message,
                error_type,
            } => {
                let error_info = serde_json::json!({
                    "message": message,
                    "error_type": error_type
                });
                app_handle
                    .emit("error", error_info)
                    .map_err(|e| e.to_string())?;
            }
            AppEvent::Log {
                level,
                message,
                timestamp,
            } => {
                let log_info = serde_json::json!({
                    "level": level,
                    "message": message,
                    "timestamp": timestamp
                });
                app_handle
                    .emit("log", log_info)
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}

impl Clone for AppStateManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            app_state: self.app_state.clone(),
            stats: self.stats.clone(),
            event_publisher: self.event_publisher.clone(),
        }
    }
}

// ========== 配置管理相关命令 ==========
#[tauri::command]
async fn load_config(state: State<'_, AppStateManager>) -> Result<Config, String> {
    info!("加载配置文件");
    let mut config_manager = state.config.lock().await;
    config_manager.load().map_err(|e| e.to_string())?;
    Ok(config_manager.config().clone())
}

#[tauri::command]
async fn save_config(
    data: serde_json::Value,
    state: State<'_, AppStateManager>,
) -> Result<(), String> {
    info!("保存配置文件 {:?}", data);
    let mut config_manager = state.config.lock().await;

    config_manager
        .update_gui_config(|config| {
            let errors = config.update_from_json(&data);
            if !errors.is_empty() {
                info!("配置验证警告: {:?}", errors);
            }
        })
        .map_err(|e| e.to_string())
}

// ========== 统一事件处理命令 - 模仿 gui_v1.py 的事件循环 ==========
#[tauri::command]
async fn handle_event(
    event: String,
    values: serde_json::Value,
    state: State<'_, AppStateManager>,
) -> Result<serde_json::Value, String> {
    info!("处理事件: {} 值: {:?}", event, values);

    // 模仿 Python gui_v1.py 的 event_handler 逻辑
    match event.as_str() {
        // 重载设备列表 - 对应 Python 的 "reload_devices"
        "reload_devices" => {
            let hostapis = sd::query_hostapis();
            let hostapi_names: Vec<String> = hostapis.into_iter().map(|h| h.name).collect();

            let devices = sd::query_devices();
            let input_devices: Vec<String> = devices
                .iter()
                .filter(|d| d.max_input_channels > 0)
                .map(|d| d.name.clone())
                .collect();

            let output_devices: Vec<String> = devices
                .iter()
                .filter(|d| d.max_output_channels > 0)
                .map(|d| d.name.clone())
                .collect();

            Ok(serde_json::json!({
                "hostapis": hostapi_names,
                "input_devices": input_devices,
                "output_devices": output_devices
            }))
        }

        // 主机API变更 - 对应 Python 的 "sg_hostapi"
        "sg_hostapi" => {
            let host_api = values.get("sg_hostapi").and_then(|v| v.as_str());

            let hostapis = sd::query_hostapis();
            let hostapi_names: Vec<String> = hostapis.into_iter().map(|h| h.name).collect();

            let devices = sd::query_devices();
            let input_devices: Vec<String> = devices
                .iter()
                .filter(|d| d.max_input_channels > 0)
                .filter(|d| {
                    if let Some(api) = host_api {
                        d.hostapi_name.contains(api)
                    } else {
                        true
                    }
                })
                .map(|d| d.name.clone())
                .collect();

            let output_devices: Vec<String> = devices
                .iter()
                .filter(|d| d.max_output_channels > 0)
                .filter(|d| {
                    if let Some(api) = host_api {
                        d.hostapi_name.contains(api)
                    } else {
                        true
                    }
                })
                .map(|d| d.name.clone())
                .collect();

            // 保存配置变更
            let mut config_manager = state.config.lock().await;
            config_manager
                .update_gui_config(|config| {
                    if let Some(api) = host_api {
                        config.sg_hostapi = Some(api.to_string());
                    }
                })
                .map_err(|e| e.to_string())?;

            Ok(serde_json::json!({
                "hostapis": hostapi_names,
                "input_devices": input_devices,
                "output_devices": output_devices
            }))
        }

        // 开始语音转换 - 对应 Python 的点击开始按钮
        "start_vc" => {
            // 验证必需参数
            let pth_path = values
                .get("pth_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let index_path = values
                .get("index_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            if pth_path.is_empty() {
                return Err("请选择pth文件".to_string());
            }
            if index_path.is_empty() {
                return Err("请选择index文件".to_string());
            }

            // 检查路径中是否包含中文
            let pattern = regex::Regex::new(r"[^\x00-\x7F]+").unwrap();
            if pattern.is_match(pth_path) {
                return Err("pth文件路径不可包含中文".to_string());
            }
            if pattern.is_match(index_path) {
                return Err("index文件路径不可包含中文".to_string());
            }

            // 更新配置 - 对应 Python 的配置保存
            let mut config_manager = state.config.lock().await;
            config_manager
                .update_gui_config(|config| {
                    let errors = config.update_from_json(&values);
                    if !errors.is_empty() {
                        info!("配置验证警告: {:?}", errors);
                    }
                })
                .map_err(|e| e.to_string())?;

            // 更新状态为转换中
            {
                let mut app_state = state.app_state.lock().await;
                *app_state = AppState::Converting;
            }

            // 计算采样率
            let sr_type = values
                .get("sr_type")
                .and_then(|v| v.as_str())
                .unwrap_or("sr_model");
            let samplerate = if sr_type == "sr_device" {
                let device_name = values
                    .get("sg_output_device")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                sd::get_device_default_sample_rate(device_name, false).unwrap_or(48000.0) as u32
            } else {
                48000 // 默认模型采样率
            };

            Ok(serde_json::json!({
                "success": true,
                "samplerate": samplerate,
                "delay_time": 100
            }))
        }

        // 停止语音转换 - 对应 Python 的停止按钮
        "stop_vc" => {
            let mut app_state = state.app_state.lock().await;
            *app_state = AppState::Stopped;
            Ok(serde_json::json!({"success": true}))
        }

        // 输入/输出设备变更 - 对应 Python 的设备选择
        "sg_input_device" | "sg_output_device" => {
            let output_device = values
                .get("sg_output_device")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            // 更新配置
            let mut config_manager = state.config.lock().await;
            config_manager
                .update_gui_config(|config| {
                    if let Some(input_device) =
                        values.get("sg_input_device").and_then(|v| v.as_str())
                    {
                        config.sg_input_device = Some(input_device.to_string());
                    }
                    if let Some(output_device) =
                        values.get("sg_output_device").and_then(|v| v.as_str())
                    {
                        config.sg_output_device = Some(output_device.to_string());
                    }
                })
                .map_err(|e| e.to_string())?;

            // 获取采样率信息
            let samplerate = if !output_device.is_empty() {
                sd::get_device_default_sample_rate(output_device, false).unwrap_or(48000.0) as u32
            } else {
                48000
            };

            Ok(serde_json::json!({
                "success": true,
                "samplerate": samplerate
            }))
        }

        // 参数热更新 - 对应 Python 的滑块/复选框变更
        "threshold" | "pitch" | "formant" | "index_rate" | "rms_mix_rate" | "i_noise_reduce"
        | "o_noise_reduce" | "use_pv" => {
            // 更新配置中的对应参数
            let mut config_manager = state.config.lock().await;
            config_manager
                .update_gui_config(|config| {
                    let mut update_data = serde_json::Map::new();
                    if let Some(value) = values.get(&event) {
                        update_data.insert(event.clone(), value.clone());
                    }
                    let errors = config.update_from_json(&serde_json::Value::Object(update_data));
                    if !errors.is_empty() {
                        info!("参数热更新警告: {:?}", errors);
                    }
                })
                .map_err(|e| e.to_string())?;

            Ok(serde_json::json!({"success": true}))
        }

        // F0算法选择 - 对应 Python 的单选按钮
        "pm" | "harvest" | "crepe" | "rmvpe" | "fcpe" => {
            let mut config_manager = state.config.lock().await;
            config_manager
                .update_gui_config(|config| {
                    config.f0method = Some(event.clone());
                })
                .map_err(|e| e.to_string())?;

            Ok(serde_json::json!({"success": true}))
        }

        // 采样率类型选择 - 对应 Python 的单选按钮
        "sr_model" | "sr_device" => {
            let mut config_manager = state.config.lock().await;
            config_manager
                .update_gui_config(|config| {
                    config.sr_type = Some(event.clone());
                })
                .map_err(|e| e.to_string())?;

            Ok(serde_json::json!({"success": true}))
        }

        // 独占模式切换 - 对应 Python 的复选框
        "sg_wasapi_exclusive" => {
            let exclusive = values
                .get("sg_wasapi_exclusive")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let mut config_manager = state.config.lock().await;
            config_manager
                .update_gui_config(|config| {
                    config.sg_wasapi_exclusive = Some(exclusive);
                })
                .map_err(|e| e.to_string())?;

            Ok(serde_json::json!({"success": true}))
        }

        // 其他事件 - 停止当前流
        _ => {
            // 对于未知事件，更新为停止状态（安全处理）
            let mut app_state = state.app_state.lock().await;
            if matches!(*app_state, AppState::Converting) {
                *app_state = AppState::Stopped;
            }
            Ok(serde_json::json!({"success": true}))
        }
    }
}

// ========== 设备管理相关命令 - 使用 sd 模块直接查询 ==========

#[tauri::command]
async fn list_host_apis() -> Result<Vec<String>, String> {
    info!("列出音频主机API");
    let hostapis = sd::query_hostapis();
    Ok(hostapis.into_iter().map(|h| h.name).collect())
}

#[tauri::command]
async fn list_input_devices(host_api: Option<String>) -> Result<Vec<String>, String> {
    info!("列出输入设备: {:?}", host_api);
    let devices = sd::query_devices();
    let input_devices: Vec<String> = devices
        .into_iter()
        .filter(|d| d.max_input_channels > 0)
        .filter(|d| {
            if let Some(ref api) = host_api {
                d.hostapi_name.contains(api)
            } else {
                true
            }
        })
        .map(|d| d.name)
        .collect();
    Ok(input_devices)
}

#[tauri::command]
async fn list_output_devices(host_api: Option<String>) -> Result<Vec<String>, String> {
    info!("列出输出设备: {:?}", host_api);
    let devices = sd::query_devices();
    let output_devices: Vec<String> = devices
        .into_iter()
        .filter(|d| d.max_output_channels > 0)
        .filter(|d| {
            if let Some(ref api) = host_api {
                d.hostapi_name.contains(api)
            } else {
                true
            }
        })
        .map(|d| d.name)
        .collect();
    Ok(output_devices)
}

// ========== 语音转换控制命令 - 通过状态管理 ==========

#[tauri::command]
async fn start_voice_conversion(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("开始语音转换");

    // 更新状态到转换中
    {
        let mut app_state = state.app_state.lock().await;
        *app_state = AppState::Converting;
    }

    // 通过事件发布器通知状态变化
    if let Some(publisher) = state.event_publisher.lock().await.as_ref() {
        let _ = publisher.publish(AppEvent::StateChanged {
            old_state: AppState::Ready,
            new_state: AppState::Converting,
        });
    }

    Ok(())
}

#[tauri::command]
async fn stop_voice_conversion(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("停止语音转换");

    // 更新状态到就绪
    {
        let mut app_state = state.app_state.lock().await;
        *app_state = AppState::Ready;
    }

    // 通过事件发布器通知状态变化
    if let Some(publisher) = state.event_publisher.lock().await.as_ref() {
        let _ = publisher.publish(AppEvent::StateChanged {
            old_state: AppState::Converting,
            new_state: AppState::Ready,
        });
    }

    Ok(())
}

// ========== 状态查询命令 ==========

// ========== 实时状态获取命令 - 保留用于兼容性，但推荐使用事件 ==========
#[tauri::command]
async fn get_realtime_status(
    state: State<'_, AppStateManager>,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let app_state = state.app_state.lock().await;
    let stats = state.stats.lock().await;

    let mut status = HashMap::new();

    // 添加状态信息
    status.insert(
        "app_state".to_string(),
        serde_json::json!(format!("{:?}", *app_state)),
    );
    status.insert(
        "is_converting".to_string(),
        serde_json::json!(matches!(*app_state, AppState::Converting)),
    );

    // 添加统计信息
    status.insert(
        "delay_time".to_string(),
        serde_json::json!(stats.algorithm_latency_ms as i32),
    );
    status.insert(
        "infer_time".to_string(),
        serde_json::json!(stats.inference_time_ms as i32),
    );
    status.insert(
        "buffer_usage".to_string(),
        serde_json::json!(stats.buffer_usage),
    );
    status.insert("cpu_usage".to_string(), serde_json::json!(stats.cpu_usage));

    if let Some(gpu_usage) = stats.gpu_usage {
        status.insert("gpu_usage".to_string(), serde_json::json!(gpu_usage));
    }

    // 添加警告，建议使用事件系统
    status.insert(
        "_deprecated".to_string(),
        serde_json::json!("请使用事件监听器替代轮询获取状态"),
    );

    Ok(status)
}

// ========== 应用初始化命令 ==========

#[tauri::command]
async fn initialize_app(
    state: State<'_, AppStateManager>,
    app_handle: AppHandle,
) -> Result<(), String> {
    info!("初始化应用");

    // 初始化事件系统
    let event_manager = state.initialize_events().await?;
    let subscriber = event_manager.subscribe();

    // 启动事件监听器
    state.start_event_listener(app_handle, subscriber).await?;

    // 设置初始状态为就绪
    event_manager.update_state(AppState::Ready).await;

    // 加载配置
    load_config(state).await?;

    Ok(())
}

// ========== 设备信息查询命令 - 使用 sd 模块直接查询 ==========

#[tauri::command]
async fn get_device_sample_rate(device_name: String) -> Result<Option<f64>, String> {
    Ok(sd::get_device_default_sample_rate(&device_name, false).ok())
}

#[tauri::command]
async fn get_device_channels(device_name: String, is_input: bool) -> Result<Option<u32>, String> {
    let devices = sd::query_devices();
    for device in devices {
        if device.name == device_name {
            if is_input {
                return Ok(Some(device.max_input_channels));
            } else {
                return Ok(Some(device.max_output_channels));
            }
        }
    }
    Ok(None)
}

// ========== Tauri 应用入口 ==========

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppStateManager::new())
        .invoke_handler(tauri::generate_handler![
            initialize_app,
            load_config,
            save_config,
            handle_event,
            list_host_apis,
            list_input_devices,
            list_output_devices,
            start_voice_conversion,
            stop_voice_conversion,
            get_realtime_status,
            get_device_sample_rate,
            get_device_channels
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
