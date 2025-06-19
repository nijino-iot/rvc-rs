//! RVC Core Library
//!
//! 这是 RVC (Retrieval-based Voice Conversion) 项目的核心 Rust 库，
//! 负责音频处理、语音转换和相关的深度学习推理功能。

pub mod config;
pub mod error;
pub mod f0;
pub mod gui;
pub mod pytorch_loader;
pub mod rtrvc;
pub mod sd;

pub mod utils;

// 重新导出主要类型和函数（避免重复导出）
pub use config::{Config, ConfigManager, GuiConfig};
pub use error::*;
pub use f0::*;
pub use gui::GuiManager;
pub use pytorch_loader::*;
pub use rtrvc::*;
pub use sd::{printt, AudioStream, DeviceInfo, HostApiInfo};

pub use utils::*;

use log::info;

/// 初始化 RVC 核心库
pub fn init() -> Result<(), RvcError> {
    env_logger::init();
    info!("RVC Core library initialized");

    // 初始化 PyTorch/tch 后端
    if !tch::Cuda::is_available() {
        info!("CUDA not available, using CPU");
    } else {
        info!("CUDA available");
    }

    Ok(())
}

/// 获取库版本信息
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        assert!(init().is_ok());
    }

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
