//! RVC 模型检查器命令行工具
//!
//! 用于检查和分析 RVC 模型文件的命令行程序
//! 支持检查 .pth 模型文件和 .index 索引文件

use rvc_core::{CheckpointInfo, CheckpointUtils, Device, RvcModelManager, RvcResult};
use std::env;
use std::path::Path;
use std::process;

fn main() -> RvcResult<()> {
    // 初始化日志
    env_logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "inspect" => {
            if args.len() < 3 {
                eprintln!("错误: 需要指定模型文件路径");
                print_usage(&args[0]);
                process::exit(1);
            }
            inspect_model(&args[2])
        }
        "compare" => {
            if args.len() < 4 {
                eprintln!("错误: 需要指定两个模型文件路径");
                print_usage(&args[0]);
                process::exit(1);
            }
            compare_models(&args[2], &args[3])
        }
        "batch" => {
            if args.len() < 3 {
                eprintln!("错误: 需要指定目录路径");
                print_usage(&args[0]);
                process::exit(1);
            }
            batch_inspect(&args[2])
        }
        "test-load" => {
            if args.len() < 3 {
                eprintln!("错误: 需要指定模型文件路径");
                print_usage(&args[0]);
                process::exit(1);
            }
            let index_path = if args.len() >= 4 {
                Some(args[3].as_str())
            } else {
                None
            };
            test_model_loading(&args[2], index_path)
        }
        "benchmark" => {
            if args.len() < 3 {
                eprintln!("错误: 需要指定模型文件路径");
                print_usage(&args[0]);
                process::exit(1);
            }
            benchmark_model(&args[2])
        }
        _ => {
            eprintln!("错误: 未知命令 '{}'", command);
            print_usage(&args[0]);
            process::exit(1);
        }
    }
}

/// 打印使用帮助
fn print_usage(program: &str) {
    println!("RVC 模型检查器");
    println!("");
    println!("用法:");
    println!(
        "  {} inspect <model.pth>                  - 检查单个模型文件",
        program
    );
    println!(
        "  {} compare <model1.pth> <model2.pth>   - 比较两个模型文件",
        program
    );
    println!(
        "  {} batch <directory>                    - 批量检查目录中的所有模型",
        program
    );
    println!(
        "  {} test-load <model.pth> [index.index] - 测试模型加载功能",
        program
    );
    println!(
        "  {} benchmark <model.pth>                - 性能基准测试",
        program
    );
    println!("");
    println!("示例:");
    println!("  {} inspect assets/weights/anbo.pth", program);
    println!(
        "  {} test-load assets/weights/anbo.pth logs/added_IVF3409_Flat_nprobe_1_anbo_v2.index",
        program
    );
    println!("  {} batch assets/weights/", program);
}

/// 检查单个模型文件
fn inspect_model(model_path: &str) -> RvcResult<()> {
    let path = Path::new(model_path);

    println!("=== 模型文件检查 ===");
    println!("文件路径: {}", path.display());

    // 检查文件是否存在
    if !path.exists() {
        eprintln!("错误: 文件不存在");
        process::exit(1);
    }

    // 获取文件基本信息
    let metadata = std::fs::metadata(path)
        .map_err(|e| rvc_core::RvcError::IoError(format!("获取文件信息失败: {}", e)))?;

    println!("文件大小: {:.2} MB", metadata.len() as f64 / 1_000_000.0);
    println!("修改时间: {:?}", metadata.modified().ok());

    // 检查是否为有效的 PyTorch 文件
    if CheckpointUtils::is_valid_checkpoint(path) {
        println!("✓ 有效的 PyTorch 检查点文件");

        // 获取详细信息
        match CheckpointUtils::get_checkpoint_info(path) {
            Ok(info) => {
                println!("");
                println!("=== 模型详细信息 ===");
                println!("{}", info);

                // 打印配置细节
                println!("");
                println!("=== 配置详情 ===");
                println!("模型版本: {}", info.version);
                println!("使用 F0: {}", if info.uses_f0 { "是" } else { "否" });
                println!("目标采样率: {} Hz", info.target_sr);
                println!("隐藏层维度: {}", info.hidden_dim);
                println!("权重参数数量: {}", info.num_weights);

                // 推断模型类型
                let model_type = infer_model_type(&info);
                println!("推断的模型类型: {}", model_type);

                // 内存使用估算
                let estimated_memory = estimate_memory_usage(&info);
                println!("预估内存使用: {:.1} MB", estimated_memory);
            }
            Err(e) => {
                eprintln!("⚠ 无法解析模型详细信息: {}", e);
            }
        }
    } else {
        println!("✗ 不是有效的 PyTorch 检查点文件");
    }

    // 检查相关文件
    check_related_files(path);

    Ok(())
}

/// 比较两个模型文件
fn compare_models(model1_path: &str, model2_path: &str) -> RvcResult<()> {
    let path1 = Path::new(model1_path);
    let path2 = Path::new(model2_path);

    println!("=== 模型对比分析 ===");
    println!("模型 1: {}", path1.display());
    println!("模型 2: {}", path2.display());

    // 检查文件存在性
    if !path1.exists() {
        eprintln!("错误: 模型 1 不存在");
        process::exit(1);
    }
    if !path2.exists() {
        eprintln!("错误: 模型 2 不存在");
        process::exit(1);
    }

    // 获取两个模型的信息
    let info1 = CheckpointUtils::get_checkpoint_info(path1)?;
    let info2 = CheckpointUtils::get_checkpoint_info(path2)?;

    println!("");
    println!("=== 基本信息对比 ===");
    println!("         │ 模型 1      │ 模型 2      │ 相同");
    println!("─────────┼─────────────┼─────────────┼─────");
    println!(
        "版本     │ {:^11} │ {:^11} │ {}",
        info1.version,
        info2.version,
        if info1.version == info2.version {
            "✓"
        } else {
            "✗"
        }
    );
    println!(
        "F0       │ {:^11} │ {:^11} │ {}",
        info1.uses_f0,
        info2.uses_f0,
        if info1.uses_f0 == info2.uses_f0 {
            "✓"
        } else {
            "✗"
        }
    );
    println!(
        "采样率   │ {:^11} │ {:^11} │ {}",
        info1.target_sr,
        info2.target_sr,
        if info1.target_sr == info2.target_sr {
            "✓"
        } else {
            "✗"
        }
    );
    println!(
        "隐藏维度 │ {:^11} │ {:^11} │ {}",
        info1.hidden_dim,
        info2.hidden_dim,
        if info1.hidden_dim == info2.hidden_dim {
            "✓"
        } else {
            "✗"
        }
    );
    println!(
        "权重数量 │ {:^11} │ {:^11} │ {}",
        info1.num_weights,
        info2.num_weights,
        if info1.num_weights == info2.num_weights {
            "✓"
        } else {
            "✗"
        }
    );
    println!(
        "文件大小 │ {:^9} MB │ {:^9} MB │ {}",
        info1.file_size / 1_000_000,
        info2.file_size / 1_000_000,
        if (info1.file_size as i64 - info2.file_size as i64).abs() < 1_000_000 {
            "≈"
        } else {
            "✗"
        }
    );

    // 兼容性检查
    println!("");
    println!("=== 兼容性分析 ===");
    if CheckpointUtils::are_compatible(&info1, &info2) {
        println!("✓ 两个模型兼容，可以互换使用");
    } else {
        println!("✗ 两个模型不兼容");

        // 详细分析不兼容的原因
        if info1.version != info2.version {
            println!("  - 版本不同: {} vs {}", info1.version, info2.version);
        }
        if info1.uses_f0 != info2.uses_f0 {
            println!("  - F0 使用不同: {} vs {}", info1.uses_f0, info2.uses_f0);
        }
        if info1.hidden_dim != info2.hidden_dim {
            println!(
                "  - 隐藏维度不同: {} vs {}",
                info1.hidden_dim, info2.hidden_dim
            );
        }
    }

    Ok(())
}

/// 批量检查目录中的模型
fn batch_inspect(directory: &str) -> RvcResult<()> {
    let dir_path = Path::new(directory);

    println!("=== 批量模型检查 ===");
    println!("目录: {}", dir_path.display());

    if !dir_path.exists() {
        eprintln!("错误: 目录不存在");
        process::exit(1);
    }

    if !dir_path.is_dir() {
        eprintln!("错误: 不是目录");
        process::exit(1);
    }

    // 查找所有 .pth 文件
    let mut model_files = Vec::new();
    for entry in std::fs::read_dir(dir_path)
        .map_err(|e| rvc_core::RvcError::IoError(format!("读取目录失败: {}", e)))?
    {
        let entry =
            entry.map_err(|e| rvc_core::RvcError::IoError(format!("读取条目失败: {}", e)))?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "pth") {
            model_files.push(path);
        }
    }

    if model_files.is_empty() {
        println!("目录中没有找到 .pth 文件");
        return Ok(());
    }

    println!("找到 {} 个模型文件", model_files.len());
    println!("");

    // 创建汇总表
    println!("=== 模型汇总 ===");
    println!(
        "{:<25} │ {:^8} │ {:^5} │ {:^8} │ {:^6} │ {:^8}",
        "文件名", "版本", "F0", "采样率", "维度", "大小(MB)"
    );
    println!(
        "{:─<25}─┼─{:─<8}─┼─{:─<5}─┼─{:─<8}─┼─{:─<6}─┼─{:─<8}",
        "", "", "", "", "", ""
    );

    let mut total_size = 0u64;
    let mut valid_models = 0;
    let mut version_counts = std::collections::HashMap::new();

    for model_file in &model_files {
        let filename = model_file.file_name().unwrap_or_default().to_string_lossy();

        let truncated_name = if filename.len() > 24 {
            format!("{}...", &filename[..21])
        } else {
            filename.to_string()
        };

        match CheckpointUtils::get_checkpoint_info(model_file) {
            Ok(info) => {
                println!(
                    "{:<25} │ {:^8} │ {:^5} │ {:^8} │ {:^6} │ {:^8.1}",
                    truncated_name,
                    info.version,
                    if info.uses_f0 { "是" } else { "否" },
                    info.target_sr,
                    info.hidden_dim,
                    info.file_size as f64 / 1_000_000.0
                );

                total_size += info.file_size;
                valid_models += 1;
                *version_counts.entry(info.version.clone()).or_insert(0) += 1;
            }
            Err(_) => {
                let file_size = std::fs::metadata(model_file).map(|m| m.len()).unwrap_or(0);

                println!(
                    "{:<25} │ {:^8} │ {:^5} │ {:^8} │ {:^6} │ {:^8.1}",
                    truncated_name,
                    "错误",
                    "-",
                    "-",
                    "-",
                    file_size as f64 / 1_000_000.0
                );

                total_size += file_size;
            }
        }
    }

    println!("");
    println!("=== 统计信息 ===");
    println!("总文件数: {}", model_files.len());
    println!("有效模型: {}", valid_models);
    println!("总大小: {:.1} MB", total_size as f64 / 1_000_000.0);

    if !version_counts.is_empty() {
        println!("版本分布:");
        for (version, count) in version_counts {
            println!("  {} 版本: {} 个", version, count);
        }
    }

    Ok(())
}

/// 测试模型加载功能
fn test_model_loading(model_path: &str, index_path: Option<&str>) -> RvcResult<()> {
    let pth_path = Path::new(model_path);

    println!("=== 模型加载测试 ===");
    println!("模型文件: {}", pth_path.display());

    if let Some(idx_path) = index_path {
        println!("索引文件: {}", idx_path);
    }

    // 检查文件存在性
    if !pth_path.exists() {
        eprintln!("错误: 模型文件不存在");
        process::exit(1);
    }

    // 选择设备
    let device = if tch::Cuda::is_available() {
        println!("使用 CUDA 设备");
        Device::Cuda(0)
    } else {
        println!("使用 CPU 设备");
        Device::Cpu
    };

    // 创建模型管理器
    let mut manager = RvcModelManager::new(device);

    println!("");
    println!("开始加载模型...");
    let start_time = std::time::Instant::now();

    // 尝试加载模型
    match manager.load_model(
        "test_model".to_string(),
        pth_path,
        index_path.map(Path::new),
        0,     // f0_up_key
        0.3,   // index_rate
        false, // is_half
    ) {
        Ok(()) => {
            let load_time = start_time.elapsed();
            println!("✓ 模型加载成功! 耗时: {:.2}秒", load_time.as_secs_f64());

            // 获取模型信息
            if let Some(model) = manager.get_model("test_model") {
                println!("");
                println!("=== 加载后的模型信息 ===");
                println!("版本: {:?}", model.version());
                println!("目标采样率: {} Hz", model.target_sample_rate());
                println!("使用 F0: {}", model.uses_f0());
                println!("设备: {:?}", model.device);

                // 检查各个组件的加载状态
                println!("");
                println!("=== 组件加载状态 ===");
                println!(
                    "HuBERT 模型: {}",
                    if model.hubert_model.is_some() {
                        "✓ 已加载"
                    } else {
                        "✗ 未加载"
                    }
                );
                println!(
                    "合成器模型: {}",
                    if model.net_g.is_some() {
                        "✓ 已加载"
                    } else {
                        "✗ 未加载"
                    }
                );
                println!(
                    "Faiss 索引: {}",
                    if model.faiss_index.is_some() {
                        "✓ 已加载"
                    } else {
                        "✗ 未加载"
                    }
                );

                // 测试推理功能
                test_inference(&mut manager)?;
            }
        }
        Err(e) => {
            let load_time = start_time.elapsed();
            eprintln!("✗ 模型加载失败! 耗时: {:.2}秒", load_time.as_secs_f64());
            eprintln!("错误: {}", e);
        }
    }

    Ok(())
}

/// 测试推理功能
fn test_inference(manager: &mut RvcModelManager) -> RvcResult<()> {
    println!("");
    println!("=== 推理功能测试 ===");

    // 创建测试音频 (1秒，16kHz)
    let sample_rate = 16000;
    let duration = 1.0;
    let num_samples = (sample_rate as f64 * duration) as i64;

    let input_audio = tch::Tensor::randn(&[num_samples], tch::kind::FLOAT_CPU);
    println!("测试音频: {} 采样点, {} Hz", num_samples, sample_rate);

    if let Some(model) = manager.get_model_mut("test_model") {
        let start_time = std::time::Instant::now();

        match model.infer(
            &input_audio,
            sample_rate, // block_frame_16k
            0,           // skip_head
            num_samples, // return_length
            "rmvpe",     // f0_method
        ) {
            Ok(output) => {
                let inference_time = start_time.elapsed();
                println!("✓ 推理成功!");
                println!("输出音频: {} 采样点", output.size()[0]);
                println!("推理耗时: {:.3}秒", inference_time.as_secs_f64());
                println!("实时率: {:.2}x", duration / inference_time.as_secs_f64());
            }
            Err(e) => {
                println!("✗ 推理失败: {}", e);
            }
        }
    }

    Ok(())
}

/// 性能基准测试
fn benchmark_model(model_path: &str) -> RvcResult<()> {
    println!("=== 性能基准测试 ===");
    println!("模型: {}", model_path);

    // TODO: 实现详细的性能基准测试
    // 包括不同音频长度、不同 F0 方法的性能对比

    println!("基准测试功能待实现...");

    Ok(())
}

/// 推断模型类型
fn infer_model_type(info: &CheckpointInfo) -> String {
    match (info.version.as_str(), info.hidden_dim) {
        ("v1", dim) if dim <= 256 => "SynthesizerTrnMs256NSFsid".to_string(),
        ("v1", _) => "SynthesizerTrnMs768NSFsid".to_string(),
        ("v2", dim) if dim <= 256 => "SynthesizerTrnMs256NSFsid_v2".to_string(),
        ("v2", _) => "SynthesizerTrnMs768NSFsid_v2".to_string(),
        _ => "Unknown".to_string(),
    }
}

/// 估算内存使用量
fn estimate_memory_usage(info: &CheckpointInfo) -> f64 {
    // 基于文件大小和模型参数估算运行时内存使用
    let base_memory = info.file_size as f64 / 1_000_000.0; // 模型权重
    let runtime_memory = base_memory * 1.5; // 运行时额外内存
    let feature_memory = 100.0; // HuBERT 特征缓存

    base_memory + runtime_memory + feature_memory
}

/// 检查相关文件
fn check_related_files(model_path: &Path) {
    println!("");
    println!("=== 相关文件检查 ===");

    let model_stem = model_path.file_stem().unwrap_or_default();
    let model_dir = model_path.parent().unwrap_or(Path::new("."));

    // 检查 TorchScript 文件
    let jit_path = model_dir.join(format!("{}.jit", model_stem.to_string_lossy()));
    let half_jit_path = model_dir.join(format!("{}.half.jit", model_stem.to_string_lossy()));

    println!(
        "TorchScript (.jit): {}",
        if jit_path.exists() {
            "✓ 存在"
        } else {
            "✗ 不存在"
        }
    );
    println!(
        "Half TorchScript (.half.jit): {}",
        if half_jit_path.exists() {
            "✓ 存在"
        } else {
            "✗ 不存在"
        }
    );

    // 检查索引文件
    let logs_dir = Path::new("logs");
    if logs_dir.exists() {
        let mut found_indices = Vec::new();

        if let Ok(entries) = std::fs::read_dir(logs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "index") {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy();
                    if filename.contains(&model_stem.to_string_lossy().as_ref()) {
                        found_indices.push(path);
                    }
                }
            }
        }

        if found_indices.is_empty() {
            println!("相关索引文件: ✗ 未找到");
        } else {
            println!("相关索引文件: ✓ 找到 {} 个", found_indices.len());
            for idx_path in found_indices {
                println!("  - {}", idx_path.display());
            }
        }
    }

    // 检查配置文件
    let config_file = model_dir.join("config.json");
    println!(
        "配置文件: {}",
        if config_file.exists() {
            "✓ 存在"
        } else {
            "✗ 不存在"
        }
    );
}
