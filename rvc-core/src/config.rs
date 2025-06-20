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
    pub pth_path: Option<String>,
    /// 索引文件路径
    pub index_path: Option<String>,
    /// 音调设置
    pub pitch: Option<i32>,
    /// 性别因子/声线粗细
    pub formant: Option<f32>,
    /// 采样率类型
    pub sr_type: Option<String>,
    /// 采样长度
    pub block_time: Option<f32>,
    /// 响应阈值 (注意: Python中拼写错误为threhold)
    pub threshold: Option<f32>,
    /// 淡入淡出长度
    pub crossfade_time: Option<f32>,
    /// 额外推理时长
    pub extra_time: Option<f32>,
    /// 是否启用输入降噪
    pub i_noise_reduce: Option<bool>,
    /// 是否启用输出降噪
    pub o_noise_reduce: Option<bool>,
    /// 是否启用相位声码器
    pub use_pv: Option<bool>,
    /// 响度因子
    pub rms_mix_rate: Option<f32>,
    /// Index Rate
    pub index_rate: Option<f32>,
    /// CPU 核心数
    pub n_cpu: Option<usize>,
    /// F0 方法
    pub f0method: Option<String>,
    /// 主机API
    pub sg_hostapi: Option<String>,
    /// WASAPI独占
    pub sg_wasapi_exclusive: Option<bool>,
    /// 输入设备名称
    pub sg_input_device: Option<String>,
    /// 输出设备名称
    pub sg_output_device: Option<String>,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            pth_path: Some(String::new()),
            index_path: Some(String::new()),
            pitch: Some(0),
            formant: Some(0.0),
            sr_type: Some("sr_model".to_string()),
            block_time: Some(0.25),
            threshold: Some(-60.0),
            crossfade_time: Some(0.05),
            extra_time: Some(2.5),
            i_noise_reduce: Some(false),
            o_noise_reduce: Some(false),
            use_pv: Some(false),
            rms_mix_rate: Some(0.0),
            index_rate: Some(0.0),
            n_cpu: Some(num_cpus::get().min(4)),
            f0method: Some("fcpe".to_string()),
            sg_hostapi: Some(String::new()),
            sg_wasapi_exclusive: Some(false),
            sg_input_device: Some(String::new()),
            sg_output_device: Some(String::new()),
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
        // 文件不存在时使用默认配置
        if !path.exists() {
            return Ok(Self::default());
        }

        // 读取文件失败时使用默认配置
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(_) => return Ok(Self::default()),
        };

        // JSON解析失败时使用默认配置并与文件内容合并
        let mut config = match serde_json::from_str::<Self>(&content) {
            Ok(config) => config,
            Err(_) => Self::default(),
        };

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

    /// 更新派生的配置值
    pub fn update_derived_values(&mut self) {
        // GUI配置中的派生值可以在这里添加
    }

    /// 从 JSON 值安全更新配置，统一处理类型转换和验证
    /// 返回验证错误列表，如果为空则表示所有字段都有效
    pub fn update_from_json(&mut self, data: &serde_json::Value) -> Vec<String> {
        let mut errors = Vec::new();

        // 路径字段 - 进行安全性验证
        if let Some(pth_path) = data.get("pth_path").and_then(|v| v.as_str()) {
            if pth_path.contains("..") {
                errors.push("pth文件路径不可包含相对路径标识".to_string());
            } else if pth_path.len() >= 1024 {
                errors.push("pth文件路径过长".to_string());
            } else if !pth_path.is_empty() && !std::path::Path::new(pth_path).exists() {
                errors.push("pth文件不存在".to_string());
            } else {
                self.pth_path = Some(pth_path.to_string());
            }
        }

        if let Some(index_path) = data.get("index_path").and_then(|v| v.as_str()) {
            if index_path.contains("..") {
                errors.push("index文件路径不可包含相对路径标识".to_string());
            } else if index_path.len() >= 1024 {
                errors.push("index文件路径过长".to_string());
            } else if !index_path.is_empty() && !std::path::Path::new(index_path).exists() {
                errors.push("index文件不存在".to_string());
            } else {
                self.index_path = Some(index_path.to_string());
            }
        }

        // 数值字段 - 进行范围验证
        if let Some(pitch) = data.get("pitch").and_then(|v| v.as_i64()) {
            let pitch = pitch as i32;
            if pitch < -16 || pitch > 16 {
                errors.push("音调设置必须在 -16 到 16 之间".to_string());
            } else {
                self.pitch = Some(pitch);
            }
        }

        if let Some(formant) = data.get("formant").and_then(|v| v.as_f64()) {
            let formant = formant as f32;
            if formant < -2.0 || formant > 2.0 {
                errors.push("性别因子必须在 -2.0 到 2.0 之间".to_string());
            } else {
                self.formant = Some(formant);
            }
        }

        if let Some(block_time) = data.get("block_time").and_then(|v| v.as_f64()) {
            let block_time = block_time as f32;
            if block_time < 0.02 || block_time > 1.5 {
                errors.push("采样长度必须在 0.02 到 1.5 之间".to_string());
            } else {
                self.block_time = Some(block_time);
            }
        }

        if let Some(threshold) = data.get("threshold").and_then(|v| v.as_f64()) {
            let threshold = threshold as f32;
            if threshold < -60.0 || threshold > 0.0 {
                errors.push("响应阈值必须在 -60 到 0 之间".to_string());
            } else {
                self.threshold = Some(threshold);
            }
        }

        if let Some(crossfade_time) = data.get("crossfade_time").and_then(|v| v.as_f64()) {
            let crossfade_time = crossfade_time as f32;
            if crossfade_time < 0.01 || crossfade_time > 0.15 {
                errors.push("淡入淡出长度必须在 0.01 到 0.15 之间".to_string());
            } else {
                self.crossfade_time = Some(crossfade_time);
            }
        }

        if let Some(extra_time) = data.get("extra_time").and_then(|v| v.as_f64()) {
            let extra_time = extra_time as f32;
            if extra_time < 0.05 || extra_time > 5.0 {
                errors.push("额外推理时长必须在 0.05 到 5.0 之间".to_string());
            } else {
                self.extra_time = Some(extra_time);
            }
        }

        if let Some(rms_mix_rate) = data.get("rms_mix_rate").and_then(|v| v.as_f64()) {
            let rms_mix_rate = rms_mix_rate as f32;
            if rms_mix_rate < 0.0 || rms_mix_rate > 1.0 {
                errors.push("响度因子必须在 0.0 到 1.0 之间".to_string());
            } else {
                self.rms_mix_rate = Some(rms_mix_rate);
            }
        }

        if let Some(index_rate) = data.get("index_rate").and_then(|v| v.as_f64()) {
            let index_rate = index_rate as f32;
            if index_rate < 0.0 || index_rate > 1.0 {
                errors.push("Index Rate 必须在 0.0 到 1.0 之间".to_string());
            } else {
                self.index_rate = Some(index_rate);
            }
        }

        if let Some(n_cpu) = data.get("n_cpu").and_then(|v| v.as_u64()) {
            let n_cpu = n_cpu as usize;
            let max_cpu = num_cpus::get();
            if n_cpu < 1 || n_cpu > max_cpu {
                errors.push(format!("CPU核心数必须在 1 到 {} 之间", max_cpu));
            } else {
                self.n_cpu = Some(n_cpu);
            }
        }

        // 布尔字段 - 无需验证
        if let Some(i_noise_reduce) = data.get("i_noise_reduce").and_then(|v| v.as_bool()) {
            self.i_noise_reduce = Some(i_noise_reduce);
        }

        if let Some(o_noise_reduce) = data.get("o_noise_reduce").and_then(|v| v.as_bool()) {
            self.o_noise_reduce = Some(o_noise_reduce);
        }

        if let Some(use_pv) = data.get("use_pv").and_then(|v| v.as_bool()) {
            self.use_pv = Some(use_pv);
        }

        if let Some(sg_wasapi_exclusive) = data.get("sg_wasapi_exclusive").and_then(|v| v.as_bool())
        {
            self.sg_wasapi_exclusive = Some(sg_wasapi_exclusive);
        }

        // 枚举字段 - 进行有效值验证
        if let Some(sr_type) = data.get("sr_type").and_then(|v| v.as_str()) {
            if matches!(sr_type, "sr_model" | "sr_device") {
                self.sr_type = Some(sr_type.to_string());
            } else {
                errors.push("不支持的采样率类型".to_string());
            }
        }

        if let Some(f0method) = data.get("f0method").and_then(|v| v.as_str()) {
            if matches!(f0method, "pm" | "harvest" | "crepe" | "rmvpe" | "fcpe") {
                self.f0method = Some(f0method.to_string());
            } else {
                errors.push("不支持的 F0 方法".to_string());
            }
        }

        // 字符串字段 - 长度限制
        if let Some(sg_hostapi) = data.get("sg_hostapi").and_then(|v| v.as_str()) {
            if sg_hostapi.len() >= 256 {
                errors.push("主机API名称过长".to_string());
            } else {
                self.sg_hostapi = Some(sg_hostapi.to_string());
            }
        }

        if let Some(sg_input_device) = data.get("sg_input_device").and_then(|v| v.as_str()) {
            if sg_input_device.len() >= 512 {
                errors.push("输入设备名称过长".to_string());
            } else {
                self.sg_input_device = Some(sg_input_device.to_string());
            }
        }

        if let Some(sg_output_device) = data.get("sg_output_device").and_then(|v| v.as_str()) {
            if sg_output_device.len() >= 512 {
                errors.push("输出设备名称过长".to_string());
            } else {
                self.sg_output_device = Some(sg_output_device.to_string());
            }
        }

        errors
    }

    /// 从 JSON 值单独更新一个字段，用于实时参数更新
    /// 返回验证错误列表，如果为空则表示字段有效
    pub fn update_field_from_json(&mut self, name: &str, value: serde_json::Value) -> Vec<String> {
        let mut temp_json = serde_json::Map::new();
        temp_json.insert(name.to_string(), value);
        let temp_value = serde_json::Value::Object(temp_json);
        self.update_from_json(&temp_value)
    }

    /// 设置 F0 方法
    pub fn set_f0method(&mut self, method: String) {
        self.f0method = Some(method);
        self.update_derived_values();
    }

    /// 设置采样率类型
    pub fn set_sr_type(&mut self, sr_type: String) {
        self.sr_type = Some(sr_type);
        self.update_derived_values();
    }

    /// 验证GUI配置的有效性
    /// 现在只是对 update_from_json 的简单包装，保持向后兼容性
    pub fn validate(&self) -> RvcResult<()> {
        // 将当前配置序列化为JSON，然后用update_from_json验证
        let json_value = serde_json::to_value(self)
            .map_err(|e| RvcError::config(format!("配置序列化失败: {}", e)))?;

        let mut temp_config = GuiConfig::default();
        let errors = temp_config.update_from_json(&json_value);

        if !errors.is_empty() {
            return Err(RvcError::config(format!(
                "配置验证失败: {}",
                errors.join(", ")
            )));
        }

        Ok(())
    }
}

/// RVC 系统配置结构体，对应 Python configs/config.py 中的 Config 类
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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

    // GUI 相关的配置字段，保持与现有 JSON 兼容
    /// 模型文件路径
    #[serde(default)]
    pub pth_path: String,
    /// 索引文件路径
    #[serde(default)]
    pub index_path: String,
    /// 音调设置
    #[serde(default)]
    pub pitch: i32,
    /// 性别因子/声线粗细
    #[serde(default)]
    pub formant: f32,
    /// 采样率类型
    #[serde(default)]
    pub sr_type: String,
    /// 采样长度
    #[serde(default)]
    pub block_time: f32,
    /// 响应阈值 (注意: Python中拼写错误为threhold)
    #[serde(default, alias = "threhold")]
    pub threshold: f32,
    /// 淡入淡出长度
    #[serde(default, alias = "crossfade_length")]
    pub crossfade_time: f32,
    /// 额外推理时长
    #[serde(default)]
    pub extra_time: f32,
    /// 响度因子
    #[serde(default)]
    pub rms_mix_rate: f32,
    /// Index Rate
    #[serde(default)]
    pub index_rate: f32,
    /// F0 方法
    #[serde(default)]
    pub f0method: String,
    /// 主机API
    #[serde(default)]
    pub sg_hostapi: String,
    /// WASAPI独占
    #[serde(default)]
    pub sg_wasapi_exclusive: bool,
    /// 输入设备名称
    #[serde(default)]
    pub sg_input_device: String,
    /// 输出设备名称
    #[serde(default)]
    pub sg_output_device: String,
    /// 是否启用相位声码器
    #[serde(default)]
    pub use_pv: bool,
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
            // GUI 字段使用默认值
            pth_path: String::new(),
            index_path: String::new(),
            pitch: 0,
            formant: 0.0,
            sr_type: "sr_model".to_string(),
            block_time: 0.3,
            threshold: -60.0,
            crossfade_time: 0.08,
            extra_time: 2.0,
            rms_mix_rate: 0.25,
            index_rate: 0.75,
            f0method: "harvest".to_string(),
            sg_hostapi: String::new(),
            sg_wasapi_exclusive: false,
            sg_input_device: String::new(),
            sg_output_device: String::new(),
            use_pv: false,
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

    /// 更新派生的配置值
    pub fn update_derived_values(&mut self) {
        // 配置值的更新逻辑
        // 大部分字段已经在反序列化时处理
    }

    /// 转换为 GuiConfig
    pub fn to_gui_config(&self) -> GuiConfig {
        GuiConfig {
            pth_path: Some(self.pth_path.clone()),
            index_path: Some(self.index_path.clone()),
            pitch: Some(self.pitch),
            formant: Some(self.formant),
            sr_type: Some(self.sr_type.clone()),
            block_time: Some(self.block_time),
            threshold: Some(self.threshold),
            crossfade_time: Some(self.crossfade_time),
            extra_time: Some(self.extra_time),
            i_noise_reduce: Some(false),
            o_noise_reduce: Some(false),
            use_pv: Some(self.use_pv),
            rms_mix_rate: Some(self.rms_mix_rate),
            index_rate: Some(self.index_rate),
            n_cpu: Some(if self.n_cpu > 0 {
                self.n_cpu
            } else {
                num_cpus::get().min(4)
            }),
            f0method: Some(self.f0method.clone()),
            sg_hostapi: Some(self.sg_hostapi.clone()),
            sg_wasapi_exclusive: Some(self.sg_wasapi_exclusive),
            sg_input_device: Some(self.sg_input_device.clone()),
            sg_output_device: Some(self.sg_output_device.clone()),
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_update_from_json_numeric_fields() {
        let mut config = GuiConfig::default();

        let test_data = json!({
            "pitch": 5,
            "formant": 1.5,
            "block_time": 0.5,
            "threshold": -30.0,
            "index_rate": 0.8,
            "n_cpu": 8
        });

        let errors = config.update_from_json(&test_data);

        assert!(
            errors.is_empty(),
            "Should have no validation errors: {:?}",
            errors
        );
        assert_eq!(config.pitch, Some(5));
        assert_eq!(config.formant, Some(1.5));
        assert_eq!(config.block_time, Some(0.5));
        assert_eq!(config.threshold, Some(-30.0));
        assert_eq!(config.index_rate, Some(0.8));
        assert_eq!(config.n_cpu, Some(8));
    }

    #[test]
    fn test_update_from_json_range_clamping() {
        // 创建一个没有默认值的空配置
        let mut config = GuiConfig {
            pth_path: None,
            index_path: None,
            pitch: None,
            formant: None,
            sr_type: None,
            block_time: None,
            threshold: None,
            crossfade_time: None,
            extra_time: None,
            i_noise_reduce: None,
            o_noise_reduce: None,
            use_pv: None,
            rms_mix_rate: None,
            index_rate: None,
            n_cpu: None,
            f0method: None,
            sg_hostapi: None,
            sg_wasapi_exclusive: None,
            sg_input_device: None,
            sg_output_device: None,
        };

        let test_data = json!({
            "pitch": 100,        // 应该被拒绝，超出范围
            "formant": -5.0,     // 应该被拒绝，超出范围
            "threshold": 50.0,   // 应该被拒绝，超出范围
            "index_rate": 1.5,   // 应该被拒绝，超出范围
            "n_cpu": 999         // 应该被拒绝，超出范围
        });

        let errors = config.update_from_json(&test_data);

        // 应该有验证错误，因为数值超出范围
        assert!(
            !errors.is_empty(),
            "Should have validation errors for out-of-range values"
        );

        // 所有字段应该保持 None，因为验证失败
        assert_eq!(config.pitch, None);
        assert_eq!(config.formant, None);
        assert_eq!(config.threshold, None);
        assert_eq!(config.index_rate, None);
        assert_eq!(config.n_cpu, None);
    }

    #[test]
    fn test_update_from_json_string_validation() {
        let mut config = GuiConfig::default();

        // 测试有效的枚举值
        let valid_data = json!({
            "sr_type": "sr_model",
            "f0method": "harvest"
        });

        let errors = config.update_from_json(&valid_data);
        assert!(
            errors.is_empty(),
            "Valid data should not produce errors: {:?}",
            errors
        );
        assert_eq!(config.sr_type, Some("sr_model".to_string()));
        assert_eq!(config.f0method, Some("harvest".to_string()));

        // 测试无效的枚举值
        let invalid_data = json!({
            "sr_type": "invalid_type",
            "f0method": "invalid_method"
        });

        let errors = config.update_from_json(&invalid_data);
        // 应该有验证错误
        assert!(
            !errors.is_empty(),
            "Invalid enum values should produce errors"
        );
        // 应该保持之前的有效值，不被无效值覆盖
        assert_eq!(config.sr_type, Some("sr_model".to_string()));
        assert_eq!(config.f0method, Some("harvest".to_string()));
    }

    #[test]
    fn test_update_from_json_path_security() {
        let mut config = GuiConfig::default();

        // 测试安全路径
        let safe_data = json!({
            "pth_path": "/safe/path/model.pth",
            "index_path": "/safe/path/index.index"
        });

        let errors = config.update_from_json(&safe_data);
        // 注意：这里可能有文件不存在的错误，因为测试路径可能不存在
        if !errors.is_empty() {
            // 如果有错误，应该是文件不存在的错误
            assert!(errors.iter().any(|e| e.contains("不存在")));
        }

        // 测试不安全路径（包含..）
        let unsafe_data = json!({
            "pth_path": "../../../etc/passwd",
            "index_path": "../../dangerous/path"
        });

        let errors = config.update_from_json(&unsafe_data);
        // 应该有安全性验证错误
        assert!(!errors.is_empty(), "Unsafe paths should produce errors");
        assert!(errors.iter().any(|e| e.contains("相对路径")));
    }

    #[test]
    fn test_update_from_json_boolean_fields() {
        let mut config = GuiConfig::default();

        let test_data = json!({
            "use_pv": true,
            "i_noise_reduce": false,
            "o_noise_reduce": true,
            "sg_wasapi_exclusive": false
        });

        let errors = config.update_from_json(&test_data);

        assert!(
            errors.is_empty(),
            "Boolean fields should not produce errors: {:?}",
            errors
        );
        assert_eq!(config.use_pv, Some(true));
        assert_eq!(config.i_noise_reduce, Some(false));
        assert_eq!(config.o_noise_reduce, Some(true));
        assert_eq!(config.sg_wasapi_exclusive, Some(false));
    }

    #[test]
    fn test_update_field_from_json() {
        let mut config = GuiConfig::default();

        // 测试单个字段更新
        let errors = config.update_field_from_json("pitch", json!(10));
        assert!(
            errors.is_empty(),
            "Valid pitch should not produce errors: {:?}",
            errors
        );
        assert_eq!(config.pitch, Some(10));

        let errors = config.update_field_from_json("formant", json!(0.5));
        assert!(
            errors.is_empty(),
            "Valid formant should not produce errors: {:?}",
            errors
        );
        assert_eq!(config.formant, Some(0.5));

        let errors = config.update_field_from_json("f0method", json!("crepe"));
        assert!(
            errors.is_empty(),
            "Valid f0method should not produce errors: {:?}",
            errors
        );
        assert_eq!(config.f0method, Some("crepe".to_string()));
    }

    #[test]
    fn test_update_from_json_type_safety() {
        let mut config = GuiConfig::default();

        // 记录原始默认值
        let original_pitch = config.pitch;
        let original_formant = config.formant;
        let original_use_pv = config.use_pv;
        let original_sr_type = config.sr_type.clone();

        // 测试错误的类型不会导致panic，而是被忽略
        let wrong_types = json!({
            "pitch": "not a number",
            "formant": true,
            "use_pv": "not a boolean",
            "sr_type": 123
        });

        let errors = config.update_from_json(&wrong_types);

        // 错误类型不应该产生验证错误，只是被忽略
        assert!(
            errors.is_empty(),
            "Wrong types should be ignored, not produce errors"
        );

        // 所有字段应该保持原有的默认值，不被错误类型覆盖
        assert_eq!(config.pitch, original_pitch);
        assert_eq!(config.formant, original_formant);
        assert_eq!(config.use_pv, original_use_pv);
        assert_eq!(config.sr_type, original_sr_type);
    }

    #[test]
    fn test_validate_method() {
        let mut config = GuiConfig::default();

        // 设置有效值
        config.pitch = Some(5);
        config.formant = Some(1.0);
        config.threshold = Some(-30.0);
        config.f0method = Some("harvest".to_string());
        config.sr_type = Some("sr_model".to_string());

        assert!(config.validate().is_ok());

        // 设置无效值
        config.pitch = Some(100); // 超出范围
        assert!(config.validate().is_err());

        config.pitch = Some(5); // 恢复有效值
        config.f0method = Some("invalid".to_string()); // 无效枚举
        assert!(config.validate().is_err());
    }
}
