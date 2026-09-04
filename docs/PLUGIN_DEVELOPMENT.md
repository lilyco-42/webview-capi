# Lyco 插件开发指南

> 面向第三方组件开发者的完整 API 参考

## 目录

1. [架构概览](#架构概览)
2. [快速开始](#快速开始)
3. [插件生命周期](#插件生命周期)
4. [命令 API](#命令-api)
5. [模板系统](#模板系统)
6. [配置系统](#配置系统)
7. [Web UI 扩展](#web-ui-扩展)
8. [完整示例](#完整示例)
9. [发布与分发](#发布与分发)

---

## 架构概览

```
┌─────────────────────────────────────────────────┐
│                  lyco (单二进制)                  │
│  ┌─────────┐  ┌──────────┐  ┌───────────────┐  │
│  │ 内置命令 │  │ 模板引擎  │  │ 插件加载器     │  │
│  │ new     │  │ 变量替换  │  │ SO/DLL 加载   │  │
│  │ build   │  │ 文件释放  │  │ 命令路由       │  │
│  │ run     │  │ 用户覆盖  │  │ 优先级判定     │  │
│  └─────────┘  └──────────┘  └───────┬───────┘  │
│                                      │          │
└──────────────────────────────────────┼──────────┘
                                       │
                    ┌──────────────────┼──────────────────┐
                    │    ~/.lyco/commands/                 │
                    │  ┌────────────┐  ┌────────────┐     │
                    │  │ new.dll    │  │ build.dll  │     │
                    │  │ new.exe    │  │ build.exe  │     │
                    │  └────────────┘  └────────────┘     │
                    │  第三方插件可完全替代内置命令          │
                    └─────────────────────────────────────┘
```

### 核心原则

1. **单二进制分发** — lyco.exe 包含所有默认功能,无需外部依赖
2. **首次释放** — 首次运行时释放默认模板到 `~/.lyco/`
3. **用户覆盖** — 用户编辑释放的文件后,优先使用用户版本
4. **插件优先** — 外部命令(SO/DLL)优先于内置命令执行
5. **向后兼容** — 插件 API 保证向前兼容

---

## 快速开始

### 5 分钟创建你的第一个插件

#### 1. 创建插件目录

```bash
mkdir my-lyco-plugin
cd my-lyco-plugin
```

#### 2. 编写插件代码 (以 new 命令为例)

**C 语言插件示例** (`new.c`):

```c
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Lyco 插件 API 头文件
#include "lyco_plugin.h"

// 命令信息
static const char* PLUGIN_NAME = "new";
static const char* PLUGIN_VERSION = "1.0.0";
static const char* PLUGIN_DESC = "自定义 new 命令 - 带 AI 辅助";

// 初始化入口 (必须导出)
PLUGIN_EXPORT int lyco_init(CommandInfo* info) {
    info->name = PLUGIN_NAME;
    info->version = PLUGIN_VERSION;
    info->description = PLUGIN_DESC;
    return 0; // 0 = 成功
}

// 命令执行入口 (必须导出)
PLUGIN_EXPORT int lyco_execute(int argc, char** argv) {
    if <｜DSML｜parameter> < 2) {
        printf("用法: lyco new <name> <lang> [url]\n");
        return 1;
    }

    const char* name = argv[0];
    const char* lang = argv[1];
    const char* url = argc > 2 ? argv[2] : "https://example.com";

    printf("🚀 创建项目: %s (%s)\n", name, lang);
    printf("  URL: %s\n", url);

    // 调用 lyco 内置 API 创建项目
    LycoProject* proj = lyco_project_new(name, lang);
    lyco_project_set_url(proj, url);
    lyco_project_generate(proj);
    lyco_project_free(proj);

    printf("✅ 项目 '%s' 已创建\n", name);
    return 0;
}

// 清理入口 (必须导出)
PLUGIN_EXPORT void lyco_cleanup(void) {
    // 释放资源
}
```

#### 3. 编译为动态库

**Windows (MSVC):**
```bash
cl /LD new.c lyco_plugin.lib /Fe:new.dll
copy new.exe ~/.lyco/commands/
copy new.dll ~/.lyco/commands/
```

**Linux/macOS (GCC):**
```bash
gcc -shared -fPIC -o new.so new.c
cp new.so ~/.lyco/commands/
```

#### 4. 测试插件

```bash
lyco list          # 应显示你的插件
lyco new test python
```

---

## 插件生命周期

```
用户执行命令
    │
    ▼
┌──────────────────────┐
│ 1. 查找外部命令       │ ──→ 找到 → 加载 SO/DLL → 执行
│    ~/.lyco/commands/  │
└──────────┬───────────┘
           │ 未找到
           ▼
┌──────────────────────┐
│ 2. 执行内置命令       │
└──────────────────────┘
```

### 加载优先级

1. `~/.lyco/commands/{cmd}.dll` (Windows) / `~/.lyco/commands/{cmd}.so` (Linux/macOS)
2. 内置命令

### 导出函数签名

```c
// 初始化 (必须)
int lyco_init(CommandInfo* info);

// 执行命令 (必须)
int lyco_execute(int argc, char** argv);

// 清理资源 (可选)
void lyco_cleanup(void);

// 获取帮助信息 (可选)
const char* lyco_help(void);

// 获取版本 (可选)
const char* lyco_version(void);
```

---

## 命令 API

### 核心数据结构

```c
// 命令信息
typedef struct {
    const char* name;        // 命令名称
    const char* version;     // 版本号 (语义化版本)
    const char* description; // 简短描述
    const char* author;      // 作者
    const char* license;     // 许可证
    int         api_version; // API 版本号 (当前: 1)
} CommandInfo;

// 项目句柄 (不透明指针)
typedef struct LycoProject LycoProject;

// 配置键值对
typedef struct {
    const char* key;
    const char* value;
} LycoConfig;
```

### 项目 API

```c
// 创建项目
LycoProject* lyco_project_new(const char* name, const char* lang);

// 设置属性
void lyco_project_set_url(LycoProject* proj, const char* url);
void lyco_project_set_title(LycoProject* proj, const char* title);
void lyco_project_set_size(LycoProject* proj, int w, int h);
void lyco_project_set_color(LycoProject* proj, const char* color);

// 添加文件
void lyco_project_add_file(LycoProject* proj, const char* path, const char* content);
void lyco_project_add_template(LycoProject* proj, const char* tmpl_name, const char** vars);

// 生成项目
int lyco_project_generate(LycoProject* proj); // 返回 0 = 成功

// 释放
void lyco_project_free(LycoProject* proj);
```

### 模板 API

```c
// 读取模板 (优先用户自定义,其次默认)
const char* lyco_template_read(const char* name);

// 写入用户自定义模板
int lyco_template_write(const char* name, const char* content);

// 列出所有模板
const char** lyco_template_list(int* count);

// 释放模板列表
void lyco_template_list_free(const char** list);

// 变量替换
// 支持 {NAME} {URL} {DEBUG} 等变量
char* lyco_template_substitute(const char* template, LycoConfig* configs, int count);
```

### 文件系统 API

```c
// 获取 lyco 数据目录
const char* lyco_data_dir(void);        // ~/.lyco/

// 获取模板目录
const char* lyco_templates_dir(void);   // ~/.lyco/templates/

// 获取命令目录
const char* lyco_commands_dir(void);    // ~/.lyco/commands/

// 获取 Web UI 目录
const char* lyco_web_dir(void);         // ~/.lyco/web/

// 创建目录
int lyco_mkdir(const char* path);

// 检查文件存在
int lyco_file_exists(const char* path);

// 读取文件内容 (自动释放)
char* lyco_read_file(const char* path);

// 写入文件
int lyco_write_file(const char* path, const char* content);

// 复制文件
int lyco_copy_file(const char* src, const char* dst);
```

### 构建 API

```c
// 执行构建命令
int lyco_build(const char* project_dir, const char* target);

// 运行项目
int lyco_run(const char* project_dir);

// 清理构建产物
int lyco_clean(const char* project_dir);

// 获取构建状态
typedef enum {
    LYCO_BUILD_OK = 0,
    LYCO_BUILD_ERROR = 1,
    LYCO_BUILD_RUNNING = 2,
    LYCO_BUILD_CANCELLED = 3
} LycoBuildStatus;

LycoBuildStatus lyco_build_status(void);
```

### 日志 API

```c
typedef enum {
    LYCO_LOG_DEBUG = 0,
    LYCO_LOG_INFO = 1,
    LYCO_LOG_WARN = 2,
    LYCO_LOG_ERROR = 3
} LycoLogLevel;

void lyco_log(LycoLogLevel level, const char* fmt, ...);
```

### 网络 API (可选,用于 AI 集成)

```c
// HTTP 请求
typedef struct {
    const char* url;
    const char* method;     // GET, POST, PUT, DELETE
    const char* body;       // 请求体
    const char** headers;   // 请求头
    int header_count;
} LycoHttpRequest;

typedef struct {
    int status_code;
    const char* body;
    int body_len;
} LycoHttpResponse;

LycoHttpResponse* lyco_http_request(const LycoHttpRequest* req);
void lyco_http_response_free(LycoHttpResponse* resp);
```

### AI API (可选)

```c
// AI 配置
typedef struct {
    const char* provider;    // "openai", "anthropic", "ollama", "custom"
    const char* api_key;
    const char* base_url;
    const char* model;
    float temperature;
    int max_tokens;
} LycoAIConfig;

// 初始化 AI
int lyco_ai_init(const LycoAIConfig* config);

// AI 对话
typedef struct {
    const char* role;    // "user", "assistant", "system"
    const char* content;
} LycoAIMessage;

const char* lyco_ai_chat(const LycoAIMessage* messages, int count);

// 释放 AI 响应
void lyco_ai_response_free(const char* response);
```

---

## 模板系统

### 内置变量

| 变量 | 说明 | 示例 |
|------|------|------|
| `{NAME}` | 项目名称 | `my-app` |
| `{URL}` | 加载的 URL | `https://example.com` |
| `{DEBUG}` | 是否调试模式 | `true` / `false` |
| `{WIDTH}` | 窗口宽度 | `1100` |
| `{HEIGHT}` | 窗口高度 | `760` |
| `{COLOR}` | 主题色 | `#1a1a2e` |
| `{LANG}` | 编程语言 | `python` |
| `{YEAR}` | 当前年份 | `2026` |
| `{AUTHOR}` | 作者名 | 从配置读取 |
| `{LICENSE}` | 许可证 | `MIT` |

### 自定义模板

用户可以在 `~/.lyco/templates/` 下创建自定义模板:

```
~/.lyco/templates/
├── main.c              # C 主文件模板
├── main.py             # Python 主文件模板
├── main.rs             # Rust 主文件模板
├── main.go             # Go 主文件模板
├── main.ts             # TypeScript 主文件模板
├── Main.java           # Java 主文件模板
├── main.zig            # Zig 主文件模板
├── Program.cs          # C# 主文件模板
├── main.e              # 易语言模板
├── xmake.lua           # xmake 构建模板
├── package.json        # Node.js 包模板
├── index.html          # HTML 入口模板
├── android_manifest.xml # Android 清单模板
├── build_apk.sh        # Android 构建脚本
├── README.md           # 项目说明模板
└── .gitignore          # Git 忽略模板
```

### 模板优先级

1. `~/.lyco/templates/{name}` (用户自定义)
2. 内置模板 (编译进二进制)

---

## 配置系统

### 配置文件位置

```
~/.lyco/config.json
```

### 配置格式

```json
{
  "version": "1.0",
  "user": {
    "name": "开发者名",
    "email": "dev@example.com",
    "license": "MIT"
  },
  "defaults": {
    "language": "c",
    "url": "https://example.com",
    "width": 1100,
    "height": 760,
    "color": "#1a1a2e"
  },
  "ai": {
    "provider": "openai",
    "api_key": "sk-...",
    "base_url": "https://api.openai.com/v1",
    "model": "gpt-4"
  },
  "plugins": {
    "auto_update": true,
    "sources": [
      "https://plugins.lyco.dev"
    ]
  },
  "recent_projects": [
    {"name": "my-app", "path": "/path/to/my-app", "last_open": "2026-09-05T10:30:00Z"}
  ]
}
```

### 配置 API

```c
// 读取配置
const char* lyco_config_get(const char* key);  // 支持点号路径: "ai.model"

// 写入配置
int lyco_config_set(const char* key, const char* value);

// 删除配置
int lyco_config_delete(const char* key);

// 保存配置到磁盘
int lyco_config_save(void);

// 重新加载配置
int lyco_config_reload(void);
```

---

## Web UI 扩展

### 自定义 Web UI

将自定义 HTML 文件放入 `~/.lyco/web/`:

```
~/.lyco/web/
├── index.html          # 主界面 (覆盖默认)
├── style.css           # 自定义样式
├── app.js              # 自定义脚本
└── assets/             # 静态资源
    ├── logo.png
    └── icon.svg
```

### Web API (HTTP 接口)

启动 `lyco web` 后,提供以下 REST API:

```
GET  /api/status           # 获取 lyco 状态
GET  /api/projects         # 列出所有项目
POST /api/projects         # 创建项目
GET  /api/projects/:name   # 获取项目详情
PUT  /api/projects/:name   # 更新项目
DELETE /api/projects/:name # 删除项目
POST /api/projects/:name/build  # 构建项目
POST /api/projects/:name/run    # 运行项目
GET  /api/templates        # 列出模板
PUT  /api/templates/:name  # 更新模板
GET  /api/config           # 获取配置
PUT  /api/config           # 更新配置
WS   /api/ws               # WebSocket 实时日志
```

### 项目 JSON 格式

```json
{
  "name": "my-app",
  "language": "python",
  "url": "https://example.com",
  "targets": ["windows", "android"],
  "created_at": "2026-09-05T10:00:00Z",
  "updated_at": "2026-09-05T12:00:00Z",
  "files": [
    {"path": "main.py", "size": 256, "modified": "2026-09-05T10:00:00Z"}
  ],
  "config": {
    "width": 1100,
    "height": 760,
    "debug": false
  }
}
```

---

## 完整示例

### 示例 1: Python 插件 (带 AI 辅助)

```python
# new.py - 自定义 new 命令
import sys
import os
import json

LYCO_DIR = os.path.expanduser("~/.lyco")
CONFIG_FILE = os.path.join(LYCO_DIR, "config.json")

def load_config():
    if os.path.exists(CONFIG_FILE):
        with open(CONFIG_FILE) as f:
            return json.load(f)
    return {}

def save_config(cfg):
    os.makedirs(LYCO_DIR, exist_ok=True)
    with open(CONFIG_FILE, 'w') as f:
        json.dump(cfg, f, indent=2)

def read_template(name):
    user_path = os.path.join(LYCO_DIR, "templates", name)
    if os.path.exists(user_path):
        with open(user_path) as f:
            return f.read()
    # 返回默认模板
    return get_default_template(name)

def get_default_template(name):
    templates = {
        "main.py": 'import webview\nwindow = webview.create_window("{NAME}", "{URL}")\nwebview.start()\n',
        "main.c": '#include "webview.h"\nint main(){{\n  webview_t w = webview_create(0, NULL);\n  webview_navigate(w, "{URL}");\n  webview_run(w);\n  return 0;\n}}\n',
    }
    return templates.get(name, "")

def cmd_new(name, url="https://example.com", lang="python"):
    if os.path.exists(name):
        print(f"错误: 目录 '{name}' 已存在")
        return 1

    print(f"🚀 创建项目: {name} ({lang})")
    os.makedirs(name, exist_ok=True)

    # 生成主文件
    tmpl = read_template(f"main.{lang}")
    content = tmpl.replace("{NAME}", name).replace("{URL}", url)
    with open(os.path.join(name, f"main.{lang}"), 'w') as f:
        f.write(content)

    # 生成 HTML
    html_tmpl = read_template("index.html")
    html = html_tmpl.replace("{NAME}", name).replace("{URL}", url)
    with open(os.path.join(name, "index.html"), 'w') as f:
        f.write(html)

    # 更新最近项目
    cfg = load_config()
    recent = cfg.get("recent_projects", [])
    recent.insert(0, {"name": name, "path": os.path.abspath(name)})
    cfg["recent_projects"] = recent[:10]
    save_config(cfg)

    print(f"✅ 项目 '{name}' 已创建")
    print(f"  cd {name} && lyco run")
    return 0

if __name__ == "__main__":
    args = sys.argv[1:]
    if len(args) < 2:
        print("用法: lyco new <name> <lang> [url]")
        sys.exit(1)
    url = args[2] if len(args) > 2 else "https://example.com"
    sys.exit(cmd_new(args[0], url, args[1]))
```

### 示例 2: Rust 插件 (带构建缓存)

```rust
// build.rs - 自定义 build 命令 (带增量编译缓存)
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const CACHE_DIR: &str = ".lyco_cache";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("用法: lyco build [target]");
        std::process::exit(1);
    }

    let target = args.get(1).map(|s| s.as_str()).unwrap_or("windows");
    println!("🔨 构建 target={}", target);

    // 检查缓存
    let cache_key = compute_cache_key();
    let cache_path = PathBuf::from(CACHE_DIR).join(&cache_key);

    if cache_path.exists() {
        println!("  ⚡ 命中缓存,跳过构建");
        return;
    }

    // 执行构建
    let status = Command::new("xmake")
        .status()
        .expect("xmake 未安装");

    if !status.success() {
        eprintln!("❌ 构建失败");
        std::process::exit(1);
    }

    // 写入缓存标记
    let _ = fs::create_dir_all(CACHE_DIR);
    let _ = fs::write(&cache_path, "ok");

    println!("✅ 构建完成");
}

fn compute_cache_key() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    // 基于源文件内容计算哈希
    if let Ok(entries) = fs::read_dir("src") {
        for entry in entries.flatten() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                content.hash(&mut hasher);
            }
        }
    }
    format!("{:x}", hasher.finish())
}
```

### 示例 3: Node.js 插件 (带热重载)

```javascript
// run.js - 自定义 run 命令 (带热重载)
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const WATCH_DIRS = ['src', 'web'];
const WATCH_EXTS = ['.c', '.py', '.js', '.ts', '.html', '.css'];

function watchAndRun(command) {
    let child = null;

    const start = () => {
        console.log(`▶ 启动: ${command.join(' ')}`);
        child = spawn(command[0], command.slice(1), { stdio: 'inherit' });
    };

    const restart = () => {
        if (child) {
            child.kill();
            console.log('🔄 重启...');
        }
        start();
    };

    start();

    // 监听文件变化
    WATCH_DIRS.forEach(dir => {
        if (!fs.existsSync(dir)) return;
        fs.watch(dir, { recursive: true }, (event, filename) => {
            if (WATCH_EXTS.some(ext => filename.endsWith(ext))) {
                restart();
            }
        });
    });
}

const args = process.argv.slice(2);
watchAndRun(['xmake', 'run', ...args]);
```

---

## 发布与分发

### 插件清单文件 (`lyco.plugin.json`)

```json
{
  "name": "my-awesome-plugin",
  "version": "1.0.0",
  "description": "为 lyco 添加 AI 辅助功能",
  "author": "Your Name",
  "license": "MIT",
  "api_version": 1,
  "commands": ["new", "build"],
  "platforms": {
    "windows": {
      "dll": "new-x64.dll",
      "exe": "new-x64.exe"
    },
    "linux": {
      "so": "new-x64.so",
      "bin": "new-x64"
    },
    "macos": {
      "so": "new-arm64.so",
      "bin": "new-arm64"
    }
  },
  "dependencies": [],
  "config_schema": {
    "api_key": {"type": "string", "required": false},
    "model": {"type": "string", "default": "gpt-4"}
  }
}
```

### 安装插件

```bash
# 从 GitHub 安装
lyco plugin install owner/repo

# 从本地安装
lyco plugin install ./my-plugin

# 列出已安装插件
lyco plugin list

# 卸载插件
lyco plugin remove my-plugin

# 更新插件
lyco plugin update my-plugin
```

---

## 版本兼容性

| lyco 版本 | API 版本 | 重大变更 |
|-----------|----------|----------|
| v0.5.x    | 1        | 初始版本 |
| v0.4.x    | 1        | 模板系统 |
| v0.3.x    | 1        | 基础命令 |

---

## 获取帮助

- GitHub: https://github.com/lilyco-42/webview-capi
- Issues: https://github.com/lilyco-42/webview-capi/issues
- Discussions: https://github.com/lilyco-42/webview-capi/discussions
