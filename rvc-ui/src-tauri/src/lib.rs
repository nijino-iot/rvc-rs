use log::info;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

// 引入 rvc_core 库
use rvc_core::{
    config::{Config, ConfigManager, GuiConfig},
    gui::{AppState, AudioDeviceInfo, RuntimeStats},
    sd,
};

/// 应用状态管理 - 仅包含配置管理器，避免线程安全问题
#[derive(Debug)]
struct AppStateManager {
    /// 配置管理器
    config: Arc<Mutex<ConfigManager>>,
    /// 配置文件路径
    config_path: PathBuf,
    /// 运行时状态
    app_state: Arc<Mutex<AppState>>,
    /// 运行时统计
    stats: Arc<Mutex<RuntimeStats>>,
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
        let config = Arc::new(Mutex::new(ConfigManager::new(config_path.clone())));
        let app_state = Arc::new(Mutex::new(AppState::Initializing));
        let stats = Arc::new(Mutex::new(RuntimeStats::default()));

        Self {
            config,
            config_path,
            app_state,
            stats,
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
async fn save_config(data: Config, state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("保存配置文件 {:?}", data);
    let mut config_manager = state.config.lock().await;
    config_manager
        .update_config(|c| {
            *c = data;
        })
        .map_err(|e| e.to_string())
}

// ========== 设备管理相关命令 ==========

#[tauri::command]
async fn list_host_apis(_state: State<'_, AppStateManager>) -> Result<Vec<String>, String> {
    info!("列出音频主机API");
    let hostapis = sd::query_hostapis();
    Ok(hostapis.into_iter().map(|h| h.name).collect())
}

#[tauri::command]
async fn list_input_devices(
    _state: State<'_, AppStateManager>,
    host_api: Option<String>,
) -> Result<Vec<AudioDeviceInfo>, String> {
    info!("列出输入设备: {:?}", host_api);
    let devices = sd::query_devices();
    let input_devices: Vec<AudioDeviceInfo> = devices
        .into_iter()
        .filter(|d| d.max_input_channels > 0)
        .filter(|d| {
            if let Some(ref api) = host_api {
                d.hostapi_name.contains(api)
            } else {
                true
            }
        })
        .map(|d| AudioDeviceInfo {
            name: d.name,
            index: d.index,
            hostapi_name: d.hostapi_name,
            max_input_channels: d.max_input_channels as u16,
            max_output_channels: d.max_output_channels as u16,
            default_samplerate: d.default_samplerate,
        })
        .collect();
    Ok(input_devices)
}

#[tauri::command]
async fn list_output_devices(
    _state: State<'_, AppStateManager>,
    host_api: Option<String>,
) -> Result<Vec<AudioDeviceInfo>, String> {
    info!("列出输出设备: {:?}", host_api);
    let devices = sd::query_devices();
    let output_devices: Vec<AudioDeviceInfo> = devices
        .into_iter()
        .filter(|d| d.max_output_channels > 0)
        .filter(|d| {
            if let Some(ref api) = host_api {
                d.hostapi_name.contains(api)
            } else {
                true
            }
        })
        .map(|d| AudioDeviceInfo {
            name: d.name,
            index: d.index,
            hostapi_name: d.hostapi_name,
            max_input_channels: d.max_input_channels as u16,
            max_output_channels: d.max_output_channels as u16,
            default_samplerate: d.default_samplerate,
        })
        .collect();
    Ok(output_devices)
}

#[tauri::command]
async fn reload_devices(
    _state: State<'_, AppStateManager>,
    _host_api: Option<&str>,
) -> Result<(), String> {
    info!("重新加载设备列表");
    // 设备重新加载通过重新查询实现，无需状态管理
    Ok(())
}

#[tauri::command]
async fn get_device_samplerate(
    _state: State<'_, AppStateManager>,
    device_name: String,
) -> Result<u32, String> {
    let rate = sd::get_device_default_sample_rate(&device_name, true).map_err(|e| e.to_string())?;
    Ok(rate as u32)
}

#[tauri::command]
async fn get_device_channels(
    _state: State<'_, AppStateManager>,
    device_name: String,
    is_input: bool,
) -> Result<u32, String> {
    let channels =
        sd::get_device_max_channels(&device_name, is_input).map_err(|e| e.to_string())?;
    Ok(channels.min(2)) // 限制为最多2通道
}

// ========== 语音转换控制命令 ==========

#[tauri::command]
async fn start_voice_conversion(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("开始语音转换");

    // 更新状态为转换中
    {
        let mut app_state = state.app_state.lock().await;
        *app_state = AppState::Converting;
    }

    // 实际的转换逻辑需要在后台线程中运行
    // 这里只是示例，实际实现需要创建 GuiManager 实例并运行
    tokio::spawn(async move {
        info!("语音转换已启动");
        // 在这里执行实际的转换逻辑
    });

    Ok(())
}

#[tauri::command]
async fn stop_voice_conversion(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("停止语音转换");

    // 更新状态为停止
    {
        let mut app_state = state.app_state.lock().await;
        *app_state = AppState::Stopped;
    }

    Ok(())
}

// ========== 参数更新命令 ==========

#[tauri::command]
async fn update_parameter(
    name: String,
    value: serde_json::Value,
    state: State<'_, AppStateManager>,
) -> Result<(), String> {
    info!("更新参数: {} = {:?}", name, value);

    // 配置文件更新
    let mut config_manager = state.config.lock().await;
    config_manager
        .update_gui_config(|config| match name.as_str() {
            "pitch" => {
                if let Some(v) = value.as_i64() {
                    config.pitch = v as i32;
                }
            }
            "formant" => {
                if let Some(v) = value.as_f64() {
                    config.formant = v as f32;
                }
            }
            "index_rate" => {
                if let Some(v) = value.as_f64() {
                    config.index_rate = v as f32;
                }
            }
            "rms_mix_rate" => {
                if let Some(v) = value.as_f64() {
                    config.rms_mix_rate = v as f32;
                }
            }
            "threshold" => {
                if let Some(v) = value.as_f64() {
                    config.threshold = v as f32;
                }
            }
            "f0method" => {
                if let Some(v) = value.as_str() {
                    config.f0method = v.to_string();
                }
            }
            "sr_type" => {
                if let Some(v) = value.as_str() {
                    config.sr_type = v.to_string();
                }
            }
            "block_time" => {
                if let Some(v) = value.as_f64() {
                    config.block_time = v as f32;
                }
            }
            "crossfade_time" => {
                if let Some(v) = value.as_f64() {
                    config.crossfade_time = v as f32;
                }
            }
            "extra_time" => {
                if let Some(v) = value.as_f64() {
                    config.extra_time = v as f32;
                }
            }
            "n_cpu" => {
                if let Some(v) = value.as_i64() {
                    config.n_cpu = v as usize;
                }
            }
            "use_pv" => {
                if let Some(v) = value.as_bool() {
                    config.use_pv = v;
                }
            }
            _ => {}
        })
        .map_err(|e| e.to_string())
}

// ========== 状态查询命令 ==========

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

    Ok(status)
}

// ========== 应用初始化命令 ==========

#[tauri::command]
async fn initialize_app(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("初始化应用");

    // 更新应用状态
    {
        let mut app_state = state.app_state.lock().await;
        *app_state = AppState::Ready;
    }

    // 加载配置
    load_config(state).await?;

    Ok(())
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
            list_host_apis,
            list_input_devices,
            list_output_devices,
            reload_devices,
            get_device_samplerate,
            get_device_channels,
            start_voice_conversion,
            stop_voice_conversion,
            update_parameter,
            get_realtime_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
