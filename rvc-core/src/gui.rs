//! GUI 状态管理模块
//!
//! 对应 Python 代码中的 GUI 类，负责管理 RVC 应用的状态和核心逻辑

use crate::{Config, ConfigManager, F0Method, RvcError, RvcResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc;

/// 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Input,
    Output,
}

/// 音频设备信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub hostapi: String,
    pub max_input_channels: usize,
    pub max_output_channels: usize,
    pub default_sample_rate: f64,
    pub is_input: bool,
    pub is_output: bool,
}

/// 设备管理器
#[derive(Debug)]
pub struct DeviceManager {
    /// 所有设备列表
    devices: Vec<AudioDeviceInfo>,
    /// 主机API列表
    host_apis: Vec<String>,
    /// 输入设备索引映射
    input_device_indices: HashMap<String, usize>,
    /// 输出设备索引映射
    output_device_indices: HashMap<String, usize>,
}

impl DeviceManager {
    /// 创建新的设备管理器
    pub fn new() -> Self {
        let mut manager = Self {
            devices: Vec::new(),
            host_apis: Vec::new(),
            input_device_indices: HashMap::new(),
            output_device_indices: HashMap::new(),
        };
        manager.initialize_default_devices();
        manager
    }

    /// 初始化默认设备
    fn initialize_default_devices(&mut self) {
        // 默认主机API
        self.host_apis = vec![
            "DirectSound".to_string(),
            "WASAPI".to_string(),
            "ASIO".to_string(),
            "MME".to_string(),
        ];

        // 模拟设备列表
        self.devices = vec![
            AudioDeviceInfo {
                id: "default_input".to_string(),
                index: 0,
                name: "Default Input Device".to_string(),
                hostapi: "DirectSound".to_string(),
                max_input_channels: 2,
                max_output_channels: 0,
                default_sample_rate: 44100.0,
                is_input: true,
                is_output: false,
            },
            AudioDeviceInfo {
                id: "default_output".to_string(),
                index: 1,
                name: "Default Output Device".to_string(),
                hostapi: "DirectSound".to_string(),
                max_input_channels: 0,
                max_output_channels: 2,
                default_sample_rate: 44100.0,
                is_input: false,
                is_output: true,
            },
            AudioDeviceInfo {
                id: "microphone".to_string(),
                index: 2,
                name: "Microphone (USB Audio)".to_string(),
                hostapi: "WASAPI".to_string(),
                max_input_channels: 1,
                max_output_channels: 0,
                default_sample_rate: 48000.0,
                is_input: true,
                is_output: false,
            },
            AudioDeviceInfo {
                id: "speakers".to_string(),
                index: 3,
                name: "Speakers (Realtek Audio)".to_string(),
                hostapi: "WASAPI".to_string(),
                max_input_channels: 0,
                max_output_channels: 2,
                default_sample_rate: 48000.0,
                is_input: false,
                is_output: true,
            },
        ];

        // 构建索引映射
        self.rebuild_indices();
    }

    /// 重建设备索引映射
    fn rebuild_indices(&mut self) {
        self.input_device_indices.clear();
        self.output_device_indices.clear();

        for (idx, device) in self.devices.iter().enumerate() {
            if device.is_input {
                self.input_device_indices.insert(device.name.clone(), idx);
            }
            if device.is_output {
                self.output_device_indices.insert(device.name.clone(), idx);
            }
        }
    }

    /// 列出主机API
    pub fn list_host_apis(&self) -> Vec<String> {
        self.host_apis.clone()
    }

    /// 列出设备
    pub fn list_devices(&self, device_type: DeviceType) -> Vec<AudioDeviceInfo> {
        self.devices
            .iter()
            .filter(|device| match device_type {
                DeviceType::Input => device.is_input,
                DeviceType::Output => device.is_output,
            })
            .cloned()
            .collect()
    }

    /// 根据主机API过滤设备
    pub fn list_devices_by_hostapi(
        &self,
        hostapi: &str,
        device_type: DeviceType,
    ) -> Vec<AudioDeviceInfo> {
        self.devices
            .iter()
            .filter(|device| {
                device.hostapi == hostapi
                    && match device_type {
                        DeviceType::Input => device.is_input,
                        DeviceType::Output => device.is_output,
                    }
            })
            .cloned()
            .collect()
    }

    /// 更新设备列表
    pub fn update_devices(&mut self, hostapi: Option<&str>) {
        // TODO: 实际实现中应该调用音频API获取真实设备
        // 这里只是模拟更新
        if let Some(api) = hostapi {
            // 过滤特定主机API的设备
            self.devices.retain(|device| device.hostapi == api);
        }
        self.rebuild_indices();
    }

    /// 获取设备信息
    pub fn get_device_info(&self, device_name: &str) -> Option<&AudioDeviceInfo> {
        self.devices
            .iter()
            .find(|device| device.name == device_name)
    }

    /// 获取设备采样率
    pub fn get_device_sample_rate(&self, device_name: &str) -> Option<f64> {
        self.get_device_info(device_name)
            .map(|device| device.default_sample_rate)
    }

    /// 获取设备通道数
    pub fn get_device_channels(&self, device_name: &str, is_input: bool) -> Option<usize> {
        self.get_device_info(device_name).map(|device| {
            if is_input {
                device.max_input_channels
            } else {
                device.max_output_channels
            }
        })
    }
}

/// GUI 应用状态
#[derive(Debug, Clone)]
pub enum AppState {
    /// 空闲状态
    Idle,
    /// 正在加载模型
    LoadingModel,
    /// 正在进行语音转换
    Converting,
    /// 输入监听模式
    InputMonitoring,
    /// 错误状态
    Error(String),
}

/// 实时统计信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct RealTimeStats {
    /// 算法延迟（毫秒）
    pub algorithm_latency_ms: f64,
    /// 推理时间（毫秒）
    pub inference_time_ms: f64,
    /// 音频缓冲区使用率
    pub buffer_usage: f64,
    /// CPU 使用率
    pub cpu_usage: f64,
    /// GPU 使用率（如果可用）
    pub gpu_usage: Option<f64>,
}

/// GUI 事件类型
#[derive(Debug, Clone)]
pub enum GuiEvent {
    /// 开始语音转换
    StartVoiceConversion,
    /// 停止语音转换
    StopVoiceConversion,
    /// 切换到输入监听
    SwitchToInputMonitoring,
    /// 切换到输出变声
    SwitchToVoiceConversion,
    /// 重新加载设备列表
    ReloadDevices,
    /// 更新配置
    UpdateConfig(Config),
    /// 加载模型
    LoadModel {
        pth_path: String,
        index_path: String,
    },
}

/// GUI 核心管理器，对应 Python 中的 GUI 类
pub struct GuiManager {
    /// 配置管理器
    config_manager: ConfigManager,
    /// 当前应用状态
    state: Arc<Mutex<AppState>>,
    /// 设备管理器
    device_manager: Arc<Mutex<DeviceManager>>,
    /// 实时统计信息
    stats: Arc<Mutex<RealTimeStats>>,
    /// 事件发送器
    event_sender: mpsc::UnboundedSender<GuiEvent>,
    /// 事件接收器
    event_receiver: Arc<Mutex<Option<mpsc::UnboundedReceiver<GuiEvent>>>>,
    /// 音频处理线程句柄
    audio_thread: Option<thread::JoinHandle<()>>,
    /// 是否正在运行
    running: Arc<Mutex<bool>>,
    /// F0 预测器
    f0_predictor: Option<Box<dyn crate::F0Predictor + Send + Sync>>,
    /// 延迟测量
    delay_time: Arc<Mutex<f64>>,
}

impl GuiManager {
    /// 创建新的 GUI 管理器
    pub fn new(config_path: PathBuf) -> RvcResult<Self> {
        let config_manager = ConfigManager::new(config_path);
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Ok(Self {
            config_manager,
            state: Arc::new(Mutex::new(AppState::Idle)),
            device_manager: Arc::new(Mutex::new(DeviceManager::new())),
            stats: Arc::new(Mutex::new(RealTimeStats {
                algorithm_latency_ms: 0.0,
                inference_time_ms: 0.0,
                buffer_usage: 0.0,
                cpu_usage: 0.0,
                gpu_usage: None,
            })),
            event_sender,
            event_receiver: Arc::new(Mutex::new(Some(event_receiver))),
            audio_thread: None,
            running: Arc::new(Mutex::new(false)),
            f0_predictor: None,
            delay_time: Arc::new(Mutex::new(0.0)),
        })
    }

    /// 初始化 GUI 管理器
    pub async fn initialize(&mut self) -> RvcResult<()> {
        // 加载配置
        self.config_manager.load()?;

        // 更新音频设备列表
        self.update_audio_devices(None).await?;

        // 初始化 F0 提取器
        let config = self.config_manager.config();
        let _f0_method = F0Method::from_str(&config.f0method)
            .ok_or_else(|| RvcError::config("Invalid F0 method"))?;

        // F0 predictor initialization would go here
        // self.f0_predictor = Some(F0PredictorFactory::create_default(f0_method)?);

        // 启动事件处理循环
        self.start_event_loop().await?;

        Ok(())
    }

    /// 获取当前应用状态
    pub fn get_state(&self) -> AppState {
        self.state.lock().unwrap().clone()
    }

    /// 设置应用状态
    pub fn set_state(&self, new_state: AppState) {
        let mut state = self.state.lock().unwrap();
        *state = new_state;
    }

    /// 获取音频设备列表
    pub fn get_audio_devices(&self) -> Vec<AudioDeviceInfo> {
        let device_manager = self.device_manager.lock().unwrap();
        device_manager.devices.clone()
    }

    /// 获取设备管理器（用于更高级的操作）
    pub fn get_device_manager(&self) -> Arc<Mutex<DeviceManager>> {
        Arc::clone(&self.device_manager)
    }

    /// 获取实时统计信息
    pub fn get_stats(&self) -> RealTimeStats {
        self.stats.lock().unwrap().clone()
    }

    /// 获取当前配置
    pub fn get_config(&self) -> Config {
        self.config_manager.config().clone()
    }

    /// 发送事件
    pub fn send_event(&self, event: GuiEvent) -> RvcResult<()> {
        self.event_sender
            .send(event)
            .map_err(|e| RvcError::other(format!("Failed to send event: {}", e)))
    }

    /// 更新音频设备列表
    pub async fn update_audio_devices(&self, hostapi: Option<&str>) -> RvcResult<()> {
        let mut device_manager = self.device_manager.lock().unwrap();
        device_manager.update_devices(hostapi);
        Ok(())
    }

    /// 获取主机 API 列表
    pub fn get_hostapis(&self) -> Vec<String> {
        let device_manager = self.device_manager.lock().unwrap();
        device_manager.list_host_apis()
    }

    /// 获取输入设备列表
    pub fn get_input_devices(&self, hostapi: Option<&str>) -> Vec<AudioDeviceInfo> {
        let device_manager = self.device_manager.lock().unwrap();
        if let Some(api) = hostapi {
            device_manager.list_devices_by_hostapi(api, DeviceType::Input)
        } else {
            device_manager.list_devices(DeviceType::Input)
        }
    }

    /// 获取输出设备列表
    pub fn get_output_devices(&self, hostapi: Option<&str>) -> Vec<AudioDeviceInfo> {
        let device_manager = self.device_manager.lock().unwrap();
        if let Some(api) = hostapi {
            device_manager.list_devices_by_hostapi(api, DeviceType::Output)
        } else {
            device_manager.list_devices(DeviceType::Output)
        }
    }

    /// 启动事件处理循环
    async fn start_event_loop(&mut self) -> RvcResult<()> {
        let receiver = {
            let mut receiver_option = self.event_receiver.lock().unwrap();
            receiver_option.take()
        };

        if let Some(mut receiver) = receiver {
            let state = Arc::clone(&self.state);
            let config_manager = self.config_manager.clone();
            let stats = Arc::clone(&self.stats);
            let running = Arc::clone(&self.running);

            tokio::spawn(async move {
                while let Some(event) = receiver.recv().await {
                    Self::handle_event(event, &state, &config_manager, &stats, &running).await;
                }
            });
        }

        Ok(())
    }

    /// 处理单个事件
    async fn handle_event(
        event: GuiEvent,
        state: &Arc<Mutex<AppState>>,
        _config_manager: &ConfigManager,
        _stats: &Arc<Mutex<RealTimeStats>>,
        _running: &Arc<Mutex<bool>>,
    ) {
        match event {
            GuiEvent::StartVoiceConversion => {
                {
                    let mut app_state = state.lock().unwrap();
                    *app_state = AppState::Converting;
                }

                {
                    let mut is_running = _running.lock().unwrap();
                    *is_running = true;
                }

                // 启动音频处理
                // 这里应该启动实际的音频流处理
                println!("Started voice conversion");
            }
            GuiEvent::StopVoiceConversion => {
                {
                    let mut app_state = state.lock().unwrap();
                    *app_state = AppState::Idle;
                }

                {
                    let mut is_running = _running.lock().unwrap();
                    *is_running = false;
                }

                println!("Stopped voice conversion");
            }
            GuiEvent::SwitchToInputMonitoring => {
                {
                    let mut app_state = state.lock().unwrap();
                    *app_state = AppState::InputMonitoring;
                }
                println!("Switched to input monitoring");
            }
            GuiEvent::SwitchToVoiceConversion => {
                {
                    let mut app_state = state.lock().unwrap();
                    *app_state = AppState::Idle;
                }
                println!("Switched to voice conversion mode");
            }
            GuiEvent::UpdateConfig(_new_config) => {
                // 这里应该更新配置并保存
                println!("Updated configuration");
            }
            GuiEvent::LoadModel {
                pth_path,
                index_path,
            } => {
                {
                    let mut app_state = state.lock().unwrap();
                    *app_state = AppState::LoadingModel;
                }

                // 模拟模型加载
                tokio::time::sleep(Duration::from_secs(2)).await;

                {
                    let mut app_state = state.lock().unwrap();
                    *app_state = AppState::Idle;
                }
                println!("Loaded model: {} with index: {}", pth_path, index_path);
            }
            GuiEvent::ReloadDevices => {
                println!("Reloading audio devices");
                // 这里应该重新扫描音频设备
            }
        }
    }

    /// 开始语音转换
    pub async fn start_voice_conversion(&self) -> RvcResult<()> {
        self.send_event(GuiEvent::StartVoiceConversion)
    }

    /// 停止语音转换
    pub async fn stop_voice_conversion(&self) -> RvcResult<()> {
        self.send_event(GuiEvent::StopVoiceConversion)
    }

    /// 加载模型
    pub async fn load_model(&self, pth_path: String, index_path: String) -> RvcResult<()> {
        self.send_event(GuiEvent::LoadModel {
            pth_path,
            index_path,
        })
    }

    /// 更新配置
    pub async fn update_config(&mut self, new_config: Config) -> RvcResult<()> {
        self.config_manager.config_mut().clone_from(&new_config);
        self.config_manager.save()?;
        self.send_event(GuiEvent::UpdateConfig(new_config))
    }

    /// 获取设备采样率
    pub fn get_device_sample_rate(&self, device_name: &str) -> Option<f64> {
        let device_manager = self.device_manager.lock().unwrap();
        device_manager.get_device_sample_rate(device_name)
    }

    /// 获取设备通道数
    pub fn get_device_channels(&self, device_name: &str, is_input: bool) -> Option<usize> {
        let device_manager = self.device_manager.lock().unwrap();
        device_manager.get_device_channels(device_name, is_input)
    }

    /// 设置延迟时间
    pub fn set_delay_time(&self, delay_ms: f64) {
        let mut delay = self.delay_time.lock().unwrap();
        *delay = delay_ms;
    }

    /// 获取延迟时间
    pub fn get_delay_time(&self) -> f64 {
        *self.delay_time.lock().unwrap()
    }

    /// 更新实时统计信息
    pub fn update_stats(&self, inference_time_ms: f64, buffer_usage: f64) {
        let mut stats = self.stats.lock().unwrap();
        stats.inference_time_ms = inference_time_ms;
        stats.buffer_usage = buffer_usage;
        stats.algorithm_latency_ms = self.get_delay_time();

        // 简化的 CPU 使用率计算
        stats.cpu_usage = 50.0; // 占位符

        // GPU 使用率（如果有）
        if crate::Cuda::is_available() {
            stats.gpu_usage = Some(30.0); // 占位符
        }
    }

    /// 验证配置
    pub fn validate_config(&self) -> RvcResult<()> {
        self.config_manager.config().validate()
    }

    /// 保存当前配置
    pub fn save_config(&self) -> RvcResult<()> {
        self.config_manager.save()
    }
}

impl Drop for GuiManager {
    fn drop(&mut self) {
        // 确保停止所有处理
        let mut running = self.running.lock().unwrap();
        *running = false;

        // 等待音频线程结束
        if let Some(handle) = self.audio_thread.take() {
            let _ = handle.join();
        }
    }
}

/// GUI 管理器的克隆实现，用于在多个组件间共享
impl Clone for ConfigManager {
    fn clone(&self) -> Self {
        // 注意：这里只是为了编译通过的简化实现
        // 实际使用中应该使用 Arc<Mutex<ConfigManager>> 来共享
        ConfigManager::new(PathBuf::from("config.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_gui_manager_creation() -> RvcResult<()> {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("gui_config.json");

        let manager = GuiManager::new(config_path)?;
        assert!(matches!(manager.get_state(), AppState::Idle));

        Ok(())
    }

    #[tokio::test]
    async fn test_audio_device_operations() -> RvcResult<()> {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("gui_config.json");

        let manager = GuiManager::new(config_path)?;
        manager.update_audio_devices(None).await?;

        let devices = manager.get_audio_devices();
        assert!(!devices.is_empty());

        let input_devices = manager.get_input_devices(None);
        let output_devices = manager.get_output_devices(None);

        assert!(input_devices.iter().all(|d| d.is_input));
        assert!(output_devices.iter().all(|d| d.is_output));

        Ok(())
    }

    #[tokio::test]
    async fn test_event_system() -> RvcResult<()> {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("gui_config.json");

        let manager = GuiManager::new(config_path)?;

        // 测试发送事件
        manager.send_event(GuiEvent::StartVoiceConversion)?;
        manager.send_event(GuiEvent::StopVoiceConversion)?;

        Ok(())
    }
}
