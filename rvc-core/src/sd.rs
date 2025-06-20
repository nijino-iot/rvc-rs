//! Sounddevice模块
//!
//! 使用cpal实现Python sounddevice库的功能，提供音频设备管理和音频流处理

use crate::error::{RvcError, RvcResult};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream};
use std::sync::{Arc, Mutex};

/// 音频回调函数类型，对应Python sounddevice的callback
pub type AudioCallback = Box<dyn FnMut(&[f32], &mut [f32]) + Send + 'static>;

/// 音频流配置，对应Python sounddevice的Stream参数
#[derive(Debug, Clone)]
pub struct StreamConfig {
    pub sample_rate: u32,
    pub channels: usize,
    pub block_size: usize,
    pub exclusive_mode: bool,
}

/// 转换为cpal的StreamConfig
impl StreamConfig {
    pub fn to_cpal_config(&self) -> cpal::StreamConfig {
        cpal::StreamConfig {
            channels: self.channels as u16,
            sample_rate: cpal::SampleRate(self.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.block_size as u32),
        }
    }
}

/// 音频设备信息，对应Python sounddevice的device info
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub index: usize,
    pub hostapi_name: String,
    pub max_input_channels: u32,
    pub max_output_channels: u32,
    pub default_samplerate: f64,
}

/// 主机API信息
#[derive(Debug, Clone)]
pub struct HostApiInfo {
    pub name: String,
    pub device_count: usize,
}

/// 音频流状态
#[derive(Debug, Clone, PartialEq)]
pub enum StreamState {
    Stopped,
    Running,
    Error,
}

/// 音频流管理器，对应Python sounddevice的Stream
pub struct AudioStream {
    input_stream: Option<Stream>,
    output_stream: Option<Stream>,
    state: Arc<Mutex<StreamState>>,
    config: crate::sd::StreamConfig,
    callback: Option<AudioCallback>,
}

/// 默认设备设置，对应Python sounddevice.default
pub struct DefaultDevice {
    pub input_device: Option<usize>,
    pub output_device: Option<usize>,
}

impl Default for DefaultDevice {
    fn default() -> Self {
        Self {
            input_device: None,
            output_device: None,
        }
    }
}

/// 全局默认设备
static mut DEFAULT_DEVICE: DefaultDevice = DefaultDevice {
    input_device: None,
    output_device: None,
};

/// 打印函数，对应Python中的printt函数
pub fn printt(msg: &str, args: &[&str]) {
    if args.is_empty() {
        println!("{}", msg);
    } else {
        let mut formatted = msg.to_string();
        for (i, arg) in args.iter().enumerate() {
            formatted = formatted.replace(&format!("%{}", i), arg);
        }
        println!("{}", formatted);
    }
}

/// 初始化sounddevice，对应Python的sd._initialize()
pub fn initialize() -> RvcResult<()> {
    // cpal不需要显式初始化，这里只是为了API兼容性
    Ok(())
}

/// 终止sounddevice，对应Python的sd._terminate()
pub fn terminate() -> RvcResult<()> {
    // cpal不需要显式终止，这里只是为了API兼容性
    Ok(())
}

/// 查询主机API，对应Python的sd.query_hostapis()
pub fn query_hostapis() -> Vec<HostApiInfo> {
    cpal::available_hosts()
        .into_iter()
        .enumerate()
        .map(|(index, host_id)| {
            let host = cpal::host_from_id(host_id).unwrap_or_else(|_| cpal::default_host());
            let device_count = host
                .input_devices()
                .map(|devices| devices.count())
                .unwrap_or(0)
                + host
                    .output_devices()
                    .map(|devices| devices.count())
                    .unwrap_or(0);

            HostApiInfo {
                name: format!("{:?}", host_id),
                device_count,
            }
        })
        .collect()
}

/// 查询音频设备，对应Python的sd.query_devices()
pub fn query_devices() -> Vec<DeviceInfo> {
    let mut devices = Vec::new();
    let hostapis = query_hostapis();

    for hostapi in hostapis {
        let host = get_host_by_name(&hostapi.name).unwrap_or_else(|_| cpal::default_host());

        // 添加输入设备
        if let Ok(input_devices) = host.input_devices() {
            for (_index, device) in input_devices.enumerate() {
                if let Ok(name) = device.name() {
                    let (max_input_channels, default_samplerate) = get_device_info(&device, true);

                    devices.push(DeviceInfo {
                        name,
                        index: devices.len(),
                        hostapi_name: hostapi.name.clone(),
                        max_input_channels,
                        max_output_channels: 0,
                        default_samplerate,
                    });
                }
            }
        }

        // 添加输出设备
        if let Ok(output_devices) = host.output_devices() {
            for (_index, device) in output_devices.enumerate() {
                if let Ok(name) = device.name() {
                    let (max_output_channels, default_samplerate) = get_device_info(&device, false);

                    devices.push(DeviceInfo {
                        name,
                        index: devices.len(),
                        hostapi_name: hostapi.name.clone(),
                        max_input_channels: 0,
                        max_output_channels,
                        default_samplerate,
                    });
                }
            }
        }
    }

    devices
}

/// 查询指定设备信息，对应Python的sd.query_devices(device=device_id)
pub fn query_device(device_id: usize) -> RvcResult<DeviceInfo> {
    let devices = query_devices();
    devices
        .get(device_id)
        .cloned()
        .ok_or_else(|| RvcError::audio(format!("Device {} not found", device_id)))
}

/// 设置默认设备，对应Python的sd.default.device = [input, output]
pub fn set_default_device(input_device: Option<usize>, output_device: Option<usize>) {
    unsafe {
        DEFAULT_DEVICE.input_device = input_device;
        DEFAULT_DEVICE.output_device = output_device;
    }
}

/// 获取默认设备
pub fn get_default_device() -> (Option<usize>, Option<usize>) {
    unsafe { (DEFAULT_DEVICE.input_device, DEFAULT_DEVICE.output_device) }
}

/// 根据主机名获取主机
fn get_host_by_name(host_name: &str) -> RvcResult<Host> {
    let available_hosts = cpal::available_hosts();

    for host_id in available_hosts {
        let host_name_str = format!("{:?}", host_id);
        if host_name_str.contains(host_name) {
            return Ok(cpal::host_from_id(host_id).map_err(|e| {
                RvcError::audio(format!("Failed to get host {}: {}", host_name, e))
            })?);
        }
    }

    Ok(cpal::default_host())
}

/// 获取设备信息（通道数和采样率）
fn get_device_info(device: &Device, is_input: bool) -> (u32, f64) {
    let config = if is_input {
        device.default_input_config()
    } else {
        device.default_output_config()
    };

    match config {
        Ok(config) => (config.channels() as u32, config.sample_rate().0 as f64),
        Err(_) => (2, 48000.0), // 默认值
    }
}

/// 获取指定主机API的设备列表
pub fn get_devices_for_hostapi(
    hostapi_name: &str,
) -> RvcResult<(Vec<String>, Vec<String>, Vec<String>, Vec<String>)> {
    let host = get_host_by_name(hostapi_name)?;

    let mut input_names = Vec::new();
    let mut input_indices = Vec::new();
    let mut output_names = Vec::new();
    let mut output_indices = Vec::new();

    // 获取输入设备
    if let Ok(devices) = host.input_devices() {
        for (idx, device) in devices.enumerate() {
            if let Ok(device_name) = device.name() {
                if let Ok(configs) = device.supported_input_configs() {
                    if configs.count() > 0 {
                        input_names.push(device_name);
                        input_indices.push(idx.to_string());
                    }
                }
            }
        }
    }

    // 获取输出设备
    if let Ok(devices) = host.output_devices() {
        for (idx, device) in devices.enumerate() {
            if let Ok(device_name) = device.name() {
                if let Ok(configs) = device.supported_output_configs() {
                    if configs.count() > 0 {
                        output_names.push(device_name);
                        output_indices.push(idx.to_string());
                    }
                }
            }
        }
    }

    Ok((input_names, input_indices, output_names, output_indices))
}

/// 获取设备的默认采样率
pub fn get_device_default_sample_rate(device_name: &str, is_input: bool) -> RvcResult<f64> {
    let host = cpal::default_host();

    let devices = if is_input {
        host.input_devices()
            .map_err(|e| RvcError::audio(format!("Failed to get input devices: {}", e)))?
    } else {
        host.output_devices()
            .map_err(|e| RvcError::audio(format!("Failed to get output devices: {}", e)))?
    };

    for device in devices {
        if let Ok(name) = device.name() {
            if name == device_name {
                let config = if is_input {
                    device.default_input_config()
                } else {
                    device.default_output_config()
                };

                if let Ok(config) = config {
                    return Ok(config.sample_rate().0 as f64);
                }
            }
        }
    }

    Ok(48000.0)
}

/// 获取设备的最大通道数
pub fn get_device_max_channels(device_name: &str, is_input: bool) -> RvcResult<u32> {
    let host = cpal::default_host();

    let devices = if is_input {
        host.input_devices()
            .map_err(|e| RvcError::audio(format!("Failed to get input devices: {}", e)))?
    } else {
        host.output_devices()
            .map_err(|e| RvcError::audio(format!("Failed to get output devices: {}", e)))?
    };

    for device in devices {
        if let Ok(name) = device.name() {
            if name == device_name {
                let config = if is_input {
                    device.default_input_config()
                } else {
                    device.default_output_config()
                };

                if let Ok(config) = config {
                    return Ok(config.channels() as u32);
                }
            }
        }
    }

    Ok(2)
}

impl AudioStream {
    /// 创建新的音频流，对应Python的sd.Stream()
    pub fn new(config: StreamConfig) -> RvcResult<Self> {
        Ok(Self {
            input_stream: None,
            output_stream: None,
            state: Arc::new(Mutex::new(StreamState::Stopped)),
            config,
            callback: None,
        })
    }

    /// 设置音频回调函数
    pub fn set_callback(&mut self, callback: AudioCallback) {
        self.callback = Some(callback);
    }

    /// 启动音频流，对应Python的stream.start()
    pub fn start(&mut self) -> RvcResult<()> {
        let host = cpal::default_host();

        // 获取默认设备
        let (input_device_id, output_device_id) = get_default_device();

        let input_device = if let Some(id) = input_device_id {
            host.input_devices()
                .map_err(|e| RvcError::audio(format!("Failed to get input devices: {}", e)))?
                .nth(id)
                .ok_or_else(|| RvcError::audio(format!("Input device {} not found", id)))?
        } else {
            host.default_input_device()
                .ok_or_else(|| RvcError::audio("No default input device found"))?
        };

        let output_device = if let Some(id) = output_device_id {
            host.output_devices()
                .map_err(|e| RvcError::audio(format!("Failed to get output devices: {}", e)))?
                .nth(id)
                .ok_or_else(|| RvcError::audio(format!("Output device {} not found", id)))?
        } else {
            host.default_output_device()
                .ok_or_else(|| RvcError::audio("No default output device found"))?
        };

        // 创建cpal的StreamConfig
        let cpal_config = self.config.to_cpal_config();

        // 创建音频流的数据缓冲区
        let state = Arc::clone(&self.state);

        // 创建共享的音频缓冲区
        let audio_buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let audio_buffer_input = Arc::clone(&audio_buffer);
        let audio_buffer_output = Arc::clone(&audio_buffer);

        // 创建输入流
        let input_stream = input_device
            .build_input_stream(
                &cpal_config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut buffer = audio_buffer_input.lock().unwrap();
                    buffer.clear();
                    buffer.extend_from_slice(data);
                },
                move |err| {
                    eprintln!("Audio input error: {}", err);
                    *state.lock().unwrap() = StreamState::Error;
                },
                None,
            )
            .map_err(|e| RvcError::audio(format!("Failed to build input stream: {}", e)))?;

        // 创建输出流
        let state_output = Arc::clone(&self.state);
        let callback = Arc::new(Mutex::new(self.callback.take()));

        let output_stream = output_device
            .build_output_stream(
                &cpal_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let input_buffer = audio_buffer_output.lock().unwrap();
                    let input_data = if input_buffer.is_empty() {
                        vec![0.0; data.len()]
                    } else {
                        input_buffer.clone()
                    };

                    // 调用用户回调函数
                    if let Ok(mut callback_guard) = callback.lock() {
                        if let Some(ref mut cb) = *callback_guard {
                            cb(&input_data, data);
                        }
                    }
                },
                move |err| {
                    eprintln!("Audio output error: {}", err);
                    *state_output.lock().unwrap() = StreamState::Error;
                },
                None,
            )
            .map_err(|e| RvcError::audio(format!("Failed to build output stream: {}", e)))?;

        // 启动流
        input_stream
            .play()
            .map_err(|e| RvcError::audio(format!("Failed to start input stream: {}", e)))?;

        output_stream
            .play()
            .map_err(|e| RvcError::audio(format!("Failed to start output stream: {}", e)))?;

        self.input_stream = Some(input_stream);
        self.output_stream = Some(output_stream);
        *self.state.lock().unwrap() = StreamState::Running;

        Ok(())
    }

    /// 停止音频流，对应Python的stream.stop()
    pub fn stop(&mut self) -> RvcResult<()> {
        if let Some(input_stream) = self.input_stream.take() {
            input_stream
                .pause()
                .map_err(|e| RvcError::audio(format!("Failed to stop input stream: {}", e)))?;
        }

        if let Some(output_stream) = self.output_stream.take() {
            output_stream
                .pause()
                .map_err(|e| RvcError::audio(format!("Failed to stop output stream: {}", e)))?;
        }

        *self.state.lock().unwrap() = StreamState::Stopped;
        Ok(())
    }

    /// 关闭音频流，对应Python的stream.close()
    pub fn close(&mut self) -> RvcResult<()> {
        self.stop()?;
        self.input_stream = None;
        self.output_stream = None;
        Ok(())
    }

    /// 获取流状态
    pub fn get_state(&self) -> StreamState {
        self.state.lock().unwrap().clone()
    }

    /// 检查流是否正在运行
    pub fn is_active(&self) -> bool {
        *self.state.lock().unwrap() == StreamState::Running
    }
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_hostapis() {
        let hostapis = query_hostapis();
        assert!(!hostapis.is_empty());
        println!("Available host APIs: {:?}", hostapis);
    }

    #[test]
    fn test_query_devices() {
        let devices = query_devices();
        assert!(!devices.is_empty());
        for device in devices {
            println!("Device: {:?}", device);
        }
    }

    #[test]
    fn test_default_device() {
        set_default_device(Some(0), Some(0));
        let (input, output) = get_default_device();
        assert_eq!(input, Some(0));
        assert_eq!(output, Some(0));
    }

    #[test]
    fn test_printt() {
        printt("Hello", &[]);
        printt("Hello %0", &["World"]);
        printt("Input device: %0:%1", &["0", "Default"]);
    }
}
