<script setup>
import { ref, reactive, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

// 响应式数据
const formData = reactive({
    // 模型路径
    pthPath: "",
    indexPath: "",

    // 音频设备
    hostApi: "",
    wasapiExclusive: false,
    inputDevice: "",
    outputDevice: "",
    srModel: true,
    srDevice: false,

    // 常规设置
    threshold: -60,
    pitch: 0,
    formant: 0.0,
    indexRate: 0,
    rmsMixRate: 0,
    f0Method: "fcpe", // pm, harvest, crepe, rmvpe, fcpe

    // 性能设置
    blockTime: 0.25,
    nCpu: 1,
    crossfadeLength: 0.05,
    extraTime: 2.5,
    inputNoiseReduce: false,
    outputNoiseReduce: false,
    usePv: false,

    // 功能选择
    inputMonitor: false,
    outputVoiceChange: true,
});

// 设备列表
const hostApis = ref([]);
const inputDevices = ref([]);
const outputDevices = ref([]);

// 状态信息
const srStream = ref("");
const delayTime = ref("0");
const inferTime = ref("0");
const isConverting = ref(false);
const appInitialized = ref(false);
const initializationError = ref("");

// 实时状态
const realtimeStatus = reactive({
    algorithmLatency: 0,
    inferenceTime: 0,
    bufferUsage: 0,
    cpuUsage: 0,
    gpuUsage: null,
    isConverting: false,
});

// 状态监控定时器
let statusTimer = null;

// 加载设备列表
const loadDevices = async () => {
    try {
        const result = await invoke("get_audio_devices");
        hostApis.value = result.hostapis || [];
        inputDevices.value = result.input_devices || [];
        outputDevices.value = result.output_devices || [];
    } catch (error) {
        console.error("加载设备列表失败:", error);
    }
};

// 重载设备列表
const reloadDevices = async () => {
    console.log("重载设备列表");
    await invoke("reload_devices");
    await loadDevices();
};

// 选择 PTH 文件
const selectPthFile = async () => {
    try {
        const result = await invoke("select_pth_file");
        if (result) {
            formData.pthPath = result;
        }
    } catch (error) {
        console.error("选择 PTH 文件失败:", error);
    }
};

// 选择 Index 文件
const selectIndexFile = async () => {
    try {
        const result = await invoke("select_index_file");
        if (result) {
            formData.indexPath = result;
        }
    } catch (error) {
        console.error("选择 Index 文件失败:", error);
    }
};

// 开始音频转换
const startVoiceConversion = async () => {
    console.log("开始音频转换", formData);

    // 验证配置
    try {
        const isValid = await invoke("validate_config", { config: formData });
        if (!isValid) {
            alert("配置验证失败，请检查模型文件路径和设备配置");
            return;
        }
    } catch (error) {
        console.error("配置验证失败:", error);
        alert(`配置验证失败: ${error}`);
        return;
    }

    try {
        await invoke("start_voice_conversion", { config: formData });
        console.log("语音转换已启动");
    } catch (error) {
        console.error("开始音频转换失败:", error);
        alert(`启动失败: ${error}`);
    }
};

// 停止音频转换
// 停止语音转换
const stopVoiceConversion = async () => {
    console.log("停止语音转换");
    try {
        await invoke("stop_voice_conversion");
        console.log("语音转换已停止");
    } catch (error) {
        console.error("停止音频转换失败:", error);
        alert(`停止失败: ${error}`);
    }
};

// 处理设备变化
const handleDeviceChange = async () => {
    console.log("设备配置变化", {
        hostApi: formData.hostApi,
        inputDevice: formData.inputDevice,
        outputDevice: formData.outputDevice,
    });
    await invoke("update_device_config", {
        hostApi: formData.hostApi,
        inputDevice: formData.inputDevice,
        outputDevice: formData.outputDevice,
        wasapiExclusive: formData.wasapiExclusive,
    });
};

// 处理参数变化
const handleParameterChange = (param, value) => {
    console.log(`参数变化: ${param} = ${value}`);
    invoke("update_parameter", { param, value });
};

// 初始化应用
const initializeApp = async () => {
    try {
        console.log("初始化 RVC 应用...");
        await invoke("initialize_app");
        appInitialized.value = true;
        initializationError.value = "";
        console.log("应用初始化成功");

        // 加载设备列表
        await loadDevices();

        // 开始状态监控
        startStatusMonitoring();
    } catch (error) {
        console.error("应用初始化失败:", error);
        initializationError.value = `初始化失败: ${error}`;
        appInitialized.value = false;
    }
};

// 开始状态监控
const startStatusMonitoring = () => {
    statusTimer = setInterval(async () => {
        try {
            const status = await invoke("get_realtime_status");
            Object.assign(realtimeStatus, status);

            // 更新界面显示
            delayTime.value = status.algorithmLatency.toFixed(1);
            inferTime.value = status.inferenceTime.toFixed(1);
            isConverting.value = status.isConverting;
        } catch (error) {
            console.error("获取实时状态失败:", error);
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

onMounted(() => {
    initializeApp();
});

onUnmounted(() => {
    stopStatusMonitoring();
});
</script>

<template>
    <div class="rvc-app">
        <header class="app-header">
            <h1>RVC - 实时语音转换</h1>
            <div v-if="!appInitialized" class="init-status">
                <div v-if="initializationError" class="error">
                    {{ initializationError }}
                </div>
                <div v-else class="loading">正在初始化应用...</div>
            </div>
        </header>

        <main class="app-main" :class="{ disabled: !appInitialized }">
            <!-- 模型加载区域 -->
            <section class="model-section">
                <h2>加载模型</h2>
                <div class="form-group">
                    <label>选择.pth文件:</label>
                    <div class="file-input-group">
                        <input
                            v-model="formData.pthPath"
                            type="text"
                            placeholder="请选择.pth文件"
                            readonly
                        />
                        <button @click="selectPthFile" class="file-btn">
                            浏览
                        </button>
                    </div>
                </div>
                <div class="form-group">
                    <label>选择.index文件:</label>
                    <div class="file-input-group">
                        <input
                            v-model="formData.indexPath"
                            type="text"
                            placeholder="请选择.index文件"
                            readonly
                        />
                        <button @click="selectIndexFile" class="file-btn">
                            浏览
                        </button>
                    </div>
                </div>
            </section>

            <!-- 音频设备区域 -->
            <section class="device-section">
                <h2>音频设备</h2>
                <div class="form-row">
                    <div class="form-group">
                        <label>设备类型:</label>
                        <select
                            v-model="formData.hostApi"
                            @change="handleDeviceChange"
                        >
                            <option
                                v-for="api in hostApis"
                                :key="api"
                                :value="api"
                            >
                                {{ api }}
                            </option>
                        </select>
                    </div>
                    <div class="form-group">
                        <label>
                            <input
                                v-model="formData.wasapiExclusive"
                                type="checkbox"
                                @change="handleDeviceChange"
                            />
                            独占 WASAPI 设备
                        </label>
                    </div>
                </div>

                <div class="form-group">
                    <label>输入设备:</label>
                    <select
                        v-model="formData.inputDevice"
                        @change="handleDeviceChange"
                    >
                        <option
                            v-for="device in inputDevices"
                            :key="device"
                            :value="device"
                        >
                            {{ device }}
                        </option>
                    </select>
                </div>

                <div class="form-group">
                    <label>输出设备:</label>
                    <select
                        v-model="formData.outputDevice"
                        @change="handleDeviceChange"
                    >
                        <option
                            v-for="device in outputDevices"
                            :key="device"
                            :value="device"
                        >
                            {{ device }}
                        </option>
                    </select>
                </div>

                <div class="form-row">
                    <button @click="reloadDevices" class="reload-btn">
                        重载设备列表
                    </button>
                    <div class="radio-group">
                        <label>
                            <input
                                v-model="formData.srModel"
                                type="radio"
                                name="srType"
                                :value="true"
                                @change="formData.srDevice = false"
                            />
                            使用模型采样率
                        </label>
                        <label>
                            <input
                                v-model="formData.srDevice"
                                type="radio"
                                name="srType"
                                :value="true"
                                @change="formData.srModel = false"
                            />
                            使用设备采样率
                        </label>
                    </div>
                    <span>采样率: {{ srStream }}</span>
                </div>
            </section>

            <div class="settings-row">
                <!-- 常规设置 -->
                <section class="general-settings">
                    <h2>常规设置</h2>

                    <div class="slider-group">
                        <label>响应阈值: {{ formData.threshold }}</label>
                        <input
                            v-model="formData.threshold"
                            type="range"
                            min="-60"
                            max="0"
                            step="1"
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
                            v-model="formData.pitch"
                            type="range"
                            min="-16"
                            max="16"
                            step="1"
                            @input="
                                handleParameterChange('pitch', formData.pitch)
                            "
                        />
                    </div>

                    <div class="slider-group">
                        <label>性别因子/声线粗细: {{ formData.formant }}</label>
                        <input
                            v-model="formData.formant"
                            type="range"
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
                        <label>Index Rate: {{ formData.indexRate }}</label>
                        <input
                            v-model="formData.indexRate"
                            type="range"
                            min="0"
                            max="1"
                            step="0.01"
                            @input="
                                handleParameterChange(
                                    'indexRate',
                                    formData.indexRate,
                                )
                            "
                        />
                    </div>

                    <div class="slider-group">
                        <label>响度因子: {{ formData.rmsMixRate }}</label>
                        <input
                            v-model="formData.rmsMixRate"
                            type="range"
                            min="0"
                            max="1"
                            step="0.01"
                            @input="
                                handleParameterChange(
                                    'rmsMixRate',
                                    formData.rmsMixRate,
                                )
                            "
                        />
                    </div>

                    <div class="form-group">
                        <label>音高算法:</label>
                        <div class="radio-group">
                            <label
                                ><input
                                    v-model="formData.f0Method"
                                    type="radio"
                                    value="pm"
                                    @change="
                                        handleParameterChange('f0Method', 'pm')
                                    "
                                />pm</label
                            >
                            <label
                                ><input
                                    v-model="formData.f0Method"
                                    type="radio"
                                    value="harvest"
                                    @change="
                                        handleParameterChange(
                                            'f0Method',
                                            'harvest',
                                        )
                                    "
                                />harvest</label
                            >
                            <label
                                ><input
                                    v-model="formData.f0Method"
                                    type="radio"
                                    value="crepe"
                                    @change="
                                        handleParameterChange(
                                            'f0Method',
                                            'crepe',
                                        )
                                    "
                                />crepe</label
                            >
                            <label
                                ><input
                                    v-model="formData.f0Method"
                                    type="radio"
                                    value="rmvpe"
                                    @change="
                                        handleParameterChange(
                                            'f0Method',
                                            'rmvpe',
                                        )
                                    "
                                />rmvpe</label
                            >
                            <label
                                ><input
                                    v-model="formData.f0Method"
                                    type="radio"
                                    value="fcpe"
                                    @change="
                                        handleParameterChange(
                                            'f0Method',
                                            'fcpe',
                                        )
                                    "
                                />fcpe</label
                            >
                        </div>
                    </div>
                </section>

                <!-- 性能设置 -->
                <section class="performance-settings">
                    <h2>性能设置</h2>

                    <div class="slider-group">
                        <label>采样长度: {{ formData.blockTime }}</label>
                        <input
                            v-model="formData.blockTime"
                            type="range"
                            min="0.02"
                            max="1.5"
                            step="0.01"
                            @input="
                                handleParameterChange(
                                    'blockTime',
                                    formData.blockTime,
                                )
                            "
                        />
                    </div>

                    <div class="slider-group">
                        <label>harvest进程数: {{ formData.nCpu }}</label>
                        <input
                            v-model="formData.nCpu"
                            type="range"
                            min="1"
                            max="8"
                            step="1"
                            @input="
                                handleParameterChange('nCpu', formData.nCpu)
                            "
                        />
                    </div>

                    <div class="slider-group">
                        <label
                            >淡入淡出长度: {{ formData.crossfadeLength }}</label
                        >
                        <input
                            v-model="formData.crossfadeLength"
                            type="range"
                            min="0.01"
                            max="0.15"
                            step="0.01"
                            @input="
                                handleParameterChange(
                                    'crossfadeLength',
                                    formData.crossfadeLength,
                                )
                            "
                        />
                    </div>

                    <div class="slider-group">
                        <label>额外推理时长: {{ formData.extraTime }}</label>
                        <input
                            v-model="formData.extraTime"
                            type="range"
                            min="0.05"
                            max="5.00"
                            step="0.01"
                            @input="
                                handleParameterChange(
                                    'extraTime',
                                    formData.extraTime,
                                )
                            "
                        />
                    </div>

                    <div class="checkbox-group">
                        <label>
                            <input
                                v-model="formData.inputNoiseReduce"
                                type="checkbox"
                                @change="
                                    handleParameterChange(
                                        'inputNoiseReduce',
                                        formData.inputNoiseReduce,
                                    )
                                "
                            />
                            输入降噪
                        </label>
                        <label>
                            <input
                                v-model="formData.outputNoiseReduce"
                                type="checkbox"
                                @change="
                                    handleParameterChange(
                                        'outputNoiseReduce',
                                        formData.outputNoiseReduce,
                                    )
                                "
                            />
                            输出降噪
                        </label>
                        <label>
                            <input
                                v-model="formData.usePv"
                                type="checkbox"
                                @change="
                                    handleParameterChange(
                                        'usePv',
                                        formData.usePv,
                                    )
                                "
                            />
                            启用相位声码器
                        </label>
                    </div>
                </section>
            </div>

            <!-- 控制区域 -->
            <section class="control-section">
                <div class="control-buttons">
                    <button
                        @click="startVoiceConversion"
                        :disabled="isConverting"
                        class="start-btn"
                    >
                        开始音频转换
                    </button>
                    <button
                        @click="stopVoiceConversion"
                        :disabled="!isConverting"
                        class="stop-btn"
                    >
                        停止音频转换
                    </button>
                </div>

                <div class="function-selection">
                    <label>
                        <input
                            v-model="formData.inputMonitor"
                            type="radio"
                            name="function"
                            :value="true"
                            @change="formData.outputVoiceChange = false"
                        />
                        输入监听
                    </label>
                    <label>
                        <input
                            v-model="formData.outputVoiceChange"
                            type="radio"
                            name="function"
                            :value="true"
                            @change="formData.inputMonitor = false"
                        />
                        输出变声
                    </label>
                </div>

                <div class="status-info">
                    <div class="status-item">
                        <span>算法延迟(ms):</span>
                        <span class="status-value">{{ delayTime }}</span>
                    </div>
                    <div class="status-item">
                        <span>推理时间(ms):</span>
                        <span class="status-value">{{ inferTime }}</span>
                    </div>
                    <div class="status-item">
                        <span>缓冲区使用率:</span>
                        <span class="status-value"
                            >{{ realtimeStatus.bufferUsage.toFixed(1) }}%</span
                        >
                    </div>
                    <div class="status-item">
                        <span>CPU使用率:</span>
                        <span class="status-value"
                            >{{ realtimeStatus.cpuUsage.toFixed(1) }}%</span
                        >
                    </div>
                    <div
                        class="status-item"
                        v-if="realtimeStatus.gpuUsage !== null"
                    >
                        <span>GPU使用率:</span>
                        <span class="status-value"
                            >{{ realtimeStatus.gpuUsage.toFixed(1) }}%</span
                        >
                    </div>
                    <div class="status-item">
                        <span>转换状态:</span>
                        <span
                            class="status-value"
                            :class="{ active: isConverting }"
                        >
                            {{ isConverting ? "运行中" : "空闲" }}
                        </span>
                    </div>
                </div>
            </section>
        </main>
    </div>
</template>

<style scoped>
.rvc-app {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
    font-family: "Segoe UI", Tahoma, Geneva, Verdana, sans-serif;
}

.app-header {
    text-align: center;
    margin-bottom: 30px;
}

.app-header h1 {
    color: #2c3e50;
    margin: 0;
}

.app-main {
    display: flex;
    flex-direction: column;
    gap: 20px;
}

section {
    background: white;
    border-radius: 8px;
    padding: 20px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

section h2 {
    margin: 0 0 15px 0;
    color: #34495e;
    font-size: 1.2em;
    border-bottom: 2px solid #3498db;
    padding-bottom: 5px;
}

.form-group {
    margin-bottom: 15px;
}

.form-row {
    display: flex;
    gap: 20px;
    align-items: center;
    flex-wrap: wrap;
}

.form-group label {
    display: block;
    margin-bottom: 5px;
    font-weight: 500;
    color: #2c3e50;
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
}

input[type="text"]:focus,
select:focus {
    outline: none;
    border-color: #3498db;
    box-shadow: 0 0 0 2px rgba(52, 152, 219, 0.2);
}

.file-btn,
.reload-btn {
    padding: 8px 16px;
    background: #3498db;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
}

.file-btn:hover,
.reload-btn:hover {
    background: #2980b9;
}

.settings-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 20px;
}

.slider-group {
    margin-bottom: 15px;
}

.slider-group label {
    display: block;
    margin-bottom: 5px;
    font-weight: 500;
    color: #2c3e50;
}

input[type="range"] {
    width: 100%;
    margin: 5px 0;
}

.radio-group {
    display: flex;
    gap: 15px;
    flex-wrap: wrap;
}

.radio-group label {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 0;
    font-weight: normal;
    cursor: pointer;
}

.checkbox-group {
    display: flex;
    flex-direction: column;
    gap: 10px;
}

.checkbox-group label {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 0;
    font-weight: normal;
    cursor: pointer;
}

.control-section {
    background: #f8f9fa;
    border: 2px solid #e9ecef;
}

.control-buttons {
    display: flex;
    gap: 15px;
    margin-bottom: 20px;
}

.start-btn,
.stop-btn {
    padding: 12px 24px;
    border: none;
    border-radius: 6px;
    font-size: 16px;
    font-weight: 500;
    cursor: pointer;
    transition: all 0.3s ease;
}

.start-btn {
    background: #27ae60;
    color: white;
}

.start-btn:hover:not(:disabled) {
    background: #229954;
}

.stop-btn {
    background: #e74c3c;
    color: white;
}

.stop-btn:hover:not(:disabled) {
    background: #c0392b;
}

.start-btn:disabled,
.stop-btn:disabled {
    background: #bdc3c7;
    cursor: not-allowed;
}

.function-selection {
    display: flex;
    gap: 20px;
    margin-bottom: 15px;
}

.function-selection label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 500;
    cursor: pointer;
}

.status-info {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 15px;
    color: #7f8c8d;
    font-size: 14px;
    padding: 15px;
    background-color: #f8f9fa;
    border-radius: 8px;
    border: 1px solid #e9ecef;
}

.status-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 5px 0;
}

.status-value {
    font-weight: 600;
    color: #2c3e50;
}

.status-value.active {
    color: #27ae60;
    font-weight: bold;
}

.init-status {
    margin-top: 10px;
    text-align: center;
}

.init-status .loading {
    color: #3498db;
    font-size: 14px;
}

.init-status .error {
    color: #e74c3c;
    font-size: 14px;
    font-weight: 500;
}

.app-main.disabled {
    opacity: 0.6;
    pointer-events: none;
}

/* 响应式设计 */
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
        grid-template-columns: 1fr;
        gap: 10px;
    }

    .status-item {
        flex-direction: row;
        justify-content: space-between;
        min-width: auto;
    }
}

/* 深色模式支持 */
@media (prefers-color-scheme: dark) {
    .rvc-app {
        background: #2c3e50;
        color: #ecf0f1;
    }

    section {
        background: #34495e;
        color: #ecf0f1;
    }

    section h2 {
        color: #ecf0f1;
        border-bottom-color: #3498db;
    }

    .form-group label {
        color: #ecf0f1;
    }

    input[type="text"],
    select {
        background: #2c3e50;
        color: #ecf0f1;
        border-color: #556983;
    }

    input[type="text"]:focus,
    select:focus {
        border-color: #3498db;
        box-shadow: 0 0 0 2px rgba(52, 152, 219, 0.3);
    }

    .control-section {
        background: #2c3e50;
        border-color: #556983;
    }

    .status-info {
        color: #95a5a6;
        background-color: #2c3e50;
        border-color: #34495e;
    }

    .status-value {
        color: #ecf0f1;
    }

    .status-value.active {
        color: #2ecc71;
    }

    .init-status .loading {
        color: #3498db;
    }

    .init-status .error {
        color: #e74c3c;
    }
}
</style>
