//! RVC 音频处理器实现
//!
//! 对应 Python 代码中的音频回调和实时处理逻辑

use crate::{
    audio_stream::{AudioProcessor, AudioResampler},
    noise_suppression::NoiseReducer,
    tensor::Tensor,
    Config, F0Predictor, RvcError, RvcResult,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// RVC 音频处理器
pub struct RvcProcessor {
    /// 配置
    config: Config,
    /// F0 预测器
    f0_predictor: Option<Box<dyn F0Predictor + Send + Sync>>,
    /// 模型推理器 (暂时使用占位符)
    model: Option<()>,
    /// 采样率
    sample_rate: u32,
    /// 目标采样率 (模型需要的)
    target_sample_rate: u32,
    /// 块大小
    block_size: usize,
    /// 交叉淡化长度
    crossfade_size: usize,
    /// 额外帧数
    extra_frames: usize,
    /// 输入缓冲区
    input_buffer: VecDeque<f32>,
    /// 输出缓冲区
    output_buffer: VecDeque<f32>,
    /// SOLA 缓冲区
    sola_buffer: Vec<f32>,
    /// 淡入窗口
    fade_in_window: Vec<f32>,
    /// 淡出窗口
    fade_out_window: Vec<f32>,
    /// 重采样器 (如果需要)
    input_resampler: Option<AudioResampler>,
    output_resampler: Option<AudioResampler>,
    /// 音高缓存
    pitch_cache: Vec<i64>,
    pitch_cache_f: Vec<f32>,
    /// RMS 缓冲区
    rms_buffer: Vec<f32>,
    /// 噪声门阈值
    threshold: f32,
    /// 输入降噪器
    input_noise_reducer: Option<NoiseReducer>,
    /// 输出降噪器
    output_noise_reducer: Option<NoiseReducer>,
    /// 处理延迟
    latency_samples: usize,
    /// 统计信息
    stats: ProcessorStats,
}

/// 处理器统计信息
#[derive(Debug, Clone, Default)]
pub struct ProcessorStats {
    /// F0 提取时间 (ms)
    pub f0_time_ms: f64,
    /// 推理时间 (ms)
    pub infer_time_ms: f64,
    /// 总处理时间 (ms)
    pub total_time_ms: f64,
    /// 处理的帧数
    pub processed_frames: u64,
}

impl RvcProcessor {
    /// 创建新的 RVC 处理器
    pub fn new(config: Config, sample_rate: u32) -> RvcResult<Self> {
        // 计算各种大小参数
        let target_sample_rate = 16000; // RVC 模型通常使用 16kHz
        let block_size = ((config.block_time * sample_rate as f32) as usize / 100) * 100;
        let crossfade_size = ((config.crossfade_length * sample_rate as f32) as usize / 100) * 100;
        let extra_frames = ((config.extra_time * sample_rate as f32) as usize / 100) * 100;

        // 创建窗口函数
        let fade_in_window: Vec<f32> = (0..crossfade_size)
            .map(|i| {
                let x = i as f32 / crossfade_size as f32;
                (0.5 * std::f32::consts::PI * x).sin().powi(2)
            })
            .collect();

        let fade_out_window: Vec<f32> = fade_in_window.iter().map(|x| 1.0 - x).collect();

        // 创建重采样器 (如果需要)
        let (input_resampler, output_resampler) = if sample_rate != target_sample_rate {
            let input_rs = AudioResampler::new(sample_rate, target_sample_rate, 1)?;
            let output_rs = AudioResampler::new(target_sample_rate, sample_rate, 1)?;
            (Some(input_rs), Some(output_rs))
        } else {
            (None, None)
        };

        // 计算延迟
        let latency_samples = block_size + crossfade_size;

        // 创建降噪器
        let input_noise_reducer = if config.i_noise_reduce {
            Some(NoiseReducer::new(config.threshold, sample_rate, true)?)
        } else {
            None
        };

        let output_noise_reducer = if config.o_noise_reduce {
            Some(NoiseReducer::new(config.threshold, sample_rate, true)?)
        } else {
            None
        };

        Ok(Self {
            config,
            f0_predictor: None,
            model: None,
            sample_rate,
            target_sample_rate,
            block_size,
            crossfade_size,
            extra_frames,
            input_buffer: VecDeque::with_capacity(block_size * 4),
            output_buffer: VecDeque::with_capacity(block_size * 4),
            sola_buffer: vec![0.0; crossfade_size],
            fade_in_window,
            fade_out_window,
            input_resampler,
            output_resampler,
            pitch_cache: vec![0; 1024],
            pitch_cache_f: vec![0.0; 1024],
            rms_buffer: vec![0.0; 4 * 100], // 4 * zc
            threshold: db_to_linear(config.threshold),
            input_noise_reducer,
            output_noise_reducer,
            latency_samples,
            stats: ProcessorStats::default(),
        })
    }

    /// 设置 F0 预测器
    pub fn set_f0_predictor(&mut self, predictor: Box<dyn F0Predictor + Send + Sync>) {
        self.f0_predictor = Some(predictor);
    }

    /// 处理音频块
    fn process_block(&mut self, input: &[f32]) -> RvcResult<Vec<f32>> {
        let start = Instant::now();

        // 1. 输入降噪（如果启用）
        let denoised_input = if let Some(reducer) = &mut self.input_noise_reducer {
            reducer.process(input)
        } else {
            input.to_vec()
        };

        // 2. 检查静音
        let rms = calculate_rms(&denoised_input);
        if rms < self.threshold {
            return Ok(vec![0.0; denoised_input.len()]);
        }

        // 3. 重采样到目标采样率 (如果需要)
        let resampled_input = if let Some(resampler) = &mut self.input_resampler {
            let input_2d = vec![denoised_input.clone()];
            let resampled = resampler.process(&input_2d)?;
            resampled[0].clone()
        } else {
            denoised_input
        };

        // 4. F0 提取
        let f0_start = Instant::now();
        let (pitch, pitchf) = if let Some(predictor) = &mut self.f0_predictor {
            let input_tensor = Tensor::from_slice(&resampled_input, &[resampled_input.len()]);
            predictor.predict(&input_tensor, self.config.pitch)?
        } else {
            // 默认音高
            let len = resampled_input.len() / 160 + 1;
            (vec![100; len], vec![440.0; len])
        };
        self.stats.f0_time_ms = f0_start.elapsed().as_secs_f64() * 1000.0;

        // 5. 特征提取和推理
        let infer_start = Instant::now();

        // TODO: 实际的模型推理
        // 这里使用简单的音高变换作为占位符
        let output = self.simple_pitch_shift(&resampled_input, &pitchf);

        self.stats.infer_time_ms = infer_start.elapsed().as_secs_f64() * 1000.0;

        // 6. 重采样回原始采样率 (如果需要)
        let resampled_output = if let Some(resampler) = &mut self.output_resampler {
            let output_2d = vec![output];
            let resampled = resampler.process(&output_2d)?;
            resampled[0].clone()
        } else {
            output
        };

        // 7. 应用 RMS 混合
        let mixed_output = if self.config.rms_mix_rate > 0.0 {
            self.apply_rms_mix(&resampled_output, input, rms)
        } else {
            resampled_output
        };

        // 8. 输出降噪（如果启用）
        let final_output = if let Some(reducer) = &mut self.output_noise_reducer {
            reducer.process(&mixed_output)
        } else {
            mixed_output
        };

        self.stats.total_time_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.stats.processed_frames += 1;

        Ok(final_output)
    }

    /// 简单的音高变换 (占位符实现)
    fn simple_pitch_shift(&self, input: &[f32], pitchf: &[f32]) -> Vec<f32> {
        // 这是一个非常简化的实现，仅用于演示
        // 实际应该使用相位声码器或其他高质量算法

        let pitch_factor = 2.0_f32.powf(self.config.pitch as f32 / 12.0);
        let mut output = vec![0.0; input.len()];

        for (i, sample) in output.iter_mut().enumerate() {
            let src_idx = (i as f32 / pitch_factor) as usize;
            if src_idx < input.len() {
                *sample = input[src_idx];
            }
        }

        output
    }

    /// 应用 RMS 混合
    fn apply_rms_mix(&self, output: &[f32], input: &[f32], input_rms: f32) -> Vec<f32> {
        let output_rms = calculate_rms(output);
        if output_rms > 0.0 {
            let scale = input_rms / output_rms;
            let mix_rate = self.config.rms_mix_rate;

            output
                .iter()
                .zip(input.iter())
                .map(|(out, inp)| {
                    let scaled_out = out * scale;
                    scaled_out * mix_rate + inp * (1.0 - mix_rate)
                })
                .collect()
        } else {
            output.to_vec()
        }
    }

    /// 应用交叉淡化
    fn crossfade(&mut self, input: &[f32], output: &mut [f32]) {
        let fade_len = self.crossfade_size.min(input.len()).min(output.len());

        // 淡出旧数据，淡入新数据
        for i in 0..fade_len {
            let fade_in = self.fade_in_window[i];
            let fade_out = self.fade_out_window[i];

            output[i] = self.sola_buffer[i] * fade_out + output[i] * fade_in;
        }

        // 更新 SOLA 缓冲区
        if output.len() >= fade_len {
            self.sola_buffer
                .copy_from_slice(&output[output.len() - fade_len..]);
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> ProcessorStats {
        self.stats.clone()
    }

    /// 更新音高
    pub fn update_pitch(&mut self, pitch: i32) {
        self.config.pitch = pitch;
    }

    /// 更新性别因子
    pub fn update_formant(&mut self, formant: f32) {
        self.config.formant = formant;
    }

    /// 更新 Index Rate
    pub fn update_index_rate(&mut self, rate: f32) {
        self.config.index_rate = rate;
    }

    /// 更新输入降噪
    pub fn update_input_noise_reduce(&mut self, enable: bool) -> RvcResult<()> {
        if enable && self.input_noise_reducer.is_none() {
            self.input_noise_reducer = Some(NoiseReducer::new(
                self.config.threshold,
                self.sample_rate,
                true,
            )?);
        } else if !enable {
            self.input_noise_reducer = None;
        }
        Ok(())
    }

    /// 更新输出降噪
    pub fn update_output_noise_reduce(&mut self, enable: bool) -> RvcResult<()> {
        if enable && self.output_noise_reducer.is_none() {
            self.output_noise_reducer = Some(NoiseReducer::new(
                self.config.threshold,
                self.sample_rate,
                true,
            )?);
        } else if !enable {
            self.output_noise_reducer = None;
        }
        Ok(())
    }
}

impl AudioProcessor for RvcProcessor {
    fn process(&mut self, input: &[f32], output: &mut [f32]) {
        // 将输入添加到缓冲区
        self.input_buffer.extend(input);

        // 如果缓冲区有足够的数据，处理一个块
        while self.input_buffer.len() >= self.block_size {
            // 取出一个块
            let block: Vec<f32> = self.input_buffer.drain(..self.block_size).collect();

            // 处理块
            match self.process_block(&block) {
                Ok(processed) => {
                    self.output_buffer.extend(processed);
                }
                Err(e) => {
                    eprintln!("处理音频块失败: {}", e);
                    // 静音输出
                    self.output_buffer.extend(vec![0.0; self.block_size]);
                }
            }
        }

        // 从输出缓冲区读取数据
        let available = self.output_buffer.len().min(output.len());
        for (i, sample) in self.output_buffer.drain(..available).enumerate() {
            output[i] = sample;
        }

        // 如果输出缓冲区数据不足，填充静音
        for i in available..output.len() {
            output[i] = 0.0;
        }
    }

    fn get_block_size(&self) -> usize {
        self.block_size
    }

    fn get_latency(&self) -> usize {
        self.latency_samples
    }
}

/// 计算 RMS (均方根)
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_squares: f32 = samples.iter().map(|x| x * x).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// dB 转线性
fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// 线性转 dB
fn linear_to_db(linear: f32) -> f32 {
    20.0 * linear.max(1e-10).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms_calculation() {
        let samples = vec![0.5, -0.5, 0.5, -0.5];
        let rms = calculate_rms(&samples);
        assert!((rms - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_db_conversion() {
        let db = -20.0;
        let linear = db_to_linear(db);
        let db_back = linear_to_db(linear);
        assert!((db - db_back).abs() < 0.001);
    }

    #[test]
    fn test_processor_creation() {
        let config = Config::default();
        let processor = RvcProcessor::new(config, 48000);
        assert!(processor.is_ok());
    }
}
