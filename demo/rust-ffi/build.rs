//! 链接 webview-capi 的 C 实现（`lib/webview.dll` 的导入库）。
//!
//! 原来这两行是错的：
//!     println!("cargo:rustc-link-search=native=.");        // lib/ 不在 demo/rust-ffi/ 下
//!     println!("cargo:rustc-link-lib=dylib=webview-capi"); // 没有叫 webview-capi 的库
//! 后果（MSVC 下实测）：
//!     LINK : fatal error LNK1181: 无法打开输入文件"webview-capi.lib"
//! 仓库里只有 `lib/webview.lib` + `lib/webview.dll`，而 src/lib.rs 里 extern "C"
//! 声明的正是 webview.dll 导出的那批符号（webview_create / webview_run / …）。
//!
//! 关于 `webview.lib` 的格式：它是 MSVC 风格的导入库，但 MinGW 的 ld 也能直接吃
//! `.lib`，所以两条工具链都能链上 —— 实测依据：本仓 demo/c-main 用 MinGW(xmake)
//! 构建出的 c-main.exe，导入表里确实有 webview.dll。
//!
//! 运行时注意：`webview.dll` 必须能被找到（与产物同目录，或放进 PATH）。

use std::path::PathBuf;

fn main() {
    // demo/rust-ffi/build.rs -> demo/rust-ffi -> demo -> 仓库根
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lib_dir = crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map(|root| root.join("lib"))
        .expect("无法从 CARGO_MANIFEST_DIR 推出仓库根目录");

    let import_lib = lib_dir.join("webview.lib");
    if !import_lib.exists() {
        panic!(
            "找不到 {}\n\
             demo/rust-ffi 依赖仓库根目录下 lib/webview.lib（webview.dll 的导入库）。\n\
             请确认 lib/ 目录完整（webview.h / webview.dll / webview.lib）。",
            import_lib.display()
        );
    }

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // 库名是 webview（webview.dll / webview.lib），不是 webview-capi
    println!("cargo:rustc-link-lib=dylib=webview");

    // 库变了要重新链接
    println!("cargo:rerun-if-changed={}", import_lib.display());
}
