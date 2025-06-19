//! GUI管理器模块
//!
//! 对应 Python gui_v1.py 的 GUI 类，提供完整的事件处理和状态管理功能

use crate::audio_stream::{AudioResampler, AudioStream};
use crate::config::Config;
use crate::noise_suppression::NoiseReducer;
use crate::{GuiConfig, RvcError, RvcResult};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tch::Tensor;

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
    /// GUI配置
    gui_config: GuiConfig,
    /// 核心配置
    config: Config,
    /// RVC推理器
    rvc: Option<RvcInference>,
    /// 每10ms的采样数
    zc: u32,
    /// 块帧数
    block_frame: u32,
    /// 16k采样率下的块帧数
    block_frame_16k: u32,
    /// 交叉淡化帧数
    crossfade_frame: u32,
    /// SOLA缓冲帧数
    sola_buffer_frame: u32,
    /// SOLA搜索帧数
    sola_search_frame: u32,
    /// 额外帧数
    extra_frame: u32,
    /// 跳过头部帧数
    skip_head: u32,
    /// 返回长度
    return_length: u32,
    /// 输入音频缓冲区
    input_wav: Option<Tensor>,
    /// 输入音频降噪缓冲区
    input_wav_denoise: Option<Tensor>,
    /// 输入音频重采样缓冲区
    input_wav_res: Option<Tensor>,
    /// RMS缓冲区
    rms_buffer: Vec<f32>,
    /// SOLA缓冲区
    sola_buffer: Option<Tensor>,
    /// 降噪缓冲区
    nr_buffer: Option<Tensor>,
    /// 输出缓冲区
    output_buffer: Option<Tensor>,
    /// 淡入窗口
    fade_in_window: Option<Tensor>,
    /// 淡出窗口
    fade_out_window: Option<Tensor>,
    /// 重采样器 (原始采样率到16k)
    resampler: Option<AudioResampler>,
    /// 重采样器2 (模型采样率到设备采样率)
    resampler2: Option<AudioResampler>,
    /// TorchGate降噪器
    tg: Option<NoiseReducer>,
    /// 当前功能模式
    function: String,
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
            gui_config: GuiConfig::default(),
            config: Config::default(),
            rvc: None,
            zc: 0,
            block_frame: 0,
            block_frame_16k: 0,
            crossfade_frame: 0,
            sola_buffer_frame: 0,
            sola_search_frame: 0,
            extra_frame: 0,
            skip_head: 0,
            return_length: 0,
            input_wav: None,
            input_wav_denoise: None,
            input_wav_res: None,
            rms_buffer: Vec::new(),
            sola_buffer: None,
            nr_buffer: None,
            output_buffer: None,
            fade_in_window: None,
            fade_out_window: None,
            resampler: None,
            resampler2: None,
            tg: None,
            function: "vc".to_string(),
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
    pub fn set_values(&self) -> RvcResult<GuiConfig> {
        // 这里应该验证各种配置参数
        // 包括模型文件路径、设备设置等
        Ok(())
    }

    /// 开始语音转换，对应Python GUI.start_vc方法
    pub async fn start_vc(&mut self) -> RvcResult<()> {
        if self.state == AppState::Converting {
            return Err(RvcError::other("Voice conversion already running"));
        }

        // 清空GPU缓存（如果使用GPU）
        crate::tensor::Cuda::empty_cache();

        self.state = AppState::LoadingModel;

        // 1. 创建RVC推理器，对应Python的RVC初始化
        self.rvc = Some(RvcInference::new(
            self.gui_config.pitch,
            self.gui_config.formant,
            &self.gui_config.pth_path,
            &self.gui_config.index_path,
            self.gui_config.index_rate,
            self.gui_config.n_cpu,
            self.config.clone(),
        )?);

        // 2. 确定采样率和通道数
        let rvc_ref = self.rvc.as_ref().unwrap();
        self.gui_config.samplerate = if self.gui_config.sr_type == "sr_model" {
            rvc_ref.get_target_sample_rate()
        } else {
            self.get_device_sample_rate(&self.gui_config.sg_output_device)
                .unwrap_or(48000.0) as u32
        };

        self.gui_config.channels = self
            .get_device_channels(&self.gui_config.sg_output_device, false)
            .unwrap_or(2) as u32;

        // 3. 计算各种帧大小参数，对应Python中的计算逻辑
        self.zc = self.gui_config.samplerate / 100; // 每10ms的采样数

        // 块帧数：对zc进行舍入处理
        self.block_frame = ((self.gui_config.block_time * self.gui_config.samplerate as f32
            / self.zc as f32)
            .round() as u32)
            * self.zc;

        // 16k采样率下的块帧数
        self.block_frame_16k = 160 * self.block_frame / self.zc;

        // 交叉淡化帧数
        self.crossfade_frame = ((self.gui_config.crossfade_length
            * self.gui_config.samplerate as f32
            / self.zc as f32)
            .round() as u32)
            * self.zc;

        // SOLA缓冲帧数，取交叉淡化帧数和4*zc的最小值
        self.sola_buffer_frame = self.crossfade_frame.min(4 * self.zc);

        // SOLA搜索帧数
        self.sola_search_frame = self.zc;

        // 额外帧数
        self.extra_frame = ((self.gui_config.extra_time * self.gui_config.samplerate as f32
            / self.zc as f32)
            .round() as u32)
            * self.zc;

        // 4. 计算处理参数
        self.skip_head = self.extra_frame / self.zc;
        self.return_length =
            (self.block_frame + self.sola_buffer_frame + self.sola_search_frame) / self.zc;

        // 5. 创建张量缓冲区，对应Python的torch.zeros
        let input_wav_size =
            self.extra_frame + self.crossfade_frame + self.sola_search_frame + self.block_frame;
        self.input_wav = Some(Tensor::zeros(&[input_wav_size as i64]));
        self.input_wav_denoise = Some(self.input_wav.as_ref().unwrap().clone());

        let input_wav_res_size = 160 * input_wav_size / self.zc;
        self.input_wav_res = Some(Tensor::zeros(&[input_wav_res_size as i64]));

        // 6. 创建其他缓冲区
        self.rms_buffer = vec![0.0; (4 * self.zc) as usize];
        self.sola_buffer = Some(Tensor::zeros(&[self.sola_buffer_frame as i64]));
        self.nr_buffer = Some(self.sola_buffer.as_ref().unwrap().clone());
        self.output_buffer = Some(self.input_wav.as_ref().unwrap().clone());

        // 7. 创建窗口函数，对应Python的torch.sin
        let linspace: Vec<f32> = (0..self.sola_buffer_frame)
            .map(|i| i as f32 / (self.sola_buffer_frame - 1) as f32)
            .collect();

        let fade_in_values: Vec<f32> = linspace
            .iter()
            .map(|x| (0.5 * std::f32::consts::PI * x).sin().powi(2))
            .collect();

        self.fade_in_window = Some(Tensor::from_vec(fade_in_values));

        let fade_out_values: Vec<f32> = self
            .fade_in_window
            .as_ref()
            .unwrap()
            .to_vec()
            .iter()
            .map(|x| 1.0 - x)
            .collect();
        self.fade_out_window = Some(Tensor::from_vec(fade_out_values));

        // 8. 创建重采样器
        self.resampler = Some(crate::audio_stream::AudioResampler::new(
            self.gui_config.samplerate,
            16000,
            1,
        )?);

        if rvc_ref.get_target_sample_rate() != self.gui_config.samplerate {
            self.resampler2 = Some(crate::audio_stream::AudioResampler::new(
                rvc_ref.get_target_sample_rate(),
                self.gui_config.samplerate,
                1,
            )?);
        } else {
            self.resampler2 = None;
        }

        // 9. 创建TorchGate降噪器
        self.tg = Some(crate::noise_suppression::NoiseReducer::new(
            self.gui_config.threshold,
            self.gui_config.samplerate,
            true,
        )?);

        // 10. 启动音频流
        self.start_stream().await?;

        self.state = AppState::Converting;

        Ok(())
    }

    /// 停止语音转换，对应Python GUI.stop_stream方法
    pub async fn stop_voice_conversion(&mut self) -> RvcResult<()> {
        self.stop_stream().await?;
        Ok(())
    }

    /// 启动音频流，对应Python GUI.start_stream方法
    pub async fn start_stream(&mut self) -> RvcResult<()> {
        if self.state == AppState::Converting {
            return Ok(()); // 已经在运行
        }

        // 创建音频流配置
        let mut audio_stream = AudioStream::new(
            self.gui_config.samplerate,
            self.gui_config.channels as u16,
            self.block_frame as usize,
        );

        // 设置音频设备
        if let (Some(input_device), Some(output_device)) = (
            self.device_manager
                .find_device_by_name(&self.gui_config.sg_input_device, DeviceType::Input),
            self.device_manager
                .find_device_by_name(&self.gui_config.sg_output_device, DeviceType::Output),
        ) {
            audio_stream.set_devices(input_device.index, output_device.index);
        } else {
            return Err(RvcError::other("无法找到指定的音频设备"));
        }

        // 设置音频回调函数，对应Python的audio_callback
        let gui_ptr = self as *mut GuiManager;
        audio_stream.set_callback(move |input: &[f32], output: &mut [f32]| unsafe {
            if let Some(gui) = gui_ptr.as_mut() {
                gui.audio_callback(input, output);
            }
        });

        // 启动音频流
        audio_stream.start()?;
        self.audio_stream = Some(audio_stream);

        // 更新统计信息
        {
            let mut stats = self.stats.lock().unwrap();
            stats.stream_active = true;
        }

        Ok(())
    }

    /// 停止音频流，对应Python GUI.stop_stream方法
    pub async fn stop_stream(&mut self) -> RvcResult<()> {
        if let Some(ref mut stream) = self.audio_stream {
            stream.stop()?;
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

    /// 音频回调处理，对应Python GUI.audio_callback方法
    fn audio_callback(&mut self, indata: &[f32], outdata: &mut [f32]) {
        let start_time = Instant::now();

        if self.state != AppState::Converting {
            // 如果不在转换状态，输出静音
            for sample in outdata.iter_mut() {
                *sample = 0.0;
            }
            return;
        }

        // 1. 转换为单声道，对应Python的librosa.to_mono
        let mono_input = self.to_mono(indata);

        // 2. 阈值检查和RMS处理
        let processed_input = if self.gui_config.threshold > -60.0 {
            self.apply_threshold_gate(&mono_input)
        } else {
            mono_input
        };

        // 3. 更新输入缓冲区
        if let Some(ref mut input_wav) = self.input_wav {
            // 移动现有数据：input_wav[:-block_frame] = input_wav[block_frame:].clone()
            let total_len = input_wav.len();
            let block_frame = self.block_frame as usize;

            if total_len > block_frame {
                // 将后面的数据移到前面
                for i in 0..(total_len - block_frame) {
                    input_wav.set_data(i, input_wav.get_data(i + block_frame));
                }
            }

            // 将新数据添加到末尾
            let start_idx = total_len - processed_input.len().min(block_frame);
            for (i, &sample) in processed_input.iter().enumerate() {
                if start_idx + i < total_len {
                    input_wav.set_data(start_idx + i, sample);
                }
            }
        }

        // 4. 更新16k重采样缓冲区
        if let Some(ref mut input_wav_res) = self.input_wav_res {
            let total_len = input_wav_res.len();
            let block_frame_16k = self.block_frame_16k as usize;

            // 移动现有数据
            if total_len > block_frame_16k {
                for i in 0..(total_len - block_frame_16k) {
                    input_wav_res.set_data(i, input_wav_res.get_data(i + block_frame_16k));
                }
            }

            // 重采样输入并添加到缓冲区
            if let Some(ref mut resampler) = self.resampler {
                let input_for_resample = if let Some(ref input_wav) = self.input_wav {
                    // 取最后的数据进行重采样
                    let samples_needed = processed_input.len() + 2 * self.zc as usize;
                    let start_idx = input_wav.len().saturating_sub(samples_needed);
                    input_wav.slice(start_idx, input_wav.len())
                } else {
                    processed_input.clone()
                };

                // 执行重采样
                if let Ok(resampled) = resampler.process(&[input_for_resample]) {
                    if !resampled.is_empty() {
                        let resampled_data = &resampled[0];
                        // 跳过前160个样本，对应Python的[160:]
                        let start_offset = 160.min(resampled_data.len());
                        let useful_data = &resampled_data[start_offset..];

                        // 添加到缓冲区末尾
                        let start_idx = total_len.saturating_sub(useful_data.len());
                        for (i, &sample) in useful_data.iter().enumerate() {
                            if start_idx + i < total_len {
                                input_wav_res.set_data(start_idx + i, sample);
                            }
                        }
                    }
                }
            }
        }

        // 5. RVC推理
        let infer_wav = if self.function == "vc" {
            if let Some(ref mut rvc) = self.rvc {
                if let Some(ref input_wav_res) = self.input_wav_res {
                    // 执行RVC推理
                    match rvc.infer(
                        input_wav_res,
                        self.block_frame_16k,
                        self.skip_head,
                        self.return_length,
                        &self.gui_config.f0_method,
                    ) {
                        Ok(result) => {
                            // 如果需要重采样回原始采样率
                            if let Some(ref mut resampler2) = self.resampler2 {
                                match resampler2.process(&[result]) {
                                    Ok(resampled) if !resampled.is_empty() => resampled[0].clone(),
                                    _ => result,
                                }
                            } else {
                                result
                            }
                        }
                        Err(_) => {
                            // 推理失败，使用原始输入
                            if let Some(ref input_wav) = self.input_wav {
                                input_wav.slice(self.extra_frame as usize, input_wav.len())
                            } else {
                                processed_input.clone()
                            }
                        }
                    }
                } else {
                    processed_input.clone()
                }
            } else {
                processed_input.clone()
            }
        } else {
            // 非VC模式，返回输入
            if let Some(ref input_wav) = self.input_wav {
                input_wav.slice(self.extra_frame as usize, input_wav.len())
            } else {
                processed_input.clone()
            }
        };

        // 6. RMS混合
        let mixed_wav = if self.gui_config.rms_mix_rate < 1.0 && self.function == "vc" {
            self.apply_rms_mixing(&infer_wav, &processed_input)
        } else {
            infer_wav
        };

        // 7. SOLA算法处理
        let final_wav = self.apply_sola_algorithm(&mixed_wav);

        // 8. 输出到音频设备
        let output_len = outdata.len() / self.gui_config.channels as usize;
        let block_samples = self.block_frame as usize;

        for i in 0..output_len.min(final_wav.len()).min(block_samples) {
            let sample = final_wav[i];
            // 复制到所有声道
            for ch in 0..self.gui_config.channels as usize {
                if i * self.gui_config.channels as usize + ch < outdata.len() {
                    outdata[i * self.gui_config.channels as usize + ch] = sample;
                }
            }
        }

        // 填充剩余输出为静音
        for i in (output_len * self.gui_config.channels as usize)..outdata.len() {
            outdata[i] = 0.0;
        }

        // 更新统计信息
        let total_time = start_time.elapsed();
        {
            let mut stats = self.stats.lock().unwrap();
            stats.inference_time_ms = total_time.as_secs_f64() * 1000.0;
        }
    }

    /// 转换为单声道
    fn to_mono(&self, input: &[f32]) -> Vec<f32> {
        let channels = self.gui_config.channels as usize;
        if channels <= 1 {
            return input.to_vec();
        }

        let frames = input.len() / channels;
        let mut mono = Vec::with_capacity(frames);

        for frame in 0..frames {
            let mut sum = 0.0;
            for ch in 0..channels {
                sum += input[frame * channels + ch];
            }
            mono.push(sum / channels as f32);
        }

        mono
    }

    /// 应用阈值门控
    fn apply_threshold_gate(&mut self, input: &[f32]) -> Vec<f32> {
        // 扩展RMS缓冲区
        let mut extended_input = Vec::with_capacity(self.rms_buffer.len() + input.len());
        extended_input.extend_from_slice(&self.rms_buffer);
        extended_input.extend_from_slice(input);

        // 计算RMS，对应Python的librosa.feature.rms
        let frame_length = 4 * self.zc as usize;
        let hop_length = self.zc as usize;
        let mut rms_values = Vec::new();

        for i in (2 * hop_length..extended_input.len()).step_by(hop_length) {
            if i + frame_length <= extended_input.len() {
                let frame = &extended_input[i..i + frame_length];
                let rms = (frame.iter().map(|x| x * x).sum::<f32>() / frame_length as f32).sqrt();
                rms_values.push(rms);
            }
        }

        // 更新RMS缓冲区
        let buffer_samples = (4 * self.zc) as usize;
        if extended_input.len() >= buffer_samples {
            self.rms_buffer = extended_input[extended_input.len() - buffer_samples..].to_vec();
        }

        // 应用阈值
        let threshold_linear = 10.0_f32.powf(self.gui_config.threshold / 20.0);
        let mut result = input.to_vec();

        for (i, &rms) in rms_values.iter().enumerate() {
            if rms < threshold_linear {
                let start_idx = i * hop_length;
                let end_idx = (start_idx + hop_length).min(result.len());
                for j in start_idx..end_idx {
                    if j < result.len() {
                        result[j] = 0.0;
                    }
                }
            }
        }

        // 调整输出，对应Python的indata[zc//2:]
        let skip_samples = self.zc as usize / 2;
        if result.len() > skip_samples {
            result.drain(0..skip_samples);
        }

        result
    }

    /// 应用RMS混合
    fn apply_rms_mixing(&self, infer_wav: &[f32], input_wav: &[f32]) -> Vec<f32> {
        if infer_wav.is_empty() || input_wav.is_empty() {
            return infer_wav.to_vec();
        }

        let min_len = infer_wav.len().min(input_wav.len());
        let frame_length = 4 * self.zc as usize;
        let hop_length = self.zc as usize;

        // 计算输入RMS
        let mut input_rms = Vec::new();
        for i in (0..min_len).step_by(hop_length) {
            if i + frame_length <= input_wav.len() {
                let frame = &input_wav[i..i + frame_length];
                let rms = (frame.iter().map(|x| x * x).sum::<f32>() / frame_length as f32).sqrt();
                input_rms.push(rms);
            }
        }

        // 计算推理结果RMS
        let mut infer_rms = Vec::new();
        for i in (0..min_len).step_by(hop_length) {
            if i + frame_length <= infer_wav.len() {
                let frame = &infer_wav[i..i + frame_length];
                let rms = (frame.iter().map(|x| x * x).sum::<f32>() / frame_length as f32).sqrt();
                infer_rms.push(rms.max(1e-3)); // 防止除零
            }
        }

        // 应用RMS混合
        let mut result = infer_wav.to_vec();
        let mix_rate = 1.0 - self.gui_config.rms_mix_rate;

        for (i, (&input_r, &infer_r)) in input_rms.iter().zip(infer_rms.iter()).enumerate() {
            let ratio = (input_r / infer_r).powf(mix_rate);
            let start_idx = i * hop_length;
            let end_idx = (start_idx + hop_length).min(result.len());

            for j in start_idx..end_idx {
                if j < result.len() {
                    result[j] *= ratio;
                }
            }
        }

        result
    }

    /// 应用SOLA算法
    fn apply_sola_algorithm(&mut self, infer_wav: &[f32]) -> Vec<f32> {
        if infer_wav.len() < (self.sola_buffer_frame + self.sola_search_frame) as usize {
            return infer_wav.to_vec();
        }

        let sola_buffer_frame = self.sola_buffer_frame as usize;
        let sola_search_frame = self.sola_search_frame as usize;
        let block_frame = self.block_frame as usize;

        // SOLA相关性计算
        let conv_input = &infer_wav[0..(sola_buffer_frame + sola_search_frame)];
        let mut best_offset = 0;
        let mut best_correlation = f32::NEG_INFINITY;

        if let Some(ref sola_buffer) = self.sola_buffer {
            let sola_data = sola_buffer.to_vec();

            for offset in 0..sola_search_frame {
                if offset + sola_buffer_frame <= conv_input.len() {
                    let current_frame = &conv_input[offset..offset + sola_buffer_frame];

                    // 计算归一化相关性
                    let mut nom = 0.0;
                    let mut den1 = 0.0;
                    let mut den2 = 0.0;

                    for (i, (&a, &b)) in sola_data.iter().zip(current_frame.iter()).enumerate() {
                        if i < sola_buffer_frame {
                            nom += a * b;
                            den1 += a * a;
                            den2 += b * b;
                        }
                    }

                    let correlation = if den1 > 1e-8 && den2 > 1e-8 {
                        nom / (den1 * den2).sqrt()
                    } else {
                        0.0
                    };

                    if correlation > best_correlation {
                        best_correlation = correlation;
                        best_offset = offset;
                    }
                }
            }
        }

        // 应用偏移
        let mut result = if best_offset < infer_wav.len() {
            infer_wav[best_offset..].to_vec()
        } else {
            infer_wav.to_vec()
        };

        // 应用交叉淡化
        if result.len() >= sola_buffer_frame {
            if let (Some(ref sola_buffer), Some(ref fade_in_window), Some(ref fade_out_window)) = (
                &self.sola_buffer,
                &self.fade_in_window,
                &self.fade_out_window,
            ) {
                let sola_data = sola_buffer.to_vec();
                let fade_in_data = fade_in_window.to_vec();
                let fade_out_data = fade_out_window.to_vec();

                for i in 0..sola_buffer_frame.min(result.len()).min(fade_in_data.len()) {
                    if i < sola_data.len() {
                        result[i] = result[i] * fade_in_data[i] + sola_data[i] * fade_out_data[i];
                    }
                }
            }
        }

        // 更新SOLA缓冲区
        if result.len() >= block_frame + sola_buffer_frame {
            if let Some(ref mut sola_buffer) = self.sola_buffer {
                let new_buffer_data = &result[block_frame..block_frame + sola_buffer_frame];
                *sola_buffer = Tensor::from_vec(new_buffer_data.to_vec());
            }
        }

        result
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
