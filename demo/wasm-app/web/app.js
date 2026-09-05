// WASM WebView 应用主逻辑
const log = (msg) => {
    const el = document.getElementById('log');
    el.textContent += `[${new Date().toLocaleTimeString()}] ${msg}\n`;
    el.scrollTop = el.scrollHeight;
};

window.runTest = () => {
    log('开始 WASM 应用测试...');
    log(`浏览器: ${navigator.userAgent.substring(0, 50)}...`);
    log(`平台: ${navigator.platform}`);
    log(`语言: ${navigator.language}`);

    // 测试本地存储
    try {
        localStorage.setItem('lyco_test', 'ok');
        log('✅ 本地存储正常');
    } catch(e) {
        log('❌ 本地存储失败: ' + e.message);
    }

    // 测试 Canvas
    try {
        const c = document.createElement('canvas');
        const ctx = c.getContext('2d');
        ctx.fillStyle = '#22c55e';
        ctx.fillRect(0, 0, 10, 10);
        log('✅ Canvas 2D 正常');
    } catch(e) {
        log('❌ Canvas 失败');
    }

    log('测试完成 ✓');
};

window.clearLog = () => {
    document.getElementById('log').textContent = '';
};

// 初始化
log('WASM WebView 应用已加载');
