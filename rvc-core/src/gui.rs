//! GUI管理器模块
//!
//! 对应 Python gui_v1.py 的 GUI 类，提供完整的事件处理和状态管理功能

use crate::audio_stream::AudioStream;
use crate::config::Config;
use crate::{RvcError, RvcResult};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// 应用状态枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppState {
    /// 空闲状态
    Idle,
    /// 初始化中
    Initializing,
    /// 加载模型中
    LoadingModel,
    /// 语音转换中
    Converting,
    /// 输入监听中
    InputMonitoring,
    /// 错误状态
    Error(String),
}

/// 设备类型
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    Input,
    Output,
}

/// 音频设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    /// 设备名称
    pub name: String,
    /// 设备索引
    pub index: usize,
    /// 主机API
    pub host_api: String,
    /// 最大通道数
    pub max_channels: u32,
    /// 默认采样率
    pub default_sample_rate: f64,
}

/// 实时统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealTimeStats {
    /// 推理时间(毫秒)
    pub inference_time_ms: f64,
    /// 算法延迟(毫秒)
    pub algorithm_latency_ms: f64,
    /// 缓冲区使用率
    pub buffer_usage: f32,
    /// CPU使用率
    pub cpu_usage: f32,
    /// GPU使用率
    pub gpu_usage: Option<f32>,
    /// 当前音频流状态
    pub stream_active: bool,
}

impl Default for RealTimeStats {
    fn default() -> Self {
        Self {
            inference_time_ms: 0.0,
            algorithm_latency_ms: 0.0,
            buffer_usage: 0.0,
            cpu_usage: 0.0,
            gpu_usage: None,
            stream_active: false,
        }
    }
}

/// 设备管理器
pub struct DeviceManager {
    /// 可用的主机API列表
    host_apis: Vec<String>,
    /// 输入设备列表
    input_devices: Vec<AudioDeviceInfo>,
    /// 输出设备列表
    output_devices: Vec<AudioDeviceInfo>,
    /// 当前选择的主机API
    current_host_api: Option<String>,
}

impl DeviceManager {
    pub fn new() -> Self {
        let mut manager = Self {
            host_apis: Vec::new(),
            input_devices: Vec::new(),
            output_devices: Vec::new(),
            current_host_api: None,
        };

        // 初始化设备列表
        manager.refresh_devices();

        manager
    }

    /// 刷新设备列表
    pub fn refresh_devices(&mut self) {
        // 模拟获取主机API列表
        self.host_apis = vec![
            "DirectSound".to_string(),
            "MME".to_string(),
            "WASAPI".to_string(),
            "ASIO".to_string(),
        ];

        // 模拟获取设备列表
        let current_host_api = self.current_host_api.clone();
        self.refresh_devices_for_hostapi(current_host_api.as_deref());
    }

    /// 为指定主机API刷新设备列表
    pub fn refresh_devices_for_hostapi(&mut self, host_api: Option<&str>) {
        self.current_host_api = host_api.map(|s| s.to_string());

        // 模拟设备发现逻辑
        self.input_devices = vec![
            AudioDeviceInfo {
                name: "Default Input".to_string(),
                index: 0,
                host_api: host_api.unwrap_or("DirectSound").to_string(),
                max_channels: 2,
                default_sample_rate: 48000.0,
            },
            AudioDeviceInfo {
                name: "Microphone".to_string(),
                index: 1,
                host_api: host_api.unwrap_or("DirectSound").to_string(),
                max_channels: 2,
                default_sample_rate: 48000.0,
            },
        ];

        self.output_devices = vec![
            AudioDeviceInfo {
                name: "Default Output".to_string(),
                index: 0,
                host_api: host_api.unwrap_or("DirectSound").to_string(),
                max_channels: 2,
                default_sample_rate: 48000.0,
            },
            AudioDeviceInfo {
                name: "Speakers".to_string(),
                index: 1,
                host_api: host_api.unwrap_or("DirectSound").to_string(),
                max_channels: 2,
                default_sample_rate: 48000.0,
            },
        ];
    }

    pub fn get_host_apis(&self) -> &[String] {
        &self.host_apis
    }

    pub fn get_input_devices(&self) -> &[AudioDeviceInfo] {
        &self.input_devices
    }

    pub fn get_output_devices(&self) -> &[AudioDeviceInfo] {
        &self.output_devices
    }

    pub fn find_device_by_name(
        &self,
        name: &str,
        device_type: DeviceType,
    ) -> Option<&AudioDeviceInfo> {
        match device_type {
            DeviceType::Input => self.input_devices.iter().find(|d| d.name == name),
            DeviceType::Output => self.output_devices.iter().find(|d| d.name == name),
        }
    }

    pub fn get_device_sample_rate(&self, name: &str) -> Option<f64> {
        self.input_devices
            .iter()
            .chain(self.output_devices.iter())
            .find(|d| d.name == name)
            .map(|d| d.default_sample_rate)
    }

    pub fn get_device_channels(&self, name: &str, is_input: bool) -> Option<u32> {
        let devices = if is_input {
            &self.input_devices
        } else {
            &self.output_devices
        };

        devices
            .iter()
            .find(|d| d.name == name)
            .map(|d| d.max_channels)
    }
}

/// GUI管理器，对应Python的GUI类
pub struct GuiManager {
    /// 当前应用状态
    state: AppState,
    /// 设备管理器
    device_manager: DeviceManager,
    /// 音频流管理器
    audio_stream: Option<AudioStream>,
    /// 实时统计信息
    stats: Arc<Mutex<RealTimeStats>>,
    /// 配置文件路径
    config_path: PathBuf,
    /// 延迟时间计算缓存
    delay_time_cache: Option<f64>,
    /// 最后一次统计更新时间
    last_stats_update: Instant,
}

impl GuiManager {
    /// 创建新的GUI管理器
    pub fn new(config_path: PathBuf) -> RvcResult<Self> {
        Ok(Self {
            state: AppState::Idle,
            device_manager: DeviceManager::new(),
            audio_stream: None,
            stats: Arc::new(Mutex::new(RealTimeStats::default())),
            config_path,
            delay_time_cache: None,
            last_stats_update: Instant::now(),
        })
    }

    /// 初始化应用，对应Python GUI.__init__和load方法的组合
    pub async fn initialize(&mut self) -> RvcResult<()> {
        self.state = AppState::Initializing;

        // 刷新设备列表
        self.device_manager.refresh_devices();

        // 验证配置文件目录
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    RvcError::Other(format!("Failed to create config directory: {}", e))
                })?;
            }
        }

        self.state = AppState::Idle;
        Ok(())
    }

    /// 验证配置，对应Python GUI.set_values方法
    pub fn validate_config(&self) -> RvcResult<()> {
        // 这里应该验证各种配置参数
        // 包括模型文件路径、设备设置等
        Ok(())
    }

    /// 开始语音转换，对应Python GUI.start_vc方法
    pub async fn start_voice_conversion(&mut self) -> RvcResult<()> {
        if self.state == AppState::Converting {
            return Err(RvcError::other("Voice conversion already running"));
        }

        self.state = AppState::LoadingModel;

        // 模拟模型加载过程
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 启动音频流
        self.start_audio_stream().await?;

        self.state = AppState::Converting;

        // 更新统计信息
        {
            let mut stats = self.stats.lock().unwrap();
            stats.stream_active = true;
        }

        Ok(())
    }

    /// 停止语音转换，对应Python GUI.stop_stream方法
    pub async fn stop_voice_conversion(&mut self) -> RvcResult<()> {
        if let Some(ref mut _stream) = self.audio_stream {
            // 停止音频流
            // _stream.stop().await?;
        }

        self.audio_stream = None;
        self.state = AppState::Idle;

        // 更新统计信息
        {
            let mut stats = self.stats.lock().unwrap();
            stats.stream_active = false;
            stats.inference_time_ms = 0.0;
            stats.algorithm_latency_ms = 0.0;
        }

        Ok(())
    }

    /// 启动音频流
    async fn start_audio_stream(&mut self) -> RvcResult<()> {
        // 这里应该创建和启动实际的音频流
        // 目前只是模拟
        Ok(())
    }

    /// 更新音频设备列表，对应Python GUI.update_devices方法
    pub async fn update_audio_devices(&mut self, host_api: Option<&str>) -> RvcResult<()> {
        self.device_manager.refresh_devices_for_hostapi(host_api);

        // 验证当前选择的设备是否仍然有效
        // 如果无效，则重置为默认设备

        Ok(())
    }

    /// 获取主机API列表，对应Python GUI event_handler中的hostapi处理
    pub fn get_hostapis(&self) -> Vec<String> {
        self.device_manager.get_host_apis().to_vec()
    }

    /// 获取输入设备列表
    pub fn get_input_devices(&self, host_api: Option<&str>) -> Vec<AudioDeviceInfo> {
        if let Some(api) = host_api {
            // 如果指定了主机API，只返回该API下的设备
            self.device_manager
                .get_input_devices()
                .iter()
                .filter(|d| d.host_api == api)
                .cloned()
                .collect()
        } else {
            self.device_manager.get_input_devices().to_vec()
        }
    }

    /// 获取输出设备列表
    pub fn get_output_devices(&self, host_api: Option<&str>) -> Vec<AudioDeviceInfo> {
        if let Some(api) = host_api {
            self.device_manager
                .get_output_devices()
                .iter()
                .filter(|d| d.host_api == api)
                .cloned()
                .collect()
        } else {
            self.device_manager.get_output_devices().to_vec()
        }
    }

    /// 获取设备采样率，对应Python GUI.get_device_samplerate方法
    pub fn get_device_sample_rate(&self, device_name: &str) -> Option<f64> {
        self.device_manager.get_device_sample_rate(device_name)
    }

    /// 获取设备通道数，对应Python GUI.get_device_channels方法
    pub fn get_device_channels(&self, device_name: &str, is_input: bool) -> Option<u32> {
        self.device_manager
            .get_device_channels(device_name, is_input)
    }

    /// 实时参数更新，对应Python GUI event_handler中的参数热更新
    pub fn update_realtime_parameter(
        &mut self,
        name: &str,
        value: serde_json::Value,
    ) -> RvcResult<()> {
        match name {
            "pitch" => {
                if let Some(v) = value.as_i64() {
                    // 更新pitch参数
                    // 如果RVC实例存在，调用change_key方法
                    log::info!("Updated pitch to: {}", v);
                }
            }
            "formant" => {
                if let Some(v) = value.as_f64() {
                    // 更新formant参数
                    // 如果RVC实例存在，调用change_formant方法
                    log::info!("Updated formant to: {}", v);
                }
            }
            "index_rate" => {
                if let Some(v) = value.as_f64() {
                    // 更新index_rate参数
                    // 如果RVC实例存在，调用change_index_rate方法
                    log::info!("Updated index_rate to: {}", v);
                }
            }
            "rms_mix_rate" => {
                if let Some(v) = value.as_f64() {
                    // 更新rms_mix_rate参数
                    log::info!("Updated rms_mix_rate to: {}", v);
                }
            }
            "threshold" => {
                if let Some(v) = value.as_f64() {
                    // 更新threshold参数
                    log::info!("Updated threshold to: {}", v);
                }
            }
            "f0method" => {
                if let Some(v) = value.as_str() {
                    // 更新f0method参数
                    log::info!("Updated f0method to: {}", v);
                }
            }
            "I_noise_reduce" => {
                if let Some(v) = value.as_bool() {
                    // 更新输入降噪设置
                    // 需要重新计算延迟时间
                    self.recalculate_delay_time();
                    log::info!("Updated I_noise_reduce to: {}", v);
                }
            }
            "O_noise_reduce" => {
                if let Some(v) = value.as_bool() {
                    // 更新输出降噪设置
                    log::info!("Updated O_noise_reduce to: {}", v);
                }
            }
            "use_pv" => {
                if let Some(v) = value.as_bool() {
                    // 更新相位声码器设置
                    log::info!("Updated use_pv to: {}", v);
                }
            }
            _ => {
                // 其他参数需要重启音频流
                if self.state == AppState::Converting {
                    // 对于不支持热更新的参数，需要重启
                    log::warn!("Parameter {} requires restart", name);
                    return Err(RvcError::other("Parameter requires restart"));
                }
            }
        }

        Ok(())
    }

    /// 重新计算延迟时间，对应Python GUI event_handler中的延迟计算
    fn recalculate_delay_time(&mut self) {
        // 计算总延迟时间
        // delay_time = stream_latency + block_time + crossfade_length + buffer_time
        let mut delay_time = 0.0;

        // 添加流延迟
        if let Some(_stream) = &self.audio_stream {
            // delay_time += stream.get_latency();
            delay_time += 0.01; // 模拟流延迟
        }

        // 添加算法延迟
        delay_time += 0.25; // block_time
        delay_time += 0.05; // crossfade_length
        delay_time += 0.01; // 缓冲时间

        // 如果启用了降噪，添加额外延迟
        // if I_noise_reduce {
        //     delay_time += min(crossfade_length, 0.04);
        // }

        self.delay_time_cache = Some(delay_time);

        // 更新统计信息
        {
            let mut stats = self.stats.lock().unwrap();
            stats.algorithm_latency_ms = delay_time * 1000.0;
        }
    }

    /// 获取实时统计信息，对应Python GUI的状态显示
    pub fn get_stats(&self) -> RealTimeStats {
        let mut stats = self.stats.lock().unwrap().clone();

        // 更新CPU使用率等信息
        if self.last_stats_update.elapsed().as_millis() > 100 {
            let mut rng = rand::thread_rng();
            // 模拟CPU使用率更新
            stats.cpu_usage = 15.0 + rng.gen::<f32>() * 10.0;

            // 如果有GPU，更新GPU使用率
            if torch::cuda::is_available() {
                stats.gpu_usage = Some(20.0 + rng.gen::<f32>() * 15.0);
            }
        }

        stats
    }

    /// 获取当前应用状态
    pub fn get_state(&self) -> AppState {
        self.state.clone()
    }

    /// 模拟音频回调处理，对应Python GUI.audio_callback方法
    fn audio_callback(&self, _input: &[f32], _output: &mut [f32]) -> RvcResult<()> {
        let start_time = Instant::now();

        // 这里应该实现实际的音频处理逻辑：
        // 1. 音频预处理（噪声抑制、阈值检查）
        // 2. RVC推理
        // 3. 后处理（RMS混合、SOLA算法）
        // 4. 相位声码器（如果启用）

        // 模拟处理时间
        std::thread::sleep(std::time::Duration::from_micros(100));

        // 更新推理时间统计
        let inference_time = start_time.elapsed();
        {
            let mut stats = self.stats.lock().unwrap();
            stats.inference_time_ms = inference_time.as_secs_f64() * 1000.0;
        }

        Ok(())
    }

    /// 切换功能模式，对应Python GUI event_handler中的模式切换
    pub async fn switch_function_mode(&mut self, mode: &str) -> RvcResult<()> {
        match mode {
            "vc" => {
                // 切换到语音转换模式
                if self.state == AppState::Converting {
                    self.stop_voice_conversion().await?;
                }
                log::info!("Switched to voice conversion mode");
            }
            "im" => {
                // 切换到输入监听模式
                if self.state == AppState::Converting {
                    self.stop_voice_conversion().await?;
                }
                // 启动简单的直通模式
                self.state = AppState::InputMonitoring;
                log::info!("Switched to input monitoring mode");
            }
            _ => {
                return Err(RvcError::other(format!("Unknown function mode: {}", mode)));
            }
        }

        Ok(())
    }

    /// 保存当前配置到文件
    pub fn save_current_settings(&self, config: &Config) -> RvcResult<()> {
        let json_str = serde_json::to_string_pretty(config)
            .map_err(|e| RvcError::other(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(&self.config_path, json_str)
            .map_err(|e| RvcError::Other(format!("Failed to write config file: {}", e)))?;

        log::info!("Configuration saved to: {:?}", self.config_path);
        Ok(())
    }

    /// 加载配置文件
    pub fn load_settings(&self) -> RvcResult<Config> {
        if !self.config_path.exists() {
            // 如果配置文件不存在，返回默认配置
            return Ok(Config::default());
        }

        let json_str = std::fs::read_to_string(&self.config_path)
            .map_err(|e| RvcError::Other(format!("Failed to read config file: {}", e)))?;

        let config: Config = serde_json::from_str(&json_str)
            .map_err(|e| RvcError::other(format!("Failed to parse config: {}", e)))?;

        Ok(config)
    }
}

impl Drop for GuiManager {
    fn drop(&mut self) {
        // 确保在销毁时停止所有音频流
        if self.state == AppState::Converting || self.state == AppState::InputMonitoring {
            // 同步版本的停止操作
            self.audio_stream = None;
            self.state = AppState::Idle;
        }
    }
}

// 添加外部依赖模块的模拟实现
mod torch {
    pub mod cuda {
        pub fn is_available() -> bool {
            // 模拟CUDA可用性检查
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_gui_manager_creation() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = GuiManager::new(config_path);
        assert!(manager.is_ok());

        let mut manager = manager.unwrap();
        assert!(matches!(manager.get_state(), AppState::Idle));

        // 测试初始化
        assert!(manager.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_voice_conversion_lifecycle() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = GuiManager::new(config_path).unwrap();
        manager.initialize().await.unwrap();

        // 测试开始转换
        assert!(manager.start_voice_conversion().await.is_ok());
        assert!(matches!(manager.get_state(), AppState::Converting));

        // 测试停止转换
        assert!(manager.stop_voice_conversion().await.is_ok());
        assert!(matches!(manager.get_state(), AppState::Idle));
    }

    #[test]
    fn test_device_management() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let manager = GuiManager::new(config_path).unwrap();

        // 测试设备列表获取
        let hostapis = manager.get_hostapis();
        assert!(!hostapis.is_empty());

        let input_devices = manager.get_input_devices(None);
        assert!(!input_devices.is_empty());

        let output_devices = manager.get_output_devices(None);
        assert!(!output_devices.is_empty());
    }

    #[test]
    fn test_parameter_updates() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        let mut manager = GuiManager::new(config_path).unwrap();

        // 测试参数更新
        let result = manager.update_realtime_parameter("pitch", serde_json::json!(12));
        assert!(result.is_ok());

        let result = manager.update_realtime_parameter("formant", serde_json::json!(1.2));
        assert!(result.is_ok());

        let result = manager.update_realtime_parameter("index_rate", serde_json::json!(0.8));
        assert!(result.is_ok());
    }
}
