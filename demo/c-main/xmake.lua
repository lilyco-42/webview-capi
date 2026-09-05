add_repositories("lyco-mirror https://github.com/lilyco-42/xmake-mirror.git")
add_requires("webview-capi")

target("c-main")
    set_kind("binary")
    add_files("src/main.c")
    add_packages("webview-capi")
    add_syslinks("user32", "shell32", "ole32", "oleaut32", "shlwapi", "version")
target_end()
