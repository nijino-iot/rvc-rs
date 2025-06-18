//! F0 提取模块
//!
//! 对应 Python 代码中的 Harvest 类和 F0 提取功能

use crate::{Device, Kind, RvcError, RvcResult, Tensor};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::sync::mpsc as async_mpsc;

/// F0 提取方法枚举
#[derive(Debug, Clone, PartialEq)]
pub enum F0Method {
    /// PM (Pitch Marking) 方法
    Pm,
    /// Harvest 方法
    Harvest,
    /// CREPE 方法
    Crepe,
    /// RMVPE 方法
    Rmvpe,
    /// FCPE 方法
    Fcpe,
}

impl F0Method {
    /// 从字符串转换为 F0Method
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pm" => Some(Self::Pm),
            "harvest" => Some(Self::Harvest),
            "crepe" => Some(Self::Crepe),
            "rmvpe" => Some(Self::Rmvpe),
            "fcpe" => Some(Self::Fcpe),
            _ => None,
        }
    }

    /// 转换为字符串
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Pm => "pm",
            Self::Harvest => "harvest",
            Self::Crepe => "crepe",
            Self::Rmvpe => "rmvpe",
            Self::Fcpe => "fcpe",
        }
    }
}

/// F0 提取器特征
pub trait F0Extractor {
    /// 提取 F0
    fn extract(&self, audio: &[f32], sample_rate: u32) -> RvcResult<Vec<f32>>;

    /// 获取方法名称
    fn method(&self) -> F0Method;
}

/// Harvest F0 提取任务
#[derive(Debug)]
pub struct HarvestTask {
    pub idx: usize,
    pub audio: Vec<f32>,
    pub sample_rate: u32,
    pub f0_ceil: f64,
    pub f0_floor: f64,
    pub frame_period: f64,
}

/// Harvest F0 提取结果
#[derive(Debug)]
pub struct HarvestResult {
    pub idx: usize,
    pub f0: Vec<f32>,
    pub time_axis: Vec<f32>,
}

/// Harvest F0 提取器，对应 Python 中的 Harvest 类
pub struct HarvestExtractor {
    /// 工作线程句柄
    workers: Vec<thread::JoinHandle<()>>,
    /// 任务发送器
    task_sender: Sender<HarvestTask>,
    /// 结果接收器
    result_receiver: Arc<Mutex<Receiver<HarvestResult>>>,
    /// 工作线程数量
    n_workers: usize,
    /// 是否正在运行
    running: Arc<Mutex<bool>>,
}

impl HarvestExtractor {
    /// 创建新的 Harvest 提取器
    pub fn new(n_workers: usize) -> RvcResult<Self> {
        let (task_sender, task_receiver) = std::sync::mpsc::channel::<HarvestTask>();
        let (result_sender, result_receiver) = std::sync::mpsc::channel::<HarvestResult>();

        let task_receiver = Arc::new(Mutex::new(task_receiver));
        let result_receiver = Arc::new(Mutex::new(result_receiver));
        let running = Arc::new(Mutex::new(true));

        let mut workers = Vec::new();

        // 启动工作线程
        for worker_id in 0..n_workers {
            let task_receiver = Arc::clone(&task_receiver);
            let result_sender = result_sender.clone();
            let running = Arc::clone(&running);

            let worker = thread::spawn(move || {
                Self::worker_loop(worker_id, task_receiver, result_sender, running);
            });

            workers.push(worker);
        }

        Ok(Self {
            workers,
            task_sender,
            result_receiver,
            n_workers,
            running,
        })
    }

    /// 工作线程主循环
    fn worker_loop(
        worker_id: usize,
        task_receiver: Arc<Mutex<Receiver<HarvestTask>>>,
        result_sender: Sender<HarvestResult>,
        running: Arc<Mutex<bool>>,
    ) {
        println!("Harvest worker {} started", worker_id);

        loop {
            // 检查是否应该停止
            {
                let running_guard = running.lock().unwrap();
                if !*running_guard {
                    break;
                }
            }

            // 尝试接收任务
            let task = {
                let receiver = task_receiver.lock().unwrap();
                receiver.try_recv()
            };

            match task {
                Ok(task) => {
                    // 处理任务
                    match Self::process_harvest_task(&task) {
                        Ok(result) => {
                            if let Err(e) = result_sender.send(result) {
                                eprintln!("Worker {} failed to send result: {}", worker_id, e);
                            }
                        }
                        Err(e) => {
                            eprintln!("Worker {} failed to process task: {}", worker_id, e);
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // 没有任务，短暂等待
                    thread::sleep(std::time::Duration::from_millis(10));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    println!("Worker {} disconnected", worker_id);
                    break;
                }
            }
        }

        println!("Harvest worker {} stopped", worker_id);
    }

    /// 处理单个 Harvest 任务（简化实现）
    fn process_harvest_task(task: &HarvestTask) -> RvcResult<HarvestResult> {
        // 这里是简化的 Harvest 算法实现
        // 实际实现需要更复杂的信号处理算法
        let frame_length = (task.sample_rate as f64 * task.frame_period / 1000.0) as usize;
        let hop_length = frame_length / 4;
        let n_frames = (task.audio.len() + hop_length - 1) / hop_length;

        let mut f0 = Vec::with_capacity(n_frames);
        let mut time_axis = Vec::with_capacity(n_frames);

        // 简化的 F0 提取算法
        for i in 0..n_frames {
            let start = i * hop_length;
            let end = (start + frame_length).min(task.audio.len());

            if end > start {
                let frame = &task.audio[start..end];

                // 简化的基频估计（实际需要更复杂的算法）
                let estimated_f0 =
                    Self::estimate_f0_simple(frame, task.sample_rate, task.f0_floor, task.f0_ceil);
                f0.push(estimated_f0);

                // 计算时间轴
                let time = (start as f64) / (task.sample_rate as f64);
                time_axis.push(time as f32);
            }
        }

        Ok(HarvestResult {
            idx: task.idx,
            f0,
            time_axis,
        })
    }

    /// 简化的 F0 估计算法
    fn estimate_f0_simple(frame: &[f32], sample_rate: u32, f0_floor: f64, f0_ceil: f64) -> f32 {
        // 简化实现：使用自相关方法估计基频
        let min_period = (sample_rate as f64 / f0_ceil) as usize;
        let max_period = (sample_rate as f64 / f0_floor) as usize;

        if frame.len() < max_period * 2 {
            return 0.0; // 无声
        }

        let mut max_correlation = 0.0;
        let mut best_period = 0;

        for period in min_period..=max_period.min(frame.len() / 2) {
            let mut correlation = 0.0;
            let mut norm1 = 0.0;
            let mut norm2 = 0.0;

            for i in 0..(frame.len() - period) {
                let x1 = frame[i];
                let x2 = frame[i + period];
                correlation += x1 * x2;
                norm1 += x1 * x1;
                norm2 += x2 * x2;
            }

            if norm1 > 0.0 && norm2 > 0.0 {
                correlation /= (norm1 * norm2).sqrt();
                if correlation > max_correlation {
                    max_correlation = correlation;
                    best_period = period;
                }
            }
        }

        if max_correlation > 0.3 && best_period > 0 {
            sample_rate as f32 / best_period as f32
        } else {
            0.0 // 无声
        }
    }

    /// 提交 F0 提取任务
    pub fn submit_task(&self, task: HarvestTask) -> RvcResult<()> {
        self.task_sender
            .send(task)
            .map_err(|e| RvcError::f0(format!("Failed to submit task: {}", e)))
    }

    /// 获取结果
    pub fn get_result(&self) -> RvcResult<Option<HarvestResult>> {
        let receiver = self.result_receiver.lock().unwrap();
        match receiver.try_recv() {
            Ok(result) => Ok(Some(result)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err(RvcError::f0("Result receiver disconnected".to_string()))
            }
        }
    }

    /// 批量处理音频
    pub fn extract_batch(
        &self,
        audio_batch: &[Vec<f32>],
        sample_rate: u32,
    ) -> RvcResult<HashMap<usize, Vec<f32>>> {
        let mut results = HashMap::new();
        let n_tasks = audio_batch.len();

        // 提交所有任务
        for (idx, audio) in audio_batch.iter().enumerate() {
            let task = HarvestTask {
                idx,
                audio: audio.clone(),
                sample_rate,
                f0_ceil: 1100.0,
                f0_floor: 50.0,
                frame_period: 10.0,
            };

            self.submit_task(task)?;
        }

        // 等待所有结果
        let mut received_count = 0;
        while received_count < n_tasks {
            if let Some(result) = self.get_result()? {
                results.insert(result.idx, result.f0);
                received_count += 1;
            } else {
                // 短暂等待
                thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        Ok(results)
    }
}

impl Drop for HarvestExtractor {
    fn drop(&mut self) {
        // 停止工作线程
        {
            let mut running = self.running.lock().unwrap();
            *running = false;
        }

        // 等待所有线程结束
        while let Some(worker) = self.workers.pop() {
            if let Err(e) = worker.join() {
                eprintln!("Failed to join worker thread: {:?}", e);
            }
        }
    }
}

impl F0Extractor for HarvestExtractor {
    fn extract(&self, audio: &[f32], sample_rate: u32) -> RvcResult<Vec<f32>> {
        let task = HarvestTask {
            idx: 0,
            audio: audio.to_vec(),
            sample_rate,
            f0_ceil: 1100.0,
            f0_floor: 50.0,
            frame_period: 10.0,
        };

        self.submit_task(task)?;

        // 等待结果
        loop {
            if let Some(result) = self.get_result()? {
                if result.idx == 0 {
                    return Ok(result.f0);
                }
            }
            thread::sleep(std::time::Duration::from_millis(1));
        }
    }

    fn method(&self) -> F0Method {
        F0Method::Harvest
    }
}

/// PM F0 提取器
pub struct PmExtractor;

impl F0Extractor for PmExtractor {
    fn extract(&self, audio: &[f32], sample_rate: u32) -> RvcResult<Vec<f32>> {
        // PM 方法的简化实现
        let frame_size = 1024;
        let hop_size = 256;
        let n_frames = (audio.len() + hop_size - 1) / hop_size;

        let mut f0 = Vec::with_capacity(n_frames);

        for i in 0..n_frames {
            let start = i * hop_size;
            let end = (start + frame_size).min(audio.len());

            if end > start {
                let frame = &audio[start..end];
                let estimated_f0 = self.estimate_f0_pm(frame, sample_rate);
                f0.push(estimated_f0);
            }
        }

        Ok(f0)
    }

    fn method(&self) -> F0Method {
        F0Method::Pm
    }
}

impl PmExtractor {
    /// PM 方法的 F0 估计
    fn estimate_f0_pm(&self, frame: &[f32], sample_rate: u32) -> f32 {
        // 简化的 PM 算法实现
        // 实际实现需要更复杂的信号处理
        let min_f0 = 50.0;
        let max_f0 = 800.0;

        let min_period = (sample_rate as f32 / max_f0) as usize;
        let max_period = (sample_rate as f32 / min_f0) as usize;

        if frame.len() < max_period {
            return 0.0;
        }

        // 找到最大的自相关延迟
        let mut max_correlation = 0.0;
        let mut best_period = 0;

        for period in min_period..=max_period.min(frame.len() / 2) {
            let mut correlation = 0.0;
            for i in 0..(frame.len() - period) {
                correlation += frame[i] * frame[i + period];
            }

            if correlation > max_correlation {
                max_correlation = correlation;
                best_period = period;
            }
        }

        if best_period > 0 && max_correlation > 0.1 {
            sample_rate as f32 / best_period as f32
        } else {
            0.0
        }
    }
}

/// F0 提取器工厂
pub struct F0ExtractorFactory;

impl F0ExtractorFactory {
    /// 创建 F0 提取器
    pub fn create(
        method: F0Method,
        n_workers: Option<usize>,
    ) -> RvcResult<Box<dyn F0Extractor + Send + Sync>> {
        match method {
            F0Method::Pm => Ok(Box::new(PmExtractor)),
            F0Method::Harvest => {
                let n_workers = n_workers.unwrap_or(num_cpus::get().min(4));
                let extractor = HarvestExtractor::new(n_workers)?;
                Ok(Box::new(extractor))
            }
            _ => Err(RvcError::f0(format!(
                "F0 method {:?} not implemented yet",
                method
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_f0_method_conversion() {
        assert_eq!(F0Method::from_str("pm"), Some(F0Method::Pm));
        assert_eq!(F0Method::from_str("harvest"), Some(F0Method::Harvest));
        assert_eq!(F0Method::from_str("invalid"), None);

        assert_eq!(F0Method::Pm.to_str(), "pm");
        assert_eq!(F0Method::Harvest.to_str(), "harvest");
    }

    #[test]
    fn test_pm_extractor() -> RvcResult<()> {
        let extractor = PmExtractor;
        let sample_rate = 16000;

        // 创建测试音频（440Hz 正弦波）
        let duration = 1.0; // 1秒
        let samples = (sample_rate as f32 * duration) as usize;
        let mut audio = Vec::with_capacity(samples);

        for i in 0..samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * std::f32::consts::PI * 440.0 * t).sin();
            audio.push(sample);
        }

        let f0 = extractor.extract(&audio, sample_rate)?;
        assert!(!f0.is_empty());

        Ok(())
    }

    #[test]
    fn test_harvest_extractor() -> RvcResult<()> {
        let extractor = HarvestExtractor::new(2)?;
        let sample_rate = 16000;

        // 创建测试音频
        let audio = vec![0.1, 0.2, -0.1, -0.2, 0.1, 0.2, -0.1, -0.2]; // 简单的周期信号
        let f0 = extractor.extract(&audio, sample_rate)?;

        assert!(!f0.is_empty());

        Ok(())
    }

    #[test]
    fn test_f0_extractor_factory() -> RvcResult<()> {
        let pm_extractor = F0ExtractorFactory::create(F0Method::Pm, None)?;
        assert_eq!(pm_extractor.method(), F0Method::Pm);

        let harvest_extractor = F0ExtractorFactory::create(F0Method::Harvest, Some(1))?;
        assert_eq!(harvest_extractor.method(), F0Method::Harvest);

        Ok(())
    }
}
