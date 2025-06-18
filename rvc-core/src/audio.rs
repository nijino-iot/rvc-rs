//! 音频处理模块
//!
//! 对应 Python 代码中的音频处理功能，包括相位声码器等

use crate::{Device, Kind, RvcError, RvcResult, Tensor};
use std::f64::consts::PI;

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
