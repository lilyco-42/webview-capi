# demo/rust-ffi

Rust 安全封装 [webview C API](../../lib/webview.h) 的绑定库。

> 注意：`lib/` 里只有 Windows 产物（`webview.dll` + `webview.lib`），
> 所以这个 demo 目前只能在 Windows 上构建。

## 构建

```bash
cargo build
```

产物：`target/debug/webview_capi.dll`、`target/debug/webview_capi.lib`
（`crate-type = ["lib", "cdylib", "staticlib"]`）。

## 作为依赖使用

```toml
[dependencies]
webview-capi = { path = "demo/rust-ffi" }
```

```rust
use webview_capi::WebView;

let w = WebView::new(false).expect("webview_create 失败");
w.set_title("Hello");
w.set_size(800, 600, 0);
w.set_html("<h1>Hello</h1>");
w.run();
```

## 运行时

`webview.dll` 必须能被加载器找到 —— 放到可执行文件旁边，或放进 `PATH`：

```bash
cp ../../lib/webview.dll target/debug/
```

不需要额外的 `WebView2Loader.dll`（见[根 README](../../README.md)）。

## 实现说明

`build.rs` 负责把链接指向仓库根的 `lib/`：

```rust
println!("cargo:rustc-link-search=native={}", lib_dir.display());
println!("cargo:rustc-link-lib=dylib=webview");
```

`lib/webview.lib` 是 MSVC 风格的导入库，但 MinGW 的 `ld` 也能直接吃 `.lib`，
所以 MSVC 与 MinGW 两条工具链都能链上。
