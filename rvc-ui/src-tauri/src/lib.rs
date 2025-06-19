use log::{error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

// 引入 rvc_core 库
use rvc_core::{
    config::{Config, ConfigManager},
    error::RvcError,
    gui::{AppState, AudioDeviceInfo, DeviceManager, DeviceType, GuiManager},
};

/// 应用状态管理 - 仅包含对 core 的引用
pub struct AppStateManager {
    /// GUI 管理器
    gui_manager: Arc<Mutex<GuiManager>>,
    /// 配置管理器
    config_manager: Arc<Mutex<ConfigManager>>,
}

impl AppStateManager {
    pub fn new() -> Self {
        let config_path = PathBuf::from("configs/inuse/config.json");
        let config_manager = Arc::new(Mutex::new(ConfigManager::new(config_path.clone())));
        let gui_manager = Arc::new(Mutex::new(
            GuiManager::new(config_path).expect("Failed to create GUI manager"),
        ));

        Self {
            gui_manager,
            config_manager,
        }
    }
}

// ========== 配置管理相关命令 ==========

#[tauri::command]
pub fn load_config(state: State<AppStateManager>) -> Result<Config, String> {
    info!("加载配置文件");
    let mut config_manager = state.config_manager.lock().unwrap();
    config_manager.load().map_err(|e| e.to_string())?;
    Ok(config_manager.config().clone())
}

#[tauri::command]
pub fn save_config(config: Config, state: State<AppStateManager>) -> Result<(), String> {
    info!("保存配置文件");
    let mut config_manager = state.config_manager.lock().unwrap();
    config_manager
        .update_config(|c| {
            *c = config;
        })
        .map_err(|e| e.to_string())
}

// ========== 设备管理相关命令 ==========

#[tauri::command]
pub fn list_host_apis(state: State<AppStateManager>) -> Vec<String> {
    info!("列出音频主机API");
    let gui_manager = state.gui_manager.lock().unwrap();
    gui_manager.get_hostapis()
}

#[tauri::command]
pub fn list_input_devices(
    state: State<AppStateManager>,
    host_api: Option<String>,
) -> Vec<AudioDeviceInfo> {
    info!("列出输入设备: {:?}", host_api);
    let gui_manager = state.gui_manager.lock().unwrap();
    gui_manager.get_input_devices(host_api.as_deref())
}

#[tauri::command]
pub fn list_output_devices(
    state: State<AppStateManager>,
    host_api: Option<String>,
) -> Vec<AudioDeviceInfo> {
    info!("列出输出设备: {:?}", host_api);
    let gui_manager = state.gui_manager.lock().unwrap();
    gui_manager.get_output_devices(host_api.as_deref())
}

#[tauri::command]
pub async fn reload_devices(
    state: State<'_, AppStateManager>,
    host_api: Option<String>,
) -> Result<(), String> {
    info!("重新加载设备列表");
    let gui_manager = state.gui_manager.lock().unwrap();
    gui_manager
        .update_audio_devices(host_api.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_device_samplerate(
    state: State<AppStateManager>,
    device_name: String,
) -> Result<u32, String> {
    let gui_manager = state.gui_manager.lock().unwrap();
    gui_manager
        .get_device_sample_rate(&device_name)
        .map(|rate| rate as u32)
        .ok_or_else(|| "设备未找到".to_string())
}

#[tauri::command]
pub fn get_device_channels(
    state: State<AppStateManager>,
    device_name: String,
    is_input: bool,
) -> Result<u32, String> {
    let gui_manager = state.gui_manager.lock().unwrap();
    gui_manager
        .get_device_channels(&device_name, is_input)
        .map(|channels| channels as u32)
        .ok_or_else(|| "设备未找到".to_string())
}

// ========== 语音转换控制命令 ==========

#[tauri::command]
pub async fn start_voice_conversion(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("开始语音转换");

    // 先验证配置
    {
        let gui_manager = state.gui_manager.lock().unwrap();
        gui_manager.validate_config().map_err(|e| e.to_string())?;
    }

    // 开始转换
    let gui_manager = state.gui_manager.lock().unwrap();
    gui_manager
        .start_voice_conversion()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_voice_conversion(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("停止语音转换");
    let gui_manager = state.gui_manager.lock().unwrap();
    gui_manager
        .stop_voice_conversion()
        .await
        .map_err(|e| e.to_string())
}

// ========== 参数更新命令 ==========

#[tauri::command]
pub fn update_parameter(
    name: String,
    value: serde_json::Value,
    state: State<AppStateManager>,
) -> Result<(), String> {
    info!("更新参数: {} = {:?}", name, value);

    let mut config_manager = state.config_manager.lock().unwrap();
    config_manager
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
pub fn get_realtime_status(state: State<AppStateManager>) -> HashMap<String, serde_json::Value> {
    let gui_manager = state.gui_manager.lock().unwrap();
    let stats = gui_manager.get_stats();
    let state = gui_manager.get_state();

    let mut status = HashMap::new();

    // 添加状态信息
    status.insert(
        "app_state".to_string(),
        serde_json::json!(format!("{:?}", state)),
    );
    status.insert(
        "is_converting".to_string(),
        serde_json::json!(matches!(state, AppState::Converting)),
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

    status
}

// ========== 应用初始化命令 ==========

#[tauri::command]
pub async fn initialize_app(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("初始化应用");

    // 初始化 GUI 管理器
    {
        let mut gui_manager = state.gui_manager.lock().unwrap();
        gui_manager.initialize().await.map_err(|e| e.to_string())?;
    }

    // 加载配置
    load_config(state)?;

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
            get_realtime_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
