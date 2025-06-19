//! 配置管理模块
//!
//! 对应 Python 代码中的 GUIConfig 类和配置管理功能

use crate::{RvcError, RvcResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// GUI 配置结构体，对应 Python gui_v1.py 中的 GUIConfig 类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    /// 模型文件路径
    pub pth_path: String,
    /// 索引文件路径
    pub index_path: String,
    /// 音调设置
    pub pitch: i32,
    /// 性别因子/声线粗细
    pub formant: f32,
    /// 采样率类型
    pub sr_type: String,
    /// 采样长度
    pub block_time: f32,
    /// 响应阈值 (注意: Python中拼写错误为threhold)
    pub threshold: f32,
    /// 淡入淡出长度
    pub crossfade_time: f32,
    /// 额外推理时长
    pub extra_time: f32,
    /// 是否启用输入降噪
    pub i_noise_reduce: bool,
    /// 是否启用输出降噪
    pub o_noise_reduce: bool,
    /// 是否启用相位声码器
    pub use_pv: bool,
    /// 响度因子
    pub rms_mix_rate: f32,
    /// Index Rate
    pub index_rate: f32,
    /// CPU 核心数
    pub n_cpu: usize,
    /// F0 方法
    pub f0method: String,
    /// 主机API
    pub sg_hostapi: String,
    /// WASAPI独占
    pub wasapi_exclusive: bool,
    /// 输入设备名称
    pub sg_input_device: String,
    /// 输出设备名称
    pub sg_output_device: String,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            pth_path: String::new(),
            index_path: String::new(),
            pitch: 0,
            formant: 0.0,
            sr_type: "sr_model".to_string(),
            block_time: 0.25,
            threshold: -60.0,
            crossfade_time: 0.05,
            extra_time: 2.5,
            i_noise_reduce: false,
            o_noise_reduce: false,
            use_pv: false,
            rms_mix_rate: 0.0,
            index_rate: 0.0,
            n_cpu: num_cpus::get().min(4),
            f0method: "fcpe".to_string(),
            sg_hostapi: String::new(),
            wasapi_exclusive: false,
            sg_input_device: String::new(),
            sg_output_device: String::new(),
        }
    }
}

impl GuiConfig {
    /// 创建新的 GUI 配置
    pub fn new() -> Self {
        Self::default()
    }

    /// 从 JSON 文件加载配置
    pub fn load_from_file(path: &PathBuf) -> RvcResult<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            RvcError::config(format!("无法读取GUI配置文件 {}: {}", path.display(), e))
        })?;

        let mut config: Self = serde_json::from_str(&content)
            .map_err(|e| RvcError::config(format!("GUI配置文件格式错误: {}", e)))?;

        // 更新派生值
        config.update_derived_values();

        Ok(config)
    }

    /// 保存配置到 JSON 文件
    pub fn save_to_file(&self, path: &PathBuf) -> RvcResult<()> {
        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| RvcError::config(format!("无法创建GUI配置目录: {}", e)))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| RvcError::config(format!("序列化GUI配置失败: {}", e)))?;

        std::fs::write(path, content)
            .map_err(|e| RvcError::config(format!("无法写入GUI配置文件: {}", e)))?;

        Ok(())
    }

    /// 更新派生的布尔值
    pub fn update_derived_values(&mut self) {
        // GUI配置中的派生值可以在这里添加
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

    /// 验证GUI配置的有效性
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

        if self.crossfade_time < 0.01 || self.crossfade_time > 0.15 {
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

/// RVC 系统配置结构体，对应 Python configs/config.py 中的 Config 类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 设备类型 (cuda:0, cpu等)
    pub device: String,
    /// 是否使用半精度
    pub is_half: bool,
    /// 是否使用 JIT 编译
    pub use_jit: bool,
    /// CPU 核心数
    pub n_cpu: usize,
    /// GPU 名称
    pub gpu_name: Option<String>,
    /// GPU 内存大小
    pub gpu_mem: Option<usize>,
    /// Python 命令
    pub python_cmd: String,
    /// 监听端口
    pub listen_port: u16,
    /// 是否为 Colab 环境
    pub iscolab: bool,
    /// 是否禁用并行
    pub noparallel: bool,
    /// 是否禁用自动打开
    pub noautoopen: bool,
    /// 是否使用 DirectML
    pub dml: bool,
    /// 替代字符串
    pub instead: String,
    /// 预处理百分比
    pub preprocess_per: f32,
    /// 设备配置参数
    pub x_pad: usize,
    pub x_query: usize,
    pub x_center: usize,
    pub x_max: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device: "cuda:0".to_string(),
            is_half: true,
            use_jit: false,
            n_cpu: 0,
            gpu_name: None,
            gpu_mem: None,
            python_cmd: "python".to_string(),
            listen_port: 7865,
            iscolab: false,
            noparallel: false,
            noautoopen: false,
            dml: false,
            instead: String::new(),
            preprocess_per: 3.7,
            x_pad: 3,
            x_query: 10,
            x_center: 60,
            x_max: 65,
            threshold: 0.5,
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

    /// 检测设备配置
    pub fn device_config(&mut self) -> (usize, usize, usize, usize) {
        // 根据设备类型和内存配置参数
        if self.device.contains("cuda") {
            if let Some(gpu_mem) = self.gpu_mem {
                if gpu_mem <= 4 {
                    self.x_pad = 1;
                    self.x_query = 5;
                    self.x_center = 30;
                    self.x_max = 32;
                } else if gpu_mem <= 8 {
                    self.x_pad = 2;
                    self.x_query = 8;
                    self.x_center = 45;
                    self.x_max = 48;
                } else {
                    self.x_pad = 3;
                    self.x_query = 10;
                    self.x_center = 60;
                    self.x_max = 65;
                }
            }
        } else {
            // CPU 配置
            self.x_pad = 1;
            self.x_query = 6;
            self.x_center = 38;
            self.x_max = 41;
        }

        (self.x_pad, self.x_query, self.x_center, self.x_max)
    }

    /// 是否有GPU
    pub fn has_gpu(&self) -> bool {
        self.device.contains("cuda") && !self.device.contains("cpu")
    }

    /// 是否使用MPS (Apple Silicon)
    pub fn has_mps(&self) -> bool {
        self.device.contains("mps")
    }

    /// 验证配置的有效性
    pub fn validate(&self) -> RvcResult<()> {
        // 验证端口范围
        if self.listen_port < 1024 || self.listen_port > 65535 {
            return Err(RvcError::config("端口号必须在 1024 到 65535 之间"));
        }

        // 验证设备字符串
        if !self.device.contains("cpu")
            && !self.device.contains("cuda")
            && !self.device.contains("mps")
        {
            return Err(RvcError::config("无效的设备类型"));
        }

        // 验证预处理百分比
        if self.preprocess_per < 0.0 || self.preprocess_per > 10.0 {
            return Err(RvcError::config("预处理百分比必须在 0.0 到 10.0 之间"));
        }

        Ok(())
    }
}

/// 配置管理器，负责管理配置的加载、保存和更新
#[derive(Debug)]
pub struct ConfigManager {
    /// 系统配置文件路径
    config_path: PathBuf,
    /// GUI配置文件路径
    gui_config_path: PathBuf,
    /// 当前系统配置
    config: Config,
    /// GUI 配置
    gui_config: GuiConfig,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new(config_path: PathBuf) -> Self {
        let gui_config_path = config_path.with_file_name("gui_config.json");
        Self {
            config_path,
            gui_config_path,
            config: Config::new(),
            gui_config: GuiConfig::new(),
        }
    }

    /// 加载配置
    pub fn load(&mut self) -> RvcResult<()> {
        // 加载系统配置
        if self.config_path.exists() {
            self.config = Config::load_from_file(&self.config_path)?;
        } else {
            self.config = Config::new();
            self.save_config()?;
        }

        // 加载GUI配置
        if self.gui_config_path.exists() {
            self.gui_config = GuiConfig::load_from_file(&self.gui_config_path)?;
        } else {
            self.gui_config = GuiConfig::new();
            self.save_gui_config()?;
        }

        Ok(())
    }

    /// 保存系统配置
    pub fn save_config(&self) -> RvcResult<()> {
        self.config.save_to_file(&self.config_path)
    }

    /// 保存GUI配置
    pub fn save_gui_config(&self) -> RvcResult<()> {
        self.gui_config.save_to_file(&self.gui_config_path)
    }

    /// 保存所有配置
    pub fn save(&self) -> RvcResult<()> {
        self.save_config()?;
        self.save_gui_config()?;
        Ok(())
    }

    /// 获取系统配置的引用
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// 获取可变系统配置的引用
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// 获取 GUI 配置的引用
    pub fn gui_config(&self) -> &GuiConfig {
        &self.gui_config
    }

    /// 获取可变 GUI 配置的引用
    pub fn gui_config_mut(&mut self) -> &mut GuiConfig {
        &mut self.gui_config
    }

    /// 更新系统配置并保存
    pub fn update_config<F>(&mut self, f: F) -> RvcResult<()>
    where
        F: FnOnce(&mut Config),
    {
        f(&mut self.config);
        self.config.validate()?;
        self.save_config()
    }

    /// 更新GUI配置并保存
    pub fn update_gui_config<F>(&mut self, f: F) -> RvcResult<()>
    where
        F: FnOnce(&mut GuiConfig),
    {
        f(&mut self.gui_config);
        self.save_gui_config()
    }
}
