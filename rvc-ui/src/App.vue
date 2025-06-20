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
                                v-model="values.pth_path"
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
                                v-model="values.index_path"
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
                                v-model="values.sg_hostapi"
                                @change="handleEvent('sg_hostapi')"
                            >
                                <option
                                    v-for="api in hostapis"
                                    :key="api"
                                    :value="api"
                                >
                                    {{ api }}
                                </option>
                            </select>
                            <label class="checkbox-label">
                                <input
                                    type="checkbox"
                                    v-model="values.sg_wasapi_exclusive"
                                    :disabled="
                                        !values.sg_hostapi.includes('WASAPI')
                                    "
                                />
                                独占 WASAPI 设备
                            </label>
                        </div>
                    </div>
                    <div class="form-group">
                        <label>输入设备</label>
                        <select v-model="values.sg_input_device">
                            <option
                                v-for="device in input_devices"
                                :key="device.name"
                                :value="device.name"
                            >
                                {{ device.name }}
                            </option>
                        </select>
                    </div>
                    <div class="form-group">
                        <label>输出设备</label>
                        <select v-model="values.sg_output_device">
                            <option
                                v-for="device in output_devices"
                                :key="device.name"
                                :value="device.name"
                            >
                                {{ device.name }}
                            </option>
                        </select>
                    </div>
                    <div class="form-group">
                        <button
                            class="reload-btn"
                            @click="handleEvent('reload_devices')"
                        >
                            重载设备列表
                        </button>
                        <div class="radio-group">
                            <label>
                                <input
                                    type="radio"
                                    v-model="values.sr_type"
                                    value="sr_model"
                                />
                                使用模型采样率
                            </label>
                            <label>
                                <input
                                    type="radio"
                                    v-model="values.sr_type"
                                    value="sr_device"
                                />
                                使用设备采样率
                            </label>
                        </div>
                        <span>采样率: {{ sr_stream || "未知" }}</span>
                    </div>
                </section>

                <!-- 常规设置 -->
                <section>
                    <h2>常规设置</h2>
                    <div class="settings-row">
                        <div class="slider-group">
                            <label>响应阈值: {{ values.threshold }}</label>
                            <input
                                type="range"
                                v-model="values.threshold"
                                min="-60"
                                max="0"
                                @input="handleEvent('threshold')"
                            />
                        </div>
                        <div class="slider-group">
                            <label>音调设置: {{ values.pitch }}</label>
                            <input
                                type="range"
                                v-model="values.pitch"
                                min="-16"
                                max="16"
                                @input="handleEvent('pitch')"
                            />
                        </div>
                        <div class="slider-group">
                            <label
                                >性别因子/声线粗细:
                                {{ values.formant.toFixed(2) }}</label
                            >
                            <input
                                type="range"
                                v-model="values.formant"
                                min="-2"
                                max="2"
                                step="0.05"
                                @input="handleEvent('formant')"
                            />
                        </div>
                        <div class="slider-group">
                            <label
                                >Index Rate:
                                {{ values.index_rate.toFixed(2) }}</label
                            >
                            <input
                                type="range"
                                v-model="values.index_rate"
                                min="0"
                                max="1"
                                step="0.01"
                                @input="handleEvent('index_rate')"
                            />
                        </div>
                        <div class="slider-group">
                            <label
                                >响度因子:
                                {{ values.rms_mix_rate.toFixed(2) }}</label
                            >
                            <input
                                type="range"
                                v-model="values.rms_mix_rate"
                                min="0"
                                max="1"
                                step="0.01"
                                @input="handleEvent('rms_mix_rate')"
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
                                    v-model="values.f0method"
                                    :value="method"
                                    @change="handleEvent(method)"
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
                                {{ values.block_time.toFixed(2) }}s</label
                            >
                            <input
                                type="range"
                                v-model="values.block_time"
                                min="0.02"
                                max="1.5"
                                step="0.01"
                            />
                        </div>
                        <div class="slider-group">
                            <label>harvest进程数: {{ values.n_cpu }}</label>
                            <input
                                type="range"
                                v-model="values.n_cpu"
                                min="1"
                                max="8"
                            />
                        </div>
                        <div class="slider-group">
                            <label
                                >淡入淡出长度:
                                {{ values.crossfade_time.toFixed(2) }}s</label
                            >
                            <input
                                type="range"
                                v-model="values.crossfade_time"
                                min="0.01"
                                max="0.15"
                                step="0.01"
                            />
                        </div>
                        <div class="slider-group">
                            <label
                                >额外推理时长:
                                {{ values.extra_time.toFixed(2) }}s</label
                            >
                            <input
                                type="range"
                                v-model="values.extra_time"
                                min="0.05"
                                max="5.00"
                                step="0.01"
                            />
                        </div>
                    </div>

                    <div class="checkbox-group">
                        <label>
                            <input
                                type="checkbox"
                                v-model="values.i_noise_reduce"
                                @change="handleEvent('i_noise_reduce')"
                            />
                            输入降噪
                        </label>
                        <label>
                            <input
                                type="checkbox"
                                v-model="values.o_noise_reduce"
                                @change="handleEvent('o_noise_reduce')"
                            />
                            输出降噪
                        </label>
                        <label>
                            <input
                                type="checkbox"
                                v-model="values.use_pv"
                                @change="handleEvent('use_pv')"
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
                            @click="handleEvent('start_vc')"
                            :disabled="flag_vc"
                        >
                            {{ flag_vc ? "转换中..." : "开始转换" }}
                        </button>
                        <button
                            class="stop-btn"
                            @click="handleEvent('stop_vc')"
                            :disabled="!flag_vc"
                        >
                            停止转换
                        </button>
                    </div>

                    <!-- 状态显示 -->
                    <div class="status-display">
                        <div class="status-item">
                            <span>延迟时间: {{ delay_time }}ms</span>
                        </div>
                        <div class="status-item">
                            <span>推理时间: {{ infer_time }}ms</span>
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
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

// 对应 Python 的 values - 所有表单数据
const values = reactive({
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

// 界面数据 - 对应 Python 的 self.* 属性
const hostapis = ref([]);
const input_devices = ref([]);
const output_devices = ref([]);
const sr_stream = ref("");
const delay_time = ref(0);
const infer_time = ref(0);
const flag_vc = ref(false); // 对应 Python 的 flag_vc
const appInitialized = ref(false);
const initializationError = ref("");

// 事件监听器引用，用于清理
let eventUnlisteners = [];

// 统一事件处理器 - 对应 Python 的 event_handler
const handleEvent = async (event) => {
    console.log("Event:", event, "Values:", values);

    try {
        const result = await invoke("handle_event", { event, values });

        // 处理返回结果
        if (result.hostapis) {
            hostapis.value = result.hostapis;
            input_devices.value = result.input_devices;
            output_devices.value = result.output_devices;
        }

        if (result.samplerate) {
            sr_stream.value = result.samplerate.toString();
        }

        if (result.delay_time) {
            delay_time.value = result.delay_time;
        }

        // 更新转换状态
        if (event === "start_vc" && result.success) {
            flag_vc.value = true;
        } else if (event === "stop_vc" && result.success) {
            flag_vc.value = false;
        }
    } catch (error) {
        console.error(`处理事件 ${event} 失败:`, error);
        alert(`操作失败: ${error}`);
    }
};

// 选择文件
const selectPthFile = async () => {
    try {
        const selected = await open({
            multiple: false,
            filters: [{ name: "PyTorch Model", extensions: ["pth"] }],
        });
        if (selected) {
            values.pth_path = selected;
        }
    } catch (error) {
        console.error("选择文件失败:", error);
    }
};

const selectIndexFile = async () => {
    try {
        const selected = await open({
            multiple: false,
            filters: [{ name: "Index File", extensions: ["index"] }],
        });
        if (selected) {
            values.index_path = selected;
        }
    } catch (error) {
        console.error("选择文件失败:", error);
    }
};

// 设置事件监听器 - 替代轮询
const setupEventListeners = async () => {
    try {
        // 监听状态变化事件
        const stateUnlisten = await listen("state-changed", (event) => {
            console.log("状态变化:", event.payload);
            const newState = event.payload;

            // 更新转换状态
            switch (newState) {
                case "Converting":
                    flag_vc.value = true;
                    break;
                case "Ready":
                case "Stopped":
                    flag_vc.value = false;
                    break;
                case "Error":
                    flag_vc.value = false;
                    console.error("应用状态错误:", newState);
                    break;
            }
        });
        eventUnlisteners.push(stateUnlisten);

        // 监听统计信息更新事件
        const statsUnlisten = await listen("stats-updated", (event) => {
            const stats = event.payload;
            // 更新延迟和推理时间等统计信息（但不直接显示，等待audio-processing事件）
        });
        eventUnlisteners.push(statsUnlisten);

        // 监听音频处理状态事件（实时更新延迟时间等）
        const audioUnlisten = await listen("audio-processing", (event) => {
            const audioStatus = event.payload;
            delay_time.value = Math.round(audioStatus.delay_time);
            infer_time.value = Math.round(audioStatus.inference_time);
        });
        eventUnlisteners.push(audioUnlisten);

        // 监听设备更新事件
        const devicesUnlisten = await listen("devices-updated", (event) => {
            const deviceInfo = event.payload;
            if (deviceInfo.input_devices) {
                input_devices.value = deviceInfo.input_devices.map(
                    (d) => d.name,
                );
            }
            if (deviceInfo.output_devices) {
                output_devices.value = deviceInfo.output_devices.map(
                    (d) => d.name,
                );
            }
        });
        eventUnlisteners.push(devicesUnlisten);

        // 监听配置更新事件
        const configUnlisten = await listen("config-updated", (event) => {
            console.log("配置更新:", event.payload);
            Object.assign(values, event.payload);
        });
        eventUnlisteners.push(configUnlisten);

        // 监听错误事件
        const errorUnlisten = await listen("error", (event) => {
            const errorInfo = event.payload;
            console.error("核心错误:", errorInfo);
            alert(`错误: ${errorInfo.message}`);
        });
        eventUnlisteners.push(errorUnlisten);

        // 监听日志事件（可选）
        const logUnlisten = await listen("log", (event) => {
            const logInfo = event.payload;
            console.log(`[${logInfo.level}] ${logInfo.message}`);
        });
        eventUnlisteners.push(logUnlisten);

        console.log("事件监听器设置完成");
    } catch (error) {
        console.error("设置事件监听器失败:", error);
    }
};

// 清理事件监听器
const cleanupEventListeners = () => {
    eventUnlisteners.forEach((unlisten) => {
        if (typeof unlisten === "function") {
            unlisten();
        }
    });
    eventUnlisteners = [];
};

// 初始化应用 - 对应 Python 的 load()
const initializeApp = async () => {
    try {
        // 设置事件监听器
        await setupEventListeners();

        // 初始化应用（这会启动事件系统）
        await invoke("initialize_app");

        // 加载配置
        const config = await invoke("load_config");
        Object.assign(values, config);

        // 初始化设备列表
        await handleEvent("reload_devices");

        appInitialized.value = true;
        console.log("应用初始化完成，使用事件驱动模式");
    } catch (error) {
        console.error("初始化失败:", error);
        initializationError.value = error.toString();
    }
};

// 生命周期
onMounted(() => {
    initializeApp();
});

onUnmounted(() => {
    cleanupEventListeners();
});
</script>

<style scoped>
.rvc-app {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
    font-family: Arial, sans-serif;
}

.app-header {
    text-align: center;
    margin-bottom: 30px;
}

.app-header h1 {
    color: #333;
    margin: 0;
}

.app-main.disabled {
    opacity: 0.6;
    pointer-events: none;
}

.init-status {
    text-align: center;
    padding: 50px;
    font-size: 18px;
}

.loading {
    color: #666;
}

.error {
    color: #e74c3c;
}

section {
    background: #f8f9fa;
    padding: 20px;
    margin-bottom: 20px;
    border-radius: 8px;
    border: 1px solid #e9ecef;
}

section h2 {
    margin-top: 0;
    color: #495057;
    border-bottom: 2px solid #007bff;
    padding-bottom: 10px;
}

.form-group {
    margin-bottom: 15px;
}

.form-row {
    display: flex;
    gap: 15px;
    align-items: center;
    flex-wrap: wrap;
}

.file-input-group {
    display: flex;
    gap: 10px;
}

.file-input-group input {
    flex: 1;
    padding: 8px 12px;
    border: 1px solid #ced4da;
    border-radius: 4px;
    background: #fff;
}

.file-btn {
    padding: 8px 16px;
    background: #007bff;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
}

.file-btn:hover {
    background: #0056b3;
}

label {
    display: block;
    margin-bottom: 5px;
    font-weight: bold;
    color: #495057;
}

.checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 0;
}

select,
input[type="text"] {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid #ced4da;
    border-radius: 4px;
    background: #fff;
}

.reload-btn {
    padding: 8px 16px;
    background: #28a745;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    margin-bottom: 10px;
}

.reload-btn:hover {
    background: #218838;
}

.radio-group {
    display: flex;
    gap: 15px;
    flex-wrap: wrap;
    margin-bottom: 10px;
}

.radio-group label {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-bottom: 0;
    font-weight: normal;
}

.settings-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
    gap: 20px;
    margin-bottom: 20px;
}

.slider-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
}

.slider-group input[type="range"] {
    width: 100%;
}

.checkbox-group {
    display: flex;
    gap: 20px;
    flex-wrap: wrap;
}

.checkbox-group label {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 0;
    font-weight: normal;
}

.control-section {
    background: #e9ecef;
}

.control-buttons {
    display: flex;
    gap: 15px;
    margin-bottom: 20px;
}

.start-btn,
.stop-btn {
    padding: 12px 24px;
    font-size: 16px;
    font-weight: bold;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: background-color 0.3s;
}

.start-btn {
    background: #28a745;
    color: white;
}

.start-btn:hover:not(:disabled) {
    background: #218838;
}

.start-btn:disabled {
    background: #6c757d;
    cursor: not-allowed;
}

.stop-btn {
    background: #dc3545;
    color: white;
}

.stop-btn:hover:not(:disabled) {
    background: #c82333;
}

.stop-btn:disabled {
    background: #6c757d;
    cursor: not-allowed;
}

.status-display {
    display: flex;
    gap: 30px;
    flex-wrap: wrap;
}

.status-item {
    background: #fff;
    padding: 10px 15px;
    border-radius: 4px;
    border: 1px solid #dee2e6;
    font-family: monospace;
}
</style>
