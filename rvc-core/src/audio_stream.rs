//! 简化的音频流处理模块
//!
//! 直接对应 Python gui_v1.py 中的音频处理逻辑

use crate::{RvcError, RvcResult};
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

/// 音频回调函数类型
pub type AudioCallback = Box<dyn FnMut(&[f32], &mut [f32]) + Send>;

/// 音频处理器接口
pub trait AudioProcessor: Send + Sync {
    /// 处理音频数据
    fn process(&mut self, input: &[f32], output: &mut [f32]);

    /// 获取期望的块大小
    fn get_block_size(&self) -> usize;

    /// 获取延迟（采样点数）
    fn get_latency(&self) -> usize;
}

/// 简化的音频流管理器，对应Python的GUI类
pub struct AudioStream {
    /// 是否正在运行
    running: bool,
    /// 音频回调
    callback: Option<AudioCallback>,
    /// 采样率
    sample_rate: u32,
    /// 通道数
    channels: u16,
    /// 块大小
    block_size: usize,
    /// 输入设备索引
    input_device: Option<usize>,
    /// 输出设备索引
    output_device: Option<usize>,
    /// 线程句柄
    thread_handle: Option<thread::JoinHandle<()>>,
    /// 停止信号
    stop_sender: Option<mpsc::Sender<()>>,
}

impl AudioStream {
    /// 创建新的音频流，对应Python的GUI.__init__
    pub fn new(sample_rate: u32, channels: u16, block_size: usize) -> Self {
        Self {
            running: false,
            callback: None,
            sample_rate,
            channels,
            block_size,
            input_device: None,
            output_device: None,
            thread_handle: None,
            stop_sender: None,
        }
    }

    /// 设置音频回调函数，对应Python的audio_callback方法
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&[f32], &mut [f32]) + Send + 'static,
    {
        self.callback = Some(Box::new(callback));
    }

    /// 设置设备，对应Python的set_devices方法
    pub fn set_devices(&mut self, input_device: usize, output_device: usize) {
        self.input_device = Some(input_device);
        self.output_device = Some(output_device);
        println!("Input device: {}", input_device);
        println!("Output device: {}", output_device);
    }

    /// 开始音频流，对应Python的start_stream方法
    pub fn start(&mut self) -> RvcResult<()> {
        if self.running {
            return Err(RvcError::audio("Stream already running"));
        }

        let input_device = self
            .input_device
            .ok_or_else(|| RvcError::audio("Input device not set"))?;

        let output_device = self
            .output_device
            .ok_or_else(|| RvcError::audio("Output device not set"))?;

        let mut callback = self
            .callback
            .take()
            .ok_or_else(|| RvcError::audio("Audio callback not set"))?;

        let (stop_sender, stop_receiver) = mpsc::channel();
        let sample_rate = self.sample_rate;
        let channels = self.channels;
        let block_size = self.block_size;

        // 启动音频处理线程
        let handle = thread::spawn(move || {
            // 这里应该使用实际的音频库(如cpal)来处理音频
            // 为了简化，我们使用模拟的音频循环
            let mut input_buffer = vec![0.0f32; block_size];
            let mut output_buffer = vec![0.0f32; block_size];

            loop {
                // 检查停止信号
                if stop_receiver.try_recv().is_ok() {
                    break;
                }

                // 模拟音频输入（实际应该从音频设备读取）
                // 这里应该调用实际的音频API

                // 调用音频回调
                callback(&input_buffer, &mut output_buffer);

                // 模拟音频输出（实际应该写入音频设备）
                // 这里应该调用实际的音频API

                // 模拟音频块的时间间隔
                std::thread::sleep(std::time::Duration::from_millis(
                    (block_size as f64 / sample_rate as f64 * 1000.0) as u64,
                ));
            }
        });

        self.running = true;
        self.thread_handle = Some(handle);
        self.stop_sender = Some(stop_sender);
        self.callback = None; // TODO: Some(callback);

        Ok(())
    }

    /// 停止音频流，对应Python的stop_stream方法
    pub fn stop(&mut self) -> RvcResult<()> {
        if !self.running {
            return Ok(());
        }

        // 发送停止信号
        if let Some(sender) = self.stop_sender.take() {
            let _ = sender.send(());
        }

        // 等待线程结束
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }

        self.running = false;
        Ok(())
    }

    /// 检查是否正在运行
    pub fn is_running(&self) -> bool {
        self.running
    }
}

/// 音频设备信息，对应Python中的设备字典
#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub index: usize,
    pub name: String,
    pub hostapi_name: String,
    pub max_input_channels: usize,
    pub max_output_channels: usize,
    pub default_samplerate: f64,
}

/// 音频设备管理器，对应Python的设备管理功能
pub struct AudioDeviceManager {
    /// 主机API列表
    pub hostapis: Vec<String>,
    /// 输入设备列表
    pub input_devices: Vec<String>,
    /// 输出设备列表
    pub output_devices: Vec<String>,
    /// 输入设备索引
    pub input_devices_indices: Vec<usize>,
    /// 输出设备索引
    pub output_devices_indices: Vec<usize>,
    /// 所有设备信息
    devices: Vec<AudioDevice>,
}

impl AudioDeviceManager {
    /// 创建新的设备管理器
    pub fn new() -> Self {
        let mut manager = Self {
            hostapis: Vec::new(),
            input_devices: Vec::new(),
            output_devices: Vec::new(),
            input_devices_indices: Vec::new(),
            output_devices_indices: Vec::new(),
            devices: Vec::new(),
        };

        manager.update_devices(None);
        manager
    }

    /// 更新设备列表，对应Python的update_devices方法
    pub fn update_devices(&mut self, hostapi_name: Option<&str>) {
        // 清空现有设备列表
        self.devices.clear();
        self.hostapis.clear();
        self.input_devices.clear();
        self.output_devices.clear();
        self.input_devices_indices.clear();
        self.output_devices_indices.clear();

        // 这里应该使用实际的音频库来查询设备
        // 为了简化，我们添加一些模拟设备
        self.add_mock_devices();

        // 选择主机API
        let selected_hostapi = hostapi_name
            .filter(|name| self.hostapis.contains(&name.to_string()))
            .unwrap_or_else(|| {
                if !self.hostapis.is_empty() {
                    &self.hostapis[0]
                } else {
                    "Default"
                }
            });

        // 筛选指定主机API的设备
        for device in &self.devices {
            if device.hostapi_name == selected_hostapi {
                if device.max_input_channels > 0 {
                    self.input_devices.push(device.name.clone());
                    self.input_devices_indices.push(device.index);
                }
                if device.max_output_channels > 0 {
                    self.output_devices.push(device.name.clone());
                    self.output_devices_indices.push(device.index);
                }
            }
        }
    }

    /// 添加模拟设备（实际实现中应该查询真实设备）
    fn add_mock_devices(&mut self) {
        // 添加模拟的主机API
        self.hostapis.push("DirectSound".to_string());
        self.hostapis.push("WASAPI".to_string());

        // 添加模拟设备
        self.devices.push(AudioDevice {
            index: 0,
            name: "Default Input".to_string(),
            hostapi_name: "DirectSound".to_string(),
            max_input_channels: 2,
            max_output_channels: 0,
            default_samplerate: 48000.0,
        });

        self.devices.push(AudioDevice {
            index: 1,
            name: "Default Output".to_string(),
            hostapi_name: "DirectSound".to_string(),
            max_input_channels: 0,
            max_output_channels: 2,
            default_samplerate: 48000.0,
        });

        self.devices.push(AudioDevice {
            index: 2,
            name: "Microphone".to_string(),
            hostapi_name: "WASAPI".to_string(),
            max_input_channels: 2,
            max_output_channels: 0,
            default_samplerate: 48000.0,
        });

        self.devices.push(AudioDevice {
            index: 3,
            name: "Speakers".to_string(),
            hostapi_name: "WASAPI".to_string(),
            max_input_channels: 0,
            max_output_channels: 2,
            default_samplerate: 48000.0,
        });
    }

    /// 获取设备采样率，对应Python的get_device_samplerate方法
    pub fn get_device_samplerate(&self, device_index: usize) -> Option<u32> {
        self.devices
            .iter()
            .find(|d| d.index == device_index)
            .map(|d| d.default_samplerate as u32)
    }

    /// 获取设备通道数，对应Python的get_device_channels方法
    pub fn get_device_channels(&self, input_index: usize, output_index: usize) -> Option<u16> {
        let input_device = self.devices.iter().find(|d| d.index == input_index)?;
        let output_device = self.devices.iter().find(|d| d.index == output_index)?;

        let max_input = input_device.max_input_channels;
        let max_output = output_device.max_output_channels;

        Some((max_input.min(max_output).min(2)) as u16)
    }

    /// 获取所有设备信息
    pub fn get_devices(&self) -> &[AudioDevice] {
        &self.devices
    }
}

impl Default for AudioDeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AudioStream {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// 简单的线性插值重采样器
pub struct AudioResampler {
    input_sample_rate: u32,
    output_sample_rate: u32,
    ratio: f64,
}

impl AudioResampler {
    /// 创建新的重采样器
    pub fn new(
        input_sample_rate: u32,
        output_sample_rate: u32,
        _channels: usize,
    ) -> RvcResult<Self> {
        let ratio = output_sample_rate as f64 / input_sample_rate as f64;

        Ok(Self {
            input_sample_rate,
            output_sample_rate,
            ratio,
        })
    }

    /// 重采样音频数据
    pub fn process(&mut self, input: &[Vec<f32>]) -> RvcResult<Vec<Vec<f32>>> {
        let mut output = Vec::new();

        for channel in input {
            let resampled = self.resample_channel(channel);
            output.push(resampled);
        }

        Ok(output)
    }

    /// 对单个通道进行重采样
    fn resample_channel(&self, input: &[f32]) -> Vec<f32> {
        if self.ratio == 1.0 {
            return input.to_vec();
        }

        let output_len = (input.len() as f64 * self.ratio) as usize;
        let mut output = vec![0.0; output_len];

        for i in 0..output_len {
            let src_idx = i as f64 / self.ratio;
            let idx_floor = src_idx.floor() as usize;
            let idx_ceil = (idx_floor + 1).min(input.len() - 1);
            let frac = src_idx - idx_floor as f64;

            if idx_floor < input.len() {
                // 线性插值
                output[i] = input[idx_floor] * (1.0 - frac as f32) + input[idx_ceil] * frac as f32;
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_manager() {
        let manager = AudioDeviceManager::new();
        assert!(!manager.hostapis.is_empty());
        assert!(!manager.devices.is_empty());
    }

    #[test]
    fn test_audio_stream_creation() {
        let stream = AudioStream::new(48000, 2, 512);
        assert!(!stream.is_running());
    }

    #[test]
    fn test_device_queries() {
        let manager = AudioDeviceManager::new();
        if let Some(sample_rate) = manager.get_device_samplerate(0) {
            assert!(sample_rate > 0);
        }

        if let Some(channels) = manager.get_device_channels(0, 1) {
            assert!(channels > 0);
        }
    }
}
