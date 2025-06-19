//! GUI管理器模块
//!
//! 对应 Python gui_v1.py 的 GUI 类，提供完整的事件处理和状态管理功能

use crate::config::{Config, GuiConfig};
use crate::error::{RvcError, RvcResult};
use crate::rvc_model::RvcRealtimeModel;
use crate::sd::{self, printt, AudioStream};
use std::time::Instant;
use tch::{Device, Kind, Tensor};

/// GUI管理器，对应Python中的GUI类
pub struct GuiManager {
    /// GUI配置
    gui_config: GuiConfig,
    /// 核心配置
    config: Config,
    /// RVC推理器
    rvc: Option<RvcRealtimeModel>,
    /// 音频流管理器
    audio_stream: Option<AudioStream>,
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
}

impl GuiManager {
    /// 创建新的GUI管理器，对应Python GUI.__init__
    pub fn new() -> RvcResult<Self> {
        let gui_config = GuiConfig::default();
        let config = Config::default();

        let mut manager = Self {
            gui_config,
            config,
            rvc: None,
            audio_stream: None,
            delay_time: 0.0,
            hostapis: Vec::new(),
            input_devices: Vec::new(),
            output_devices: Vec::new(),
            input_devices_indices: Vec::new(),
            output_devices_indices: Vec::new(),
            function: "vc".to_string(),
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
        };

        // 初始化设备信息
        manager.update_devices(None)?;

        Ok(manager)
    }

    /// 设置配置值，对应Python GUI.set_values
    pub fn set_values(&mut self, values: GuiConfig) -> RvcResult<bool> {
        // 验证路径
        if values.pth_path.is_empty() {
            return Err(RvcError::other("请选择pth文件"));
        }
        if values.index_path.is_empty() {
            return Err(RvcError::other("请选择index文件"));
        }

        // 检查路径中是否包含非ASCII字符
        if !values.pth_path.is_ascii() {
            return Err(RvcError::other("pth文件路径不可包含中文"));
        }
        if !values.index_path.is_ascii() {
            return Err(RvcError::other("index文件路径不可包含中文"));
        }

        // 设置设备
        self.set_devices(&values.sg_input_device, &values.sg_output_device)?;

        // 更新配置
        self.gui_config = values;

        Ok(true)
    }

    /// 启动语音转换，对应Python GUI.start_vc
    pub fn start_vc(&mut self) -> RvcResult<()> {
        // 清空GPU缓存（如果使用GPU）
        if tch::Cuda::is_available() {
            // tch::Cuda::empty_cache(); // 注释掉因为方法不存在
        }

        // 1. 创建RVC推理器，对应Python的RVC初始化
        let device = if self.config.device.contains("cuda") {
            Device::Cuda(0)
        } else {
            Device::Cpu
        };

        self.rvc = Some(RvcRealtimeModel::new(
            std::path::Path::new(&self.gui_config.pth_path),
            if self.gui_config.index_path.is_empty() {
                None
            } else {
                Some(std::path::Path::new(&self.gui_config.index_path))
            },
            self.gui_config.pitch as i64,
            self.gui_config.index_rate as f64,
            device,
            self.config.is_half,
        )?);

        // 2. 确定采样率和通道数
        let rvc_ref = self.rvc.as_ref().unwrap();
        let sample_rate = if self.gui_config.sr_type == "sr_model" {
            rvc_ref.target_sample_rate() as u32
        } else {
            self.get_device_sample_rate(&self.gui_config.sg_output_device)
                .unwrap_or(48000.0) as u32
        };

        let channels = self
            .get_device_channels(&self.gui_config.sg_output_device)
            .unwrap_or(2) as u32;

        // 3. 计算各种帧大小参数，对应Python中的计算逻辑
        self.zc = sample_rate / 100; // 每10ms的采样数

        // 块帧数：对zc进行舍入处理
        self.block_frame = ((self.gui_config.block_time * sample_rate as f32 / self.zc as f32)
            .round() as u32)
            * self.zc;

        // 16k采样率下的块帧数
        self.block_frame_16k = 160 * self.block_frame / self.zc;

        // 交叉淡化帧数
        self.crossfade_frame =
            ((self.gui_config.crossfade_time * sample_rate as f32 / self.zc as f32).round() as u32)
                * self.zc;

        // SOLA缓冲帧数，取交叉淡化帧数和4*zc的最小值
        self.sola_buffer_frame = self.crossfade_frame.min(4 * self.zc);
        self.sola_search_frame = self.zc;

        // 额外帧数
        self.extra_frame = ((self.gui_config.extra_time * sample_rate as f32 / self.zc as f32)
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
        self.rms_buffer = vec![0.0; (4 * self.zc) as usize];

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
        let sample_rate = if self.gui_config.sr_type == "sr_model" {
            if let Some(rvc) = &self.rvc {
                rvc.target_sample_rate() as u32
            } else {
                48000
            }
        } else {
            48000 // 默认采样率
        };

        // 创建音频回调函数
        let callback = Box::new(move |input: &[f32], output: &mut [f32]| {
            // 这里是音频处理回调的简化实现
            // 实际实现需要调用audio_callback方法
            for (i, sample) in output.iter_mut().enumerate() {
                *sample = if i < input.len() { input[i] } else { 0.0 };
            }
        });

        // 创建音频流
        self.audio_stream = Some(AudioStream::new(
            sample_rate,
            self.block_frame as usize,
            2,
            callback,
        )?);

        if let Some(stream) = &mut self.audio_stream {
            // 启动流
            stream.start()?;
        }

        Ok(())
    }

    /// 停止音频流，对应Python GUI.stop_stream
    pub fn stop_stream(&mut self) -> RvcResult<()> {
        if let Some(mut stream) = self.audio_stream.take() {
            stream.stop()?;
        }
        Ok(())
    }

    /// 音频回调函数，对应Python GUI.audio_callback
    fn audio_callback(&mut self, indata: &[f32], outdata: &mut [f32]) -> RvcResult<()> {
        let start_time = Instant::now();

        // 1. 转换为单声道，对应Python的librosa.to_mono
        let mono_input = self.to_mono(indata);

        // 2. 应用阈值门控（如果启用）
        let processed_input = if self.gui_config.threshold > -60.0 {
            self.apply_threshold_gate(&mono_input)?
        } else {
            mono_input
        };

        // 3. 更新输入缓冲区
        if let Some(input_wav) = &mut self.input_wav {
            // 移动现有数据
            let block_size = self.block_frame as usize;
            let total_size = input_wav.size()[0] as usize;

            if total_size > block_size {
                let moved_data =
                    input_wav.narrow(0, block_size as i64, (total_size - block_size) as i64);
                let _ = input_wav
                    .narrow(0, 0, (total_size - block_size) as i64)
                    .copy_(&moved_data);
            }

            // 添加新数据
            let new_data = Tensor::from_slice(&processed_input).to_device(input_wav.device());
            let start_idx = total_size - processed_input.len();
            let _ = input_wav
                .narrow(0, start_idx as i64, processed_input.len() as i64)
                .copy_(&new_data);
        }

        // 4. 执行推理
        let infer_result = if self.function == "vc" {
            if let (Some(rvc), Some(input_wav_res)) = (&mut self.rvc, &self.input_wav_res) {
                rvc.infer(
                    input_wav_res,
                    self.block_frame_16k as i64,
                    self.skip_head as i64,
                    self.return_length as i64,
                    &self.gui_config.f0method,
                )?
            } else {
                return Err(RvcError::other("RVC not initialized"));
            }
        } else {
            // 直通模式
            if let Some(input_wav) = &self.input_wav {
                input_wav.narrow(0, self.extra_frame as i64, self.block_frame as i64)
            } else {
                return Err(RvcError::other("Input buffer not initialized"));
            }
        };

        // 5. 转换为输出格式并写入输出缓冲区
        let output_samples = infer_result.narrow(0, 0, self.block_frame as i64);
        let output_cpu = output_samples.to(tch::Device::Cpu);
        let output_data: Vec<f64> = Vec::try_from(output_cpu).unwrap_or_default();
        let output_data: Vec<f32> = output_data.into_iter().map(|x| x as f32).collect();

        // 扩展到多声道
        let channels = 2; // 简化为立体声
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
}
