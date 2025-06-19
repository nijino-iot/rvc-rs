//! 简化的 WORLD 算法实现
//!
//! 提供类似 PyWorld 的基频提取功能，包括 DIO 和 Harvest 算法的简化版本

use crate::RvcResult;
use rustfft::{num_complex::Complex, FftPlanner};
use std::f64::consts::PI;

/// WORLD 算法配置
#[derive(Debug, Clone)]
pub struct WorldConfig {
    pub fs: f64,           // 采样率
    pub f0_floor: f64,     // 最小基频
    pub f0_ceil: f64,      // 最大基频
    pub frame_period: f64, // 帧周期 (ms)
    pub speed: i32,        // 速度参数 (1 = 最快，默认 1)
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            fs: 44100.0,
            f0_floor: 71.0,
            f0_ceil: 800.0,
            frame_period: 5.0,
            speed: 1,
        }
    }
}

/// DIO 算法实现
pub struct DioAlgorithm {
    config: WorldConfig,
    fft_planner: FftPlanner<f64>,
}

impl DioAlgorithm {
    /// 创建新的 DIO 算法实例
    pub fn new(config: WorldConfig) -> Self {
        Self {
            config,
            fft_planner: FftPlanner::new(),
        }
    }

    /// 执行 DIO 算法
    pub fn estimate_f0(&mut self, x: &[f64]) -> RvcResult<(Vec<f64>, Vec<f64>)> {
        let x_length = x.len();
        let frame_shift = (self.config.fs * self.config.frame_period / 1000.0).round() as usize;
        let number_of_frames = Self::get_number_of_frames(x_length, frame_shift);

        // 时间轴
        let mut time_axis = vec![0.0; number_of_frames];
        for i in 0..number_of_frames {
            time_axis[i] = i as f64 * self.config.frame_period / 1000.0;
        }

        // 基频估计
        let mut f0 = vec![0.0; number_of_frames];

        // 计算帧窗长度
        let window_length = self.get_window_length();
        let half_window_length = window_length / 2;

        for i in 0..number_of_frames {
            let current_position = (i * frame_shift).min(x_length - 1);
            let safe_index = current_position;

            // 提取窗口
            let start_index = safe_index.saturating_sub(half_window_length);
            let end_index = (safe_index + half_window_length).min(x_length);

            if end_index > start_index {
                let window = &x[start_index..end_index];
                f0[i] = self.get_f0_candidate(window, safe_index - start_index)?;
            }
        }

        // 后处理：移除不合理的F0值
        self.refine_f0(&mut f0);

        Ok((f0, time_axis))
    }

    /// 获取帧数
    fn get_number_of_frames(x_length: usize, frame_shift: usize) -> usize {
        ((x_length as f64 - frame_shift as f64) / frame_shift as f64).ceil() as usize + 1
    }

    /// 获取窗口长度
    fn get_window_length(&self) -> usize {
        (self.config.fs / self.config.f0_floor * 2.0) as usize
    }

    /// 获取F0候选值
    fn get_f0_candidate(&mut self, x: &[f64], center_index: usize) -> RvcResult<f64> {
        let x_length = x.len();
        if x_length < 4 || center_index >= x_length {
            return Ok(0.0);
        }

        // 零填充到合适的FFT长度
        let fft_size = self.get_suitable_fft_size(x_length);
        let mut x_windowed = vec![0.0; fft_size];

        // 应用窗函数并复制数据
        let start = center_index.saturating_sub(x_length / 2);
        let end = (start + x_length).min(x.len());

        for i in 0..(end - start) {
            if i < x_windowed.len() {
                // 简单的汉宁窗
                let window_value = 0.5 - 0.5 * (2.0 * PI * i as f64 / (end - start) as f64).cos();
                x_windowed[i] = x[start + i] * window_value;
            }
        }

        // 计算功率谱
        let power_spectrum = self.compute_power_spectrum(&x_windowed)?;

        // 寻找峰值
        let f0_candidate = self.find_f0_from_spectrum(&power_spectrum, fft_size)?;

        Ok(f0_candidate)
    }

    /// 获取合适的FFT大小
    fn get_suitable_fft_size(&self, minimum_length: usize) -> usize {
        let mut fft_size = 1;
        while fft_size < minimum_length * 2 {
            fft_size *= 2;
        }
        fft_size.max(512)
    }

    /// 计算功率谱
    fn compute_power_spectrum(&mut self, x: &[f64]) -> RvcResult<Vec<f64>> {
        let fft_size = x.len();
        let mut input: Vec<Complex<f64>> = x.iter().map(|&val| Complex::new(val, 0.0)).collect();

        let fft = self.fft_planner.plan_fft_forward(fft_size);
        fft.process(&mut input);

        // 计算功率谱
        let power_spectrum: Vec<f64> = input
            .iter()
            .take(fft_size / 2 + 1)
            .map(|c| c.norm_sqr())
            .collect();

        Ok(power_spectrum)
    }

    /// 从功率谱中寻找F0
    fn find_f0_from_spectrum(&self, power_spectrum: &[f64], fft_size: usize) -> RvcResult<f64> {
        let frequency_resolution = self.config.fs / fft_size as f64;

        // 转换F0范围到频率索引
        let min_index = (self.config.f0_floor / frequency_resolution) as usize;
        let max_index =
            ((self.config.f0_ceil / frequency_resolution) as usize).min(power_spectrum.len() - 1);

        if min_index >= max_index || max_index >= power_spectrum.len() {
            return Ok(0.0);
        }

        // 寻找功率谱峰值
        let mut max_power = 0.0;
        let mut best_index = 0;

        for i in min_index..=max_index {
            if power_spectrum[i] > max_power {
                max_power = power_spectrum[i];
                best_index = i;
            }
        }

        // 检查是否找到有效峰值
        if max_power < 1e-10 {
            return Ok(0.0);
        }

        // 抛物线插值提高精度
        let refined_index = if best_index > 0 && best_index < power_spectrum.len() - 1 {
            let y1 = power_spectrum[best_index - 1].ln();
            let y2 = power_spectrum[best_index].ln();
            let y3 = power_spectrum[best_index + 1].ln();

            let a = (y1 - 2.0 * y2 + y3) / 2.0;
            if a.abs() > 1e-10 {
                let shift = (y1 - y3) / (4.0 * a);
                best_index as f64 + shift
            } else {
                best_index as f64
            }
        } else {
            best_index as f64
        };

        let f0 = refined_index * frequency_resolution;

        // 检查F0是否在有效范围内
        if f0 >= self.config.f0_floor && f0 <= self.config.f0_ceil {
            Ok(f0)
        } else {
            Ok(0.0)
        }
    }

    /// 精化F0序列
    fn refine_f0(&self, f0: &mut [f64]) {
        // 简单的中值滤波
        let window_size = 3;
        let mut refined_f0 = f0.to_vec();

        for i in window_size / 2..f0.len() - window_size / 2 {
            let mut window: Vec<f64> = f0[i - window_size / 2..=i + window_size / 2]
                .iter()
                .filter(|&&x| x > 0.0) // 只考虑有声段
                .copied()
                .collect();

            if !window.is_empty() {
                window.sort_by(|a, b| a.partial_cmp(b).unwrap());
                refined_f0[i] = window[window.len() / 2]; // 中值
            }
        }

        f0.copy_from_slice(&refined_f0);
    }
}

/// Harvest 算法实现
pub struct HarvestAlgorithm {
    config: WorldConfig,
    fft_planner: FftPlanner<f64>,
}

impl HarvestAlgorithm {
    /// 创建新的 Harvest 算法实例
    pub fn new(config: WorldConfig) -> Self {
        Self {
            config,
            fft_planner: FftPlanner::new(),
        }
    }

    /// 执行 Harvest 算法
    pub fn estimate_f0(&mut self, x: &[f64]) -> RvcResult<(Vec<f64>, Vec<f64>)> {
        let x_length = x.len();
        let frame_shift = (self.config.fs * self.config.frame_period / 1000.0).round() as usize;
        let number_of_frames = DioAlgorithm::get_number_of_frames(x_length, frame_shift);

        // 时间轴
        let mut time_axis = vec![0.0; number_of_frames];
        for i in 0..number_of_frames {
            time_axis[i] = i as f64 * self.config.frame_period / 1000.0;
        }

        // 使用更精确的方法估计F0
        let mut f0 = vec![0.0; number_of_frames];

        for i in 0..number_of_frames {
            let current_position = (i * frame_shift).min(x_length - 1);
            f0[i] = self.harvest_f0_estimation(x, current_position)?;
        }

        // 后处理
        self.post_process(&mut f0);

        Ok((f0, time_axis))
    }

    /// Harvest F0估计
    fn harvest_f0_estimation(&mut self, x: &[f64], center: usize) -> RvcResult<f64> {
        let window_length = (self.config.fs / self.config.f0_floor * 3.0) as usize;
        let half_window = window_length / 2;

        let start = center.saturating_sub(half_window);
        let end = (center + half_window).min(x.len());

        if end <= start {
            return Ok(0.0);
        }

        let window = &x[start..end];

        // 计算瞬时频率
        let instantaneous_frequency = self.compute_instantaneous_frequency(window)?;

        // 验证频率是否在有效范围内
        if instantaneous_frequency >= self.config.f0_floor
            && instantaneous_frequency <= self.config.f0_ceil
        {
            Ok(instantaneous_frequency)
        } else {
            Ok(0.0)
        }
    }

    /// 计算瞬时频率
    fn compute_instantaneous_frequency(&mut self, x: &[f64]) -> RvcResult<f64> {
        let x_length = x.len();
        if x_length < 3 {
            return Ok(0.0);
        }

        // 计算解析信号
        let analytic_signal = self.compute_analytic_signal(x)?;

        // 计算瞬时相位
        let mut instantaneous_phase = vec![0.0; x_length];
        for i in 0..x_length {
            instantaneous_phase[i] = analytic_signal[i].arg();
        }

        // 计算相位差分（瞬时频率）
        let mut frequency_sum = 0.0;
        let mut count = 0;

        for i in 1..x_length {
            let mut phase_diff = instantaneous_phase[i] - instantaneous_phase[i - 1];

            // 解决相位包装问题
            while phase_diff > PI {
                phase_diff -= 2.0 * PI;
            }
            while phase_diff < -PI {
                phase_diff += 2.0 * PI;
            }

            let freq = phase_diff * self.config.fs / (2.0 * PI);
            if freq > 0.0 && freq >= self.config.f0_floor && freq <= self.config.f0_ceil {
                frequency_sum += freq;
                count += 1;
            }
        }

        if count > 0 {
            Ok(frequency_sum / count as f64)
        } else {
            Ok(0.0)
        }
    }

    /// 计算解析信号（使用Hilbert变换）
    fn compute_analytic_signal(&mut self, x: &[f64]) -> RvcResult<Vec<Complex<f64>>> {
        let x_length = x.len();
        let fft_size = self.get_fft_size(x_length);

        // 零填充
        let mut x_padded = vec![0.0; fft_size];
        x_padded[..x_length].copy_from_slice(x);

        // FFT
        let mut spectrum: Vec<Complex<f64>> =
            x_padded.iter().map(|&val| Complex::new(val, 0.0)).collect();

        let fft = self.fft_planner.plan_fft_forward(fft_size);
        fft.process(&mut spectrum);

        // Hilbert变换：将负频率成分置零，正频率成分加倍
        for i in 1..fft_size / 2 {
            spectrum[i] *= 2.0;
        }
        for i in fft_size / 2 + 1..fft_size {
            spectrum[i] = Complex::new(0.0, 0.0);
        }

        // IFFT
        let ifft = self.fft_planner.plan_fft_inverse(fft_size);
        ifft.process(&mut spectrum);

        // 归一化并截取原始长度
        let mut result = vec![Complex::new(0.0, 0.0); x_length];
        for i in 0..x_length {
            result[i] = spectrum[i] / fft_size as f64;
        }

        Ok(result)
    }

    /// 获取FFT大小
    fn get_fft_size(&self, length: usize) -> usize {
        let mut size = 1;
        while size < length * 2 {
            size *= 2;
        }
        size
    }

    /// 后处理F0序列
    fn post_process(&self, f0: &mut [f64]) {
        // 移除孤立的F0值
        let mut cleaned_f0 = f0.to_vec();

        for i in 1..f0.len() - 1 {
            if f0[i] > 0.0 && f0[i - 1] == 0.0 && f0[i + 1] == 0.0 {
                // 孤立的有声帧，可能是噪声
                if (f0[i] - self.config.f0_floor).abs() < 50.0
                    || (f0[i] - self.config.f0_ceil).abs() < 50.0
                {
                    cleaned_f0[i] = 0.0;
                }
            }
        }

        // 平滑处理
        for i in 1..cleaned_f0.len() - 1 {
            if cleaned_f0[i] > 0.0 && cleaned_f0[i - 1] > 0.0 && cleaned_f0[i + 1] > 0.0 {
                // 简单的3点平滑
                cleaned_f0[i] = (cleaned_f0[i - 1] + cleaned_f0[i] + cleaned_f0[i + 1]) / 3.0;
            }
        }

        f0.copy_from_slice(&cleaned_f0);
    }
}

/// StoneMask 后处理
pub struct StoneMask {
    config: WorldConfig,
}

impl StoneMask {
    /// 创建新的 StoneMask 实例
    pub fn new(config: WorldConfig) -> Self {
        Self { config }
    }

    /// 应用 StoneMask 后处理
    pub fn refine_f0(&self, x: &[f64], rough_f0: &[f64], time_axis: &[f64]) -> RvcResult<Vec<f64>> {
        let mut refined_f0 = rough_f0.to_vec();

        for i in 0..refined_f0.len() {
            if rough_f0[i] > 0.0 {
                // 在原始信号中寻找更精确的F0
                let time_index = (time_axis[i] * self.config.fs) as usize;
                if time_index < x.len() {
                    let refined_value = self.refine_single_f0(x, time_index, rough_f0[i])?;
                    refined_f0[i] = refined_value;
                }
            }
        }

        Ok(refined_f0)
    }

    /// 精化单个F0值
    fn refine_single_f0(&self, x: &[f64], center: usize, rough_f0: f64) -> RvcResult<f64> {
        let search_range = rough_f0 * 0.05; // 5%的搜索范围
        let search_step = rough_f0 * 0.001; // 0.1%的搜索步长

        let window_length = (self.config.fs / rough_f0 * 2.0) as usize;
        let half_window = window_length / 2;

        let start = center.saturating_sub(half_window);
        let end = (center + half_window).min(x.len());

        if end <= start {
            return Ok(rough_f0);
        }

        let window = &x[start..end];

        let mut best_f0 = rough_f0;
        let mut best_correlation = 0.0;

        let mut f0_candidate = rough_f0 - search_range;
        while f0_candidate <= rough_f0 + search_range {
            let correlation = self.compute_normalized_correlation(window, f0_candidate)?;
            if correlation > best_correlation {
                best_correlation = correlation;
                best_f0 = f0_candidate;
            }
            f0_candidate += search_step;
        }

        Ok(best_f0)
    }

    /// 计算归一化相关性
    fn compute_normalized_correlation(&self, x: &[f64], f0: f64) -> RvcResult<f64> {
        let period = (self.config.fs / f0) as usize;
        if period >= x.len() {
            return Ok(0.0);
        }

        let mut correlation = 0.0;
        let mut norm_x = 0.0;
        let mut norm_y = 0.0;

        for i in 0..(x.len() - period) {
            let x1 = x[i];
            let x2 = x[i + period];
            correlation += x1 * x2;
            norm_x += x1 * x1;
            norm_y += x2 * x2;
        }

        if norm_x > 0.0 && norm_y > 0.0 {
            Ok(correlation / (norm_x * norm_y).sqrt())
        } else {
            Ok(0.0)
        }
    }
}

/// 便捷函数：DIO算法
pub fn dio(
    x: &[f64],
    fs: f64,
    f0_floor: f64,
    f0_ceil: f64,
    frame_period: f64,
) -> RvcResult<(Vec<f64>, Vec<f64>)> {
    let config = WorldConfig {
        fs,
        f0_floor,
        f0_ceil,
        frame_period,
        speed: 1,
    };

    let mut dio = DioAlgorithm::new(config);
    dio.estimate_f0(x)
}

/// 便捷函数：Harvest算法
pub fn harvest(
    x: &[f64],
    fs: f64,
    f0_floor: f64,
    f0_ceil: f64,
    frame_period: f64,
) -> RvcResult<(Vec<f64>, Vec<f64>)> {
    let config = WorldConfig {
        fs,
        f0_floor,
        f0_ceil,
        frame_period,
        speed: 1,
    };

    let mut harvest = HarvestAlgorithm::new(config);
    harvest.estimate_f0(x)
}

/// 便捷函数：StoneMask后处理
pub fn stonemask(x: &[f64], f0: &[f64], time_axis: &[f64], fs: f64) -> RvcResult<Vec<f64>> {
    let config = WorldConfig {
        fs,
        f0_floor: 71.0,
        f0_ceil: 800.0,
        frame_period: 5.0,
        speed: 1,
    };

    let stonemask = StoneMask::new(config);
    stonemask.refine_f0(x, f0, time_axis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dio_algorithm() -> RvcResult<()> {
        let fs = 16000.0;
        let duration = 0.5;
        let f0_target = 220.0;

        // 生成测试信号
        let n_samples = (fs * duration) as usize;
        let mut signal = vec![0.0; n_samples];

        for i in 0..n_samples {
            let t = i as f64 / fs;
            signal[i] = (2.0 * PI * f0_target * t).sin();
        }

        let config = WorldConfig {
            fs,
            f0_floor: 50.0,
            f0_ceil: 500.0,
            frame_period: 10.0,
            speed: 1,
        };

        let mut dio = DioAlgorithm::new(config);
        let (f0, time_axis) = dio.estimate_f0(&signal)?;

        assert!(!f0.is_empty());
        assert_eq!(f0.len(), time_axis.len());

        // 检查是否估计出了正确的基频（允许一定误差）
        let valid_f0s: Vec<f64> = f0.iter().filter(|&&x| x > 0.0).copied().collect();
        if !valid_f0s.is_empty() {
            let mean_f0 = valid_f0s.iter().sum::<f64>() / valid_f0s.len() as f64;
            assert!(
                (mean_f0 - f0_target).abs() < 50.0,
                "Estimated F0: {}, Expected: {}",
                mean_f0,
                f0_target
            );
        }

        Ok(())
    }

    #[test]
    fn test_harvest_algorithm() -> RvcResult<()> {
        let fs = 16000.0;
        let duration = 0.3;
        let f0_target = 150.0;

        // 生成测试信号
        let n_samples = (fs * duration) as usize;
        let mut signal = vec![0.0; n_samples];

        for i in 0..n_samples {
            let t = i as f64 / fs;
            signal[i] = (2.0 * PI * f0_target * t).sin();
        }

        let config = WorldConfig {
            fs,
            f0_floor: 50.0,
            f0_ceil: 400.0,
            frame_period: 10.0,
            speed: 1,
        };

        let mut harvest = HarvestAlgorithm::new(config);
        let (f0, time_axis) = harvest.estimate_f0(&signal)?;

        assert!(!f0.is_empty());
        assert_eq!(f0.len(), time_axis.len());

        Ok(())
    }

    #[test]
    fn test_convenience_functions() -> RvcResult<()> {
        let fs = 16000.0;
        let mut signal = vec![];
        for _ in 0..1000 {
            signal.extend_from_slice(&[0.1, 0.2, -0.1, -0.2]);
        }

        let (f0_dio, time_dio) = dio(&signal, fs, 70.0, 400.0, 10.0)?;
        assert!(!f0_dio.is_empty());

        let (f0_harvest, time_harvest) = harvest(&signal, fs, 70.0, 400.0, 10.0)?;
        assert!(!f0_harvest.is_empty());

        let f0_refined = stonemask(&signal, &f0_dio, &time_dio, fs)?;
        assert_eq!(f0_refined.len(), f0_dio.len());

        Ok(())
    }
}
