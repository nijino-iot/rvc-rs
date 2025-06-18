//! 错误处理模块
//!
//! 定义了 RVC 核心库中使用的所有错误类型

use thiserror::Error;

/// RVC 核心库的主要错误类型
#[derive(Error, Debug)]
pub enum RvcError {
    #[error("音频处理错误: {0}")]
    AudioError(String),

    #[error("模型加载错误: {0}")]
    ModelError(String),

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("设备错误: {0}")]
    DeviceError(String),

    #[error("推理错误: {0}")]
    InferenceError(String),

    #[error("F0 提取错误: {0}")]
    F0Error(String),

    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("PyTorch 错误: {0}")]
    TorchError(String),

    #[error("数学计算错误: {0}")]
    MathError(String),

    #[error("内存错误: {0}")]
    MemoryError(String),

    #[error("线程/并发错误: {0}")]
    ConcurrencyError(String),

    #[error("不支持的操作: {0}")]
    UnsupportedError(String),

    #[error("参数错误: {0}")]
    ParameterError(String),

    #[error("未知错误: {0}")]
    Other(String),
}

/// RVC 结果类型别名
pub type RvcResult<T> = Result<T, RvcError>;

impl RvcError {
    /// 创建音频错误
    pub fn audio(msg: impl Into<String>) -> Self {
        Self::AudioError(msg.into())
    }

    /// 创建模型错误
    pub fn model(msg: impl Into<String>) -> Self {
        Self::ModelError(msg.into())
    }

    /// 创建配置错误
    pub fn config(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }

    /// 创建设备错误
    pub fn device(msg: impl Into<String>) -> Self {
        Self::DeviceError(msg.into())
    }

    /// 创建推理错误
    pub fn inference(msg: impl Into<String>) -> Self {
        Self::InferenceError(msg.into())
    }

    /// 创建 F0 错误
    pub fn f0(msg: impl Into<String>) -> Self {
        Self::F0Error(msg.into())
    }

    /// 创建数学错误
    pub fn math(msg: impl Into<String>) -> Self {
        Self::MathError(msg.into())
    }

    /// 创建内存错误
    pub fn memory(msg: impl Into<String>) -> Self {
        Self::MemoryError(msg.into())
    }

    /// 创建并发错误
    pub fn concurrency(msg: impl Into<String>) -> Self {
        Self::ConcurrencyError(msg.into())
    }

    /// 创建不支持的操作错误
    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::UnsupportedError(msg.into())
    }

    /// 创建参数错误
    pub fn parameter(msg: impl Into<String>) -> Self {
        Self::ParameterError(msg.into())
    }

    /// 创建其他错误
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = RvcError::audio("test error");
        assert!(matches!(err, RvcError::AudioError(_)));
    }

    #[test]
    fn test_error_display() {
        let err = RvcError::audio("test message");
        assert_eq!(err.to_string(), "音频处理错误: test message");
    }
}
