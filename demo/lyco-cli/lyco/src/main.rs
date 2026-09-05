use std::collections::HashMap;
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
fn db_path()      -> PathBuf { data_dir().join("lyco.db") }

// ── Lyco.toml (cargo 风格清单) ───────────────────────────────
mod manifest;

// ── 数据库 ───────────────────────────────────────────────────
mod db {
    use super::*;
    use rusqlite::Connection;
    use std::sync::Mutex;

    static DB: once_cell::sync::Lazy<Mutex<Option<Connection>>> =
        once_cell::sync::Lazy::new(|| Mutex::new(None));

    fn get() -> std::sync::MutexGuard<'static, Option<Connection>> {
        DB.lock().unwrap()
    }

    pub fn init() {
        let mut db = get();
        if db.is_none() {
            let _ = fs::create_dir_all(data_dir());
            let conn = Connection::open(db_path()).expect("数据库打开失败");
            conn.execute_batch("
                CREATE TABLE IF NOT EXISTS projects (
                    name TEXT PRIMARY KEY,
                    lang TEXT NOT NULL,
                    url TEXT,
                    path TEXT NOT NULL,
                    targets TEXT,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    last_open DATETIME
                );
                CREATE TABLE IF NOT EXISTS builds (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    project_name TEXT,
                    target TEXT,
                    status TEXT DEFAULT 'pending',
                    log TEXT,
                    duration_ms INTEGER,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );
                CREATE TABLE IF NOT EXISTS plugins (
                    name TEXT PRIMARY KEY,
                    version TEXT,
                    path TEXT,
                    enabled INTEGER DEFAULT 1,
                    description TEXT
                );
                CREATE TABLE IF NOT EXISTS config (
                    key TEXT PRIMARY KEY,
                    value TEXT
                );
            ").expect("数据库初始化失败");
            *db = Some(conn);
        }
    }

    pub fn add_project(name: &str, lang: &str, url: &str, path: &str, targets: &[String]) {
        init();
        let db = get();
        let conn = db.as_ref().unwrap();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO projects (name, lang, url, path, targets) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![name, lang, url, path, targets.join(",")],
        );
    }

    pub fn list_projects() -> Vec<(String, String, String, String)> {
        init();
        let db = get();
        let conn = db.as_ref().unwrap();
        let mut stmt = conn.prepare(
            "SELECT name, lang, path, created_at FROM projects ORDER BY last_open DESC"
        ).unwrap_or_else(|_| conn.prepare("SELECT name, lang, path, created_at FROM projects").unwrap());
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_,String>(0)?, row.get::<_,String>(1)?, row.get::<_,String>(2)?, row.get::<_,String>(3)?))
        });
        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => vec![],
        }
    }

    pub fn get_config(key: &str) -> Option<String> {
        init();
        let db = get();
        let conn = db.as_ref().unwrap();
        conn.query_row("SELECT value FROM config WHERE key = ?", [key], |row| row.get(0)).ok()
    }

    pub fn set_config(key: &str, value: &str) {
        init();
        let db = get();
        let conn = db.as_ref().unwrap();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO config (key, value) VALUES (?, ?)",
            [key, value],
        );
    }
}

// ── 模板引擎 (Tera) ──────────────────────────────────────────
mod tmpl {
    use super::*;
    use tera::{Tera, Context, Result};

    pub fn render(template_str: &str, vars: &HashMap<&str, &str>) -> Result<String> {
        let mut tera = Tera::default();
        let mut ctx = Context::new();
        for (k, v) in vars { ctx.insert(&**k, *v); }
        tera.render_str(template_str, &ctx)
    }

    pub fn render_file(path: &Path, vars: &HashMap<&str, &str>) -> Result<String> {
        let mut tera = Tera::new(&format!("{}", path.display())).unwrap_or_default();
        let mut ctx = Context::new();
        for (k, v) in vars { ctx.insert(&**k, *v); }
        let name = path.file_name().unwrap().to_str().unwrap();
        tera.render(name, &ctx)
    }
}

// ── 内置模板 ─────────────────────────────────────────────────
static TEMPLATE_MAIN_C: &str = include_str!("../templates/main.c");
static TEMPLATE_XMAKE: &str = include_str!("../templates/xmake.lua");
static TEMPLATE_HTML: &str = include_str!("../templates/index.html");
static TEMPLATE_MAIN_PY: &str = include_str!("../templates/main.py");
static TEMPLATE_MAIN_GO: &str = include_str!("../templates/main.go");
static TEMPLATE_MAIN_RS: &str = include_str!("../templates/main.rs");
static TEMPLATE_MAIN_TS: &str = include_str!("../templates/main.ts");
static TEMPLATE_MAIN_JAVA: &str = include_str!("../templates/Main.java");
static TEMPLATE_MAIN_ZIG: &str = include_str!("../templates/main.zig");
static TEMPLATE_MAIN_CS: &str = include_str!("../templates/Program.cs");
static TEMPLATE_MAIN_E: &str = include_str!("../templates/main.e.txt");
static TEMPLATE_PKG_JSON: &str = include_str!("../templates/package.json");
static TEMPLATE_README: &str = include_str!("../templates/README.md");
static TEMPLATE_GITIGNORE: &str = include_str!("../templates/.gitignore");
static TEMPLATE_WEB_HTML: &str = include_str!("../templates/web.html");
static TEMPLATE_LYCO_TOML: &str = include_str!("../templates/Lyco.toml");

const ALL_TEMPLATES: &[(&str, &str)] = &[
    ("main.c", TEMPLATE_MAIN_C),
    ("Lyco.toml", TEMPLATE_LYCO_TOML),
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
    ("README.md", TEMPLATE_README),
];

fn release_templates(force: bool) -> std::io::Result<()> {
    let dir = templates_dir();
    fs::create_dir_all(&dir)?;
    for (name, content) in ALL_TEMPLATES {
        let p = dir.join(name);
        if force || !p.exists() { fs::write(&p, content)?; }
    }
    let web = web_dir();
    fs::create_dir_all(&web)?;
    let web_index = web.join("index.html");
    if force || !web_index.exists() { fs::write(&web_index, TEMPLATE_WEB_HTML)?; }
    Ok(())
}

fn ensure_initialized() {
    // 版本戳: lyco 升级后自动重释放模板 (用户自定义会被覆盖, 提示备份)
    let stamp = templates_dir().join(".version");
    let cur = env!("CARGO_PKG_VERSION");
    let stale = fs::read_to_string(&stamp).map(|v| v.trim() != cur).unwrap_or(true);
    if !templates_dir().exists() || stale {
        if templates_dir().exists() {
            println!("📦 lyco v{cur}: 模板已更新并重新释放 (自定义修改请先备份 ~/.lyco/templates/)");
        } else {
            println!("📦 首次运行,释放默认模板...");
        }
        release_templates(templates_dir().exists()).expect("释放失败");
        let _ = fs::create_dir_all(templates_dir());
        let _ = fs::write(templates_dir().join(".version"), cur);
    }
}

fn read_template_or_default(name: &str, default: &str) -> String {
    let p = templates_dir().join(name);
    if p.exists() { fs::read_to_string(p).unwrap_or_default() } else { default.to_string() }
}

fn subst(template: &str, vars: &[(&str, &str)]) -> String {
    // 模板统一用 {K} 单花括号占位 (Tera 会把 {K} 当纯文本, 故直接替换)
    let mut r = template.to_string();
    for (k, v) in vars { r = r.replace(&format!("{{{k}}}"), v); }
    r
}

fn write_file(path: &str, content: &str) {
    if let Some(p) = Path::new(path).parent() { let _ = fs::create_dir_all(p); }
    if fs::write(path, content).is_err() {
        eprintln!("写入失败: {path}");
        std::process::exit(1);
    }
}

// ── 插件发现 ─────────────────────────────────────────────────
fn find_external_cmd(name: &str) -> Option<PathBuf> {
    let dir = commands_dir();
    if !dir.exists() { return None; }
    let ext = if cfg!(windows) { "dll" } else { "so" };
    for entry in fs::read_dir(&dir).ok()? {
        let p = entry.ok()?.path();
        if p.extension().map(|e| e == ext).unwrap_or(false)
            && p.file_stem().map(|s| s == name).unwrap_or(false) {
            return Some(p);
        }
    }
    None
}

// ── 内置命令 ─────────────────────────────────────────────────
fn cmd_new(name: &str, url: &str, lang: &str) {
    if Path::new(name).exists() { eprintln!("目录已存在: {name}"); std::process::exit(1); }
    ensure_initialized();

    let vars = [("NAME", name), ("URL", url), ("DEBUG", "false"), ("YEAR", "2026")];
    let read = |f: &str, def: &str| read_template_or_default(f, def);

    match lang.to_lowercase().as_str() {
        "c" => {
            let _ = fs::create_dir_all(format!("{name}/src"));
            write_file(&format!("{name}/src/main.c"), &subst(&read("main.c", TEMPLATE_MAIN_C), &vars));
            // cargo 化: Lyco.toml 是唯一配置, xmake.lua 由 lyco build 生成
            write_file(&format!("{name}/Lyco.toml"), &subst(&read("Lyco.toml", TEMPLATE_LYCO_TOML), &vars));
        }
        "python" | "py" => {
            write_file(&format!("{name}/main.py"), &subst(&read("main.py", TEMPLATE_MAIN_PY), &vars));
            write_file(&format!("{name}/requirements.txt"), "pywebview>=4.0\n");
        }
        "typescript" | "ts" => {
            write_file(&format!("{name}/main.ts"), &subst(&read("main.ts", TEMPLATE_MAIN_TS), &vars));
            write_file(&format!("{name}/package.json"), &subst(&read("package.json", TEMPLATE_PKG_JSON), &vars));
        }
        "rust" | "rs" => {
            let _ = fs::create_dir_all(format!("{name}/src"));
            write_file(&format!("{name}/src/main.rs"), &subst(&read("main.rs", TEMPLATE_MAIN_RS), &vars));
            write_file(&format!("{name}/Cargo.toml"), &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nwebview = \"0.1\"\n"));
        }
        "go" => {
            write_file(&format!("{name}/main.go"), &subst(&read("main.go", TEMPLATE_MAIN_GO), &vars));
            write_file(&format!("{name}/go.mod"), &format!("module {name}\n\ngo 1.21\n"));
        }
        "java" => {
            let _ = fs::create_dir_all(format!("{name}/src"));
            write_file(&format!("{name}/src/Main.java"), &subst(&read("Main.java", TEMPLATE_MAIN_JAVA), &vars));
        }
        "zig" => write_file(&format!("{name}/main.zig"), &subst(&read("main.zig", TEMPLATE_MAIN_ZIG), &vars)),
        "csharp" | "cs" | "c#" => write_file(&format!("{name}/Program.cs"), &subst(&read("Program.cs", TEMPLATE_MAIN_CS), &vars)),
        "e" | "el" | "易语言" => write_file(&format!("{name}/main.e"), &subst(&read("main.e", TEMPLATE_MAIN_E), &vars)),
        _ => { eprintln!("不支持: {lang}"); std::process::exit(1); }
    }

    write_file(&format!("{name}/index.html"), &subst(&read("index.html", TEMPLATE_HTML), &vars));
    write_file(&format!("{name}/README.md"), &subst(&read("README.md", TEMPLATE_README), &vars));
    write_file(&format!("{name}/.gitignore"), &read(".gitignore", TEMPLATE_GITIGNORE));

    // 持久化到数据库
    db::add_project(name, lang, url, &std::fs::canonicalize(name).unwrap_or_default().to_string_lossy(), &[lang.to_string()]);

    println!("✅ 已创建 {name} ({lang})");
    println!("  cd {name} && lyco run");
}

// 解析 build/run 通用旗标: -r/--release, --target <plat>
fn parse_build_flags(cmd_args: &[String]) -> (bool, Option<String>) {
    let mut release = false;
    let mut target = None;
    let mut i = 0;
    while i < cmd_args.len() {
        match cmd_args[i].as_str() {
            "-r" | "--release" => release = true,
            "--target" => { i += 1; target = cmd_args.get(i).cloned(); }
            _ => {}
        }
        i += 1;
    }
    (release, target)
}

fn cmd_build(cmd_args: &[String]) {
    let (release, target) = parse_build_flags(cmd_args);
    if Path::new(manifest::MANIFEST).exists() {
        match manifest::build(release, target.as_deref()) {
            Ok(()) => println!("✅ 完成"),
            Err(e) => { eprintln!("❌ {e}"); std::process::exit(1); }
        }
        return;
    }
    println!("🔨 构建...");
    if Path::new("xmake.lua").exists() { let _ = Command::new("xmake").status(); }
    else if Path::new("CMakeLists.txt").exists() {
        let _ = fs::create_dir_all("build");
        let _ = Command::new("cmake").args([".."]).current_dir("build").status();
        let _ = Command::new("cmake").args(["--build", "."]).current_dir("build").status();
    }
    println!("✅ 完成");
}

fn cmd_run(cmd_args: &[String]) {
    if Path::new(manifest::MANIFEST).exists() {
        let (release, target) = parse_build_flags(cmd_args);
        if let Err(e) = manifest::run(release, target.as_deref()) {
            eprintln!("❌ {e}"); std::process::exit(1);
        }
        return;
    }
    cmd_build(&[]);
    println!("▶ 运行...");
    let _ = Command::new("xmake").arg("run").status();
}

fn cmd_add(dep: &str) {
    if let Err(e) = manifest::add(dep) { eprintln!("❌ {e}"); std::process::exit(1); }
    println!("  下一步: lyco build");
}

fn cmd_remove(dep: &str) {
    if let Err(e) = manifest::remove(dep) { eprintln!("❌ {e}"); std::process::exit(1); }
}

fn cmd_clean() {
    let dirs = ["build", ".xmake"];
    for d in dirs {
        if Path::new(d).exists() {
            let _ = fs::remove_dir_all(d);
            println!("🧹 已清除 {d}/");
        }
    }
}

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
    println!("📁 {}", d.display());
    if d.exists() {
        let t = fs::read_dir(templates_dir()).map(|r| r.count()).unwrap_or(0);
        let c = fs::read_dir(commands_dir()).map(|r| r.count()).unwrap_or(0);
        println!("  模板: {t} 文件 | 外部命令: {c} 文件");
    }
}

fn cmd_list() {
    println!("内置命令: new build run web reset info list");
    let dir = commands_dir();
    if dir.exists() {
        let ext = if cfg!(windows) { "dll" } else { "so" };
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.extension().map(|x| x == ext).unwrap_or(false) {
                println!("  外部: {} ({})", p.file_stem().unwrap().to_string_lossy(), p.display());
            }
        }
    }
}

fn print_help() {
    print!(r#"lyco v1.1.0 - cargo 风格的跨平台 WebView 项目管理器 (Lyco.toml + xmake)

用法: lyco <command> [args]

命令:
  new <name> <lang> [url]       新建项目 (含 Lyco.toml)
  add <dep>[@<ver>]             添加依赖, 例: lyco add webview-capi@1.0
  remove <dep>                  移除依赖
  build [-r] [--target <plat>]  构建 (-r = release; plat: windows/linux/macos/android/wasm)
  run [-r] [--target <plat>]    构建 + 运行
  clean                         清除构建产物
  web                           可视化 Web UI
  reset                         重置 ~/.lyco/
  info / list                   配置信息 / 列出命令

Lyco.toml (与 Cargo.toml 同风格):
  [package]
  name = "hello"
  version = "0.1.0"

  [dependencies]
  webview-capi = "*"                      # WebView 窗口 (自动镜像源+系统库)
  webui = {{ version = "*" }}              # WebUI, 任意浏览器做前端

语言: c, python, typescript, rust, go, java, zig, c#, e(易语言)
插件: 将 .{} 放入 ~/.lyco/commands/ 扩展
模板: 编辑 ~/.lyco/templates/ 定制
"#,
        if cfg!(windows) { "dll" } else { "so" }
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // 双击检测:无参数时启动 Web UI
    if args.len() < 2 {
        ensure_initialized();
        let port = env::var("PORT").unwrap_or_else(|_| "8080".into());
        let url = format!("http://localhost:{port}");
        println!("🌐 Lyco WebView Studio 启动中...");
        println!("   打开浏览器访问: {url}");
        println!("   按 Ctrl+C 退出");

        // 先启动 HTTP 服务(后台),再打开浏览器
        let port_clone = port.clone();
        let web_dir_clone = web_dir();
        std::thread::spawn(move || {
            let _ = Command::new("python")
                .args(["-m", "http.server", &port_clone])
                .current_dir(web_dir_clone)
                .status();
        });

        // 等待服务启动
        std::thread::sleep(std::time::Duration::from_millis(500));

        // 打开浏览器
        #[cfg(windows)]
        {
            // 尝试 Edge (Windows 10/11 自带)
            let edge_paths = [
                "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
                "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
            ];
            let mut opened = false;
            for path in &edge_paths {
                if std::path::Path::new(path).exists() {
                    if Command::new(path).arg(&url).spawn().is_ok() {
                        opened = true;
                        break;
                    }
                }
            }
            if !opened {
                // 兜底: cmd /c start
                let _ = Command::new("cmd").args(["/c", "start", "", &url]).spawn();
            }
        }
        #[cfg(not(windows))]
        {
            let _ = Command::new("xdg-open").arg(&url).spawn();
        }

        // 阻塞主线程保持运行
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    let cmd = args[1].as_str();
    let cmd_args: &[String] = &args[2..];

    // 优先外部命令
    if let Some(p) = find_external_cmd(cmd) {
        let ext = if cfg!(windows) { "exe" } else { "" };
        let exe = if cfg!(windows) { p.with_extension("exe") } else { p.clone() };
        if exe.exists() {
            let status = Command::new(&exe).args(cmd_args).status().unwrap();
            std::process::exit(status.code().unwrap_or(0));
        } else {
            eprintln!("找到插件 {}, 但缺少同名可执行文件", p.display());
        }
    }

    match cmd {
        "new" => {
            if cmd_args.len() < 2 { eprintln!("用法: lyco new <name> <lang> [url]"); std::process::exit(1); }
            let url = cmd_args.get(2).map(|s| s.as_str()).unwrap_or("https://example.com");
            cmd_new(&cmd_args[0], url, &cmd_args[1]);
        }
        "build" => cmd_build(cmd_args),
        "run"   => cmd_run(cmd_args),
        "add" => {
            if cmd_args.is_empty() { eprintln!("用法: lyco add <dep>[@<version>]   例: lyco add webview-capi@1.0"); std::process::exit(1); }
            cmd_add(&cmd_args[0]);
        }
        "remove" => {
            if cmd_args.is_empty() { eprintln!("用法: lyco remove <dep>"); std::process::exit(1); }
            cmd_remove(&cmd_args[0]);
        }
        "clean" => cmd_clean(),
        "web"   => cmd_web(),
        "reset" => cmd_reset(),
        "info"  => cmd_info(),
        "list"  => cmd_list(),
        _ => { print_help(); std::process::exit(1); }
    }
}
