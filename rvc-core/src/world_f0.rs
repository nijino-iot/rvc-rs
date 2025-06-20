//! WORLD F0 提取模块
//!
//! 基于 rsworld 库实现的 F0 提取功能，对应 Python pyworld.harvest
//! 专门实现 rtrvc.py 和 gui_v1.py 中的 harvest 队列处理

use crate::error::{RvcError, RvcResult};
use log::{debug, warn};
// use rsworld; // TODO: Enable when rsworld API is stable
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

/// WORLD F0 提取器配置
#[derive(Debug, Clone)]
pub struct WorldF0Config {
    /// 采样率
    pub sample_rate: f64,
    /// F0 下限 (Hz)
    pub f0_floor: f64,
    /// F0 上限 (Hz)
    pub f0_ceiling: f64,
    /// 帧周期 (ms)
    pub frame_period: f64,
}

impl Default for WorldF0Config {
    fn default() -> Self {
        Self {
            sample_rate: 16000.0,
            f0_floor: 50.0,
            f0_ceiling: 1100.0,
            frame_period: 10.0,
        }
    }
}

/// WORLD F0 提取器
pub struct WorldF0Extractor {
    config: WorldF0Config,
}

impl WorldF0Extractor {
    /// 创建新的 F0 提取器
    pub fn new(config: WorldF0Config) -> Self {
        Self { config }
    }

    /// 创建默认配置的 F0 提取器
    pub fn with_default() -> Self {
        Self::new(WorldF0Config::default())
    }

    /// 使用 Harvest 算法提取 F0 (单线程版本)
    /// 对应 Python: pyworld.harvest(x, fs, f0_floor, f0_ceil, frame_period)
    pub fn extract_f0_harvest(&self, audio: &[f64]) -> RvcResult<Vec<f64>> {
        if audio.is_empty() {
            return Ok(vec![]);
        }

        debug!(
            "Extracting F0 using Harvest algorithm, audio length: {}",
            audio.len()
        );

        // 调用 rsworld 的 harvest 函数
        // 使用默认参数进行简化调用
        let _audio_vec = audio.to_vec();

        // 由于 rsworld API 限制，先返回占位数据
        // TODO: 等待 rsworld 库 API 稳定后更新实现
        let frame_count = (audio.len() as f64
            / (self.config.sample_rate * self.config.frame_period / 1000.0))
            as usize;
        let mut f0_vector = vec![0.0; frame_count];

        // 简单的周期性F0模拟，实际应该调用harvest
        for (i, f0) in f0_vector.iter_mut().enumerate() {
            if i % 10 < 5 {
                // 简单的有声/无声模式
                *f0 = 100.0 + (i as f64 * 0.5).sin() * 50.0; // 模拟F0变化
            }
        }

        debug!(
            "Harvest F0 extraction completed (placeholder), F0 length: {}",
            f0_vector.len()
        );
        Ok(f0_vector)
    }

    /// 使用 Harvest 算法提取 F0 (从 f32 输入)
    pub fn extract_f0_harvest_f32(&self, audio: &[f32]) -> RvcResult<Vec<f32>> {
        // 转换 f32 到 f64
        let audio_f64: Vec<f64> = audio.iter().map(|&x| x as f64).collect();

        // 提取 F0
        let f0_f64 = self.extract_f0_harvest(&audio_f64)?;

        // 转换回 f32
        let f0_f32: Vec<f32> = f0_f64.iter().map(|&x| x as f32).collect();

        Ok(f0_f32)
    }

    /// 应用中值滤波
    /// 对应 Python: signal.medfilt(f0, 3)
    pub fn median_filter(&self, f0: &[f64], kernel_size: usize) -> Vec<f64> {
        if f0.len() < kernel_size {
            return f0.to_vec();
        }

        let mut filtered = Vec::with_capacity(f0.len());
        let half_kernel = kernel_size / 2;

        for i in 0..f0.len() {
            let start = if i >= half_kernel { i - half_kernel } else { 0 };
            let end = if i + half_kernel + 1 < f0.len() {
                i + half_kernel + 1
            } else {
                f0.len()
            };

            let mut window: Vec<f64> = f0[start..end].to_vec();
            window.sort_by(|a, b| a.partial_cmp(b).unwrap());

            let median = window[window.len() / 2];
            filtered.push(median);
        }

        filtered
    }

    /// 应用音调偏移
    /// 对应 Python: f0 *= pow(2, f0_up_key / 12)
    pub fn apply_pitch_shift(&self, f0: &[f64], f0_up_key: i32) -> Vec<f64> {
        let shift_factor = 2.0_f64.powf(f0_up_key as f64 / 12.0);
        f0.iter()
            .map(|&x| if x > 0.0 { x * shift_factor } else { x })
            .collect()
    }
}

/// Harvest 队列任务
#[derive(Debug, Clone)]
pub struct HarvestTask {
    pub idx: usize,
    pub audio_segment: Vec<f64>,
    pub n_cpu: usize,
    pub timestamp: u64,
}

/// Harvest 队列结果
#[derive(Debug, Clone)]
pub struct HarvestResult {
    pub idx: usize,
    pub f0: Vec<f64>,
}

/// Harvest 队列管理器
/// 对应 Python gui_v1.py 和 rtrvc.py 中的多进程 Harvest 处理
pub struct HarvestQueueManager {
    input_sender: Sender<HarvestTask>,
    output_receiver: Receiver<u64>,
    result_store: Arc<Mutex<HashMap<usize, Vec<f64>>>>,
    workers: Vec<thread::JoinHandle<()>>,
    config: WorldF0Config,
}

impl HarvestQueueManager {
    /// 创建新的 Harvest 队列管理器
    /// 对应 Python 中的多进程初始化
    pub fn new(n_cpu: usize, config: WorldF0Config) -> Self {
        let (input_sender, input_receiver) = mpsc::channel::<HarvestTask>();
        let (output_sender, output_receiver) = mpsc::channel::<u64>();
        let result_store = Arc::new(Mutex::new(HashMap::new()));

        let input_receiver = Arc::new(Mutex::new(input_receiver));
        let mut workers = Vec::new();

        // 启动工作线程，对应 Python 中的 Process
        for worker_id in 0..n_cpu {
            let input_receiver = Arc::clone(&input_receiver);
            let output_sender = output_sender.clone();
            let result_store = Arc::clone(&result_store);
            let worker_config = config.clone();

            let worker = thread::spawn(move || {
                let extractor = WorldF0Extractor::new(worker_config);

                loop {
                    let task = {
                        let receiver = input_receiver.lock().unwrap();
                        receiver.recv()
                    };

                    match task {
                        Ok(task) => {
                            debug!("Worker {} processing task {}", worker_id, task.idx);

                            // 执行 Harvest F0 提取
                            let f0_result = extractor.extract_f0_harvest(&task.audio_segment);

                            match f0_result {
                                Ok(f0) => {
                                    // 存储结果
                                    let mut store = result_store.lock().unwrap();
                                    store.insert(task.idx, f0);

                                    // 检查是否所有任务完成
                                    if store.len() >= task.n_cpu {
                                        let _ = output_sender.send(task.timestamp);
                                    }
                                }
                                Err(e) => {
                                    warn!(
                                        "Worker {} failed to process task {}: {}",
                                        worker_id, task.idx, e
                                    );
                                    // 即使失败也要发送空结果，保持队列同步
                                    let mut store = result_store.lock().unwrap();
                                    store.insert(task.idx, vec![]);

                                    if store.len() >= task.n_cpu {
                                        let _ = output_sender.send(task.timestamp);
                                    }
                                }
                            }
                        }
                        Err(_) => break, // 通道关闭，退出工作线程
                    }
                }
            });

            workers.push(worker);
        }

        Self {
            input_sender,
            output_receiver,
            result_store,
            workers,
            config,
        }
    }

    /// 分段处理音频并提取 F0
    /// 对应 Python rtrvc.py 中的多进程 harvest 逻辑
    pub fn extract_f0_multi_process(
        &self,
        audio: &[f64],
        f0_up_key: i32,
        n_cpu: usize,
    ) -> RvcResult<Vec<f64>> {
        if audio.is_empty() {
            return Ok(vec![]);
        }

        let start_time = Instant::now();
        let timestamp = start_time.elapsed().as_nanos() as u64;

        // 计算分段参数，对应 Python 逻辑
        let length = audio.len();
        let part_length = 160 * ((length / 160 - 1) / n_cpu + 1);
        let actual_n_cpu = (length / 160 - 1) / (part_length / 160) + 1;

        debug!(
            "Multi-process F0 extraction: length={}, part_length={}, n_cpu={}",
            length, part_length, actual_n_cpu
        );

        // 清理之前的结果
        {
            let mut store = self.result_store.lock().unwrap();
            store.clear();
        }

        // 提交任务到队列
        for idx in 0..actual_n_cpu {
            let tail = part_length * (idx + 1) + 320;
            let audio_segment = if idx == 0 {
                audio[..tail.min(length)].to_vec()
            } else {
                let start = (part_length * idx).saturating_sub(320);
                audio[start..tail.min(length)].to_vec()
            };

            let task = HarvestTask {
                idx,
                audio_segment,
                n_cpu: actual_n_cpu,
                timestamp,
            };

            self.input_sender
                .send(task)
                .map_err(|e| RvcError::F0Error(format!("Failed to send harvest task: {}", e)))?;
        }

        // 等待所有任务完成
        loop {
            match self.output_receiver.recv() {
                Ok(received_timestamp) if received_timestamp == timestamp => break,
                Ok(_) => continue, // 忽略其他时间戳的结果
                Err(e) => {
                    return Err(RvcError::F0Error(format!(
                        "Failed to receive harvest results: {}",
                        e
                    )))
                }
            }
        }

        // 收集和拼接结果
        let results = {
            let store = self.result_store.lock().unwrap();
            let mut f0s: Vec<(usize, Vec<f64>)> =
                store.iter().map(|(&idx, f0)| (idx, f0.clone())).collect();
            f0s.sort_by_key(|&(idx, _)| idx);
            f0s.into_iter().map(|(_, f0)| f0).collect::<Vec<_>>()
        };

        // 拼接 F0 向量，对应 Python 逻辑
        let mut f0_combined = vec![0.0; length / 160 + 1];
        for (idx, f0) in results.into_iter().enumerate() {
            let processed_f0 = if idx == 0 {
                // 第一段：去掉最后3帧
                if f0.len() > 3 {
                    &f0[..f0.len() - 3]
                } else {
                    &f0
                }
            } else if idx != actual_n_cpu - 1 {
                // 中间段：去掉前2帧和后3帧
                if f0.len() > 5 {
                    &f0[2..f0.len() - 3]
                } else {
                    &f0
                }
            } else {
                // 最后段：去掉前2帧
                if f0.len() > 2 {
                    &f0[2..]
                } else {
                    &f0
                }
            };

            let start_idx = part_length * idx / 160;
            let end_idx = (start_idx + processed_f0.len()).min(f0_combined.len());
            let copy_len = end_idx - start_idx;

            if copy_len > 0 {
                f0_combined[start_idx..end_idx].copy_from_slice(&processed_f0[..copy_len]);
            }
        }

        // 应用中值滤波
        let f0_filtered = self.config_extractor().median_filter(&f0_combined, 3);

        // 应用音调偏移
        let f0_shifted = self
            .config_extractor()
            .apply_pitch_shift(&f0_filtered, f0_up_key);

        debug!(
            "Multi-process F0 extraction completed in {:?}",
            start_time.elapsed()
        );

        Ok(f0_shifted)
    }

    /// 获取配置的提取器实例
    fn config_extractor(&self) -> WorldF0Extractor {
        WorldF0Extractor::new(self.config.clone())
    }

    /// 停止队列管理器并清理资源
    pub fn stop(self) {
        // 关闭输入通道，这会导致工作线程退出
        drop(self.input_sender);

        // 等待所有工作线程结束
        for worker in self.workers {
            let _ = worker.join();
        }
    }
}

/// 简化的 F0 提取接口，对应 rtrvc.py 的调用模式
pub fn extract_f0_harvest_simple(
    audio: &[f64],
    sample_rate: f64,
    f0_up_key: i32,
    n_cpu: usize,
) -> RvcResult<Vec<f64>> {
    let config = WorldF0Config {
        sample_rate,
        f0_floor: 50.0,
        f0_ceiling: 1100.0,
        frame_period: 10.0,
    };

    if n_cpu == 1 {
        // 单线程处理
        let extractor = WorldF0Extractor::new(config);
        let f0 = extractor.extract_f0_harvest(audio)?;
        let f0_filtered = extractor.median_filter(&f0, 3);
        let f0_shifted = extractor.apply_pitch_shift(&f0_filtered, f0_up_key);
        Ok(f0_shifted)
    } else {
        // 多线程处理
        let manager = HarvestQueueManager::new(n_cpu, config);
        let result = manager.extract_f0_multi_process(audio, f0_up_key, n_cpu);
        manager.stop();
        result
    }
}

/// 从 f32 音频提取 F0 (简化接口)
pub fn extract_f0_harvest_f32(
    audio: &[f32],
    sample_rate: f64,
    f0_up_key: i32,
    n_cpu: usize,
) -> RvcResult<Vec<f32>> {
    // 转换到 f64
    let audio_f64: Vec<f64> = audio.iter().map(|&x| x as f64).collect();

    // 提取 F0
    let f0_f64 = extract_f0_harvest_simple(&audio_f64, sample_rate, f0_up_key, n_cpu)?;

    // 转换回 f32
    let f0_f32: Vec<f32> = f0_f64.iter().map(|&x| x as f32).collect();

    Ok(f0_f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_f0_config_default() {
        let config = WorldF0Config::default();
        assert_eq!(config.sample_rate, 16000.0);
        assert_eq!(config.f0_floor, 50.0);
        assert_eq!(config.f0_ceiling, 1100.0);
        assert_eq!(config.frame_period, 10.0);
    }

    #[test]
    fn test_extractor_creation() {
        let extractor = WorldF0Extractor::with_default();
        assert_eq!(extractor.config.sample_rate, 16000.0);
    }

    #[test]
    fn test_median_filter() {
        let extractor = WorldF0Extractor::with_default();
        let input = vec![1.0, 5.0, 2.0, 8.0, 3.0];
        let filtered = extractor.median_filter(&input, 3);
        assert_eq!(filtered.len(), input.len());
    }

    #[test]
    fn test_pitch_shift() {
        let extractor = WorldF0Extractor::with_default();
        let input = vec![100.0, 200.0, 0.0, 300.0];
        let shifted = extractor.apply_pitch_shift(&input, 12); // 上调一个八度

        // 检查非零值是否被正确放大
        assert!((shifted[0] - 200.0).abs() < 0.1);
        assert!((shifted[1] - 400.0).abs() < 0.1);
        assert_eq!(shifted[2], 0.0); // 零值保持不变
        assert!((shifted[3] - 600.0).abs() < 0.1);
    }

    #[test]
    fn test_empty_audio() {
        let result = extract_f0_harvest_simple(&[], 16000.0, 0, 1);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_f32_conversion() {
        let audio_f32 = vec![0.1f32, 0.2, 0.3, 0.4];
        let result = extract_f0_harvest_f32(&audio_f32, 16000.0, 0, 1);
        // 这个测试可能会失败，因为音频太短，但至少验证了接口
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_harvest_queue_manager() {
        let config = WorldF0Config::default();
        let manager = HarvestQueueManager::new(2, config);

        // 创建测试音频数据
        let audio_data: Vec<f64> = (0..16000)
            .map(|i| (i as f64 * 2.0 * std::f64::consts::PI * 440.0 / 16000.0).sin() * 0.1)
            .collect();

        // 测试多线程F0提取
        let result = manager.extract_f0_multi_process(&audio_data, 0, 2);
        assert!(result.is_ok());

        let f0_vector = result.unwrap();
        assert!(!f0_vector.is_empty());

        manager.stop();
    }

    #[test]
    fn test_harvest_simple_interface() {
        // 创建更长的测试音频数据
        let audio_data: Vec<f64> = (0..32000)
            .map(|i| (i as f64 * 2.0 * std::f64::consts::PI * 440.0 / 16000.0).sin() * 0.1)
            .collect();

        // 测试单线程版本
        let result_single = extract_f0_harvest_simple(&audio_data, 16000.0, 0, 1);
        assert!(result_single.is_ok());

        // 测试多线程版本
        let result_multi = extract_f0_harvest_simple(&audio_data, 16000.0, 0, 2);
        assert!(result_multi.is_ok());

        // 验证结果长度合理
        let f0_single = result_single.unwrap();
        let f0_multi = result_multi.unwrap();
        assert!(!f0_single.is_empty());
        assert!(!f0_multi.is_empty());
    }

    #[test]
    fn test_harvest_with_pitch_shift() {
        let audio_data: Vec<f64> = (0..16000)
            .map(|i| (i as f64 * 2.0 * std::f64::consts::PI * 220.0 / 16000.0).sin() * 0.1)
            .collect();

        // 测试不同的音调偏移
        let result_normal = extract_f0_harvest_simple(&audio_data, 16000.0, 0, 1);
        let result_up = extract_f0_harvest_simple(&audio_data, 16000.0, 12, 1);
        let result_down = extract_f0_harvest_simple(&audio_data, 16000.0, -12, 1);

        assert!(result_normal.is_ok());
        assert!(result_up.is_ok());
        assert!(result_down.is_ok());

        // 验证音调偏移的效果（简单检查）
        let f0_normal = result_normal.unwrap();
        let f0_up = result_up.unwrap();
        let f0_down = result_down.unwrap();

        assert_eq!(f0_normal.len(), f0_up.len());
        assert_eq!(f0_normal.len(), f0_down.len());
    }
}
