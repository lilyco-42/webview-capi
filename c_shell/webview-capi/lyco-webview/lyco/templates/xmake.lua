add_rules("mode.debug", "mode.release")
target("{NAME}")
    set_kind("binary")
    add_files("src/main.c")
    add_includedirs(".")
    add_syslinks("user32","shell32","ole32","oleaut32","shlwapi","version")
    if is_mode("release") then set_optimize("smallest") end
target_end()
