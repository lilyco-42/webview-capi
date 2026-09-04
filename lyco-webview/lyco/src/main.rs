use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

const WEBVIEW_H_URL: &str = "https://raw.githubusercontent.com/webview/webview/master/webview.h";

const TEMPLATE_MAIN_C: &str = r#"#include "webview.h"

int main(void) {
    webview_t w = webview_create(0, NULL);
    webview_set_title(w, "{NAME}");
    webview_set_size(w, 1100, 760, WEBVIEW_HINT_NONE);
    webview_navigate(w, "{URL}");
    webview_run(w);
    webview_destroy(w);
    return 0;
}
"#;

const TEMPLATE_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{NAME}</title>
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
       background: #1a1a2e; color: #e0e0e8; display: flex; flex-direction: column;
       align-items: center; justify-content: center; height: 100vh; }
h1 { color: #e94560; margin-bottom: 20px; }
.card { background: #16213e; border-radius: 12px; padding: 24px; margin: 12px;
        max-width: 400px; width: 100%; }
button { background: #0f3460; color: #e0e0e8; border: none; padding: 12px 24px;
         border-radius: 8px; cursor: pointer; font-size: 1rem; margin: 4px; }
button:hover { background: #e94560; }
#log { margin-top: 16px; font-family: monospace; font-size: 0.85rem; color: #8b949e; }
</style>
</head>
<body>
<div class="card">
    <h1>{NAME}</h1>
    <p>WebView 跨平台应用 · 一键生成</p>
    <div style="margin-top:16px">
        <button onclick="window.lyco && window.lyco.alert('Hello!')">调用原生</button>
        <button onclick="log('JS 工作正常')">测试 JS</button>
    </div>
    <div id="log"></div>
</div>
<script>
function log(msg) {
    document.getElementById('log').textContent = msg;
    console.log(msg);
}
</script>
</body>
</html>
"#;

const TEMPLATE_XMAKE: &str = r#"add_rules("mode.debug", "mode.release")

target("{NAME}")
    set_kind("binary")
    add_files("src/main.c")
    add_includedirs(".")
    add_syslinks("user32", "shell32", "ole32", "oleaut32", "shlwapi", "version")
    if is_mode("release") then
        set_optimize("smallest")
    end
target_end()
"#;

const TEMPLATE_CMAKE: &str = r#"cmake_minimum_required(VERSION 3.10)
project({NAME} C)
set(CMAKE_C_STANDARD 11)
add_executable({NAME} src/main.c)
target_include_directories({NAME} PRIVATE ${{CMAKE_CURRENT_SOURCE_DIR}})
target_link_libraries({NAME} PRIVATE
    user32 shell32 ole32 oleaut32 shlwapi version)
"#;

const TEMPLATE_ANDROID_JAVA: &str = r#"package local.mc.console;

import android.app.Activity;
import android.os.Bundle;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.webkit.WebSettings;

public class MainActivity extends Activity {{
    private WebView web;
    @Override
    protected void onCreate(Bundle savedInstanceState) {{
        super.onCreate(savedInstanceState);
        web = new WebView(this);
        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        web.setWebViewClient(new WebViewClient());
        setContentView(web);
        web.loadUrl("{URL}");
    }}
}}
"#;

const TEMPLATE_WASM_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>{NAME} - WASM</title>
<style>
body{{font-family:system-ui,sans-serif;margin:0;padding:40px;background:#1a1a2e;color:#e0e0e8;text-align:center}}
h1{{color:#e94560}}
</style>
</head>
<body>
<h1>{NAME}</h1>
<p>WASM 模式 - 纯前端 WebView 应用</p>
<canvas id="canvas" width="400" height="300" style="border:1px solid #333"></canvas>
<script>
const c = document.getElementById('canvas');
const ctx = c.getContext('2d');
ctx.fillStyle = '#0f3460';
ctx.fillRect(50,50,300,200);
ctx.fillStyle = '#e94560';
ctx.font = '24px sans-serif';
ctx.fillText('WASM WebView OK', 70, 160);
</script>
</body>
</html>
"#;

struct ProjectConfig {
    name: String,
    url: String,
    targets: Vec<String>,
}

fn template_replace(template: &str, vars: &HashMap<&str, &str>) -> String {
    let mut result = template.to_string();
    for (key, val) in vars {
        result = result.replace(&format!("{{{}}}", key), val);
    }
    result
}

fn write_file(path: &str, content: &str) {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(path, content).unwrap_or_else(|e| {
        eprintln!("写入失败 {}: {}", path, e);
        std::process::exit(1);
    });
}

fn fetch_webview_h() -> bool {
    if Path::new("webview.h").exists() {
        return true;
    }
    println!("  下载 webview.h...");
    let Ok(output) = Command::new("curl")
        .args(["-fsSL", "-o", "webview.h", WEBVIEW_H_URL])
        .output() else { return false };
    output.status.success() || Path::new("webview.h").exists()
}

fn cmd_new(name: &str, url: &str, targets: &[&str]) {
    if Path::new(name).exists() {
        eprintln!("错误: 目录 '{}' 已存在", name);
        std::process::exit(1);
    }

    println!("🚀 创建项目: {}", name);
    fs::create_dir_all(format!("{}/src", name)).unwrap();

    let mut vars = HashMap::new();
    vars.insert("NAME", name);
    vars.insert("URL", url);

    // C 主文件
    write_file(&format!("{}/src/main.c", name), &template_replace(TEMPLATE_MAIN_C, &vars));

    // HTML
    write_file(&format!("{}/index.html", name), &template_replace(TEMPLATE_HTML, &vars));

    // WASM 模式
    write_file(&format!("{}/wasm.html", name), &template_replace(TEMPLATE_WASM_HTML, &vars));

    // xmake.lua
    write_file(&format!("{}/xmake.lua", name), &template_replace(TEMPLATE_XMAKE, &vars));

    // CMakeLists.txt
    write_file(&format!("{}/CMakeLists.txt", name), &template_replace(TEMPLATE_CMAKE, &vars));

    // 下载 webview.h
    std::env::set_current_dir(name).ok();
    fetch_webview_h();
    std::env::set_current_dir("..").ok();

    // Android
    if targets.contains(&"android") || targets.contains(&"all") {
        let android_java = template_replace(TEMPLATE_ANDROID_JAVA, &vars);
        write_file(&format!("{}/android/java/local/mc/console/MainActivity.java", name), &android_java);
        write_file(&format!("{}/android/AndroidManifest.xml", name), &format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="local.mc.console" android:versionCode="1" android:versionName="1.0">
    <uses-sdk android:minSdkVersion="24" android:targetSdkVersion="36"/>
    <uses-permission android:name="android.permission.INTERNET"/>
    <application android:label="{NAME}" android:usesCleartextTraffic="true"
        android:theme="@android:style/Theme.Material.Light.NoActionBar">
        <activity android:name=".MainActivity" android:exported="true">
            <intent-filter>
                <action android:name="android.intent.action.MAIN"/>
                <category android:name="android.intent.category.LAUNCHER"/>
            </intent-filter>
        </activity>
    </application>
</manifest>"#, NAME = name));
    }

    // README
    write_file(&format!("{}/README.md", name), &format!(
        r#"# {NAME}

一键生成的 WebView 跨平台应用。

## 构建

```bash
# Windows
lyco build

# 运行
lyco run

# Android
lyco pack --target android

# WASM (纯前端)
lyco serve
```

## 项目结构

```
{NAME}/
├── src/main.c          # C 主文件
├── index.html          # WebView 内容
├── wasm.html           # WASM 模式
├── xmake.lua           # xmake 构建
├── CMakeLists.txt      # CMake 构建
├── android/            # Android 构建
└── webview.h           # WebView 头文件
```

Generated by lyco-webview v0.1.0
"#, NAME = name));

    println!("✅ 项目 '{}' 已创建", name);
    println!("  进入: cd {}", name);
    println!("  构建: lyco build");
    println!("  运行: lyco run");
}

fn cmd_build() {
    println!("🔨 构建...");
    if Path::new("xmake.lua").exists() {
        let status = Command::new("xmake").status().expect("xmake 未安装");
        if !status.success() { std::process::exit(1); }
    } else if Path::new("CMakeLists.txt").exists() {
        fs::create_dir_all("build").ok();
        Command::new("cmake").args([".."]).current_dir("build").status().ok();
        let status = Command::new("cmake").args(["--build", "."]).current_dir("build").status().expect("cmake 未安装");
        if !status.success() { std::process::exit(1); }
    } else {
        eprintln!("错误: 未找到 xmake.lua 或 CMakeLists.txt");
        std::process::exit(1);
    }
    println!("✅ 构建完成");
}

fn cmd_run() {
    cmd_build();
    println!("▶ 运行...");
    if Path::new("xmake.lua").exists() {
        Command::new("xmake").arg("run").status().ok();
    } else if Path::new("build").exists() {
        // 尝试找 exe 并运行
        for entry in fs::read_dir("build").unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".exe") {
                Command::new(entry.path()).status().ok();
                break;
            }
        }
    }
}

fn cmd_serve() {
    println!("🌐 WASM 模式服务...");
    let port = std::env::var("PORT").unwrap_or("8080".to_string());
    println!("  打开 http://localhost:{}", port);
    Command::new("python").args(["-m", "http.server", &port]).status().ok();
}

fn cmd_pack(target: &str) {
    println!("📦 打包 target={}", target);
    match target {
        "android" => {
            if !Path::new("android").exists() {
                eprintln!("错误: android/ 目录不存在");
                std::process::exit(1);
            }
            println!("  cd android && bash build_apk.sh");
        }
        "windows" => {
            cmd_build();
            println!("  产物: build/windows/x64/release/");
        }
        "macos" | "linux" => {
            cmd_build();
            println!("  产物: build/{}/x64/release/", target);
        }
        "wasm" => {
            cmd_serve();
        }
        _ => {
            eprintln!("未知 target: {}", target);
            eprintln!("  支持: android, windows, macos, linux, wasm");
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("lyco - 一键生成跨平台 WebView 项目");
        println!();
        println!("用法:");
        println!("  lyco new <name> [url] [--targets windows,android,macos,linux,wasm]");
        println!("  lyco build");
        println!("  lyco run");
        println!("  lyco serve              # WASM 模式");
        println!("  lyco pack --target <target>");
        return;
    }

    match args[1].as_str() {
        "new" => {
            let name = args.get(2).expect("需要项目名称").as_str();
            let url = args.get(3).map(|s| s.as_str()).unwrap_or("https://example.com");
            let targets: Vec<&str> = args.iter()
                .find(|a| a.starts_with("--targets="))
                .map(|a| a.trim_start_matches("--targets=").split(',').collect())
                .unwrap_or_else(|| vec!["windows", "android", "wasm"]);
            cmd_new(name, url, &targets);
        }
        "build" => cmd_build(),
        "run" => cmd_run(),
        "serve" => cmd_serve(),
        "pack" => {
            let target = args.iter()
                .find(|a| a.starts_with("--target="))
                .map(|a| a.trim_start_matches("--target="))
                .unwrap_or("windows");
            cmd_pack(target);
        }
        _ => {
            eprintln!("未知命令: {}", args[1]);
        }
    }
}
