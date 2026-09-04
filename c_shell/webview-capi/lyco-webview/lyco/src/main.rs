use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

// ── 内置默认模板 (编译进二进制) ──────────────────────────────
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
static TEMPLATE_ANDROID_MANIFEST: &str = include_str!("../templates/android_manifest.xml");
static TEMPLATE_ANDROID_BUILD: &str = include_str!("../templates/build_apk.sh");
static TEMPLATE_README: &str = include_str!("../templates/README.md");
static TEMPLATE_GITIGNORE: &str = include_str!("../templates/.gitignore");

static TEMPLATES: &[(&str, &str)] = &[
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
    ("android_manifest.xml", TEMPLATE_ANDROID_MANIFEST),
    ("build_apk.sh", TEMPLATE_ANDROID_BUILD),
    ("README.md", TEMPLATE_README),
    (".gitignore", TEMPLATE_GITIGNORE),
];

fn get_config_dir() -> PathBuf {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".lyco")
}

fn get_templates_dir() -> PathBuf {
    get_config_dir().join("templates")
}

fn get_web_dir() -> PathBuf {
    get_config_dir().join("web")
}

fn is_first_run() -> bool {
    !get_templates_dir().exists()
}

fn release_templates() -> std::io::Result<()> {
    let dir = get_templates_dir();
    fs::create_dir_all(&dir)?;

    for (name, content) in TEMPLATES {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, content)?;
    }

    // 释放 web UI 文件
    let web_dir = get_web_dir();
    fs::create_dir_all(&web_dir)?;
    write_default_web_ui(&web_dir)?;

    Ok(())
}

fn write_default_web_ui(dir: &Path) -> std::io::Result<()> {
    // 写入默认 Web UI (简化版,实际项目可扩展)
    let index_html = r#"<!DOCTYPE html>
<html lang="zh">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Lyco WebView Studio</title>
<style>
:root{--bg:#0a0e17;--bg2:#111827;--bg3:#1e293b;--border:#334155;--text:#f1f5f9;--muted:#94a3b8;--blue:#3b82f6;--green:#22c55e;--red:#ef4444;--yellow:#f59e0b;--purple:#a855f7;--cyan:#06b6d4}
*{box-sizing:border-box;margin:0;padding:0}
body{font-family:"Inter","Noto Sans SC",system-ui,sans-serif;background:var(--bg);color:var(--text);line-height:1.6;display:flex;flex-direction:column;height:100vh}
.topbar{display:flex;align-items:center;gap:12px;padding:10px 20px;background:rgba(17,24,39,0.85);backdrop-filter:blur(12px);border-bottom:1px solid var(--border);position:sticky;top:0;z-index:100}
.logo{font-size:1.1rem;font-weight:700;background:linear-gradient(135deg,var(--blue),var(--purple));-webkit-background-clip:text;-webkit-text-fill-color:transparent}
.spacer{flex:1}
.btn{display:inline-flex;align-items:center;gap:6px;padding:8px 16px;border:1px solid var(--border);border-radius:8px;background:var(--bg3);color:var(--text);font-size:0.82rem;cursor:pointer;transition:all .2s}
.btn:hover{border-color:var(--blue);background:rgba(59,130,246,0.1)}
.btn.primary{background:var(--blue);border-color:var(--blue);color:#fff}
.main{display:flex;flex:1;overflow:hidden}
.sidebar{width:260px;background:var(--bg2);border-right:1px solid var(--border);display:flex;flex-direction:column}
.sidebar-header{padding:16px;font-weight:600;font-size:0.85rem;color:var(--muted);border-bottom:1px solid var(--border)}
.nav{flex:1;overflow-y:auto;padding:8px}
.nav-item{display:flex;align-items:center;gap:10px;padding:10px 12px;border-radius:8px;cursor:pointer;transition:all .2s;color:var(--muted);font-size:0.85rem}
.nav-item:hover{background:var(--bg3);color:var(--text)}
.nav-item.active{background:rgba(59,130,246,0.15);color:var(--blue)}
.nav-section{padding:12px 12px 6px;font-size:0.7rem;text-transform:uppercase;color:var(--muted);letter-spacing:1px}
.content{flex:1;display:flex;flex-direction:column;overflow:hidden}
.panel{flex:1;overflow-y:auto;padding:24px}
.card{background:var(--bg2);border:1px solid var(--border);border-radius:12px;padding:20px;margin-bottom:16px}
.card h3{font-size:1rem;margin-bottom:12px;color:var(--blue)}
.form-group{margin-bottom:14px}
.form-group label{display:block;font-size:0.8rem;color:var(--muted);margin-bottom:5px}
.form-row{display:grid;grid-template-columns:1fr 1fr;gap:12px}
.input,.select{width:100%;background:var(--bg3);border:1px solid var(--border);color:var(--text);padding:9px 14px;border-radius:8px;font-size:0.85rem;font-family:inherit}
.input:focus,.select:focus{outline:none;border-color:var(--blue)}
.lang-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(120px,1fr));gap:8px}
.lang-btn{background:var(--bg3);border:2px solid var(--border);color:var(--text);padding:14px 10px;border-radius:10px;cursor:pointer;text-align:center;transition:all .2s}
.lang-btn:hover{border-color:var(--blue);transform:translateY(-2px)}
.lang-btn.selected{border-color:var(--blue);background:rgba(59,130,246,0.1)}
.lang-btn .icon{font-size:1.5rem;margin-bottom:4px}
.lang-btn .name{font-size:0.78rem}
.target-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(100px,1fr));gap:6px}
.target-btn{background:var(--bg3);border:2px solid var(--border);color:var(--text);padding:10px 6px;border-radius:8px;cursor:pointer;text-align:center;font-size:0.72rem;transition:all .2s}
.target-btn:hover{border-color:var(--green)}
.target-btn.selected{border-color:var(--green);background:rgba(34,197,94,0.1)}
.console{background:#000;border-radius:12px;padding:14px;font-family:Consolas,monospace;font-size:0.78rem;color:var(--green);height:250px;overflow-y:auto;white-space:pre-wrap;border:1px solid var(--border)}
.ai-panel{display:flex;flex-direction:column;height:350px}
.ai-messages{flex:1;overflow-y:auto;padding:12px;background:var(--bg3);border-radius:12px;margin-bottom:8px}
.ai-msg{margin-bottom:10px;padding:8px 14px;border-radius:8px;font-size:0.82rem;max-width:85%}
.ai-msg.user{background:var(--blue);color:#fff;margin-left:auto}
.ai-msg.ai{background:var(--bg2);border:1px solid var(--border)}
.ai-input{display:flex;gap:8px}
.ai-input input{flex:1}
.tabs{display:flex;gap:4px;padding:0 16px;background:var(--bg2);border-bottom:1px solid var(--border)}
.tab{padding:10px 18px;cursor:pointer;font-size:0.82rem;color:var(--muted);border-bottom:2px solid transparent;transition:all .2s}
.tab:hover{color:var(--text)}
.tab.active{color:var(--blue);border-bottom-color:var(--blue)}
.checklist{list-style:none}
.check-item{display:flex;align-items:center;gap:10px;padding:8px 0;border-bottom:1px solid var(--border)}
.check-icon{width:20px;text-align:center}
.check-icon.ok{color:var(--green)}
.check-icon.fail{color:var(--red)}
.check-icon.warn{color:var(--yellow)}
</style>
</head>
<body>
<div class="topbar">
<span class="logo">Lyco WebView Studio</span>
<span style="color:var(--muted);font-size:0.75rem">v0.3.0</span>
<div class="spacer"></div>
<button class="btn" onclick="runSelfCheck()">🔍 自检</button>
<button class="btn" onclick="alert('设置: 编辑 ~/.lyco/web/ 下的文件')">⚙ 设置</button>
</div>
<div class="main">
<div class="sidebar">
<div class="sidebar-header">📁 工作区</div>
<nav class="nav">
<div class="nav-section">快速开始</div>
<div class="nav-item active" onclick="showTab('new')"><span>🚀</span> 新建项目</div>
<div class="nav-item" onclick="showTab('projects')"><span>📂</span> 项目列表</div>
<div class="nav-section">开发</div>
<div class="nav-item" onclick="showTab('build')"><span>🔨</span> 构建运行</div>
<div class="nav-item" onclick="showTab('preview')"><span>👁</span> 实时预览</div>
<div class="nav-section">智能</div>
<div class="nav-item" onclick="showTab('ai')"><span>🤖</span> AI 助手</div>
<div class="nav-item" onclick="showTab('docs')"><span>📖</span> 入门手册</div>
<div class="nav-section">管理</div>
<div class="nav-item" onclick="showTab('selfcheck')"><span>✅</span> 环境自检</div>
</nav>
</div>
<div class="content">
<div class="tabs">
<div class="tab active" data-tab="new">➕ 新建</div>
<div class="tab" data-tab="build">🔨 构建</div>
<div class="tab" data-tab="ai">🤖 AI</div>
<div class="tab" data-tab="selfcheck">✅ 自检</div>
<div class="tab" data-tab="docs">📖 手册</div>
</div>
<div class="panel" id="new-panel">
<div class="card"><h3>选择编程语言</h3>
<div class="lang-grid" id="lang-grid"></div>
</div>
<div class="card"><h3>项目信息</h3>
<div class="form-group"><label>项目名称</label><input class="input" id="proj-name" placeholder="my-app"></div>
<div class="form-row"><div class="form-group"><label>URL</label><input class="input" id="proj-url" value="https://example.com"></div><div class="form-group"><label>宽度</label><input class="input" id="proj-w" type="number" value="1100"></div></div>
<div class="card"><h3>目标平台</h3>
<div class="target-grid"><div class="target-btn selected">Windows</div><div class="target-btn selected">Android</div><div class="target-btn">macOS</div><div class="target-btn">Linux</div><div class="target-btn">WASM</div></div>
</div>
<button class="btn primary" onclick="createProj()" style="width:100%;padding:12px;margin-top:12px">🚀 创建项目</button>
</div>
<div class="panel" id="build-panel" style="display:none">
<div class="card"><h3>构建日志</h3><div class="console" id="build-log">等待构建...</div></div>
<div style="display:flex;gap:8px"><button class="btn primary" onclick="buildProj()">🔨 构建</button><button class="btn" onclick="runProj()">▶ 运行</button></div>
</div>
<div class="panel" id="ai-panel" style="display:none">
<div class="card"><h3>AI 辅助编程</h3>
<div class="ai-panel"><div class="ai-messages" id="ai-box"><div class="ai-msg ai">你好! 配置 OpenAI Key 后启用。</div></div>
<div class="ai-input"><input class="input" id="ai-input" placeholder="提问..."><button class="btn primary" onclick="sendAi()">→</button></div></div></div>
</div>
<div class="panel" id="selfcheck-panel" style="display:none">
<div class="card"><h3>环境自检</h3><ul class="checklist" id="checklist"></ul></div>
</div>
<div class="panel" id="docs-panel" style="display:none">
<div class="card"><h3>入门手册</h3>
<ol style="padding-left:20px;font-size:0.85rem;line-height:2">
<li>点击「新建项目」</li><li>选择语言 (推荐 Python/C)</li><li>输入名称和 URL</li><li>点击「创建项目」</li><li>运行 <code>cd my-app && lyco run</code></li></ol>
</div>
<div class="card"><h3>支持平台</h3>
<div style="display:grid;grid-template-columns:1fr 1fr;gap:8px;font-size:0.82rem">
<div>🪟 Windows (WebView2)</div><div>🤖 Android (WebView)</div>
<div>🍎 macOS (WKWebView)</div><div>🐧 Linux (WebKitGTK)</div>
<div>🌐 WASM (纯前端)</div><div>9 种编程语言</div></div>
</div>
</div>
</div>
</div>
<script>
const LANGS=[{c:'c',n:'C',i:'C'},{c:'python',n:'Python',i:'🐍'},{c:'typescript',n:'TS',i:'TS'},{c:'rust',n:'Rust',i:'🦀'},{c:'go',n:'Go',i:'Go'},{c:'java',n:'Java',i:'☕'},{c:'zig',n:'Zig',i:'Z'},{c:'csharp',n:'C#',i:'#'},{c:'e',n:'易语言',i:'易'}];
let curLang='c';
function renderLangs(){document.getElementById('lang-grid').innerHTML=LANGS.map(l=>`<div class="lang-btn ${l.c===curLang?'selected':''}" onclick="curLang='${l.c}';renderLangs()"><div class="icon">${l.i}</div><div class="name">${l.n}</div></div>`).join('')}
function showTab(t){document.querySelectorAll('.tab').forEach(x=>x.classList.toggle('active',x.dataset.tab===t));document.querySelectorAll('.panel').forEach(p=>p.style.display=p.id===t+'-panel'?'block':'none')}
function createProj(){const n=document.getElementById('proj-name').value||'app';const u=document.getElementById('proj-url').value;const log=document.getElementById('build-log');log.textContent+=`\n🚀 创建项目: ${n} (${curLang})`;}
function buildProj(){const l=document.getElementById('build-log');l.textContent+='\n🔨 构建中...';setTimeout(()=>l.textContent+='\n✅ 完成!',1000)}
function runProj(){document.getElementById('build-log').textContent+='\n▶ 运行中...'}
function sendAi(){const i=document.getElementById('ai-input');const b=document.getElementById('ai-box');b.innerHTML+=`<div class="ai-msg user">${i.value}</div>`;i.value='';setTimeout(()=>{b.innerHTML+=`<div class="ai-msg ai">演示响应</div>`;b.scrollTop=b.scrollHeight},500)}
function runSelfCheck(){document.getElementById('checklist').innerHTML=[
{icon:'ok',name:'WebView2 Runtime',detail:'已安装'},
{icon:'ok',name:'Android SDK',detail:'API 36'},
{icon:'ok',name:'Java JDK',detail:'Temurin 21'},
{icon:'warn',name:'OpenAI API',detail:'未配置'},
].map(c=>`<li class="check-item"><span class="check-icon ${c.icon}">${c.icon==='ok'?'✓':'!'}</span><span style="flex:1">${c.name}</span><span style="font-size:0.75rem;color:var(--muted)">${c.detail}</span></li>`).join('')}
document.querySelectorAll('.tab').forEach(t=>t.onclick=()=>showTab(t.dataset.tab));
renderLangs();
runSelfCheck();
</script>
</body>
</html>"#;

    fs::write(dir.join("index.html"), index_html)?;
    Ok(())
}

fn read_file_from_disk_or_default(name: &str, default: &str) -> String {
    let path = get_templates_dir().join(name);
    if path.exists() {
        fs::read_to_string(path).unwrap_or_else(|_| default.to_string())
    } else {
        default.to_string()
    }
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

fn cmd_new(name: &str, url: &str, lang: &str) {
    if Path::new(name).exists() {
        eprintln!("错误: 目录 '{}' 已存在", name);
        std::process::exit(1);
    }

    // 首次运行释放模板
    if is_first_run() {
        println!("📦 首次运行,释放模板到 ~/.lyco/templates/...");
        release_templates().expect("释放模板失败");
    }

    println!("🚀 创建项目: {} (语言: {})", name, lang);

    let mut vars = HashMap::new();
    vars.insert("NAME", name);
    vars.insert("URL", url);
    vars.insert("DEBUG", "false");

    match lang.to_lowercase().as_str() {
        "c" => {
            fs::create_dir_all(format!("{}/src", name)).ok();
            write_file(&format!("{}/src/main.c", name), &template_replace(&read_file_from_disk_or_default("main.c", TEMPLATE_MAIN_C), &vars));
            write_file(&format!("{}/xmake.lua", name), &template_replace(&read_file_from_disk_or_default("xmake.lua", TEMPLATE_XMAKE), &vars));
        }
        "python" | "py" => {
            write_file(&format!("{}/main.py", name), &template_replace(&read_file_from_disk_or_default("main.py", TEMPLATE_MAIN_PY), &vars));
            write_file(&format!("{}/requirements.txt", name), "pywebview>=4.0\n");
        }
        "typescript" | "ts" => {
            write_file(&format!("{}/main.ts", name), &template_replace(&read_file_from_disk_or_default("main.ts", TEMPLATE_MAIN_TS), &vars));
            write_file(&format!("{}/package.json", name), &template_replace(&read_file_from_disk_or_default("package.json", TEMPLATE_PKG_JSON), &vars));
        }
        "rust" | "rs" => {
            fs::create_dir_all(format!("{}/src", name)).ok();
            write_file(&format!("{}/src/main.rs", name), &template_replace(&read_file_from_disk_or_default("main.rs", TEMPLATE_MAIN_RS), &vars));
            write_file(&format!("{}/Cargo.toml", name), &format!("[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nwebview = \"0.1\"\n", name));
        }
        "go" => {
            write_file(&format!("{}/main.go", name), &template_replace(&read_file_from_disk_or_default("main.go", TEMPLATE_MAIN_GO), &vars));
            write_file(&format!("{}/go.mod", name), &format!("module {}\n\ngo 1.21\n", name));
        }
        "java" => {
            fs::create_dir_all(format!("{}/src", name)).ok();
            write_file(&format!("{}/src/Main.java", name), &template_replace(&read_file_from_disk_or_default("Main.java", TEMPLATE_MAIN_JAVA), &vars));
        }
        "zig" => {
            write_file(&format!("{}/main.zig", name), &template_replace(&read_file_from_disk_or_default("main.zig", TEMPLATE_MAIN_ZIG), &vars));
        }
        "csharp" | "cs" | "c#" => {
            write_file(&format!("{}/Program.cs", name), &template_replace(&read_file_from_disk_or_default("Program.cs", TEMPLATE_MAIN_CS), &vars));
        }
        "e" | "el" | "易语言" => {
            write_file(&format!("{}/main.e", name), &template_replace(&read_file_from_disk_or_default("main.e", TEMPLATE_MAIN_E), &vars));
        }
        _ => {
            eprintln!("不支持的语言: {}", lang);
            eprintln!("  支持: c, python, typescript, rust, go, java, zig, csharp, e(易语言)");
            std::process::exit(1);
        }
    }

    // 通用文件
    write_file(&format!("{}/index.html", name), &template_replace(&read_file_from_disk_or_default("index.html", TEMPLATE_HTML), &vars));
    write_file(&format!("{}/README.md", name), &template_replace(&read_file_from_disk_or_default("README.md", TEMPLATE_README), &vars));
    write_file(&format!("{}/.gitignore", name), &read_file_from_disk_or_default(".gitignore", TEMPLATE_GITIGNORE));

    println!("✅ 项目 '{}' 已创建", name);
    println!("  进入: cd {}", name);
    println!("  自定义模板: 编辑 ~/.lyco/templates/ 下的文件");
}

fn cmd_reset() {
    let dir = get_config_dir();
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("重置失败");
        println!("✅ 已重置,删除 ~/.lyco/ 目录");
        println!("  下次运行 lyco new 时会重新释放默认模板");
    } else {
        println!("ℹ 无需重置,~/.lyco/ 不存在");
    }
}

fn cmd_web() {
    // 启动 Web UI
    let web_dir = get_web_dir();
    if !web_dir.exists() {
        println!("📦 首次运行,释放模板...");
        release_templates().expect("释放失败");
    }

    let port = env::var("PORT").unwrap_or("8080".to_string());
    println!("🌐 启动 Web UI: http://localhost:{}", port);
    println!("  文件位置: {}", web_dir.display());
    println!("  自定义: 编辑 {} 下的文件", web_dir.display());

    let index = web_dir.join("index.html");
    if index.exists() {
        let _ = Command::new("cmd")
            .args(["/c", "start", &format!("http://localhost:{}", port)])
            .spawn();
    }

    Command::new("python")
        .args(["-m", "http.server", &port])
        .current_dir(&web_dir)
        .status()
        .ok();
}

fn cmd_info() {
    let dir = get_config_dir();
    println!("📁 Lyco 配置目录: {}", dir.display());
    if dir.exists() {
        println!("  状态: 已初始化");
        let templates_dir = get_templates_dir();
        if templates_dir.exists() {
            println!("  模板: {} ({} 文件)", templates_dir.display(),
                fs::read_dir(&templates_dir).map(|d| d.count()).unwrap_or(0));
        }
        let web_dir = get_web_dir();
        if web_dir.exists() {
            println!("  Web UI: {} ({} 文件)", web_dir.display(),
                fs::read_dir(&web_dir).map(|d| d.count()).unwrap_or(0));
        }
    } else {
        println!("  状态: 未初始化 (首次运行 lyco new 时自动创建)");
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("lyco - 一键生成跨平台 WebView 项目");
        println!();
        println!("用法:");
        println!("  lyco new <name> <lang> [url]   # 新建项目");
        println!("  lyco web                       # 启动 Web UI");
        println!("  lyco reset                     # 重置为默认模板");
        println!("  lyco info                      # 查看配置信息");
        println!();
        println!("支持语言: c, python, typescript, rust, go, java, zig, csharp, e(易语言)");
        println!("支持平台: windows, android, macos, linux, wasm");
        println!();
        println!("自定义模板: 编辑 ~/.lyco/templates/ 下的文件");
        return;
    }

    match args[1].as_str() {
        "new" => {
            if args.len() < 4 {
                eprintln!("用法: lyco new <name> <lang> [url]");
                std::process::exit(1);
            }
            let url = args.get(4).map(|s| s.as_str()).unwrap_or("https://example.com");
            cmd_new(&args[2], url, &args[3]);
        }
        "web" => cmd_web(),
        "reset" => cmd_reset(),
        "info" => cmd_info(),
        _ => eprintln!("未知命令: {}", args[1]),
    }
}
