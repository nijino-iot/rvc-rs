use log::info;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

// 引入 rvc_core 库
use rvc_core::{
    config::{Config, ConfigManager},
    gui::{AppState, AudioDeviceInfo, GuiManager},
};

/// 应用状态管理 - 仅包含对 core 的引用
struct AppStateManager {
    /// GUI 管理器
    gui: Arc<Mutex<GuiManager>>,
    /// 配置管理器
    config: Arc<Mutex<ConfigManager>>,
}

use directories::ProjectDirs;

fn get_config_path() -> PathBuf {
    // org, app name, subproject name（任意写）
    let proj_dirs = ProjectDirs::from("me", "kigis", "rvc-ui").expect("无法获取系统配置路径");
    proj_dirs.config_dir().join("config.json")
}

impl AppStateManager {
    pub fn new() -> Self {
        let config_path = get_config_path();
        let config = Arc::new(Mutex::new(ConfigManager::new(config_path.clone())));
        let gui = Arc::new(Mutex::new(
            GuiManager::new(config_path).expect("Failed to create GUI manager"),
        ));

        Self { gui, config }
    }
}

// ========== 配置管理相关命令 ==========
#[tauri::command]
async fn load_config(state: State<'_, AppStateManager>) -> Result<Config, String> {
    info!("加载配置文件");
    let mut config = state.config.lock().await;
    config.load().map_err(|e| e.to_string())?;
    Ok(config.config().clone())
}

#[tauri::command]
async fn save_config(data: Config, state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("保存配置文件 {:?}", data);
    let mut config = state.config.lock().await;
    config
        .update_config(|c| {
            *c = data;
        })
        .map_err(|e| e.to_string())
}

// ========== 设备管理相关命令 ==========

#[tauri::command]
async fn list_host_apis(state: State<'_, AppStateManager>) -> Result<Vec<String>, String> {
    info!("列出音频主机API");
    let gui = state.gui.lock().await;
    Ok(gui.get_hostapis())
}

#[tauri::command]
async fn list_input_devices(
    state: State<'_, AppStateManager>,
    host_api: Option<String>,
) -> Result<Vec<AudioDeviceInfo>, String> {
    info!("列出输入设备: {:?}", host_api);
    let gui = state.gui.lock().await;
    Ok(gui.get_input_devices(host_api.as_deref()))
}

#[tauri::command]
async fn list_output_devices(
    state: State<'_, AppStateManager>,
    host_api: Option<String>,
) -> Result<Vec<AudioDeviceInfo>, String> {
    info!("列出输出设备: {:?}", host_api);
    let gui = state.gui.lock().await;
    Ok(gui.get_output_devices(host_api.as_deref()))
}

#[tauri::command]
async fn reload_devices(
    state: State<'_, AppStateManager>,
    host_api: Option<&str>,
) -> Result<(), String> {
    info!("重新加载设备列表");
    let mut gui = state.gui.lock().await;
    gui.update_audio_devices(host_api)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_device_samplerate(
    state: State<'_, AppStateManager>,
    device_name: String,
) -> Result<u32, String> {
    let gui = state.gui.lock().await;
    gui.get_device_sample_rate(&device_name)
        .map(|rate| rate as u32)
        .ok_or_else(|| "设备未找到".to_string())
}

#[tauri::command]
async fn get_device_channels(
    state: State<'_, AppStateManager>,
    device_name: String,
    is_input: bool,
) -> Result<u32, String> {
    let gui = state.gui.lock().await;
    gui.get_device_channels(&device_name, is_input)
        .map(|channels| channels as u32)
        .ok_or_else(|| "设备未找到".to_string())
}

// ========== 语音转换控制命令 ==========

#[tauri::command]
async fn start_voice_conversion(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("开始语音转换");
    let mut gui = state.gui.lock().await;
    gui.validate_config().map_err(|e| e.to_string())?;
    gui.start_voice_conversion()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn stop_voice_conversion(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("停止语音转换");
    let mut gui = state.gui.lock().await;
    gui.stop_voice_conversion().await.map_err(|e| e.to_string())
}

// ========== 参数更新命令 ==========

#[tauri::command]
async fn update_parameter(
    name: String,
    value: serde_json::Value,
    state: State<'_, AppStateManager>,
) -> Result<(), String> {
    info!("更新参数: {} = {:?}", name, value);

    // 尝试实时参数更新
    {
        let mut gui = state.gui.lock().await;
        match gui.update_realtime_parameter(&name, value.clone()) {
            Ok(()) => return Ok(()),
            Err(_) => {
                // 如果实时更新失败，继续使用配置管理器更新
            }
        }
    }

    // 配置文件更新
    let mut config = state.config.lock().await;
    config
        .update_config(|config| {
            match name.as_str() {
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
                        config.set_f0method(v.to_string());
                    }
                }
                "sr_type" => {
                    if let Some(v) = value.as_str() {
                        config.set_sr_type(v.to_string());
                    }
                }
                _ => {
                    // 处理其他参数
                    match name.as_str() {
                        "block_time" => {
                            if let Some(v) = value.as_f64() {
                                config.block_time = v as f32;
                            }
                        }
                        "crossfade_time" => {
                            if let Some(v) = value.as_f64() {
                                config.crossfade_length = v as f32;
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
                    }
                }
            }
        })
        .map_err(|e| e.to_string())
}

// ========== 状态查询命令 ==========

#[tauri::command]
async fn get_realtime_status(
    state: State<'_, AppStateManager>,
) -> Result<HashMap<String, serde_json::Value>, String> {
    let gui = state.gui.lock().await;
    let stats = gui.get_stats();
    let app_state = gui.get_state();

    let mut status = HashMap::new();

    // 添加状态信息
    status.insert(
        "app_state".to_string(),
        serde_json::json!(format!("{:?}", app_state)),
    );
    status.insert(
        "is_converting".to_string(),
        serde_json::json!(matches!(app_state, AppState::Converting)),
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
    {
        let mut gui = state.gui.lock().await;
        gui.initialize().await.map_err(|e| e.to_string())?;
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
