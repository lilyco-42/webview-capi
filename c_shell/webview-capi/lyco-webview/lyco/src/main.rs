use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ── 配置目录 ─────────────────────────────────────────────────
fn data_dir() -> PathBuf {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".lyco")
}

fn templates_dir() -> PathBuf { data_dir().join("templates") }
fn commands_dir() -> PathBuf { data_dir().join("commands") }
fn web_dir()      -> PathBuf { data_dir().join("web") }

// ── 内置模板 ─────────────────────────────────────────────────
macro_rules! builtin_template {
    ($name:expr) => { include_str!(concat!("../templates/", $name)) };
}

static TEMPLATE_MAIN_C:       &str = builtin_template!("main.c");
static TEMPLATE_XMAKE:        &str = builtin_template!("xmake.lua");
static TEMPLATE_HTML:         &str = builtin_template!("index.html");
static TEMPLATE_MAIN_PY:      &str = builtin_template!("main.py");
static TEMPLATE_MAIN_GO:      &str = builtin_template!("main.go");
static TEMPLATE_MAIN_RS:      &str = builtin_template!("main.rs");
static TEMPLATE_MAIN_TS:      &str = builtin_template!("main.ts");
static TEMPLATE_MAIN_JAVA:    &str = builtin_template!("Main.java");
static TEMPLATE_MAIN_ZIG:     &str = builtin_template!("main.zig");
static TEMPLATE_MAIN_CS:      &str = builtin_template!("Program.cs");
static TEMPLATE_MAIN_E:       &str = builtin_template!("main.e.txt");
static TEMPLATE_PKG_JSON:     &str = builtin_template!("package.json");
static TEMPLATE_ANDROID_XML:  &str = builtin_template!("android_manifest.xml");
static TEMPLATE_ANDROID_SH:   &str = builtin_template!("build_apk.sh");
static TEMPLATE_README:       &str = builtin_template!("README.md");
static TEMPLATE_GITIGNORE:    &str = builtin_template!(".gitignore");
static DEFAULT_WEB_UI:        &str = builtin_template!("web.html");

// ── 工具函数 ─────────────────────────────────────────────────
fn subst(s: &str, vars: &[(&str, &str)]) -> String {
    let mut r = s.to_string();
    for (k, v) in vars { r = r.replace(&format!("{{{k}}}"), v); }
    r
}

fn write(path: &str, content: &str) {
    if let Some(p) = Path::new(path).parent() { let _ = fs::create_dir_all(p); }
    if fs::write(path, content).is_err() {
        eprintln!("写入失败: {path}");
        std::process::exit(1);
    }
}

fn read_user_or_default(user_path: &Path, default: &str) -> String {
    if user_path.exists() {
        fs::read_to_string(user_path).unwrap_or_default()
    } else {
        default.to_string()
    }
}

// ── 首次释放 ─────────────────────────────────────────────────
const ALL_TEMPLATES: &[(&str, &str)] = &[
    ("main.c", TEMPLATE_MAIN_C),
    ("xmake.lua", TEMPLATE_XMAKE),
    ("index.html", TEMPLATE_HTML),
    ("main.py", TEMPLATE_MAIN_PY),
    ("main.go", TEMPLATE_MAIN_GO),
    ("main.rs", TEMPLATE_MAIN_RS),
    ("main.ts", TEMPLATE_MAIN_TS),
    ("Main.java", TEMPLATE_MAIN_JAVA),
    ("main.zig", TEMPLATE_MAIN_ZIG),
    ("Program.cs", TEMPLATE_MAIN_CS),
    ("main.e", TEMPLATE_MAIN_E),
    ("package.json", TEMPLATE_PKG_JSON),
    ("android_manifest.xml", TEMPLATE_ANDROID_XML),
    ("build_apk.sh", TEMPLATE_ANDROID_SH),
    ("README.md", TEMPLATE_README),
    (".gitignore", TEMPLATE_GITIGNORE),
];

fn release_templates() -> std::io::Result<()> {
    let dir = templates_dir();
    fs::create_dir_all(&dir)?;
    for (name, content) in ALL_TEMPLATES {
        let p = dir.join(name);
        if !p.exists() { write(&p.to_string_lossy(), content); }
    }
    // Web UI
    let web = web_dir();
    fs::create_dir_all(&web)?;
    let web_index = web.join("index.html");
    if !web_index.exists() { write(&web_index.to_string_lossy(), DEFAULT_WEB_UI); }
    Ok(())
}

fn ensure_initialized() {
    if !templates_dir().exists() {
        println!("📦 首次运行,释放默认模板到 ~/.lyco/");
        release_templates().expect("释放模板失败");
    }
}

// ── 内置命令 ─────────────────────────────────────────────────
fn cmd_new(name: &str, url: &str, lang: &str) {
    if Path::new(name).exists() { eprintln!("目录已存在: {name}"); std::process::exit(1); }
    ensure_initialized();

    let user = |f: &str| templates_dir().join(f);
    let vars = [("NAME", name), ("URL", url), ("DEBUG", "false")];
    let read = |f: &str, def: &str| read_user_or_default(&user(f), def);

    match lang.to_lowercase().as_str() {
        "c" => {
            let _ = fs::create_dir_all(format!("{name}/src"));
            write(&format!("{name}/src/main.c"), &subst(&read("main.c", TEMPLATE_MAIN_C), &vars));
            write(&format!("{name}/xmake.lua"), &subst(&read("xmake.lua", TEMPLATE_XMAKE), &vars));
        }
        "python" | "py" => {
            write(&format!("{name}/main.py"), &subst(&read("main.py", TEMPLATE_MAIN_PY), &vars));
            write(&format!("{name}/requirements.txt"), "pywebview>=4.0\n");
        }
        "typescript" | "ts" => {
            write(&format!("{name}/main.ts"), &subst(&read("main.ts", TEMPLATE_MAIN_TS), &vars));
            write(&format!("{name}/package.json"), &subst(&read("package.json", TEMPLATE_PKG_JSON), &vars));
        }
        "rust" | "rs" => {
            let _ = fs::create_dir_all(format!("{name}/src"));
            write(&format!("{name}/src/main.rs"), &subst(&read("main.rs", TEMPLATE_MAIN_RS), &vars));
            write(&format!("{name}/Cargo.toml"), &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nwebview = \"0.1\"\n"));
        }
        "go" => {
            write(&format!("{name}/main.go"), &subst(&read("main.go", TEMPLATE_MAIN_GO), &vars));
            write(&format!("{name}/go.mod"), &format!("module {name}\n\ngo 1.21\n"));
        }
        "java" => {
            let _ = fs::create_dir_all(format!("{name}/src"));
            write(&format!("{name}/src/Main.java"), &subst(&read("Main.java", TEMPLATE_MAIN_JAVA), &vars));
        }
        "zig" => {
            write(&format!("{name}/main.zig"), &subst(&read("main.zig", TEMPLATE_MAIN_ZIG), &vars));
        }
        "csharp" | "cs" | "c#" => {
            write(&format!("{name}/Program.cs"), &subst(&read("Program.cs", TEMPLATE_MAIN_CS), &vars));
        }
        "e" | "el" | "易语言" => {
            write(&format!("{name}/main.e"), &subst(&read("main.e", TEMPLATE_MAIN_E), &vars));
        }
        _ => {
            eprintln!("不支持: {lang}");
            eprintln!("支持: c, python, typescript, rust, go, java, zig, csharp, e");
            std::process::exit(1);
        }
    }

    write(&format!("{name}/index.html"),   &subst(&read("index.html", TEMPLATE_HTML), &vars));
    write(&format!("{name}/README.md"),     &subst(&read("README.md", TEMPLATE_README), &vars));
    write(&format!("{name}/.gitignore"),    &read(".gitignore", TEMPLATE_GITIGNORE));

    println!("✅ 已创建 {name} ({lang})");
    println!("  cd {name} && lyco run");
}

fn cmd_build() {
    println!("🔨 构建...");
    if Path::new("xmake.lua").exists() { let _ = Command::new("xmake").status(); }
    else if Path::new("CMakeLists.txt").exists() {
        let _ = fs::create_dir_all("build");
        let _ = Command::new("cmake").args([".."]).current_dir("build").status();
        let _ = Command::new("cmake").args(["--build", "."]).current_dir("build").status();
    }
    println!("✅ 完成");
}

fn cmd_run() { cmd_build(); println!("▶ 运行..."); let _ = Command::new("xmake").arg("run").status(); }

fn cmd_web() {
    ensure_initialized();
    let port = env::var("PORT").unwrap_or_else(|_| "8080".into());
    println!("🌐 http://localhost:{port}  (文件: {})", web_dir().display());
    let _ = Command::new("python").args(["-m","http.server",&port]).current_dir(web_dir()).status();
}

fn cmd_reset() {
    if data_dir().exists() { let _ = fs::remove_dir_all(data_dir()); }
    println!("✅ 已重置 ~/.lyco/");
}

fn cmd_info() {
    let d = data_dir();
    if d.exists() {
        let t = templates_dir().read_dir().map(|r| r.count()).unwrap_or(0);
        let c = commands_dir().read_dir().map(|r| r.count()).unwrap_or(0);
        println!("📁 {} (模板: {t}, 外部命令: {c})", d.display());
    } else {
        println!("未初始化,运行 lyco new 自动创建");
    }
}

fn cmd_list() {
    println!("内置命令: new build run web reset info list");
    let dir = commands_dir();
    if dir.exists() {
        let ext = if cfg!(windows) { "dll" } else { "so" };
        for e in fs::read_dir(dir).unwrap() {
            if let Ok(e) = e {
                let p = e.path();
                if p.extension().map(|x| x == ext).unwrap_or(false) {
                    println!("  外部: {} ({})", p.file_stem().unwrap().to_string_lossy(), p.display());
                }
            }
        }
    }
}

// ── 外部命令查找 ─────────────────────────────────────────────
fn find_external(cmd: &str) -> Option<PathBuf> {
    let dir = commands_dir();
    if !dir.exists() { return None; }
    let ext = if cfg!(windows) { "dll" } else { "so" };
    for entry in fs::read_dir(&dir).ok()? {
        let p = entry.ok()?.path();
        if p.extension().map(|e| e == ext).unwrap_or(false)
            && p.file_stem().map(|s| s == cmd).unwrap_or(false) {
            return Some(p);
        }
    }
    None
}

// ── 主入口 ───────────────────────────────────────────────────
fn print_help() {
    print!(r#"lyco v0.5.0 - 跨平台 WebView 项目生成器 (插件架构)

用法: lyco <command> [args]

内置命令:
  new <name> <lang> [url]   新建项目
  build                     构建
  run                       构建 + 运行
  web                       启动可视化 Web UI
  reset                     重置为默认模板
  info                      查看配置
  list                      列出所有可用命令

支持语言: c, python, typescript, rust, go, java, zig, c#, e(易语言)
支持平台: windows, android, macos, linux, wasm

插件扩展:
  将 .{} 文件放入 ~/.lyco/commands/
  例: new.{} => lyco new 时优先加载
  第三方工具可完全替代内置命令

自定义模板: 编辑 ~/.lyco/templates/ 下的文件
"#,
        if cfg!(windows) { "dll" } else { "so" },
        if cfg!(windows) { "dll" } else { "so" }
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 { print_help(); return; }

    let cmd = args[1].as_str();
    let cmd_args: &[String] = &args[2..];

    // 优先加载外部命令 (SO/DLL)
    if let Some(lib_path) = find_external(cmd) {
        // 外部命令约定: 同名的可执行文件
        let exe_path = if cfg!(windows) {
            commands_dir().join(format!("{cmd}.exe"))
        } else {
            commands_dir().join(cmd)
        };
        if exe_path.exists() {
            let status = Command::new(&exe_path)
                .args(cmd_args)
                .status()
                .unwrap_or_else(|e| { eprintln!("加载外部命令失败: {e}"); std::process::exit(1); });
            std::process::exit(status.code().unwrap_or(0));
        } else {
            eprintln!("找到插件 {} 但缺少同名可执行文件", lib_path.display());
            eprintln!("提示: 将 {} 放入 ~/.lyco/commands/", exe_path.display());
        }
    }

    // 内置命令
    match cmd {
        "new" => {
            if cmd_args.len() < 2 { eprintln!("用法: lyco new <name> <lang> [url]"); std::process::exit(1); }
            let url = cmd_args.get(2).map(|s| s.as_str()).unwrap_or("https://example.com");
            cmd_new(&cmd_args[0], url, &cmd_args[1]);
        }
        "build" => cmd_build(),
        "run"   => cmd_run(),
        "web"   => cmd_web(),
        "reset" => cmd_reset(),
        "info"  => cmd_info(),
        "list"  => cmd_list(),
        _ => { print_help(); std::process::exit(1); }
    }
}
