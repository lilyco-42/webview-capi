//! webview-capi - C API for WebView2
//! 
//! # Example
//! ```c
//! #include "webview.h"
//! webview_t w = webview_create(0, NULL);
//! webview_run(w);
//! webview_destroy(w);
//! ```

// C 侧类型。用 `pub use` 一次完成「引入 + 对外重导出」——
// 原来这里还有一行 `use std::os::raw::{c_char, c_int, c_void};`，
// 和文件末尾的 `pub use` 同名，直接 E0252（the name `c_char` is defined multiple times），
// 整个 crate 编译不过。
pub use std::ffi::CString;
pub use std::os::raw::{c_char, c_int, c_void};

/// Opaque webview handle
///
/// 对应 C 侧的 `typedef void *webview_t;`（见 lib/webview.h:197）。
/// Rust 侧只当不透明指针用，不需要知道内部布局。
///
/// `#[repr(C)]` + 零长度字段 = 标准的「不透明 FFI 类型」写法；
/// 不加 repr 会报 `not FFI-safe: this struct has unspecified layout`。
#[repr(C)]
pub struct Webview {
    _private: [u8; 0],
}

// 返回值说明：C 侧 webview_run / webview_set_* 等返回 `webview_error_t`，
// 那是个普通 C 枚举（WEBVIEW_ERROR_OK=0 / _FAILED=1 / _NOT_FOUND=2），
// ABI 上是 int 宽度，所以这里用 c_int 与之等价。
extern "C" {
    fn webview_create(debug: c_int, window: *mut c_void) -> *mut Webview;
    fn webview_destroy(w: *mut Webview) -> c_int;
    fn webview_run(w: *mut Webview) -> c_int;
    fn webview_terminate(w: *mut Webview) -> c_int;
    fn webview_set_title(w: *mut Webview, title: *const c_char) -> c_int;
    fn webview_set_size(w: *mut Webview, width: c_int, height: c_int, hints: c_int) -> c_int;
    fn webview_navigate(w: *mut Webview, url: *const c_char) -> c_int;
    fn webview_set_html(w: *mut Webview, html: *const c_char) -> c_int;
    fn webview_eval(w: *mut Webview, js: *const c_char) -> c_int;
}

/// Safe wrapper around webview C API
pub struct WebView {
    ptr: *mut Webview,
}

impl WebView {
    /// Create a new webview instance
    pub fn new(debug: bool) -> Option<Self> {
        let ptr = unsafe { webview_create(debug as i32, std::ptr::null_mut()) };
        if ptr.is_null() {
            None
        } else {
            Some(Self { ptr })
        }
    }

    /// Set window title
    pub fn set_title(&self, title: &str) {
        let c_title = std::ffi::CString::new(title).unwrap();
        unsafe { webview_set_title(self.ptr, c_title.as_ptr()) };
    }

    /// Set window size
    pub fn set_size(&self, width: i32, height: i32, hints: i32) {
        unsafe { webview_set_size(self.ptr, width, height, hints) };
    }

    /// Navigate to URL
    pub fn navigate(&self, url: &str) {
        let c_url = std::ffi::CString::new(url).unwrap();
        unsafe { webview_navigate(self.ptr, c_url.as_ptr()) };
    }

    /// Set HTML content
    pub fn set_html(&self, html: &str) {
        let c_html = std::ffi::CString::new(html).unwrap();
        unsafe { webview_set_html(self.ptr, c_html.as_ptr()) };
    }

    /// Evaluate JavaScript
    pub fn eval(&self, js: &str) -> i32 {
        let c_js = std::ffi::CString::new(js).unwrap();
        unsafe { webview_eval(self.ptr, c_js.as_ptr()) }
    }

    /// Run the main loop
    pub fn run(&self) -> i32 {
        unsafe { webview_run(self.ptr) }
    }

    /// Terminate the main loop
    pub fn terminate(&self) -> i32 {
        unsafe { webview_terminate(self.ptr) }
    }
}

impl Drop for WebView {
    fn drop(&mut self) {
        // webview_destroy 返回 webview_error_t（int 宽度），丢弃返回值即可
        let _ = unsafe { webview_destroy(self.ptr) };
    }
}
