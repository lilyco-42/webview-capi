-- 注意：用户实际拿到的是 xmake-mirror 里的那份
--   https://github.com/lilyco-42/xmake-mirror/blob/main/packages/w/webview-capi/xmake.lua
-- 本文件与它保持一致。改配方时两处都要改（或只改镜像仓，再把这里同步过去）。
--
-- 旧版本这里的四处问题（都已修）：
--   1. add_versions("1.0.0", "main") —— tag 1.0.0 不存在（只有 v1.0.0/v1.1.0/v1.2.0），
--      tarball URL 会 404，只能退回到 git clone 整个仓库
--   2. set_urls 少了 v 前缀（v$(version) 才对得上 tag 名）
--   3. on_install 从仓库根目录 cp webview.h / webview.dll / webview.lib ——
--      这三个文件在 lib/ 下，cp 会直接失败
--   4. 缺 add_links("webview")，且 add_includedirs(".") 应为 "include"
--      （on_install 是装到 installdir("include")）
package("webview-capi")
    set_homepage("https://github.com/lilyco-42/webview-capi")
    set_description("C API for WebView2 - callable from any language (Python/Go/Rust/Node.js)")
    set_license("MIT")

    set_urls("https://github.com/lilyco-42/webview-capi/archive/refs/tags/v$(version).tar.gz",
             "https://github.com/lilyco-42/webview-capi.git")

    -- 1.1.0 与 1.2.0 的 lib/ 三个文件字节完全相同（blob sha 一致），
    -- 所以 pin 在 1.1.0 不损失什么。
    add_versions("1.1.0", "11420ea13763ebc82cbc21866e2107884d097fbd4d188c5dca3be506797645d1")

    add_syslinks("user32", "shell32", "ole32", "oleaut32", "shlwapi", "version")
    add_links("webview")

    on_install("windows", "mingw", function (package)
        -- lib/ 下: webview.h (单头文件) + webview.dll/.lib (预编译)
        os.cp("lib/webview.h", package:installdir("include"))
        os.cp("lib/webview.dll", package:installdir("bin"))
        os.cp("lib/webview.lib", package:installdir("lib"))
        if package:is_plat("mingw") then
            -- mingw ld 可直接链 DLL (搜索 libwebview.dll / webview.dll)
            os.cp("lib/webview.dll", package:installdir("lib"))
        end
    end)

    on_test(function (package)
        assert(package:has_cfuncs("webview_create", {includes = "webview.h"}))
    end)
package_end()
