# webview-capi

C 语言 API for WebView2 — 让任何语言都能调用 WebView2 创建跨平台桌面应用。

## 这是什么

`webview-capi` 是一个极小的 C 封装,通过 `webview.h` 单头文件,让你用 C 语言即可创建 WebView2 桌面窗口。同一个 API 在 Windows / macOS / Linux 上透明工作。

## 核心用法

```c
#include "webview.h"   // 单头文件引入

int main(void) {
    webview_t w = webview_create(0, NULL);        // 创建窗口
    webview_set_title(w, "My App");               // 标题
    webview_set_size(w, 800, 600, WEBVIEW_HINT_NONE); // 尺寸
    webview_navigate(w, "https://example.com");   // 加载网页
    webview_run(w);                               // 运行消息循环
    webview_destroy(w);                           // 销毁
    return 0;
}
```

编译只需链接 `webview.lib` + 系统库(位于 [`lib/`](lib/)):

```bash
cl /O2 /MT main.c /link /SUBSYSTEM:WINDOWS lib\webview.lib \
   user32.lib shell32.lib ole32.lib oleaut32.lib shlwapi.lib version.lib
```

## API 一览

| 函数 | 作用 |
|------|------|
| `webview_create(debug, window)` | 创建窗口,debug 开启 DevTools |
| `webview_destroy(w)` | 销毁窗口 |
| `webview_run(w)` | 进入消息循环(阻塞) |
| `webview_terminate(w)` | 终止 `webview_run`(可跨线程) |
| `webview_set_title(w, str)` | 设置标题 |
| `webview_set_size(w, w, h, hint)` | 设置尺寸,NONE/FIXED/MIN/MAX |
| `webview_navigate(w, url)` | 导航到 URL |
| `webview_set_html(w, html)` | 内嵌 HTML(完全离线) |
| `webview_eval(w, js)` | 执行 JS |
| `webview_bind(w, name, fn, arg)` | 把 JS 回调绑定到 C 函数 |
| `webview_get_window(w)` | 取原生窗口句柄 |

## 它有什么用

- **小而快的桌面壳**:把 Web 技术(CSS/JS/Canvas)用在桌面,产物 ~200KB
- **跨语言调用**:同样的 C API 可被 Python/Go/Rust/Node 通过 FFI 调用
- **内嵌离线 UI**:`webview_set_html` 生成界面,零网络依赖

## 平台

| 平台 | 底层 | 状态 |
|------|------|------|
| Windows | WebView2 (Edge) | ✅ |
| macOS | WKWebView | ✅ |
| Linux | WebKitGTK | ✅ |
| Android | WebView | ✅ (demo/android) |

## 示例

所有示例都在 [`demo/`](demo/) 子目录:

- [demo/c-main](demo/c-main) — C + WebView2 最简壳
- [demo/python-app](demo/python-app) — Python + pywebview
- [demo/android](demo/android) — Android WebView APK
- [demo/wasm-app](demo/wasm-app) — WASM 纯前端
- [demo/rust-ffi](demo/rust-ffi) — Rust FFI 绑定(cdylib)
- [demo/lyco-cli](demo/lyco-cli) — Rust CLI 生成器(用本 API 生成项目)

## 构建产物

预编译产物在 [`lib/`](lib/) 子目录:

- `lib/webview.dll` — Windows 动态库 (218KB)
- `lib/webview.lib` — 导入库
- `lib/webview.h` — 单头文件

发布打包(含示例 + 文档 + DLL)在 [`dist/`](dist/):

- `dist/webview-capi-v1.0.0-windows-x64.zip`

## License

MIT

> 相关:xmake 镜像 [lilyco-42/xmake-mirror](https://github.com/lilyco-42/xmake-mirror) 提供 `add_requires("webview-capi")` 一键引入。
