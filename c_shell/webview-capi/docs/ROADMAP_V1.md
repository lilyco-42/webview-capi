# Lyco v1.0 架构蓝图 — 面向 2036

## 核心原则

1. **WASM 插件** — 沙箱隔离,跨版本,语言无关
2. **Tauri 2** — 轻量桌面,Rust 后端,Web 前端
3. **本地 AI** — 优先 Ollama/本地模型,可选云端
4. **SQLite + wasm** — 嵌入式存储,零运维
5. **原子化构建** — 复杂系统由基本单元组合

## 技术栈

| 层 | 技术 | 理由 |
|----|------|------|
| CLI | Rust | 单二进制,283KB,性能极好 |
| 桌面 GUI | Tauri 2 (Rust + WebView) | <5MB,原生性能,跨平台 |
| 插件系统 | WASM (wasmtime) | 沙箱,跨版本,安全隔离 |
| 模板引擎 | Tera (Rust) | 条件/循环/继承 |
| 数据存储 | SQLite (rusqlite) | 嵌入式,零配置,10年不变 |
| AI 推理 | wasm-local-ai 或子进程调用 Ollama | 本地优先,隐私保护 |
| 包管理 | xmake mirror | C/C++ 生态标准 |

## 架构图

```
┌─────────────────────────────────────────────────────────────┐
│                    lyco v1.0 (2026-2036)                     │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐     │
│  │              CLI (单二进制, ~500KB)                   │     │
│  │  lyco new / build / run / web / ai / plugin          │     │
│  └─────────────────────────┬───────────────────────────┘     │
│                            │                                 │
│  ┌─────────────────────────▼───────────────────────────┐     │
│  │              插件运行时 (wasmtime)                     │     │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐              │     │
│  │  │ new.wasm │ │build.wasm│ │ ai.wasm  │  ...        │     │
│  │  └──────────┘ └──────────┘ └──────────┘              │     │
│  │  第三方开发:C/Rust/Go/Zig/TypeScript→WASM             │     │
│  └─────────────────────────────────────────────────────┘     │
│                            │                                 │
│  ┌─────────────────────────▼───────────────────────────┐     │
│  │              持久化层 (SQLite + Tera)                 │     │
│  │  项目列表 / 配置 / 构建历史 / AI 对话 / 插件清单       │     │
│  └─────────────────────────────────────────────────────┘     │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐     │
│  │           Tauri 2 桌面 (可选,<5MB)                    │     │
│  │  Rust 后端 + WebView 前端 (Alpine.js + Tailwind)     │     │
│  └─────────────────────────────────────────────────────┘     │
└─────────────────────────────────────────────────────────────┘
```

## 版本路线

### v0.5 (当前) — 单二进制 + 动态释放
- ✅ Rust CLI,283KB
- ✅ include_str! 内置模板
- ✅ 首次释放到 ~/.lyco/
- ✅ 9 语言 + 5 平台
- ✅ xmake mirror

### v1.0 (6 个月后) — WASM 插件 + Tauri
- ⬜ 插件系统改为 WASM (wasmtime)
- ⬜ Tauri 2 桌面 GUI
- ⬜ SQLite 持久化
- ⬜ Tera 模板引擎(条件/循环/继承)

### v2.0 (1 年后) — AI 原生
- ⬜ 本地 AI 辅助 (Ollama 集成)
- ⬜ AI 自动生成项目
- ⬜ AI 调试构建问题
- ⬜ AI 插件开发助手

### v3.0 (3 年后) — 生态成熟
- ⬜ 插件市场 (类似 crates.io)
- ⬜ 多用户协作
- ⬜ CI/CD 集成
- ⬜ 企业级权限

## 关键技术验证

### 1. WASM 插件 (wasmtime)

```rust
// 运行时
use wasmtime::*;

struct PluginEngine {
    engine: Engine,
    store: Store<()>,
}

impl PluginEngine {
    fn load(&mut self, path: &Path) -> Result<Plugin> {
        let module = Module::from_file(&self.engine, path)?;
        let instance = Instance::new(&mut self.store, &module, &[])?;
        Ok(Plugin { instance })
    }
}

// 插件接口 (WIT 定义)
// package lyco:plugin@1.0.0
// interface command {
//   exec: func(args: list<string>) -> result<string, string>
//   info: func() -> command-info
// }
```

### 2. Tauri 2 桌面

```rust
// src-tauri/src/main.rs
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            cmd_new, cmd_build, cmd_run, cmd_ai_chat
        ])
        .run(tauri::generate_context!())
        .expect("error");
}

#[tauri::command]
fn cmd_new(name: String, lang: String) -> Result<String, String> {
    // 调用 lyco 核心逻辑
    lyco_core::cmd_new(&name, &lang)
}
```

### 3. SQLite 持久化

```rust
use rusqlite::Connection;

struct Database {
    conn: Connection,
}

impl Database {
    fn init(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS projects (
                name TEXT PRIMARY KEY,
                lang TEXT NOT NULL,
                path TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                last_open DATETIME
            );
            CREATE TABLE IF NOT EXISTS builds (
                id INTEGER PRIMARY KEY,
                project_name TEXT,
                target TEXT,
                status TEXT,
                log TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE IF NOT EXISTS plugins (
                name TEXT PRIMARY KEY,
                version TEXT,
                path TEXT,
                enabled BOOLEAN DEFAULT 1
            );
        ")?;
        Ok(Self { conn })
    }
}
```

### 4. 本地 AI 集成

```rust
// 优先本地 Ollama,可选 OpenAI/Anthropic
enum AIProvider {
    Ollama { model: String, url: String },
    OpenAI { model: String, key: String },
    Anthropic { model: String, key: String },
}

async fn ai_chat(prompt: &str, provider: &AIProvider) -> Result<String> {
    match provider {
        AIProvider::Ollama { model, url } => {
            // 调用 Ollama HTTP API
            let client = reqwest::Client::new();
            client.post(format!("{}/api/chat", url))
                .json(&serde_json::json!({ "model": model, "messages": [{"role":"user","content":prompt}] }))
                .send().await?
                .text().await
        }
        AIProvider::OpenAI { model, key } => { /* ... */ }
        AIProvider::Anthropic { model, key } => { /* ... */ }
    }
}
```

## 为什么这些技术能撑 10 年

| 技术 | 10年信心 | 理由 |
|------|---------|------|
| WASM | ⭐⭐⭐⭐⭐ | W3C 标准,所有浏览器支持,服务端运行时成熟 |
| Tauri | ⭐⭐⭐⭐⭐ | Electron 的替代,<5MB,Rust 保证内存安全 |
| SQLite | ⭐⭐⭐⭐⭐ | 50 年历史,地球上部署最多的数据库 |
| Rust | ⭐⭐⭐⭐⭐ | Linux 内核采用,10 年不变的基础设施 |
| Ollama/本地AI | ⭐⭐⭐⭐ | 隐私趋势,模型会变小变强 |
| xmake | ⭐⭐⭐⭐ | C/C++ 生态核心,长期维护 |

## 下一步

立即开始:
1. 添加 Tera 模板引擎(替换字符串替换)
2. 添加 rusqlite 持久化
3. 用 wasmtime 实现 WASM 插件加载
4. Tauri 2 GUI (可选,CLI 仍是核心)
