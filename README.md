# Lyco WebView Studio

> 一键生成跨平台 WebView 项目,支持 9 种编程语言

[![Release](https://img.shields.io/github/v/release/lilyco-42/webview-capi)](https://github.com/lilyco-42/webview-capi/releases)
[![CI](https://github.com/lilyco-42/webview-capi/workflows/Windows%20CI/badge.svg)](https://github.com/lilyco-42/webview-capi/actions)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

## 快速开始

```bash
# 1. 下载安装
# MSI/MSIX: https://github.com/lilyco-42/webview-capi/releases
# 或直接下载 lyco.exe (单二进制, ~6MB)

# 2. 创建项目
lyco new my-app python
cd my-app && lyco run
```

## 核心特性

| 特性 | 说明 |
|------|------|
| **单二进制** | Rust 编译,一个 exe,无需安装 |
| **9 种语言** | C/Python/TypeScript/Rust/Go/Java/Zig/C#/易语言 |
| **5 平台** | Windows/macOS/Linux/Android/WASM |
| **可视化** | `lyco web` 启动 Web 管理器 |
| **插件架构** | 第三方 DLL/SO 完全替代内置命令 |
| **持久化** | SQLite 记录项目/构建历史 |

## xmake 一键引入

```lua
add_repositories("lyco-mirror https://github.com/lilyco-42/xmake-mirror.git")
add_requires("webview-capi")

target("my-app")
    set_kind("binary")
    add_files("src/main.c")
    add_packages("webview-capi")
    add_syslinks("user32", "shell32", "ole32", "oleaut32", "shlwapi", "version")
target_end()
```

## 命令参考

| 命令 | 说明 |
|------|------|
| `lyco new <name> <lang> [url]` | 新建项目 |
| `lyco build` | 构建当前项目 |
| `lyco run` | 构建 + 运行 |
| `lyco web` | 启动可视化 Web 管理器 |
| `lyco info` | 查看配置状态 |
| `lyco list` | 列出所有可用命令 |
| `lyco reset` | 重置为默认模板 |

## 支持的语言与平台

**语言**: C · Python · TypeScript · Rust · Go · Java · Zig · C# · 易语言

**平台**:
- 🪟 Windows (WebView2)
- 🍎 macOS (WKWebView)
- 🐧 Linux (WebKitGTK)
- 🤖 Android (WebView)
- 🌐 WASM (纯前端 PWA)

## 可视化 Web 管理器

```bash
lyco web
# 打开 http://localhost:8080
```

功能:项目向导、构建日志、AI 辅助、环境自检、插件管理

## 插件开发

将动态库和可执行文件放入 `~/.lyco/commands/`:

```
~/.lyco/commands/
├── new.dll      # 覆盖 new 命令
├── new.exe      # 实际执行
├── build.dll    # 覆盖 build 命令
└── build.exe    # 实际执行
```

详见 [docs/PLUGIN_DEVELOPMENT.md](docs/PLUGIN_DEVELOPMENT.md)

## 项目结构

```
webview-capi/
├── lyco-webview/lyco/    # CLI 源码 (Rust)
├── web/                  # 可视化 Web UI
├── android/              # Android WebView APK
├── wix/                  # MSI 安装包配置
├── msix/                 # MSIX 应用包配置
├── docs/                 # 开发文档
└── .github/workflows/    # CI/CD (Windows/Android/Rust)
```

## 技术路线图

- **v1.0** ✅ 单二进制 + Tera 模板 + SQLite + 可视化 UI
- **v2.0** ⬜ WASM 插件系统 + Tauri 2 桌面
- **v3.0** ⬜ 本地 AI (Ollama) + 插件市场

详见 [docs/ROADMAP_V1.md](docs/ROADMAP_V1.md)

## License

MIT
