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
- 依赖写法与 Cargo.toml 同风格: `"1.0"` 或 `{ version = "1.0" }`
- `{ git = "URL" }`: 把该 URL 当 **xmake 包仓库** 加进 `xmake.lua`
  (即 `add_repositories("<依赖名> <URL>")`)。注意 xmake **没有 per-包 的 git 源**
  (`add_requires` 不接受 `git` 选项), 所以这个 URL 要指向含
  `packages/<名>/xmake.lua` 的包仓库, 而不是普通源码仓 —— 指向错了
  xmake 会明确报「not found in any repository」, 不会静默失败
- `{ path = "../myrepo" }`: 同上, 但指向**本地**包仓库目录。相对路径会先
  归一化成绝对路径 —— xmake 要求 `add_repositories` 的本地目录必须是
  绝对路径、且不能含 `..`, 所以这一步由 lyco 代做。目录不存在时
  `lyco build` 直接报错, 不会生成一个跑不通的 `xmake.lua`
- `git` 与 `path` 只能二选一 (同时写会报错)
- CLI 也能加: `lyco add myrepo --git <URL>` / `lyco add myrepo --path ../myrepo`

## 傻瓜依赖注册表

| 依赖 | lyco 自动处理 |
|------|--------------|
| `webview-capi` | lyco-mirror 镜像源 + user32/shell32/ole32/oleaut32/shlwapi/version + webview.dll 随 exe |
| `webview-mini` | 同 webview-capi 系统库 |
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

- `lyco web` — 可视化 Web UI（**UI 预览版**，需 `python3`/`python`；页面是纯静态演示，
  按钮尚未接入后端，实际操作请用命令行）
- `lyco reset` — 重置 ~/.lyco/（模板在 lyco 升级**或模板内容变化**时更新。
  **模板备份不会被删**，见下）
- `lyco info` / `lyco list` — 配置信息(含当前项目) / 已注册项目 + 命令列表
- 模板: `~/.lyco/templates/` 可以直接编辑。更新时**只覆盖你没改过的文件** ——
  改过的会被保留，并在输出里列出来（要换成新版就先把它们移走）。
- 备份: 凡是被新版本覆盖掉的**原内容**，都先存一份到 `~/.lyco/backup/<旧版本戳>/`，
  **布局镜像 `~/.lyco/`**（模板在 `templates/` 下、网页在 `web/` 下），
  目录里有 `.meta` 说明它是哪一版留下的、被哪一版替换掉的。
  只保留最近 **10** 份（更早的自动清理，清理时会告诉你）；要全部保留就设
  `LYCO_BACKUP_KEEP=0`。`lyco reset` 不会删这个目录。
  > 迁移（老用户第一次升级到带清单的版本）和注册表损坏自愈那两次，程序
  > **无从判断**哪些文件是你的心血，只能按默认覆盖 —— 备份就是为这两次准备的。
- `lyco restore` — 把备份放回去。不带参数列出可用备份（名字、文件数、多久以前、
  被哪一版替换）；带名字则恢复那一份。**恢复本身也不会不可逆**：覆盖前会先把当前
  内容另存一份（`pre-restore-<时间戳>`）。它不动 `.version` 与 `.manifest` ——
  所以恢复过的文件之后会被视为「你改过的」，后续模板更新不会覆盖它们。
- 插件: 在 `~/.lyco/commands/` 里放**两个**文件 ——
  `xxx.dll`(Linux/macOS 上是 `xxx.so`) 作为标记，以及同名的可执行文件
  `xxx.exe`(Linux/macOS 上是 `xxx`)。之后 `lyco xxx` 就会调用它。
  > 注意：**只放 `.dll` 是不够的**。原文档只写了 `.dll`，照着做会得到
  > 「找到插件 …但缺少配套的可执行文件」。`.dll` 本身从不被加载，
  > 它只是「这里有个插件」的标记。
