add_repositories("lyco-mirror https://github.com/lilyco-42/xmake-mirror.git")
add_requires("webview-capi")

target("c-main")
    set_kind("binary")
    add_files("src/main.c")
    add_packages("webview-capi")
    add_syslinks("user32", "shell32", "ole32", "oleaut32", "shlwapi", "version")
    -- WebView 是 GUI 应用：不加 -mwindows 会多挂一个控制台窗口
    -- （默认产物 subsystem=3/console，而仓库自带的 mc-webview.exe 是 2/GUI）。
    -- 注意：xmake 会探测 flag 是否有用，不写 {force = true} 会被静默丢弃。
    if is_plat("mingw") then
        add_ldflags("-mwindows", {force = true})
    end
target_end()
