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
fn backup_root()  -> PathBuf { data_dir().join("backup") }
fn db_path()      -> PathBuf { data_dir().join("lyco.db") }

/// 备份保留份数。默认 10 —— 一份备份就是那 15 个模板文件（几十 KB），
/// 留着比删掉便宜得多；设 `LYCO_BACKUP_KEEP=0` 表示**永不自动清理**。
/// 值不合法时退回默认（不报错 —— 一个环境变量的笔误不该让命令失败）。
fn backup_keep() -> usize {
    env::var("LYCO_BACKUP_KEEP")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(10)
}

/// 去掉 Windows `canonicalize()` 返回的 `\\?\` verbatim 前缀。
///
/// 不去掉的话，`lyco new` 会把 `\\?\C:\Users\…\demo` 原样写进注册表，
/// `lyco list` 再把它打出来 —— 满屏反斜杠加问号。
/// （UNC 的形式是 `\\?\UNC\server\share`，要还原成 `\\server\share`。）
fn tidy_path(p: &Path) -> String {
    let s = p.to_string_lossy().to_string();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.to_string()
    } else {
        s
    }
}

// ── Lyco.toml (cargo 风格清单) ───────────────────────────────
mod manifest;

// ── 数据库（项目注册表） ──────────────────────────────────────
// 只留 `projects` 一张表 —— 它是**唯一**有代码真的在用的表：
// `lyco new` 写（add_project）、`lyco list` 读（list_projects）。
//
// 这次清掉的死东西（都是「看着正常其实没生效」）：
//   * `builds` / `plugins` / `config` 三张表：全仓没有任何一行代码读或写它们。
//     `plugins` 尤其明显 —— 插件信息本来就是扫 `commands/` 目录拿到的，
//     那张表永远不会有数据。`config` 只被下面那两个函数服务，而那两个函数
//     也从没被调用过。
//   * `get_config` / `set_config`：从未被调用。没有命令、没有文档、也没有
//     任何已定义的配置键，为它新造一个 `lyco config` 命令属于凭空加功能，
//     所以选择删掉而不是补完（与 `list_projects` 的处理不同 —— 那个已经有
//     `lyco list` 这个天然归宿）。
//   * `url` / `targets` 两列：`targets` 存的就是 `[lang]`（见 cmd_new 的调用），
//     和 `lang` 完全重复；`url` 写了但没人读。现在表的每一列都既写又读。
//   * `last_open` 列：从来没有任何代码写过它，而原来的
//     `ORDER BY last_open DESC` 于是对所有行都是 NULL、排序等于没排。
mod db {
    use super::*;
    use rusqlite::Connection;
    use std::sync::Mutex;

    static DB: once_cell::sync::Lazy<Mutex<Option<Connection>>> =
        once_cell::sync::Lazy::new(|| Mutex::new(None));

    fn get() -> std::sync::MutexGuard<'static, Option<Connection>> {
        DB.lock().unwrap()
    }

    /// 打开数据库并建表。**失败不 panic**。
    ///
    /// 原来这里是 `.expect("数据库打开失败")` / `.expect("数据库初始化失败")` ——
    /// 也就是说 `lyco new` 会因为「一个用来记项目名的附带数据库打不开」
    /// 而整条命令 panic。注册表只是便利功能，不该有这种杀伤力。
    pub fn init() {
        let mut db = get();
        if db.is_none() {
            let _ = fs::create_dir_all(data_dir());
            let conn = match Connection::open(db_path()) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("⚠  项目注册表不可用 ({e}); 不影响构建与新建项目");
                    return;
                }
            };
            if let Err(e) = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS projects (
                    name TEXT PRIMARY KEY,
                    lang TEXT NOT NULL,
                    path TEXT NOT NULL,
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
                );",
            ) {
                eprintln!("⚠  项目注册表建表失败 ({e}); 不影响构建与新建项目");
                return;
            }
            *db = Some(conn);
        }
    }

    pub fn add_project(name: &str, lang: &str, path: &str) {
        init();
        let db = get();
        let Some(conn) = db.as_ref() else { return }; // 注册表不可用就静默跳过
        let _ = conn.execute(
            "INSERT OR REPLACE INTO projects (name, lang, path) VALUES (?1, ?2, ?3)",
            rusqlite::params![name, lang, path],
        );
    }

    /// 返回 `(名字, 语言, 路径, 创建时间)`，新的在前。
    pub fn list_projects() -> Vec<(String, String, String, String)> {
        // 没建过库就说明一个项目都没登记过 —— 别为了「列个表」这种只读操作
        // 顺手把 ~/.lyco/lyco.db 创建出来。
        if !db_path().exists() {
            return vec![];
        }
        init();
        let db = get();
        let Some(conn) = db.as_ref() else { return vec![] };
        // 只 SELECT 新旧 schema 都有的列，免得遇到老库直接失败
        let Ok(mut stmt) =
            conn.prepare("SELECT name, lang, path, created_at FROM projects ORDER BY created_at DESC")
        else {
            return vec![];
        };
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        });
        match rows {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => vec![],
        }
    }
}

// ── 内置模板 ─────────────────────────────────────────────────
// 模板渲染不用模板引擎：`subst()` 直接替换 `{K}` 占位符就够了。
// 这里原来还有一个 `mod tmpl`（Tera 封装），但两个函数从未被调用过 ——
// 而且 `subst()` 的注释里已经写明「Tera 会把 {K} 当纯文本, 故直接替换」，
// 也就是说 Tera 是被**主动换掉**的，那个模块是换完之后没删干净的残留。
// 连同 `tera` 依赖一起删了（删完 `cargo build` 的警告从 9 个降到 0 个）。
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

/// 已释放内容的清单：每行 `<fnv1a 十六进制>\t<相对路径>`。
/// 用途：重释放时区分「用户没动过」（可安全覆盖）与「用户改过」（必须保留）。
fn manifest_path() -> PathBuf { templates_dir().join(".manifest") }

fn released_hashes() -> Vec<(String, String)> {
    fs::read_to_string(manifest_path())
        .map(|s| {
            s.lines()
                .filter_map(|l| l.split_once('\t'))
                .map(|(h, n)| (n.to_string(), h.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// `release_templates` 的结果 —— 用来决定该跟用户说什么。
#[derive(Default)]
struct ReleaseReport {
    /// 用户改过、被保留（没覆盖）的文件
    kept: Vec<String>,
    /// 覆盖前成功备份过的文件
    backed_up: Vec<String>,
    /// 想备份但失败了 —— 这些文件的**原内容已经丢了**，必须报出来
    backup_failed: Vec<String>,
    /// 备份目录（`backed_up` 非空时才有）
    backup_dir: Option<PathBuf>,
    /// 是否走了「无从判断 → 按老行为覆盖」那条路
    forced: bool,
    /// 因为超过保留份数而被清理掉的**更早**的备份（目录名）
    pruned: Vec<String>,
}

/// 把版本戳变成安全的目录名。
///
/// 戳是从**用户可写的文件**（`~/.lyco/templates/.version`）里读出来的，
/// 不能让 `..`、`/`、`\` 之类穿进路径 —— 一个 `..` 就能把备份写到 `~/.lyco/` 里去。
/// 也不能让结果以 `.` 开头（隐藏目录 = 用户看不见自己的备份）。
fn sanitize_stamp(s: &str) -> String {
    let mapped: String = s
        .chars()
        .take(64)
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') { c } else { '_' }
        })
        .collect();
    // 再去掉开头的 `.` / `-` / `_`：
    // * 开头的 `.` 会让目录**变成隐藏目录**（`..evil` → `.._evil`）—— 用户备份完
    //   用自己的 `ls` 根本看不见，等于没备份；
    // * 开头的 `-` 会让目录名在命令行里长得像选项。
    // 顺带覆盖了「`..` / `___` 这类没信息量的名字」→ 落到 `unknown`。
    let out = mapped.trim_start_matches(['.', '-', '_']);
    if out.is_empty() { "unknown".to_string() } else { out.to_string() }
}

/// 释放内嵌模板。
///
/// 策略：**只覆盖用户没动过的文件**。判据是 `.manifest`（上次释放时各文件的哈希）：
///
/// * 没有清单、或清单里一条可用条目都没有（首次安装、从还没有清单的旧版本升上来、
///   清单损坏）→ 无从判断，按老行为覆盖 —— 旧版本本来就在升级时全覆盖，
///   所以这不是倒退，而是迁移；
/// * 有清单、且当前内容与清单一致 → 用户没动过 → 覆盖成新版；
/// * 有清单、但内容不一致 → **用户改过 → 保留**，只把文件名报回去。
///
/// 另外，**凡是要覆盖掉一份不同的内容，先把它备份**到
/// `~/.lyco/backup/<旧版本戳>/`（布局镜像 `~/.lyco/`，所以 `cp -r` 就能放回去；
/// 也可以直接用 `lyco restore <版本戳>`）。同一个旧戳被覆盖两次时**不会互相覆盖** ——
/// 第二次会退让成 `<旧版本戳>-2`、`-3`…（见 `unique_backup_dir`）。
/// 这是兜底：迁移那一次和清单损坏自愈那一次，
/// 我们**无从判断**哪些文件是用户的心血，备份至少让损失可恢复；
/// 「存在但读不出来」（非 UTF-8）的文件也是靠这一层救回来的
/// —— 否则它会被当成"不存在"直接覆盖掉。
///
/// 刻意保留的行为：**文件被删掉时会写回来**。删掉也算"动过"，但一个残缺的模板集
/// 是坏状态（虽然 `read_template_or_default` 会兜底），而误删后无声地永久缺一个文件
/// 更难排查。所以这里选择"恢复并让人看见"。
///
/// 这样「编辑 `~/.lyco/templates/` 定制」才真的能用：以前任何一次升级
/// （现在是任何一次模板改动）都会把定制**无声**抹掉。
///
/// `old_stamp` 只用来给备份目录命名（= 用户升级前的那一版），取不到时用 `unknown`。
/// `new_stamp` 写进备份目录的自述文件，让用户知道「这份备份是被哪一版替换掉的」。
fn release_templates(old_stamp: &str, new_stamp: &str) -> std::io::Result<ReleaseReport> {
    let dir = templates_dir();
    fs::create_dir_all(&dir)?;
    let web = web_dir();
    fs::create_dir_all(&web)?;

    // `(清单键, 备份里的相对路径, 内容, 目标路径)`
    //
    // 备份路径**镜像 `~/.lyco/` 的结构**（`templates/…`、`web/…`）。以前是平铺的
    // —— 模板文件直接躺在备份根目录下、网页却在 `web/` 子目录里，而这两者其实
    // 属于 `~/.lyco/` 下**两个不同的根**。用户看到 `backup/1.2.0/main.c` 与
    // `backup/1.2.0/web/index.html` 根本推不出各自该回到哪儿，所谓"备份"也就
    // 只能靠人工试。镜像之后，恢复就是一次机械的目录拷贝（`lyco restore` 做的
    // 就是这件事，但用户自己 `cp -r` 也能对）。
    //
    // 清单键**刻意保持原样**（还是 `main.c`，不是 `templates/main.c`）：改它会让
    // 老用户已有的 `.manifest` 全部失配，那些文件会被判成"你改过"而永久保留，
    // 模板更新从此静默失效。布局是给人看的，清单是给自己看的，两者不必一致。
    let mut jobs: Vec<(String, String, &str, PathBuf)> = ALL_TEMPLATES
        .iter()
        .map(|(n, c)| ((*n).to_string(), format!("templates/{n}"), *c, dir.join(n)))
        .collect();
    jobs.push((
        "web/index.html".to_string(),
        "web/index.html".to_string(),
        TEMPLATE_WEB_HTML,
        web.join("index.html"),
    ));

    let prev = released_hashes();
    // 清单里**一条可用条目都没有** = 无从判断 → 按老行为覆盖。
    //
    // 这里刻意判「有没有条目」而不是「文件在不在」：清单损坏或被清空时，
    // 后者会让每个文件都落进 `(Some(_), None) => !force` 那一支 ——
    // 全部被当成「用户改过」保留，且清单又被重写成空，**从此模板更新永久失效**
    // （每次只打一行警告，不报错）。判条目为空可以让这种状态**自愈**：
    // 覆盖一次、清单重建，下次就正常了。
    // 正常写出的清单永远非空（jobs 固定 15 项，至少有一项会被写入），
    // 所以这个判据与「没有清单」在实际场景下等价。
    let force = prev.is_empty();
    let mut rep = ReleaseReport { forced: force, ..Default::default() };
    let mut manifest = String::new();

    for (rel, backup_rel, content, dest) in jobs {
        let prev_hash = prev.iter().find(|(n, _)| *n == rel).map(|(_, h)| h.clone());
        let existed = dest.exists();
        let cur = fs::read_to_string(&dest).ok();
        let user_modified = match (cur.as_ref(), prev_hash.as_ref()) {
            (Some(c), Some(h)) => &hash_str(c) != h,
            (Some(_), None) => !force,
            // 不存在，或存在但**读不出来**（非 UTF-8）→ 覆盖。
            // 后者会在下面先被备份，所以不算无声丢失。
            (None, _) => false,
        };
        if user_modified {
            rep.kept.push(rel.clone());
            // 清单里仍记「上次释放的内容」—— 用户文件保持不动
            if let Some(h) = prev_hash {
                manifest.push_str(&format!("{h}\t{rel}\n"));
            }
            continue;
        }

        // 要覆盖掉一份**不同的**内容 → 先备份，否则这次覆盖不可恢复。
        // （内容一模一样就没必要备份。）
        if existed && cur.as_deref() != Some(content) {
            let target = {
                if rep.backup_dir.is_none() {
                    // 用 unique_backup_dir 而不是直接 join：同一个旧戳被覆盖两次时，
                    // 第二次不能把第一次的备份盖掉（见函数注释）。
                    rep.backup_dir = Some(unique_backup_dir(&sanitize_stamp(old_stamp)));
                }
                rep.backup_dir.as_ref().unwrap().join(&backup_rel)
            };
            if let Some(p) = target.parent() {
                let _ = fs::create_dir_all(p);
            }
            if fs::copy(&dest, &target).is_ok() {
                rep.backed_up.push(rel.clone());
            } else {
                // 备份失败就不能不吭声 —— 这次覆盖是真的会丢内容
                rep.backup_failed.push(rel.clone());
            }
        }

        fs::write(&dest, content)?;
        manifest.push_str(&format!("{}\t{rel}\n", hash_str(content)));
    }
    let _ = fs::write(manifest_path(), manifest);

    // 给这次备份写一份自述。两个用处：
    // 1. 用户打开目录就能看懂「这是哪一版留下的、被哪一版换掉的」；
    // 2. 清理时能**可靠排序** —— 目录名里的内容指纹是随机序（`1.2.0-aa87…`），
    //    按名字排会删错。
    if let Some(b) = &rep.backup_dir {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let _ = fs::write(
            b.join(".meta"),
            format!(
                "created={now}\nreplaced_by={new_stamp}\nlyco={}\n",
                env!("CARGO_PKG_VERSION")
            ),
        );
    }
    // 清理更早的备份。`LYCO_BACKUP_KEEP=0` → 永不自动清理。
    // 先算出来再赋值，避免「赋值目标与借用的字段在同一句里」这种需要
    // 借用检查器细究的写法（本机不编译，不给自己留疑问）。
    let pruned = prune_backups(backup_keep(), rep.backup_dir.as_deref());
    rep.pruned = pruned;
    Ok(rep)
}

/// 给备份找一个**还没被占用**的目录名。
///
/// 为什么需要它：备份目录名只含「被覆盖的那一版戳」。同一个旧戳被覆盖两次时
/// （清单损坏自愈、或用户手动把 `.version` 改回旧值），两次会写进同一个目录 ——
/// 第二次的 `fs::copy` 会把**第一次的备份覆盖掉**，而那份可能就是用户唯一的副本。
/// 所以这里退让一位：`1.2.0` 被占了就用 `1.2.0-2`、`1.2.0-3`…
///
/// 生成的名字必须仍是 `sanitize_stamp` 的不动点（只含 `[A-Za-z0-9._-]`），
/// 否则 `prune_backups` / `cmd_restore` 的「只认自己的东西」判据会把它当用户的杂物跳过。
fn unique_backup_dir(base: &str) -> PathBuf {
    let root = backup_root();
    let first = root.join(base);
    if !first.exists() {
        return first;
    }
    // 上限只是防御性的：正常最多撞一两次。
    for i in 2u32..10_000 {
        let cand = root.join(format!("{base}-{i}"));
        if !cand.exists() {
            return cand;
        }
    }
    // 极端情况（同一秒里撞上万次）—— 时间戳兜底，仍是合法名字。
    root.join(format!("{base}-{}", now_secs()))
}

/// 备份目录的排序键（越大越新）。
///
/// 优先读自述文件里的 `created=`；读不到（本次改动之前创建的备份、或自述写失败）
/// 就退回**目录 mtime** —— 备份写完之后我们不再动它，所以 mtime 就是创建时刻。
fn backup_sort_key(dir: &Path) -> u64 {
    if let Ok(t) = fs::read_to_string(dir.join(".meta")) {
        for line in t.lines() {
            if let Some(v) = line.strip_prefix("created=") {
                if let Ok(n) = v.trim().parse::<u64>() {
                    return n;
                }
            }
        }
    }
    fs::metadata(dir)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 只保留最新的 `keep` 份备份，返回被清理掉的目录名。`keep == 0` 表示全部保留。
///
/// **只动我们自己的东西**，三道闸门缺一不可（外加一条 `protect`）：
/// * 只处理 `backup/` 的**直接子目录**（不递归、不碰散落的文件）；
/// * 名字必须满足 `sanitize_stamp(name) == name` —— `sanitize_stamp` 的值域
///   正好是它的不动点集合，所以这条判据精确等于「这个名字可能是我们创建的」，
///   天然挡掉 `..evil`、带空格/中文的名字；
/// * 目录里必须有我们写的 `.meta`。**这一条才是真正区分「我们的备份」与
///   「用户在 backup/ 里自己放的东西」的判据** —— 只靠名字区分不了
///   （`my-own-notes` 也是合法的 sanitize 结果）。所以没有 `.meta` 的目录一律不碰。
/// * `protect` 指定的目录（本次刚写的那份）永不删除。
fn prune_backups(keep: usize, protect: Option<&Path>) -> Vec<String> {
    if keep == 0 {
        return Vec::new();
    }
    let root = backup_root();
    let rd = match fs::read_dir(&root) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut dirs: Vec<(u64, PathBuf)> = Vec::new();
    for e in rd.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let name = match p.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue, // 非 UTF-8 名字：不认领，也就不动它
        };
        if sanitize_stamp(&name) != name {
            continue; // 不是我们会创建的名字 → 是用户自己的东西，别碰
        }
        if !p.join(".meta").is_file() {
            continue; // 没有我们的自述文件 → 不是我们的备份，别碰
        }
        if protect == Some(p.as_path()) {
            continue; // 本次刚写的那份，永不删
        }
        dirs.push((backup_sort_key(&p), p));
    }
    if dirs.len() <= keep {
        return Vec::new();
    }
    dirs.sort_by(|a, b| b.0.cmp(&a.0)); // 新的在前
    let mut removed = Vec::new();
    for (_, p) in dirs.into_iter().skip(keep) {
        let name = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if fs::remove_dir_all(&p).is_ok() {
            removed.push(name);
        }
    }
    removed
}

/// 当前 Unix 秒。系统时钟早于 1970（取不到）时当 0。
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 把时间差说成人话。只做粗粒度，够用户判断"这是不是刚才那次"就行。
fn human_age(now: u64, then: u64) -> String {
    let d = now.saturating_sub(then);
    if d < 60 {
        format!("{d} 秒前")
    } else if d < 3600 {
        format!("{} 分钟前", d / 60)
    } else if d < 86_400 {
        format!("{} 小时前", d / 3600)
    } else {
        format!("{} 天前", d / 86_400)
    }
}

/// 读备份自述文件里的某个字段（`created=` / `replaced_by=` / `lyco=`）。
/// 读不到就返回 `?` —— 展示用，不值得为它报错。
fn meta_field(dir: &Path, key: &str) -> String {
    fs::read_to_string(dir.join(".meta"))
        .ok()
        .and_then(|t| {
            t.lines()
                .find_map(|l| l.strip_prefix(key).and_then(|v| v.strip_prefix('=')))
                .map(|v| v.trim().to_string())
        })
        .unwrap_or_else(|| "?".to_string())
}

/// 递归列出目录下的文件，返回 `(相对路径, 绝对路径)`，按相对路径排序。
///
/// 跳过以 `.` 开头的项 —— 我们自己的 `.meta` 就在备份根目录里，它不是模板；
/// 模板名也都不以点开头（`ALL_TEMPLATES` 里没有）。
/// 相对路径统一成正斜杠，方便打印、比较与写进断言。
fn walk_files(root: &Path) -> Vec<(String, PathBuf)> {
    let mut out: Vec<(String, PathBuf)> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let rd = match fs::read_dir(&d) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for e in rd.flatten() {
            let p = e.path();
            let name = match p.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            if name.starts_with('.') {
                continue;
            }
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if let Ok(r) = p.strip_prefix(root) {
                out.push((r.to_string_lossy().replace('\\', "/"), p));
            }
        }
    }
    out.sort();
    out
}

/// 备份里的相对路径 → 它该回到 `~/.lyco/` 下的哪个位置。
///
/// 新布局镜像 `~/.lyco/`（`templates/…` / `web/…`）；老布局是平铺的
/// （模板文件直接躺在备份根目录下），所以**没有前缀**的按模板处理 ——
/// 这样两种布局都能恢复，不用为老备份写迁移。
fn backup_rel_to_dest(rel: &str) -> PathBuf {
    if let Some(rest) = rel.strip_prefix("templates/") {
        templates_dir().join(rest)
    } else if let Some(rest) = rel.strip_prefix("web/") {
        web_dir().join(rest)
    } else {
        templates_dir().join(rel)
    }
}

/// `lyco restore` —— 不带参数列出可用备份；带名字则把那份备份放回去。
///
/// 刻意**不动 `.version` 与 `.manifest`**：恢复之后这些文件与清单不一致，
/// 于是会被 `release_templates` 判定为「用户改过」而**保留** —— 这正是用户
/// 要的结果（我就是要这一版）。若顺手把清单改成恢复后的内容，下一次释放会
/// 立刻把它们覆盖掉，恢复等于白做。
///
/// 恢复会覆盖**当前**内容，所以先把当前内容存一份（`backup/pre-restore-<秒>/`，
/// 同一秒内恢复两次会退让成 `-2`、`-3`…）—— 不能让"恢复"本身变成一次新的丢失。
fn cmd_restore(args: &[String]) {
    let root = backup_root();
    let mut dirs: Vec<(u64, PathBuf)> = Vec::new();
    if let Ok(rd) = fs::read_dir(&root) {
        for e in rd.flatten() {
            let p = e.path();
            // 与我们自己的备份判据一致：目录 + 有自述文件。
            // 用户在 backup/ 里自己放的东西不会被列进来，也不会被碰。
            if !p.is_dir() || !p.join(".meta").is_file() {
                continue;
            }
            dirs.push((backup_sort_key(&p), p));
        }
    }
    dirs.sort_by(|a, b| b.0.cmp(&a.0)); // 新的在前

    let Some(want) = args.first() else {
        if dirs.is_empty() {
            println!("没有备份 ({})", root.display());
            return;
        }
        let now = now_secs();
        println!("可用备份 (新 → 旧):");
        for (t, p) in &dirs {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            let cnt = walk_files(p).len();
            let by = meta_field(p, "replaced_by");
            println!(
                "  {name:<40} {cnt:>3} 个文件  {}  被 {by} 替换",
                human_age(now, *t)
            );
        }
        println!("\n用法: lyco restore <名字>");
        return;
    };

    let stamp = want.as_str();
    // 名字来自**命令行** → 当不可信输入处理。要求它是 `sanitize_stamp` 的不动点
    // （= 我们可能创建出来的名字），`../..` 这类会在这里被挡住。
    if sanitize_stamp(stamp) != stamp {
        eprintln!("❌ 备份名不合法: {stamp}");
        eprintln!("   用 `lyco restore` 不带参数可以看到可用的名字");
        std::process::exit(1);
    }
    let src = root.join(stamp);
    if !src.is_dir() || !src.join(".meta").is_file() {
        eprintln!("❌ 找不到备份 {stamp} ({})", src.display());
        eprintln!("   用 `lyco restore` 不带参数可以看到可用的名字");
        std::process::exit(1);
    }
    let files = walk_files(&src);
    if files.is_empty() {
        eprintln!("❌ 备份 {stamp} 里没有文件");
        std::process::exit(1);
    }

    // 同样走 unique_backup_dir：同一秒内恢复两次也不会互相覆盖。
    let pre = unique_backup_dir(&format!("pre-restore-{}", now_secs()));
    let mut restored: Vec<String> = Vec::new();
    let mut saved: Vec<String> = Vec::new();
    let mut failed: Vec<String> = Vec::new();
    for (rel, from) in files {
        let dest = backup_rel_to_dest(&rel);
        if let Some(par) = dest.parent() {
            let _ = fs::create_dir_all(par);
        }
        // 只有"当前内容与备份里那份不一样"才值得先存 —— 一样就没什么可丢的。
        let differs = match (fs::read(&dest), fs::read(&from)) {
            (Ok(a), Ok(b)) => a != b,
            (Err(_), _) => false, // 现在没这个文件 → 覆盖不会丢东西
            (Ok(_), Err(_)) => true,
        };
        if differs {
            let keep = pre.join(&rel);
            if let Some(par) = keep.parent() {
                let _ = fs::create_dir_all(par);
            }
            if fs::copy(&dest, &keep).is_ok() {
                saved.push(rel.clone());
            }
        }
        match fs::copy(&from, &dest) {
            Ok(_) => restored.push(rel),
            Err(e) => failed.push(format!("{rel} ({e})")),
        }
    }
    if !saved.is_empty() {
        let _ = fs::write(
            pre.join(".meta"),
            format!(
                "created={}\nreplaced_by=restore:{stamp}\nlyco={}\n",
                now_secs(),
                env!("CARGO_PKG_VERSION")
            ),
        );
    }
    println!("♻️  已从备份 {stamp} 恢复 {} 个文件", restored.len());
    for r in &restored {
        println!("   {r}");
    }
    if !saved.is_empty() {
        println!("   💾 恢复前的内容已先存到 {}", pre.display());
    }
    if !failed.is_empty() {
        println!("   ❌ 以下文件恢复失败: {}", failed.join(", "));
        std::process::exit(1);
    }
    println!("   注意: 没有改动 .version 与 .manifest —— 这些文件之后会被视为");
    println!("   「你改过的」, 后续模板更新不会覆盖它们。");
}

/// FNV-1a 64 位。自己实现而不引依赖：只要输入相同、结果永远相同
/// （std 的 `DefaultHasher` 不保证跨 Rust 版本稳定）。
fn fnv1a(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h ^= u64::from(*b);
        *h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

fn hash_str(s: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    fnv1a(&mut h, s.as_bytes());
    format!("{h:016x}")
}

/// 模板内容指纹：按固定顺序把「文件名 + 内容」喂进 FNV-1a。
fn templates_fingerprint() -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for (name, content) in ALL_TEMPLATES {
        fnv1a(&mut h, name.as_bytes());
        fnv1a(&mut h, b"\0"); // 分隔符: 防止 ("ab","c") 与 ("a","bc") 撞
        fnv1a(&mut h, content.as_bytes());
        fnv1a(&mut h, b"\0");
    }
    fnv1a(&mut h, b"web/index.html\0");
    fnv1a(&mut h, TEMPLATE_WEB_HTML.as_bytes());
    format!("{h:016x}")
}

fn ensure_initialized() {
    // 版本戳 = 版本号 + **模板内容指纹**。只比对版本号是不够的：
    // 模板改了但 Cargo.toml 的版本没动时（例如只改 web.html 的文案），
    // 老用户永远拿不到新模板 —— 实测踩到过：~/.lyco/web/index.html 里一直
    // 留着旧页面（含一个开发机内网地址），因为 .version 还写着 1.2.0。
    let ver = env!("CARGO_PKG_VERSION");
    let want = format!("{ver}-{}", templates_fingerprint());
    let stamp = templates_dir().join(".version");
    let old_stamp = fs::read_to_string(&stamp).unwrap_or_default();
    let stale = old_stamp.trim() != want;
    if !templates_dir().exists() || stale {
        // 注意：必须在 release_templates 之前取 —— 它自己会 create_dir_all
        let first = !templates_dir().exists();
        let rep = match release_templates(old_stamp.trim(), &want) {
            Ok(r) => r,
            Err(e) => {
                // 释放失败不该让整个命令挂掉：模板读不到时
                // `read_template_or_default` 会回退到内嵌默认值，功能照常。
                eprintln!("⚠  模板释放失败 ({e}); 本次使用内嵌默认模板");
                return; // 不写版本戳 → 下次再试
            }
        };
        if first {
            println!("📦 首次运行,释放默认模板...");
        } else if rep.forced {
            println!("📦 lyco v{ver}: 模板已更新 (本地注册表缺失或损坏, 本次按默认模板覆盖)");
        } else {
            println!("📦 lyco v{ver}: 模板已更新");
        }
        if !rep.kept.is_empty() {
            println!(
                "   ⚠  以下模板你改过, 已保留未覆盖 (要换成新版请先移走它们): {}",
                rep.kept.join(", ")
            );
        }
        if let Some(b) = &rep.backup_dir {
            println!(
                "   💾 有 {} 个文件被新版本覆盖, 覆盖前的原内容已备份到 {}",
                rep.backed_up.len(),
                b.display()
            );
            // 光说"备份到哪"不够 —— 得让用户知道**怎么拿回来**，
            // 而且要能直接复制粘贴（所以打印目录名，不是全路径）。
            if let Some(n) = b.file_name() {
                println!("      (要恢复: lyco restore {})", n.to_string_lossy());
            }
        }
        if !rep.backup_failed.is_empty() {
            println!(
                "   ❌ 以下文件**备份失败**, 覆盖前的原内容已丢失: {}",
                rep.backup_failed.join(", ")
            );
        }
        if !rep.pruned.is_empty() {
            // 清理掉的要说出来，并给出"我不想让它清"的开关 —— 自动删用户的东西
            // 必须可见、可关。
            println!(
                "   🧹 备份只保留最近 {} 份, 已清理 {} 份更早的: {}",
                backup_keep(),
                rep.pruned.len(),
                rep.pruned.join(", ")
            );
            println!("      (要全部保留: 设 LYCO_BACKUP_KEEP=0)");
        }
        let _ = fs::create_dir_all(templates_dir());
        let _ = fs::write(&stamp, &want);
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
    // `flatten()` 跳过读不出来的目录项 —— 原来是 `entry.ok()?`，一个坏项会让
    // **整个插件发现**失败（表现是「插件明明装了却报未知命令」），而且原因
    // 完全看不出来。本项目的 `prune_backups` 也是这么处理的。
    for entry in fs::read_dir(&dir).ok()?.flatten() {
        let p = entry.path();
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
    // 登记到项目注册表（`lyco list` 会读它）。只记 name/lang/路径 ——
    // 原来的 url / targets 两列没人读，targets 还和 lang 完全重复。
    let abs = std::fs::canonicalize(name).unwrap_or_default();
    db::add_project(name, lang, &tidy_path(&abs));

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

/// 跑一个子进程，把「启动不了」和「退出码非 0」都变成**失败**。
///
/// 这一段原来全是 `let _ = Command::new(..).status()` —— 把退出码整个吞掉，
/// 然后无条件打印「✅ 完成」。后果是**同一件事有两套说法**：有 `Lyco.toml`
/// 时 `manifest::build` 会如实报错，没有清单时却编译失败也报成功；连
/// 「什么工程文件都没有、压根没干活」也报成功。用户看到 ✅ 就去跑产物，
/// 发现根本没有 —— 属于报喜不报忧。
fn run_step(what: &str, mut cmd: Command) -> Result<(), String> {
    match cmd.status() {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(match s.code() {
            Some(c) => format!("{what} 失败 (退出码 {c})"),
            // 被信号终止时 code() 是 None（Unix 上 SIGINT/SIGKILL 都这样）
            None => format!("{what} 被信号终止"),
        }),
        Err(e) => Err(format!("无法执行 {what}: {e}")),
    }
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

    // 没有 `Lyco.toml` → 兼容手写的 xmake / cmake 工程。
    //
    // 这一段原来把子进程退出码整个吞掉（`let _ = …status()`），再无条件打印
    // 「✅ 完成」：编译失败报成功，**连一个工程文件都没有、压根没干活**也报成功
    // —— 而且这两种情况的输出一模一样，用户根本分不出自己属于哪种。
    let r: Result<(), String> = if Path::new("xmake.lua").exists() {
        println!("🔨 构建 (xmake)...");
        run_step("xmake", Command::new("xmake"))
    } else if Path::new("CMakeLists.txt").exists() {
        println!("🔨 构建 (cmake)...");
        let mut r = fs::create_dir_all("build").map_err(|e| format!("创建 build/ 失败: {e}"));
        if r.is_ok() {
            let mut c = Command::new("cmake");
            c.args([".."]).current_dir("build");
            r = run_step("cmake 配置", c);
        }
        if r.is_ok() {
            let mut c = Command::new("cmake");
            c.args(["--build", "."]).current_dir("build");
            r = run_step("cmake 构建", c);
        }
        r
    } else {
        Err("当前目录没有可构建的工程: Lyco.toml / xmake.lua / CMakeLists.txt 都不存在".to_string())
    };
    match r {
        Ok(()) => println!("✅ 完成"),
        Err(e) => {
            eprintln!("❌ {e}");
            eprintln!("   装 xmake: scoop install xmake   或 https://xmake.io");
            std::process::exit(1);
        }
    }
}

fn cmd_run(cmd_args: &[String]) {
    if Path::new(manifest::MANIFEST).exists() {
        let (release, target) = parse_build_flags(cmd_args);
        if let Err(e) = manifest::run(release, target.as_deref()) {
            eprintln!("❌ {e}"); std::process::exit(1);
        }
        return;
    }
    // 非 `Lyco.toml` 工程：先构建。构建不成功会在 `cmd_build` 里 `exit(1)`，
    // 走不到「▶ 运行...」—— 原来那句 `let _ = xmake run` 是**构建失败也照跑**。
    cmd_build(&[]);
    println!("▶ 运行...");
    let mut c = Command::new("xmake");
    c.arg("run");
    if let Err(e) = run_step("xmake run", c) {
        eprintln!("❌ {e}");
        std::process::exit(1);
    }
}

/// `lyco add <dep>[@<ver>] [--git <url> | --path <dir>]`
fn cmd_add(args: &[String]) {
    let mut dep: Option<&str> = None;
    let mut git: Option<&str> = None;
    let mut path: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a.starts_with("--") {
            let (key, inline) = match a.split_once('=') {
                Some((k, v)) => (k, Some(v)),
                None => (a, None),
            };
            match key {
                "--git" | "--path" => {
                    let val = match inline {
                        Some(v) => v,
                        None => {
                            i += 1;
                            match args.get(i) {
                                Some(v) => v.as_str(),
                                None => { eprintln!("{key} 后面要跟一个值"); std::process::exit(1); }
                            }
                        }
                    };
                    if key == "--git" { git = Some(val) } else { path = Some(val) }
                }
                _ => { eprintln!("未知参数: {a}  (支持 --git <url> / --path <dir>)"); std::process::exit(1); }
            }
        } else if dep.is_none() {
            dep = Some(a);
        } else {
            eprintln!("多余的参数: {a}");
            std::process::exit(1);
        }
        i += 1;
    }
    let dep = match dep {
        Some(d) => d,
        None => {
            eprintln!("用法: lyco add <dep>[@<version>] [--git <url> | --path <dir>]");
            eprintln!("例:   lyco add webview-capi@1.0");
            eprintln!("      lyco add webui --git https://github.com/you/pkg-repo.git");
            eprintln!("      lyco add mypkg --path ../pkg-repo");
            std::process::exit(1);
        }
    };
    if let Err(e) = manifest::add(dep, git, path) { eprintln!("❌ {e}"); std::process::exit(1); }
    println!("  下一步: lyco build");
}

fn cmd_remove(dep: &str) {
    if let Err(e) = manifest::remove(dep) { eprintln!("❌ {e}"); std::process::exit(1); }
}

fn cmd_clean() {
    let dirs = ["build", ".xmake"];
    let mut failed: Vec<String> = Vec::new();
    for d in dirs {
        if Path::new(d).exists() {
            // 原来 `let _ = fs::remove_dir_all(d)` 之后无条件报「已清除」——
            // 删不掉（权限不足 / 被占用 / 同名的是个文件）也说清了。
            match fs::remove_dir_all(d) {
                Ok(()) => println!("🧹 已清除 {d}/"),
                Err(e) => failed.push(format!("{d} ({e})")),
            }
        }
    }
    if !failed.is_empty() {
        eprintln!("❌ 以下内容没能清除: {}", failed.join(", "));
        std::process::exit(1);
    }
}

// cargo init: 在现有目录生成清单 (不覆盖已有文件)
fn cmd_init(name: Option<&str>) {
    if Path::new(manifest::MANIFEST).exists() { eprintln!("Lyco.toml 已存在"); std::process::exit(1); }
    ensure_initialized();
    let dir_name = env::current_dir().ok()
        .and_then(|d| d.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "app".into());
    let name = name.unwrap_or(&dir_name);
    let vars = [("NAME", name), ("URL", "https://example.com"), ("DEBUG", "false"), ("YEAR", "2026")];
    write_file(manifest::MANIFEST, &subst(&read_template_or_default("Lyco.toml", TEMPLATE_LYCO_TOML), &vars));
    if !Path::new("src").exists() {
        let _ = fs::create_dir_all("src");
        write_file("src/main.c", &subst(&read_template_or_default("main.c", TEMPLATE_MAIN_C), &vars));
    }
    if !Path::new("index.html").exists() {
        write_file("index.html", &subst(&read_template_or_default("index.html", TEMPLATE_HTML), &vars));
    }
    println!("✅ 已在当前目录初始化 {name}");
    println!("  lyco add webview-capi   # 一行添加 webview UI");
    println!("  lyco run");
}

fn cmd_doc() {
    let ok = Command::new("doxygen").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false);
    if !ok { eprintln!("doc 需要 doxygen (scoop install doxygen)"); std::process::exit(1); }
    if !Path::new("Doxyfile").exists() {
        let name = manifest::Manifest::load().map(|m| m.project_name()).unwrap_or_else(|_| "app".into());
        // 原来是 `.expect(...)` —— 写不进去就 panic，甩一屏 backtrace 给用户看。
        if let Err(e) = fs::write("Doxyfile", format!(
            "PROJECT_NAME = \"{name}\"\nINPUT = src\nRECURSIVE = YES\nGENERATE_HTML = YES\nGENERATE_LATEX = NO\nOUTPUT_DIRECTORY = docs/api\nQUIET = YES\n"
        )) {
            eprintln!("❌ 写 Doxyfile 失败: {e}");
            std::process::exit(1);
        }
        println!("📄 已生成默认 Doxyfile (可自行修改)");
    }
    // 原来是 `let _ = …status()` 再无条件报「✅ 文档输出」——
    // doxygen 出错（Doxyfile 写坏、源码有语法问题）也会被告知「已输出」。
    let mut c = Command::new("doxygen");
    c.arg("Doxyfile");
    if let Err(e) = run_step("doxygen", c) {
        eprintln!("❌ {e}");
        std::process::exit(1);
    }
    println!("✅ 文档输出: docs/api/html/index.html");
}

// ── 本地静态服务器（`lyco web`） ─────────────────────────────
// 页面是纯静态文件，用 Python 自带的 http.server 托管。
//
// 原来硬编码 `Command::new("python")` 且用 `let _` 吞掉失败 —— 两个后果：
//   1. 多数 Linux/macOS 上根本没有 `python`（只有 `python3`），
//      于是 `lyco web` 在这些平台上**永远起不来**；
//   2. 起不来也不报错，用户看到的却是「打开浏览器访问 http://localhost:8080」，
//      打开发现连不上。属于**报喜不报忧**。

/// 找一个真能用的 Python。顺序：`python3` → `python` → `py -3`（Windows 启动器）。
///
/// 探测用 `-c "print(1)"` 而不是 `--version`：Windows 上未安装 Python 时，
/// `python` 往往是个「打开微软商店」的假 exe，`--version` 分辨不出来，
/// 而它不会打印 `1`。
fn find_python() -> Option<(&'static str, &'static [&'static str])> {
    const CANDIDATES: &[(&str, &[&str])] = &[
        ("python3", &[]),
        ("python", &[]),
        ("py", &["-3"]),
    ];
    for (exe, pre) in CANDIDATES {
        let ok = Command::new(exe)
            .args(*pre)
            .args(["-c", "print(1)"])
            .output()
            .map(|o| o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "1")
            .unwrap_or(false);
        if ok {
            return Some((exe, pre));
        }
    }
    None
}

/// 等本地端口真的能连上（最多 `tries` × 100ms）。只连 `127.0.0.1` —— 服务就绑在本机。
///
/// 为什么要**探测端口**而不是看退出码：`python -m http.server` 在**端口被占用**时
/// 会立刻退出，而 `status()` 要等进程结束才返回 —— 调用方那时还没拿到任何信息，
/// 却已经把「打开浏览器访问 http://localhost:8080」打出去了。
fn wait_port(port: &str, tries: u32) -> bool {
    let addr = format!("127.0.0.1:{port}");
    for _ in 0..tries {
        // 用 `as_str()` 而不是 `&addr`：`&String` 虽然也实现了 `ToSocketAddrs`，
        // 但本机不编译，不给类型推导留任何需要细究的余地。
        if std::net::TcpStream::connect(addr.as_str()).is_ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    false
}

/// 起静态服务器托管 `~/.lyco/web/`，**先确认端口真的能连上**，再把句柄交出去。
///
/// 原来这里是 `serve_web(..) -> bool`，用 `status().is_ok()` 判成败 —— 那个判据是错的：
/// `status()` 返回 `io::Result<ExitStatus>`，`is_ok()` 只说明「进程能启动、并且跑完了」，
/// **完全没看退出码**。端口被占用时 http.server 立刻以 1 退出，`is_ok()` 照样为真，
/// 于是「服务已经死了」被报成「起来了」。
fn spawn_web_server(port: &str) -> Result<std::process::Child, String> {
    let py = find_python().ok_or_else(web_no_python_hint)?;
    let (exe, pre) = py;
    let mut a: Vec<&str> = pre.to_vec();
    a.push("-m");
    a.push("http.server");
    a.push(port);
    let mut child = Command::new(exe)
        .args(&a)
        .current_dir(web_dir())
        .spawn()
        .map_err(|e| format!("启动 {exe} 失败: {e}"))?;
    // 每 100ms 探一次，最多 5 秒。两个提前退出条件：
    //   * 端口通了 → 成功，把句柄交给调用方（由它决定 wait 还是 kill）；
    //   * 子进程**已经结束**（端口被占 / 端口非法）→ 立刻失败，不必干等满 5 秒。
    //     python 自己的报错（如 `OverflowError: port must be 0-65535`）没有被重定向，
    //     会直接打在终端上，用户看得到根因。
    for _ in 0..50 {
        if wait_port(port, 1) {
            return Ok(child);
        }
        if let Ok(Some(_)) = child.try_wait() {
            return Err(web_fail_hint(port));
        }
    }
    let _ = child.kill();
    Err(web_fail_hint(port))
}

/// 服务起不来时的**统一提示**。两条失败路径共用同一句话 ——
/// 这样断言只需要认这一句，不必分别匹配（也就不会漏掉其中一条）。
fn web_fail_hint(port: &str) -> String {
    format!(
        "本地服务器没能起来 (127.0.0.1:{port} 连不上) —— 端口可能被占用或非法, 换一个: PORT=8090 lyco"
    )
}

/// 找不到 Python 时的提示：说清「为什么」和「还能怎么办」，别只说失败。
fn web_no_python_hint() -> String {
    format!(
        "❌ 未找到 python3 / python，无法启动本地服务器。\n   \
         页面本身是纯静态文件，也可以用任意静态服务器托管这个目录:\n   {}",
        web_dir().display()
    )
}

/// 打开系统浏览器。**失败时说一句** —— 原来两处都是 `let _ = …spawn()`：静默失败，
/// 而屏幕上刚打过「打开浏览器访问: …」，用户会一直等一个不会来的窗口。
fn open_browser(url: &str) {
    #[cfg(windows)]
    {
        // 优先 Edge（Windows 10/11 自带）
        let edge = [
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ];
        for p in &edge {
            if Path::new(p).exists() && Command::new(p).arg(url).spawn().is_ok() {
                return;
            }
        }
        // 兜底: cmd /c start
        if Command::new("cmd").args(["/c", "start", "", url]).spawn().is_ok() {
            return;
        }
    }
    #[cfg(not(windows))]
    {
        if Command::new("xdg-open").arg(url).spawn().is_ok() {
            return;
        }
    }
    eprintln!("⚠  没能自动打开浏览器, 请手动访问 {url}");
}

fn cmd_web() {
    ensure_initialized();
    let port = env::var("PORT").unwrap_or_else(|_| "8080".into());
    // 顺序很重要：**先确认服务真的在监听，再打印地址**。
    // 原来是反的（先 `println!` 再起服务），判据还是 `status().is_ok()` ——
    // 端口被占用时用户会拿到一个连不上的地址，然后怀疑是自己的浏览器有问题。
    let mut child = match spawn_web_server(&port) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ {e}");
            std::process::exit(1);
        }
    };
    println!("🌐 http://localhost:{port}  (文件: {})", web_dir().display());
    println!("   按 Ctrl+C 退出");
    // 阻塞到服务退出（用户 Ctrl+C 时 http.server 收到 SIGINT 结束）。
    let _ = child.wait();
}

fn cmd_reset() {
    let d = data_dir();
    if !d.exists() {
        println!("✅ 已重置 ~/.lyco/");
        return;
    }
    // `backup/` 是**覆盖前留下的恢复副本** —— 可能是用户唯一能找回定制的地方。
    // `reset` 的语义是「把模板/配置恢复成默认」，不该顺手把安全网也剪了，
    // 而且它原来是 `let _ = remove_dir_all(...)`：删失败也不吭声。
    let bk = backup_root();
    let kept_backups = fs::read_dir(&bk)
        .map(|r| r.filter_map(|e| e.ok()).filter(|e| e.path().is_dir()).count())
        .unwrap_or(0);

    let mut failed = Vec::new();
    if let Ok(rd) = fs::read_dir(&d) {
        for e in rd.flatten() {
            let p = e.path();
            if p == bk {
                continue;
            }
            // 目录用 remove_dir_all，文件用 remove_file —— 两者都会在类型不匹配时失败，
            // 所以「两个都失败」才算真失败。
            if fs::remove_dir_all(&p).is_err() && fs::remove_file(&p).is_err() {
                failed.push(p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
            }
        }
    }
    if failed.is_empty() {
        println!("✅ 已重置 ~/.lyco/");
    } else {
        println!("⚠  已重置 ~/.lyco/, 但以下内容删不掉 (可能被占用): {}", failed.join(", "));
    }
    if kept_backups > 0 {
        println!(
            "   💾 {kept_backups} 份模板备份已保留在 {} (要一并删除请手动删)",
            bk.display()
        );
    }
}

fn cmd_info() {
    let d = data_dir();
    println!("📁 {}", d.display());
    if d.exists() {
        // 只数真正的模板。`.version` / `.manifest` 是内部文件，不是模板 ——
        // 它们混在计数里会让人以为模板数变了。
        let t = fs::read_dir(templates_dir())
            .map(|r| {
                r.filter_map(|e| e.ok())
                    .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
                    .count()
            })
            .unwrap_or(0);
        let c = fs::read_dir(commands_dir()).map(|r| r.count()).unwrap_or(0);
        let n = db::list_projects().len();
        println!("  模板: {t} 文件 | 外部命令: {c} 文件 | 已注册项目: {n} 个");
    }
    // 当前目录有清单时，把项目本身也报出来。
    // `Lyco.toml` 的 `version` 字段原来解析了却从来没被读过
    // （编译警告 `field \`version\` is never read`），这里让它真的派上用场。
    if Path::new(manifest::MANIFEST).exists() {
        match manifest::Manifest::load() {
            Ok(m) => {
                let ver = m.package.version.as_deref().unwrap_or("(未写)");
                println!(
                    "📦 当前项目: {} v{} | 依赖 {} 个",
                    m.project_name(),
                    ver,
                    m.dependencies.len()
                );
            }
            Err(e) => println!("⚠  {} 解析失败: {e}", manifest::MANIFEST),
        }
    }
}

fn cmd_list() {
    // 已注册项目（`lyco new` 登记、存在 ~/.lyco/lyco.db）。
    // 这是 `list_projects()` 的**唯一**调用点 —— 在此之前那个函数从没被
    // 调用过，于是 `lyco new` 一直在往一张没人读的表里写数据。
    let projects = db::list_projects();
    if !projects.is_empty() {
        println!("已注册项目 ({}):", projects.len());
        for (name, lang, path, created) in &projects {
            // created_at 是 SQLite 的 "YYYY-MM-DD HH:MM:SS"，只取日期那段
            let day = created.split([' ', 'T']).next().unwrap_or("");
            // 显示时也过一遍 tidy_path：老库里可能存着带 `\\?\` 前缀的路径
            println!("  {name:<16} {lang:<10} {day}  {}", tidy_path(Path::new(path)));
        }
        println!();
    }

    println!("内置命令: new init add remove build check run test doc search update install uninstall clean web reset restore info list bench publish");
    let dir = commands_dir();
    if let Ok(rd) = fs::read_dir(dir) {
        let ext = if cfg!(windows) { "dll" } else { "so" };
        for e in rd.filter_map(|e| e.ok()) {
            let p = e.path();
            if p.extension().map(|x| x == ext).unwrap_or(false) {
                let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                println!("  外部: {stem} ({})", p.display());
            }
        }
    }
}

fn print_help() {
    // 版本号从 Cargo.toml 取（`env!(CARGO_PKG_VERSION)`），别写死 ——
    // 写死的版本号一定会过期，而且同一个文件里 `ensure_initialized` 的
    // 模板版本戳本来就是这么取的，两处不一致迟早出事。
    print!(concat!("lyco v", env!("CARGO_PKG_VERSION"), r#" - cargo 风格的跨平台 WebView 项目管理器 (Lyco.toml + xmake)

用法: lyco <command> [args]        (b/c/r/t/d 为 build/check/run/test/doc 别名)

命令:
  new <name> <lang> [url]       新建项目 (含 Lyco.toml)
  init [name]                   在现有目录初始化清单
  add <dep>[@<ver>] [--git|--path]  添加依赖 (例: lyco add webview-capi@1.0)
  remove <dep>                  移除依赖 (保留注释与格式)
  build [-r] [--target <plat>]  构建 (-r = release; 默认 debug)
  check                         语法检查 (不产出目标文件)
  run [-r] [--target <plat>]    构建 + 运行
  test                          运行 tests/*.c (每个文件一个测试)
  doc                           生成文档 (需 doxygen)
  search [关键词]               搜索依赖注册表
  update                        更新包仓库 (xmake repo -u)
  install / uninstall [name]    构建并安装到 ~/.lyco/bin / 卸载
  clean                         清除构建产物
  web                           可视化 Web UI (纯静态预览页, 需 python3/python)
  reset                         重置 ~/.lyco/ (模板备份会保留, 见下)
  restore [名字]                列出模板备份 / 把某一份放回去 (lyco restore)
  info / list                   配置信息(含当前项目) / 已注册项目 + 命令列表

平台 (--target): windows / mingw / linux / macos / android / ios / wasm

Lyco.toml (与 Cargo.toml 同风格):
  [package]
  name = "hello"
  version = "0.1.0"

  [dependencies]
  webview-capi = "*"                      # WebView 窗口 (自动镜像源+系统库)
  webui = {{ version = "*" }}              # WebUI, 任意浏览器做前端
  mypkg = {{ git = "https://…/repo.git" }} # 从 git 包仓库取 (也可 path = "../repo")

语言: c, python, typescript, rust, go, java, zig, c#, e(易语言)
插件: 在 ~/.lyco/commands/ 放 <名>.{} (作为标记) + 同名的可执行文件 <名>{}
模板: 编辑 ~/.lyco/templates/ 定制 (升级时只覆盖你没改过的, 改动会被保留)
备份: 被覆盖掉的原内容存在 ~/.lyco/backup/<版本戳>/ (镜像 ~/.lyco/ 的结构,
      所以 cp -r 也能放回去); lyco restore 列出可用备份, lyco restore <版本戳> 恢复。
      只保留最近 10 份 (全部保留设 LYCO_BACKUP_KEEP=0)
"#),
        if cfg!(windows) { "dll" } else { "so" },
        if cfg!(windows) { ".exe" } else { "" }
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // 双击检测:无参数时启动 Web UI
    if args.len() < 2 {
        ensure_initialized();
        let port = env::var("PORT").unwrap_or_else(|_| "8080".into());
        let url = format!("http://localhost:{port}");

        // 起服务并**等它真的在监听**，然后才打印地址、才去开浏览器。
        // 原来是「spawn 一个后台线程 + 固定 sleep 500ms」—— 端口被占用时
        // http.server 立刻退出，500ms 后照样打印地址、照样开浏览器，
        // 用户对着一个连不上的页面发呆，还以为是自己浏览器的问题。
        let _child = match spawn_web_server(&port) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("❌ {e}");
                std::process::exit(1);
            }
        };

        println!("🌐 Lyco WebView Studio 启动中...");
        println!("   打开浏览器访问: {url}");
        println!("   按 Ctrl+C 退出");

        open_browser(&url);

        // 阻塞主线程保持运行（`_child` 一直被持有，服务随本进程一起退出）
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    let cmd = args[1].as_str();
    let cmd_args: &[String] = &args[2..];

    // 优先外部命令
    if let Some(p) = find_external_cmd(cmd) {
        // `find_external_cmd` 返回的是 `commands/<名字>.dll`(Windows) /
        // `<名字>.so`(其他平台) —— 那是**插件本体**，不是能执行的东西。
        // 真正要跑的是同名的可执行文件：Windows 上是 `<名字>.exe`，
        // 其他平台就是去掉扩展名后的 `<名字>`。
        //
        // 原来非 Windows 分支直接拿 `.so` 的路径去 `Command::new` ——
        // 共享库没有 PT_INTERP，execve 必然失败（ENOEXEC），
        // 也就是**插件在 Linux/macOS 上永远跑不起来**，而报错信息只说
        // 「缺少同名可执行文件」，完全指不到原因。
        let exe = if cfg!(windows) {
            p.with_extension("exe")
        } else {
            p.with_extension("")
        };
        if exe.exists() {
            // 原来是 `.unwrap()` —— 插件文件在、但没有执行权限（Linux/macOS 上
            // 把二进制复制过来忘了 `chmod +x` 是常事）会直接 panic，甩一屏
            // backtrace，而不是告诉用户「为什么跑不起来」。
            match Command::new(&exe).args(cmd_args).status() {
                Ok(s) => std::process::exit(s.code().unwrap_or(1)),
                Err(e) => {
                    eprintln!("❌ 无法执行插件 {}: {e}", exe.display());
                    eprintln!("   Linux/macOS 上先确认它有执行权限: chmod +x {}", exe.display());
                    std::process::exit(1);
                }
            }
        } else {
            // 报完就退出，别再落到下面的 match 里 —— 否则还会把 38 行帮助
            // 当成「未知命令」打出来，而这条命令其实是**已知的、只是装坏了**。
            eprintln!(
                "找到插件 {}, 但缺少配套的可执行文件 {}",
                p.display(),
                exe.display()
            );
            std::process::exit(1);
        }
    }

    match cmd {
        "new" => {
            if cmd_args.len() < 2 { eprintln!("用法: lyco new <name> <lang> [url]"); std::process::exit(1); }
            let url = cmd_args.get(2).map(|s| s.as_str()).unwrap_or("https://example.com");
            cmd_new(&cmd_args[0], url, &cmd_args[1]);
        }
        "build" | "b" => cmd_build(cmd_args),
        "run"   | "r" => cmd_run(cmd_args),
        "check" | "c" => if let Err(e) = manifest::check() { eprintln!("❌ {e}"); std::process::exit(1); },
        "test"  | "t" => if let Err(e) = manifest::test() { eprintln!("❌ {e}"); std::process::exit(1); },
        "doc"   | "d" => cmd_doc(),
        "init"  => cmd_init(cmd_args.first().map(|s| s.as_str())),
        "update" => {
            println!("⬆ 更新包仓库 (xmake repo -u) ...");
            // 原来失败时**什么都不打印**、退出码还是 0 —— 用户以为已经更新过了。
            let mut c = Command::new("xmake");
            c.args(["repo", "-u"]);
            if let Err(e) = run_step("xmake repo -u", c) {
                eprintln!("❌ {e}");
                std::process::exit(1);
            }
            println!("✅ 已更新");
        },
        "search" => manifest::search(cmd_args.first().map(|s| s.as_str()).unwrap_or("")),
        "install" => if let Err(e) = manifest::install() { eprintln!("❌ {e}"); std::process::exit(1); },
        "uninstall" => if let Err(e) = manifest::uninstall(cmd_args.first().map(|s| s.as_str())) { eprintln!("❌ {e}"); std::process::exit(1); },
        "bench" => {
            println!("ℹ cargo bench 无直接对应。建议: lyco build -r 后对产物压测;");
            println!("  或把基准程序放 tests/, 用 lyco test 运行。");
        }
        "publish" => {
            println!("ℹ 未实现 (roadmap): git tag v<Lyco.toml version> + gh release 上传构建产物");
        }
        "add" => {
            cmd_add(cmd_args);
        }
        "remove" => {
            if cmd_args.is_empty() { eprintln!("用法: lyco remove <dep>"); std::process::exit(1); }
            cmd_remove(&cmd_args[0]);
        }
        "clean" => cmd_clean(),
        "web"   => cmd_web(),
        "reset" => cmd_reset(),
        "restore" => cmd_restore(cmd_args),
        "info"  => cmd_info(),
        "list"  => cmd_list(),
        _ => { print_help(); std::process::exit(1); }
    }
}
