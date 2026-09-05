# demo/lyco-cli

Lyco CLI — cargo 风格的跨平台 WebView 项目管理器 (Rust)。

`Lyco.toml` 一个文件管所有,`lyco` 一行命令搞定构建。xmake 用户获得 cargo 体验。

## 30 秒上手

```bash
lyco new hello c        # 生成项目 (含 Lyco.toml, 默认依赖 webview-capi)
cd hello
lyco run                # 构建并弹出 WebView 窗口
```

## 常用命令 (与 cargo 对应)

| lyco | cargo 等价 | 作用 |
|------|-----------|------|
| `lyco new <name> <lang>` | `cargo new` | 新建项目 |
| `lyco add <dep>[@<ver>]` | `cargo add` | 添加依赖 (傻瓜式: 自动镜像源+系统库) |
| `lyco remove <dep>` | `cargo remove` | 移除依赖 (保留注释与格式) |
| `lyco build [-r]` | `cargo build [--release]` | 构建 (默认 debug) |
| `lyco build --target <plat>` | `cargo zigbuild --target` | 交叉构建 |
| `lyco run [-r]` | `cargo run` | 构建 + 运行 |
| `lyco check` | `cargo check` | 语法检查 (不产出目标文件) |
| `lyco test` | `cargo test` | 运行 tests/*.c (每个文件一个测试) |
| `lyco doc` | `cargo doc` | 生成文档 (需 doxygen) |
| `lyco init` | `cargo init` | 在现有目录初始化清单 |
| `lyco search [词]` | `cargo search` | 搜索依赖注册表 |
| `lyco update` | `cargo update` | 更新包仓库 |
| `lyco install` / `uninstall` | `cargo install` / `uninstall` | 安装到 ~/.lyco/bin / 卸载 |
| `lyco clean` | `cargo clean` | 清除构建产物 |

别名: `b`/`c`/`r`/`t`/`d` = build/check/run/test/doc。
未映射: `bench`(提示用 -r 产物压测)、`publish`(roadmap: tag + gh release)。

平台 (`--target`): windows / mingw / linux / macos / android / ios / wasm

## Lyco.toml

```toml
[package]
name = "hello"
version = "0.1.0"

[dependencies]
webview-capi = "*"
# lyco add webui        ← WebUI, 任意浏览器做前端
# lyco add webview-mini ← 单头文件极简版
```

- `lyco build` 每次从清单重新生成 `xmake.lua` (生成物, 勿手编)
- 依赖写法与 Cargo.toml 同风格: `"1.0"` 或 `{ version = "*", git = "..." }`

## 傻瓜依赖注册表

| 依赖 | lyco 自动处理 |
|------|--------------|
| `webview-capi` | lyco-mirror 镜像源 + user32/shell32/ole32/oleaut32/shlwapi/version + webview.dll 随 exe |
| `webui` | ws2_32/user32/gdi32/shell32/ole32 |
| `webview` | 同 webview-capi 系统库 |

## 工具链 (Windows)

自动探测: 有 `gcc` (如 `scoop install gcc`) → MinGW;否则回退 MSVC。
也可 `lyco build --target windows` 强制 MSVC、`--target mingw` 强制 MinGW。

## 构建 CLI 自身

```bash
cd lyco
cargo build --release   # 产物 target/release/lyco.exe
```

## 其他命令

- `lyco web` — 可视化 Web UI
- `lyco reset` — 重置 ~/.lyco/ (模板随 lyco 版本自动更新)
- `lyco info` / `lyco list` — 配置信息 / 命令列表
- 插件: 把 `xxx.dll` 放进 `~/.lyco/commands/`, `lyco xxx` 即可调用
