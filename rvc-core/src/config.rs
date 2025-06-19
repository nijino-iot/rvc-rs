//! 配置管理模块
//!
//! 对应 Python 代码中的 GUIConfig 类和配置管理功能

use crate::{RvcError, RvcResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// GUI 配置结构体，对应 Python 中的 GUIConfig 类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    /// CPU 核心数，对应 Python 中的 n_cpu
    pub n_cpu: usize,
    // 其他配置参数可以在这里添加
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            n_cpu: num_cpus::get().min(4), // 默认使用系统 CPU 核心数，最多 4 个
        }
    }
}

impl GuiConfig {
    /// 创建新的 GUI 配置
    pub fn new() -> Self {
        Self::default()
    }
}

/// RVC 主配置结构体，对应 Python 中 Config 类的功能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 模型文件路径
    pub pth_path: String,
    /// 索引文件路径
    pub index_path: String,
    /// 音频设备相关配置
    pub sg_hostapi: String,
    pub sg_wasapi_exclusive: bool,
    pub sg_input_device: String,
    pub sg_output_device: String,
    /// 采样率类型
    pub sr_type: String,
    /// 响应阈值
    pub threshold: f32,
    /// 音调设置
    pub pitch: i32,
    /// 性别因子/声线粗细
    pub formant: f32,
    /// Index Rate
    pub index_rate: f32,
    /// 响度因子
    pub rms_mix_rate: f32,
    /// 采样长度
    pub block_time: f32,
    /// 淡入淡出长度
    pub crossfade_length: f32,
    /// 额外推理时长
    pub extra_time: f32,
    /// harvest 进程数
    pub n_cpu: usize,
    /// F0 方法
    pub f0method: String,
    /// 是否使用 JIT
    pub use_jit: bool,
    /// 是否启用相位声码器
    pub use_pv: bool,
    /// 是否启用输入降噪
    pub i_noise_reduce: bool,
    /// 是否启用输出降噪
    pub o_noise_reduce: bool,
    /// 派生的布尔值配置
    pub sr_model: bool,
    pub sr_device: bool,
    pub pm: bool,
    pub harvest: bool,
    pub crepe: bool,
    pub rmvpe: bool,
    pub fcpe: bool,
}

impl Default for Config {
    fn default() -> Self {
        let f0method = "rmvpe".to_string();
        let sr_type = "sr_model".to_string();

        Self {
            pth_path: String::new(),
            index_path: String::new(),
            sg_hostapi: String::new(),
            sg_wasapi_exclusive: false,
            sg_input_device: String::new(),
            sg_output_device: String::new(),
            sr_type: sr_type.clone(),
            threshold: -60.0,
            pitch: 0,
            formant: 0.0,
            index_rate: 0.0,
            rms_mix_rate: 0.0,
            block_time: 0.25,
            crossfade_length: 0.05,
            extra_time: 2.5,
            n_cpu: 4,
            f0method: f0method.clone(),
            use_jit: false,
            use_pv: false,
            i_noise_reduce: false,
            o_noise_reduce: false,
            // 派生值
            sr_model: sr_type == "sr_model",
            sr_device: sr_type == "sr_device",
            pm: f0method == "pm",
            harvest: f0method == "harvest",
            crepe: f0method == "crepe",
            rmvpe: f0method == "rmvpe",
            fcpe: f0method == "fcpe",
        }
    }
}

impl Config {
    /// 创建新的配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 从 JSON 文件加载配置
    pub fn load_from_file(path: &PathBuf) -> RvcResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| RvcError::config(format!("无法读取配置文件 {}: {}", path.display(), e)))?;

        let mut config: Self = serde_json::from_str(&content)
            .map_err(|e| RvcError::config(format!("配置文件格式错误: {}", e)))?;

        // 更新派生值
        config.update_derived_values();

        Ok(config)
    }

    /// 保存配置到 JSON 文件
    pub fn save_to_file(&self, path: &PathBuf) -> RvcResult<()> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RvcError::config(format!("无法创建配置目录: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| RvcError::config(format!("序列化配置失败: {}", e)))?;

        std::fs::write(path, content)
            .map_err(|e| RvcError::config(format!("无法写入配置文件: {}", e)))?;

        Ok(())
    }

    /// 更新派生的布尔值
    pub fn update_derived_values(&mut self) {
        self.sr_model = self.sr_type == "sr_model";
        self.sr_device = self.sr_type == "sr_device";
        self.pm = self.f0method == "pm";
        self.harvest = self.f0method == "harvest";
        self.crepe = self.f0method == "crepe";
        self.rmvpe = self.f0method == "rmvpe";
        self.fcpe = self.f0method == "fcpe";
    }

    /// 设置 F0 方法
    pub fn set_f0method(&mut self, method: String) {
        self.f0method = method;
        self.update_derived_values();
    }

    /// 设置采样率类型
    pub fn set_sr_type(&mut self, sr_type: String) {
        self.sr_type = sr_type;
        self.update_derived_values();
    }

    /// 验证配置的有效性
    pub fn validate(&self) -> RvcResult<()> {
        // 验证路径
        if !self.pth_path.is_empty() && !std::path::Path::new(&self.pth_path).exists() {
            return Err(RvcError::config("模型文件不存在"));
        }

        if !self.index_path.is_empty() && !std::path::Path::new(&self.index_path).exists() {
            return Err(RvcError::config("索引文件不存在"));
        }

        // 验证数值范围
        if self.threshold < -60.0 || self.threshold > 0.0 {
            return Err(RvcError::config("响应阈值必须在 -60 到 0 之间"));
        }

        if self.pitch < -16 || self.pitch > 16 {
            return Err(RvcError::config("音调设置必须在 -16 到 16 之间"));
        }

        if self.formant < -2.0 || self.formant > 2.0 {
            return Err(RvcError::config("性别因子必须在 -2.0 到 2.0 之间"));
        }

        if self.index_rate < 0.0 || self.index_rate > 1.0 {
            return Err(RvcError::config("Index Rate 必须在 0.0 到 1.0 之间"));
        }

        if self.rms_mix_rate < 0.0 || self.rms_mix_rate > 1.0 {
            return Err(RvcError::config("响度因子必须在 0.0 到 1.0 之间"));
        }

        if self.block_time < 0.02 || self.block_time > 1.5 {
            return Err(RvcError::config("采样长度必须在 0.02 到 1.5 之间"));
        }

        if self.crossfade_length < 0.01 || self.crossfade_length > 0.15 {
            return Err(RvcError::config("淡入淡出长度必须在 0.01 到 0.15 之间"));
        }

        if self.extra_time < 0.05 || self.extra_time > 5.0 {
            return Err(RvcError::config("额外推理时长必须在 0.05 到 5.0 之间"));
        }

        // 验证 F0 方法
        match self.f0method.as_str() {
            "pm" | "harvest" | "crepe" | "rmvpe" | "fcpe" => {}
            _ => return Err(RvcError::config("不支持的 F0 方法")),
        }

        // 验证采样率类型
        match self.sr_type.as_str() {
            "sr_model" | "sr_device" => {}
            _ => return Err(RvcError::config("不支持的采样率类型")),
        }

        Ok(())
    }
}

/// 配置管理器，负责管理配置的加载、保存和更新
#[derive(Debug)]
pub struct ConfigManager {
    /// 配置文件路径
    config_path: PathBuf,
    /// 当前配置
    config: Config,
    /// GUI 配置
    gui_config: GuiConfig,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            config: Config::new(),
            gui_config: GuiConfig::new(),
        }
    }

    /// 加载配置
    pub fn load(&mut self) -> RvcResult<()> {
        if self.config_path.exists() {
            self.config = Config::load_from_file(&self.config_path)?;
        } else {
            // 如果配置文件不存在，创建默认配置并保存
            self.config = Config::new();
            self.save()?;
        }
        Ok(())
    }

    /// 保存配置
    pub fn save(&self) -> RvcResult<()> {
        self.config.save_to_file(&self.config_path)
    }

    /// 获取配置的引用
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取可变配置的引用
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// 获取 GUI 配置的引用
    pub fn gui_config(&self) -> &GuiConfig {
        &self.gui_config
    }

    /// 更新配置并保存
    pub fn update_config<F>(&mut self, f: F) -> RvcResult<()>
    where
        F: FnOnce(&mut Config),
    {
        f(&mut self.config);
        self.config.validate()?;
        self.save()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.f0method, "rmvpe");
        assert_eq!(config.sr_type, "sr_model");
        assert!(config.sr_model);
        assert!(!config.sr_device);
        assert!(config.rmvpe);
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();

        // 测试有效配置
        assert!(config.validate().is_ok());

        // 测试无效的响应阈值
        config.threshold = -70.0;
        assert!(config.validate().is_err());

        config.threshold = -30.0;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_derived_values() {
        let mut config = Config::default();

        config.set_f0method("harvest".to_string());
        assert!(config.harvest);
        assert!(!config.rmvpe);

        config.set_sr_type("sr_device".to_string());
        assert!(config.sr_device);
        assert!(!config.sr_model);
    }

    #[test]
    fn test_config_save_load() -> RvcResult<()> {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("test_config.json");

        // 创建并保存配置
        let mut original_config = Config::default();
        original_config.pitch = 5;
        original_config.threshold = -45.0;
        original_config.save_to_file(&config_path)?;

        // 加载配置
        let loaded_config = Config::load_from_file(&config_path)?;

        assert_eq!(loaded_config.pitch, 5);
        assert_eq!(loaded_config.threshold, -45.0);

        Ok(())
    }

    #[test]
    fn test_config_manager() -> RvcResult<()> {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("manager_config.json");

        let mut manager = ConfigManager::new(config_path);
        manager.load()?;

        // 更新配置
        manager.update_config(|config| {
            config.pitch = 3;
            config.set_f0method("crepe".to_string());
        })?;

        assert_eq!(manager.config().pitch, 3);
        assert!(manager.config().crepe);

        Ok(())
    }
}
