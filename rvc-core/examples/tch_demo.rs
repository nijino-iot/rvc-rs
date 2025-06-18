//! 演示 tch 集成的测试程序
//!
//! 这个程序演示了如何使用真实的 PyTorch 绑定 (tch) 进行张量操作

use rvc_core::{Device, Kind, RvcResult, Tensor};

fn main() -> RvcResult<()> {
    println!("=== RVC-RS tch 集成演示 ===\n");

    // 检查 CUDA 可用性
    println!("1. 检查设备支持:");
    let cuda_available = rvc_core::Cuda::is_available();
    let cuda_device_count = rvc_core::Cuda::device_count();
    println!("   CUDA 可用: {}", cuda_available);
    println!("   CUDA 设备数量: {}", cuda_device_count);

    let device = if cuda_available {
        Device::Cuda(0)
    } else {
        Device::Cpu
    };
    println!("   使用设备: {:?}\n", device);

    // 基本张量操作
    println!("2. 基本张量操作:");
    let a = Tensor::from_slice(&[1.0, 2.0, 3.0, 4.0]);
    let b = Tensor::from_slice(&[5.0, 6.0, 7.0, 8.0]);

    println!("   张量 a: {:?}", a.size());
    println!("   张量 b: {:?}", b.size());

    let c = a.add(&b);
    println!("   a + b 结果形状: {:?}", c.size());

    let d = a.mul(&b);
    println!("   a * b 结果形状: {:?}", d.size());

    // 标量操作
    let e = a.mul_scalar(2.0);
    println!("   a * 2.0 结果形状: {:?}", e.size());

    // 数学函数
    let f = a.sin();
    println!("   sin(a) 结果形状: {:?}", f.size());

    let g = a.sqrt();
    println!("   sqrt(a) 结果形状: {:?}\n", g.size());

    // 矩阵操作
    println!("3. 矩阵操作:");
    let matrix_a = Tensor::randn(&[3, 4], (Kind::Float, device));
    let matrix_b = Tensor::randn(&[4, 5], (Kind::Float, device));

    println!("   矩阵 A 形状: {:?}", matrix_a.size());
    println!("   矩阵 B 形状: {:?}", matrix_b.size());

    let matrix_c = matrix_a.matmul(&matrix_b);
    println!("   A @ B 结果形状: {:?}", matrix_c.size());

    // 转置
    let matrix_t = matrix_a.transpose(0, 1);
    println!("   A 转置后形状: {:?}\n", matrix_t.size());

    // 形状操作
    println!("4. 形状操作:");
    let tensor = Tensor::randn(&[2, 3, 4], (Kind::Float, device));
    println!("   原始张量形状: {:?}", tensor.size());

    let reshaped = tensor.view(&[6, 4]);
    println!("   重塑后形状: {:?}", reshaped.size());

    let unsqueezed = tensor.unsqueeze(0);
    println!("   unsqueeze(0) 后形状: {:?}", unsqueezed.size());

    let narrowed = tensor.narrow(0, 0, 1);
    println!("   narrow(0, 0, 1) 后形状: {:?}\n", narrowed.size());

    // 聚合操作
    println!("5. 聚合操作:");
    let data = Tensor::randn(&[2, 3, 4], (Kind::Float, device));
    println!("   原始数据形状: {:?}", data.size());

    let sum_result = data.sum_dim_intlist(&[1], false, Some(Kind::Float));
    println!("   沿维度1求和后形状: {:?}", sum_result.size());

    let mean_all = data.sum_dim_intlist(&[], false, Some(Kind::Float));
    println!("   全局求和后形状: {:?}\n", mean_all.size());

    // FFT 操作
    println!("6. 频域操作:");
    let signal = Tensor::randn(&[16], (Kind::Float, device));
    println!("   信号形状: {:?}", signal.size());

    let fft_result = signal.fft_rfft(&[0], false);
    println!("   FFT 结果形状: {:?}\n", fft_result.size());

    // 激活函数
    println!("7. 激活函数:");
    let input = Tensor::randn(&[10], (Kind::Float, device));
    println!("   输入形状: {:?}", input.size());

    let relu_out = input.relu();
    println!("   ReLU 输出形状: {:?}", relu_out.size());

    let softmax_out = input.softmax(0, Kind::Float);
    println!("   Softmax 输出形状: {:?}\n", softmax_out.size());

    // 拼接操作
    println!("8. 拼接操作:");
    let tensor1 = Tensor::ones(&[2, 3], (Kind::Float, device));
    let tensor2 = Tensor::zeros(&[2, 3], (Kind::Float, device));

    println!("   张量1形状: {:?}", tensor1.size());
    println!("   张量2形状: {:?}", tensor2.size());

    let cat_result = Tensor::cat(&[&tensor1, &tensor2], 0);
    println!("   cat(dim=0) 结果形状: {:?}", cat_result.size());

    let stack_result = Tensor::stack(&[&tensor1, &tensor2], 0);
    println!("   stack(dim=0) 结果形状: {:?}\n", stack_result.size());

    // 梯度禁用演示
    println!("9. 无梯度计算:");
    let result = rvc_core::no_grad(|| {
        let x = Tensor::randn(&[5, 5], (Kind::Float, device));
        let y = Tensor::randn(&[5, 5], (Kind::Float, device));
        x.matmul(&y)
    });
    println!("   无梯度计算结果形状: {:?}\n", result.size());

    // 设备移动（如果有CUDA）
    if cuda_available {
        println!("10. 设备移动:");
        let cpu_tensor = Tensor::ones(&[3, 3], (Kind::Float, Device::Cpu));
        println!("    CPU 张量设备: {:?}", cpu_tensor.device());

        let gpu_tensor = cpu_tensor.to_device(Device::Cuda(0));
        println!("    GPU 张量设备: {:?}", gpu_tensor.device());

        let back_to_cpu = gpu_tensor.to_device(Device::Cpu);
        println!("    回到 CPU 张量设备: {:?}\n", back_to_cpu.device());
    }

    // 类型转换测试
    println!("11. 数据类型:");
    let float_tensor = Tensor::ones(&[5], (Kind::Float, device));
    let int_tensor = Tensor::ones(&[5], (Kind::Int64, device));

    println!("    Float 张量类型: {:?}", float_tensor.kind());
    println!("    Int64 张量类型: {:?}", int_tensor.kind());

    // 内存管理测试
    println!("12. 内存管理:");
    let original = Tensor::randn(&[1000, 1000], (Kind::Float, device));
    println!("    原始张量形状: {:?}", original.size());

    let shallow = original.shallow_clone();
    println!("    浅拷贝张量形状: {:?}", shallow.size());

    let deep = original.copy();
    println!("    深拷贝张量形状: {:?}", deep.size());

    let contiguous = original.contiguous();
    println!("    连续张量形状: {:?}\n", contiguous.size());

    println!("=== 所有测试完成！tch 集成正常工作 ===");

    Ok(())
}
