# WebP Converter

[English](README.md) | [简体中文](README.zh-CN.md)

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

将 PNG、JPG、JPEG 转为 WebP 的桌面应用，支持拖放、批量转换与完整 cwebp 参数配置。

---

## 功能

- **批量转换** — 选择文件/文件夹或拖放，一次转换多张图片
- **完整 cwebp 参数** — 质量、无损、方法、预设、Alpha、滤镜等（分组折叠面板）
- **进度反馈** — 单文件状态与压缩比；可选转换成功后删除原图
- **双语界面** — 中文 / English
- **跨平台** — 基于 [Tauri 2](https://tauri.app/)，支持 Windows 与 macOS
- **轻量** — 原生二进制 + 系统 WebView，无 Electron

---

## 环境要求

- [Node.js](https://nodejs.org/)（LTS）与 [pnpm](https://pnpm.io/)（`npm install -g pnpm`）
- [Rust](https://www.rust-lang.org/tools/install)（用于构建 Tauri 应用）
- 平台相关：
  - **macOS**：Xcode Command Line Tools
  - **Windows**：[Visual Studio C++ 构建工具](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

---

## 安装

```bash
git clone https://github.com/YOUR_USERNAME/webp-converter.git
cd webp-converter
pnpm install
```

将 `YOUR_USERNAME` 替换为你的 GitHub 用户名或组织名。

---

## 开发

以开发模式运行（Vite 开发服务 + Tauri 窗口）：

```bash
pnpm tauri dev
```

---

## 打包成安装包

Tauri 打出来的是**独立安装包**，用户无需安装 Node、Rust 或任何额外运行时（系统自带 WebView 即可）。

### 在 macOS 上打包（得到 macOS 安装包）

1. 确保已安装 [Xcode Command Line Tools](https://developer.apple.com/xcode/)、Node.js、pnpm、Rust。
2. 在项目根目录执行：

```bash
pnpm install
pnpm tauri build
```

3. 安装包输出目录：`src-tauri/target/release/bundle/`
   - **`.dmg`** — 磁盘镜像，用户双击挂载后把应用拖到「应用程序」即可。
   - **`.app`** — 应用包，可直接复制或从 .dmg 里拖出。

### 在 Windows 上打包（得到 Windows 安装包）

1. 确保已安装 [Visual Studio C++ 构建工具](https://visualstudio.microsoft.com/visual-cpp-build-tools/)、Node.js、pnpm、Rust。
2. 在项目根目录执行：

```bash
pnpm install
pnpm tauri build
```

3. 安装包输出目录：`src-tauri\target\release\bundle\`
   - **`.msi`** — 安装程序，用户双击按向导安装即可。
   - **`.exe`** — NSIS 安装程序，同样双击即可安装。

### 跨平台说明

- **在 macOS 上只能打出 macOS 的 .dmg/.app**；**在 Windows 上只能打出 Windows 的 .msi/.exe**。
- 若要同时提供两个平台的安装包：在 Mac 上打一次、在 Windows 上打一次；或用 [GitHub Actions](https://tauri.app/v2/guides/building/ci/) 等 CI 分别构建。

---

## 技术栈

| 层级     | 技术 |
|----------|------|
| 前端     | Vue 3、TypeScript、Vite |
| 桌面壳   | Tauri 2 |
| 编码     | libwebp（Rust：`libwebp-sys`）、`image`（解码 PNG/JPEG） |

---

## 项目结构

```
webp-converter/
├── src/                 # Vue 前端
│   ├── App.vue          # 主界面、参数、文件列表、多语言
│   ├── main.ts
│   └── style.css
├── src-tauri/           # Tauri（Rust）后端
│   ├── src/
│   │   ├── lib.rs       # 应用入口、拖放、命令
│   │   └── convert.rs   # WebP 转换、cwebp 参数
│   ├── icons/
│   └── tauri.conf.json
├── package.json
└── README.md
```

---

## 许可证

详见 [LICENSE](LICENSE)。
