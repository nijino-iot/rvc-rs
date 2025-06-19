//! F0 预测模块
//!
//! 实现基于 DIO、Harvest 和 PM 算法的基频（F0）提取功能
//! 对应 Python 代码中的 F0Predictor 相关类

use crate::{RvcError, RvcResult};
use std::f64::consts::PI;

/// F0 提取方法枚举
#[derive(Debug, Clone, PartialEq)]
pub enum F0Method {
    /// DIO (Distributed Inline Optimization) 方法
    Dio,
    /// Harvest 方法
    Harvest,
    /// PM (Pitch Marking) 方法
    Pm,
    /// RMVPE 方法
    Rmvpe,
    /// CREPE 方法
    Crepe,
    /// FCPE 方法
    Fcpe,
}

impl F0Method {
    /// 从字符串转换为 F0Method
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "dio" => Some(Self::Dio),
            "harvest" => Some(Self::Harvest),
            "pm" => Some(Self::Pm),
            "rmvpe" => Some(Self::Rmvpe),
            "crepe" => Some(Self::Crepe),
            "fcpe" => Some(Self::Fcpe),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Dio => "dio",
            Self::Harvest => "harvest",
            Self::Pm => "pm",
            Self::Rmvpe => "rmvpe",
            Self::Crepe => "crepe",
            Self::Fcpe => "fcpe",
        }
    }
}

/// F0 预测器特征
pub trait F0Predictor {
    /// 计算 F0
    fn compute_f0(&self, wav: &[f32], p_len: Option<usize>) -> RvcResult<Vec<f32>>;

    /// 计算 F0 和 UV (有声/无声) 标志
    fn compute_f0_uv(&self, wav: &[f32], p_len: Option<usize>) -> RvcResult<(Vec<f32>, Vec<f32>)>;

    /// 获取方法名称
    fn method(&self) -> F0Method;
}

/// DIO F0 预测器配置
#[derive(Debug, Clone)]
pub struct DioConfig {
    pub hop_length: usize,
    pub f0_min: f64,
    pub f0_max: f64,
    pub sampling_rate: u32,
    pub frame_period: f64,
    pub allowed_range: f64,
}

impl Default for DioConfig {
    fn default() -> Self {
        Self {
            hop_length: 512,
            f0_min: 50.0,
            f0_max: 1100.0,
            sampling_rate: 44100,
            frame_period: 5.0,
            allowed_range: 0.1,
        }
    }
}

/// DIO F0 预测器实现
pub struct DioF0Predictor {
    config: DioConfig,
}

impl DioF0Predictor {
    /// 创建新的 DIO F0 预测器
    pub fn new(config: DioConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建 DIO F0 预测器
    pub fn with_default() -> Self {
        Self::new(DioConfig::default())
    }

    /// F0 插值处理
    fn interpolate_f0(&self, f0: &[f64]) -> (Vec<f32>, Vec<f32>) {
        let mut ip_data = f0.to_vec();
        let mut vuv_vector = vec![0.0f32; f0.len()];

        // 设置有声/无声标志
        for (i, &val) in f0.iter().enumerate() {
            vuv_vector[i] = if val > 0.0 { 1.0 } else { 0.0 };
        }

        let frame_number = f0.len();
        let mut last_value = 0.0;

        for i in 0..frame_number {
            if f0[i] <= 0.0 {
                // 找到下一个有效值
                let mut j = i + 1;
                while j < frame_number && f0[j] <= 0.0 {
                    j += 1;
                }

                if j < frame_number {
                    if last_value > 0.0 {
                        // 线性插值
                        let step = (f0[j] - f0[i.saturating_sub(1)]) / (j - i) as f64;
                        for k in i..j {
                            ip_data[k] = f0[i.saturating_sub(1)] + step * (k - i + 1) as f64;
                        }
                    } else {
                        // 使用后面的值填充
                        for k in i..j {
                            ip_data[k] = f0[j];
                        }
                    }
                } else {
                    // 使用最后一个有效值填充
                    for k in i..frame_number {
                        ip_data[k] = last_value;
                    }
                }
            } else {
                ip_data[i] = f0[i];
                last_value = f0[i];
            }
        }

        let f0_float: Vec<f32> = ip_data.iter().map(|&x| x as f32).collect();
        (f0_float, vuv_vector)
    }

    /// 调整 F0 长度
    fn resize_f0(&self, f0: &[f64], target_len: usize) -> Vec<f64> {
        if f0.is_empty() || target_len == 0 {
            return vec![0.0; target_len];
        }

        let source_len = f0.len();
        if source_len == target_len {
            return f0.to_vec();
        }

        let mut result = vec![0.0; target_len];
        let ratio = source_len as f64 / target_len as f64;

        for i in 0..target_len {
            let src_idx = (i as f64 * ratio) as usize;
            if src_idx < source_len {
                result[i] = f0[src_idx];
            }
        }

        // 处理 NaN 和负值
        for val in result.iter_mut() {
            if val.is_nan() || *val < 0.001 {
                *val = 0.0;
            }
        }

        result
    }

    /// DIO 算法核心实现
    fn dio_core(&self, wav: &[f64]) -> RvcResult<(Vec<f64>, Vec<f64>)> {
        let fs = self.config.sampling_rate as f64;
        let frame_period = self.config.frame_period;
        let f0_floor = self.config.f0_min;
        let f0_ceil = self.config.f0_max;

        // 计算帧参数
        let frame_shift = (fs * frame_period / 1000.0).round() as usize;
        let num_frames =
            ((wav.len() as f64 - frame_shift as f64) / frame_shift as f64).ceil() as usize + 1;

        let mut f0 = vec![0.0; num_frames];
        let mut time_axis = vec![0.0; num_frames];

        // 生成时间轴
        for i in 0..num_frames {
            time_axis[i] = i as f64 * frame_period / 1000.0;
        }

        // DIO 算法主要步骤
        for i in 0..num_frames {
            let center = (i * frame_shift).min(wav.len().saturating_sub(1));
            let window_size = (fs / f0_floor * 2.0) as usize;

            let start = center.saturating_sub(window_size / 2);
            let end = (center + window_size / 2).min(wav.len());

            if end > start {
                let window = &wav[start..end];
                f0[i] = self.estimate_f0_dio(window, fs, f0_floor, f0_ceil);
            }
        }

        Ok((f0, time_axis))
    }

    /// DIO F0 估计
    fn estimate_f0_dio(&self, window: &[f64], fs: f64, f0_floor: f64, f0_ceil: f64) -> f64 {
        if window.len() < 4 {
            return 0.0;
        }

        let min_period = (fs / f0_ceil) as usize;
        let max_period = (fs / f0_floor) as usize;

        if window.len() < max_period * 2 {
            return 0.0;
        }

        // 零交叉率分析
        let zcr = self.calculate_zero_crossing_rate(window);
        if zcr > 0.5 {
            return 0.0; // 可能是噪音
        }

        // 自相关分析
        let autocorr = self.calculate_autocorrelation(window);
        let mut best_period = 0;
        let mut max_correlation = 0.0;

        for period in min_period..=max_period.min(autocorr.len() - 1) {
            let correlation = autocorr[period];
            if correlation > max_correlation {
                max_correlation = correlation;
                best_period = period;
            }
        }

        // 检查相关性阈值
        if max_correlation > 0.3 && best_period > 0 {
            // 精细化基频估计
            let refined_period = self.refine_period(window, best_period as f64, fs);
            fs / refined_period
        } else {
            0.0
        }
    }

    /// 计算零交叉率
    fn calculate_zero_crossing_rate(&self, signal: &[f64]) -> f64 {
        let mut crossings = 0;
        for i in 1..signal.len() {
            if (signal[i] >= 0.0) != (signal[i - 1] >= 0.0) {
                crossings += 1;
            }
        }
        crossings as f64 / (signal.len() - 1) as f64
    }

    /// 计算自相关
    fn calculate_autocorrelation(&self, signal: &[f64]) -> Vec<f64> {
        let n = signal.len();
        let mut autocorr = vec![0.0; n];

        for lag in 0..n {
            let mut sum = 0.0;
            let mut norm = 0.0;

            for i in 0..(n - lag) {
                sum += signal[i] * signal[i + lag];
                norm += signal[i] * signal[i];
            }

            if norm > 0.0 {
                autocorr[lag] = sum / norm;
            }
        }

        autocorr
    }

    /// 精细化周期估计
    fn refine_period(&self, signal: &[f64], initial_period: f64, _fs: f64) -> f64 {
        let search_range = initial_period * 0.1;
        let search_step = 0.1;

        let start = (initial_period - search_range).max(1.0);
        let end = initial_period + search_range;

        let mut best_period = initial_period;
        let mut max_correlation = 0.0;

        let mut period = start;
        while period <= end {
            let correlation = self.calculate_period_correlation(signal, period);
            if correlation > max_correlation {
                max_correlation = correlation;
                best_period = period;
            }
            period += search_step;
        }

        best_period
    }

    /// 计算特定周期的相关性
    fn calculate_period_correlation(&self, signal: &[f64], period: f64) -> f64 {
        let lag = period as usize;
        if lag >= signal.len() {
            return 0.0;
        }

        let mut correlation = 0.0;
        let mut norm1 = 0.0;
        let mut norm2 = 0.0;

        for i in 0..(signal.len() - lag) {
            let x1 = signal[i];
            let x2 = signal[i + lag];
            correlation += x1 * x2;
            norm1 += x1 * x1;
            norm2 += x2 * x2;
        }

        if norm1 > 0.0 && norm2 > 0.0 {
            correlation / (norm1 * norm2).sqrt()
        } else {
            0.0
        }
    }
}

impl F0Predictor for DioF0Predictor {
    fn compute_f0(&self, wav: &[f32], p_len: Option<usize>) -> RvcResult<Vec<f32>> {
        let p_len = p_len.unwrap_or(wav.len() / self.config.hop_length);

        // 转换为 f64
        let wav_f64: Vec<f64> = wav.iter().map(|&x| x as f64).collect();

        // 执行 DIO 算法
        let (mut f0, _time_axis) = self.dio_core(&wav_f64)?;

        // 四舍五入到一位小数
        for f in f0.iter_mut() {
            *f = (*f * 10.0).round() / 10.0;
        }

        // 调整长度
        let resized_f0 = self.resize_f0(&f0, p_len);

        // 插值处理
        let (interpolated_f0, _uv) = self.interpolate_f0(&resized_f0);

        Ok(interpolated_f0)
    }

    fn compute_f0_uv(&self, wav: &[f32], p_len: Option<usize>) -> RvcResult<(Vec<f32>, Vec<f32>)> {
        let p_len = p_len.unwrap_or(wav.len() / self.config.hop_length);

        // 转换为 f64
        let wav_f64: Vec<f64> = wav.iter().map(|&x| x as f64).collect();

        // 执行 DIO 算法
        let (mut f0, _time_axis) = self.dio_core(&wav_f64)?;

        // 四舍五入到一位小数
        for f in f0.iter_mut() {
            *f = (*f * 10.0).round() / 10.0;
        }

        // 调整长度
        let resized_f0 = self.resize_f0(&f0, p_len);

        // 插值处理
        let (interpolated_f0, uv) = self.interpolate_f0(&resized_f0);

        Ok((interpolated_f0, uv))
    }

    fn method(&self) -> F0Method {
        F0Method::Dio
    }
}

/// Harvest F0 预测器配置
#[derive(Debug, Clone)]
pub struct HarvestConfig {
    pub hop_length: usize,
    pub f0_min: f64,
    pub f0_max: f64,
    pub sampling_rate: u32,
    pub frame_period: f64,
}

impl Default for HarvestConfig {
    fn default() -> Self {
        Self {
            hop_length: 512,
            f0_min: 50.0,
            f0_max: 1100.0,
            sampling_rate: 44100,
            frame_period: 5.0,
        }
    }
}

/// Harvest F0 预测器实现
pub struct HarvestF0Predictor {
    config: HarvestConfig,
}

impl HarvestF0Predictor {
    /// 创建新的 Harvest F0 预测器
    pub fn new(config: HarvestConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建 Harvest F0 预测器
    pub fn with_default() -> Self {
        Self::new(HarvestConfig::default())
    }

    /// F0 插值处理（与 DIO 相同）
    fn interpolate_f0(&self, f0: &[f64]) -> (Vec<f32>, Vec<f32>) {
        let mut ip_data = f0.to_vec();
        let mut vuv_vector = vec![0.0f32; f0.len()];

        // 设置有声/无声标志
        for (i, &val) in f0.iter().enumerate() {
            vuv_vector[i] = if val > 0.0 { 1.0 } else { 0.0 };
        }

        let frame_number = f0.len();
        let mut last_value = 0.0;

        for i in 0..frame_number {
            if f0[i] <= 0.0 {
                let mut j = i + 1;
                while j < frame_number && f0[j] <= 0.0 {
                    j += 1;
                }

                if j < frame_number {
                    if last_value > 0.0 {
                        let step = (f0[j] - f0[i.saturating_sub(1)]) / (j - i) as f64;
                        for k in i..j {
                            ip_data[k] = f0[i.saturating_sub(1)] + step * (k - i + 1) as f64;
                        }
                    } else {
                        for k in i..j {
                            ip_data[k] = f0[j];
                        }
                    }
                } else {
                    for k in i..frame_number {
                        ip_data[k] = last_value;
                    }
                }
            } else {
                ip_data[i] = f0[i];
                last_value = f0[i];
            }
        }

        let f0_float: Vec<f32> = ip_data.iter().map(|&x| x as f32).collect();
        (f0_float, vuv_vector)
    }

    /// 调整 F0 长度（与 DIO 相同）
    fn resize_f0(&self, f0: &[f64], target_len: usize) -> Vec<f64> {
        if f0.is_empty() || target_len == 0 {
            return vec![0.0; target_len];
        }

        let source_len = f0.len();
        if source_len == target_len {
            return f0.to_vec();
        }

        let mut result = vec![0.0; target_len];
        let ratio = source_len as f64 / target_len as f64;

        for i in 0..target_len {
            let src_idx = (i as f64 * ratio) as usize;
            if src_idx < source_len {
                result[i] = f0[src_idx];
            }
        }

        for val in result.iter_mut() {
            if val.is_nan() || *val < 0.001 {
                *val = 0.0;
            }
        }

        result
    }

    /// Harvest 算法核心实现
    fn harvest_core(&self, wav: &[f64]) -> RvcResult<(Vec<f64>, Vec<f64>)> {
        let fs = self.config.sampling_rate as f64;
        let frame_period = self.config.frame_period;
        let f0_floor = self.config.f0_min;
        let f0_ceil = self.config.f0_max;

        let frame_shift = (fs * frame_period / 1000.0).round() as usize;
        let num_frames =
            ((wav.len() as f64 - frame_shift as f64) / frame_shift as f64).ceil() as usize + 1;

        let mut f0 = vec![0.0; num_frames];
        let mut time_axis = vec![0.0; num_frames];

        for i in 0..num_frames {
            time_axis[i] = i as f64 * frame_period / 1000.0;
        }

        // Harvest 算法实现
        for i in 0..num_frames {
            let center = (i * frame_shift).min(wav.len().saturating_sub(1));
            let window_size = (fs / f0_floor * 3.0) as usize;

            let start = center.saturating_sub(window_size / 2);
            let end = (center + window_size / 2).min(wav.len());

            if end > start {
                let window = &wav[start..end];
                f0[i] = self.estimate_f0_harvest(window, fs, f0_floor, f0_ceil);
            }
        }

        Ok((f0, time_axis))
    }

    /// Harvest F0 估计
    fn estimate_f0_harvest(&self, window: &[f64], fs: f64, f0_floor: f64, f0_ceil: f64) -> f64 {
        if window.len() < 4 {
            return 0.0;
        }

        // 计算瞬时频率
        let instantaneous_freq = self.calculate_instantaneous_frequency(window, fs);
        if instantaneous_freq < f0_floor || instantaneous_freq > f0_ceil {
            return 0.0;
        }

        // 使用更精确的方法估计基频
        let filtered_signal = self.apply_bandpass_filter(window, f0_floor, f0_ceil, fs);
        let estimated_f0 =
            self.estimate_fundamental_frequency(&filtered_signal, fs, f0_floor, f0_ceil);

        estimated_f0
    }

    /// 计算瞬时频率
    fn calculate_instantaneous_frequency(&self, signal: &[f64], fs: f64) -> f64 {
        if signal.len() < 3 {
            return 0.0;
        }

        let mut phase_diff_sum = 0.0;
        let mut count = 0;

        for i in 1..signal.len() - 1 {
            let derivative = (signal[i + 1] - signal[i - 1]) / 2.0;
            if signal[i].abs() > 1e-10 {
                let phase_diff = derivative / signal[i];
                phase_diff_sum += phase_diff.abs();
                count += 1;
            }
        }

        if count > 0 {
            (phase_diff_sum / count as f64) * fs / (2.0 * PI)
        } else {
            0.0
        }
    }

    /// 应用带通滤波器（简化实现）
    fn apply_bandpass_filter(&self, signal: &[f64], _f_low: f64, f_high: f64, fs: f64) -> Vec<f64> {
        // 简化的带通滤波器实现
        let mut filtered = signal.to_vec();

        // 简单的移动平均滤波
        let window_size = (fs / f_high) as usize;
        if window_size > 0 && window_size < signal.len() {
            for i in window_size..signal.len() - window_size {
                let mut sum = 0.0;
                for j in i - window_size..=i + window_size {
                    sum += signal[j];
                }
                filtered[i] = sum / (2 * window_size + 1) as f64;
            }
        }

        filtered
    }

    /// 估计基频
    fn estimate_fundamental_frequency(
        &self,
        signal: &[f64],
        fs: f64,
        f0_floor: f64,
        f0_ceil: f64,
    ) -> f64 {
        let min_period = (fs / f0_ceil) as usize;
        let max_period = (fs / f0_floor) as usize;

        if signal.len() < max_period * 2 {
            return 0.0;
        }

        let autocorr = self.calculate_normalized_autocorrelation(signal);
        let mut best_period = 0;
        let mut max_correlation = 0.0;

        for period in min_period..=max_period.min(autocorr.len() - 1) {
            let correlation = autocorr[period];
            if correlation > max_correlation {
                max_correlation = correlation;
                best_period = period;
            }
        }

        if max_correlation > 0.4 && best_period > 0 {
            fs / best_period as f64
        } else {
            0.0
        }
    }

    /// 计算标准化自相关
    fn calculate_normalized_autocorrelation(&self, signal: &[f64]) -> Vec<f64> {
        let n = signal.len();
        let mut autocorr = vec![0.0; n];

        for lag in 0..n {
            let mut correlation = 0.0;
            let mut norm_x = 0.0;
            let mut norm_y = 0.0;

            for i in 0..(n - lag) {
                let x = signal[i];
                let y = signal[i + lag];
                correlation += x * y;
                norm_x += x * x;
                norm_y += y * y;
            }

            if norm_x > 0.0 && norm_y > 0.0 {
                autocorr[lag] = correlation / (norm_x * norm_y).sqrt();
            }
        }

        autocorr
    }
}

impl F0Predictor for HarvestF0Predictor {
    fn compute_f0(&self, wav: &[f32], p_len: Option<usize>) -> RvcResult<Vec<f32>> {
        let p_len = p_len.unwrap_or(wav.len() / self.config.hop_length);

        let wav_f64: Vec<f64> = wav.iter().map(|&x| x as f64).collect();
        let (mut f0, _time_axis) = self.harvest_core(&wav_f64)?;

        for f in f0.iter_mut() {
            *f = (*f * 10.0).round() / 10.0;
        }

        let resized_f0 = self.resize_f0(&f0, p_len);
        let (interpolated_f0, _uv) = self.interpolate_f0(&resized_f0);

        Ok(interpolated_f0)
    }

    fn compute_f0_uv(&self, wav: &[f32], p_len: Option<usize>) -> RvcResult<(Vec<f32>, Vec<f32>)> {
        let p_len = p_len.unwrap_or(wav.len() / self.config.hop_length);

        let wav_f64: Vec<f64> = wav.iter().map(|&x| x as f64).collect();
        let (mut f0, _time_axis) = self.harvest_core(&wav_f64)?;

        for f in f0.iter_mut() {
            *f = (*f * 10.0).round() / 10.0;
        }

        let resized_f0 = self.resize_f0(&f0, p_len);
        let (interpolated_f0, uv) = self.interpolate_f0(&resized_f0);

        Ok((interpolated_f0, uv))
    }

    fn method(&self) -> F0Method {
        F0Method::Harvest
    }
}

/// PM F0 预测器配置
#[derive(Debug, Clone)]
pub struct PmConfig {
    pub hop_length: usize,
    pub f0_min: f64,
    pub f0_max: f64,
    pub sampling_rate: u32,
}

impl Default for PmConfig {
    fn default() -> Self {
        Self {
            hop_length: 512,
            f0_min: 50.0,
            f0_max: 1100.0,
            sampling_rate: 44100,
        }
    }
}

/// PM F0 预测器实现
pub struct PmF0Predictor {
    config: PmConfig,
}

impl PmF0Predictor {
    /// 创建新的 PM F0 预测器
    pub fn new(config: PmConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建 PM F0 预测器
    pub fn with_default() -> Self {
        Self::new(PmConfig::default())
    }

    /// F0 插值处理
    fn interpolate_f0(&self, f0: &[f64]) -> (Vec<f32>, Vec<f32>) {
        let mut ip_data = f0.to_vec();
        let mut vuv_vector = vec![0.0f32; f0.len()];

        for (i, &val) in f0.iter().enumerate() {
            vuv_vector[i] = if val > 0.0 { 1.0 } else { 0.0 };
        }

        let frame_number = f0.len();
        let mut last_value = 0.0;

        for i in 0..frame_number {
            if f0[i] <= 0.0 {
                let mut j = i + 1;
                while j < frame_number && f0[j] <= 0.0 {
                    j += 1;
                }

                if j < frame_number {
                    if last_value > 0.0 {
                        let step = (f0[j] - f0[i.saturating_sub(1)]) / (j - i) as f64;
                        for k in i..j {
                            ip_data[k] = f0[i.saturating_sub(1)] + step * (k - i + 1) as f64;
                        }
                    } else {
                        for k in i..j {
                            ip_data[k] = f0[j];
                        }
                    }
                } else {
                    for k in i..frame_number {
                        ip_data[k] = last_value;
                    }
                }
            } else {
                ip_data[i] = f0[i];
                last_value = f0[i];
            }
        }

        let f0_float: Vec<f32> = ip_data.iter().map(|&x| x as f32).collect();
        (f0_float, vuv_vector)
    }

    /// PM 算法核心实现
    fn pm_core(&self, wav: &[f64]) -> RvcResult<Vec<f64>> {
        let fs = self.config.sampling_rate as f64;
        let hop_length = self.config.hop_length;
        let f0_min = self.config.f0_min;
        let f0_max = self.config.f0_max;

        let n_frames = wav.len() / hop_length;
        let mut f0 = vec![0.0; n_frames];

        for i in 0..n_frames {
            let start = i * hop_length;
            let end = (start + hop_length * 2).min(wav.len());

            if end > start {
                let frame = &wav[start..end];
                f0[i] = self.estimate_f0_pm(frame, fs, f0_min, f0_max);
            }
        }

        Ok(f0)
    }

    /// PM F0 估计
    fn estimate_f0_pm(&self, frame: &[f64], fs: f64, f0_min: f64, f0_max: f64) -> f64 {
        if frame.len() < 4 {
            return 0.0;
        }

        let min_period = (fs / f0_max) as usize;
        let max_period = (fs / f0_min) as usize;

        if frame.len() < max_period {
            return 0.0;
        }

        // 基于峰值检测的PM方法
        let peaks = self.find_peaks(frame);
        if peaks.len() < 2 {
            return 0.0;
        }

        // 计算平均周期
        let mut periods = Vec::new();
        for i in 1..peaks.len() {
            let period = peaks[i] - peaks[i - 1];
            if period >= min_period && period <= max_period {
                periods.push(period);
            }
        }

        if periods.is_empty() {
            return 0.0;
        }

        let avg_period = periods.iter().sum::<usize>() as f64 / periods.len() as f64;
        fs / avg_period
    }

    /// 寻找峰值
    fn find_peaks(&self, signal: &[f64]) -> Vec<usize> {
        let mut peaks = Vec::new();
        let threshold = signal.iter().map(|x| x.abs()).fold(0.0, f64::max) * 0.1;

        for i in 1..signal.len() - 1 {
            if signal[i] > signal[i - 1] && signal[i] > signal[i + 1] && signal[i] > threshold {
                peaks.push(i);
            }
        }

        peaks
    }
}

impl F0Predictor for PmF0Predictor {
    fn compute_f0(&self, wav: &[f32], p_len: Option<usize>) -> RvcResult<Vec<f32>> {
        let p_len = p_len.unwrap_or(wav.len() / self.config.hop_length);

        let wav_f64: Vec<f64> = wav.iter().map(|&x| x as f64).collect();
        let mut f0 = self.pm_core(&wav_f64)?;

        // 调整长度到目标长度
        if f0.len() != p_len {
            let mut resized_f0 = vec![0.0; p_len];
            let ratio = f0.len() as f64 / p_len as f64;

            for i in 0..p_len {
                let src_idx = (i as f64 * ratio) as usize;
                if src_idx < f0.len() {
                    resized_f0[i] = f0[src_idx];
                }
            }
            f0 = resized_f0;
        }

        let (interpolated_f0, _uv) = self.interpolate_f0(&f0);
        Ok(interpolated_f0)
    }

    fn compute_f0_uv(&self, wav: &[f32], p_len: Option<usize>) -> RvcResult<(Vec<f32>, Vec<f32>)> {
        let p_len = p_len.unwrap_or(wav.len() / self.config.hop_length);

        let wav_f64: Vec<f64> = wav.iter().map(|&x| x as f64).collect();
        let mut f0 = self.pm_core(&wav_f64)?;

        // 调整长度到目标长度
        if f0.len() != p_len {
            let mut resized_f0 = vec![0.0; p_len];
            let ratio = f0.len() as f64 / p_len as f64;

            for i in 0..p_len {
                let src_idx = (i as f64 * ratio) as usize;
                if src_idx < f0.len() {
                    resized_f0[i] = f0[src_idx];
                }
            }
            f0 = resized_f0;
        }

        let (interpolated_f0, uv) = self.interpolate_f0(&f0);
        Ok((interpolated_f0, uv))
    }

    fn method(&self) -> F0Method {
        F0Method::Pm
    }
}

/// F0 预测器工厂
pub struct F0PredictorFactory;

impl F0PredictorFactory {
    /// 创建 F0 预测器
    pub fn create(
        method: F0Method,
        sampling_rate: Option<u32>,
        hop_length: Option<usize>,
        f0_min: Option<f64>,
        f0_max: Option<f64>,
    ) -> RvcResult<Box<dyn F0Predictor + Send + Sync>> {
        let sampling_rate = sampling_rate.unwrap_or(44100);
        let hop_length = hop_length.unwrap_or(512);
        let f0_min = f0_min.unwrap_or(50.0);
        let f0_max = f0_max.unwrap_or(1100.0);

        match method {
            F0Method::Dio => {
                let config = DioConfig {
                    hop_length,
                    f0_min,
                    f0_max,
                    sampling_rate,
                    frame_period: 5.0,
                    allowed_range: 0.1,
                };
                Ok(Box::new(DioF0Predictor::new(config)))
            }
            F0Method::Harvest => {
                let config = HarvestConfig {
                    hop_length,
                    f0_min,
                    f0_max,
                    sampling_rate,
                    frame_period: 5.0,
                };
                Ok(Box::new(HarvestF0Predictor::new(config)))
            }
            F0Method::Pm => {
                let config = PmConfig {
                    hop_length,
                    f0_min,
                    f0_max,
                    sampling_rate,
                };
                Ok(Box::new(PmF0Predictor::new(config)))
            }
            _ => Err(RvcError::f0(format!(
                "F0 method {:?} not implemented yet",
                method
            ))),
        }
    }

    /// 使用默认配置创建 F0 预测器
    pub fn create_default(method: F0Method) -> RvcResult<Box<dyn F0Predictor + Send + Sync>> {
        Self::create(method, None, None, None, None)
    }
}
