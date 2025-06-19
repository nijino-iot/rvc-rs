<template>
    <div class="rvc-app">
        <header class="app-header">
            <h1>RVC - 实时语音转换</h1>
        </header>

        <main class="app-main" :class="{ disabled: !appInitialized }">
            <!-- 初始化状态 -->
            <div v-if="!appInitialized" class="init-status">
                <div v-if="!initializationError" class="loading">
                    正在初始化应用...
                </div>
                <div v-else class="error">
                    初始化失败: {{ initializationError }}
                </div>
            </div>

            <!-- 主界面 -->
            <template v-else>
                <!-- 模型加载 -->
                <section>
                    <h2>加载模型</h2>
                    <div class="form-group">
                        <label>模型文件 (.pth)</label>
                        <div class="file-input-group">
                            <input
                                type="text"
                                v-model="formData.pth_path"
                                placeholder="选择.pth文件"
                                readonly
                            />
                            <button class="file-btn" @click="selectPthFile">
                                选择文件
                            </button>
                        </div>
                    </div>
                    <div class="form-group">
                        <label>Index文件 (.index)</label>
                        <div class="file-input-group">
                            <input
                                type="text"
                                v-model="formData.index_path"
                                placeholder="选择.index文件"
                                readonly
                            />
                            <button class="file-btn" @click="selectIndexFile">
                                选择文件
                            </button>
                        </div>
                    </div>
                </section>

                <!-- 音频设备 -->
                <section>
                    <h2>音频设备</h2>
                    <div class="form-group">
                        <div class="form-row">
                            <label>设备类型</label>
                            <select
                                v-model="formData.sg_hostapi"
                                @change="handleHostApiChange"
                            >
                                <option
                                    v-for="api in hostApis"
                                    :key="api"
                                    :value="api"
                                >
                                    {{ api }}
                                </option>
                            </select>
                            <label class="checkbox-label">
                                <input
                                    type="checkbox"
                                    v-model="formData.sg_wasapi_exclusive"
                                    :disabled="
                                        !formData.sg_hostapi.includes('WASAPI')
                                    "
                                />
                                独占 WASAPI 设备
                            </label>
                        </div>
                    </div>
                    <div class="form-group">
                        <label>输入设备</label>
                        <select
                            v-model="formData.sg_input_device"
                            @change="handleDeviceChange"
                        >
                            <option
                                v-for="device in inputDevices"
                                :key="device.id"
                                :value="device.name"
                            >
                                {{ device.name }}
                            </option>
                        </select>
                    </div>
                    <div class="form-group">
                        <label>输出设备</label>
                        <select
                            v-model="formData.sg_output_device"
                            @change="handleDeviceChange"
                        >
                            <option
                                v-for="device in outputDevices"
                                :key="device.id"
                                :value="device.name"
                            >
                                {{ device.name }}
                            </option>
                        </select>
                    </div>
                    <div class="form-group">
                        <button class="reload-btn" @click="reloadDevices">
                            重载设备列表
                        </button>
                        <div class="radio-group">
                            <label>
                                <input
                                    type="radio"
                                    v-model="formData.sr_type"
                                    value="sr_model"
                                    @change="
                                        handleParameterChange(
                                            'sr_type',
                                            formData.sr_type,
                                        )
                                    "
                                />
                                使用模型采样率
                            </label>
                            <label>
                                <input
                                    type="radio"
                                    v-model="formData.sr_type"
                                    value="sr_device"
                                    @change="
                                        handleParameterChange(
                                            'sr_type',
                                            formData.sr_type,
                                        )
                                    "
                                />
                                使用设备采样率
                            </label>
                        </div>
                        <span>采样率: {{ srStream || "未知" }}</span>
                    </div>
                </section>

                <!-- 常规设置 -->
                <section>
                    <h2>常规设置</h2>
                    <div class="settings-row">
                        <div class="slider-group">
                            <label>响应阈值: {{ formData.threshold }}</label>
                            <input
                                type="range"
                                v-model="formData.threshold"
                                min="-60"
                                max="0"
                                @input="
                                    handleParameterChange(
                                        'threshold',
                                        formData.threshold,
                                    )
                                "
                            />
                        </div>
                        <div class="slider-group">
                            <label>音调设置: {{ formData.pitch }}</label>
                            <input
                                type="range"
                                v-model="formData.pitch"
                                min="-16"
                                max="16"
                                @input="
                                    handleParameterChange(
                                        'pitch',
                                        formData.pitch,
                                    )
                                "
                            />
                        </div>
                        <div class="slider-group">
                            <label
                                >性别因子/声线粗细:
                                {{ formData.formant.toFixed(2) }}</label
                            >
                            <input
                                type="range"
                                v-model="formData.formant"
                                min="-2"
                                max="2"
                                step="0.05"
                                @input="
                                    handleParameterChange(
                                        'formant',
                                        formData.formant,
                                    )
                                "
                            />
                        </div>
                        <div class="slider-group">
                            <label
                                >Index Rate:
                                {{ formData.index_rate.toFixed(2) }}</label
                            >
                            <input
                                type="range"
                                v-model="formData.index_rate"
                                min="0"
                                max="1"
                                step="0.01"
                                @input="
                                    handleParameterChange(
                                        'index_rate',
                                        formData.index_rate,
                                    )
                                "
                            />
                        </div>
                        <div class="slider-group">
                            <label
                                >响度因子:
                                {{ formData.rms_mix_rate.toFixed(2) }}</label
                            >
                            <input
                                type="range"
                                v-model="formData.rms_mix_rate"
                                min="0"
                                max="1"
                                step="0.01"
                                @input="
                                    handleParameterChange(
                                        'rms_mix_rate',
                                        formData.rms_mix_rate,
                                    )
                                "
                            />
                        </div>
                    </div>

                    <div class="form-group">
                        <label>音高算法</label>
                        <div class="radio-group">
                            <label
                                v-for="method in [
                                    'pm',
                                    'harvest',
                                    'crepe',
                                    'rmvpe',
                                    'fcpe',
                                ]"
                                :key="method"
                            >
                                <input
                                    type="radio"
                                    v-model="formData.f0method"
                                    :value="method"
                                    @change="
                                        handleParameterChange(
                                            'f0method',
                                            formData.f0method,
                                        )
                                    "
                                />
                                {{ method.toUpperCase() }}
                            </label>
                        </div>
                    </div>
                </section>

                <!-- 性能设置 -->
                <section>
                    <h2>性能设置</h2>
                    <div class="settings-row">
                        <div class="slider-group">
                            <label
                                >采样长度:
                                {{ formData.block_time.toFixed(2) }}s</label
                            >
                            <input
                                type="range"
                                v-model="formData.block_time"
                                min="0.02"
                                max="1.5"
                                step="0.01"
                                @change="
                                    handleParameterChange(
                                        'block_time',
                                        formData.block_time,
                                    )
                                "
                            />
                        </div>
                        <div class="slider-group">
                            <label>harvest进程数: {{ formData.n_cpu }}</label>
                            <input
                                type="range"
                                v-model="formData.n_cpu"
                                min="1"
                                max="8"
                                @change="
                                    handleParameterChange(
                                        'n_cpu',
                                        formData.n_cpu,
                                    )
                                "
                            />
                        </div>
                        <div class="slider-group">
                            <label
                                >淡入淡出长度:
                                {{ formData.crossfade_time.toFixed(2) }}s</label
                            >
                            <input
                                type="range"
                                v-model="formData.crossfade_time"
                                min="0.01"
                                max="0.15"
                                step="0.01"
                                @change="
                                    handleParameterChange(
                                        'crossfade_time',
                                        formData.crossfade_time,
                                    )
                                "
                            />
                        </div>
                        <div class="slider-group">
                            <label
                                >额外推理时长:
                                {{ formData.extra_time.toFixed(2) }}s</label
                            >
                            <input
                                type="range"
                                v-model="formData.extra_time"
                                min="0.05"
                                max="5.00"
                                step="0.01"
                                @change="
                                    handleParameterChange(
                                        'extra_time',
                                        formData.extra_time,
                                    )
                                "
                            />
                        </div>
                    </div>

                    <div class="checkbox-group">
                        <label>
                            <input
                                type="checkbox"
                                v-model="formData.i_noise_reduce"
                                @change="
                                    handleParameterChange(
                                        'i_noise_reduce',
                                        formData.i_noise_reduce,
                                    )
                                "
                            />
                            输入降噪
                        </label>
                        <label>
                            <input
                                type="checkbox"
                                v-model="formData.o_noise_reduce"
                                @change="
                                    handleParameterChange(
                                        'o_noise_reduce',
                                        formData.o_noise_reduce,
                                    )
                                "
                            />
                            输出降噪
                        </label>
                        <label>
                            <input
                                type="checkbox"
                                v-model="formData.use_pv"
                                @change="
                                    handleParameterChange(
                                        'use_pv',
                                        formData.use_pv,
                                    )
                                "
                            />
                            启用相位声码器
                        </label>
                    </div>
                </section>

                <!-- 控制按钮 -->
                <section class="control-section">
                    <div class="control-buttons">
                        <button
                            class="start-btn"
                            @click="startVoiceConversion"
                            :disabled="isConverting"
                        >
                            开始音频转换
                        </button>
                        <button
                            class="stop-btn"
                            @click="stopVoiceConversion"
                            :disabled="!isConverting"
                        >
                            停止音频转换
                        </button>
                    </div>

                    <div class="function-selection">
                        <label>
                            <input
                                type="radio"
                                v-model="functionMode"
                                value="im"
                            />
                            输入监听
                        </label>
                        <label>
                            <input
                                type="radio"
                                v-model="functionMode"
                                value="vc"
                            />
                            输出变声
                        </label>
                    </div>

                    <div class="status-info">
                        <div class="status-item">
                            <span>算法延迟(ms):</span>
                            <span
                                class="status-value"
                                :class="{ active: isConverting }"
                            >
                                {{ delayTime }}
                            </span>
                        </div>
                        <div class="status-item">
                            <span>推理时间(ms):</span>
                            <span
                                class="status-value"
                                :class="{ active: isConverting }"
                            >
                                {{ inferTime }}
                            </span>
                        </div>
                    </div>
                </section>
            </template>
        </main>
    </div>
</template>

<script setup>
import { ref, reactive, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

// 表单数据
const formData = reactive({
    pth_path: "",
    index_path: "",
    pitch: 0,
    formant: 0.0,
    sr_type: "sr_model",
    block_time: 0.25,
    threshold: -60,
    crossfade_time: 0.05,
    extra_time: 2.5,
    i_noise_reduce: false,
    o_noise_reduce: false,
    use_pv: false,
    rms_mix_rate: 0.0,
    index_rate: 0.0,
    n_cpu: 4,
    f0method: "fcpe",
    sg_hostapi: "",
    sg_wasapi_exclusive: false,
    sg_input_device: "",
    sg_output_device: "",
});

// 设备列表
const hostApis = ref([]);
const inputDevices = ref([]);
const outputDevices = ref([]);

// 状态信息
const srStream = ref("");
const delayTime = ref(0);
const inferTime = ref(0);
const isConverting = ref(false);
const appInitialized = ref(false);
const initializationError = ref("");
const functionMode = ref("vc");

// 实时状态
let statusTimer = null;

// 加载设备列表
const loadDevices = async (hostApi = null) => {
    try {
        const inputs = await invoke("list_input_devices", { hostApi });
        const outputs = await invoke("list_output_devices", { hostApi });
        inputDevices.value = inputs;
        outputDevices.value = outputs;
    } catch (error) {
        console.error("加载设备失败:", error);
    }
};

// 重载设备
const reloadDevices = async () => {
    try {
        await invoke("reload_devices", {
            hostApi: formData.sg_hostapi || null,
        });
        await loadDevices(formData.sg_hostapi);
    } catch (error) {
        console.error("重载设备失败:", error);
    }
};

// 选择 pth 文件
const selectPthFile = async () => {
    try {
        const selected = await open({
            multiple: false,
            filters: [
                {
                    name: "PyTorch Model",
                    extensions: ["pth"],
                },
            ],
        });
        if (selected) {
            formData.pth_path = selected;
        }
    } catch (error) {
        console.error("选择文件失败:", error);
    }
};

// 选择 index 文件
const selectIndexFile = async () => {
    try {
        const selected = await open({
            multiple: false,
            filters: [
                {
                    name: "Index File",
                    extensions: ["index"],
                },
            ],
        });
        if (selected) {
            formData.index_path = selected;
        }
    } catch (error) {
        console.error("选择文件失败:", error);
    }
};

// 开始语音转换
const startVoiceConversion = async () => {
    try {
        // 保存配置
        await invoke("save_config", { data: formData });

        // 开始转换
        await invoke("start_voice_conversion");
        isConverting.value = true;

        // 开始监控状态
        startStatusMonitoring();
    } catch (error) {
        console.error("开始转换失败:", error);
        alert(`开始转换失败: ${error}`);
    }
};

// 停止语音转换
const stopVoiceConversion = async () => {
    try {
        await invoke("stop_voice_conversion");
        isConverting.value = false;

        // 停止监控
        stopStatusMonitoring();
    } catch (error) {
        console.error("停止转换失败:", error);
    }
};

// 处理主机API变化
const handleHostApiChange = async () => {
    await loadDevices(formData.sg_hostapi);
    if (
        inputDevices.value.length > 0 &&
        !inputDevices.value.find((d) => d.name === formData.sg_input_device)
    ) {
        formData.sg_input_device = inputDevices.value[0].name;
    }
    if (
        outputDevices.value.length > 0 &&
        !outputDevices.value.find((d) => d.name === formData.sg_output_device)
    ) {
        formData.sg_output_device = outputDevices.value[0].name;
    }
};

// 处理设备变化
const handleDeviceChange = async () => {
    // TODO: 更新采样率显示
    if (formData.sr_type === "sr_device" && formData.sg_output_device) {
        try {
            const samplerate = await invoke("get_device_samplerate", {
                deviceName: formData.sg_output_device,
            });
            srStream.value = samplerate.toString();
        } catch (error) {
            console.error("获取采样率失败:", error);
        }
    }
};

// 处理参数变化
const handleParameterChange = async (name, value) => {
    try {
        await invoke("update_parameter", { name, value });
    } catch (error) {
        console.error("更新参数失败:", error);
    }
};

// 初始化应用
const initializeApp = async () => {
    try {
        // 初始化后端
        await invoke("initialize_app");

        // 加载主机API列表
        hostApis.value = await invoke("list_host_apis");

        // 加载配置
        const config = await invoke("load_config");
        Object.assign(formData, config);

        // 加载设备列表
        await loadDevices(formData.sg_hostapi);

        appInitialized.value = true;
    } catch (error) {
        console.error("初始化失败:", error);
        initializationError.value = error.toString();
    }
};

// 开始状态监控
const startStatusMonitoring = () => {
    statusTimer = setInterval(async () => {
        try {
            const status = await invoke("get_realtime_status");
            if (status.delay_time !== undefined) {
                delayTime.value = status.delay_time;
            }
            if (status.infer_time !== undefined) {
                inferTime.value = status.infer_time;
            }
        } catch (error) {
            console.error("获取状态失败:", error);
        }
    }, 100); // 每100ms更新一次
};

// 停止状态监控
const stopStatusMonitoring = () => {
    if (statusTimer) {
        clearInterval(statusTimer);
        statusTimer = null;
    }
};

// 生命周期
onMounted(() => {
    initializeApp();
});

onUnmounted(() => {
    stopStatusMonitoring();
});
</script>

<style scoped>
.rvc-app {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
    font-family:
        -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
}

.app-header {
    text-align: center;
    margin-bottom: 30px;
}

.app-header h1 {
    font-size: 2em;
    color: #333;
}

.app-main {
    display: flex;
    flex-direction: column;
    gap: 20px;
}

section {
    background: #f5f5f5;
    border-radius: 8px;
    padding: 20px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

section h2 {
    margin: 0 0 15px 0;
    font-size: 1.3em;
    color: #444;
    border-bottom: 2px solid #e0e0e0;
    padding-bottom: 10px;
}

.form-group {
    margin-bottom: 15px;
}

.form-row {
    display: flex;
    align-items: center;
    gap: 15px;
    flex-wrap: wrap;
}

.form-group label {
    display: block;
    margin-bottom: 5px;
    font-weight: 500;
    color: #555;
}

.file-input-group {
    display: flex;
    gap: 10px;
}

.file-input-group input {
    flex: 1;
}

input[type="text"],
select {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid #ddd;
    border-radius: 4px;
    font-size: 14px;
    background: white;
}

input[type="text"]:focus,
select:focus {
    outline: none;
    border-color: #4caf50;
    box-shadow: 0 0 0 2px rgba(76, 175, 80, 0.2);
}

.file-btn,
.reload-btn {
    padding: 8px 16px;
    background: #4caf50;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    white-space: nowrap;
}

.file-btn:hover,
.reload-btn:hover {
    background: #45a049;
}

.settings-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 20px;
}

.slider-group {
    margin-bottom: 15px;
}

.slider-group label {
    display: block;
    margin-bottom: 5px;
    font-size: 14px;
    color: #555;
}

input[type="range"] {
    width: 100%;
    margin-top: 5px;
}

.radio-group {
    display: flex;
    gap: 15px;
    flex-wrap: wrap;
}

.radio-group label,
.checkbox-label {
    display: flex;
    align-items: center;
    gap: 5px;
    cursor: pointer;
    font-size: 14px;
    color: #555;
}

.checkbox-group {
    display: flex;
    gap: 20px;
    flex-wrap: wrap;
}

.checkbox-group label {
    display: flex;
    align-items: center;
    gap: 5px;
    cursor: pointer;
    font-size: 14px;
    color: #555;
}

.control-section {
    background: #f9f9f9;
    border: 2px solid #e0e0e0;
}

.control-buttons {
    display: flex;
    gap: 15px;
    margin-bottom: 20px;
}

.start-btn,
.stop-btn {
    flex: 1;
    padding: 12px 24px;
    font-size: 16px;
    font-weight: 500;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    transition: background-color 0.3s;
}

.start-btn {
    background: #4caf50;
    color: white;
}

.start-btn:hover:not(:disabled) {
    background: #45a049;
}

.stop-btn {
    background: #f44336;
    color: white;
}

.stop-btn:hover:not(:disabled) {
    background: #da190b;
}

.start-btn:disabled,
.stop-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}

.function-selection {
    display: flex;
    gap: 20px;
    margin-bottom: 20px;
}

.function-selection label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 14px;
    color: #555;
}

.status-info {
    display: flex;
    gap: 30px;
    padding: 15px;
    background: white;
    border-radius: 4px;
    border: 1px solid #e0e0e0;
}

.status-item {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 14px;
}

.status-value {
    font-weight: bold;
    color: #666;
}

.status-value.active {
    color: #4caf50;
}

.init-status {
    text-align: center;
    padding: 50px;
}

.init-status .loading {
    font-size: 18px;
    color: #666;
}

.init-status .error {
    font-size: 18px;
    color: #f44336;
    white-space: pre-wrap;
}

.app-main.disabled {
    opacity: 0.5;
    pointer-events: none;
}

/* 响应式布局 */
@media (max-width: 768px) {
    .settings-row {
        grid-template-columns: 1fr;
    }

    .form-row {
        flex-direction: column;
        align-items: stretch;
    }

    .control-buttons {
        flex-direction: column;
    }

    .function-selection {
        flex-direction: column;
        gap: 10px;
    }

    .status-info {
        flex-direction: column;
        gap: 10px;
    }

    .status-item {
        justify-content: space-between;
    }
}

/* 暗色主题支持 */
@media (prefers-color-scheme: dark) {
    .rvc-app {
        background: #1a1a1a;
        color: #e0e0e0;
    }

    section {
        background: #2a2a2a;
        box-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
    }

    section h2 {
        color: #e0e0e0;
        border-bottom-color: #444;
    }

    .form-group label {
        color: #ccc;
    }

    input[type="text"],
    select {
        background: #333;
        border-color: #444;
        color: #e0e0e0;
    }

    input[type="text"]:focus,
    select:focus {
        border-color: #4caf50;
        box-shadow: 0 0 0 2px rgba(76, 175, 80, 0.3);
    }

    .control-section {
        background: #252525;
        border-color: #444;
    }

    .status-info {
        background: #333;
        border-color: #444;
    }

    .status-value {
        color: #aaa;
    }

    .status-value.active {
        color: #66bb6a;
    }

    .init-status .loading {
        color: #aaa;
    }

    .init-status .error {
        color: #ef5350;
    }
}
</style>
