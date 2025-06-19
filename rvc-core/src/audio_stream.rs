//! 实时音频流处理模块
//!
//! 对应 Python 代码中的音频回调和流处理功能

use crate::{RvcError, RvcResult};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, StreamConfig, SupportedStreamConfig};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// 音频缓冲区
#[derive(Debug)]
pub struct AudioBuffer {
    /// 输入缓冲区
    input_buffer: VecDeque<f32>,
    /// 输出缓冲区
    output_buffer: VecDeque<f32>,
    /// 缓冲区大小
    buffer_size: usize,
}

impl AudioBuffer {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            input_buffer: VecDeque::with_capacity(buffer_size * 2),
            output_buffer: VecDeque::with_capacity(buffer_size * 2),
            buffer_size,
        }
    }

    /// 写入输入数据
    pub fn write_input(&mut self, data: &[f32]) {
        self.input_buffer.extend(data);
        // 防止缓冲区过大
        while self.input_buffer.len() > self.buffer_size * 4 {
            self.input_buffer.pop_front();
        }
    }

    /// 读取输入数据
    pub fn read_input(&mut self, len: usize) -> Vec<f32> {
        let actual_len = len.min(self.input_buffer.len());
        self.input_buffer.drain(..actual_len).collect()
    }

    /// 写入输出数据
    pub fn write_output(&mut self, data: &[f32]) {
        self.output_buffer.extend(data);
    }

    /// 读取输出数据
    pub fn read_output(&mut self, len: usize) -> Vec<f32> {
        let actual_len = len.min(self.output_buffer.len());
        let mut result = vec![0.0; len];
        for (i, sample) in self.output_buffer.drain(..actual_len).enumerate() {
            result[i] = sample;
        }
        result
    }

    /// 获取输入缓冲区大小
    pub fn input_size(&self) -> usize {
        self.input_buffer.len()
    }

    /// 获取输出缓冲区大小
    pub fn output_size(&self) -> usize {
        self.output_buffer.len()
    }
}

/// 音频处理回调接口
pub trait AudioProcessor: Send + Sync {
    /// 处理音频数据
    fn process(&mut self, input: &[f32], output: &mut [f32]);

    /// 获取期望的块大小
    fn get_block_size(&self) -> usize;

    /// 获取延迟（采样点数）
    fn get_latency(&self) -> usize;
}

/// 音频流管理器
pub struct AudioStreamManager {
    /// 输入设备
    input_device: Option<Device>,
    /// 输出设备
    output_device: Option<Device>,
    /// 输入流
    input_stream: Option<Stream>,
    /// 输出流
    output_stream: Option<Stream>,
    /// 音频缓冲区
    buffer: Arc<Mutex<AudioBuffer>>,
    /// 音频处理器
    processor: Arc<Mutex<Box<dyn AudioProcessor>>>,
    /// 是否正在运行
    running: Arc<AtomicBool>,
    /// 采样率
    sample_rate: u32,
    /// 通道数
    channels: u16,
    /// 块大小
    block_size: usize,
    /// 统计信息
    stats: Arc<Mutex<StreamStats>>,
}

/// 流统计信息
#[derive(Debug, Clone)]
pub struct StreamStats {
    /// 处理延迟
    pub processing_latency_ms: f64,
    /// 缓冲区延迟
    pub buffer_latency_ms: f64,
    /// 输入缓冲区使用率
    pub input_buffer_usage: f64,
    /// 输出缓冲区使用率
    pub output_buffer_usage: f64,
    /// 丢失的样本数
    pub dropped_samples: u64,
    /// 处理的总样本数
    pub processed_samples: u64,
}

impl Default for StreamStats {
    fn default() -> Self {
        Self {
            processing_latency_ms: 0.0,
            buffer_latency_ms: 0.0,
            input_buffer_usage: 0.0,
            output_buffer_usage: 0.0,
            dropped_samples: 0,
            processed_samples: 0,
        }
    }
}

impl AudioStreamManager {
    /// 创建新的音频流管理器
    pub fn new(
        processor: Box<dyn AudioProcessor>,
        sample_rate: u32,
        channels: u16,
        block_size: usize,
    ) -> Self {
        let buffer_size = block_size * 4; // 4个块的缓冲

        Self {
            input_device: None,
            output_device: None,
            input_stream: None,
            output_stream: None,
            buffer: Arc::new(Mutex::new(AudioBuffer::new(buffer_size))),
            processor: Arc::new(Mutex::new(processor)),
            running: Arc::new(AtomicBool::new(false)),
            sample_rate,
            channels,
            block_size,
            stats: Arc::new(Mutex::new(StreamStats::default())),
        }
    }

    /// 设置输入设备
    pub fn set_input_device(&mut self, device_name: &str) -> RvcResult<()> {
        let host = cpal::default_host();

        // 查找设备
        let device = host
            .input_devices()
            .map_err(|e| RvcError::audio(format!("Failed to enumerate input devices: {}", e)))?
            .find(|d| d.name().map(|name| name == device_name).unwrap_or(false))
            .ok_or_else(|| RvcError::audio(format!("Input device '{}' not found", device_name)))?;

        self.input_device = Some(device);
        Ok(())
    }

    /// 设置输出设备
    pub fn set_output_device(&mut self, device_name: &str) -> RvcResult<()> {
        let host = cpal::default_host();

        // 查找设备
        let device = host
            .output_devices()
            .map_err(|e| RvcError::audio(format!("Failed to enumerate output devices: {}", e)))?
            .find(|d| d.name().map(|name| name == device_name).unwrap_or(false))
            .ok_or_else(|| RvcError::audio(format!("Output device '{}' not found", device_name)))?;

        self.output_device = Some(device);
        Ok(())
    }

    /// 开始音频流
    pub fn start(&mut self) -> RvcResult<()> {
        if self.running.load(Ordering::Relaxed) {
            return Err(RvcError::audio("Audio stream already running"));
        }

        // 确保设备已设置
        let input_device = self
            .input_device
            .as_ref()
            .ok_or_else(|| RvcError::audio("Input device not set"))?;
        let output_device = self
            .output_device
            .as_ref()
            .ok_or_else(|| RvcError::audio("Output device not set"))?;

        // 获取设备配置
        let input_config = self.get_stream_config(input_device, true)?;
        let output_config = self.get_stream_config(output_device, false)?;

        // 创建输入流
        let input_stream = self.create_input_stream(input_device, &input_config)?;

        // 创建输出流
        let output_stream = self.create_output_stream(output_device, &output_config)?;

        // 启动流
        input_stream
            .play()
            .map_err(|e| RvcError::audio(format!("Failed to start input stream: {}", e)))?;
        output_stream
            .play()
            .map_err(|e| RvcError::audio(format!("Failed to start output stream: {}", e)))?;

        self.input_stream = Some(input_stream);
        self.output_stream = Some(output_stream);
        self.running.store(true, Ordering::Relaxed);

        Ok(())
    }

    /// 停止音频流
    pub fn stop(&mut self) -> RvcResult<()> {
        self.running.store(false, Ordering::Relaxed);

        // 停止并释放流
        if let Some(stream) = self.input_stream.take() {
            stream
                .pause()
                .map_err(|e| RvcError::audio(format!("Failed to stop input stream: {}", e)))?;
        }

        if let Some(stream) = self.output_stream.take() {
            stream
                .pause()
                .map_err(|e| RvcError::audio(format!("Failed to stop output stream: {}", e)))?;
        }

        Ok(())
    }

    /// 获取流配置
    fn get_stream_config(&self, device: &Device, is_input: bool) -> RvcResult<StreamConfig> {
        let supported_config = if is_input {
            device.default_input_config()
        } else {
            device.default_output_config()
        }
        .map_err(|e| RvcError::audio(format!("Failed to get device config: {}", e)))?;

        // 使用请求的配置，如果设备支持的话
        let config = StreamConfig {
            channels: self.channels,
            sample_rate: cpal::SampleRate(self.sample_rate),
            buffer_size: cpal::BufferSize::Fixed(self.block_size as u32),
        };

        Ok(config)
    }

    /// 创建输入流
    fn create_input_stream(&self, device: &Device, config: &StreamConfig) -> RvcResult<Stream> {
        let buffer = Arc::clone(&self.buffer);
        let running = Arc::clone(&self.running);
        let stats = Arc::clone(&self.stats);
        let sample_rate = config.sample_rate.0 as f64;

        let err_fn = |err| eprintln!("Input stream error: {}", err);

        let stream = device
            .build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !running.load(Ordering::Relaxed) {
                        return;
                    }

                    let start = Instant::now();

                    // 写入缓冲区
                    let mut buffer = buffer.lock().unwrap();
                    buffer.write_input(data);

                    // 更新统计
                    let mut stats = stats.lock().unwrap();
                    stats.processed_samples += data.len() as u64;
                    stats.input_buffer_usage =
                        (buffer.input_size() as f64 / buffer.buffer_size as f64) * 100.0;
                    stats.buffer_latency_ms = (buffer.input_size() as f64 / sample_rate) * 1000.0;
                },
                err_fn,
                None,
            )
            .map_err(|e| RvcError::audio(format!("Failed to build input stream: {}", e)))?;

        Ok(stream)
    }

    /// 创建输出流
    fn create_output_stream(&self, device: &Device, config: &StreamConfig) -> RvcResult<Stream> {
        let buffer = Arc::clone(&self.buffer);
        let processor = Arc::clone(&self.processor);
        let running = Arc::clone(&self.running);
        let stats = Arc::clone(&self.stats);
        let block_size = self.block_size;

        let err_fn = |err| eprintln!("Output stream error: {}", err);

        let stream = device
            .build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if !running.load(Ordering::Relaxed) {
                        // 静音
                        for sample in data.iter_mut() {
                            *sample = 0.0;
                        }
                        return;
                    }

                    let start = Instant::now();

                    // 获取输入数据并处理
                    let mut buffer = buffer.lock().unwrap();
                    let mut processor = processor.lock().unwrap();

                    // 确保有足够的输入数据
                    if buffer.input_size() >= block_size {
                        let input = buffer.read_input(block_size);
                        let mut output = vec![0.0; block_size];

                        // 处理音频
                        processor.process(&input, &mut output);

                        // 写入输出缓冲
                        buffer.write_output(&output);
                    }

                    // 从输出缓冲读取数据
                    let output_data = buffer.read_output(data.len());
                    data.copy_from_slice(&output_data);

                    // 更新统计
                    let processing_time = start.elapsed();
                    let mut stats = stats.lock().unwrap();
                    stats.processing_latency_ms = processing_time.as_secs_f64() * 1000.0;
                    stats.output_buffer_usage =
                        (buffer.output_size() as f64 / buffer.buffer_size as f64) * 100.0;
                },
                err_fn,
                None,
            )
            .map_err(|e| RvcError::audio(format!("Failed to build output stream: {}", e)))?;

        Ok(stream)
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> StreamStats {
        self.stats.lock().unwrap().clone()
    }

    /// 获取设备列表
    pub fn enumerate_devices() -> RvcResult<(Vec<String>, Vec<String>)> {
        let host = cpal::default_host();

        // 输入设备
        let input_devices: Vec<String> = host
            .input_devices()
            .map_err(|e| RvcError::audio(format!("Failed to enumerate input devices: {}", e)))?
            .filter_map(|device| device.name().ok())
            .collect();

        // 输出设备
        let output_devices: Vec<String> = host
            .output_devices()
            .map_err(|e| RvcError::audio(format!("Failed to enumerate output devices: {}", e)))?
            .filter_map(|device| device.name().ok())
            .collect();

        Ok((input_devices, output_devices))
    }

    /// 获取主机API列表
    pub fn enumerate_hosts() -> Vec<String> {
        cpal::available_hosts()
            .into_iter()
            .map(|host_id| format!("{:?}", host_id))
            .collect()
    }
}

/// 音频重采样器
pub struct AudioResampler {
    resampler: SincFixedIn<f32>,
    input_sample_rate: u32,
    output_sample_rate: u32,
}

impl AudioResampler {
    /// 创建新的重采样器
    pub fn new(
        input_sample_rate: u32,
        output_sample_rate: u32,
        channels: usize,
    ) -> RvcResult<Self> {
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 160,
            window: WindowFunction::BlackmanHarris2,
        };

        let resampler = SincFixedIn::<f32>::new(
            output_sample_rate as f64 / input_sample_rate as f64,
            2.0,
            params,
            1024,
            channels,
        )
        .map_err(|e| RvcError::audio(format!("Failed to create resampler: {}", e)))?;

        Ok(Self {
            resampler,
            input_sample_rate,
            output_sample_rate,
        })
    }

    /// 重采样音频数据
    pub fn process(&mut self, input: &[Vec<f32>]) -> RvcResult<Vec<Vec<f32>>> {
        self.resampler
            .process(input, None)
            .map_err(|e| RvcError::audio(format!("Resampling failed: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyProcessor;

    impl AudioProcessor for DummyProcessor {
        fn process(&mut self, input: &[f32], output: &mut [f32]) {
            // 简单复制
            output.copy_from_slice(input);
        }

        fn get_block_size(&self) -> usize {
            512
        }

        fn get_latency(&self) -> usize {
            0
        }
    }

    #[test]
    fn test_audio_buffer() {
        let mut buffer = AudioBuffer::new(1024);

        // 测试写入和读取
        let data = vec![1.0, 2.0, 3.0, 4.0];
        buffer.write_input(&data);
        assert_eq!(buffer.input_size(), 4);

        let read_data = buffer.read_input(2);
        assert_eq!(read_data, vec![1.0, 2.0]);
        assert_eq!(buffer.input_size(), 2);
    }

    #[test]
    fn test_enumerate_devices() {
        // 仅测试函数不会崩溃
        let result = AudioStreamManager::enumerate_devices();
        if let Ok((inputs, outputs)) = result {
            println!("Input devices: {:?}", inputs);
            println!("Output devices: {:?}", outputs);
        }
    }

    #[test]
    fn test_stream_manager_creation() {
        let processor = Box::new(DummyProcessor);
        let manager = AudioStreamManager::new(processor, 48000, 2, 512);
        assert!(!manager.running.load(Ordering::Relaxed));
    }
}
