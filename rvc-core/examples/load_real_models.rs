//! 实际模型加载示例
//!
//! 演示如何加载真实的 RVC 模型文件：
//! - assets/weights/anbo.pth (PyTorch 模型权重)
//! - logs/added_IVF3409_Flat_nprobe_1_anbo_v2.index (Faiss 索引文件)

use rvc_core::{Device, RvcModelManager, RvcRealtimeModel, RvcResult};
use std::path::Path;

fn main() -> RvcResult<()> {
    // 初始化日志
    env_logger::init();

    println!("=== RVC 实际模型加载示例 ===");

    // 检查模型文件是否存在
    let pth_path = Path::new("assets/weights/anbo.pth");
    let index_path = Path::new("logs/added_IVF3409_Flat_nprobe_1_anbo_v2.index");

    if !pth_path.exists() {
        eprintln!("错误: 模型文件不存在: {}", pth_path.display());
        eprintln!("请确保 anbo.pth 文件存在于 assets/weights/ 目录中");
        return Ok(());
    }

    if !index_path.exists() {
        eprintln!("警告: 索引文件不存在: {}", index_path.display());
        eprintln!("将在没有索引增强的情况下运行");
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
    let mut model_manager = RvcModelManager::new(device);

    println!("\n1. 加载 RVC 模型...");
    println!("   模型文件: {}", pth_path.display());
    if index_path.exists() {
        println!("   索引文件: {}", index_path.display());
    }

    // 模型参数
    let model_name = "anbo".to_string();
    let f0_up_key = 0; // 音调调整 (半音数)
    let index_rate = if index_path.exists() { 0.3 } else { 0.0 }; // 索引混合率
    let is_half = false; // 是否使用半精度 (FP16)

    // 加载模型
    match model_manager.load_model(
        model_name.clone(),
        pth_path,
        if index_path.exists() {
            Some(index_path)
        } else {
            None
        },
        f0_up_key,
        index_rate,
        is_half,
    ) {
        Ok(()) => {
            println!("✓ 模型加载成功!");
        }
        Err(e) => {
            eprintln!("✗ 模型加载失败: {}", e);
            return Err(e);
        }
    }

    // 获取模型信息
    if let Some(model) = model_manager.get_model(&model_name) {
        println!("\n2. 模型信息:");
        println!("   版本: {:?}", model.version());
        println!("   目标采样率: {} Hz", model.target_sample_rate());
        println!("   使用 F0: {}", model.uses_f0());
        println!("   F0 调整: {} 半音", model.f0_up_key);
        println!("   索引混合率: {:.1}%", model.index_rate * 100.0);
        println!("   使用半精度: {}", model.is_half);
        println!("   设备: {:?}", model.device);

        if model.faiss_index.is_some() {
            println!("   ✓ Faiss 索引已加载 (特征检索增强启用)");
        } else {
            println!("   ✗ 未加载 Faiss 索引");
        }

        if model.hubert_model.is_some() {
            println!("   ✓ HuBERT 特征提取模型已加载");
        } else {
            println!("   ✗ HuBERT 模型未加载");
        }

        if model.net_g.is_some() {
            println!("   ✓ 神经声码器已加载");
        } else {
            println!("   ✗ 神经声码器未加载");
        }
    }

    println!("\n3. 模拟音频推理...");

    // 创建模拟音频输入 (16kHz, 1 秒)
    let sample_rate = 16000;
    let duration_seconds = 1.0;
    let num_samples = (sample_rate as f64 * duration_seconds) as i64;

    let input_audio = tch::Tensor::randn(&[num_samples], tch::kind::FLOAT_CPU);
    println!("   输入音频: {} 采样点, {} Hz", num_samples, sample_rate);

    // 推理参数
    let block_frame_16k = sample_rate; // 块大小 (1 秒)
    let skip_head = 0; // 跳过的头部样本数
    let return_length = num_samples; // 返回的音频长度
    let f0_method = "rmvpe"; // F0 提取方法

    // 执行推理
    if let Some(model) = model_manager.get_model_mut(&model_name) {
        match model.infer(
            &input_audio,
            block_frame_16k,
            skip_head,
            return_length,
            f0_method,
        ) {
            Ok(output_audio) => {
                println!("   ✓ 推理成功!");
                println!("   输出音频: {} 采样点", output_audio.size()[0]);

                // 可以在这里保存输出音频文件
                // save_audio(&output_audio, "output.wav", model.target_sample_rate())?;
            }
            Err(e) => {
                eprintln!("   ✗ 推理失败: {}", e);
            }
        }
    }

    println!("\n4. 动态调整模型参数...");

    if let Some(model) = model_manager.get_model_mut(&model_name) {
        // 调整音调
        let new_key = 5; // 升高 5 个半音
        model.change_key(new_key);
        println!("   音调调整: {} 半音", new_key);

        // 调整索引混合率
        let new_index_rate = 0.5;
        model.change_index_rate(new_index_rate);
        println!("   索引混合率: {:.1}%", new_index_rate * 100.0);
    }

    println!("\n5. 模型管理操作...");

    // 列出所有模型
    let models = model_manager.list_models();
    println!("   已加载的模型: {:?}", models);

    println!("\n=== 示例完成 ===");
    println!("注意: 这是一个演示程序，实际的 PyTorch 模型加载和推理");
    println!("      需要完整的模型架构实现。当前使用模拟数据进行演示。");

    Ok(())
}

/// 保存音频文件 (示例函数)
#[allow(dead_code)]
fn save_audio(audio: &tch::Tensor, filename: &str, sample_rate: i64) -> RvcResult<()> {
    use std::fs::File;

    println!("保存音频到: {} (采样率: {} Hz)", filename, sample_rate);

    // 转换张量为 Vec<f32>
    let audio_data: Vec<f32> = audio.try_into().map_err(|e| {
        rvc_core::RvcError::AudioError(format!("Failed to convert tensor: {:?}", e))
    })?;

    // 使用 hound 库保存 WAV 文件
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate as u32,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(filename, spec).map_err(|e| {
        rvc_core::RvcError::AudioError(format!("Failed to create WAV writer: {}", e))
    })?;

    for sample in audio_data {
        writer.write_sample(sample).map_err(|e| {
            rvc_core::RvcError::AudioError(format!("Failed to write sample: {}", e))
        })?;
    }

    writer.finalize().map_err(|e| {
        rvc_core::RvcError::AudioError(format!("Failed to finalize WAV file: {}", e))
    })?;

    println!("✓ 音频保存成功");
    Ok(())
}

/// 演示不同 F0 提取方法的性能对比
#[allow(dead_code)]
fn benchmark_f0_methods(model: &mut RvcRealtimeModel, input_audio: &tch::Tensor) {
    println!("\n=== F0 提取方法性能对比 ===");

    let methods = ["harvest", "crepe", "rmvpe", "fcpe"];
    let block_frame_16k = 16000;
    let skip_head = 0;
    let return_length = input_audio.size()[0];

    for method in &methods {
        println!("测试方法: {}", method);

        let start_time = std::time::Instant::now();

        match model.infer(
            input_audio,
            block_frame_16k,
            skip_head,
            return_length,
            method,
        ) {
            Ok(_) => {
                let duration = start_time.elapsed();
                println!("  ✓ 耗时: {:.2} ms", duration.as_millis());
            }
            Err(e) => {
                println!("  ✗ 失败: {}", e);
            }
        }
    }
}

/// 演示批量模型加载
#[allow(dead_code)]
fn demo_batch_loading() -> RvcResult<()> {
    println!("\n=== 批量模型加载演示 ===");

    let device = Device::Cpu;
    let mut manager = RvcModelManager::new(device);

    // 假设有多个模型文件
    let model_configs = vec![
        (
            "anbo",
            "assets/weights/anbo.pth",
            Some("logs/added_IVF3409_Flat_nprobe_1_anbo_v2.index"),
        ),
        // 可以添加更多模型...
    ];

    for (name, pth_path, index_path) in model_configs {
        println!("加载模型: {}", name);

        let pth = Path::new(pth_path);
        let index = index_path.map(Path::new);

        if pth.exists() {
            match manager.load_model(
                name.to_string(),
                pth,
                index,
                0,     // f0_up_key
                0.3,   // index_rate
                false, // is_half
            ) {
                Ok(()) => println!("  ✓ {} 加载成功", name),
                Err(e) => println!("  ✗ {} 加载失败: {}", name, e),
            }
        } else {
            println!("  ✗ {} 文件不存在: {}", name, pth.display());
        }
    }

    println!("总共加载了 {} 个模型", manager.list_models().len());
    Ok(())
}
