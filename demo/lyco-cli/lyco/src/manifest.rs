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
    //
    // target 名由 `test_sources()` 决定 —— 与 `test()` 里决定「跑哪个 target」
    // 用的是**同一个函数**。命名规则抄两遍的话，哪天改了一处，另一处就会静默地
    // 「找不到 target」或「跑错 target」。
    for (target, fname) in test_sources() {
        s.push_str(&format!(
            "\ntarget(\"{target}\")\n    set_kind(\"binary\")\n    set_default(false)\n    add_files(\"tests/{fname}\")\n    add_includedirs(\".\", \"src\")\n"
        ));
        s.push_str(&deps_block);
        s.push_str("target_end()\n");
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
    match fs::read_dir("tests") {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err("没有 tests/ 目录 (放 tests/xxx.c 后重试)".into());
        }
        Err(e) => return Err(format!("无法读取 tests/ 目录: {e}")),
    }

    // 要跑哪些 target，由 `test_sources()` 决定 —— 与 `gen_xmake_lua` 写进
    // `xmake.lua` 的那一份必然一致（撞名时的 `_<扩展名>` 后缀也是同一处算的）。
    let sources = test_sources();
    if sources.is_empty() { return Err("tests/ 下没有 .c/.cpp 测试文件".into()); }

    let mut ran = 0;
    let mut failed = 0;
    for (target, fname) in &sources {
        // 把 **target 名**也打出来 —— 撞名消歧之后，用户能看出
        // `a.c` 与 `a.cpp` 跑的是两个不同的 target，而不是同一个。
        println!("▶ 运行测试 {fname} ({target}) ...");
        // 这里用 `?` 而不是 `_ => 算失败`：xmake **起不来**（没装 / 没执行权限 /
        // PATH 不对）不是「测试失败」。原来这一支被 `_` 吞掉，真实错误
        // （`program not found`）一个字都不打印，用户看到的是
        // 「3/3 个测试失败」，然后去翻自己的测试代码。
        // 全仓其它调用点都是「无法执行 <命令>: <真实错误>」，这里跟上。
        let st = Command::new("xmake").args(["run", target.as_str()]).status()
            .map_err(|e| format!("无法执行 xmake run {target}: {e}"))?;
        // 只有**真的跑起来了**才计数。`ran` 原来在 `match` 之前无条件 `+= 1`，
        // 连「压根没执行」也算进去 —— `{failed}/{ran}` 里的分母于是是假的。
        ran += 1;
        if st.success() {
            println!("✔ {fname}");
        } else {
            println!("✘ {fname}");
            failed += 1;
        }
    }
    if failed > 0 { return Err(format!("{failed}/{ran} 个测试失败")); }
    println!("✅ {ran} 个测试全部通过");
    Ok(())
}

/// `tests/` 下的测试源文件 → `(target 名, 文件名)`，按文件名排序。
///
/// **命名规则只有这一份**：`gen_xmake_lua` 用它生成 `xmake.lua` 里的 `test_*`
/// target，`test()` 用它决定跑哪个 target。两处各写一遍的话，哪天改了一处，
/// 另一处就会静默地「找不到 target」或「跑错 target」。
///
/// 同名不同扩展（`tests/a.c` + `tests/a.cpp`）会撞成同一个 `test_a`。
/// 实测（修复前，`tests/` 里有 `a.c` / `a.cpp` / `b.c`）：
/// 生成的 `xmake.lua` 里出现**两个** `target("test_a")`，而 `lyco test`
/// 把两个文件都算成「跑过了」，最后打印
///
///     ▶ 运行测试 a ...        ← a.c
///     ▶ 运行测试 a ...        ← a.cpp，跑的却是**同一个** target
///     ✅ 3 个测试全部通过
///
/// —— 其中 `a.cpp` **一次都没被编译**。所以撞名时带上扩展名区分
/// （`test_a_c` / `test_a_cpp`）。
fn test_sources() -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = Vec::new();
    if let Ok(rd) = fs::read_dir("tests") {
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            // 目录名恰好像源文件（`tests/x.cpp/`）不算测试文件 —— `read_dir`
            // 会把它也列出来，`extension()` 同样是 `cpp`。
            if !p.is_file() { continue; }
            let ext = match p.extension().and_then(|x| x.to_str()) {
                Some(x) if x == "c" || x == "cpp" => x.to_string(),
                _ => continue,
            };
            let stem = match p.file_stem().and_then(|x| x.to_str()) {
                Some(s) if !s.is_empty() => s.to_string(),
                _ => continue,
            };
            files.push((stem, ext));
        }
    }
    // `read_dir` 的顺序是**任意**的 —— 不排序的话生成结果不确定（同一份
    // `tests/` 两次构建可能得到不同的 target 名），而撞名消歧又依赖「谁先谁后」。
    files.sort();
    let mut out = Vec::new();
    for (stem, ext) in &files {
        let dup = files.iter().filter(|(s, _)| s == stem).count() > 1;
        let target = if dup {
            format!("test_{stem}_{ext}")
        } else {
            format!("test_{stem}")
        };
        out.push((target, format!("{stem}.{ext}")));
    }
    out
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

// ── 已安装命令的清单（`~/.lyco/installed.toml`） ─────────────
//
// `~/.lyco/bin` 原来是**只写不读**的：`install` 往里拷文件、`uninstall` 按
// **文件名**去猜，中间没有任何记录。于是三件事都做不到：
//
//   1. `lyco info` / `lyco list` 完全不提这个目录 —— 用户装了东西之后
//      **没有任何手段枚举**自己装过什么（`info` 甚至打了「模板 / 外部命令 /
//      已注册项目」三个计数，偏偏漏了这个）；
//   2. `lyco install myapp` 覆盖掉**另一个项目**装的 `myapp` 时一声不响；
//   3. `lyco uninstall`（不带名字）按**项目名**找文件 —— 而
//      `lyco install myapp` 装出来的文件叫 `myapp`，于是它报「未安装」，
//      用户以为卸干净了，其实那个命令还在 `bin/` 里。
//
// 实测（修复前，`~/.lyco/bin/myapp.exe` 确实是项目 p1 装的）：
//
//     $ lyco uninstall
//     ℹ 未安装: C:\...\.lyco\bin\p1.exe
//     $ echo $?
//     0
//
// 清单放在 `~/.lyco/installed.toml`（**不放进 `bin/`**）：`bin/` 里应该只有
// 「能被 PATH 直接调用的东西」，`ls ~/.lyco/bin` 看到的就是全部命令。
//
// 为什么是 TOML 而不是 JSON：`serde_json` 不是这个 crate 的依赖，而 `toml`
// 是（`Lyco.toml` 本来就要解析）。为一个辅助文件新引一个依赖不值。

/// 清单里的一条：**哪个项目**把**哪个名字**装进了 `bin/`。
///
/// `#[serde(default)]`（容器级）让**缺字段也能解析** —— 用户手改这份文件、
/// 或者哪天加了新字段，都不该让 `read_installed()` 直接退化成「空清单」
/// （那会让 `lyco info` 静默地不再列出任何已安装命令）。
#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
#[serde(default)]
struct Installed {
    /// `bin/` 里的命令名（不含 `EXE_SUFFIX`）
    name: String,
    /// 装它的项目名（`Lyco.toml` 的 `package.name`）
    project: String,
    /// 构建产物路径（覆盖别人时能告诉用户「这东西是从哪来的」）
    source: String,
    /// 安装时间（Unix 秒）。没引 chrono，只用一个标准算法换算成日期。
    when: u64,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct InstalledFile {
    #[serde(default)]
    installed: Vec<Installed>,
}

fn installed_path() -> PathBuf { super::data_dir().join("installed.toml") }

/// 读清单。**文件不在 / 解析不了都当空**（一个辅助文件坏了，不该让
/// `lyco info` 直接报错），并且**丢掉文件已经不存在的条目** ——
/// 用户手动 `rm ~/.lyco/bin/myapp.exe` 之后，清单里不该留一个幽灵条目
/// （「判据要能自愈」那条：判「有没有可用条目」，别判「文件在不在」）。
fn read_installed() -> Vec<Installed> {
    let t = match fs::read_to_string(installed_path()) { Ok(t) => t, Err(_) => return Vec::new() };
    let f: InstalledFile = match toml::from_str(&t) { Ok(f) => f, Err(_) => return Vec::new() };
    let bin = super::data_dir().join("bin");
    f.installed
        .into_iter()
        .filter(|i| bin.join(format!("{}{EXE_SUFFIX}", i.name)).exists())
        .collect()
}

/// 写回清单。**先写临时文件再改名** —— 中途失败不会留下半份清单。
///
/// 返回 `Err` 时调用方**不要**把整个操作报成失败（文件已经装好了），
/// 而是把「清单没更新」这件事单独说出来。
fn write_installed(list: &[Installed]) -> Result<(), String> {
    let f = InstalledFile { installed: list.to_vec() };
    let t = toml::to_string(&f).map_err(|e| format!("无法序列化: {e}"))?;
    let p = installed_path();
    let tmp = p.with_extension("toml.tmp");
    fs::write(&tmp, t).map_err(|e| format!("无法写 {}: {e}", tmp.display()))?;
    fs::rename(&tmp, &p).map_err(|e| format!("无法更新 {}: {e}", p.display()))?;
    Ok(())
}

/// 清单更新失败时的统一说法：**说清后果**，而不是静默吞掉。
fn warn_manifest(e: &str) {
    println!("⚠ 已安装命令的清单没能更新: {e}");
    println!("   (命令本身没问题; 但 `lyco info` / `lyco list` 不会列出它)");
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Unix 秒 → `(年, 月, 日)`，**UTC**。
///
/// 没引 chrono —— 只需要一个日期，用标准的 civil_from_days 就够。
/// 用 UTC 而不是本地时区：CI 的 runner 时区不固定，本地时区会让断言飘。
fn ymd(secs: u64) -> (i64, u32, u32) {
    let days = (secs / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d as u32)
}

/// 供 `lyco info` / `lyco list` 用的 `YYYY-MM-DD`。
pub fn day(secs: u64) -> String {
    let (y, m, d) = ymd(secs);
    format!("{y:04}-{m:02}-{d:02}")
}

/// 供 `lyco info` / `lyco list` 用：`(命令名, 项目名, 安装时间)`，按名字排序。
pub fn installed() -> Vec<(String, String, u64)> {
    let mut v: Vec<(String, String, u64)> = read_installed()
        .into_iter()
        .map(|i| (i.name, i.project, i.when))
        .collect();
    v.sort();
    v
}

/// `bin/` 里**真实存在**的命令名（去掉 `EXE_SUFFIX`），排序。
///
/// 与 `read_installed()` 的区别：这个不看清单、只看文件系统。两个一起用，
/// 就能发现「清单里没有、但文件在」的情况（老版本 lyco 装的、或用户自己放的）。
fn bin_entries(bin: &Path) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    if let Ok(rd) = fs::read_dir(bin) {
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if !p.is_file() { continue; }
            let f = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            match f.strip_suffix(EXE_SUFFIX) {
                Some(s) if !s.is_empty() => v.push(s.to_string()),
                _ => continue,
            }
        }
    }
    v.sort();
    v
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
    let cmd = name.unwrap_or(&proj);
    let dest = dest_dir.join(format!("{cmd}{EXE_SUFFIX}"));

    // ── 覆盖之前先说清楚覆盖了什么 ──
    // 原来是直接 `fs::copy`：如果 `bin/myapp` 是**另一个项目**装的，
    // 用户一个字都不会看到，而他的命令已经被换掉了。
    let mut list = read_installed();
    match list.iter().find(|i| i.name == cmd) {
        Some(prev) if prev.project == proj => {
            println!("↻ 重新安装 {cmd}（覆盖上一次自己装的）");
        }
        Some(prev) => {
            println!("⚠ 覆盖 {cmd} —— 它原先是项目 {} 装的", prev.project);
            println!("   (原构建产物: {})", prev.source);
        }
        None if dest.exists() => {
            // 文件在、清单里没有：老版本 lyco 装的，或者用户自己放进去的。
            println!("⚠ 覆盖 {cmd}{EXE_SUFFIX} —— 它已经存在，但不是 lyco 装的");
            println!("   ({})", dest.display());
        }
        None => {}
    }

    fs::copy(&exe, &dest).map_err(|e| format!("无法安装到 {}: {e}", dest.display()))?;
    println!("✅ 已安装 {}", dest.display());
    if cmd != proj {
        // 改名安装时把两件事都说清：装成了什么、怎么卸掉。
        println!("   (项目 {proj} → 命令 {cmd}; 卸载: lyco uninstall {cmd})");
    }
    println!("   提示: 把 {} 加入 PATH 后可全局调用", dest_dir.display());

    // ── 记进清单 ──
    // `lyco info` / `lyco list` / `uninstall`（不带名字）都靠它。
    // 文件已经装好了，所以清单写失败**不算安装失败** —— 但必须说出来。
    let src = fs::canonicalize(&exe).map(|p| super::tidy_path(&p)).unwrap_or_else(|_| super::tidy_path(&exe));
    list.retain(|i| i.name != cmd);
    list.push(Installed {
        name: cmd.to_string(),
        project: proj.clone(),
        source: src,
        when: now_secs(),
    });
    if let Err(e) = write_installed(&list) { warn_manifest(&e); }
    Ok(())
}

/// `lyco uninstall [名字]`。
///
/// * `Some(name)` —— 卸那一个。
/// * `None` —— 卸**当前项目装的全部命令**，按清单查。
///
/// 原来 `None` 是按**文件名** `<项目名>` 找 —— 而 `lyco install myapp`
/// 装出来的文件叫 `myapp`，于是它报「未安装」并**退出 0**。
/// 实测（修复前）：`bin/myapp.exe` 确实是当前项目装的，而
/// `lyco uninstall` 输出 `ℹ 未安装: …\p1.exe`、`$? = 0`。
///
/// 现在 `None` 且清单里没有当前项目的条目时：
///   * `bin/` 确实空 → 幂等成功（并把「为什么没卸到」说清）；
///   * `bin/` 里有别的东西 → **报错退出**，并把它们列出来。
///     不能只说「未安装」：那会让用户以为已经卸干净了。
pub fn uninstall(name: Option<&str>) -> Result<(), String> {
    let bin = super::data_dir().join("bin");
    let mut list = read_installed();

    let names: Vec<String> = match name {
        Some(n) => vec![n.to_string()],
        None => {
            let proj = Manifest::load()?.project_name();
            let mine: Vec<String> = list
                .iter()
                .filter(|i| i.project == proj)
                .map(|i| i.name.clone())
                .collect();
            if mine.is_empty() {
                let others = bin_entries(&bin);
                if others.is_empty() {
                    println!("ℹ 项目 {proj} 没有已安装的命令（{} 是空的）", bin.display());
                    return Ok(());
                }
                return Err(format!(
                    "项目 {proj} 没有已安装的命令, 但 {} 里有 {} 个: {}\n   \
                     (要卸哪一个: lyco uninstall <名字>)",
                    bin.display(),
                    others.len(),
                    others.join(", ")
                ));
            }
            mine
        }
    };

    let mut done = 0;
    for n in &names {
        let p = bin.join(format!("{n}{EXE_SUFFIX}"));
        if p.exists() {
            fs::remove_file(&p).map_err(|e| format!("无法删除 {}: {e}", p.display()))?;
            println!("✅ 已卸载 {}", p.display());
            done += 1;
        } else {
            println!("ℹ 未安装: {}", p.display());
        }
        // 清单条目一并清掉（`read_installed` 已经把「文件不在」的条目滤掉了，
        // 但这里还是要清 —— 否则刚删掉的那条会留在文件里）。
        list.retain(|i| i.name != *n);
    }
    if let Err(e) = write_installed(&list) { warn_manifest(&e); }
    if names.len() > 1 {
        println!("   (共 {done} 个)");
    }
    Ok(())
}
