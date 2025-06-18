//! 实用工具模块
//!
//! 提供 RVC 项目中常用的工具函数和辅助类型

use crate::{Device, Kind, RvcError, RvcResult, Tensor};
use std::path::Path;
use std::time::{Duration, Instant};

/// 时间测量工具
pub struct Timer {
    start_time: Instant,
    name: String,
}

impl Timer {
    /// 创建新的计时器
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            start_time: Instant::now(),
            name: name.into(),
        }
    }

    /// 获取经过的时间（毫秒）
    pub fn elapsed_ms(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64() * 1000.0
    }

    /// 获取经过的时间
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// 重启计时器
    pub fn restart(&mut self) {
        self.start_time = Instant::now();
    }

    /// 打印经过的时间
    pub fn print_elapsed(&self) {
        println!("{}: {:.2}ms", self.name, self.elapsed_ms());
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        log::debug!("{}: {:.2}ms", self.name, self.elapsed_ms());
    }
}

/// 音频实用工具
pub mod audio_utils {
    use super::*;

    /// 将音频数据从 f32 转换为 i16
    pub fn f32_to_i16(samples: &[f32]) -> Vec<i16> {
        samples
            .iter()
            .map(|&sample| {
                let clamped = sample.clamp(-1.0, 1.0);
                (clamped * i16::MAX as f32) as i16
            })
            .collect()
    }

    /// 将音频数据从 i16 转换为 f32
    pub fn i16_to_f32(samples: &[i16]) -> Vec<f32> {
        samples
            .iter()
            .map(|&sample| sample as f32 / i16::MAX as f32)
            .collect()
    }

    /// 归一化音频数据
    pub fn normalize_audio(samples: &mut [f32]) {
        let max_abs = samples.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);

        if max_abs > 0.0 && max_abs != 1.0 {
            let scale = 1.0 / max_abs;
            for sample in samples {
                *sample *= scale;
            }
        }
    }

    /// 应用增益
    pub fn apply_gain(samples: &mut [f32], gain_db: f32) {
        let gain_linear = 10.0f32.powf(gain_db / 20.0);
        for sample in samples {
            *sample *= gain_linear;
        }
    }

    /// 计算 RMS（均方根）
    pub fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f32 = samples.iter().map(|&x| x * x).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }

    /// 计算分贝值
    pub fn linear_to_db(linear: f32) -> f32 {
        if linear <= 0.0 {
            -100.0 // 静音阈值
        } else {
            20.0 * linear.log10()
        }
    }

    /// 从分贝值转换为线性值
    pub fn db_to_linear(db: f32) -> f32 {
        10.0f32.powf(db / 20.0)
    }

    /// 重采样音频（简单线性插值）
    pub fn resample_linear(input: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
        if input_rate == output_rate {
            return input.to_vec();
        }

        let ratio = input_rate as f64 / output_rate as f64;
        let output_len = (input.len() as f64 / ratio) as usize;
        let mut output = Vec::with_capacity(output_len);

        for i in 0..output_len {
            let src_index = i as f64 * ratio;
            let index_floor = src_index.floor() as usize;
            let index_ceil = (index_floor + 1).min(input.len() - 1);
            let fract = src_index - index_floor as f64;

            let sample = if index_floor >= input.len() {
                0.0
            } else if index_floor == index_ceil {
                input[index_floor]
            } else {
                let a = input[index_floor];
                let b = input[index_ceil];
                a + (b - a) * fract as f32
            };

            output.push(sample);
        }

        output
    }
}

/// 张量实用工具
pub mod tensor_utils {
    use super::*;

    /// 从音频数据创建张量
    pub fn audio_to_tensor(audio: &[f32], device: Device) -> Tensor {
        Tensor::from_slice(audio).to_device(device)
    }

    /// 从张量提取音频数据
    pub fn tensor_to_audio(tensor: &Tensor) -> RvcResult<Vec<f32>> {
        let tensor = tensor.clone().to_device(Device::Cpu);
        let data: Vec<f32> = tensor.try_into()?;
        Ok(data)
    }

    /// 创建汉宁窗
    pub fn hann_window(size: i64, device: Device) -> Tensor {
        let n = Tensor::arange(size, (Kind::Float, device));
        let pi = std::f64::consts::PI;
        let factor = 2.0 * pi / (size - 1) as f64;
        (n * factor).sin().pow_tensor_scalar(2.0)
    }

    /// 预加重滤波
    pub fn preemphasis(audio: &Tensor, coeff: f32) -> RvcResult<Tensor> {
        let size = audio.size();
        if size.is_empty() {
            return Ok(audio.shallow_clone());
        }

        let shifted = Tensor::cat(
            &[
                &Tensor::zeros(&[1], audio.kind_device()),
                &audio.narrow(0, 0, size[0] - 1),
            ],
            0,
        );
        Ok(audio.sub(&shifted.mul_scalar(coeff as f64)))
    }

    /// 去加重滤波
    pub fn deemphasis(audio: &Tensor, coeff: f32) -> RvcResult<Tensor> {
        let mut result = Tensor::zeros_like(audio);
        let size = audio.size()[0];

        // 简化的递归实现
        for i in 0..size {
            let current_sample = audio.get(i as i64);
            let prev_sample = if i > 0 {
                result.get((i - 1) as i64)
            } else {
                Tensor::zeros(&[], audio.kind_device())
            };
            let new_sample = current_sample.add(&prev_sample.mul_scalar(coeff as f64));
            result = result.slice_scatter(&new_sample.unsqueeze(0), 0, i as i64, 1);
        }

        Ok(result)
    }

    /// 填充张量到指定长度
    pub fn pad_or_trim(tensor: &Tensor, target_length: i64) -> Tensor {
        let current_length = tensor.size()[0];

        if current_length == target_length {
            tensor.shallow_clone()
        } else if current_length < target_length {
            // 填充零
            let padding = target_length - current_length;
            let zeros = Tensor::zeros(&[padding], tensor.kind_device());
            Tensor::cat(&[&tensor.shallow_clone(), &zeros], 0)
        } else {
            // 截断
            tensor.narrow(0, 0, target_length)
        }
    }

    /// 批量填充张量
    pub fn batch_pad(tensors: &[Tensor]) -> Tensor {
        if tensors.is_empty() {
            panic!("Cannot pad empty tensor list");
        }

        let max_length = tensors.iter().map(|t| t.size()[0]).max().unwrap();
        let padded_tensors: Vec<Tensor> =
            tensors.iter().map(|t| pad_or_trim(t, max_length)).collect();

        let tensor_refs: Vec<&Tensor> = padded_tensors.iter().collect();
        Tensor::stack(&tensor_refs, 0)
    }
}

/// 文件系统实用工具
pub mod fs_utils {
    use super::*;

    /// 确保目录存在
    pub fn ensure_dir_exists<P: AsRef<Path>>(path: P) -> RvcResult<()> {
        let path = path.as_ref();
        if !path.exists() {
            std::fs::create_dir_all(path).map_err(|e| {
                RvcError::other(format!(
                    "Failed to create directory {}: {}",
                    path.display(),
                    e
                ))
            })?;
        }
        Ok(())
    }

    /// 获取文件扩展名
    pub fn get_file_extension<P: AsRef<Path>>(path: P) -> Option<String> {
        path.as_ref()
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
    }

    /// 检查文件是否是音频文件
    pub fn is_audio_file<P: AsRef<Path>>(path: P) -> bool {
        match get_file_extension(path) {
            Some(ext) => matches!(ext.as_str(), "wav" | "mp3" | "flac" | "ogg" | "m4a" | "aac"),
            None => false,
        }
    }

    /// 检查文件是否是模型文件
    pub fn is_model_file<P: AsRef<Path>>(path: P) -> bool {
        match get_file_extension(path) {
            Some(ext) => matches!(ext.as_str(), "pth" | "pt" | "ckpt" | "safetensors"),
            None => false,
        }
    }

    /// 获取文件大小（字节）
    pub fn get_file_size<P: AsRef<Path>>(path: P) -> RvcResult<u64> {
        let metadata = std::fs::metadata(path.as_ref())
            .map_err(|e| RvcError::other(format!("Failed to get file metadata: {}", e)))?;
        Ok(metadata.len())
    }

    /// 格式化文件大小
    pub fn format_file_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }
}

/// 数学实用工具
pub mod math_utils {
    use super::*;

    /// 线性插值
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    /// 限制值在指定范围内
    pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
        value.max(min).min(max)
    }

    /// 将值从一个范围映射到另一个范围
    pub fn map_range(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
        let normalized = (value - from_min) / (from_max - from_min);
        to_min + normalized * (to_max - to_min)
    }

    /// 计算两个向量的余弦相似度
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    /// 计算移动平均
    pub fn moving_average(data: &[f32], window_size: usize) -> Vec<f32> {
        if window_size == 0 || data.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(data.len());
        let mut sum = 0.0;

        for (i, &value) in data.iter().enumerate() {
            sum += value;

            if i >= window_size {
                sum -= data[i - window_size];
                result.push(sum / window_size as f32);
            } else if i == window_size - 1 {
                result.push(sum / window_size as f32);
            }
        }

        result
    }

    /// 找到峰值点
    pub fn find_peaks(data: &[f32], min_height: f32, min_distance: usize) -> Vec<usize> {
        let mut peaks = Vec::new();

        for i in 1..data.len() - 1 {
            if data[i] > min_height && data[i] > data[i - 1] && data[i] > data[i + 1] {
                // 检查最小距离约束
                if peaks.is_empty() || i - peaks.last().unwrap() >= min_distance {
                    peaks.push(i);
                }
            }
        }

        peaks
    }
}

/// 配置验证工具
pub mod validation {
    use super::*;

    /// 验证音频采样率
    pub fn validate_sample_rate(sample_rate: u32) -> RvcResult<()> {
        match sample_rate {
            8000 | 16000 | 22050 | 44100 | 48000 => Ok(()),
            _ => Err(RvcError::parameter(format!(
                "Unsupported sample rate: {}",
                sample_rate
            ))),
        }
    }

    /// 验证音调范围
    pub fn validate_pitch_shift(pitch: i32) -> RvcResult<()> {
        if pitch >= -24 && pitch <= 24 {
            Ok(())
        } else {
            Err(RvcError::parameter(format!(
                "Pitch shift {} is out of range [-24, 24]",
                pitch
            )))
        }
    }

    /// 验证概率值
    pub fn validate_probability(value: f32, name: &str) -> RvcResult<()> {
        if value >= 0.0 && value <= 1.0 {
            Ok(())
        } else {
            Err(RvcError::parameter(format!(
                "{} must be between 0.0 and 1.0, got {}",
                name, value
            )))
        }
    }

    /// 验证正数
    pub fn validate_positive(value: f32, name: &str) -> RvcResult<()> {
        if value > 0.0 {
            Ok(())
        } else {
            Err(RvcError::parameter(format!(
                "{} must be positive, got {}",
                name, value
            )))
        }
    }

    /// 验证非负数
    pub fn validate_non_negative(value: f32, name: &str) -> RvcResult<()> {
        if value >= 0.0 {
            Ok(())
        } else {
            Err(RvcError::parameter(format!(
                "{} must be non-negative, got {}",
                name, value
            )))
        }
    }
}

/// 性能监控工具
pub struct PerformanceMonitor {
    timers: std::collections::HashMap<String, Timer>,
    counters: std::collections::HashMap<String, u64>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            timers: std::collections::HashMap::new(),
            counters: std::collections::HashMap::new(),
        }
    }

    /// 开始计时
    pub fn start_timer(&mut self, name: &str) {
        self.timers.insert(name.to_string(), Timer::new(name));
    }

    /// 结束计时并返回经过的毫秒数
    pub fn end_timer(&mut self, name: &str) -> Option<f64> {
        self.timers.remove(name).map(|timer| timer.elapsed_ms())
    }

    /// 增加计数器
    pub fn increment_counter(&mut self, name: &str) {
        *self.counters.entry(name.to_string()).or_insert(0) += 1;
    }

    /// 设置计数器值
    pub fn set_counter(&mut self, name: &str, value: u64) {
        self.counters.insert(name.to_string(), value);
    }

    /// 获取计数器值
    pub fn get_counter(&self, name: &str) -> u64 {
        self.counters.get(name).copied().unwrap_or(0)
    }

    /// 打印所有统计信息
    pub fn print_stats(&self) {
        println!("=== Performance Statistics ===");

        if !self.counters.is_empty() {
            println!("Counters:");
            for (name, count) in &self.counters {
                println!("  {}: {}", name, count);
            }
        }

        if !self.timers.is_empty() {
            println!("Active Timers:");
            for (name, timer) in &self.timers {
                println!("  {}: {:.2}ms (running)", name, timer.elapsed_ms());
            }
        }
    }

    /// 重置所有统计信息
    pub fn reset(&mut self) {
        self.timers.clear();
        self.counters.clear();
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer() {
        let timer = Timer::new("test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(timer.elapsed_ms() >= 10.0);
    }

    #[test]
    fn test_audio_conversion() {
        let f32_samples = vec![0.5, -0.5, 1.0, -1.0];
        let i16_samples = audio_utils::f32_to_i16(&f32_samples);
        let converted_back = audio_utils::i16_to_f32(&i16_samples);

        for (original, converted) in f32_samples.iter().zip(converted_back.iter()) {
            assert!((original - converted).abs() < 0.001);
        }
    }

    #[test]
    fn test_rms_calculation() {
        let samples = vec![1.0, 0.0, -1.0, 0.0];
        let rms = audio_utils::calculate_rms(&samples);
        assert!((rms - 0.707).abs() < 0.01); // sqrt(0.5) ≈ 0.707
    }

    #[test]
    fn test_db_conversion() {
        let linear = 0.5;
        let db = audio_utils::linear_to_db(linear);
        let back_to_linear = audio_utils::db_to_linear(db);
        assert!((linear - back_to_linear).abs() < 0.001);
    }

    #[test]
    fn test_math_utils() {
        assert_eq!(math_utils::lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(math_utils::clamp(5.0, 0.0, 10.0), 5.0);
        assert_eq!(math_utils::clamp(-5.0, 0.0, 10.0), 0.0);
        assert_eq!(math_utils::clamp(15.0, 0.0, 10.0), 10.0);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((math_utils::cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        assert!((math_utils::cosine_similarity(&c, &d) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_file_size_formatting() {
        assert_eq!(fs_utils::format_file_size(512), "512 B");
        assert_eq!(fs_utils::format_file_size(1536), "1.5 KB");
        assert_eq!(fs_utils::format_file_size(1048576), "1.0 MB");
    }

    #[test]
    fn test_validation() {
        assert!(validation::validate_sample_rate(44100).is_ok());
        assert!(validation::validate_sample_rate(12345).is_err());

        assert!(validation::validate_pitch_shift(12).is_ok());
        assert!(validation::validate_pitch_shift(30).is_err());

        assert!(validation::validate_probability(0.5, "test").is_ok());
        assert!(validation::validate_probability(1.5, "test").is_err());
    }

    #[test]
    fn test_performance_monitor() {
        let mut monitor = PerformanceMonitor::new();

        monitor.start_timer("test");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = monitor.end_timer("test");

        assert!(elapsed.is_some());
        assert!(elapsed.unwrap() >= 10.0);

        monitor.increment_counter("calls");
        monitor.increment_counter("calls");
        assert_eq!(monitor.get_counter("calls"), 2);
    }

    #[test]
    fn test_moving_average() {
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let avg = math_utils::moving_average(&data, 3);
        assert!(!avg.is_empty());
        assert_eq!(avg.len(), 3); // 5 - 3 + 1 = 3
    }

    #[test]
    fn test_tensor_utils() {
        let device = Device::Cpu;
        let audio = vec![0.1, 0.2, 0.3];

        let tensor = tensor_utils::audio_to_tensor(&audio, device);
        assert_eq!(tensor.size(), &[3]);

        let converted_back = tensor_utils::tensor_to_audio(&tensor).unwrap();
        for (original, converted) in audio.iter().zip(converted_back.iter()) {
            assert!((original - converted).abs() < 0.001);
        }
    }
}
