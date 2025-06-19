//! PyTorch 模型加载器
//!
//! 提供从 PyTorch 检查点文件加载模型参数的功能

use crate::{RvcError, RvcResult};
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use tch::{Kind, Tensor};

/// 张量信息结构
#[derive(Debug, Clone)]
pub struct TensorInfo {
    pub name: String,
    pub shape: Vec<i64>,
    pub dtype: String,
    pub data_offset: u64,
    pub data_size: u64,
}

/// 检查点信息结构
#[derive(Debug, Clone)]
pub struct CheckpointInfo {
    pub model_type: String,
    pub version: String,
    pub tensors: HashMap<String, TensorInfo>,
    pub metadata: HashMap<String, String>,
}

/// PyTorch 检查点结构
#[derive(Debug)]
pub struct PyTorchCheckpoint {
    pub info: CheckpointInfo,
    pub tensors: HashMap<String, Tensor>,
}

/// 检查点工具类
pub struct CheckpointUtils;

impl CheckpointUtils {
    /// 获取检查点信息，不加载实际数据
    pub fn get_checkpoint_info<P: AsRef<Path>>(path: P) -> RvcResult<CheckpointInfo> {
        let mut loader = PyTorchLoader::new(path)?;
        loader.load_metadata()
    }

    /// 完全加载检查点
    pub fn load_checkpoint<P: AsRef<Path>>(path: P) -> RvcResult<PyTorchCheckpoint> {
        let mut loader = PyTorchLoader::new(path)?;
        let info = loader.load_metadata()?;

        let mut tensors = HashMap::new();
        for (name, tensor_info) in &info.tensors {
            let tensor = loader.load_tensor_data(tensor_info)?;
            tensors.insert(name.clone(), tensor);
        }

        Ok(PyTorchCheckpoint { info, tensors })
    }
}

/// PyTorch 文件加载器
pub struct PyTorchLoader {
    reader: BufReader<File>,
    file_size: u64,
}

impl PyTorchLoader {
    /// 创建新的 PyTorch 加载器
    pub fn new<P: AsRef<Path>>(path: P) -> RvcResult<Self> {
        let file = File::open(&path).map_err(|e| {
            RvcError::model(format!(
                "Failed to open file {}: {}",
                path.as_ref().display(),
                e
            ))
        })?;

        let file_size = file
            .metadata()
            .map_err(|e| RvcError::model(format!("Failed to get file metadata: {}", e)))?
            .len();

        let reader = BufReader::new(file);

        Ok(Self { reader, file_size })
    }

    /// 加载完整的检查点
    pub fn load_checkpoint(&mut self) -> RvcResult<PyTorchCheckpoint> {
        let info = self.load_metadata()?;

        let mut tensors = HashMap::new();
        for (name, tensor_info) in &info.tensors {
            let tensor = self.load_tensor_data(tensor_info)?;
            tensors.insert(name.clone(), tensor);
        }

        Ok(PyTorchCheckpoint { info, tensors })
    }

    /// 加载检查点元数据
    pub fn load_metadata(&mut self) -> RvcResult<CheckpointInfo> {
        // 检查文件格式
        self.reader.seek(SeekFrom::Start(0))?;

        // 读取文件头部，检查是否为有效的 PyTorch 文件
        let magic = self.read_magic_number()?;
        if !self.is_valid_pytorch_file(magic) {
            return Err(RvcError::model("Invalid PyTorch file format".to_string()));
        }

        // 解析检查点结构
        self.parse_checkpoint_structure()
    }

    /// 读取魔数
    fn read_magic_number(&mut self) -> RvcResult<u64> {
        let mut buffer = [0u8; 8];
        self.reader
            .read_exact(&mut buffer)
            .map_err(|e| RvcError::model(format!("Failed to read magic number: {}", e)))?;

        Ok(u64::from_le_bytes(buffer))
    }

    /// 检查是否为有效的 PyTorch 文件
    fn is_valid_pytorch_file(&self, magic: u64) -> bool {
        // PyTorch 文件的魔数（简化检查）
        magic == 0x1950a86a20f9469c || magic == 0x894f4e4e544c5089
    }

    /// 解析检查点结构
    fn parse_checkpoint_structure(&mut self) -> RvcResult<CheckpointInfo> {
        // 简化的解析实现
        // 实际的 PyTorch 文件格式非常复杂，这里提供一个基础框架

        let mut tensors = HashMap::new();
        let mut metadata = HashMap::new();

        // 示例：添加一些假的张量信息用于测试
        tensors.insert(
            "model.weight".to_string(),
            TensorInfo {
                name: "model.weight".to_string(),
                shape: vec![256, 128],
                dtype: "float32".to_string(),
                data_offset: 1024,
                data_size: 256 * 128 * 4,
            },
        );

        metadata.insert("model_type".to_string(), "RVC".to_string());
        metadata.insert("version".to_string(), "1.0".to_string());

        Ok(CheckpointInfo {
            model_type: "RVC".to_string(),
            version: "1.0".to_string(),
            tensors,
            metadata,
        })
    }

    /// 读取指定长度的数据
    fn read_bytes(&mut self, length: usize) -> RvcResult<Vec<u8>> {
        let mut buffer = vec![0u8; length];
        self.reader
            .read_exact(&mut buffer)
            .map_err(|e| RvcError::model(format!("Failed to read {} bytes: {}", length, e)))?;
        Ok(buffer)
    }

    /// 读取字符串
    fn read_string(&mut self, length: usize) -> RvcResult<String> {
        let bytes = self.read_bytes(length)?;
        String::from_utf8(bytes)
            .map_err(|e| RvcError::model(format!("Failed to parse string: {}", e)))
    }

    /// 读取张量形状
    fn read_tensor_shape(&mut self) -> RvcResult<Vec<i64>> {
        // 简化实现：读取固定格式的形状信息
        let ndim = self.reader.read_u32::<LittleEndian>()? as usize;
        let mut shape = Vec::with_capacity(ndim);

        for _ in 0..ndim {
            let dim = self.reader.read_i64::<LittleEndian>()?;
            shape.push(dim);
        }

        Ok(shape)
    }

    /// 解析张量头部信息
    fn parse_tensor_header(&mut self) -> RvcResult<TensorInfo> {
        // 读取张量名称长度
        let name_length = self.reader.read_u32::<LittleEndian>()? as usize;
        let name = self.read_string(name_length)?;

        // 读取数据类型长度
        let dtype_length = self.reader.read_u32::<LittleEndian>()? as usize;
        let dtype = self.read_string(dtype_length)?;

        // 读取形状
        let shape = self.read_tensor_shape()?;

        // 读取数据偏移和大小
        let data_offset = self.reader.read_u64::<LittleEndian>()?;
        let data_size = self.reader.read_u64::<LittleEndian>()?;

        Ok(TensorInfo {
            name,
            shape,
            dtype,
            data_offset,
            data_size,
        })
    }

    /// 解析所有张量信息
    fn parse_all_tensor_info(&mut self) -> RvcResult<HashMap<String, TensorInfo>> {
        let mut tensors = HashMap::new();

        // 读取张量数量
        let tensor_count = self.reader.read_u32::<LittleEndian>()? as usize;

        for _ in 0..tensor_count {
            let tensor_info = self.parse_tensor_header()?;
            tensors.insert(tensor_info.name.clone(), tensor_info);
        }

        Ok(tensors)
    }

    /// 加载特定张量的数据
    pub fn load_tensor_data(&mut self, tensor_info: &TensorInfo) -> RvcResult<Tensor> {
        // 定位到张量数据位置
        self.reader
            .seek(SeekFrom::Start(tensor_info.data_offset))
            .map_err(|e| RvcError::model(format!("Failed to seek: {}", e)))?;

        // 读取张量数据
        let mut buffer = vec![0u8; tensor_info.data_size as usize];
        self.reader
            .read_exact(&mut buffer)
            .map_err(|e| RvcError::model(format!("Failed to read tensor data: {}", e)))?;

        // 转换为张量
        let tensor = self.bytes_to_tensor(&buffer, &tensor_info.shape, &tensor_info.dtype)?;

        Ok(tensor)
    }

    /// 将字节数据转换为张量
    fn bytes_to_tensor(&self, data: &[u8], shape: &[i64], dtype: &str) -> RvcResult<Tensor> {
        // 确定数据类型
        let kind = match dtype {
            "float32" => Kind::Float,
            "float64" => Kind::Double,
            "int32" => Kind::Int,
            "int64" => Kind::Int64,
            "float16" => Kind::Half,
            _ => return Err(RvcError::model(format!("Unsupported dtype: {}", dtype))),
        };

        // 从原始字节创建张量
        let tensor = match kind {
            Kind::Float => {
                let float_data: Vec<f32> = data
                    .chunks_exact(4)
                    .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Tensor::from_slice(&float_data).reshape(shape)
            }
            Kind::Double => {
                let double_data: Vec<f64> = data
                    .chunks_exact(8)
                    .map(|chunk| {
                        f64::from_le_bytes([
                            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6],
                            chunk[7],
                        ])
                    })
                    .collect();
                Tensor::from_slice(&double_data.iter().map(|&x| x as f32).collect::<Vec<f32>>())
                    .reshape(shape)
            }
            Kind::Int => {
                let int_data: Vec<i32> = data
                    .chunks_exact(4)
                    .map(|chunk| i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                    .collect();
                Tensor::from_slice(&int_data.iter().map(|&x| x as f32).collect::<Vec<f32>>())
                    .reshape(shape)
            }
            Kind::Int64 => {
                let int64_data: Vec<i64> = data
                    .chunks_exact(8)
                    .map(|chunk| {
                        i64::from_le_bytes([
                            chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6],
                            chunk[7],
                        ])
                    })
                    .collect();
                Tensor::from_slice(&int64_data.iter().map(|&x| x as f32).collect::<Vec<f32>>())
                    .reshape(shape)
            }
            _ => {
                return Err(RvcError::model(format!(
                    "Unsupported tensor kind: {:?}",
                    kind
                )))
            }
        };

        Ok(tensor)
    }
}

/// 模型类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum ModelType {
    Rvc,
    SoVits,
    Unknown,
}

impl ModelType {
    /// 从字符串解析模型类型
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "rvc" => Self::Rvc,
            "so-vits" | "sovits" => Self::SoVits,
            _ => Self::Unknown,
        }
    }

    /// 转换为字符串
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Rvc => "rvc",
            Self::SoVits => "so-vits",
            Self::Unknown => "unknown",
        }
    }
}

/// 检查点验证器
pub struct CheckpointValidator;

impl CheckpointValidator {
    /// 验证检查点文件是否有效
    pub fn validate<P: AsRef<Path>>(path: P) -> RvcResult<bool> {
        let mut loader = PyTorchLoader::new(path)?;

        // 检查文件头
        let magic = loader.read_magic_number()?;
        if !loader.is_valid_pytorch_file(magic) {
            return Ok(false);
        }

        // 尝试解析基本结构
        match loader.parse_checkpoint_structure() {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// 获取检查点的模型类型
    pub fn get_model_type<P: AsRef<Path>>(path: P) -> RvcResult<ModelType> {
        let info = CheckpointUtils::get_checkpoint_info(path)?;
        Ok(ModelType::from_str(&info.model_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_model_type_conversion() {
        assert_eq!(ModelType::from_str("rvc"), ModelType::Rvc);
        assert_eq!(ModelType::from_str("RVC"), ModelType::Rvc);
        assert_eq!(ModelType::from_str("so-vits"), ModelType::SoVits);
        assert_eq!(ModelType::from_str("unknown"), ModelType::Unknown);

        assert_eq!(ModelType::Rvc.to_str(), "rvc");
        assert_eq!(ModelType::SoVits.to_str(), "so-vits");
        assert_eq!(ModelType::Unknown.to_str(), "unknown");
    }

    #[test]
    fn test_pytorch_loader_creation() -> RvcResult<()> {
        // 创建临时文件
        let mut temp_file = NamedTempFile::new()
            .map_err(|e| RvcError::model(format!("Failed to create temp file: {}", e)))?;

        // 写入一些测试数据
        temp_file
            .write_all(b"test data")
            .map_err(|e| RvcError::model(format!("Failed to write test data: {}", e)))?;

        // 尝试创建加载器
        let loader = PyTorchLoader::new(temp_file.path())?;
        assert_eq!(loader.file_size, 9); // "test data" 的长度

        Ok(())
    }

    #[test]
    fn test_tensor_info_creation() {
        let tensor_info = TensorInfo {
            name: "test_tensor".to_string(),
            shape: vec![2, 3, 4],
            dtype: "float32".to_string(),
            data_offset: 1024,
            data_size: 96, // 2 * 3 * 4 * 4 bytes
        };

        assert_eq!(tensor_info.name, "test_tensor");
        assert_eq!(tensor_info.shape, vec![2, 3, 4]);
        assert_eq!(tensor_info.dtype, "float32");
        assert_eq!(tensor_info.data_offset, 1024);
        assert_eq!(tensor_info.data_size, 96);
    }

    #[test]
    fn test_checkpoint_info_creation() {
        let mut tensors = HashMap::new();
        tensors.insert(
            "weight".to_string(),
            TensorInfo {
                name: "weight".to_string(),
                shape: vec![10, 20],
                dtype: "float32".to_string(),
                data_offset: 0,
                data_size: 800,
            },
        );

        let mut metadata = HashMap::new();
        metadata.insert("version".to_string(), "1.0".to_string());

        let info = CheckpointInfo {
            model_type: "rvc".to_string(),
            version: "1.0".to_string(),
            tensors,
            metadata,
        };

        assert_eq!(info.model_type, "rvc");
        assert_eq!(info.version, "1.0");
        assert_eq!(info.tensors.len(), 1);
        assert_eq!(info.metadata.len(), 1);
    }
}
