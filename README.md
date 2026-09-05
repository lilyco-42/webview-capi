# webview-capi

> xmake 一键引入的 WebView2 C API,跨平台 WebView 应用开发

[![Release](https://img.shields.io/github/v/release/lilyco-42/webview-capi)](https://github.com/lilyco-42/webview-capi/releases)
[![CI](https://github.com/lilyco-42/webview-capi/workflows/Windows%20CI/badge.svg)](https://github.com/lilyco-42/webview-capi/actions)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

## 快速开始 (xmake 一键引入)

### 1. 添加 xmake 镜像源

```lua
-- xmake.lua
add_repositories("lyco-mirror https://github.com/lilyco-42/xmake-mirror.git")
```

### 2. 引入包

```lua
add_requires("webview-capi")

target("my-app")
    set_kind("binary")
    add_files("src/main.c")
    add_packages("webview-capi")
    add_syslinks("user32", "shell32", "ole32", "oleaut32", "shlwapi", "version")
target_end()
```

### 3. 写代码

```c
// src/main.c
#include "webview.h"

int main(void) {
    webview_t w = webview_create(0, NULL);
    webview_set_title(w, "MC Console");
    webview_set_size(w, 1100, 760, WEBVIEW_HINT_NONE);
    webview_navigate(w, "http://192.168.10.165:8765");
    webview_run(w);
    webview_destroy(w);
    return 0;
}
```

### 4. 编译运行

```bash
xmake
xmake run
```

## 示例项目

| 示例 | 说明 | 路径 |
|------|------|------|
| c-main | C + WebView2 示例 | [demo/c-main](demo/c-main) |
| python-app | Python + pywebview 示例 | [demo/python-app](demo/python-app) |
| android | Android WebView APK | [demo/android](demo/android) |
| lyco-cli | Rust CLI 工具源码 | [demo/lyco-cli](demo/lyco-cli) |

## 直接下载

不想用 xmake? 直接下载单二进制 CLI:

```bash
# 下载 lyco.exe (6MB)
# https://github.com/lilyco-42/webview-capi/releases

# 双击启动可视化界面
# 或命令行:
lyco new my-app python
lyco build && lyco run
```

## 支持平台

- 🪟 Windows (WebView2, Win10+)
- 🍎 macOS (WKWebView)
- 🐧 Linux (WebKitGTK)
- 🤖 Android (WebView, API 24+)
- 🌐 WASM (纯前端)

## 支持语言

C · Python · TypeScript · Rust · Go · Java · Zig · C# · 易语言

## 包管理

| 包名 | 说明 |
|------|------|
| `webview-capi` | C API DLL + 头文件 |
| `webview-mini` | 单头文件最小封装 |

## 文档

- [插件开发](docs/PLUGIN_DEVELOPMENT.md)
- [路线图](docs/ROADMAP_V1.md)

## License

MIT
