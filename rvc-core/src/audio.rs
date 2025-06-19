//! 音频处理模块
//!
//! 对应 Python 代码中的音频处理功能，包括相位声码器、音频加载和重采样等

use crate::{Kind, RvcError, RvcResult, Tensor};
use hound::{SampleFormat, WavReader, WavSpec, WavWriter};
use std::f64::consts::PI;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// 打印函数，对应 Python 中的 printt 函数
pub fn printt(msg: &str, args: &[&str]) {
    if args.is_empty() {
        println!("{}", msg);
    } else {
        // 简单的字符串格式化，实际应用中可能需要更复杂的实现
        let mut formatted = msg.to_string();
        for (i, arg) in args.iter().enumerate() {
            formatted = formatted.replace(&format!("%{}", i), arg);
        }
        println!("{}", formatted);
    }
}

/// 音频格式转换，对应 Python 中的 wav2 函数
///
/// # 参数
/// - `input_path`: 输入文件路径
/// - `output_path`: 输出文件路径
/// - `format`: 目标格式 ("wav", "mp3", "flac", etc.)
pub fn wav2<P: AsRef<Path>>(input_path: P, output_path: P, format: &str) -> RvcResult<()> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();

    // 读取输入音频文件
    let (audio_data, sample_rate) = load_audio_wav(input_path)?;

    // 根据格式写入输出文件
    match format.to_lowercase().as_str() {
        "wav" => {
            write_wav(output_path, &audio_data, sample_rate)?;
        }
        _ => {
            return Err(RvcError::audio(format!("Unsupported format: {}", format)));
        }
    }

    Ok(())
}

/// 加载音频文件，对应 Python 中的 load_audio 函数
///
/// # 参数
/// - `file_path`: 音频文件路径
/// - `sr`: 目标采样率
///
/// # 返回
/// 返回重采样后的音频数据
pub fn load_audio<P: AsRef<Path>>(file_path: P, sr: u32) -> RvcResult<Vec<f32>> {
    let file_path = file_path.as_ref();

    // 检查文件是否存在
    if !file_path.exists() {
        return Err(RvcError::audio(format!(
            "Audio file does not exist: {}",
            file_path.display()
        )));
    }

    // 读取音频文件
    let (mut audio_data, original_sr) = load_audio_wav(file_path)?;

    // 如果采样率不同，进行重采样
    if original_sr != sr {
        audio_data = resample(&audio_data, original_sr, sr)?;
    }

    Ok(audio_data)
}

/// 从WAV文件读取音频数据
fn load_audio_wav<P: AsRef<Path>>(file_path: P) -> RvcResult<(Vec<f32>, u32)> {
    let file = File::open(file_path.as_ref())
        .map_err(|e| RvcError::audio(format!("Failed to open file: {}", e)))?;
    let reader = BufReader::new(file);

    let mut wav_reader = WavReader::new(reader)
        .map_err(|e| RvcError::audio(format!("Failed to read WAV: {}", e)))?;

    let spec = wav_reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels as usize;

    // 读取所有样本
    let samples: Result<Vec<_>, _> = match spec.sample_format {
        SampleFormat::Float => wav_reader.samples::<f32>().collect(),
        SampleFormat::Int => wav_reader
            .samples::<i32>()
            .collect::<Result<Vec<_>, _>>()
            .map(|samples| {
                samples
                    .into_iter()
                    .map(|s| s as f32 / i32::MAX as f32)
                    .collect()
            }),
    };

    let samples = samples.map_err(|e| RvcError::audio(format!("Failed to read samples: {}", e)))?;

    // 转换为单声道（如果是立体声，取平均值）
    let mono_samples = if channels == 1 {
        samples
    } else {
        samples
            .chunks(channels)
            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
            .collect()
    };

    Ok((mono_samples, sample_rate))
}

/// 写入WAV文件
fn write_wav<P: AsRef<Path>>(file_path: P, audio_data: &[f32], sample_rate: u32) -> RvcResult<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: SampleFormat::Float,
    };

    let file = File::create(file_path.as_ref())
        .map_err(|e| RvcError::audio(format!("Failed to create file: {}", e)))?;
    let writer = BufWriter::new(file);

    let mut wav_writer = WavWriter::new(writer, spec)
        .map_err(|e| RvcError::audio(format!("Failed to create WAV writer: {}", e)))?;

    for &sample in audio_data {
        wav_writer
            .write_sample(sample)
            .map_err(|e| RvcError::audio(format!("Failed to write sample: {}", e)))?;
    }

    wav_writer
        .finalize()
        .map_err(|e| RvcError::audio(format!("Failed to finalize WAV: {}", e)))?;

    Ok(())
}

/// 音频重采样
///
/// # 参数
/// - `audio_data`: 输入音频数据
/// - `original_sr`: 原始采样率
/// - `target_sr`: 目标采样率
///
/// # 返回
/// 重采样后的音频数据
pub fn resample(audio_data: &[f32], original_sr: u32, target_sr: u32) -> RvcResult<Vec<f32>> {
    if original_sr == target_sr {
        return Ok(audio_data.to_vec());
    }

    let ratio = original_sr as f64 / target_sr as f64;
    let output_len = (audio_data.len() as f64 / ratio) as usize;
    let mut output = Vec::with_capacity(output_len);

    // 简单的线性插值重采样
    for i in 0..output_len {
        let src_idx = i as f64 * ratio;
        let idx0 = src_idx.floor() as usize;
        let idx1 = (idx0 + 1).min(audio_data.len() - 1);
        let frac = src_idx - idx0 as f64;

        if idx0 < audio_data.len() {
            let sample0 = audio_data[idx0];
            let sample1 = if idx1 < audio_data.len() {
                audio_data[idx1]
            } else {
                sample0
            };

            // 线性插值
            let interpolated = sample0 + (sample1 - sample0) * frac as f32;
            output.push(interpolated);
        } else {
            output.push(0.0);
        }
    }

    Ok(output)
}

/// 清理文件路径，对应 Python 中的 clean_path 函数
pub fn clean_path(path_str: &str) -> String {
    let mut cleaned = path_str.to_string();

    // 移除 Unicode 控制字符
    cleaned = cleaned
        .chars()
        .filter(|c| !c.is_control() || *c == '\n')
        .collect();

    // 移除首尾空格和引号
    cleaned = cleaned.trim().trim_matches('"').trim().to_string();

    // 在 Windows 上将 / 替换为 \
    #[cfg(windows)]
    {
        cleaned = cleaned.replace('/', "\\");
    }

    cleaned
}

/// 相位声码器实现，对应 Python 中的 phase_vocoder 函数
///
/// # 参数
/// - `a`: 输入音频张量 A
/// - `b`: 输入音频张量 B
/// - `fade_out`: 淡出窗口
/// - `fade_in`: 淡入窗口
///
/// # 返回
/// 处理后的音频张量
pub fn phase_vocoder(
    a: &Tensor,
    b: &Tensor,
    fade_out: &Tensor,
    fade_in: &Tensor,
) -> RvcResult<Tensor> {
    let device = a.device();

    // window = torch.sqrt(fade_out * fade_in)
    let window = (fade_out * fade_in).sqrt();

    // fa = torch.fft.rfft(a * window)
    let fa = (a * &window).fft_rfft(&[0], false);

    // fb = torch.fft.rfft(b * window)
    let fb = (b * &window).fft_rfft(&[0], false);

    // absab = torch.abs(fa) + torch.abs(fb)
    let absab = fa.abs().add(&fb.abs());

    // n = a.shape[0]
    let n = a.size()[0];

    // 调整 absab：如果 n 是偶数，absab[1:-1] *= 2；如果是奇数，absab[1:] *= 2
    let mut absab_adjusted = absab.copy();
    if n % 2 == 0 {
        // absab[1:-1] *= 2
        let slice = absab_adjusted.narrow(0, 1, (n / 2 - 1) as i64);
        let doubled = slice * 2.0;
        absab_adjusted = absab_adjusted.slice_scatter(&doubled, 0, 1, 1);
    } else {
        // absab[1:] *= 2
        let slice = absab_adjusted.narrow(0, 1, (n / 2) as i64);
        let doubled = slice * 2.0;
        absab_adjusted = absab_adjusted.slice_scatter(&doubled, 0, 1, 1);
    }

    // phia = torch.angle(fa)
    let phia = fa.angle();

    // phib = torch.angle(fb)
    let phib = fb.angle();

    // deltaphase = phib - phia
    let deltaphase = &phib - &phia;

    // deltaphase = deltaphase - 2 * np.pi * torch.floor(deltaphase / 2 / np.pi + 0.5)
    let two_pi = Tensor::scalar(2.0 * PI).to_device(device);
    let deltaphase_normalized = deltaphase.div(&two_pi).add(&Tensor::scalar(0.5));
    let floored = deltaphase_normalized.floor();
    let deltaphase_wrapped = deltaphase.sub(&two_pi.mul(&floored));

    // w = 2 * np.pi * torch.arange(n // 2 + 1).to(a) + deltaphase
    let freq_bins = (n / 2 + 1) as i64;
    let arange = Tensor::arange(freq_bins, (Kind::Float, device));
    let w = two_pi.mul(&arange).add(&deltaphase_wrapped);

    // t = torch.arange(n).unsqueeze(-1).to(a) / n
    let time_steps = Tensor::arange(n, (Kind::Float, device)).unsqueeze(-1) / (n as f64);

    // result = (
    //     a * (fade_out**2)
    //     + b * (fade_in**2)
    //     + torch.sum(absab * torch.cos(w * t + phia), -1) * window / n
    // )
    let fade_out_sq = fade_out.pow_tensor_scalar(2.0);
    let fade_in_sq = fade_in.pow_tensor_scalar(2.0);

    // w * t + phia 的计算需要广播
    let w_expanded = w.unsqueeze(0); // shape: [1, freq_bins]
    let time_expanded = time_steps; // shape: [n, 1]
    let phia_expanded = phia.unsqueeze(0); // shape: [1, freq_bins]

    // w * t 通过矩阵乘法实现广播
    let wt = time_expanded.matmul(&w_expanded); // shape: [n, freq_bins]
    let phase = wt.add(&phia_expanded); // shape: [n, freq_bins]

    let cos_term = phase.cos(); // shape: [n, freq_bins]
    let absab_expanded = absab_adjusted.unsqueeze(0).expand(&cos_term.size(), true);
    let weighted_cos = absab_expanded * cos_term; // shape: [n, freq_bins]
    let summed = weighted_cos.sum_dim(1, false, Kind::Float); // shape: [n]

    let result = a
        .mul(&fade_out_sq)
        .add(&b.mul(&fade_in_sq))
        .add(&summed.mul(&window).div(&Tensor::scalar(n as f64)));

    Ok(result)
}

/// 音频设备信息
#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub index: usize,
    pub name: String,
    pub hostapi: String,
    pub max_input_channels: usize,
    pub max_output_channels: usize,
    pub default_sample_rate: f64,
}

impl AudioDevice {
    pub fn new(
        index: usize,
        name: String,
        hostapi: String,
        max_input_channels: usize,
        max_output_channels: usize,
        default_sample_rate: f64,
    ) -> Self {
        Self {
            index,
            name,
            hostapi,
            max_input_channels,
            max_output_channels,
            default_sample_rate,
        }
    }
}

/// 音频流配置
#[derive(Debug, Clone)]
pub struct AudioStreamConfig {
    pub sample_rate: u32,
    pub frames_per_buffer: u32,
    pub input_device: Option<usize>,
    pub output_device: Option<usize>,
    pub input_channels: u32,
    pub output_channels: u32,
}

impl Default for AudioStreamConfig {
    fn default() -> Self {
        Self {
            sample_rate: 44100,
            frames_per_buffer: 1024,
            input_device: None,
            output_device: None,
            input_channels: 1,
            output_channels: 1,
        }
    }
}

/// 音频处理器特征
pub trait AudioProcessor {
    /// 处理音频数据
    fn process(&mut self, input: &[f32], output: &mut [f32]) -> RvcResult<()>;

    /// 获取延迟（以样本为单位）
    fn get_latency(&self) -> usize;

    /// 重置处理器状态
    fn reset(&mut self);
}

/// 音频缓冲区管理器
pub struct AudioBuffer {
    buffer: Vec<f32>,
    write_pos: usize,
    read_pos: usize,
    size: usize,
}

impl AudioBuffer {
    /// 创建新的音频缓冲区
    pub fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            write_pos: 0,
            read_pos: 0,
            size,
        }
    }

    /// 写入数据
    pub fn write(&mut self, data: &[f32]) -> usize {
        let mut written = 0;
        for &sample in data {
            if self.available_write() > 0 {
                self.buffer[self.write_pos] = sample;
                self.write_pos = (self.write_pos + 1) % self.size;
                written += 1;
            } else {
                break;
            }
        }
        written
    }

    /// 读取数据
    pub fn read(&mut self, data: &mut [f32]) -> usize {
        let mut read = 0;
        for i in 0..data.len() {
            if self.available_read() > 0 {
                data[i] = self.buffer[self.read_pos];
                self.read_pos = (self.read_pos + 1) % self.size;
                read += 1;
            } else {
                // 填充剩余位置为零
                for j in i..data.len() {
                    data[j] = 0.0;
                }
                break;
            }
        }
        read
    }

    /// 获取可写入的样本数
    pub fn available_write(&self) -> usize {
        if self.write_pos >= self.read_pos {
            self.size - 1 - (self.write_pos - self.read_pos)
        } else {
            self.read_pos - self.write_pos - 1
        }
    }

    /// 获取可读取的样本数
    pub fn available_read(&self) -> usize {
        if self.write_pos >= self.read_pos {
            self.write_pos - self.read_pos
        } else {
            self.size - (self.read_pos - self.write_pos)
        }
    }

    /// 清空缓冲区
    pub fn clear(&mut self) {
        self.write_pos = 0;
        self.read_pos = 0;
        self.buffer.fill(0.0);
    }
}

/// 音频窗口函数
pub mod windows {
    use crate::{Device, Kind, Tensor};

    /// 汉宁窗
    pub fn hann_window(size: i64, device: Device) -> Tensor {
        let n = Tensor::arange(size, (Kind::Float, device));
        let pi = std::f64::consts::PI;
        let factor = 2.0 * pi / (size - 1) as f64;
        (n * factor).sin().pow_tensor_scalar(2.0)
    }

    /// 汉明窗
    pub fn hamming_window(size: i64, device: Device) -> Tensor {
        let n = Tensor::arange(size, (Kind::Float, device));
        let pi = std::f64::consts::PI;
        let factor = 2.0 * pi / (size - 1) as f64;
        Tensor::scalar(0.54).sub(&Tensor::scalar(0.46).mul(&(n.mul_scalar(factor as f64)).cos()))
    }

    /// 布莱克曼窗
    pub fn blackman_window(size: i64, device: Device) -> Tensor {
        let n = Tensor::arange(size, (Kind::Float, device));
        let pi = std::f64::consts::PI;
        let factor = 2.0 * pi / (size - 1) as f64;

        Tensor::scalar(0.42)
            .sub(&Tensor::scalar(0.5).mul(&(n.mul_scalar(factor as f64)).cos()))
            .add(&Tensor::scalar(0.08).mul(&(n.mul_scalar(factor * 2.0)).cos()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Device, Kind};

    #[test]
    fn test_printt() {
        printt("Hello", &[]);
        printt("Hello %0", &["World"]);
    }

    #[test]
    fn test_clean_path() {
        let path = "  \"test/path.wav\"  ";
        let cleaned = clean_path(path);
        #[cfg(windows)]
        assert_eq!(cleaned, "test\\path.wav");
        #[cfg(not(windows))]
        assert_eq!(cleaned, "test/path.wav");
    }

    #[test]
    fn test_resample() -> RvcResult<()> {
        // 创建44.1kHz的测试信号
        let original_sr = 44100;
        let target_sr = 16000;
        let duration = 0.1; // 0.1秒
        let samples = (original_sr as f32 * duration) as usize;

        let mut audio_data = Vec::with_capacity(samples);
        for i in 0..samples {
            let t = i as f32 / original_sr as f32;
            let sample = (2.0 * std::f32::consts::PI * 440.0 * t).sin(); // 440Hz正弦波
            audio_data.push(sample);
        }

        let resampled = resample(&audio_data, original_sr, target_sr)?;
        let expected_len =
            (audio_data.len() as f32 * target_sr as f32 / original_sr as f32) as usize;

        // 允许一定的误差
        assert!((resampled.len() as i32 - expected_len as i32).abs() <= 10);

        Ok(())
    }

    #[test]
    fn test_phase_vocoder() -> RvcResult<()> {
        let device = Device::Cpu;
        let size = 512i64;

        // 创建测试数据
        let a = Tensor::randn(&[size], (Kind::Float, device));
        let b = Tensor::randn(&[size], (Kind::Float, device));
        let fade_out = Tensor::ones(&[size], (Kind::Float, device)) * 0.5;
        let fade_in = Tensor::ones(&[size], (Kind::Float, device)) * 0.5;

        let result = phase_vocoder(&a, &b, &fade_out, &fade_in)?;

        // 检查输出形状
        assert_eq!(result.size(), &[size]);

        Ok(())
    }

    #[test]
    fn test_audio_buffer() {
        let mut buffer = AudioBuffer::new(10);

        // 测试写入
        let data = vec![1.0, 2.0, 3.0];
        let written = buffer.write(&data);
        assert_eq!(written, 3);
        assert_eq!(buffer.available_read(), 3);

        // 测试读取
        let mut output = vec![0.0; 2];
        let read = buffer.read(&mut output);
        assert_eq!(read, 2);
        assert_eq!(output, vec![1.0, 2.0]);
        assert_eq!(buffer.available_read(), 1);
    }

    #[test]
    fn test_windows() {
        let device = Device::Cpu;
        let size = 64;

        let hann = windows::hann_window(size, device);
        assert_eq!(hann.size(), &[size]);

        let hamming = windows::hamming_window(size, device);
        assert_eq!(hamming.size(), &[size]);

        let blackman = windows::blackman_window(size, device);
        assert_eq!(blackman.size(), &[size]);
    }
}
