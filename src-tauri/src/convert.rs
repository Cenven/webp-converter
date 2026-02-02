// Copyright (c) 2025 WebP Tool Contributors. All rights reserved.
// SPDX-License-Identifier: MIT

use libwebp_sys::{
    WebPConfig, WebPConfigInitInternal, WebPConfigLosslessPreset, WebPImageHint,
    WebPPictureFree, WebPPictureImportRGBA, WebPPictureImportRGB, WebPPictureInit, WebPEncode,
    WebPMemoryWriter, WebPMemoryWriterClear, WebPMemoryWriterInit, WebPMemoryWrite,
    WebPPreset, WEBP_ENCODER_ABI_VERSION,
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::fs;
use std::os::raw::c_int;
use std::path::Path;
use tauri::{Emitter, Manager};
use walkdir::WalkDir;

const SUPPORTED_EXT: [&str; 3] = ["png", "jpg", "jpeg"];

thread_local! {
    /// 编码进度回调，由 progress_hook 从 C 回调时调用
    static PROGRESS_CB: RefCell<Option<Box<dyn Fn(u32)>>> = RefCell::new(None);
}

unsafe extern "C" fn progress_hook(percent: c_int, _picture: *const libwebp_sys::WebPPicture) -> c_int {
    PROGRESS_CB.with(|cb| {
        if let Some(f) = cb.borrow().as_ref() {
            f(percent.clamp(0, 100) as u32);
        }
    });
    1 // 1 = 继续编码，0 = 中止
}

/// cwebp 全部参数，与官方文档一一对应
/// 参考: https://developers.google.com/speed/webp/docs/cwebp?hl=zh-cn
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConvertOptions {
    // === 基本选项 ===
    /// -q: 压缩系数 0-100，默认 75
    #[serde(default = "default_quality")]
    pub quality: f32,
    /// -lossless: 无损编码
    #[serde(default)]
    pub lossless: bool,
    /// -m: 压缩方法 0-6，默认 4
    #[serde(default = "default_method")]
    pub method: i32,
    /// -preset: default/photo/picture/drawing/icon/text
    #[serde(default = "default_preset")]
    pub preset: String,
    /// -z: 无损预设级别 0-9，仅 lossless 时生效
    #[serde(default = "default_lossless_level")]
    pub lossless_level: i32,
    /// -near_lossless: 近乎无损 0-100，100=关闭
    #[serde(default = "default_near_lossless")]
    pub near_lossless: i32,

    // === Alpha 选项 ===
    /// -alpha_q: Alpha 压缩质量 0-100，默认 100
    #[serde(default = "default_alpha_quality")]
    pub alpha_quality: i32,
    /// -alpha_method: 0=不压缩 1=无损压缩，默认 1
    #[serde(default = "default_alpha_method")]
    pub alpha_compression: i32,
    /// -alpha_filter: none/fast/best
    #[serde(default = "default_alpha_filter")]
    pub alpha_filter: String,
    /// -exact: 保留透明区域 RGB 值
    #[serde(default)]
    pub exact: bool,
    /// -noalpha: 丢弃透明通道
    #[serde(default)]
    pub noalpha: bool,

    // === 有损选项 ===
    /// -size: 目标大小（字节）
    #[serde(default)]
    pub target_size: i32,
    /// -psnr: 目标 PSNR (dB)
    #[serde(default)]
    pub target_psnr: f32,
    /// -pass: 分析遍数 1-10
    #[serde(default = "default_pass")]
    pub pass: i32,
    /// -af: 自动过滤
    #[serde(default)]
    pub autofilter: bool,
    /// -jpeg_like: 匹配 JPEG 大小
    #[serde(default)]
    pub jpeg_like: bool,

    // === 高级选项 ===
    /// -f: 去块滤波强度 0-100，默认 60
    #[serde(default = "default_filter_strength")]
    pub filter_strength: i32,
    /// -sharpness: 锐度 0-7，默认 0
    #[serde(default)]
    pub filter_sharpness: i32,
    /// -strong/-nostrong: 强过滤
    #[serde(default = "default_strong_filter")]
    pub strong_filter: bool,
    /// -sharp_yuv: 更精确 RGB->YUV
    #[serde(default)]
    pub sharp_yuv: bool,
    /// -sns: 空间噪声整形 0-100，默认 50
    #[serde(default = "default_sns_strength")]
    pub sns_strength: i32,
    /// -segments: 分区数 1-4，默认 4
    #[serde(default = "default_segments")]
    pub segments: i32,
    /// -partition_limit: 0-100，默认 0
    #[serde(default)]
    pub partition_limit: i32,
    /// -qrange: qmin qmax
    #[serde(default)]
    pub qmin: i32,
    #[serde(default = "default_qmax")]
    pub qmax: i32,
    /// -pre: 预处理
    #[serde(default)]
    pub preprocessing: i32,
    /// -hint: photo/picture/graph
    #[serde(default = "default_hint")]
    pub hint: String,

    // === 其他 ===
    /// -mt: 多线程
    #[serde(default = "default_mt")]
    pub mt: bool,
    /// -low_memory: 低内存模式
    #[serde(default)]
    pub low_memory: bool,
}

fn default_quality() -> f32 {
    75.0
}
fn default_method() -> i32 {
    4
}
fn default_preset() -> String {
    "default".to_string()
}
fn default_lossless_level() -> i32 {
    6
}
fn default_near_lossless() -> i32 {
    100
}
fn default_alpha_quality() -> i32 {
    100
}
fn default_alpha_method() -> i32 {
    1
}
fn default_alpha_filter() -> String {
    "fast".to_string()
}
fn default_pass() -> i32 {
    1
}
fn default_filter_strength() -> i32 {
    60
}
fn default_strong_filter() -> bool {
    true
}
fn default_sns_strength() -> i32 {
    50
}
fn default_segments() -> i32 {
    4
}
fn default_qmax() -> i32 {
    100
}
fn default_hint() -> String {
    "default".to_string()
}
fn default_mt() -> bool {
    true
}

fn preset_to_enum(s: &str) -> WebPPreset {
    match s.to_lowercase().as_str() {
        "photo" => WebPPreset::WEBP_PRESET_PHOTO,
        "picture" => WebPPreset::WEBP_PRESET_PICTURE,
        "drawing" => WebPPreset::WEBP_PRESET_DRAWING,
        "icon" => WebPPreset::WEBP_PRESET_ICON,
        "text" => WebPPreset::WEBP_PRESET_TEXT,
        _ => WebPPreset::WEBP_PRESET_DEFAULT,
    }
}

fn alpha_filter_to_int(s: &str) -> i32 {
    match s.to_lowercase().as_str() {
        "none" => 0,
        "best" => 2,
        _ => 1, // fast
    }
}

fn hint_to_enum(s: &str) -> i32 {
    match s.to_lowercase().as_str() {
        "photo" => 2,   // WEBP_HINT_PHOTO
        "picture" => 1, // WEBP_HINT_PICTURE
        "graph" => 3,   // WEBP_HINT_GRAPH
        _ => 0,         // WEBP_HINT_DEFAULT
    }
}

fn build_webp_config(options: &ConvertOptions) -> Result<WebPConfig, String> {
    let preset = preset_to_enum(&options.preset);
    let mut config = unsafe {
        let mut c = std::mem::MaybeUninit::uninit();
        if WebPConfigInitInternal(
            c.as_mut_ptr(),
            preset,
            options.quality,
            WEBP_ENCODER_ABI_VERSION as i32,
        ) == 0
        {
            return Err("WebPConfigInit 失败".to_string());
        }
        c.assume_init()
    };

    config.lossless = if options.lossless { 1 } else { 0 };
    config.method = options.method.clamp(0, 6);
    config.near_lossless = options.near_lossless.clamp(0, 100);
    if options.lossless && options.near_lossless < 100 {
        config.lossless = 1;
    }

    config.alpha_quality = options.alpha_quality.clamp(0, 100);
    config.alpha_compression = options.alpha_compression.clamp(0, 1);
    config.alpha_filtering = alpha_filter_to_int(&options.alpha_filter);
    config.exact = if options.exact { 1 } else { 0 };

    config.target_size = options.target_size.max(0);
    config.target_PSNR = options.target_psnr.max(0.0);
    config.pass = options.pass.clamp(1, 10);
    config.autofilter = if options.autofilter { 1 } else { 0 };
    config.emulate_jpeg_size = if options.jpeg_like { 1 } else { 0 };

    config.filter_strength = options.filter_strength.clamp(0, 100);
    config.filter_sharpness = options.filter_sharpness.clamp(0, 7);
    config.filter_type = if options.strong_filter { 1 } else { 0 };
    config.use_sharp_yuv = if options.sharp_yuv { 1 } else { 0 };
    config.sns_strength = options.sns_strength.clamp(0, 100);
    config.segments = options.segments.clamp(1, 4);
    config.partition_limit = options.partition_limit.clamp(0, 100);
    config.qmin = options.qmin.clamp(0, 100);
    config.qmax = options.qmax.clamp(0, 100);
    if config.qmin > config.qmax {
        config.qmin = 0;
        config.qmax = 100;
    }
    config.preprocessing = options.preprocessing.clamp(0, 7);
    config.image_hint = match hint_to_enum(&options.hint) {
        1 => WebPImageHint::WEBP_HINT_PICTURE,
        2 => WebPImageHint::WEBP_HINT_PHOTO,
        3 => WebPImageHint::WEBP_HINT_GRAPH,
        _ => WebPImageHint::WEBP_HINT_DEFAULT,
    };

    config.thread_level = if options.mt { 1 } else { 0 };
    config.low_memory = if options.low_memory { 1 } else { 0 };

    if options.lossless && options.near_lossless >= 100 {
        if unsafe { WebPConfigLosslessPreset(&mut config, options.lossless_level.clamp(0, 9)) } == 0
        {
            return Err("WebPConfigLosslessPreset 失败".to_string());
        }
    }

    if unsafe { libwebp_sys::WebPValidateConfig(&config) } == 0 {
        return Err("WebPValidateConfig 失败".to_string());
    }
    Ok(config)
}

/// 单文件进度事件 payload（progress_percent 为编码过程实时百分比，仅 status=pending 时有值）
#[derive(Clone, serde::Serialize)]
pub struct FileProgressPayload {
    pub path: String,
    pub status: String, // "ok" | "fail" | "pending"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_percent: Option<u32>,
}

/// 前端传入的完整 payload，确保 delete_original 正确反序列化
#[derive(Deserialize)]
pub struct ConvertPayload {
    pub paths: Vec<String>,
    pub options: ConvertOptions,
    #[serde(default)]
    pub delete_original: Option<bool>,
}

#[tauri::command]
pub fn convert_to_webp(app: tauri::AppHandle, payload: ConvertPayload) -> Result<(), String> {
    eprintln!("[webp] convert_to_webp 收到 paths 数量: {}", payload.paths.len());
    for (i, p) in payload.paths.iter().enumerate() {
        eprintln!("[webp]   paths[{}] 原始: {:?}", i, p);
    }

    let delete_original = payload.delete_original.unwrap_or(true);
    let mut all_paths: Vec<String> = Vec::new();
    let mut had_dir = false;
    for p in &payload.paths {
        let normalized = normalize_path(p);
        let path = Path::new(&normalized);
        let is_file = path.is_file();
        let is_dir = path.is_dir();
        eprintln!(
            "[webp] 路径 规范化后: {:?}  is_file: {}  is_dir: {}  is_supported: {}",
            normalized,
            is_file,
            is_dir,
            is_supported(&normalized)
        );
        if is_file {
            if is_supported(&normalized) {
                all_paths.push(normalized);
            }
        } else if is_dir {
            had_dir = true;
            let mut dir_file_count = 0u32;
            for entry in WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .map(|e| {
                    if let Err(ref err) = e {
                        eprintln!("[webp] WalkDir 错误: {:?}", err);
                    }
                    e.ok()
                })
                .filter_map(|x| x)
            {
                let entry_path = entry.path();
                if entry_path.is_file() {
                    if let Some(s) = entry_path.to_str() {
                        if is_supported(s) {
                            all_paths.push(s.to_string());
                            dir_file_count += 1;
                        }
                    }
                }
            }
            eprintln!("[webp] 目录内收集到支持格式文件数: {}", dir_file_count);
        } else {
            eprintln!("[webp] 路径既不是文件也不是目录，跳过");
        }
    }

    eprintln!("[webp] all_paths 总数: {}", all_paths.len());
    if all_paths.len() <= 5 {
        for (i, ap) in all_paths.iter().enumerate() {
            eprintln!("[webp]   all_paths[{}]: {:?}", i, ap);
        }
    } else {
        for (i, ap) in all_paths.iter().take(3).enumerate() {
            eprintln!("[webp]   all_paths[{}]: {:?}", i, ap);
        }
        eprintln!("[webp]   ... 共 {} 个", all_paths.len());
    }

    if all_paths.is_empty() {
        let code = if had_dir {
            "ERR_NO_IMAGES_IN_DIR"
        } else {
            "ERR_NO_IMAGES"
        };
        eprintln!("[webp] all_paths 为空，返回错误码: {}", code);
        return Err(code.to_string());
    }

    if let Some(w) = app.get_webview_window("main") {
        let _ = w.emit("convert-file-list", &all_paths);
    }

    let options = payload.options.clone();
    let app_clone = app.clone();
    std::thread::spawn(move || {
        let mut ok = 0u32;
        let mut fail = 0u32;
        let mut errors: Vec<String> = Vec::new();

        for path_str in &all_paths {
            let original_size = fs::metadata(path_str).ok().map(|m| m.len()).unwrap_or(0);
            let out_path = Path::new(path_str).with_extension("webp");

            if let Some(w) = app_clone.get_webview_window("main") {
                let _ = w.emit(
                    "convert-file-progress",
                    FileProgressPayload {
                        path: path_str.clone(),
                        status: "pending".to_string(),
                        original_size: None,
                        new_size: None,
                        error: None,
                        progress_percent: Some(0),
                    },
                );
            }

            let path_owned = path_str.clone();
            let app_for_progress = app_clone.clone();
            match convert_one(path_str, &options, move |percent| {
                if let Some(w) = app_for_progress.get_webview_window("main") {
                    let _ = w.emit(
                        "convert-file-progress",
                        FileProgressPayload {
                            path: path_owned.clone(),
                            status: "pending".to_string(),
                            original_size: None,
                            new_size: None,
                            error: None,
                            progress_percent: Some(percent),
                        },
                    );
                }
            }) {
                Ok(()) => {
                    let new_size = fs::metadata(&out_path).ok().map(|m| m.len()).unwrap_or(0);
                    if delete_original {
                        let _ = fs::remove_file(path_str);
                    }
                    ok += 1;
                    if let Some(w) = app_clone.get_webview_window("main") {
                        let _ = w.emit(
                            "convert-file-progress",
                            FileProgressPayload {
                                path: path_str.clone(),
                                status: "ok".to_string(),
                                original_size: Some(original_size),
                                new_size: Some(new_size),
                                error: None,
                                progress_percent: Some(100),
                            },
                        );
                    }
                }
                Err(e) => {
                    eprintln!("[webp] convert_one 失败 path: {:?} error: {}", path_str, e);
                    fail += 1;
                    errors.push(format!("{}: {}", path_str, e));
                    if let Some(w) = app_clone.get_webview_window("main") {
                        let _ = w.emit(
                            "convert-file-progress",
                            FileProgressPayload {
                                path: path_str.clone(),
                                status: "fail".to_string(),
                                original_size: None,
                                new_size: None,
                                error: Some(e.clone()),
                                progress_percent: None,
                            },
                        );
                    }
                }
            }
        }

        if let Some(w) = app_clone.get_webview_window("main") {
            let _ = w.emit(
                "convert-done",
                ConvertResult {
                    ok,
                    fail,
                    errors,
                },
            );
        }
    });

    Ok(())
}

#[derive(Clone, serde::Serialize)]
pub struct ConvertResult {
    pub ok: u32,
    pub fail: u32,
    pub errors: Vec<String>,
}

/// 返回 cwebp 默认参数，供前端回显
#[tauri::command]
pub fn get_default_options() -> ConvertOptions {
    ConvertOptions {
        quality: 75.0,
        lossless: false,
        method: 4,
        preset: "default".to_string(),
        lossless_level: 6,
        near_lossless: 100,
        alpha_quality: 100,
        alpha_compression: 1,
        alpha_filter: "fast".to_string(),
        exact: false,
        noalpha: false,
        target_size: 0,
        target_psnr: 0.0,
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
        hint: "default".to_string(),
        mt: true,
        low_memory: false,
    }
}

/// 规范化路径：拖放时 macOS 等可能传入 file:// 前缀或带末尾斜杠，需统一才能被识别为目录/文件
fn normalize_path(p: &str) -> String {
    let mut s = p.trim().to_string();
    if s.starts_with("file:///") {
        // "file:///" 索引 6 是第三根斜杠，保留 "/Users/..." 这种绝对路径（macOS 拖放常为此格式）
        s = s[6..].to_string();
    }
    // 去掉末尾斜杠，避免 Path::is_dir() 在某些环境下判断异常
    while s.len() > 1 && s.ends_with('/') {
        s.pop();
    }
    s
}

fn is_supported(path: &str) -> bool {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();
    SUPPORTED_EXT.contains(&ext.as_str())
}

/// 使用 libwebp_sys 编码并实时上报 progress_hook 进度
fn convert_one<F: Fn(u32) + 'static>(path_str: &str, options: &ConvertOptions, on_progress: F) -> Result<(), String> {
    let path = Path::new(path_str);
    let img = image::open(path).map_err(|e| e.to_string())?;
    let out_path = path.with_extension("webp");

    let width = img.width() as c_int;
    let height = img.height() as c_int;
    let rgba = img.to_rgba8();
    let raw = rgba.as_raw();

    let config = build_webp_config(options)?;

    let mut picture = unsafe {
        let mut p = std::mem::MaybeUninit::<libwebp_sys::WebPPicture>::uninit();
        if !WebPPictureInit(p.as_mut_ptr()) {
            return Err("WebPPictureInit 失败".to_string());
        }
        p.assume_init()
    };

    picture.width = width;
    picture.height = height;

    let stride = (width * 4) as c_int;
    let ok_import = if options.noalpha {
        let rgb: Vec<u8> = rgba
            .pixels()
            .flat_map(|p| [p[0], p[1], p[2]])
            .collect();
        let rgb_stride = (width * 3) as c_int;
        unsafe { WebPPictureImportRGB(&mut picture, rgb.as_ptr(), rgb_stride) != 0 }
    } else {
        unsafe { WebPPictureImportRGBA(&mut picture, raw.as_ptr(), stride) != 0 }
    };
    if !ok_import {
        unsafe { WebPPictureFree(&mut picture) };
        return Err("WebPPictureImport 失败".to_string());
    }

    let mut writer = WebPMemoryWriter {
        mem: std::ptr::null_mut(),
        size: 0,
        max_size: 0,
        pad: [0],
    };
    unsafe { WebPMemoryWriterInit(&mut writer) };
    picture.writer = Some(WebPMemoryWrite);
    picture.custom_ptr = &mut writer as *mut WebPMemoryWriter as *mut std::ffi::c_void;
    picture.progress_hook = Some(progress_hook);

    PROGRESS_CB.with(|cb| {
        *cb.borrow_mut() = Some(Box::new(move |p| on_progress(p)));
    });
    let encode_ok = unsafe { WebPEncode(&config, &mut picture) != 0 };
    PROGRESS_CB.with(|cb| {
        *cb.borrow_mut() = None;
    });

    if !encode_ok {
        unsafe {
            WebPMemoryWriterClear(&mut writer);
            WebPPictureFree(&mut picture);
        }
        return Err("WebPEncode 失败".to_string());
    }

    let webp_data = unsafe {
        std::slice::from_raw_parts(writer.mem, writer.size).to_vec()
    };
    unsafe {
        WebPMemoryWriterClear(&mut writer);
        WebPPictureFree(&mut picture);
    };

    std::fs::write(&out_path, &webp_data).map_err(|e| e.to_string())?;

    Ok(())
}
