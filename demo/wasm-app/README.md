# demo/wasm-app

WASM 纯前端 WebView 应用 — 无需后端,浏览器即跑。

## 运行

```bash
# 方式 1: Python HTTP 服务器
python -m http.server 8080 --directory web/

# 方式 2: Node.js
npx serve web/

# 打开 http://localhost:8080
```

## 项目结构

```
wasm-app/
├── src/
│   └── main.js          # WASM 模块 JS 胶水
├── web/
│   ├── index.html       # 入口页面
│   ├── app.js           # 应用逻辑
│   └── style.css        # 样式
└── README.md
```

## 编译 WASM (可选)

如果需要 Rust → WASM:

```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
wasm-pack build --target web
```
