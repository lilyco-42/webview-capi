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

```bat
cl /O2 /MT main.c /link lib\webview.lib ^
   user32.lib shell32.lib ole32.lib oleaut32.lib shlwapi.lib version.lib
```

> **别加 `/SUBSYSTEM:WINDOWS`。** 上面是 `int main`,加了它会去要 `WinMain`,
> 报 `LNK2019: unresolved external symbol WinMain`。
> 想要不弹控制台的 GUI 程序,就把入口换成 `WinMain` 并补上
> `/SUBSYSTEM:WINDOWS /ENTRY:WinMainCRTStartup` —— 可参照
> [`mc-webview-shell`](https://github.com/lilyco-42/mc-webview-shell)。

运行时把 `lib\webview.dll` 放到 exe 旁边。**不需要额外准备 `WebView2Loader.dll`**:
`webview.dll` 的导入表里没有它(只有 ADVAPI32/KERNEL32/SHELL32/SHLWAPI/USER32/VERSION/ole32),
加载器是运行时按需查找的,找不到会回退到内置实现(读注册表定位 Edge WebView2 Runtime)。
所以 `ctypes.CDLL("lib/webview.dll")` 之类的 FFI 加载不会因为缺 DLL 而失败。

## API 一览

`lib/webview.dll` 实际导出 **17 个函数**(`dumpbin /exports lib\webview.dll` 可复核):

| 函数 | 作用 |
|------|------|
| `webview_create(debug, window)` | 创建窗口,debug 开启 DevTools。`window` 传 NULL 则自己建窗口并管生命周期;传原生句柄则嵌进该窗口,生命周期由调用方负责 |
| `webview_destroy(w)` | 销毁窗口 |
| `webview_run(w)` | 进入消息循环(阻塞) |
| `webview_terminate(w)` | 终止 `webview_run`(可跨线程) |
| `webview_dispatch(w, fn, arg)` | 把函数调度到跑消息循环的那个线程(多线程下安全回到主/GUI 线程) |
| `webview_set_title(w, str)` | 设置标题 |
| `webview_set_size(w, w, h, hint)` | 设置尺寸,NONE/FIXED/MIN/MAX |
| `webview_navigate(w, url)` | 导航到 URL |
| `webview_set_html(w, html)` | 内嵌 HTML(完全离线) |
| `webview_init(w, js)` | 注入一段 JS,在页面加载时、`window.onload` 之前执行 |
| `webview_eval(w, js)` | 执行 JS |
| `webview_bind(w, name, fn, arg)` | 把 JS 回调绑定到 C 函数 |
| `webview_unbind(w, name)` | 移除 `webview_bind` 建过的绑定 |
| `webview_return(w, id, status, result)` | 回应 JS 侧的绑定调用(可跨线程) |
| `webview_get_window(w)` | 取原生窗口句柄 |
| `webview_get_native_handle(w, kind)` | 取指定种类的原生句柄 |
| `webview_version()` | 取库版本信息 |

## 它有什么用

- **小而快的桌面壳**:把 Web 技术(CSS/JS/Canvas)用在桌面,产物 ~200KB
- **跨语言调用**:同样的 C API 可被 Python/Go/Rust/Node 通过 FFI 调用
- **内嵌离线 UI**:`webview_set_html` 生成界面,零网络依赖

## 平台

| 平台 | 底层 | 本仓预编译产物 | 状态 |
|------|------|---------------|------|
| Windows | WebView2 (Edge) | ✅ `lib/webview.dll` + `.lib` | ✅ |
| macOS | WKWebView | — 需自行编译 | ✅ |
| Linux | WebKitGTK | — 需自行编译 | ✅ |
| Android | WebView | — 用系统 WebView,见 `demo/android` | ✅ |

> `lib/` 里只有 Windows 产物。macOS / Linux 要拿 `lib/webview.h` 自己编
> (头文件里带 GTK / Cocoa / WebKitGTK 后端);
> Android 那个 demo 走的是系统 WebView,跟这个 DLL 无关。

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

- `lib/webview.dll` — Windows 动态库 (218 KB)
- `lib/webview.lib` — 导入库
- `lib/webview.h` — 单头文件

发布打包(含示例 + 文档 + DLL)在 [`dist/`](dist/):

- `dist/webview-capi-v1.0.0-windows-x64.zip` — **v1.0.0 的历史快照**,不是最新版;
  要最新的请直接用 `lib/` 或下面的 xmake 包

## xmake 快速添加

详细教程: [docs/XMAKE_TUTORIAL.md](docs/XMAKE_TUTORIAL.md)

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

```bash
xmake
xmake run
```

## License

MIT

> 相关:xmake 镜像 [lilyco-42/xmake-mirror](https://github.com/lilyco-42/xmake-mirror) 提供 `add_requires("webview-capi")` 一键引入。
