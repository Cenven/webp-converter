// Copyright (c) 2025 WebP Tool Contributors. All rights reserved.
// SPDX-License-Identifier: MIT

// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    webp_converter::run()
}
