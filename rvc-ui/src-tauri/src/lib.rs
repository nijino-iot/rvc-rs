use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

// 导入 rvc-core 模块
use rvc_core::{Config, F0Method, GuiManager, RvcError};

#[derive(Debug, Serialize, Deserialize)]
pub struct VoiceConversionConfig {
    #[serde(rename = "pthPath")]
    pub pth_path: String,
    #[serde(rename = "indexPath")]
    pub index_path: String,
    #[serde(rename = "hostApi")]
    pub host_api: String,
    #[serde(rename = "wasapiExclusive")]
    pub wasapi_exclusive: bool,
    #[serde(rename = "inputDevice")]
    pub input_device: String,
    #[serde(rename = "outputDevice")]
    pub output_device: String,
    #[serde(rename = "srModel")]
    pub sr_model: bool,
    #[serde(rename = "srDevice")]
    pub sr_device: bool,
    pub threshold: i32,
    pub pitch: i32,
    pub formant: f64,
    #[serde(rename = "indexRate")]
    pub index_rate: f64,
    #[serde(rename = "rmsMixRate")]
    pub rms_mix_rate: f64,
    #[serde(rename = "f0Method")]
    pub f0_method: String,
    #[serde(rename = "blockTime")]
    pub block_time: f64,
    #[serde(rename = "nCpu")]
    pub n_cpu: i32,
    #[serde(rename = "crossfadeLength")]
    pub crossfade_length: f64,
    #[serde(rename = "extraTime")]
    pub extra_time: f64,
    #[serde(rename = "inputNoiseReduce")]
    pub input_noise_reduce: bool,
    #[serde(rename = "outputNoiseReduce")]
    pub output_noise_reduce: bool,
    #[serde(rename = "usePv")]
    pub use_pv: bool,
    #[serde(rename = "inputMonitor")]
    pub input_monitor: bool,
    #[serde(rename = "outputVoiceChange")]
    pub output_voice_change: bool,
}

impl VoiceConversionConfig {
    /// 转换为 rvc-core 的配置格式
    pub fn to_rvc_config(&self) -> Result<Config, String> {
        let f0_method = F0Method::from_str(&self.f0_method)
            .ok_or_else(|| format!("Invalid F0 method: {}", self.f0_method))?;

        Ok(Config {
            pth_path: self.pth_path.clone(),
            index_path: self.index_path.clone(),
            sg_hostapi: self.host_api.clone(),
            sg_wasapi_exclusive: self.wasapi_exclusive,
            sg_input_device: self.input_device.clone(),
            sg_output_device: self.output_device.clone(),
            sr_type: if self.sr_model {
                "sr_model"
            } else {
                "sr_device"
            }
            .to_string(),
            threshold: self.threshold,
            pitch: self.pitch,
            formant: self.formant,
            index_rate: self.index_rate,
            rms_mix_rate: self.rms_mix_rate,
            block_time: self.block_time,
            crossfade_length: self.crossfade_length,
            extra_time: self.extra_time,
            n_cpu: self.n_cpu as usize,
            f0method: self.f0_method.clone(),
            use_jit: false, // 默认关闭 JIT
            use_pv: self.use_pv,
            sr_model: self.sr_model,
            sr_device: self.sr_device,
            pm: self.f0_method == "pm",
            harvest: self.f0_method == "harvest",
            crepe: self.f0_method == "crepe",
            rmvpe: self.f0_method == "rmvpe",
            fcpe: self.f0_method == "fcpe",
        })
    }
}

#[derive(Debug, Serialize)]
pub struct AudioDevices {
    pub hostapis: Vec<String>,
    pub input_devices: Vec<String>,
    pub output_devices: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RealTimeStatus {
    #[serde(rename = "algorithmLatency")]
    pub algorithm_latency: f64,
    #[serde(rename = "inferenceTime")]
    pub inference_time: f64,
    #[serde(rename = "bufferUsage")]
    pub buffer_usage: f64,
    #[serde(rename = "cpuUsage")]
    pub cpu_usage: f64,
    #[serde(rename = "gpuUsage")]
    pub gpu_usage: Option<f64>,
    #[serde(rename = "isConverting")]
    pub is_converting: bool,
}

/// 应用状态管理器
pub struct AppStateManager {
    gui_manager: Arc<Mutex<Option<GuiManager>>>,
    config_path: PathBuf,
}

impl AppStateManager {
    pub fn new() -> Self {
        let config_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("configs")
            .join("inuse")
            .join("config.json");

        Self {
            gui_manager: Arc::new(Mutex::new(None)),
            config_path,
        }
    }

    pub async fn initialize(&self) -> Result<(), RvcError> {
        // 初始化 rvc-core
        rvc_core::init()?;

        // 创建配置目录（如果不存在）
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RvcError::io(format!("Failed to create config directory: {}", e)))?;
        }

        // 创建 GUI 管理器
        let mut gui_manager = GuiManager::new(self.config_path.clone())?;
        gui_manager.initialize().await?;

        // 存储管理器
        let mut manager_guard = self.gui_manager.lock().unwrap();
        *manager_guard = Some(gui_manager);

        info!("App state manager initialized successfully");
        Ok(())
    }

    pub fn with_gui_manager<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&GuiManager) -> R,
    {
        let manager_guard = self.gui_manager.lock().unwrap();
        if let Some(ref manager) = *manager_guard {
            Ok(f(manager))
        } else {
            Err("GUI manager not initialized".to_string())
        }
    }

    pub async fn with_gui_manager_async<F, Fut, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(Arc<Mutex<Option<GuiManager>>>) -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let manager = Arc::clone(&self.gui_manager);
        Ok(f(manager).await)
    }
}

// Tauri 命令实现

/// 初始化应用
#[tauri::command]
async fn initialize_app(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("Initializing RVC application...");

    state.initialize().await.map_err(|e| {
        error!("Failed to initialize app: {}", e);
        format!("Initialization failed: {}", e)
    })?;

    info!("RVC application initialized successfully");
    Ok(())
}

/// 获取音频设备列表
#[tauri::command]
async fn get_audio_devices(state: State<'_, AppStateManager>) -> Result<AudioDevices, String> {
    info!("Getting audio devices list");

    state.with_gui_manager(|manager| {
        let hostapis = manager.get_hostapis();
        let input_devices = manager
            .get_input_devices()
            .iter()
            .map(|device| device.name.clone())
            .collect();
        let output_devices = manager
            .get_output_devices()
            .iter()
            .map(|device| device.name.clone())
            .collect();

        AudioDevices {
            hostapis,
            input_devices,
            output_devices,
        }
    })
}

/// 重载设备列表
#[tauri::command]
async fn reload_devices(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("Reloading audio devices");

    state
        .with_gui_manager_async(|manager_arc| async move {
            let manager_guard = manager_arc.lock().unwrap();
            if let Some(ref manager) = *manager_guard {
                manager.update_audio_devices().await.map_err(|e| {
                    error!("Failed to reload devices: {}", e);
                    format!("Failed to reload devices: {}", e)
                })
            } else {
                Err("GUI manager not initialized".to_string())
            }
        })
        .await?
}

/// 选择 PTH 文件
#[tauri::command]
async fn select_pth_file() -> Result<Option<String>, String> {
    info!("Opening PTH file selection dialog");

    // TODO: 实现文件选择对话框
    // 这里使用 tauri 的文件对话框 API
    use tauri::api::dialog::FileDialogBuilder;

    // 异步文件选择暂时用同步方式模拟
    // 实际实现时需要使用 tauri 的异步文件对话框
    let weights_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("assets")
        .join("weights");

    // 模拟文件选择结果
    if weights_dir.exists() {
        Ok(Some(
            weights_dir
                .join("example.pth")
                .to_string_lossy()
                .to_string(),
        ))
    } else {
        Ok(None)
    }
}

/// 选择 Index 文件
#[tauri::command]
async fn select_index_file() -> Result<Option<String>, String> {
    info!("Opening Index file selection dialog");

    // TODO: 实现文件选择对话框
    let logs_dir = std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("logs");

    // 模拟文件选择结果
    if logs_dir.exists() {
        Ok(Some(
            logs_dir.join("example.index").to_string_lossy().to_string(),
        ))
    } else {
        Ok(None)
    }
}

/// 开始语音转换
#[tauri::command]
async fn start_voice_conversion(
    config: VoiceConversionConfig,
    state: State<'_, AppStateManager>,
) -> Result<(), String> {
    info!("Starting voice conversion with config: {:?}", config);

    // 验证配置
    if config.pth_path.is_empty() {
        warn!("PTH path is empty");
        return Err("请选择 PTH 模型文件".to_string());
    }

    // 转换配置格式
    let rvc_config = config.to_rvc_config()?;

    state
        .with_gui_manager_async(|manager_arc| async move {
            let manager_guard = manager_arc.lock().unwrap();
            if let Some(ref manager) = *manager_guard {
                // 更新配置
                drop(manager_guard); // 释放锁以避免死锁

                // 先加载模型
                let pth_path = rvc_config.pth_path.clone();
                let index_path = rvc_config.index_path.clone();
                manager
                    .load_model(pth_path, index_path)
                    .await
                    .map_err(|e| {
                        error!("Failed to load model: {}", e);
                        format!("Failed to load model: {}", e)
                    })?;

                // 等待模型加载完成
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // 开始语音转换
                manager.start_voice_conversion().await.map_err(|e| {
                    error!("Failed to start voice conversion: {}", e);
                    format!("Failed to start voice conversion: {}", e)
                })?;

                info!("Voice conversion started successfully");
                Ok(())
            } else {
                Err("GUI manager not initialized".to_string())
            }
        })
        .await
}

/// 停止语音转换
#[tauri::command]
async fn stop_voice_conversion(state: State<'_, AppStateManager>) -> Result<(), String> {
    info!("Stopping voice conversion");

    state
        .with_gui_manager_async(|manager_arc| async move {
            let manager_guard = manager_arc.lock().unwrap();
            if let Some(ref manager) = *manager_guard {
                manager.stop_voice_conversion().await.map_err(|e| {
                    error!("Failed to stop voice conversion: {}", e);
                    format!("Failed to stop voice conversion: {}", e)
                })?;

                info!("Voice conversion stopped successfully");
                Ok(())
            } else {
                Err("GUI manager not initialized".to_string())
            }
        })
        .await
}

/// 更新设备配置
#[tauri::command]
async fn update_device_config(
    host_api: String,
    input_device: String,
    output_device: String,
    wasapi_exclusive: bool,
    state: State<'_, AppStateManager>,
) -> Result<(), String> {
    info!(
        "Updating device config: host_api={}, input={}, output={}, exclusive={}",
        host_api, input_device, output_device, wasapi_exclusive
    );

    state.with_gui_manager(|manager| {
        // TODO: 实现设备配置更新
        info!("Device configuration updated");
    })
}

/// 更新参数
#[tauri::command]
async fn update_parameter(
    param: String,
    value: serde_json::Value,
    state: State<'_, AppStateManager>,
) -> Result<(), String> {
    info!("Updating parameter: {} = {:?}", param, value);

    state.with_gui_manager(|manager| {
        // TODO: 根据参数类型更新相应的配置
        match param.as_str() {
            "threshold" => {
                if let Some(val) = value.as_i64() {
                    info!("Updated threshold to: {}", val);
                }
            }
            "pitch" => {
                if let Some(val) = value.as_i64() {
                    info!("Updated pitch to: {}", val);
                }
            }
            "formant" => {
                if let Some(val) = value.as_f64() {
                    info!("Updated formant to: {}", val);
                }
            }
            "indexRate" => {
                if let Some(val) = value.as_f64() {
                    info!("Updated index rate to: {}", val);
                }
            }
            "rmsMixRate" => {
                if let Some(val) = value.as_f64() {
                    info!("Updated RMS mix rate to: {}", val);
                }
            }
            "f0Method" => {
                if let Some(val) = value.as_str() {
                    info!("Updated F0 method to: {}", val);
                }
            }
            "blockTime" => {
                if let Some(val) = value.as_f64() {
                    info!("Updated block time to: {}", val);
                }
            }
            "nCpu" => {
                if let Some(val) = value.as_i64() {
                    info!("Updated CPU count to: {}", val);
                }
            }
            "crossfadeLength" => {
                if let Some(val) = value.as_f64() {
                    info!("Updated crossfade length to: {}", val);
                }
            }
            "extraTime" => {
                if let Some(val) = value.as_f64() {
                    info!("Updated extra time to: {}", val);
                }
            }
            "inputNoiseReduce" => {
                if let Some(val) = value.as_bool() {
                    info!("Updated input noise reduce to: {}", val);
                }
            }
            "outputNoiseReduce" => {
                if let Some(val) = value.as_bool() {
                    info!("Updated output noise reduce to: {}", val);
                }
            }
            "usePv" => {
                if let Some(val) = value.as_bool() {
                    info!("Updated use phase vocoder to: {}", val);
                }
            }
            _ => {
                warn!("Unknown parameter: {}", param);
            }
        }
    })
}

/// 获取实时状态
#[tauri::command]
async fn get_realtime_status(state: State<'_, AppStateManager>) -> Result<RealTimeStatus, String> {
    state.with_gui_manager(|manager| {
        let stats = manager.get_stats();
        let app_state = manager.get_state();

        RealTimeStatus {
            algorithm_latency: stats.algorithm_latency_ms,
            inference_time: stats.inference_time_ms,
            buffer_usage: stats.buffer_usage,
            cpu_usage: stats.cpu_usage,
            gpu_usage: stats.gpu_usage,
            is_converting: matches!(app_state, rvc_core::gui::AppState::Converting),
        }
    })
}

/// 验证配置
#[tauri::command]
async fn validate_config(
    config: VoiceConversionConfig,
    state: State<'_, AppStateManager>,
) -> Result<bool, String> {
    info!("Validating configuration");

    // 基本验证
    if config.pth_path.is_empty() {
        return Ok(false);
    }

    // 检查文件是否存在
    if !PathBuf::from(&config.pth_path).exists() {
        warn!("PTH file does not exist: {}", config.pth_path);
        return Ok(false);
    }

    if !config.index_path.is_empty() && !PathBuf::from(&config.index_path).exists() {
        warn!("Index file does not exist: {}", config.index_path);
        return Ok(false);
    }

    // 转换并验证 rvc-core 配置
    let rvc_config = config.to_rvc_config()?;

    state.with_gui_manager(|manager| {
        // TODO: 使用 rvc-core 的配置验证
        info!("Configuration validation passed");
        true
    })
}

// 旧的示例命令（保留以防需要）
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // 初始化日志
    env_logger::init();

    // 创建应用状态管理器
    let app_state = AppStateManager::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            greet,
            initialize_app,
            get_audio_devices,
            reload_devices,
            select_pth_file,
            select_index_file,
            start_voice_conversion,
            stop_voice_conversion,
            update_device_config,
            update_parameter,
            get_realtime_status,
            validate_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
