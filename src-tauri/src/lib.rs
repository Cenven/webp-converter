// Copyright (c) 2025 WebP Tool Contributors. All rights reserved.
// SPDX-License-Identifier: MIT

mod convert;

use tauri::{Emitter, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            convert::convert_to_webp,
            convert::get_default_options,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            if let Some(win) = app.get_webview_window("main") {
                win.on_window_event(move |event: &tauri::WindowEvent| {
                    match event {
                        tauri::WindowEvent::CloseRequested { .. } => {
                            // 点击窗口关闭按钮时退出整个应用，避免 macOS 下“下次打不开”
                            let _ = handle.exit(0);
                        }
                        tauri::WindowEvent::DragDrop(drop) => {
                            if let tauri::DragDropEvent::Drop { paths, .. } = drop {
                                eprintln!("[webp] 拖放收到 paths 数量: {}", paths.len());
                                let paths: Vec<String> = paths
                                    .iter()
                                    .filter_map(|p| {
                                        let s = p.to_str().map(String::from);
                                        if let Some(ref x) = s {
                                            eprintln!("[webp] 拖放 path to_str: {:?}", x);
                                        }
                                        s
                                    })
                                    .collect();
                                eprintln!("[webp] 拖放转成 String 后数量: {}", paths.len());
                                if !paths.is_empty() {
                                    let _ = handle.get_webview_window("main").map(|w| {
                                        let _ = w.emit("drop-paths", paths);
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                });
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(move |_app_handle, _event| {
            // 不再阻止退出，以便点击窗口 X 或 Cmd+Q 时能正常退出
        });
}
