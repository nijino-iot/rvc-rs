//! 噪声抑制模块
//!
//! 提供简单的噪声门和降噪功能

use crate::RvcResult;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use rustfft::num_complex::Complex;
use std::sync::Arc;

/// 噪声门
pub struct NoiseGate {
    /// 阈值（线性值）
    threshold: f32,
    /// 释放时间（采样点）
    release_samples: usize,
    /// 攻击时间（采样点）
    attack_samples: usize,
    /// 当前增益
    current_gain: f32,
    /// 采样率
    sample_rate: u32,
}

impl NoiseGate {
    /// 创建新的噪声门
    pub fn new(threshold_db: f32, sample_rate: u32) -> Self {
        let threshold = db_to_linear(threshold_db);
        let attack_time = 0.001; // 1ms
        let release_time = 0.05; // 50ms

        Self {
            threshold,
            release_samples: (release_time * sample_rate as f32) as usize,
            attack_samples: (attack_time * sample_rate as f32) as usize,
            current_gain: 0.0,
            sample_rate,
        }
    }

    /// 处理音频
    pub fn process(&mut self, input: &[f32], output: &mut [f32]) {
        for (inp, out) in input.iter().zip(output.iter_mut()) {
            let level = inp.abs();

            // 计算目标增益
            let target_gain = if level > self.threshold { 1.0 } else { 0.0 };

            // 平滑增益变化
            if target_gain > self.current_gain {
                // 攻击（打开门）
                let attack_rate = 1.0 / self.attack_samples as f32;
                self.current_gain = (self.current_gain + attack_rate).min(1.0);
            } else if target_gain < self.current_gain {
                // 释放（关闭门）
                let release_rate = 1.0 / self.release_samples as f32;
                self.current_gain = (self.current_gain - release_rate).max(0.0);
            }

            *out = inp * self.current_gain;
        }
    }
}

/// 频谱噪声抑制器
pub struct SpectralNoiseSuppressor {
    /// FFT 大小
    fft_size: usize,
    /// 跳跃大小
    hop_size: usize,
    /// 窗口函数
    window: Vec<f32>,
    /// 输入缓冲区
    input_buffer: Vec<f32>,
    /// 输出缓冲区
    output_buffer: Vec<f32>,
    /// FFT 计划
    fft: Arc<dyn RealToComplex<f32>>,
    /// IFFT 计划
    ifft: Arc<dyn ComplexToReal<f32>>,
    /// 频谱缓冲区
    spectrum_buffer: Vec<Complex<f32>>,
    /// 噪声谱估计
    noise_spectrum: Vec<f32>,
    /// 抑制因子
    suppression_factor: f32,
    /// 是否需要更新噪声谱
    update_noise: bool,
    /// 噪声谱更新计数
    update_count: usize,
}

impl SpectralNoiseSuppressor {
    /// 创建新的频谱噪声抑制器
    pub fn new(sample_rate: u32) -> RvcResult<Self> {
        let fft_size = 2048;
        let hop_size = fft_size / 4;

        // 创建汉宁窗
        let window: Vec<f32> = (0..fft_size)
            .map(|i| {
                0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (fft_size - 1) as f32).cos())
            })
            .collect();

        // 创建 FFT 计划
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(fft_size);
        let ifft = planner.plan_fft_inverse(fft_size);

        let spectrum_size = fft_size / 2 + 1;

        Ok(Self {
            fft_size,
            hop_size,
            window,
            input_buffer: vec![0.0; fft_size],
            output_buffer: vec![0.0; fft_size],
            fft,
            ifft,
            spectrum_buffer: vec![Complex::new(0.0, 0.0); spectrum_size],
            noise_spectrum: vec![0.0; spectrum_size],
            suppression_factor: 0.8,
            update_noise: true,
            update_count: 0,
        })
    }

    /// 处理音频帧
    pub fn process_frame(&mut self, input: &[f32]) -> Vec<f32> {
        let mut output = vec![0.0; input.len()];
        let mut input_pos = 0;
        let mut output_pos = 0;

        while input_pos + self.hop_size <= input.len() {
            // 将输入数据移动到缓冲区
            for i in 0..self.fft_size - self.hop_size {
                self.input_buffer[i] = self.input_buffer[i + self.hop_size];
            }
            for i in 0..self.hop_size {
                if input_pos + i < input.len() {
                    self.input_buffer[self.fft_size - self.hop_size + i] = input[input_pos + i];
                }
            }

            // 应用窗口
            let mut windowed = vec![0.0; self.fft_size];
            for i in 0..self.fft_size {
                windowed[i] = self.input_buffer[i] * self.window[i];
            }

            // FFT
            self.fft
                .process(&mut windowed, &mut self.spectrum_buffer)
                .unwrap();

            // 噪声抑制
            if self.update_noise && self.update_count < 10 {
                // 更新噪声谱估计（前10帧）
                for i in 0..self.spectrum_buffer.len() {
                    let magnitude = self.spectrum_buffer[i].norm();
                    self.noise_spectrum[i] = (self.noise_spectrum[i] * self.update_count as f32
                        + magnitude)
                        / (self.update_count + 1) as f32;
                }
                self.update_count += 1;
                if self.update_count >= 10 {
                    self.update_noise = false;
                }
            } else {
                // 应用频谱减法
                for i in 0..self.spectrum_buffer.len() {
                    let magnitude = self.spectrum_buffer[i].norm();
                    let phase = self.spectrum_buffer[i].arg();

                    // 频谱减法
                    let suppressed_magnitude =
                        (magnitude - self.suppression_factor * self.noise_spectrum[i]).max(0.0);

                    // 重建复数
                    self.spectrum_buffer[i] = Complex::from_polar(suppressed_magnitude, phase);
                }
            }

            // IFFT
            let mut time_domain = vec![0.0; self.fft_size];
            self.ifft
                .process(&mut self.spectrum_buffer, &mut time_domain)
                .unwrap();

            // 归一化
            let scale = 1.0 / self.fft_size as f32;
            for sample in &mut time_domain {
                *sample *= scale;
            }

            // 重叠相加
            for i in 0..self.fft_size {
                self.output_buffer[i] += time_domain[i] * self.window[i];
            }

            // 输出
            for i in 0..self.hop_size {
                if output_pos < output.len() {
                    output[output_pos] = self.output_buffer[i];
                    output_pos += 1;
                }
            }

            // 移动输出缓冲区
            for i in 0..self.fft_size - self.hop_size {
                self.output_buffer[i] = self.output_buffer[i + self.hop_size];
            }
            for i in self.fft_size - self.hop_size..self.fft_size {
                self.output_buffer[i] = 0.0;
            }

            input_pos += self.hop_size;
        }

        output
    }

    /// 重置噪声谱估计
    pub fn reset_noise_estimation(&mut self) {
        self.noise_spectrum.fill(0.0);
        self.update_noise = true;
        self.update_count = 0;
    }

    /// 设置抑制因子
    pub fn set_suppression_factor(&mut self, factor: f32) {
        self.suppression_factor = factor.clamp(0.0, 2.0);
    }
}

/// 简单的降噪器（组合噪声门和频谱抑制）
pub struct NoiseReducer {
    /// 噪声门
    noise_gate: NoiseGate,
    /// 频谱噪声抑制器
    spectral_suppressor: Option<SpectralNoiseSuppressor>,
    /// 是否启用频谱抑制
    use_spectral: bool,
}

impl NoiseReducer {
    /// 创建新的降噪器
    pub fn new(threshold_db: f32, sample_rate: u32, use_spectral: bool) -> RvcResult<Self> {
        let noise_gate = NoiseGate::new(threshold_db, sample_rate);

        let spectral_suppressor = if use_spectral {
            Some(SpectralNoiseSuppressor::new(sample_rate)?)
        } else {
            None
        };

        Ok(Self {
            noise_gate,
            spectral_suppressor,
            use_spectral,
        })
    }

    /// 处理音频
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        let mut output = vec![0.0; input.len()];

        // 先应用噪声门
        self.noise_gate.process(input, &mut output);

        // 如果启用，应用频谱噪声抑制
        if self.use_spectral {
            if let Some(suppressor) = &mut self.spectral_suppressor {
                output = suppressor.process_frame(&output);
            }
        }

        output
    }

    /// 设置阈值
    pub fn set_threshold(&mut self, threshold_db: f32) {
        self.noise_gate.threshold = db_to_linear(threshold_db);
    }

    /// 启用/禁用频谱抑制
    pub fn set_use_spectral(&mut self, use_spectral: bool) {
        self.use_spectral = use_spectral;
    }
}

/// dB 转线性值
fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// 线性值转 dB
fn linear_to_db(linear: f32) -> f32 {
    20.0 * linear.max(1e-10).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_gate() {
        let mut gate = NoiseGate::new(-40.0, 48000);
        let input = vec![0.001; 100]; // 低于阈值的信号
        let mut output = vec![0.0; 100];

        gate.process(&input, &mut output);

        // 检查输出是否被衰减
        assert!(output.iter().all(|&x| x < 0.001));
    }

    #[test]
    fn test_noise_reducer_creation() {
        let reducer = NoiseReducer::new(-40.0, 48000, false);
        assert!(reducer.is_ok());
    }

    #[test]
    fn test_db_conversion() {
        let db = -20.0;
        let linear = db_to_linear(db);
        assert!((linear - 0.1).abs() < 0.01);

        let db_back = linear_to_db(linear);
        assert!((db - db_back).abs() < 0.01);
    }
}
