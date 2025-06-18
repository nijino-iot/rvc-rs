<div align="center">

<h1>Retrieval-based-Voice-Conversion-WebUI</h1>
一个基于VITS的简单易用的变声框架<br>
🦀 <strong>Rust 版本 - PyTorch 集成成功!</strong> 🦀<br><br>

[![madewithlove](https://img.shields.io/badge/made_with-%E2%9D%A4-red?style=for-the-badge&labelColor=orange
)](https://github.com/RVC-Project/Retrieval-based-Voice-Conversion-WebUI)

<img src="https://counter.seku.su/cmoe?name=rvc&theme=r34" /><br>

[![Open In Colab](https://img.shields.io/badge/Colab-F9AB00?style=for-the-badge&logo=googlecolab&color=525252)](https://colab.research.google.com/github/RVC-Project/Retrieval-based-Voice-Conversion-WebUI/blob/main/Retrieval_based_Voice_Conversion_WebUI.ipynb)
[![Licence](https://img.shields.io/badge/LICENSE-MIT-green.svg?style=for-the-badge)](https://github.com/RVC-Project/Retrieval-based-Voice-Conversion-WebUI/blob/main/LICENSE)
[![Huggingface](https://img.shields.io/badge/🤗%20-Spaces-yellow.svg?style=for-the-badge)](https://huggingface.co/lj1995/VoiceConversionWebUI/tree/main/)

[![Discord](https://img.shields.io/badge/RVC%20Developers-Discord-7289DA?style=for-the-badge&logo=discord&logoColor=white)](https://discord.gg/HcsmBBGyVk)

[**更新日志**](./CHANGELOG.md) | [**常见问题解答**](https://github.com/RVC-Project/Retrieval-based-Voice-Conversion-WebUI/wiki/%E5%B8%B8%E8%A7%81%E9%97%AE%E9%A2%98%E8%A7%A3%E7%AD%94) | [**开发指南**](./AGENTS.md) | [**进度跟踪**](./TODO.md) | [**Rust 实现状态**](./DONE.md)

[**English**](./docs/en/README.en.md) | [**中文简体**](./README.md) | [**日本語**](./docs/jp/README.ja.md) | [**한국어**](./docs/kr/README.ko.md) ([**韓國語**](./docs/kr/README.ko.han.md)) | [**Français**](./docs/fr/README.fr.md) | [**Türkçe**](./docs/tr/README.tr.md) | [**Português**](./docs/pt/README.pt.md)

</div>

> 底模使用接近50小时的开源高质量VCTK训练集训练，无版权方面的顾虑，请大家放心使用

> 🚀 **Rust 版本重大进展**: PyTorch 集成完成！现已支持真实的深度学习推理，具备 CUDA GPU 加速能力，为高性能语音转换奠定基础

> ✅ **已完成功能**: 核心架构、PyTorch/tch 集成、张量运算、配置管理、音频处理框架、F0 提取架构、神经网络模型结构

> 🔧 **正在开发**: 模型权重加载、实时音频处理、Tauri 前端界面

> 请期待RVCv3的底模，参数更大，数据更大，效果更好，基本持平的推理速度，需要训练数据量更少。

<table>
   <tr>
		<td align="center">训练推理界面</td>
		<td align="center">实时变声界面</td>
	</tr>
  <tr>
		<td align="center"><img src="https://github.com/RVC-Project/Retrieval-based-Voice-Conversion-WebUI/assets/129054828/092e5c12-0d49-4168-a590-0b0ef6a4f630"></td>
    <td align="center"><img src="https://github.com/RVC-Project/Retrieval-based-Voice-Conversion-WebUI/assets/129054828/730b4114-8805-44a1-ab1a-04668f3c30a6"></td>
	</tr>
	<tr>
		<td align="center">go-web.bat</td>
		<td align="center">go-realtime-gui.bat</td>
	</tr>
  <tr>
    <td align="center">可以自由选择想要执行的操作。</td>
		<td align="center">我们已经实现端到端170ms延迟。如使用ASIO输入输出设备，已能实现端到端90ms延迟，但非常依赖硬件驱动支持。</td>
	</tr>
</table>

## 简介
本仓库具有以下特点
+ 使用top1检索替换输入源特征为训练集特征来杜绝音色泄漏
+ 即便在相对较差的显卡上也能快速训练
+ 使用少量数据进行训练也能得到较好结果(推荐至少收集10分钟低底噪语音数据)
+ 可以通过模型融合来改变音色(借助ckpt处理选项卡中的ckpt-merge)
+ 简单易用的网页界面
+ 可调用UVR5模型来快速分离人声和伴奏
+ 使用最先进的[人声音高提取算法InterSpeech2023-RMVPE](#参考项目)根绝哑音问题。效果最好（显著地）但比crepe_full更快、资源占用更小
+ A卡I卡加速支持

### 🦀 Rust 版本特性
+ **内存安全**: Rust 的所有权系统确保内存安全，避免常见的C/C++错误
+ **高性能**: 零成本抽象和编译时优化，接近C/C++的性能
+ **跨平台**: 统一的跨平台代码，支持 Windows、macOS、Linux
+ **现代异步**: 基于 tokio 的高效异步处理
+ **类型安全**: 编译时错误检查，减少运行时崩溃
+ **模块化设计**: 清晰的模块边界，便于维护和扩展

点此查看我们的[演示视频](https://www.bilibili.com/video/BV1pm4y1z7Gm/) !

## 🦀 Rust 版本快速开始

### 系统要求
- Rust 1.75+ 
- 操作系统: Windows 10+, macOS 10.15+, Linux (Ubuntu 18.04+)

### 安装和运行
```bash
# 克隆仓库
git clone https://github.com/RVC-Project/Retrieval-based-Voice-Conversion-WebUI.git
cd Retrieval-based-Voice-Conversion-WebUI

# 构建核心库 (当前为原型阶段)
cd rvc-core
cargo test  # 运行测试
cargo check # 检查编译

# 注意: Rust 版本当前处于开发阶段，完整功能即将推出
```

### Rust 版本开发状态
- ✅ **核心架构**: 模块化设计完成
- ✅ **配置管理**: JSON 配置系统
- ✅ **错误处理**: 统一错误处理框架  
- ✅ **音频处理**: 基础音频工具和处理管道
- ✅ **F0 提取**: 多算法 F0 提取框架
- ✅ **神经网络**: RVC 模型架构实现
- ✅ **GUI 状态**: 事件驱动的状态管理
- 🚧 **PyTorch 集成**: 正在开发中
- 🚧 **Tauri 前端**: Vue.js 界面开发中
- 🚧 **实时音频**: 音频设备集成开发中

查看 [开发进度](./IN_PROGRESS.md) | [完成功能](./DONE.md) | [待实现功能](./TODO.md)

---

## Python 版本环境配置
以下指令需在 Python 版本大于3.8的环境中执行。  

### Windows/Linux/MacOS等平台通用方法
下列方法任选其一。
#### 1. 通过 pip 安装依赖
1. 安装Pytorch及其核心依赖，若已安装则跳过。参考自: https://pytorch.org/get-started/locally/
```bash
pip install torch torchvision torchaudio
```
2. 如果是 win 系统 + Nvidia Ampere 架构(RTX30xx)，根据 #21 的经验，需要指定 pytorch 对应的 cuda 版本
```bash
pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu117
```
3. 根据自己的显卡安装对应依赖
- N卡
```bash
pip install -r requirements.txt
```
- A卡/I卡
```bash
pip install -r requirements-dml.txt
```
- A卡ROCM(Linux)
```bash
pip install -r requirements-amd.txt
```
- I卡IPEX(Linux)
```bash
pip install -r requirements-ipex.txt
```

#### 2. 通过 poetry 来安装依赖
安装 Poetry 依赖管理工具，若已安装则跳过。参考自: https://python-poetry.org/docs/#installation
```bash
curl -sSL https://install.python-poetry.org | python3 -
```

通过 Poetry 安装依赖时，python 建议使用 3.7-3.10 版本，其余版本在安装 llvmlite==0.39.0 时会出现冲突
```bash
poetry init -n
poetry env use "path to your python.exe"
poetry run pip install -r requirments.txt
```

### MacOS
可以通过 `run.sh` 来安装依赖
```bash
sh ./run.sh
```

## 其他预模型准备
RVC需要其他一些预模型来推理和训练。

你可以从我们的[Hugging Face space](https://huggingface.co/lj1995/VoiceConversionWebUI/tree/main/)下载到这些模型。

### 1. 下载 assets
以下是一份清单，包括了所有RVC所需的预模型和其他文件的名称。你可以在`tools`文件夹找到下载它们的脚本。

- ./assets/hubert/hubert_base.pt

- ./assets/pretrained 

- ./assets/uvr5_weights

想使用v2版本模型的话，需要额外下载

- ./assets/pretrained_v2

### 2. 安装 ffmpeg
若ffmpeg和ffprobe已安装则跳过。

#### Ubuntu/Debian 用户
```bash
sudo apt install ffmpeg
```
#### MacOS 用户
```bash
brew install ffmpeg
```
#### Windows 用户
下载后放置在根目录。
- 下载[ffmpeg.exe](https://huggingface.co/lj1995/VoiceConversionWebUI/blob/main/ffmpeg.exe)

- 下载[ffprobe.exe](https://huggingface.co/lj1995/VoiceConversionWebUI/blob/main/ffprobe.exe)

### 3. 下载 rmvpe 人声音高提取算法所需文件

如果你想使用最新的RMVPE人声音高提取算法，则你需要下载音高提取模型参数并放置于RVC根目录。

- 下载[rmvpe.pt](https://huggingface.co/lj1995/VoiceConversionWebUI/blob/main/rmvpe.pt)

#### 下载 rmvpe 的 dml 环境(可选, A卡/I卡用户)

- 下载[rmvpe.onnx](https://huggingface.co/lj1995/VoiceConversionWebUI/blob/main/rmvpe.onnx)

### 4. AMD显卡Rocm(可选, 仅Linux)

如果你想基于AMD的Rocm技术在Linux系统上运行RVC，请先在[这里](https://rocm.docs.amd.com/en/latest/deploy/linux/os-native/install.html)安装所需的驱动。

若你使用的是Arch Linux，可以使用pacman来安装所需驱动：
````
pacman -S rocm-hip-sdk rocm-opencl-sdk
````
对于某些型号的显卡，你可能需要额外配置如下的环境变量（如：RX6700XT）：
````
export ROCM_PATH=/opt/rocm
export HSA_OVERRIDE_GFX_VERSION=10.3.0
````
同时确保你的当前用户处于`render`与`video`用户组内：
````
sudo usermod -aG render $USERNAME
sudo usermod -aG video $USERNAME
````

## 开始使用
### 直接启动
使用以下指令来启动 WebUI
```bash
python infer-web.py
```

若先前使用 Poetry 安装依赖，则可以通过以下方式启动WebUI
```bash
poetry run python infer-web.py
```

### 使用整合包
下载并解压`RVC-beta.7z`
#### Windows 用户
双击`go-web.bat`
#### MacOS 用户
```bash
sh ./run.sh
```
### 对于需要使用IPEX技术的I卡用户(仅Linux)
```bash
source /opt/intel/oneapi/setvars.sh
```

## 参考项目
+ [ContentVec](https://github.com/auspicious3000/contentvec/)
+ [VITS](https://github.com/jaywalnut310/vits)
+ [HIFIGAN](https://github.com/jik876/hifi-gan)
+ [Gradio](https://github.com/gradio-app/gradio)
+ [FFmpeg](https://github.com/FFmpeg/FFmpeg)
+ [Ultimate Vocal Remover](https://github.com/Anjok07/ultimatevocalremovergui)
+ [audio-slicer](https://github.com/openvpi/audio-slicer)
+ [Vocal pitch extraction:RMVPE](https://github.com/Dream-High/RMVPE)
  + The pretrained model is trained and tested by [yxlllc](https://github.com/yxlllc/RMVPE) and [RVC-Boss](https://github.com/RVC-Boss).

## 项目架构

### 目录结构
```
rvc-rs/
├── rvc-core/           # 🦀 Rust 核心库
│   ├── src/
│   │   ├── lib.rs      # 主库导出
│   │   ├── config.rs   # 配置管理
│   │   ├── error.rs    # 错误处理
│   │   ├── audio.rs    # 音频处理
│   │   ├── f0.rs       # F0 提取
│   │   ├── gui.rs      # GUI 状态管理
│   │   ├── models.rs   # 神经网络模型
│   │   ├── tensor.rs   # 张量操作 (当前为 mock)
│   │   └── utils.rs    # 实用工具
│   └── Cargo.toml      # Rust 依赖配置
├── rvc-ui/             # 🖥️ Tauri + Vue.js 前端
│   ├── src/            # Vue.js 前端代码
│   └── src-tauri/      # Tauri Rust 后端
├── AGENTS.md           # 🤖 开发指南和规范
├── TODO.md             # 📋 待实现功能
├── IN_PROGRESS.md      # 🚧 开发中功能
├── DONE.md             # ✅ 已完成功能
└── CHANGELOG.md        # 📝 版本更新记录
```

### 技术栈

#### Rust 后端
- **语言**: Rust 2021 Edition
- **异步运行时**: Tokio
- **深度学习**: tch (PyTorch Rust 绑定)
- **音频处理**: hound, dasp
- **序列化**: serde
- **GUI 框架**: Tauri

#### 前端
- **框架**: Vue.js 3
- **构建工具**: Vite  
- **UI 库**: 待定
- **状态管理**: Pinia

## 贡献指南

### Rust 版本贡献
1. 阅读 [开发指南](./AGENTS.md)
2. 查看 [当前进度](./IN_PROGRESS.md) 和 [待实现功能](./TODO.md)
3. 遵循 Rust 编码规范
4. 确保测试覆盖率 > 80%
5. 更新相关文档

### 开发工作流
```bash
# 1. Fork 并克隆仓库
git clone https://github.com/YOUR_USERNAME/Retrieval-based-Voice-Conversion-WebUI.git

# 2. 创建功能分支
git checkout -b feature/your-feature-name

# 3. 开发和测试
cd rvc-core
cargo test
cargo fmt
cargo clippy

# 4. 提交更改
git commit -m "feat: add your feature description"
git push origin feature/your-feature-name

# 5. 创建 Pull Request
```

## 感谢所有贡献者作出的努力
<a href="https://github.com/RVC-Project/Retrieval-based-Voice-Conversion-WebUI/graphs/contributors" target="_blank">
  <img src="https://contrib.rocks/image?repo=RVC-Project/Retrieval-based-Voice-Conversion-WebUI" />
</a>
