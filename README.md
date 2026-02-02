# WebP Converter

[English](README.md) | [简体中文](README.zh-CN.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A desktop app to convert PNG, JPG, and JPEG images to WebP. Drag & drop, batch conversion, and full cwebp-style options.

---

## Features

- **Batch conversion** — Select files/folders or drag & drop; convert multiple images at once
- **Full cwebp options** — Quality, lossless, method, preset, alpha, filters, and more (grouped in collapsible panels)
- **Progress feedback** — Per-file status and compression ratio; optional delete original after success
- **Bilingual UI** — 中文 / English
- **Cross-platform** — Built with [Tauri 2](https://tauri.app/) for Windows and macOS
- **Lightweight** — Native binary + small WebView; no Electron

---

## Prerequisites

- [Node.js](https://nodejs.org/) (LTS) and [pnpm](https://pnpm.io/) (`npm install -g pnpm`)
- [Rust](https://www.rust-lang.org/tools/install) (for building the Tauri app)
- Platform-specific:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

---

## Installation

```bash
git clone https://github.com/YOUR_USERNAME/webp-tool.git
cd webp-tool
pnpm install
```

Replace `YOUR_USERNAME` with your GitHub username or org.

---

## Development

Run the app in development mode (Vite dev server + Tauri window):

```bash
pnpm tauri dev
```

---

## Building installers

Tauri produces **standalone** installers. End users do not need Node, Rust, or any extra runtime (the system WebView is enough).

### Build on macOS (get macOS installers)

1. Install [Xcode Command Line Tools](https://developer.apple.com/xcode/), Node.js, pnpm, and Rust.
2. In the project root:

```bash
pnpm install
pnpm tauri build
```

3. Output directory: `src-tauri/target/release/bundle/`
   - **`.dmg`** — Disk image; users mount it and drag the app to Applications.
   - **`.app`** — Application bundle; can be copied or dragged out of the .dmg.

### Build on Windows (get Windows installers)

1. Install [Visual Studio C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/), Node.js, pnpm, and Rust.
2. In the project root:

```bash
pnpm install
pnpm tauri build
```

3. Output directory: `src-tauri\target\release\bundle\`
   - **`.msi`** — Installer; users run it and follow the wizard.
   - **`.exe`** — NSIS installer; same flow.

### Cross-platform

- On **macOS** you can only build **macOS** (.dmg/.app). On **Windows** you can only build **Windows** (.msi/.exe).
- To ship both: build once on Mac and once on Windows, or use [GitHub Actions](https://tauri.app/v2/guides/building/ci/) (or other CI) for each platform.

---


## Tech Stack

| Layer    | Stack |
|----------|--------|
| Frontend | Vue 3, TypeScript, Vite |
| Desktop  | Tauri 2 |
| Encoding | libwebp (Rust: `libwebp-sys`), `image` (decode PNG/JPEG) |

---

## Project Structure

```
webp-tool/
├── src/                 # Vue frontend
│   ├── App.vue          # Main UI, options, file list, i18n
│   ├── main.ts
│   └── style.css
├── src-tauri/           # Tauri (Rust) backend
│   ├── src/
│   │   ├── lib.rs       # App entry, drag-drop, commands
│   │   └── convert.rs   # WebP conversion, cwebp options
│   ├── icons/
│   └── tauri.conf.json
├── package.json
└── README.md
```

---

## License

See [LICENSE](LICENSE) for details.
