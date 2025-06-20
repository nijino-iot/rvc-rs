//! GUI管理器模块
//!
//! 对应 Python gui_v1.py 的 GUI 类，提供完整的事件处理和状态管理功能

use crate::config::GuiConfig;
use crate::error::{RvcError, RvcResult};
use crate::rtrvc::RVC;
use crate::sd::{self, printt, AudioStream};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tch::{Kind, Tensor};

/// 音频设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    /// 设备名称
    pub name: String,
    /// 设备索引
    pub index: usize,
    /// 主机API名称
    pub hostapi_name: String,
    /// 最大输入通道数
    pub max_input_channels: u16,
    /// 最大输出通道数
    pub max_output_channels: u16,
    /// 默认采样率
    pub default_samplerate: f64,
}

/// 应用状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AppState {
    /// 初始化状态
    Initializing,
    /// 就绪状态
    Ready,
    /// 转换中
    Converting,
    /// 错误状态
    Error(String),
    /// 停止状态
    Stopped,
}

/// 运行时统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStats {
    /// 算法延迟（毫秒）
    pub algorithm_latency_ms: f64,
    /// 推理时间（毫秒）
    pub inference_time_ms: f64,
    /// 缓冲区使用率
    pub buffer_usage: f32,
    /// CPU使用率
    pub cpu_usage: f32,
    /// GPU使用率（可选）
    pub gpu_usage: Option<f32>,
}

impl Default for RuntimeStats {
    fn default() -> Self {
        Self {
            algorithm_latency_ms: 0.0,
            inference_time_ms: 0.0,
            buffer_usage: 0.0,
            cpu_usage: 0.0,
            gpu_usage: None,
        }
    }
}

/// GUI管理器，对应Python中的GUI类
pub struct GuiManager {
    /// 配置管理器
    config_manager: crate::config::ConfigManager,
    /// RVC推理器
    model: Option<RVC>,
    /// 音频流管理器
    audio_stream: Option<AudioStream>,
    /// 音频处理器
    audio_processor: Option<Arc<Mutex<AudioProcessor>>>,
    /// 延迟时间
    delay_time: f64,
    /// 主机API列表
    hostapis: Vec<String>,
    /// 输入设备列表
    input_devices: Vec<String>,
    /// 输出设备列表
    output_devices: Vec<String>,
    /// 输入设备索引列表
    input_devices_indices: Vec<usize>,
    /// 输出设备索引列表
    output_devices_indices: Vec<usize>,
    /// 当前功能模式
    function: String,
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
    rms_buffer: Option<Tensor>,
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
}

impl GuiManager {
    /// 创建新的GUI管理器，对应Python GUI.__init__
    pub fn new(config_path: PathBuf) -> RvcResult<Self> {
        let mut config_manager = crate::config::ConfigManager::new(config_path);
        config_manager.load()?;

        let mut manager = Self {
            config_manager,
            model: None,
            audio_stream: None,
            audio_processor: None,
            delay_time: 0.0,
            hostapis: Vec::new(),
            input_devices: Vec::new(),
            output_devices: Vec::new(),
            input_devices_indices: Vec::new(),
            output_devices_indices: Vec::new(),
            function: "vc".to_string(),
            zc: 48000 / 1000,
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
            rms_buffer: None,
            sola_buffer: None,
            nr_buffer: None,
            output_buffer: None,
            fade_in_window: None,
            fade_out_window: None,
        };

        // 初始化设备信息
        manager.update_devices(None)?;

        Ok(manager)
    }

    /// 设置配置值，对应Python GUI.set_values
    pub fn set_values(&mut self, values: GuiConfig) -> RvcResult<bool> {
        // 验证路径
        let pth_path = values
            .pth_path
            .as_ref()
            .ok_or_else(|| RvcError::other("请选择pth文件"))?;
        if pth_path.is_empty() {
            return Err(RvcError::other("请选择pth文件"));
        }
        if !std::path::Path::new(pth_path).exists() {
            return Err(RvcError::other("pth文件不存在"));
        }

        // 检查路径中是否包含非ASCII字符
        if !pth_path.is_ascii() {
            return Err(RvcError::other("pth文件路径不可包含中文"));
        }
        if let Some(index_path) = &values.index_path {
            if !index_path.is_ascii() {
                return Err(RvcError::other("index文件路径不可包含中文"));
            }
        }

        // 克隆需要的字段以避免移动
        let input_device = values
            .sg_input_device
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");
        let output_device = values
            .sg_output_device
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("");

        // 设置设备
        self.set_devices(input_device, output_device)?;

        // 更新配置
        self.config_manager.update_gui_config(|config| {
            *config = values;
        })?;

        Ok(true)
    }

    /// 启动语音转换，对应Python GUI.start_vc
    pub fn start_vc(&mut self) -> RvcResult<()> {
        // 清空GPU缓存（如果使用GPU）
        if tch::Cuda::is_available() {
            // tch::Cuda::empty_cache(); // 注释掉因为方法不存在
        }

        // 1. 创建RVC推理器，对应Python的RVC初始化
        let gui_config = self.config_manager.gui_config();
        let config = self.config_manager.config();

        let device = if config.device.contains("cuda") {
            tch::Device::Cuda(0)
        } else {
            tch::Device::Cpu
        };

        self.model = Some(RVC::new(
            gui_config.pitch.unwrap_or(0),
            gui_config.formant.unwrap_or(0.0),
            gui_config.pth_path.as_ref().unwrap(),
            gui_config.index_path.as_ref().filter(|p| !p.is_empty()),
            gui_config.index_rate.unwrap_or(0.75),
            gui_config.n_cpu.unwrap_or(4) as i32,
            config.clone(),
            None,
        )?);

        // 2. 确定采样率和通道数
        let rvc_ref = self.model.as_ref().unwrap();
        let sample_rate = if gui_config
            .sr_type
            .as_ref()
            .map_or(false, |s| s == "sr_model")
        {
            rvc_ref.tgt_sr as u32
        } else {
            self.get_device_sample_rate(
                gui_config
                    .sg_output_device
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or(""),
            )
            .unwrap_or(48000.0) as u32
        };

        // 3. 计算各种帧大小参数，对应Python中的计算逻辑
        self.zc = sample_rate / 100; // 每10ms的采样数

        // 块帧数：对zc进行舍入处理
        self.block_frame = ((gui_config.block_time.unwrap_or(0.3) * sample_rate as f32
            / self.zc as f32)
            .round() as u32)
            * self.zc;

        // 16k采样率下的块帧数
        self.block_frame_16k = 160 * self.block_frame / self.zc;

        // 交叉淡化帧数
        self.crossfade_frame = ((gui_config.crossfade_time.unwrap_or(0.08) * sample_rate as f32
            / self.zc as f32)
            .round() as u32)
            * self.zc;

        // SOLA缓冲帧数，取交叉淡化帧数和4*zc的最小值
        self.sola_buffer_frame = self.crossfade_frame.min(4 * self.zc);
        self.sola_search_frame = self.zc;

        // 额外帧数
        self.extra_frame = ((gui_config.extra_time.unwrap_or(2.0) * sample_rate as f32
            / self.zc as f32)
            .round() as u32)
            * self.zc;

        // 4. 初始化张量缓冲区，对应Python中的张量初始化
        let total_input_length =
            self.extra_frame + self.crossfade_frame + self.sola_search_frame + self.block_frame;

        self.input_wav = Some(Tensor::zeros(
            &[total_input_length as i64],
            (Kind::Float, device),
        ));

        self.input_wav_denoise = Some(self.input_wav.as_ref().unwrap().copy());

        self.input_wav_res = Some(Tensor::zeros(
            &[(160 * total_input_length / self.zc) as i64],
            (Kind::Float, device),
        ));

        // 初始化RMS缓冲区
        self.rms_buffer = Some(Tensor::zeros(&[4 * self.zc as i64], (Kind::Float, device)));

        // 初始化SOLA缓冲区
        self.sola_buffer = Some(Tensor::zeros(
            &[self.sola_buffer_frame as i64],
            (Kind::Float, device),
        ));

        self.nr_buffer = Some(self.sola_buffer.as_ref().unwrap().copy());
        self.output_buffer = Some(self.input_wav.as_ref().unwrap().copy());

        // 5. 计算跳过和返回参数
        self.skip_head = self.extra_frame / self.zc;
        self.return_length =
            (self.block_frame + self.sola_buffer_frame + self.sola_search_frame) / self.zc;

        // 6. 初始化淡入淡出窗口
        let fade_window = Tensor::linspace(
            0.0,
            1.0,
            self.sola_buffer_frame as i64,
            (Kind::Float, device),
        );
        let fade_window = (fade_window * (std::f64::consts::PI / 2.0))
            .sin()
            .pow_tensor_scalar(2);

        self.fade_in_window = Some(fade_window.copy());
        self.fade_out_window = Some(Tensor::ones_like(&fade_window) - &fade_window);

        // 7. 启动音频流
        self.start_stream()?;

        Ok(())
    }

    /// 启动音频流，对应Python GUI.start_stream
    pub fn start_stream(&mut self) -> RvcResult<()> {
        let gui_config = self.config_manager.gui_config();

        // 确定采样率
        let sample_rate = if gui_config
            .sr_type
            .as_ref()
            .map_or(false, |s| s == "sr_model")
        {
            if let Some(rvc) = &self.model {
                rvc.tgt_sr as u32
            } else {
                48000
            }
        } else {
            48000
        };

        // 确定声道数
        let channels = self
            .get_device_channels(
                gui_config
                    .sg_output_device
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or(""),
            )
            .unwrap_or(2) as usize;

        // 确定块大小
        let block_size = self.block_frame as usize;

        // 检查是否使用WASAPI独占模式
        let exclusive_mode = gui_config
            .sg_hostapi
            .as_ref()
            .map_or(false, |api| api.contains("WASAPI"))
            && gui_config.sg_wasapi_exclusive.unwrap_or(false);

        log::info!(
            "Starting audio stream - Sample rate: {}, Channels: {}, Block size: {}, Exclusive: {}",
            sample_rate,
            channels,
            block_size,
            exclusive_mode
        );

        // 创建音频流配置
        let stream_config = sd::StreamConfig {
            sample_rate,
            channels,
            block_size,
            exclusive_mode,
        };

        // 创建音频处理器
        let audio_processor = Arc::new(Mutex::new(AudioProcessor::new(
            self.config_manager.gui_config().clone(),
            None, // TODO: 传递模型
            self.function.clone(),
            self.zc as i32,
            self.block_frame as i32,
            self.block_frame_16k as i32,
            self.crossfade_frame as i32,
            self.sola_buffer_frame as i32,
            self.sola_search_frame as i32,
            self.extra_frame as i32,
            self.skip_head as i32,
            self.return_length as i32,
        )?));

        let processor_clone = Arc::clone(&audio_processor);

        // 创建音频流
        let mut audio_stream = AudioStream::new(stream_config)?;

        // 设置音频回调函数
        let callback = Box::new(move |input: &[f32], output: &mut [f32]| {
            if let Ok(mut processor) = processor_clone.lock() {
                if let Err(e) = processor.audio_callback(input, output) {
                    log::error!("Audio callback error: {}", e);
                }
            }
        });

        audio_stream.set_callback(callback);

        // 启动流
        audio_stream.start()?;
        log::info!("Audio stream started successfully");

        self.audio_stream = Some(audio_stream);
        self.audio_processor = Some(audio_processor);

        Ok(())
    }

    /// 计算RMS值，对应Python的librosa.feature.rms
    fn compute_rms(&self, data: &[f32], frame_length: usize, hop_length: usize) -> Vec<f32> {
        let mut rms_values = Vec::new();

        if data.len() < frame_length {
            return rms_values;
        }

        let num_frames = (data.len() - frame_length) / hop_length + 1;

        for i in 0..num_frames {
            let start = i * hop_length;
            let end = (start + frame_length).min(data.len());

            if end <= start {
                break;
            }

            let frame = &data[start..end];
            let rms = (frame.iter().map(|x| x * x).sum::<f32>() / frame.len() as f32).sqrt();

            // 转换为分贝
            let rms_db = if rms > 0.0 {
                20.0 * rms.log10()
            } else {
                -60.0 // 静音阈值
            };

            rms_values.push(rms_db);
        }

        rms_values
    }

    /// 计算张量的RMS值
    fn compute_tensor_rms(
        &self,
        tensor: &Tensor,
        frame_length: usize,
        hop_length: usize,
    ) -> Tensor {
        let data: Vec<f32> = Vec::try_from(tensor.to(tch::Device::Cpu)).unwrap_or_default();

        if data.len() < frame_length {
            return Tensor::zeros(&[1], (tch::Kind::Float, tch::Device::Cpu));
        }

        let num_frames = (data.len() - frame_length) / hop_length + 1;
        let mut rms_values = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            let start = i * hop_length;
            let end = (start + frame_length).min(data.len());

            if end <= start {
                break;
            }

            let frame = &data[start..end];
            let rms = (frame.iter().map(|x| x * x).sum::<f32>() / frame.len() as f32).sqrt();
            rms_values.push(rms);
        }

        Tensor::from_slice(&rms_values).to(tensor.device())
    }

    /// 停止音频流，对应Python GUI.stop_stream
    pub fn stop_stream(&mut self) -> RvcResult<()> {
        if let Some(stream) = &mut self.audio_stream {
            stream.stop()?;
            log::info!("Audio stream stopped");
        }
        self.audio_stream = None;
        Ok(())
    }

    /// 更新音频设备列表，对应Python GUI.update_devices
    pub fn update_devices(&mut self, hostapi_name: Option<&str>) -> RvcResult<()> {
        // 使用sd模块获取主机API列表，对应Python的sd.query_hostapis()
        let hostapis = sd::query_hostapis();
        self.hostapis = hostapis.into_iter().map(|h| h.name).collect();

        // 选择要使用的主机API
        let selected_hostapi = hostapi_name.unwrap_or(&self.hostapis[0]);

        // 获取指定主机API的设备列表，对应Python的设备过滤逻辑
        let (input_names, input_indices, output_names, output_indices) =
            sd::get_devices_for_hostapi(selected_hostapi)?;

        self.input_devices = input_names;
        self.input_devices_indices = input_indices
            .into_iter()
            .map(|s| s.parse().unwrap_or(0))
            .collect();

        self.output_devices = output_names;
        self.output_devices_indices = output_indices
            .into_iter()
            .map(|s| s.parse().unwrap_or(0))
            .collect();

        Ok(())
    }

    /// 设置音频设备，对应Python GUI.set_devices
    pub fn set_devices(&mut self, input_device: &str, output_device: &str) -> RvcResult<()> {
        // 验证设备是否存在
        let input_idx = self
            .input_devices
            .iter()
            .position(|d| d == input_device)
            .ok_or_else(|| RvcError::other("输入设备不存在"))?;
        let output_idx = self
            .output_devices
            .iter()
            .position(|d| d == output_device)
            .ok_or_else(|| RvcError::other("输出设备不存在"))?;

        // 获取设备索引，对应Python的sd.default.device[0]和sd.default.device[1]
        let input_device_idx = self.input_devices_indices[input_idx];
        let output_device_idx = self.output_devices_indices[output_idx];

        // 设置默认设备，对应Python的sd.default.device设置
        sd::set_default_device(Some(input_device_idx), Some(output_device_idx));

        // 使用printt输出，对应Python的printt调用
        printt(
            "Input device: %0:%1",
            &[&input_device_idx.to_string(), input_device],
        );
        printt(
            "Output device: %0:%1",
            &[&output_device_idx.to_string(), output_device],
        );

        Ok(())
    }

    /// 获取设备采样率，对应Python GUI.get_device_samplerate
    pub fn get_device_sample_rate(&self, device_name: &str) -> RvcResult<f64> {
        // 对应Python的sd.query_devices(device=sd.default.device[0])["default_samplerate"]
        sd::get_device_default_sample_rate(device_name, true)
    }

    /// 获取设备通道数，对应Python GUI.get_device_channels
    pub fn get_device_channels(&self, device_name: &str) -> RvcResult<u32> {
        // 对应Python的获取max_input_channels和max_output_channels然后取最小值
        let max_input_channels = sd::get_device_max_channels(device_name, true)?;
        let max_output_channels = sd::get_device_max_channels(device_name, false)?;

        // 返回输入输出通道数和2的最小值，对应Python的min(max_input_channels, max_output_channels, 2)
        Ok(max_input_channels.min(max_output_channels).min(2))
    }

    /// 转换为单声道
    fn to_mono(&self, input: &[f32]) -> Vec<f32> {
        let channels = 2; // 简化为立体声
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

    // ========== 为 Tauri 添加的方法 ==========

    /// 获取主机API列表
    pub fn get_hostapis(&self) -> Vec<String> {
        self.hostapis.clone()
    }

    /// 获取输入设备列表
    pub fn get_input_devices(&self, host_api: Option<&str>) -> Vec<AudioDeviceInfo> {
        // 如果指定了主机API，则重新更新设备列表
        if let Some(api) = host_api {
            // 这里简化实现，实际应该根据主机API过滤
            self.input_devices
                .iter()
                .enumerate()
                .map(|(i, name)| AudioDeviceInfo {
                    name: name.clone(),
                    index: i,
                    hostapi_name: api.to_string(),
                    max_input_channels: 2,
                    max_output_channels: 0,
                    default_samplerate: 48000.0,
                })
                .collect()
        } else {
            self.input_devices
                .iter()
                .enumerate()
                .map(|(i, name)| AudioDeviceInfo {
                    name: name.clone(),
                    index: i,
                    hostapi_name: "default".to_string(),
                    max_input_channels: 2,
                    max_output_channels: 0,
                    default_samplerate: 48000.0,
                })
                .collect()
        }
    }

    /// 获取输出设备列表
    pub fn get_output_devices(&self, host_api: Option<&str>) -> Vec<AudioDeviceInfo> {
        // 如果指定了主机API，则重新更新设备列表
        if let Some(api) = host_api {
            self.output_devices
                .iter()
                .enumerate()
                .map(|(i, name)| AudioDeviceInfo {
                    name: name.clone(),
                    index: i,
                    hostapi_name: api.to_string(),
                    max_input_channels: 0,
                    max_output_channels: 2,
                    default_samplerate: 48000.0,
                })
                .collect()
        } else {
            self.output_devices
                .iter()
                .enumerate()
                .map(|(i, name)| AudioDeviceInfo {
                    name: name.clone(),
                    index: i,
                    hostapi_name: "default".to_string(),
                    max_input_channels: 0,
                    max_output_channels: 2,
                    default_samplerate: 48000.0,
                })
                .collect()
        }
    }

    /// 异步更新音频设备
    pub async fn update_audio_devices(&mut self, host_api: Option<&str>) -> RvcResult<()> {
        self.update_devices(host_api)
    }

    /// 获取设备采样率（简化实现）
    pub fn get_device_sample_rate_simple(&self, _device_name: &str) -> Option<f64> {
        // 简化实现，返回默认采样率
        Some(48000.0)
    }

    /// 获取设备通道数（简化实现）
    pub fn get_device_channels_simple(&self, _device_name: &str, _is_input: bool) -> Option<u32> {
        // 简化实现，返回立体声
        Some(2)
    }

    /// 验证配置
    pub fn validate_config(&self) -> RvcResult<()> {
        // 简化实现，总是返回成功
        Ok(())
    }

    /// 异步启动语音转换
    pub async fn start_vc_async(&mut self) -> RvcResult<()> {
        self.start_vc()
    }

    /// 异步停止语音转换
    pub async fn stop_voice_conversion(&mut self) -> RvcResult<()> {
        // 停止音频流
        if let Some(_stream) = &mut self.audio_stream {
            // 停止流
        }
        self.audio_stream = None;
        Ok(())
    }

    /// 更新实时参数
    pub fn update_realtime_parameter(
        &mut self,
        name: &str,
        value: serde_json::Value,
    ) -> RvcResult<()> {
        self.config_manager.update_gui_config(|config| {
            // 使用统一的字段更新方法
            let errors = config.update_field_from_json(name, value.clone());
            if !errors.is_empty() {
                log::warn!(
                    "实时参数更新验证警告: {} = {:?}, 错误: {:?}",
                    name,
                    value,
                    errors
                );
                // 对于实时参数更新，我们记录警告但不阻止操作
            }
        })
    }

    /// 获取运行时统计信息
    pub fn get_stats(&self) -> RuntimeStats {
        RuntimeStats::default()
    }

    /// 获取应用状态
    pub fn get_state(&self) -> AppState {
        if self.audio_stream.is_some() {
            AppState::Converting
        } else {
            AppState::Ready
        }
    }

    /// 异步初始化
    pub async fn initialize(&mut self) -> RvcResult<()> {
        Ok(())
    }
}

/// 音频处理器，用于处理实时音频回调
pub struct AudioProcessor {
    gui_config: GuiConfig,
    model: Option<RVC>,
    function: String,
    zc: i32,
    block_frame: i32,
    block_frame_16k: i32,
    crossfade_frame: i32,
    sola_buffer_frame: i32,
    sola_search_frame: i32,
    extra_frame: i32,
    skip_head: i32,
    return_length: i32,
    input_wav: Option<Tensor>,
    input_wav_denoise: Option<Tensor>,
    input_wav_res: Option<Tensor>,
    rms_buffer: Option<Tensor>,
    sola_buffer: Option<Tensor>,
    nr_buffer: Option<Tensor>,
    output_buffer: Option<Tensor>,
    fade_in_window: Option<Tensor>,
    fade_out_window: Option<Tensor>,
}

impl AudioProcessor {
    pub fn new(
        gui_config: GuiConfig,
        model: Option<RVC>,
        function: String,
        zc: i32,
        block_frame: i32,
        block_frame_16k: i32,
        crossfade_frame: i32,
        sola_buffer_frame: i32,
        sola_search_frame: i32,
        extra_frame: i32,
        skip_head: i32,
        return_length: i32,
    ) -> RvcResult<Self> {
        let device = tch::Device::Cpu; // TODO: 从配置中获取设备

        // 初始化音频缓冲区
        let total_wav_len = (48000.0 * 3.0) as i64;
        let input_wav = Some(Tensor::zeros(&[total_wav_len], (tch::Kind::Float, device)));

        let total_wav_res_len = (16000.0 * 3.0) as i64;
        let input_wav_res = Some(Tensor::zeros(
            &[total_wav_res_len],
            (tch::Kind::Float, device),
        ));

        // 初始化其他缓冲区
        let rms_buffer = Some(Tensor::zeros(&[4 * zc as i64], (tch::Kind::Float, device)));
        let sola_buffer = Some(Tensor::zeros(
            &[sola_buffer_frame as i64],
            (tch::Kind::Float, device),
        ));
        let nr_buffer = Some(Tensor::zeros(
            &[sola_buffer_frame as i64],
            (tch::Kind::Float, device),
        ));
        let output_buffer = Some(Tensor::zeros(&[total_wav_len], (tch::Kind::Float, device)));

        // 创建淡入淡出窗口
        let fade_in_window = Some(Self::create_fade_window(
            sola_buffer_frame as usize,
            true,
            device,
        ));
        let fade_out_window = Some(Self::create_fade_window(
            sola_buffer_frame as usize,
            false,
            device,
        ));

        Ok(Self {
            gui_config,
            model,
            function,
            zc,
            block_frame,
            block_frame_16k,
            crossfade_frame,
            sola_buffer_frame,
            sola_search_frame,
            extra_frame,
            skip_head,
            return_length,
            input_wav,
            input_wav_denoise: None,
            input_wav_res,
            rms_buffer,
            sola_buffer,
            nr_buffer,
            output_buffer,
            fade_in_window,
            fade_out_window,
        })
    }

    fn create_fade_window(size: usize, fade_in: bool, device: tch::Device) -> Tensor {
        let mut window = vec![0.0f32; size];
        for i in 0..size {
            let t = i as f32 / size as f32;
            window[i] = if fade_in {
                t // 淡入：从0到1
            } else {
                1.0 - t // 淡出：从1到0
            };
        }
        Tensor::from_slice(&window).to(device)
    }

    /// 音频回调函数，对应Python GUI.audio_callback
    pub fn audio_callback(&mut self, indata: &[f32], outdata: &mut [f32]) -> RvcResult<()> {
        let start_time = Instant::now();

        // 1. 转换为单声道，对应Python的librosa.to_mono
        let mut mono_input = self.to_mono(indata);

        let gui_config = self.gui_config.clone();
        let device = tch::Device::Cpu; // TODO: 从配置中获取设备
        let zc = self.zc;

        // 2. 应用阈值门控（如果启用）
        if gui_config.threshold.unwrap_or(-60.0) > -60.0 {
            // 更新RMS缓冲区
            if let Some(rms_buffer) = &mut self.rms_buffer {
                // 将新数据添加到RMS缓冲区
                let combined_data = [
                    Vec::<f32>::try_from(rms_buffer.to(tch::Device::Cpu)).unwrap_or_default(),
                    mono_input.clone(),
                ]
                .concat();

                // 计算RMS
                let frame_length = 4 * zc as usize;
                let hop_length = zc as usize;
                let rms_values = Self::compute_rms_static(&combined_data, frame_length, hop_length);

                // 更新RMS缓冲区
                let buffer_size = 4 * zc as usize;
                if combined_data.len() >= buffer_size {
                    let new_buffer = &combined_data[combined_data.len() - buffer_size..];
                    *rms_buffer = Tensor::from_slice(new_buffer).to(device);
                }

                // 应用阈值门控
                let threshold_db = gui_config.threshold.unwrap_or(-60.0);
                let start_idx = 2 * zc as usize - zc as usize / 2;
                if combined_data.len() > start_idx {
                    mono_input = combined_data[start_idx..].to_vec();

                    // 对每个窗口应用阈值
                    for (i, &rms_db) in rms_values.iter().enumerate() {
                        if rms_db < threshold_db {
                            let start = i * hop_length;
                            let end = (start + hop_length).min(mono_input.len());
                            for j in start..end {
                                mono_input[j] = 0.0;
                            }
                        }
                    }

                    // 移除前半部分
                    if mono_input.len() > zc as usize / 2 {
                        mono_input = mono_input[zc as usize / 2..].to_vec();
                    }
                }
            }
        }

        // 3. 更新输入缓冲区
        if let Some(input_wav) = &mut self.input_wav {
            // 滑动窗口：移动现有数据
            let block_size = self.block_frame as usize;
            let total_size = input_wav.size()[0] as usize;

            if total_size > block_size {
                let shifted_data =
                    input_wav.narrow(0, block_size as i64, (total_size - block_size) as i64);
                let _ = input_wav
                    .narrow(0, 0, (total_size - block_size) as i64)
                    .copy_(&shifted_data);
            }

            // 添加新数据到末尾
            let new_data = Tensor::from_slice(&mono_input).to(device);
            let start_idx = (total_size - mono_input.len()).max(0);
            let _ = input_wav
                .narrow(0, start_idx as i64, mono_input.len() as i64)
                .copy_(&new_data);
        }

        // 4. 更新16kHz重采样缓冲区
        if let Some(input_wav_res) = &mut self.input_wav_res {
            let block_size_16k = self.block_frame_16k as usize;
            let total_size = input_wav_res.size()[0] as usize;

            if total_size > block_size_16k {
                let shifted_data = input_wav_res.narrow(
                    0,
                    block_size_16k as i64,
                    (total_size - block_size_16k) as i64,
                );
                let _ = input_wav_res
                    .narrow(0, 0, (total_size - block_size_16k) as i64)
                    .copy_(&shifted_data);
            }

            // 重采样新数据到16kHz
            if let Some(input_wav) = &self.input_wav {
                let resample_input_size = mono_input.len() + 2 * self.zc as usize;
                let resample_input = input_wav.narrow(
                    0,
                    (input_wav.size()[0] - resample_input_size as i64).max(0),
                    resample_input_size as i64,
                );

                // TODO: 实现重采样器
                // let resampled = self.resampler.resample(&resample_input)?;
                // 暂时使用简单的下采样
                let resampled_data: Vec<f32> =
                    Vec::try_from(resample_input.to(tch::Device::Cpu)).unwrap_or_default();
                let downsampled: Vec<f32> = resampled_data.iter().step_by(3).cloned().collect(); // 48kHz -> 16kHz 大约是3:1

                let new_data = Tensor::from_slice(&downsampled).to(device);
                let start_idx = (total_size - downsampled.len()).max(0);
                if start_idx < total_size {
                    let _ = input_wav_res
                        .narrow(0, start_idx as i64, downsampled.len() as i64)
                        .copy_(&new_data);
                }
            }
        }

        // 5. 噪声抑制处理（如果启用）
        let processed_wav = if gui_config.i_noise_reduce.unwrap_or(false) && self.function == "vc" {
            // TODO: 实现噪声抑制
            if let Some(input_wav) = &self.input_wav {
                input_wav.narrow(0, self.extra_frame as i64, self.block_frame as i64)
            } else {
                return Err(RvcError::other("Input buffer not initialized"));
            }
        } else {
            if let Some(input_wav) = &self.input_wav {
                input_wav.narrow(0, self.extra_frame as i64, self.block_frame as i64)
            } else {
                return Err(RvcError::other("Input buffer not initialized"));
            }
        };

        // 6. 执行推理
        let mut infer_wav = if self.function == "vc" {
            if let (Some(rvc), Some(input_wav_res)) = (&mut self.model, &self.input_wav_res) {
                use crate::f0::F0Method;
                let f0_method = match gui_config
                    .f0method
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("harvest")
                {
                    "harvest" => F0Method::Harvest,
                    "pm" => F0Method::Pm,
                    "crepe" => F0Method::Crepe,
                    "rmvpe" => F0Method::Rmvpe,
                    "fcpe" => F0Method::Fcpe,
                    _ => F0Method::Harvest,
                };

                rvc.infer(
                    input_wav_res,
                    self.block_frame_16k as usize,
                    self.skip_head as usize,
                    self.return_length as usize,
                    f0_method,
                )?
            } else {
                return Err(RvcError::other("RVC not initialized"));
            }
        } else {
            processed_wav
        };

        // 7. 输出噪声抑制（如果启用）
        if gui_config.o_noise_reduce.unwrap_or(false) && self.function == "vc" {
            if let Some(output_buffer) = &mut self.output_buffer {
                // 更新输出缓冲区
                let block_size = self.block_frame as usize;
                let total_size = output_buffer.size()[0] as usize;

                if total_size > block_size {
                    let shifted_data = output_buffer.narrow(
                        0,
                        block_size as i64,
                        (total_size - block_size) as i64,
                    );
                    let _ = output_buffer
                        .narrow(0, 0, (total_size - block_size) as i64)
                        .copy_(&shifted_data);
                }

                let infer_block = infer_wav.narrow(
                    0,
                    (infer_wav.size()[0] - block_size as i64).max(0),
                    block_size as i64,
                );
                let _ = output_buffer
                    .narrow(0, (total_size - block_size) as i64, block_size as i64)
                    .copy_(&infer_block);

                // TODO: 应用噪声抑制
                // infer_wav = self.tg.forward(&infer_wav.unsqueeze(0), &output_buffer.unsqueeze(0)).squeeze(0);
            }
        }

        // 8. RMS混合（如果启用）
        // TODO: 暂时注释掉，避免编译错误
        // if gui_config.rms_mix_rate.unwrap_or(1.0) < 1.0 && self.function == "vc" {
        //     // RMS混合逻辑待实现
        // }

        // 9. SOLA算法处理
        if let Some(sola_buffer) = self.sola_buffer.as_ref() {
            let sola_buffer_frame = self.sola_buffer_frame as usize;
            let sola_search_frame = self.sola_search_frame as usize;

            // 计算相关性
            let _conv_input =
                infer_wav.narrow(0, 0, (sola_buffer_frame + sola_search_frame) as i64);

            // TODO: 实现卷积相关性计算
            // 暂时使用简单的SOLA处理
            let sola_offset = 0; // 暂时设为0

            // 基于SOLA偏移调整推理结果
            let infer_wav_shifted =
                infer_wav.narrow(0, sola_offset, infer_wav.size()[0] - sola_offset as i64);

            // 应用淡入淡出窗口 (对应Python的SOLA算法)
            if let (Some(fade_in), Some(fade_out)) = (&self.fade_in_window, &self.fade_out_window) {
                // 应用淡入淡出混合
                let output_section = infer_wav_shifted.narrow(0, 0, sola_buffer_frame as i64);
                let blended_section = &output_section * fade_in + sola_buffer * fade_out;

                // 将混合结果写回到infer_wav_shifted的前面部分
                let _ = infer_wav_shifted
                    .narrow(0, 0, sola_buffer_frame as i64)
                    .copy_(&blended_section);

                // 更新SOLA缓冲区 (对应Python的self.sola_buffer[:] = infer_wav[...])
                let block_start = self.block_frame as i64;
                let block_end = block_start + sola_buffer_frame as i64;
                if infer_wav_shifted.size()[0] > block_end {
                    let new_sola_buffer =
                        infer_wav_shifted.narrow(0, block_start, sola_buffer_frame as i64);
                    if let Some(ref mut sola_buf) = self.sola_buffer {
                        let _ = sola_buf.copy_(&new_sola_buffer);
                    }
                }

                infer_wav = infer_wav_shifted;
            }
        }

        // 10. 转换为输出格式并写入输出缓冲区
        let output_samples = infer_wav.narrow(0, 0, self.block_frame as i64);
        let output_cpu = output_samples.to(tch::Device::Cpu);
        let output_data: Vec<f64> = Vec::try_from(output_cpu).unwrap_or_default();
        let output_data: Vec<f32> = output_data.into_iter().map(|x| x as f32).collect();

        // 扩展到多声道
        let channels = 2; // 默认立体声
        for (i, &sample) in output_data.iter().enumerate() {
            for ch in 0..channels {
                if i * channels + ch < outdata.len() {
                    outdata[i * channels + ch] = sample;
                }
            }
        }

        // 记录推理时间
        let inference_time = start_time.elapsed().as_millis();
        log::debug!("Inference time: {}ms", inference_time);

        Ok(())
    }

    /// 计算RMS值，对应Python的librosa.feature.rms
    fn compute_rms(&self, data: &[f32], frame_length: usize, hop_length: usize) -> Vec<f32> {
        let mut rms_values = Vec::new();

        if data.len() < frame_length {
            return rms_values;
        }

        let num_frames = (data.len() - frame_length) / hop_length + 1;

        for i in 0..num_frames {
            let start = i * hop_length;
            let end = (start + frame_length).min(data.len());

            if end <= start {
                break;
            }

            let frame = &data[start..end];
            let rms = (frame.iter().map(|x| x * x).sum::<f32>() / frame.len() as f32).sqrt();

            // 转换为分贝
            let rms_db = if rms > 0.0 {
                20.0 * rms.log10()
            } else {
                -60.0 // 静音阈值
            };

            rms_values.push(rms_db);
        }

        rms_values
    }

    /// 计算张量的RMS值
    fn compute_tensor_rms(
        &self,
        tensor: &Tensor,
        frame_length: usize,
        hop_length: usize,
    ) -> Tensor {
        let data: Vec<f32> = Vec::try_from(tensor.to(tch::Device::Cpu)).unwrap_or_default();

        if data.len() < frame_length {
            return Tensor::zeros(&[1], (tch::Kind::Float, tch::Device::Cpu));
        }

        let num_frames = (data.len() - frame_length) / hop_length + 1;
        let mut rms_values = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            let start = i * hop_length;
            let end = (start + frame_length).min(data.len());

            if end <= start {
                break;
            }

            let frame = &data[start..end];
            let rms = (frame.iter().map(|x| x * x).sum::<f32>() / frame.len() as f32).sqrt();
            rms_values.push(rms);
        }

        Tensor::from_slice(&rms_values).to(tensor.device())
    }

    /// 转换为单声道，对应Python的librosa.to_mono
    fn to_mono(&self, data: &[f32]) -> Vec<f32> {
        let channels = 2; // 默认立体声

        if channels == 1 {
            return data.to_vec();
        }

        let mut mono = Vec::new();
        for chunk in data.chunks_exact(channels) {
            let sum: f32 = chunk.iter().sum();
            mono.push(sum / channels as f32);
        }

        mono
    }

    /// 静态版本的计算RMS值，用于避免借用冲突
    fn compute_rms_static(data: &[f32], frame_length: usize, hop_length: usize) -> Vec<f32> {
        let mut rms_values = Vec::new();

        if data.len() < frame_length {
            return rms_values;
        }

        let num_frames = (data.len() - frame_length) / hop_length + 1;

        for i in 0..num_frames {
            let start = i * hop_length;
            let end = (start + frame_length).min(data.len());

            if end <= start {
                break;
            }

            let frame = &data[start..end];
            let rms = (frame.iter().map(|x| x * x).sum::<f32>() / frame.len() as f32).sqrt();

            // 转换为分贝
            let rms_db = if rms > 0.0 {
                20.0 * rms.log10()
            } else {
                -60.0 // 静音阈值
            };

            rms_values.push(rms_db);
        }

        rms_values
    }
}
