//! RVC Rust 核心功能演示
//!
//! 这个演示程序展示了 RVC Rust 版本中已经实现的核心功能，
//! 包括配置管理、音频处理、F0 提取、模型管理和 GUI 状态管理。

use rvc_core::*;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> RvcResult<()> {
    // 初始化 RVC 核心库
    init()?;

    println!("🦀 RVC Rust 核心功能演示");
    println!("========================================");

    // 1. 配置管理演示
    demo_config_management().await?;

    // 2. 音频处理演示
    demo_audio_processing()?;

    // 3. F0 提取演示
    demo_f0_extraction().await?;

    // 4. 模型管理演示
    demo_model_management()?;

    // 5. GUI 状态管理演示
    demo_gui_management().await?;

    // 6. 实用工具演示
    demo_utilities()?;

    println!("\n✅ 所有功能演示完成！");
    println!("📋 查看 TODO.md 了解待实现功能");
    println!("🚧 查看 IN_PROGRESS.md 了解开发进度");
    println!("✅ 查看 DONE.md 了解已完成功能");

    Ok(())
}

/// 配置管理功能演示
async fn demo_config_management() -> RvcResult<()> {
    println!("\n1️⃣ 配置管理演示");
    println!("------------------");

    // 创建临时配置文件路径
    let config_path = PathBuf::from("/tmp/rvc_demo_config.json");

    // 创建配置管理器
    let mut config_manager = ConfigManager::new(config_path.clone());

    // 加载默认配置
    config_manager.load()?;
    println!("✓ 加载默认配置");

    // 修改配置
    config_manager.update_config(|config| {
        config.pitch = 5;
        config.threshold = -45.0;
        config.set_f0method("harvest".to_string());
    })?;
    println!("✓ 更新配置: pitch=5, threshold=-45.0, f0method=harvest");

    // 验证配置
    let config = config_manager.config();
    println!("✓ 当前配置验证通过");
    println!("  - 音调偏移: {}", config.pitch);
    println!("  - 响应阈值: {} dB", config.threshold);
    println!("  - F0 方法: {}", config.f0method);
    println!("  - 采样长度: {} 秒", config.block_time);

    Ok(())
}

/// 音频处理功能演示
fn demo_audio_processing() -> RvcResult<()> {
    println!("\n2️⃣ 音频处理演示");
    println!("------------------");

    // 创建模拟音频数据
    let sample_rate = 44100;
    let duration = 1.0; // 1秒
    let samples = (sample_rate as f32 * duration) as usize;

    // 生成 440Hz 正弦波（A4 音符）
    let mut audio_data = Vec::with_capacity(samples);
    for i in 0..samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
        audio_data.push(sample);
    }
    println!("✓ 生成 440Hz 测试音频 ({} 样本)", samples);

    // 音频格式转换
    let i16_data = audio_utils::f32_to_i16(&audio_data);
    let converted_back = audio_utils::i16_to_f32(&i16_data);
    println!("✓ 音频格式转换 (f32 ↔ i16)");

    // 计算 RMS 和分贝值
    let rms = audio_utils::calculate_rms(&audio_data);
    let db = audio_utils::linear_to_db(rms);
    println!("✓ 音频分析: RMS={:.4}, {}dB", rms, db);

    // 音频归一化
    let mut normalized_audio = audio_data.clone();
    audio_utils::normalize_audio(&mut normalized_audio);
    println!("✓ 音频归一化完成");

    // 重采样演示
    let resampled = audio_utils::resample_linear(&audio_data, sample_rate, 22050);
    println!(
        "✓ 重采样: {}Hz → 22050Hz ({} → {} 样本)",
        sample_rate,
        audio_data.len(),
        resampled.len()
    );

    // 窗口函数演示
    let device = Device::Cpu;
    let window_size = 1024i64;
    let hann_window = windows::hann_window(window_size, device);
    println!("✓ 生成 Hann 窗口 (大小: {})", window_size);

    Ok(())
}

/// F0 提取功能演示
async fn demo_f0_extraction() -> RvcResult<()> {
    println!("\n3️⃣ F0 提取演示");
    println!("---------------");

    // 创建测试音频数据
    let sample_rate = 16000;
    let duration = 0.5; // 0.5秒
    let samples = (sample_rate as f32 * duration) as usize;

    // 生成包含多个频率的复合信号
    let mut audio_data = Vec::with_capacity(samples);
    for i in 0..samples {
        let t = i as f32 / sample_rate as f32;
        // 基频 220Hz + 谐波
        let fundamental = (2.0 * std::f32::consts::PI * 220.0 * t).sin();
        let harmonic = 0.3 * (2.0 * std::f32::consts::PI * 440.0 * t).sin();
        audio_data.push((fundamental + harmonic) * 0.5);
    }
    println!("✓ 生成测试音频 (220Hz 基频)");

    // PM 方法演示
    println!("\n📍 PM F0 提取方法:");
    let pm_extractor = F0ExtractorFactory::create(F0Method::Pm, None)?;
    let f0_pm = pm_extractor.extract(&audio_data, sample_rate)?;

    // 分析 F0 结果
    let valid_f0: Vec<f32> = f0_pm.iter().filter(|&&f| f > 0.0).cloned().collect();
    if !valid_f0.is_empty() {
        let avg_f0 = valid_f0.iter().sum::<f32>() / valid_f0.len() as f32;
        println!("  - 检测到 F0 帧数: {}/{}", valid_f0.len(), f0_pm.len());
        println!("  - 平均基频: {:.2} Hz", avg_f0);
        println!(
            "  - F0 范围: {:.2} - {:.2} Hz",
            valid_f0.iter().fold(f32::INFINITY, |a, &b| a.min(b)),
            valid_f0.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b))
        );
    } else {
        println!("  - 未检测到有效 F0");
    }

    // Harvest 方法演示
    println!("\n📍 Harvest F0 提取方法:");
    let harvest_extractor = F0ExtractorFactory::create(F0Method::Harvest, Some(2))?;
    let f0_harvest = harvest_extractor.extract(&audio_data, sample_rate)?;

    let valid_f0_harvest: Vec<f32> = f0_harvest.iter().filter(|&&f| f > 0.0).cloned().collect();
    if !valid_f0_harvest.is_empty() {
        let avg_f0 = valid_f0_harvest.iter().sum::<f32>() / valid_f0_harvest.len() as f32;
        println!(
            "  - 检测到 F0 帧数: {}/{}",
            valid_f0_harvest.len(),
            f0_harvest.len()
        );
        println!("  - 平均基频: {:.2} Hz", avg_f0);
    } else {
        println!("  - 未检测到有效 F0 (Harvest 为简化实现)");
    }

    println!("✓ F0 提取框架演示完成");

    Ok(())
}

/// 模型管理功能演示
fn demo_model_management() -> RvcResult<()> {
    println!("\n4️⃣ 模型管理演示");
    println!("------------------");

    // 创建模型配置
    let mut model_config = ModelConfig::default();
    model_config.sample_rate = 22050;
    model_config.hidden_dim = 256;
    model_config.num_speakers = 1;
    println!("✓ 创建模型配置:");
    println!("  - 采样率: {} Hz", model_config.sample_rate);
    println!("  - 隐藏层维度: {}", model_config.hidden_dim);
    println!("  - 说话人数量: {}", model_config.num_speakers);

    // 创建 RVC 模型
    let device = Device::Cpu;
    let model = RvcModel::new(model_config.clone(), device)?;
    println!("✓ 创建 RVC 模型 (设备: {:?})", model.device());

    // 模型前向传播演示
    let batch_size = 1i64;
    let seq_len = 100i64;
    let input_dim = model_config.input_dim;

    let input_features = Tensor::randn(&[batch_size, seq_len, input_dim], (Kind::Float, device));
    println!(
        "✓ 生成输入特征 [{}, {}, {}]",
        batch_size, seq_len, input_dim
    );

    let output = model.inference(&input_features, Some(0))?;
    println!("✓ 模型推理完成，输出形状: {:?}", output.size());

    // 模型管理器演示
    let mut model_manager = ModelManager::new(device);
    println!("✓ 创建模型管理器");

    // 模拟模型加载（实际文件操作在真实环境中进行）
    println!("📁 模型管理器功能:");
    println!("  - 支持多模型管理");
    println!("  - 支持模型热切换");
    println!("  - 支持配置驱动加载");

    Ok(())
}

/// GUI 状态管理功能演示
async fn demo_gui_management() -> RvcResult<()> {
    println!("\n5️⃣ GUI 状态管理演示");
    println!("---------------------");

    // 创建 GUI 管理器
    let config_path = PathBuf::from("/tmp/rvc_gui_demo.json");
    let mut gui_manager = GuiManager::new(config_path)?;

    // 初始化 GUI 管理器
    gui_manager.initialize().await?;
    println!("✓ GUI 管理器初始化完成");

    // 获取初始状态
    let initial_state = gui_manager.get_state();
    println!("✓ 当前应用状态: {:?}", initial_state);

    // 获取音频设备列表
    let devices = gui_manager.get_audio_devices();
    println!("✓ 检测到 {} 个音频设备:", devices.len());
    for device in &devices {
        let io_type = if device.is_input && device.is_output {
            "输入/输出"
        } else if device.is_input {
            "输入"
        } else {
            "输出"
        };
        println!("  - {}: {} ({})", device.name, io_type, device.hostapi);
    }

    // 获取主机 API 列表
    let hostapis = gui_manager.get_hostapis();
    println!("✓ 支持的音频 API: {:?}", hostapis);

    // 演示事件处理
    println!("\n📡 事件系统演示:");

    // 模拟模型加载事件
    gui_manager
        .load_model("demo_model.pth".to_string(), "demo_index.index".to_string())
        .await?;

    // 等待一段时间让异步事件处理
    tokio::time::sleep(Duration::from_millis(100)).await;

    // 模拟开始语音转换
    gui_manager.start_voice_conversion().await?;
    tokio::time::sleep(Duration::from_millis(50)).await;

    // 检查状态更新
    let current_state = gui_manager.get_state();
    println!("✓ 状态更新: {:?}", current_state);

    // 获取实时统计
    let stats = gui_manager.get_stats();
    println!("✓ 实时统计:");
    println!("  - 算法延迟: {:.2} ms", stats.algorithm_latency_ms);
    println!("  - 推理时间: {:.2} ms", stats.inference_time_ms);
    println!("  - CPU 使用率: {:.1}%", stats.cpu_usage);
    if let Some(gpu_usage) = stats.gpu_usage {
        println!("  - GPU 使用率: {:.1}%", gpu_usage);
    }

    // 停止转换
    gui_manager.stop_voice_conversion().await?;
    tokio::time::sleep(Duration::from_millis(50)).await;

    println!("✓ GUI 状态管理演示完成");

    Ok(())
}

/// 实用工具功能演示
fn demo_utilities() -> RvcResult<()> {
    println!("\n6️⃣ 实用工具演示");
    println!("------------------");

    // 计时器演示
    println!("\n⏱️ 计时器功能:");
    let mut timer = Timer::new("演示任务");
    std::thread::sleep(Duration::from_millis(50));
    timer.print_elapsed();

    // 数学工具演示
    println!("\n🧮 数学工具:");
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![1.0, 2.0, 3.0];
    let similarity = math_utils::cosine_similarity(&a, &b);
    println!("✓ 余弦相似度: {:.4}", similarity);

    let value = math_utils::map_range(0.5, 0.0, 1.0, -10.0, 10.0);
    println!("✓ 范围映射: 0.5 → {}", value);

    let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 4.0, 3.0, 2.0, 1.0];
    let moving_avg = math_utils::moving_average(&data, 3);
    println!(
        "✓ 移动平均计算完成 ({} → {} 点)",
        data.len(),
        moving_avg.len()
    );

    // 文件系统工具演示
    println!("\n📁 文件系统工具:");
    let test_path = "/path/to/test.wav";
    let is_audio = fs_utils::is_audio_file(test_path);
    let extension = fs_utils::get_file_extension(test_path);
    println!("✓ 文件分析: {} 是音频文件: {}", test_path, is_audio);
    println!("✓ 文件扩展名: {:?}", extension);

    let file_size = 1024 * 1024 * 5; // 5MB
    let formatted_size = fs_utils::format_file_size(file_size);
    println!("✓ 文件大小格式化: {} bytes → {}", file_size, formatted_size);

    // 验证工具演示
    println!("\n✅ 参数验证:");
    let pitch_result = validation::validate_pitch_shift(12);
    let prob_result = validation::validate_probability(0.85, "test_param");
    println!("✓ 音调验证 (12): {:?}", pitch_result.is_ok());
    println!("✓ 概率验证 (0.85): {:?}", prob_result.is_ok());

    // 性能监控演示
    println!("\n📊 性能监控:");
    let mut monitor = PerformanceMonitor::new();
    monitor.start_timer("test_operation");
    std::thread::sleep(Duration::from_millis(10));
    let elapsed = monitor.end_timer("test_operation");
    monitor.increment_counter("operations");
    monitor.set_counter("processed_samples", 44100);

    if let Some(time) = elapsed {
        println!("✓ 操作耗时: {:.2} ms", time);
    }
    println!("✓ 操作计数: {}", monitor.get_counter("operations"));
    println!("✓ 处理样本: {}", monitor.get_counter("processed_samples"));

    Ok(())
}

/// 张量操作演示
#[allow(dead_code)]
fn demo_tensor_operations() -> RvcResult<()> {
    println!("\n🔢 张量操作演示");
    println!("------------------");

    let device = Device::Cpu;

    // 创建张量
    let a = Tensor::randn(&[2, 3], (Kind::Float, device));
    let b = Tensor::ones(&[2, 3], (Kind::Float, device));
    println!("✓ 创建张量 A: {:?}, B: {:?}", a.size(), b.size());

    // 基础运算
    let c = a.add(&b);
    let d = a.mul(&b);
    println!("✓ 张量加法和乘法完成");

    // 形状变换
    let reshaped = c.view(&[6]);
    let transposed = d.transpose(0, 1);
    println!("✓ 形状变换: {:?} → {:?}", c.size(), reshaped.size());
    println!("✓ 转置操作: {:?} → {:?}", d.size(), transposed.size());

    // 数学函数
    let e = a.sin();
    let f = b.sqrt();
    println!("✓ 数学函数计算完成");

    Ok(())
}
