//! 事件系统模块
//!
//! 提供应用程序状态变化的事件通知机制，支持异步事件发布和订阅

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio_util::sync::CancellationToken;

/// 应用程序事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    /// 应用状态变化
    StateChanged {
        old_state: AppState,
        new_state: AppState,
    },
    /// 运行时统计信息更新
    StatsUpdated { stats: RuntimeStats },
    /// 设备列表更新
    DevicesUpdated {
        input_devices: Vec<AudioDeviceInfo>,
        output_devices: Vec<AudioDeviceInfo>,
    },
    /// 配置参数更新
    ConfigUpdated {
        config: HashMap<String, serde_json::Value>,
    },
    /// 音频处理状态更新
    AudioProcessing {
        delay_time: f64,
        inference_time: f64,
        buffer_usage: f32,
    },
    /// 错误事件
    Error { message: String, error_type: String },
    /// 日志事件
    Log {
        level: String,
        message: String,
        timestamp: u64,
    },
}

/// 应用状态枚举
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AppState {
    /// 初始化中
    Initializing,
    /// 就绪状态
    Ready,
    /// 转换中
    Converting,
    /// 错误状态
    Error(String),
    /// 已停止
    Stopped,
}

/// 运行时统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeStats {
    /// 算法延迟(毫秒)
    pub algorithm_latency_ms: f64,
    /// 推理时间(毫秒)
    pub inference_time_ms: f64,
    /// 缓冲区使用率(0.0-1.0)
    pub buffer_usage: f32,
    /// CPU使用率(0.0-1.0)
    pub cpu_usage: f32,
    /// GPU使用率(0.0-1.0)，可选
    pub gpu_usage: Option<f32>,
    /// 内存使用量(MB)
    pub memory_usage_mb: f32,
    /// 处理的音频帧数
    pub processed_frames: u64,
    /// 丢帧数
    pub dropped_frames: u64,
}

impl Default for RuntimeStats {
    fn default() -> Self {
        Self {
            algorithm_latency_ms: 0.0,
            inference_time_ms: 0.0,
            buffer_usage: 0.0,
            cpu_usage: 0.0,
            gpu_usage: None,
            memory_usage_mb: 0.0,
            processed_frames: 0,
            dropped_frames: 0,
        }
    }
}

/// 音频设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    /// 设备名称
    pub name: String,
    /// 设备索引
    pub index: usize,
    /// 主机API名称
    pub hostapi_name: String,
    /// 最大输入通道数
    pub max_input_channels: u32,
    /// 最大输出通道数
    pub max_output_channels: u32,
    /// 默认采样率
    pub default_samplerate: f64,
}

/// 事件发布器
#[derive(Debug, Clone)]
pub struct EventPublisher {
    sender: broadcast::Sender<AppEvent>,
}

impl EventPublisher {
    /// 创建新的事件发布器
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// 发布事件
    pub fn publish(&self, event: AppEvent) -> Result<usize, broadcast::error::SendError<AppEvent>> {
        self.sender.send(event)
    }

    /// 创建订阅器
    pub fn subscribe(&self) -> EventSubscriber {
        EventSubscriber {
            receiver: self.sender.subscribe(),
        }
    }

    /// 获取订阅者数量
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

/// 事件订阅器
#[derive(Debug)]
pub struct EventSubscriber {
    receiver: broadcast::Receiver<AppEvent>,
}

impl EventSubscriber {
    /// 接收下一个事件
    pub async fn recv(&mut self) -> Result<AppEvent, broadcast::error::RecvError> {
        self.receiver.recv().await
    }

    /// 尝试接收事件（非阻塞）
    pub fn try_recv(&mut self) -> Result<AppEvent, broadcast::error::TryRecvError> {
        self.receiver.try_recv()
    }
}

/// 事件管理器
#[derive(Debug)]
pub struct EventManager {
    publisher: EventPublisher,
    current_state: Arc<RwLock<AppState>>,
    current_stats: Arc<RwLock<RuntimeStats>>,
    cancellation_token: CancellationToken,
}

impl EventManager {
    /// 创建新的事件管理器
    pub fn new(capacity: usize) -> Self {
        Self {
            publisher: EventPublisher::new(capacity),
            current_state: Arc::new(RwLock::new(AppState::Initializing)),
            current_stats: Arc::new(RwLock::new(RuntimeStats::default())),
            cancellation_token: CancellationToken::new(),
        }
    }

    /// 获取事件发布器
    pub fn publisher(&self) -> EventPublisher {
        self.publisher.clone()
    }

    /// 创建事件订阅器
    pub fn subscribe(&self) -> EventSubscriber {
        self.publisher.subscribe()
    }

    /// 更新应用状态
    pub async fn update_state(&self, new_state: AppState) {
        let mut current_state = self.current_state.write().await;
        let old_state = current_state.clone();

        if old_state != new_state {
            *current_state = new_state.clone();
            drop(current_state);

            let _ = self.publisher.publish(AppEvent::StateChanged {
                old_state,
                new_state,
            });
        }
    }

    /// 获取当前状态
    pub async fn get_state(&self) -> AppState {
        self.current_state.read().await.clone()
    }

    /// 更新运行时统计信息
    pub async fn update_stats(&self, stats: RuntimeStats) {
        *self.current_stats.write().await = stats.clone();
        let _ = self.publisher.publish(AppEvent::StatsUpdated { stats });
    }

    /// 获取当前统计信息
    pub async fn get_stats(&self) -> RuntimeStats {
        self.current_stats.read().await.clone()
    }

    /// 发布设备列表更新
    pub fn publish_devices_updated(
        &self,
        input_devices: Vec<AudioDeviceInfo>,
        output_devices: Vec<AudioDeviceInfo>,
    ) {
        let _ = self.publisher.publish(AppEvent::DevicesUpdated {
            input_devices,
            output_devices,
        });
    }

    /// 发布配置更新
    pub fn publish_config_updated(&self, config: HashMap<String, serde_json::Value>) {
        let _ = self.publisher.publish(AppEvent::ConfigUpdated { config });
    }

    /// 发布音频处理状态
    pub fn publish_audio_processing(
        &self,
        delay_time: f64,
        inference_time: f64,
        buffer_usage: f32,
    ) {
        let _ = self.publisher.publish(AppEvent::AudioProcessing {
            delay_time,
            inference_time,
            buffer_usage,
        });
    }

    /// 发布错误事件
    pub fn publish_error(&self, message: String, error_type: String) {
        let _ = self.publisher.publish(AppEvent::Error {
            message,
            error_type,
        });
    }

    /// 发布日志事件
    pub fn publish_log(&self, level: String, message: String) {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let _ = self.publisher.publish(AppEvent::Log {
            level,
            message,
            timestamp,
        });
    }

    /// 获取订阅者数量
    pub fn subscriber_count(&self) -> usize {
        self.publisher.subscriber_count()
    }

    /// 关闭事件管理器
    pub fn shutdown(&self) {
        self.cancellation_token.cancel();
    }

    /// 获取取消令牌
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }
}

impl Clone for EventManager {
    fn clone(&self) -> Self {
        Self {
            publisher: self.publisher.clone(),
            current_state: self.current_state.clone(),
            current_stats: self.current_stats.clone(),
            cancellation_token: self.cancellation_token.clone(),
        }
    }
}

/// 统计收集器 - 定期收集系统统计信息
pub struct StatsCollector {
    event_manager: EventManager,
    interval: std::time::Duration,
}

impl StatsCollector {
    /// 创建新的统计收集器
    pub fn new(event_manager: EventManager, interval_ms: u64) -> Self {
        Self {
            event_manager,
            interval: std::time::Duration::from_millis(interval_ms),
        }
    }

    /// 启动统计收集
    pub async fn start(&self) {
        let mut interval = tokio::time::interval(self.interval);
        let token = self.event_manager.cancellation_token();

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Some(stats) = self.collect_stats().await {
                        self.event_manager.update_stats(stats).await;
                    }
                }
                _ = token.cancelled() => {
                    break;
                }
            }
        }
    }

    /// 收集系统统计信息
    async fn collect_stats(&self) -> Option<RuntimeStats> {
        // 这里可以实现实际的统计信息收集逻辑
        // 例如：CPU使用率、内存使用率、GPU使用率等
        // 目前返回模拟数据
        Some(RuntimeStats {
            algorithm_latency_ms: 0.0,
            inference_time_ms: 0.0,
            buffer_usage: 0.0,
            cpu_usage: self.get_cpu_usage(),
            gpu_usage: self.get_gpu_usage(),
            memory_usage_mb: self.get_memory_usage(),
            processed_frames: 0,
            dropped_frames: 0,
        })
    }

    /// 获取CPU使用率
    fn get_cpu_usage(&self) -> f32 {
        // 实现CPU使用率获取逻辑
        0.0
    }

    /// 获取GPU使用率
    fn get_gpu_usage(&self) -> Option<f32> {
        // 实现GPU使用率获取逻辑
        None
    }

    /// 获取内存使用量
    fn get_memory_usage(&self) -> f32 {
        // 实现内存使用量获取逻辑
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_manager() {
        let event_manager = EventManager::new(100);
        let mut subscriber = event_manager.subscribe();

        // 测试状态更新
        event_manager.update_state(AppState::Ready).await;

        // 接收事件
        let event = subscriber.recv().await.unwrap();
        match event {
            AppEvent::StateChanged {
                old_state,
                new_state,
            } => {
                assert_eq!(old_state, AppState::Initializing);
                assert_eq!(new_state, AppState::Ready);
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_stats_update() {
        let event_manager = EventManager::new(100);
        let mut subscriber = event_manager.subscribe();

        let stats = RuntimeStats {
            algorithm_latency_ms: 10.0,
            inference_time_ms: 5.0,
            buffer_usage: 0.5,
            cpu_usage: 0.3,
            gpu_usage: Some(0.2),
            memory_usage_mb: 100.0,
            processed_frames: 1000,
            dropped_frames: 5,
        };

        event_manager.update_stats(stats.clone()).await;

        let event = subscriber.recv().await.unwrap();
        match event {
            AppEvent::StatsUpdated {
                stats: received_stats,
            } => {
                assert_eq!(
                    received_stats.algorithm_latency_ms,
                    stats.algorithm_latency_ms
                );
                assert_eq!(received_stats.inference_time_ms, stats.inference_time_ms);
            }
            _ => panic!("Unexpected event type"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let event_manager = EventManager::new(100);
        let mut subscriber1 = event_manager.subscribe();
        let mut subscriber2 = event_manager.subscribe();

        assert_eq!(event_manager.subscriber_count(), 2);

        event_manager.update_state(AppState::Converting).await;

        // 两个订阅者都应该收到事件
        let event1 = subscriber1.recv().await.unwrap();
        let event2 = subscriber2.recv().await.unwrap();

        match (event1, event2) {
            (AppEvent::StateChanged { .. }, AppEvent::StateChanged { .. }) => {}
            _ => panic!("Both subscribers should receive the same event"),
        }
    }
}
