# xmake 快速添加教程

> 30 秒通过 `add_requires("webview-capi")` 让你的 xmake 项目拥有 WebView 能力

## 前置条件

- [xmake](https://xmake.io) >= 2.8.0
- Windows 10+ / macOS 10.13+ / Linux (带 WebKitGTK)

## 第 1 步:添加镜像源

在项目的 `xmake.lua` 中添加 lyco 镜像:

```lua
add_repositories("lyco-mirror https://github.com/lilyco-42/xmake-mirror.git")
```

## 第 2 步:引入包

```lua
add_requires("webview-capi")
```

## 第 3 步:写代码

创建 `src/main.c`:

```c
#include "webview.h"

int main(void) {
    webview_t w = webview_create(0, NULL);
    webview_set_title(w, "My WebView App");
    webview_set_size(w, 800, 600, WEBVIEW_HINT_NONE);
    webview_navigate(w, "https://example.com");
    webview_run(w);
    webview_destroy(w);
    return 0;
}
```

## 第 4 步:配置 target

```lua
target("my-app")
    set_kind("binary")
    add_files("src/main.c")
    add_packages("webview-capi")
    add_syslinks("user32", "shell32", "ole32", "oleaut32", "shlwapi", "version")
target_end()
```

## 第 5 步:编译运行

```bash
xmake              # 配置 + 构建
xmake run          # 运行
```

## 完整 xmake.lua 示例

```lua
add_rules("mode.debug", "mode.release")
add_repositories("lyco-mirror https://github.com/lilyco-42/xmake-mirror.git")
add_requires("webview-capi")

target("my-app")
    set_kind("binary")
    add_files("src/main.c")
    add_packages("webview-capi")
    add_syslinks("user32", "shell32", "ole32", "oleaut32", "shlwapi", "version")
    if is_mode("release") then
        set_optimize("smallest")
    end
target_end()
```

## 多语言示例

### Python + pywebview

```python
# main.py
import webview

window = webview.create_window("My App", "https://example.com", width=800, height=600)
webview.start()
```

```bash
pip install pywebview
python main.py
```

### Rust + webview crate

```rust
// src/main.rs
use webview::WebView;

fn main() {
    let w = WebView::create(true);
    w.set_title("My App");
    w.set_size(800, 600, 0);
    w.navigate("https://example.com");
    w.run();
    w.destroy();
}
```

```toml
# Cargo.toml
[dependencies]
webview = "0.1"
```

### Go + webview

```go
// main.go
package main
import "github.com/webview/webview"

func main() {
    w := webview.New(false)
    defer w.Destroy()
    w.SetTitle("My App")
    w.SetSize(800, 600, webview.HintNone)
    w.Navigate("https://example.com")
    w.Run()
}
```

## 常见问题

### Q: 找不到 webview.h?

确认 `add_packages("webview-capi")` 已添加,且镜像源配置正确。

### Q: 链接错误 LNK2019?

检查 `add_syslinks` 是否包含所有必需库:
```lua
add_syslinks("user32", "shell32", "ole32", "oleaut32", "shlwapi", "version")
```

### Q: macOS/Linux 支持?

- macOS: 自动使用 WKWebView,无需额外配置
- Linux: 需要安装 `libwebkit2gtk-4.0-dev`

### Q: 如何内嵌 HTML?

```c
webview_set_html(w, "<html><body><h1>Hello</h1></body></html>");
```

### Q: 如何执行 JS?

```c
webview_eval(w, "console.log('hello from C')");
```

### Q: 如何绑定 C 函数到 JS?

```c
// C 回调
void my_callback(const char* seq, const char* req, void* arg) {
    printf("JS called: %s\n", req);
    webview_return(w, seq, 0, "{\"result\":\"ok\"}");
}

// 绑定
webview_bind(w, "myFunc", my_callback, NULL);

// JS 调用
// window.myFunc("hello").then(r => console.log(r));
```

## 更多 API

详见 [README.md](../README.md#api-一览)
