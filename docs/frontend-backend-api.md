# RVC 前后端 API 文档

本文档描述了 RVC 应用中 Vue 前端与 Tauri/Rust 后端之间的通信接口。

## 架构概述

```
┌─────────────────┐     Tauri IPC     ┌──────────────────┐
│                 │ ←───────────────→ │                  │
│   Vue Frontend  │                   │  Tauri Backend   │
│   (rvc-ui)      │                   │  (Thin Wrapper)  │
│                 │                   │                  │
└─────────────────┘                   └────────┬─────────┘
                                               │
                                               │ Delegates to
                                               ↓
                                      ┌──────────────────┐
                                      │                  │
                                      │    RVC Core      │
                                      │ (Business Logic) │
                                      │                  │
                                      └──────────────────┘
```

## API 命令列表

### 1. 配置管理

#### `load_config`
加载应用配置文件。

**参数**: 无

**返回**: `Config` 对象
```typescript
interface Config {
  pth_path: string;
  index_path: string;
  sg_hostapi: string;
  sg_wasapi_exclusive: boolean;
  sg_input_device: string;
  sg_output_device: string;
  sr_type: string;
  threshold: number;
  pitch: number;
  formant: number;
  index_rate: number;
  rms_mix_rate: number;
  block_time: number;
  crossfade_length: number;
  extra_time: number;
  n_cpu: number;
  f0method: string;
  use_jit: boolean;
  use_pv: boolean;
  // 派生布尔值
  sr_model: boolean;
  sr_device: boolean;
  pm: boolean;
  harvest: boolean;
  crepe: boolean;
  rmvpe: boolean;
  fcpe: boolean;
}
```

**示例**:
```javascript
const config = await invoke('load_config');
```

#### `save_config`
保存配置到文件。

**参数**: 
- `config: Config` - 要保存的配置对象

**返回**: `void`

**示例**:
```javascript
await invoke('save_config', { config: formData });
```

### 2. 设备管理

#### `list_host_apis`
获取可用的音频主机 API 列表。

**参数**: 无

**返回**: `string[]` - 主机 API 名称数组

**示例**:
```javascript
const hostApis = await invoke('list_host_apis');
// 返回: ["DirectSound", "WASAPI", "ASIO", "MME"]
```

#### `list_input_devices`
获取输入设备列表。

**参数**:
- `host_api?: string` - 可选，按主机 API 过滤

**返回**: `AudioDeviceInfo[]`
```typescript
interface AudioDeviceInfo {
  id: string;
  index: number;
  name: string;
  hostapi: string;
  max_input_channels: number;
  max_output_channels: number;
  default_sample_rate: number;
  is_input: boolean;
  is_output: boolean;
}
```

**示例**:
```javascript
const inputDevices = await invoke('list_input_devices', { 
  hostApi: "WASAPI" 
});
```

#### `list_output_devices`
获取输出设备列表。

**参数**:
- `host_api?: string` - 可选，按主机 API 过滤

**返回**: `AudioDeviceInfo[]`

**示例**:
```javascript
const outputDevices = await invoke('list_output_devices');
```

#### `reload_devices`
重新加载设备列表。

**参数**:
- `host_api?: string` - 可选，按主机 API 过滤

**返回**: `void`

**示例**:
```javascript
await invoke('reload_devices', { hostApi: "WASAPI" });
```

#### `get_device_samplerate`
获取指定设备的采样率。

**参数**:
- `device_name: string` - 设备名称

**返回**: `number` - 采样率 (Hz)

**示例**:
```javascript
const sampleRate = await invoke('get_device_samplerate', { 
  deviceName: "Microphone (USB Audio)" 
});
```

#### `get_device_channels`
获取指定设备的通道数。

**参数**:
- `device_name: string` - 设备名称
- `is_input: boolean` - 是否为输入设备

**返回**: `number` - 通道数

**示例**:
```javascript
const channels = await invoke('get_device_channels', { 
  deviceName: "Speakers",
  isInput: false 
});
```

### 3. 语音转换控制

#### `start_voice_conversion`
开始实时语音转换。

**参数**: 无

**返回**: `void`

**错误**:
- 配置验证失败时返回错误信息
- 已在转换中时返回错误

**示例**:
```javascript
try {
  await invoke('start_voice_conversion');
} catch (error) {
  alert(`开始转换失败: ${error}`);
}
```

#### `stop_voice_conversion`
停止语音转换。

**参数**: 无

**返回**: `void`

**示例**:
```javascript
await invoke('stop_voice_conversion');
```

### 4. 参数更新

#### `update_parameter`
实时更新参数值。

**参数**:
- `name: string` - 参数名称
- `value: any` - 参数值

**支持的参数**:
- `pitch` (number): 音调 (-16 到 16)
- `formant` (number): 性别因子 (-2.0 到 2.0)
- `index_rate` (number): Index Rate (0.0 到 1.0)
- `rms_mix_rate` (number): 响度因子 (0.0 到 1.0)
- `threshold` (number): 响应阈值 (-60 到 0)
- `f0method` (string): F0 提取方法 ("pm", "harvest", "crepe", "rmvpe", "fcpe")
- `sr_type` (string): 采样率类型 ("sr_model", "sr_device")
- `block_time` (number): 采样长度 (0.02 到 1.5)
- `crossfade_time` (number): 淡入淡出长度 (0.01 到 0.15)
- `extra_time` (number): 额外推理时长 (0.05 到 5.0)
- `n_cpu` (number): CPU 核心数
- `use_pv` (boolean): 是否启用相位声码器

**返回**: `void`

**示例**:
```javascript
// 更新音调
await invoke('update_parameter', { 
  name: 'pitch', 
  value: 5 
});

// 更新 F0 方法
await invoke('update_parameter', { 
  name: 'f0method', 
  value: 'rmvpe' 
});
```

### 5. 状态查询

#### `get_realtime_status`
获取实时状态信息。

**参数**: 无

**返回**: `Record<string, any>`
```typescript
interface RealtimeStatus {
  app_state: string;        // 应用状态
  is_converting: boolean;   // 是否正在转换
  delay_time: number;       // 算法延迟 (ms)
  infer_time: number;       // 推理时间 (ms)
  buffer_usage: number;     // 缓冲区使用率 (%)
  cpu_usage: number;        // CPU 使用率 (%)
  gpu_usage?: number;       // GPU 使用率 (%) - 可选
}
```

**示例**:
```javascript
const status = await invoke('get_realtime_status');
console.log(`延迟: ${status.delay_time}ms`);
```

### 6. 应用初始化

#### `initialize_app`
初始化应用，包括加载配置、初始化设备管理器等。

**参数**: 无

**返回**: `void`

**示例**:
```javascript
await invoke('initialize_app');
```

## 使用示例

### 完整的初始化流程

```javascript
// 1. 初始化应用
await invoke('initialize_app');

// 2. 加载配置
const config = await invoke('load_config');

// 3. 获取设备列表
const hostApis = await invoke('list_host_apis');
const inputDevices = await invoke('list_input_devices');
const outputDevices = await invoke('list_output_devices');

// 4. 应用配置到表单
Object.assign(formData, config);
```

### 开始语音转换流程

```javascript
// 1. 保存当前配置
await invoke('save_config', { config: formData });

// 2. 开始转换
try {
  await invoke('start_voice_conversion');
  
  // 3. 开始监控状态
  const timer = setInterval(async () => {
    const status = await invoke('get_realtime_status');
    updateUI(status);
  }, 100);
  
} catch (error) {
  alert(`启动失败: ${error}`);
}
```

### 实时参数调整

```javascript
// 监听滑块变化
const handlePitchChange = async (value) => {
  await invoke('update_parameter', { 
    name: 'pitch', 
    value: parseInt(value) 
  });
};
```

## 错误处理

所有命令都可能返回错误，前端应该妥善处理：

```javascript
try {
  const result = await invoke('command_name', params);
  // 处理成功结果
} catch (error) {
  console.error('命令执行失败:', error);
  // 显示用户友好的错误信息
  showErrorMessage(error.toString());
}
```

## 注意事项

1. **异步操作**: 所有 Tauri 命令都是异步的，使用 `await` 或 `.then()` 处理。

2. **参数验证**: 后端会验证参数范围，超出范围会返回错误。

3. **状态一致性**: 前端应该定期查询后端状态，确保 UI 与实际状态同步。

4. **错误恢复**: 如果操作失败，前端应该提供恢复机制（如重试按钮）。

5. **性能考虑**: 
   - 状态查询不要太频繁（建议 100ms 间隔）
   - 批量更新参数时考虑防抖处理

## 待实现功能

以下功能已在接口中定义，但实际实现仍在开发中：

1. 真实的音频设备枚举（目前使用模拟数据）
2. 实际的语音转换处理（目前只改变状态）
3. 模型加载和推理
4. 实时音频流处理
5. GPU 使用率监控
6. 高级音频参数（降噪等）

详见 `TODO.md` 文件了解完整的待实现功能列表。