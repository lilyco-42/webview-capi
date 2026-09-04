# webview-capi

C API for WebView2 - 跨平台 WebView 生成器,支持 9 种编程语言。

[![Release](https://img.shields.io/github/v/release/lilyco-42/webview-capi)](https://github.com/lilyco-42/webview-capi/releases)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

## 快速开始

```bash
# 下载 lyco CLI (单二进制,283KB)
# https://github.com/lilyco-42/webview-capi/releases

# 创建项目
lyco new my-app python
cd my-app && lyco run
```

## 特性

- **单二进制** (283KB): 内置模板,首次运行自动释放
- **9 种语言**: C/Python/TypeScript/Rust/Go/Java/Zig/C#/易语言
- **5 平台**: Windows/Android/macOS/Linux/WASM
- **插件架构**: SO/DLL 动态加载第三方命令
- **可视化 UI**: `lyco web` 启动 Web 管理器

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

## 命令

| 命令 | 说明 |
|------|------|
| `lyco new <name> <lang> [url]` | 新建项目 |
| `lyco build` | 构建 |
| `lyco run` | 构建 + 运行 |
| `lyco web` | 启动可视化 Web UI |
| `lyco reset` | 重置为默认模板 |
| `lyco info` | 查看配置 |
| `lyco list` | 列出可用命令 |

## 支持语言

C · Python · TypeScript · Rust · Go · Java · Zig · C# · 易语言

## 插件开发

将 `.dll` + `.exe` (Windows) 或 `.so` + 可执行文件 (Linux/macOS) 放入 `~/.lyco/commands/` 即可覆盖内置命令。

详见 [docs/PLUGIN_DEVELOPMENT.md](docs/PLUGIN_DEVELOPMENT.md)。

## License

MIT
