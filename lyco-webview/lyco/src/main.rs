use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

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

fn read_template(name: &str) -> String {
    let exe_path = std::env::current_exe().unwrap_or_default();
    let exe_dir = exe_path.parent().unwrap_or(Path::new("."));
    let paths = [
        exe_dir.join("templates").join(name),
        exe_dir.join("../templates").join(name),
        Path::new("templates").join(name).to_path_buf(),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("templates").join(name),
    ];
    for p in &paths {
        if p.exists() {
            return fs::read_to_string(p).unwrap_or_default();
        }
    }
    eprintln!("模板未找到: {}", name);
    std::process::exit(1);
}

fn cmd_new(name: &str, url: &str, lang: &str) {
    if Path::new(name).exists() {
        eprintln!("错误: 目录 '{}' 已存在", name);
        std::process::exit(1);
    }
    println!("🚀 创建项目: {} (语言: {})", name, lang);

    let mut vars = HashMap::new();
    vars.insert("NAME", name);
    vars.insert("URL", url);
    vars.insert("DEBUG", "false");

    match lang.to_lowercase().as_str() {
        "c" => {
            fs::create_dir_all(format!("{}/src", name)).ok();
            write_file(&format!("{}/src/main.c", name), &template_replace(&read_template("main.c"), &vars));
            write_file(&format!("{}/xmake.lua", name), &template_replace(&read_template("xmake.lua"), &vars));
        }
        "python" | "py" => {
            write_file(&format!("{}/main.py", name), &template_replace(&read_template("main.py"), &vars));
            write_file(&format!("{}/requirements.txt", name), "pywebview>=4.0\n");
        }
        "typescript" | "ts" => {
            write_file(&format!("{}/main.ts", name), &template_replace(&read_template("main.ts"), &vars));
            write_file(&format!("{}/package.json", name), &template_replace(&read_template("package.json"), &vars));
        }
        "rust" | "rs" => {
            fs::create_dir_all(format!("{}/src", name)).ok();
            write_file(&format!("{}/src/main.rs", name), &template_replace(&read_template("main.rs"), &vars));
            write_file(&format!("{}/Cargo.toml", name), &format!(
                r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
webview = "0.1"
"#, name));
        }
        "go" => {
            write_file(&format!("{}/main.go", name), &template_replace(&read_template("main.go"), &vars));
            write_file(&format!("{}/go.mod", name), &format!("module {}\n\ngo 1.21\n", name));
        }
        "java" => {
            fs::create_dir_all(format!("{}/src", name)).ok();
            write_file(&format!("{}/src/Main.java", name), &template_replace(&read_template("Main.java"), &vars));
        }
        "zig" => {
            write_file(&format!("{}/main.zig", name), &template_replace(&read_template("main.zig"), &vars));
            write_file(&format!("{}/build.zig", name), "const std = @import(\"zig\");\n");
        }
        "csharp" | "cs" | "c#" => {
            write_file(&format!("{}/Program.cs", name), &template_replace(&read_template("Program.cs"), &vars));
            write_file(&format!("{}/{}", name, name), &format!("<Project Sdk=\"Microsoft.NET.Sdk\">\n<PropertyGroup><OutputType>WinExe></OutputType><TargetFramework>net8.0</TargetFramework></PropertyGroup>\n</Project>"));
        }
        "e" | "el" | "易语言" => {
            write_file(&format!("{}/main.e", name), &template_replace(&read_template("main.e.txt"), &vars));
        }
        _ => {
            eprintln!("不支持的语言: {}", lang);
            eprintln!("  支持: c, python, typescript, rust, go, java, zig, csharp, e(易语言)");
            std::process::exit(1);
        }
    }

    // 通用文件
    write_file(&format!("{}/index.html", name), &template_replace(&read_template("index.html"), &vars));
    write_file(&format!("{}/README.md", name), &template_replace(&read_template("README.md"), &vars));

    println!("✅ 项目 '{}' 已创建 (语言: {})", name, lang);
    println!("  进入: cd {}", name);
    match lang.to_lowercase().as_str() {
        "c" => { println!("  构建: xmake / cmake"); println!("  运行: xmake run"); }
        "python" | "py" => { println!("  安装: pip install -r requirements.txt"); println!("  运行: python main.py"); }
        "typescript" | "ts" => { println!("  安装: npm install"); println!("  运行: npm run dev"); }
        "rust" | "rs" => { println!("  构建: cd {} && cargo build --release", name); println!("  运行: cargo run"); }
        "go" => { println!("  构建: go build"); println!("  运行: go run main.go"); }
        "java" => { println!("  运行: javac src/Main.java && java -cp src Main"); }
        "zig" => { println!("  构建: zig build"); }
        "csharp" | "cs" | "c#" => { println!("  构建: dotnet build"); println!("  运行: dotnet run"); }
        "e" | "el" | "易语言" => { println!("  打开: 使用易语言 IDE 打开 main.e"); }
        _ => {}
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("lyco - 一键生成跨平台 WebView 项目 (多语言支持)");
        println!();
        println!("用法:");
        println!("  lyco new <name> <lang> [url]");
        println!();
        println!("支持语言:");
        println!("  c          - C + WebView2 (Windows/macOS/Linux)");
        println!("  python     - Python + pywebview");
        println!("  typescript - TypeScript + webview");
        println!("  rust       - Rust + webview crate");
        println!("  go         - Go + webview");
        println!("  java       - Java + JavaFX WebView");
        println!("  zig        - Zig + webview.h");
        println!("  csharp     - C# + WebView2 (.NET)");
        println!("  e          - 易语言 + WebView2");
        println!();
        println!("示例:");
        println!("  lyco new my-app python");
        println!("  lyco new my-app c http://192.168.10.165:8765");
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
        _ => eprintln!("未知命令: {}", args[1]),
    }
}
