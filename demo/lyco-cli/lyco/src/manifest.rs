// Lyco.toml — cargo 风格清单: 解析 / xmake.lua 生成 / add / remove / build / run
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const MANIFEST: &str = "Lyco.toml";
const MIRROR: &str = "https://github.com/lilyco-42/xmake-mirror.git";

// ── 已知依赖注册表: 傻瓜式, 自动补 Windows 系统库 ──────────────
fn known_syslinks(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "webview-capi" | "webview-mini" | "webview" => {
            Some(&["user32", "shell32", "ole32", "oleaut32", "shlwapi", "version"])
        }
        "webui" => Some(&["ws2_32", "user32", "gdi32", "shell32", "ole32"]),
        _ => None,
    }
}

/// 构建后需拷到 exe 旁边的运行时 DLL
fn known_dll(name: &str) -> Option<&'static str> {
    match name {
        "webview-capi" => Some("webview.dll"),
        _ => None,
    }
}

// ── 注册表 (lyco search) ────────────────────────────────────
pub const REGISTRY: &[(&str, &str)] = &[
    ("webview-capi", "WebView2/WKWebView/WebKitGTK 极小 C API (预编译, lyco 维护)"),
    ("webview-mini", "webview/webview 单头文件极简封装"),
    ("webview",      "webview/webview 官方库"),
    ("webui",        "WebUI — 用任意浏览器做 GUI"),
];

pub fn search(q: &str) {
    let q = q.to_lowercase();
    let mut found = 0;
    for (n, d) in REGISTRY {
        if n.contains(&q) || d.to_lowercase().contains(&q) {
            println!("{n:<14} {d}");
            found += 1;
        }
    }
    if found == 0 {
        println!("(注册表无匹配 — 任意 xmake 包仍可用: lyco add <xmake包名>)");
    }
}

#[derive(serde::Deserialize, Default)]
pub struct Manifest {
    #[serde(default)]
    pub package: Pkg,
    #[serde(default)]
    pub dependencies: BTreeMap<String, toml::Value>,
}

#[derive(serde::Deserialize, Default)]
pub struct Pkg {
    pub name: Option<String>,
    pub version: Option<String>,
}

fn dep_version(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => s.clone(),
        toml::Value::Table(t) => {
            t.get("version").and_then(|x| x.as_str()).unwrap_or("*").to_string()
        }
        _ => "*".into(),
    }
}

/// 取 `{ git = "..." }` 里的 URL；不是表、或没有该键时返回 None。
///
/// 为什么是「当成包仓库」而不是「当成这个包的源」：
/// xmake 的 `add_requires` **没有** git 选项 —— 实测
/// `add_requires("p", {git = "…"})` 会警告
/// `has unknown option: {git=…}`；其合法选项在 xmake 源码
/// `modules/private/action/require/impl/package.lua` 的 `extra_options` 里写死：
/// plat / arch / kind / host / targetos / alias / group / system / option /
/// default / optional / debug / verify / external / private / build / configs /
/// version / public。
/// 从 git 拿包的唯一机制是 `add_repositories("<名字> <URL>")`，
/// 之后 `add_requires("<包名>")` 就能从那个仓里找到（已实测：用它拉
/// lyco-mirror 能解析出 `webview-capi 1.1.0`）。
fn dep_git(v: &toml::Value) -> Option<String> {
    match v {
        toml::Value::Table(t) => t.get("git").and_then(|x| x.as_str()).map(|s| s.to_string()),
        _ => None,
    }
}

/// 取 `{ path = "..." }` 里的原始值（未规范化）。
fn dep_path_raw(v: &toml::Value) -> Option<&str> {
    match v {
        toml::Value::Table(t) => t.get("path").and_then(|x| x.as_str()),
        _ => None,
    }
}

/// 把 `{ path = "..." }` 的目录规范化成 xmake 能认的绝对路径。
///
/// xmake 侧和 git 走同一个出口（`add_repositories("<名字> <目录>")`），
/// 但有两组实测得来的硬约束：
///
/// 1. **必须是绝对路径** —— 相对路径会被当成 git URL：
///      add_repositories("myrepo ../localrepo")
///      -> updating repositories .. error: fatal: repository '../localrepo' does not exist
/// 2. **不能带 `..`** —— 同理，`C:/a/b/../c` 也被当成 git URL：
///      fatal: 'C:/…/proj/../localrepo' does not appear to be a git repository
///
/// 所以这里自己做**词法**归一化：相对路径接上 CWD 变绝对、再把 `.`/`..` 消掉。
/// 特意**不用** `canonicalize`，它有两个副作用：
///   * Windows 上返回 `\\?\C:\…` 这种 verbatim 前缀，写进 Lua 字符串会报
///     `invalid escape sequence near '"myrepo \?\C'` —— 还得额外剥前缀；
///   * 目标不存在时直接失败，而报错路径（下面 `dep_path_resolve`）恰恰
///     需要在「不存在」时也能算出它本来该指向哪。
/// 词法归一化对 xmake 够用（它只要一个不含 `..` 的绝对路径，不要求真实存在）。
///
/// 另一个坑：Lua 字符串里 `\` 是转义符，所以最后统一换成正斜杠。
fn dep_path_resolve(raw: &str) -> String {
    let p = Path::new(raw);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        match std::env::current_dir() {
            Ok(cwd) => cwd.join(p),
            Err(_) => p.to_path_buf(),
        }
    };
    let mut out = PathBuf::new();
    for c in abs.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                // 别把根/盘符弹掉，否则 "C:/../x" 会变成盘符相对路径
                let at_root = matches!(
                    out.components().last(),
                    None | Some(std::path::Component::RootDir)
                        | Some(std::path::Component::Prefix(_))
                );
                if !at_root {
                    out.pop();
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out.to_string_lossy().replace('\\', "/")
}

fn dep_path_abs(raw: &str) -> String {
    dep_path_resolve(raw)
}

/// 校验 `{ path = "..." }` 指向的目录确实存在。
///
/// `add` 与 `gen_xmake_lua` **共用**这一个函数 —— 否则会出现
/// 「`lyco add` 打印 ✅、紧接着 `lyco build` 报 ❌」这种最气人的组合
/// （用户点了一个注定失败的按钮，还已经被告知成功）。
///
/// 报错里带上解析后的绝对路径：相对路径写错时，光看原文根本看不出
/// 它被解析到了哪，这是最难查的一类错。
fn check_dep_path(dep: &str, raw: &str) -> Result<(), String> {
    if Path::new(raw).exists() {
        return Ok(());
    }
    Err(format!(
        "依赖 {dep}: path = \"{raw}\" 不存在 (解析为 {})。\n\
         path 相对项目目录 (Lyco.toml 所在处), 要指向一个 xmake 包仓库 \
         (含 packages/<包名>/xmake.lua)",
        dep_path_resolve(raw)
    ))
}

fn sample_manifest() -> String {
    format!(
        "未找到 {MANIFEST}。最小示例:\n\n\
         [package]\nname = \"app\"\nversion = \"0.1.0\"\n\n\
         [dependencies]\nwebview-capi = \"*\"\n\n\
         或 lyco new <name> c 一键生成。"
    )
}

impl Manifest {
    pub fn load() -> Result<Manifest, String> {
        if !Path::new(MANIFEST).exists() {
            return Err(sample_manifest());
        }
        let s = fs::read_to_string(MANIFEST).map_err(|e| e.to_string())?;
        toml::from_str(&s).map_err(|e| format!("{MANIFEST} 解析失败: {e}"))
    }

    pub fn project_name(&self) -> String {
        self.package.name.clone().unwrap_or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "app".into())
        })
    }
}

// ── 源文件探测 ──────────────────────────────────────────────
fn detect_src_pattern() -> &'static str {
    let src = Path::new("src");
    if !src.exists() {
        return "src/**.c";
    }
    let has = |ext: &str| {
        fs::read_dir(src).map(|rd| {
            rd.filter_map(|e| e.ok())
                .any(|e| e.path().extension().map(|x| x == ext).unwrap_or(false))
        }).unwrap_or(false)
    };
    if has("cpp") || has("cxx") || has("cc") { "src/**.cpp" } else { "src/**.c" }
}

// ── 生成 xmake.lua (构建产物, 勿手编) ────────────────────────
pub fn gen_xmake_lua(m: &Manifest) -> Result<(), String> {
    let name = m.project_name();
    let mut s = String::from("-- generated by lyco from Lyco.toml, do not edit\n");
    s.push_str("add_rules(\"mode.debug\", \"mode.release\")\n");
    s.push_str(&format!("add_repositories(\"lyco-mirror {MIRROR}\")\n"));

    // `{ git = "..." }` / `{ path = "..." }` 的依赖：把那个 URL / 目录
    // 当成一个 xmake 包仓库加进来。必须在 add_requires 之前声明，
    // xmake 才找得到包（见 dep_git / dep_path_abs 的注释）。
    for (d, v) in &m.dependencies {
        let git = dep_git(v);
        let p = dep_path_raw(v);
        if git.is_some() && p.is_some() {
            return Err(format!("依赖 {d}: `git` 与 `path` 只能二选一, 不能同时写"));
        }
        if let Some(raw) = p {
            // 提前检查, 免得 xmake 给出一句难懂的错误。
            // 与 `lyco add` 共用同一个检查 —— 保证「add 说行」就等于「build 也行」。
            check_dep_path(d, raw)?;
            s.push_str(&format!("add_repositories(\"{d} {}\")\n", dep_path_abs(raw)));
        } else if let Some(url) = git {
            s.push_str(&format!("add_repositories(\"{d} {url}\")\n"));
        }
    }
    s.push('\n');

    let mut pkgs: Vec<String> = Vec::new();
    for (d, v) in &m.dependencies {
        let ver = dep_version(v);
        // xmake 版本约束语法: add_requires("zlib >=1.2"), "*" 只写包名
        let req = if ver == "*" || ver.is_empty() { d.clone() } else { format!("{d} >={ver}") };
        s.push_str(&format!("add_requires(\"{req}\")\n"));
        pkgs.push(d.clone());
    }

    // 依赖接线块 (主 target 与测试 target 共用)
    let syslinks: Vec<&str> = m.dependencies.keys()
        .filter_map(|d| known_syslinks(d))
        .flat_map(|sl| sl.iter().copied())
        .collect();
    let mut uniq: Vec<&str> = vec![];
    for s2 in &syslinks { if !uniq.contains(s2) { uniq.push(s2); } }
    let mut deps_block = String::new();
    if !pkgs.is_empty() {
        deps_block.push_str(&format!("    add_packages({})\n",
            pkgs.iter().map(|p| format!("\"{p}\"")).collect::<Vec<_>>().join(", ")));
    }
    if !uniq.is_empty() {
        deps_block.push_str("    if is_plat(\"windows\") then\n");
        deps_block.push_str(&format!("        add_syslinks({})\n",
            uniq.iter().map(|l| format!("\"{l}\"")).collect::<Vec<_>>().join(", ")));
        deps_block.push_str("    end\n");
    }
    // 运行时 DLL 自动跟随 exe (mingw 不拷贝包的 bin, 这里兜底)
    for d in m.dependencies.keys() {
        if let Some(dll) = known_dll(d) {
            deps_block.push_str(&format!(
                "    after_build(function (target)\n        local pkg = target:pkg(\"{d}\")\n        if pkg then\n            local dll = path.join(pkg:installdir(), \"bin\", \"{dll}\")\n            if os.isfile(dll) then os.cp(dll, target:targetdir()) end\n        end\n    end)\n"
            ));
        }
    }

    s.push_str(&format!("\ntarget(\"{name}\")\n    set_kind(\"binary\")\n"));
    s.push_str(&format!("    add_files(\"{}\")\n", detect_src_pattern()));
    s.push_str("    add_includedirs(\".\", \"src\")\n");
    s.push_str(&deps_block);
    s.push_str("    if is_mode(\"release\") then set_optimize(\"smallest\") end\ntarget_end()\n");

    // tests/*.c → 独立测试 target (lyco test 驱动, 不参与默认构建)
    if let Ok(rd) = fs::read_dir("tests") {
        for e in rd.filter_map(|e| e.ok()).collect::<Vec<_>>() {
            let p = e.path();
            if !matches!(p.extension().and_then(|x| x.to_str()), Some("c") | Some("cpp")) { continue; }
            let stem = p.file_stem().unwrap().to_string_lossy();
            let fname = p.file_name().unwrap().to_string_lossy();
            s.push_str(&format!(
                "\ntarget(\"test_{stem}\")\n    set_kind(\"binary\")\n    set_default(false)\n    add_files(\"tests/{fname}\")\n    add_includedirs(\".\", \"src\")\n"
            ));
            s.push_str(&deps_block);
            s.push_str("target_end()\n");
        }
    }
    fs::write("xmake.lua", s).map_err(|e| e.to_string())
}

// ── build / run ─────────────────────────────────────────────
fn plat(x: &str) -> Result<String, String> {
    Ok(match x.to_lowercase().as_str() {
        "windows" | "win" | "msvc" => "windows",
        "mingw" | "mingw64" | "gnu" => "mingw",
        "linux" => "linux",
        "macos" | "mac" | "darwin" => "macosx",
        "android" => "android",
        "ios" => "iphoneos",
        "wasm" | "wasi" => "wasm",
        other => return Err(format!("未知平台: {other} (windows/mingw/linux/macos/android/ios/wasm)")),
    }.to_string())
}

// ── 平台/工具链自动探测 ─────────────────────────────────────
/// 找 MinGW-w64 工具链根目录 (含 bin/gcc.exe)。找不到返回 None。
fn find_mingw_sdk() -> Option<String> {
    // 1) PATH 里的 gcc 若位于 <root>/bin/gcc.exe, 直接取 root (覆盖非 shim 安装)
    if let Ok(out) = Command::new("where").arg("gcc").output() {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
            let first = stdout.lines().next().unwrap_or("").trim();
            let p = Path::new(first);
            if p.file_name().map(|f| f == "gcc.exe").unwrap_or(false) {
                if let (Some(bin), Some(root)) = (p.parent(), p.parent().and_then(Path::parent)) {
                    if bin.file_name().map(|f| f == "bin").unwrap_or(false)
                        && root.join("bin").join("gcc.exe").exists() {
                        return Some(root.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }
    // 2) scoop 布局 (shims 里的 gcc 拿不到真实路径, 按约定目录探测)
    let candidates = [
        env::var("SCOOP").ok().map(|s| format!("{s}\\apps\\gcc\\current")),
        env::var("USERPROFILE").ok().map(|h| format!("{h}\\scoop\\apps\\gcc\\current")),
    ];
    for c in candidates.into_iter().flatten() {
        if Path::new(&c).join("bin").join("gcc.exe").exists() {
            return Some(c);
        }
    }
    None
}

/// xmake 配置的平台/工具链参数 (build 与 check 共用)
fn platform_args(target_plat: Option<&str>) -> Result<Vec<String>, String> {
    let mut v: Vec<String> = vec![];
    let plat_arg = target_plat.map(plat).transpose()?;
    if let Some(p) = plat_arg.as_deref() {
        v.push("-p".into());
        v.push(p.into());
        if p == "mingw" {
            if let Some(sdk) = find_mingw_sdk() {
                v.push(format!("--sdk={sdk}")); // xmake 只认等号形式
            }
        }
    } else if cfg!(windows) {
        // Windows 宿主默认: 优先 MinGW (MSVC 常缺 Windows SDK)
        if let Some(sdk) = find_mingw_sdk() {
            println!("🔧 工具链: MinGW-w64 ({sdk})");
            v.push("-p".into());
            v.push("mingw".into());
            v.push(format!("--sdk={sdk}"));
        } else {
            println!("🔧 工具链: MSVC (未发现 MinGW, 需要 Visual Studio C++ 工具集)");
            v.push("-p".into());
            v.push("windows".into());
        }
    }
    Ok(v)
}

/// `pub`：`main.rs` 里「没有 `Lyco.toml`、只有手写 `xmake.lua`」的回退分支也要用它
/// —— 那条路径原来是自己拼一条光杆 `Command::new("xmake")`，把 `-r` / `--target`
/// 整个丢掉了（`lyco build -r` 建的是 debug、`--target android` 建的是宿主平台，
/// 而两者都打印「✅ 完成」）。平台名映射（`plat`）只能有这一份，别再抄一遍。
pub fn xmake_config(release: bool, target_plat: Option<&str>) -> Result<(), String> {
    let mut conf = Command::new("xmake");
    conf.args(["f", "-y", "-m", if release { "release" } else { "debug" }]);
    for a in platform_args(target_plat)? {
        conf.arg(a);
    }
    // 别把真实的 `io::Error` 丢掉、换成一个**猜出来的结论**：`xmake` 起不来可能
    // 是 PATH 没配、目录不对、没有执行权限……直接报「未安装」是在替用户下结论，
    // 而且抹掉了唯一的线索。统一成 `run_step` 那套说法（「无法执行 <命令>: <真实错误>」），
    // 两条路径（有 `Lyco.toml` / 只有手写 `xmake.lua`）的错误文案也因此一致。
    let st = conf.status().map_err(|e| format!("无法执行 xmake: {e}"))?;
    if !st.success() { return Err("xmake 配置失败 (缺平台 SDK? 见上方输出)".into()); }
    Ok(())
}

pub fn build(release: bool, target_plat: Option<&str>) -> Result<(), String> {
    let m = Manifest::load()?;
    gen_xmake_lua(&m)?;
    println!("⚙ 已从 {MANIFEST} 生成 xmake.lua ({} 个依赖)", m.dependencies.len());
    xmake_config(release, target_plat)?;
    let st = Command::new("xmake").args(["-y"]).status()
        .map_err(|e| format!("无法执行 xmake: {e}"))?;
    if !st.success() { return Err("构建失败".into()); }
    Ok(())
}

pub fn run(release: bool, target_plat: Option<&str>) -> Result<(), String> {
    let m = Manifest::load()?;
    build(release, target_plat)?;
    println!("▶ 运行 {}...", m.project_name());
    let st = Command::new("xmake").arg("run").status()
        .map_err(|e| format!("无法执行 xmake run: {e}"))?;
    if !st.success() { return Err("运行失败".into()); }
    Ok(())
}

// ── add / remove (toml_edit 保注释保格式, 与 cargo 同款) ─────

/// 造一个内联表 `{ [version = "…", ] <key> = "<val>" }`，
/// 用来写 `dep = { version = "1.0", git = "…" }` 这种依赖。
fn dep_inline(ver: &str, key: &str, val: &str) -> toml_edit::InlineTable {
    let mut t = toml_edit::InlineTable::new();
    if !ver.is_empty() && ver != "*" {
        t.insert("version", toml_edit::Value::from(ver));
    }
    t.insert(key, toml_edit::Value::from(val));
    t
}

/// `lyco add <dep>[@<ver>] [--git <url> | --path <dir>]`
pub fn add(dep: &str, git: Option<&str>, path: Option<&str>) -> Result<(), String> {
    if !Path::new(MANIFEST).exists() { return Err(sample_manifest()); }
    if git.is_some() && path.is_some() {
        return Err("`--git` 与 `--path` 只能二选一".into());
    }
    let (name, ver) = match dep.split_once('@') {
        Some((n, v)) => (n.trim().to_string(), v.trim().to_string()),
        None => (dep.trim().to_string(), "*".to_string()),
    };
    let content = fs::read_to_string(MANIFEST).map_err(|e| e.to_string())?;
    let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| e.to_string())?;
    if doc.get("dependencies").is_none() {
        doc["dependencies"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    // 先把 path 校验掉再落盘：否则会打印 ✅、随后 `lyco build` 才报 ❌
    // （用户已经被告知成功，却拿不到能用的 xmake.lua）。
    if let Some(p) = path {
        check_dep_path(&name, p)?;
        // 软提示：xmake 包仓库的固定布局是 packages/<包名>/xmake.lua，
        // 没有 packages/ 说明这个 path 多半指向了普通源码仓。
        if !Path::new(p).join("packages").is_dir() {
            eprintln!(
                "⚠  {} 下没有 packages/ 目录 —— xmake 包仓库要求 packages/<包名>/xmake.lua，\
                 确认这个 path 指的是包仓库而不是源码仓",
                dep_path_resolve(p)
            );
        }
    }
    let item = if let Some(u) = git {
        toml_edit::value(dep_inline(ver.as_str(), "git", u))
    } else if let Some(p) = path {
        toml_edit::value(dep_inline("", "path", p))
    } else {
        toml_edit::value(ver.as_str())
    };
    doc["dependencies"][&name] = item;
    fs::write(MANIFEST, doc.to_string()).map_err(|e| e.to_string())?;
    let src = if git.is_some() { " (git 包仓库)" } else if path.is_some() { " (本地包仓库)" } else { "" };
    match known_syslinks(&name) {
        Some(sl) => println!("✅ 已添加 {name}{src} (自动镜像源 + 系统库 {})", sl.join(", ")),
        None => println!("✅ 已添加 {name}{src}"),
    }
    Ok(())
}

pub fn remove(dep: &str) -> Result<(), String> {
    if !Path::new(MANIFEST).exists() { return Err(sample_manifest()); }
    let content = fs::read_to_string(MANIFEST).map_err(|e| e.to_string())?;
    let mut doc = content.parse::<toml_edit::DocumentMut>().map_err(|e| e.to_string())?;
    let removed = doc
        .get_mut("dependencies")
        .and_then(|i| i.as_table_mut())
        .and_then(|t| t.remove(dep))
        .is_some();
    if !removed { return Err(format!("{MANIFEST} 中没有依赖 {dep}")); }
    fs::write(MANIFEST, doc.to_string()).map_err(|e| e.to_string())?;
    println!("✅ 已移除 {dep}");
    Ok(())
}

// ── check: 语法检查 (不产出目标文件) ────────────────────────
fn collect_includes(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 5 { return; }
    let rd = match fs::read_dir(dir) { Ok(r) => r, Err(_) => return };
    for e in rd.filter_map(|e| e.ok()) {
        let p = e.path();
        if p.is_dir() {
            if p.file_name().map(|n| n == "include").unwrap_or(false) {
                out.push(p.clone());
            }
            collect_includes(&p, depth + 1, out);
        }
    }
}

fn collect_src(dir: &Path, out: &mut Vec<PathBuf>) {
    let rd = match fs::read_dir(dir) { Ok(r) => r, Err(_) => return };
    for e in rd.filter_map(|e| e.ok()) {
        let p = e.path();
        if p.is_dir() {
            collect_src(&p, out);
        } else if matches!(p.extension().and_then(|x| x.to_str()), Some("c") | Some("cpp")) {
            out.push(p);
        }
    }
}

pub fn check() -> Result<(), String> {
    let m = Manifest::load()?;
    gen_xmake_lua(&m)?;
    xmake_config(false, None)?; // 配置+装包, 使包 include 目录就绪

    let mut incs: Vec<PathBuf> = vec![PathBuf::from("."), PathBuf::from("src")];
    if let Ok(base) = env::var("LOCALAPPDATA") {
        collect_includes(&PathBuf::from(base).join(".xmake/packages"), 0, &mut incs);
    }
    let gcc = find_mingw_sdk().map(|s| format!("{s}\\bin\\gcc.exe"));
    let prog = match gcc {
        Some(g) if Path::new(&g).exists() => g,
        _ => return Err("check 需要编译器: scoop install gcc (或安装完整 MSVC)".into()),
    };

    let mut files: Vec<PathBuf> = vec![];
    collect_src(Path::new("src"), &mut files);
    if files.is_empty() { return Err("src/ 下没有源文件".into()); }

    let mut bad = 0;
    for f in &files {
        let mut c = Command::new(&prog);
        c.arg("-fsyntax-only");
        for i in &incs { c.arg(format!("-I{}", i.display())); }
        c.arg(f);
        let out = c.output();
        let ok = out.as_ref().map(|o| o.status.success()).unwrap_or(false);
        println!("{} {}", if ok { "✔" } else { "✘" }, f.display());
        if !ok {
            if let Ok(o) = out {
                let err = String::from_utf8_lossy(&o.stderr);
                for line in err.lines().take(6) { println!("    {line}"); }
            }
            bad += 1;
        }
    }
    if bad > 0 { return Err(format!("{bad} 个文件未通过语法检查")); }
    println!("✅ 语法检查通过 ({} 个文件)", files.len());
    Ok(())
}

// ── test: tests/*.c → 每个文件一个测试 target 并逐个运行 ────
pub fn test() -> Result<(), String> {
    // 这里不用先 `Manifest::load()?` —— 紧接着的 `build()` 自己就会加载清单，
    // 失败时给出的是同一个错误。（原来那行是 `let m = Manifest::load()?;`，
    // 而那个 `m` 从头到尾没被用过。）
    build(false, None)?; // gen_xmake_lua 会为 tests/ 生成 test_* target

    // 原来这里是 `fs::read_dir("tests").map_err(|_| "没有 tests/ 目录 …")` ——
    // 把真实的 `io::Error` 丢掉、换成一个**猜出来的结论**。`tests` 存在但没有读
    // 权限、或者 `tests` 是个**文件**（`read_dir` 会报 ENOTDIR），都会被说成
    // 「没有 tests/ 目录」，而那句提示让用户去「放 tests/xxx.c 后重试」——
    // 他会照做，然后发现目录明明在。现在只有 `NotFound` 才说「没有」。
    let rd = match fs::read_dir("tests") {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err("没有 tests/ 目录 (放 tests/xxx.c 后重试)".into());
        }
        Err(e) => return Err(format!("无法读取 tests/ 目录: {e}")),
    };
    let mut ran = 0;
    let mut failed = 0;
    for e in rd.filter_map(|e| e.ok()) {
        let p = e.path();
        if !matches!(p.extension().and_then(|x| x.to_str()), Some("c") | Some("cpp")) { continue; }
        let stem = p.file_stem().unwrap().to_string_lossy().into_owned();
        println!("▶ 运行测试 {stem} ...");
        let st = Command::new("xmake").args(["run", &format!("test_{stem}")]).status();
        ran += 1;
        match st {
            Ok(s) if s.success() => println!("✔ {stem}"),
            _ => { println!("✘ {stem}"); failed += 1; }
        }
    }
    if ran == 0 { return Err("tests/ 下没有 .c/.cpp 测试文件".into()); }
    if failed > 0 { return Err(format!("{failed}/{ran} 个测试失败")); }
    println!("✅ {ran} 个测试全部通过");
    Ok(())
}

// ── install / uninstall: 构建并安装到 ~/.lyco/bin ───────────

/// 可执行文件后缀：Windows 是 `.exe`，其他平台是空串。
///
/// 这三处（`find_built_exe` 的匹配、`install` 的目标名、`uninstall` 的查找名）
/// 原来都把 `.exe` 写死。而 xmake 在 Linux/macOS 上产出的是
/// `build/<plat>/<arch>/<mode>/<名字>` —— **没有扩展名**。于是：
///   * `lyco install` 在 ubuntu/macOS 上**必然**报「构建产物未找到」，
///     而且那句错误信息还叫用户去找一个永远不存在的 `.exe`；
///   * 即使找到了，也会被装成 `<名字>.exe`，在 Unix 上是个看着就不对的文件名。
/// 用 `std::env::consts::EXE_SUFFIX` 让平台自己决定，别再猜。
const EXE_SUFFIX: &str = std::env::consts::EXE_SUFFIX;

fn find_built_exe(dir: &Path, name: &str) -> Option<PathBuf> {
    let rd = fs::read_dir(dir).ok()?;
    let want = format!("{name}{EXE_SUFFIX}");
    for e in rd.filter_map(|e| e.ok()) {
        let p = e.path();
        if p.is_dir() {
            if let Some(f) = find_built_exe(&p, name) { return Some(f); }
        } else if p.file_name().map(|n| n.to_string_lossy() == want).unwrap_or(false) {
            return Some(p);
        }
    }
    None
}

/// 构建 (release) 并安装到 `~/.lyco/bin`。
///
/// `name` 就是帮助里写的 `lyco install [名字]`。它**一直**在帮助里写着，但
/// 代码从来没读过 —— 原来签名是 `install()`，连形参都没有，于是
/// `lyco install myapp` 会静默地按 `Lyco.toml` 的项目名装成另一个命令。
/// 现在它真的生效，并且与 `uninstall [名字]` 对称：`uninstall myapp` 找的
/// 就是这里装出来的那个文件。
pub fn install(name: Option<&str>) -> Result<(), String> {
    build(true, None)?;
    let m = Manifest::load()?;
    let proj = m.project_name();
    // 报错里必须写出**真的去找了什么**。原来那句是字面量
    // `"构建产物未找到 (build/**/<name>.exe)"` —— `<name>` 会原样打给用户
    // （占位符泄漏），而且后缀在 Linux/macOS 上本来就是错的。
    let exe = find_built_exe(Path::new("build"), &proj)
        .ok_or_else(|| format!("构建产物未找到 (build/**/{proj}{EXE_SUFFIX})"))?;
    let dest_dir = super::data_dir().join("bin");
    fs::create_dir_all(&dest_dir).map_err(|e| e.to_string())?;
    let installed = name.unwrap_or(&proj);
    let dest = dest_dir.join(format!("{installed}{EXE_SUFFIX}"));
    fs::copy(&exe, &dest).map_err(|e| e.to_string())?;
    println!("✅ 已安装 {}", dest.display());
    if installed != proj {
        // 改名安装时把两件事都说清：装成了什么、怎么卸掉。
        // 不说的话，`lyco uninstall`（不带名字）会去找项目名，然后报「未安装」。
        println!("   (项目 {proj} → 命令 {installed}; 卸载: lyco uninstall {installed})");
    }
    println!("   提示: 把 {} 加入 PATH 后可全局调用", dest_dir.display());
    Ok(())
}

pub fn uninstall(name: Option<&str>) -> Result<(), String> {
    let n = match name {
        Some(n) => n.to_string(),
        None => Manifest::load()?.project_name(),
    };
    let p = super::data_dir().join("bin").join(format!("{n}{EXE_SUFFIX}"));
    if p.exists() {
        fs::remove_file(&p).map_err(|e| e.to_string())?;
        println!("✅ 已卸载 {}", p.display());
    } else {
        println!("ℹ 未安装: {}", p.display());
    }
    Ok(())
}
