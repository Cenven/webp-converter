<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

const STORAGE_KEY_LANG = "webp-converter-lang";
const STORAGE_KEY_OPTIONS = "webp-converter-options";
const STORAGE_KEY_DELETE_ORIGINAL = "webp-converter-delete-original";

// cwebp 全部参数，与后端 ConvertOptions 对应
interface ConvertOptions {
  quality: number;
  lossless: boolean;
  method: number;
  preset: string;
  lossless_level: number;
  near_lossless: number;
  alpha_quality: number;
  alpha_compression: number;
  alpha_filter: string;
  exact: boolean;
  noalpha: boolean;
  target_size: number;
  target_psnr: number;
  pass: number;
  autofilter: boolean;
  jpeg_like: boolean;
  filter_strength: number;
  filter_sharpness: number;
  strong_filter: boolean;
  sharp_yuv: boolean;
  sns_strength: number;
  segments: number;
  partition_limit: number;
  qmin: number;
  qmax: number;
  preprocessing: number;
  hint: string;
  mt: boolean;
  low_memory: boolean;
}

interface FileTask {
  path: string;
  status: "pending" | "ok" | "fail";
  originalSize?: number;
  newSize?: number;
  error?: string;
  progressPercent?: number;
}

function formatBytes(n: number): string {
  if (n < 1024) return n + " B";
  if (n < 1024 * 1024) return (n / 1024).toFixed(2) + " KB";
  return (n / (1024 * 1024)).toFixed(2) + " MB";
}

function reductionPercent(orig: number, now: number): number {
  if (orig <= 0) return 0;
  return Math.round((1 - now / orig) * 100);
}

function fileName(path: string): string {
  const sep = path.includes("/") ? "/" : "\\";
  const parts = path.split(sep);
  return parts[parts.length - 1] ?? path;
}

const options = ref<ConvertOptions>({
  quality: 75,
  lossless: false,
  method: 4,
  preset: "default",
  lossless_level: 6,
  near_lossless: 100,
  alpha_quality: 100,
  alpha_compression: 1,
  alpha_filter: "fast",
  exact: false,
  noalpha: false,
  target_size: 0,
  target_psnr: 0,
  pass: 1,
  autofilter: false,
  jpeg_like: false,
  filter_strength: 60,
  filter_sharpness: 0,
  strong_filter: true,
  sharp_yuv: false,
  sns_strength: 50,
  segments: 4,
  partition_limit: 0,
  qmin: 0,
  qmax: 100,
  preprocessing: 0,
  hint: "default",
  mt: true,
  low_memory: false,
});

/** 转换成功后删除原文件，独立于 cwebp 配置，默认开启 */
const deleteOriginal = ref(true);

const expandedSections = ref<Record<string, boolean>>({
  basic: true,
  alpha: false,
  lossy: false,
  advanced: false,
  other: false,
});

const lang = ref<"zh" | "en">("en");

const filePaths = ref<string[]>([]);
const fileList = ref<FileTask[]>([]);
const isDragging = ref(false);
const isConverting = ref(false);
const convertResult = ref<{ ok: number; fail: number; errors: string[] } | null>(null);

const t = computed(() => (lang.value === "zh" ? i18nZh : i18nEn));

const presetOptions = [
  { value: "default", labelZh: "默认", labelEn: "Default" },
  { value: "photo", labelZh: "照片", labelEn: "Photo" },
  { value: "picture", labelZh: "图片", labelEn: "Picture" },
  { value: "drawing", labelZh: "绘图", labelEn: "Drawing" },
  { value: "icon", labelZh: "图标", labelEn: "Icon" },
  { value: "text", labelZh: "文字", labelEn: "Text" },
];

const alphaFilterOptions = [
  { value: "none", labelZh: "无", labelEn: "None" },
  { value: "fast", labelZh: "快速", labelEn: "Fast" },
  { value: "best", labelZh: "最佳", labelEn: "Best" },
];

const hintOptions = [
  { value: "default", labelZh: "默认", labelEn: "Default" },
  { value: "photo", labelZh: "照片", labelEn: "Photo" },
  { value: "picture", labelZh: "图片", labelEn: "Picture" },
  { value: "graph", labelZh: "图形", labelEn: "Graph" },
];

const i18nZh = {
  title: "WebP 转换工具",
  dragHint: "拖拽图片或目录到此转换",
  selectDir: "选择目录",
  selectFile: "选择文件",
  converting: "转换中…",
  convertDone: "转换完成",
  successCount: "成功",
  failCount: "失败",
  resetDefaults: "恢复默认",
  sectionBasic: "基本选项",
  sectionAlpha: "Alpha 选项",
  sectionLossy: "有损选项",
  sectionAdvanced: "高级选项",
  sectionOther: "其他",
  quality: "质量 -q",
  qualityHint: "0-100，默认 75",
  lossless: "无损 -lossless",
  method: "方法 -m",
  methodHint: "0=快 6=慢，默认 4",
  preset: "预设 -preset",
  losslessLevel: "无损级别 -z",
  losslessLevelHint: "0-9，仅无损时生效",
  nearLossless: "近乎无损 -near_lossless",
  nearLosslessHint: "0-100，100=关",
  alphaQuality: "Alpha 质量 -alpha_q",
  alphaMethod: "Alpha 方法 -alpha_method",
  alphaFilter: "Alpha 过滤 -alpha_filter",
  exact: "精确 -exact",
  noalpha: "丢弃透明 -noalpha",
  targetSize: "目标大小 -size",
  targetPsnr: "目标 PSNR -psnr",
  pass: "分析遍数 -pass",
  autofilter: "自动过滤 -af",
  jpegLike: "JPEG 风格 -jpeg_like",
  filterStrength: "滤波强度 -f",
  filterSharpness: "锐度 -sharpness",
  strongFilter: "强过滤 -strong",
  sharpYuv: "精确 YUV -sharp_yuv",
  snsStrength: "噪声整形 -sns",
  segments: "分区数 -segments",
  partitionLimit: "分区限制 -partition_limit",
  qmin: "质量下限 qmin",
  qmax: "质量上限 qmax",
  preprocessing: "预处理 -pre",
  hint: "提示 -hint",
  mt: "多线程 -mt",
  lowMemory: "低内存 -low_memory",
  deleteOriginal: "转换成功后删除原文件",
  clearList: "清除列表",
  fileCount: "文件数量",
  convertingFile: "转换中…",
  errNoImagesInDir: "拖入的目录内未找到 png/jpg/jpeg，或无法读取子目录（请检查路径与权限）",
  errNoImages: "未找到可转换的图片（仅支持 png/jpg/jpeg；若拖入的是文件夹，请确认路径格式与权限）",
};

const i18nEn = {
  title: "WebP Converter",
  dragHint: "Drag images or folder here",
  selectDir: "Select Directory",
  selectFile: "Select File",
  converting: "Converting…",
  convertDone: "Done",
  successCount: "Success",
  failCount: "Failed",
  resetDefaults: "Reset Defaults",
  sectionBasic: "Basic",
  sectionAlpha: "Alpha",
  sectionLossy: "Lossy",
  sectionAdvanced: "Advanced",
  sectionOther: "Other",
  quality: "Quality -q",
  qualityHint: "0-100, default 75",
  lossless: "Lossless -lossless",
  method: "Method -m",
  methodHint: "0=fast 6=slow, default 4",
  preset: "Preset -preset",
  losslessLevel: "Lossless level -z",
  losslessLevelHint: "0-9, lossless only",
  nearLossless: "Near lossless",
  nearLosslessHint: "0-100, 100=off",
  alphaQuality: "Alpha quality -alpha_q",
  alphaMethod: "Alpha method -alpha_method",
  alphaFilter: "Alpha filter -alpha_filter",
  exact: "Exact -exact",
  noalpha: "Drop alpha -noalpha",
  targetSize: "Target size -size",
  targetPsnr: "Target PSNR -psnr",
  pass: "Pass -pass",
  autofilter: "Auto filter -af",
  jpegLike: "JPEG-like -jpeg_like",
  filterStrength: "Filter strength -f",
  filterSharpness: "Sharpness -sharpness",
  strongFilter: "Strong filter -strong",
  sharpYuv: "Sharp YUV -sharp_yuv",
  snsStrength: "SNS -sns",
  segments: "Segments -segments",
  partitionLimit: "Partition limit",
  qmin: "Q min",
  qmax: "Q max",
  preprocessing: "Preprocessing -pre",
  hint: "Hint -hint",
  mt: "Multi-thread -mt",
  lowMemory: "Low memory -low_memory",
  deleteOriginal: "Delete original after success",
  clearList: "Clear list",
  fileCount: "Files",
  convertingFile: "Converting…",
  errNoImagesInDir: "No png/jpg/jpeg found in the dropped directory, or unable to read subdirectory (check path and permissions).",
  errNoImages: "No convertible images found (only png/jpg/jpeg supported; if you dropped a folder, check path format and permissions).",
};

function getErrorMessage(err: string): string {
  if (err === "ERR_NO_IMAGES_IN_DIR") return t.value.errNoImagesInDir;
  if (err === "ERR_NO_IMAGES") return t.value.errNoImages;
  return err;
}

function toggleSection(key: string) {
  expandedSections.value[key] = !expandedSections.value[key];
}

async function loadDefaults() {
  try {
    const def = await invoke<ConvertOptions>("get_default_options");
    options.value = def;
  } catch {
    // 使用本地默认值
  }
}

function loadSavedLang() {
  try {
    const s = localStorage.getItem(STORAGE_KEY_LANG);
    if (s === "zh" || s === "en") lang.value = s;
  } catch {}
}

function saveLang() {
  try {
    localStorage.setItem(STORAGE_KEY_LANG, lang.value);
  } catch {}
}

function loadSavedOptions(): boolean {
  try {
    const s = localStorage.getItem(STORAGE_KEY_OPTIONS);
    if (!s) return false;
    const parsed = JSON.parse(s) as Partial<ConvertOptions>;
    if (parsed && typeof parsed === "object") {
      options.value = { ...options.value, ...parsed };
      return true;
    }
  } catch {}
  return false;
}

function saveOptions() {
  try {
    localStorage.setItem(STORAGE_KEY_OPTIONS, JSON.stringify(options.value));
  } catch {}
}

function loadSavedDeleteOriginal() {
  try {
    const s = localStorage.getItem(STORAGE_KEY_DELETE_ORIGINAL);
    if (s === "true" || s === "false") deleteOriginal.value = s === "true";
  } catch {}
}

function saveDeleteOriginal() {
  try {
    localStorage.setItem(STORAGE_KEY_DELETE_ORIGINAL, String(deleteOriginal.value));
  } catch {}
}

async function resetDefaults() {
  await loadDefaults();
  saveOptions();
  saveDeleteOriginal();
}

async function onSelectDir() {
  const selected = await open({ directory: true, multiple: false });
  if (selected) {
    filePaths.value = Array.isArray(selected) ? selected : [selected];
    await startConvert();
  }
}

async function onSelectFile() {
  const selected = await open({
    multiple: true,
    filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg"] }],
  });
  if (selected) {
    filePaths.value = Array.isArray(selected) ? selected : [selected];
    await startConvert();
  }
}

async function startConvert() {
  if (filePaths.value.length === 0) return;
  isConverting.value = true;
  convertResult.value = null;
  fileList.value = [];
  const pathsToConvert = [...filePaths.value];
  filePaths.value = [];
  try {
    const opts = { ...options.value };
    opts.quality = Number(opts.quality);
    opts.method = Number(opts.method);
    opts.lossless_level = Number(opts.lossless_level);
    opts.near_lossless = Number(opts.near_lossless);
    opts.alpha_quality = Number(opts.alpha_quality);
    opts.alpha_compression = Number(opts.alpha_compression);
    opts.target_size = Number(opts.target_size);
    opts.target_psnr = Number(opts.target_psnr);
    opts.pass = Number(opts.pass);
    opts.filter_strength = Number(opts.filter_strength);
    opts.filter_sharpness = Number(opts.filter_sharpness);
    opts.sns_strength = Number(opts.sns_strength);
    opts.segments = Number(opts.segments);
    opts.partition_limit = Number(opts.partition_limit);
    opts.qmin = Number(opts.qmin);
    opts.qmax = Number(opts.qmax);
    opts.preprocessing = Number(opts.preprocessing);

    console.log("[webp] 前端调用 convert_to_webp paths:", pathsToConvert);
    await invoke("convert_to_webp", {
      payload: {
        paths: pathsToConvert,
        options: opts,
        delete_original: deleteOriginal.value,
      },
    });
  } catch (e) {
    console.error("[webp] convert_to_webp 失败:", e);
    convertResult.value = { ok: 0, fail: pathsToConvert.length, errors: [String(e)] };
    isConverting.value = false;
  }
}

function clearList() {
  fileList.value = [];
  convertResult.value = null;
}

function onDrop(e: DragEvent) {
  e.preventDefault();
  isDragging.value = false;
}

function onDragOver(e: DragEvent) {
  e.preventDefault();
  isDragging.value = true;
}

function onDragLeave() {
  isDragging.value = false;
}

onMounted(async () => {
  loadSavedLang();
  await loadDefaults();
  loadSavedOptions();
  loadSavedDeleteOriginal();
  watch(lang, () => saveLang());
  watch(
    options,
    () => saveOptions(),
    { deep: true }
  );
  watch(deleteOriginal, () => saveDeleteOriginal());
  listen<string[]>("convert-file-list", (event) => {
    const paths = event.payload ?? [];
    fileList.value = paths.map((p) => ({ path: p, status: "pending" as const }));
  });
  listen<{
    path: string;
    status: string;
    original_size?: number;
    new_size?: number;
    error?: string;
    progress_percent?: number;
  }>("convert-file-progress", (event) => {
    const p = event.payload;
    if (!p) return;
    const idx = fileList.value.findIndex((t) => t.path === p.path);
    if (idx >= 0) {
      fileList.value[idx] = {
        path: p.path,
        status: p.status as "ok" | "fail" | "pending",
        originalSize: p.original_size,
        newSize: p.new_size,
        error: p.error,
        progressPercent: p.progress_percent,
      };
    }
  });
  listen<{ ok: number; fail: number; errors: string[] }>("convert-done", (event) => {
    const result = event.payload;
    if (result) {
      convertResult.value = result;
    }
    isConverting.value = false;
  });
});

listen<string[]>("drop-paths", (event) => {
  console.log("[webp] 前端收到 drop-paths:", event.payload?.length, event.payload);
  if (event.payload?.length) {
    filePaths.value = event.payload;
    startConvert();
  }
}).catch(() => {});
</script>

<template>
  <div class="layout">
    <main class="main">
      <section
        class="drop-zone"
        :class="{ dragging: isDragging, 'drop-zone--has-list': fileList.length > 0 }"
        @drop="onDrop"
        @dragover="onDragOver"
        @dragleave="onDragLeave"
      >
        <template v-if="fileList.length === 0">
          <p class="drop-hint">{{ t.dragHint }}</p>
          <div class="buttons">
            <button type="button" @click="onSelectDir">{{ t.selectDir }}</button>
            <button type="button" @click="onSelectFile">{{ t.selectFile }}</button>
          </div>
          <p v-if="isConverting" class="status">{{ t.converting }}</p>
          <p v-else-if="convertResult?.errors?.length" class="status status--error">
            {{ convertResult.errors.map(getErrorMessage).join(" ") }}
          </p>
        </template>
        <template v-else>
          <ul class="file-list">
            <li
              v-for="(task, i) in fileList"
              :key="task.path + String(i)"
              class="file-item"
              :class="{ 'file-item--fail': task.status === 'fail' }"
            >
              <span class="file-icon" :class="'file-icon--' + task.status" :title="task.status === 'ok' ? '' : task.status === 'fail' ? (lang === 'zh' ? '转换失败' : 'Failed') : (lang === 'zh' ? '转换中' : 'Converting')">
                {{ task.status === "ok" ? "✓" : task.status === "fail" ? "✗" : "⋯" }}
              </span>
              <div class="file-info">
                <div class="file-info-row">
                  <span class="file-name" :title="task.path">{{ fileName(task.path) }}</span>
                  <span v-if="task.status === 'pending'" class="file-status file-status--pending">
                    {{ task.progressPercent != null ? `${task.progressPercent}%` : t.convertingFile }}
                  </span>
                  <template v-else-if="task.status === 'ok' && task.originalSize != null && task.newSize != null">
                    <span class="file-status file-status--done">100%</span>
                  </template>
                  <span v-else-if="task.status === 'fail'" class="file-status file-status--fail">✗</span>
                </div>
                <template v-if="task.status === 'ok' && task.originalSize != null && task.newSize != null">
                  <span class="file-size">
                    {{ formatBytes(task.originalSize) }} → {{ formatBytes(task.newSize) }}
                    <span class="file-reduction">{{ reductionPercent(task.originalSize, task.newSize) }}% ↓</span>
                  </span>
                </template>
                <span v-else-if="task.status === 'fail'" class="file-error">{{ task.error }}</span>
              </div>
            </li>
          </ul>
          <div class="file-list-footer">
            <span class="file-list-count">{{ t.fileCount }}: {{ fileList.length }}</span>
            <button type="button" class="file-list-clear" @click="clearList">{{ t.clearList }}</button>
          </div>
        </template>
      </section>

      <aside class="sidebar">
        <div class="sidebar-inner">
          <div class="sidebar-toolbar">
            <button type="button" class="reset-btn" @click="resetDefaults">{{ t.resetDefaults }}</button>
            <label class="delete-original-row">
              <input v-model="deleteOriginal" type="checkbox" />
              <span>{{ t.deleteOriginal }}</span>
            </label>
          </div>

          <!-- 基本选项 -->
        <div class="section">
          <button type="button" class="section-header" @click="toggleSection('basic')">
            <span>{{ t.sectionBasic }}</span>
            <span class="toggle-icon">{{ expandedSections.basic ? "−" : "+" }}</span>
          </button>
          <div v-show="expandedSections.basic" class="section-body">
            <div class="field">
              <label :title="t.qualityHint">{{ t.quality }}</label>
              <input v-model.number="options.quality" type="number" min="0" max="100" step="1" />
            </div>
            <div class="field">
              <label>{{ t.lossless }}</label>
              <label class="checkbox-row">
                <input v-model="options.lossless" type="checkbox" />
                <span>{{ options.lossless ? (lang === "zh" ? "开" : "On") : (lang === "zh" ? "关" : "Off") }}</span>
              </label>
            </div>
            <div class="field">
              <label :title="t.methodHint">{{ t.method }}</label>
              <input v-model.number="options.method" type="number" min="0" max="6" step="1" />
            </div>
            <div class="field">
              <label>{{ t.preset }}</label>
              <select v-model="options.preset">
                <option v-for="o in presetOptions" :key="o.value" :value="o.value">
                  {{ lang === "zh" ? o.labelZh : o.labelEn }}
                </option>
              </select>
            </div>
            <div class="field">
              <label :title="t.losslessLevelHint">{{ t.losslessLevel }}</label>
              <input v-model.number="options.lossless_level" type="number" min="0" max="9" step="1" />
            </div>
            <div class="field">
              <label :title="t.nearLosslessHint">{{ t.nearLossless }}</label>
              <input v-model.number="options.near_lossless" type="number" min="0" max="100" step="1" />
            </div>
          </div>
        </div>

        <!-- Alpha 选项 -->
        <div class="section">
          <button type="button" class="section-header" @click="toggleSection('alpha')">
            <span>{{ t.sectionAlpha }}</span>
            <span class="toggle-icon">{{ expandedSections.alpha ? "−" : "+" }}</span>
          </button>
          <div v-show="expandedSections.alpha" class="section-body">
            <div class="field">
              <label>{{ t.alphaQuality }}</label>
              <input v-model.number="options.alpha_quality" type="number" min="0" max="100" step="1" />
            </div>
            <div class="field">
              <label>{{ t.alphaMethod }}</label>
              <input v-model.number="options.alpha_compression" type="number" min="0" max="1" step="1" />
            </div>
            <div class="field">
              <label>{{ t.alphaFilter }}</label>
              <select v-model="options.alpha_filter">
                <option v-for="o in alphaFilterOptions" :key="o.value" :value="o.value">
                  {{ lang === "zh" ? o.labelZh : o.labelEn }}
                </option>
              </select>
            </div>
            <div class="field">
              <label>{{ t.exact }}</label>
              <label class="checkbox-row">
                <input v-model="options.exact" type="checkbox" />
              </label>
            </div>
            <div class="field">
              <label>{{ t.noalpha }}</label>
              <label class="checkbox-row">
                <input v-model="options.noalpha" type="checkbox" />
              </label>
            </div>
          </div>
        </div>

        <!-- 有损选项 -->
        <div class="section">
          <button type="button" class="section-header" @click="toggleSection('lossy')">
            <span>{{ t.sectionLossy }}</span>
            <span class="toggle-icon">{{ expandedSections.lossy ? "−" : "+" }}</span>
          </button>
          <div v-show="expandedSections.lossy" class="section-body">
            <div class="field">
              <label>{{ t.targetSize }}</label>
              <input v-model.number="options.target_size" type="number" min="0" step="1" />
            </div>
            <div class="field">
              <label>{{ t.targetPsnr }}</label>
              <input v-model.number="options.target_psnr" type="number" min="0" step="0.1" />
            </div>
            <div class="field">
              <label>{{ t.pass }}</label>
              <input v-model.number="options.pass" type="number" min="1" max="10" step="1" />
            </div>
            <div class="field">
              <label>{{ t.autofilter }}</label>
              <label class="checkbox-row">
                <input v-model="options.autofilter" type="checkbox" />
              </label>
            </div>
            <div class="field">
              <label>{{ t.jpegLike }}</label>
              <label class="checkbox-row">
                <input v-model="options.jpeg_like" type="checkbox" />
              </label>
            </div>
          </div>
        </div>

        <!-- 高级选项 -->
        <div class="section">
          <button type="button" class="section-header" @click="toggleSection('advanced')">
            <span>{{ t.sectionAdvanced }}</span>
            <span class="toggle-icon">{{ expandedSections.advanced ? "−" : "+" }}</span>
          </button>
          <div v-show="expandedSections.advanced" class="section-body">
            <div class="field">
              <label>{{ t.filterStrength }}</label>
              <input v-model.number="options.filter_strength" type="number" min="0" max="100" step="1" />
            </div>
            <div class="field">
              <label>{{ t.filterSharpness }}</label>
              <input v-model.number="options.filter_sharpness" type="number" min="0" max="7" step="1" />
            </div>
            <div class="field">
              <label>{{ t.strongFilter }}</label>
              <label class="checkbox-row">
                <input v-model="options.strong_filter" type="checkbox" />
              </label>
            </div>
            <div class="field">
              <label>{{ t.sharpYuv }}</label>
              <label class="checkbox-row">
                <input v-model="options.sharp_yuv" type="checkbox" />
              </label>
            </div>
            <div class="field">
              <label>{{ t.snsStrength }}</label>
              <input v-model.number="options.sns_strength" type="number" min="0" max="100" step="1" />
            </div>
            <div class="field">
              <label>{{ t.segments }}</label>
              <input v-model.number="options.segments" type="number" min="1" max="4" step="1" />
            </div>
            <div class="field">
              <label>{{ t.partitionLimit }}</label>
              <input v-model.number="options.partition_limit" type="number" min="0" max="100" step="1" />
            </div>
            <div class="field">
              <label>{{ t.qmin }}</label>
              <input v-model.number="options.qmin" type="number" min="0" max="100" step="1" />
            </div>
            <div class="field">
              <label>{{ t.qmax }}</label>
              <input v-model.number="options.qmax" type="number" min="0" max="100" step="1" />
            </div>
            <div class="field">
              <label>{{ t.preprocessing }}</label>
              <input v-model.number="options.preprocessing" type="number" min="0" max="7" step="1" />
            </div>
            <div class="field">
              <label>{{ t.hint }}</label>
              <select v-model="options.hint">
                <option v-for="o in hintOptions" :key="o.value" :value="o.value">
                  {{ lang === "zh" ? o.labelZh : o.labelEn }}
                </option>
              </select>
            </div>
          </div>
        </div>

        <!-- 其他 -->
        <div class="section">
          <button type="button" class="section-header" @click="toggleSection('other')">
            <span>{{ t.sectionOther }}</span>
            <span class="toggle-icon">{{ expandedSections.other ? "−" : "+" }}</span>
          </button>
          <div v-show="expandedSections.other" class="section-body">
            <div class="field">
              <label>{{ t.mt }}</label>
              <label class="checkbox-row">
                <input v-model="options.mt" type="checkbox" />
              </label>
            </div>
            <div class="field">
              <label>{{ t.lowMemory }}</label>
              <label class="checkbox-row">
                <input v-model="options.low_memory" type="checkbox" />
              </label>
            </div>
          </div>
        </div>
        </div>
      </aside>
    </main>

    <footer class="footer">
      <select v-model="lang" class="lang-select">
        <option value="zh">中文简体</option>
        <option value="en">English</option>
      </select>
    </footer>
  </div>
</template>

<style scoped>
.layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
}

.main {
  display: flex;
  flex: 1;
  min-height: 0;
}

.drop-zone {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  border-right: 1px solid var(--border-color);
  margin: 12px;
  border: 2px dashed var(--drop-zone-border);
  border-radius: 12px;
  background: var(--drop-zone-bg);
  padding: 24px;
}

.drop-zone.dragging {
  border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 10%, var(--drop-zone-bg));
}

.drop-zone--has-list {
  padding: 8px;
  align-items: stretch;
  justify-content: flex-start;
  gap: 4px;
  border-style: solid;
}

.file-list {
  list-style: none;
  margin: 0;
  padding: 0 8px;
  width: 100%;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  scrollbar-width: thin;
  scrollbar-color: var(--border-color) transparent;
}

.file-list::-webkit-scrollbar {
  width: 6px;
}

.file-list::-webkit-scrollbar-track {
  background: transparent;
}

.file-list::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 3px;
}

.file-list::-webkit-scrollbar-thumb:hover {
  background: var(--text-secondary);
}

.file-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 8px 4px;
  border-bottom: 1px solid var(--border-color);
  transition: background 0.15s ease;
}

.file-item:last-child {
  border-bottom: none;
}

.file-item--fail {
  background: color-mix(in srgb, rgba(200, 0, 0, 0.06), var(--drop-zone-bg));
}

.file-icon {
  flex-shrink: 0;
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  font-weight: bold;
  border-radius: 50%;
  line-height: 1;
}

.file-icon--pending {
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--text-secondary) 15%, transparent);
  animation: file-icon-pulse 1.2s ease-in-out infinite;
}

@keyframes file-icon-pulse {
  50% { opacity: 0.6; }
}

.file-icon--ok {
  color: #0a0;
  background: color-mix(in srgb, #0a0 18%, transparent);
}

.file-icon--fail {
  color: #c00;
  background: color-mix(in srgb, #c00 18%, transparent);
}

.file-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.file-info-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.file-name {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary, inherit);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-status {
  flex-shrink: 0;
  font-size: 12px;
}

.file-status--pending {
  color: var(--text-secondary);
}

.file-status--done {
  color: var(--accent);
  font-weight: 500;
}

.file-status--fail {
  color: #c00;
  font-weight: 600;
}

.file-size {
  font-size: 12px;
  color: var(--text-secondary);
}

.file-reduction {
  color: var(--accent);
  margin-left: 4px;
  font-weight: 500;
}

.file-error {
  font-size: 12px;
  font-weight: 600;
  color: #c00;
  word-break: break-word;
}

.file-list-footer {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 4px 8px;
  background: var(--drop-zone-bg);
  font-size: 12px;
}

.file-list-count {
  color: var(--text-secondary);
}

.file-list-clear {
  padding: 2px 8px;
  font-size: 12px;
}

.drop-hint {
  color: var(--text-secondary);
  font-size: 14px;
  text-align: center;
}

.buttons {
  display: flex;
  gap: 12px;
}

.status {
  font-size: 13px;
  color: var(--text-secondary);
}

.status--error {
  color: #c00;
  font-weight: 500;
  word-break: break-word;
  max-width: 100%;
}

.sidebar {
  width: 320px;
  flex-shrink: 0;
  min-height: 0;
  padding: 12px;
  border-left: 1px solid var(--border-color);
  overflow-y: auto;
}

.sidebar-inner {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.sidebar-toolbar {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 4px;
}

.reset-btn {
  align-self: flex-start;
  padding: 6px 12px;
  font-size: 12px;
}

.delete-original-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  cursor: pointer;
  color: var(--text-secondary);
}

.section {
  border: 1px solid var(--border-color);
  border-radius: 8px;
  overflow: hidden;
}

.section-header {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 12px;
  font-size: 13px;
  font-weight: 500;
  background: var(--drop-zone-bg);
  border: none;
  cursor: pointer;
  text-align: left;
}

.section-header:hover {
  background: color-mix(in srgb, var(--accent) 8%, var(--drop-zone-bg));
}

.toggle-icon {
  font-size: 14px;
  opacity: 0.8;
}

.section-body {
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  border-top: 1px solid var(--border-color);
}

.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.field label {
  font-size: 12px;
  color: var(--text-secondary);
}

.field input[type="number"],
.field select {
  padding: 4px 8px;
  font-size: 13px;
}

.checkbox-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding: 4px 12px;
  border-top: 1px solid var(--border-color);
  flex-shrink: 0;
  font-size: 12px;
}

.lang-select {
  width: 110px;
  font-size: 12px;
  padding: 4px 6px;
}
</style>
