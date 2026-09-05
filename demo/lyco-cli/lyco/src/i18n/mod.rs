use std::collections::HashMap;

pub struct I18n {
    lang: String,
    messages: HashMap<String, HashMap<String, String>>,
}

impl I18n {
    pub fn new(lang: &str) -> Self {
        let mut messages = HashMap::new();
        
        let mut zh = HashMap::new();
        zh.insert("app_title".to_string(), "Lyco WebView 管理工具".to_string());
        zh.insert("new_project".to_string(), "新建项目".to_string());
        zh.insert("build".to_string(), "构建".to_string());
        zh.insert("run".to_string(), "运行".to_string());
        zh.insert("pack".to_string(), "打包".to_string());
        zh.insert("language".to_string(), "语言".to_string());
        zh.insert("project_name".to_string(), "项目名称".to_string());
        zh.insert("url".to_string(), "网址".to_string());
        zh.insert("ai_assist".to_string(), "AI 辅助".to_string());
        zh.insert("advanced".to_string(), "高级".to_string());
        zh.insert("beginner".to_string(), "小白".to_string());
        zh.insert("select_target".to_string(), "选择目标平台".to_string());
        zh.insert("windows".to_string(), "Windows".to_string());
        zh.insert("android".to_string(), "Android".to_string());
        zh.insert("macos".to_string(), "macOS".to_string());
        zh.insert("linux".to_string(), "Linux".to_string());
        zh.insert("wasm".to_string(), "WASM".to_string());
        zh.insert("settings".to_string(), "设置".to_string());
        zh.insert("github_auth".to_string(), "GitHub 认证".to_string());
        zh.insert("openai_key".to_string(), "OpenAI API Key".to_string());
        zh.insert("save".to_string(), "保存".to_string());
        zh.insert("cancel".to_string(), "取消".to_string());
        zh.insert("console".to_string(), "控制台".to_string());
        zh.insert("files".to_string(), "文件".to_string());
        zh.insert("preview".to_string(), "预览".to_string());
        zh.insert("new".to_string(), "新建".to_string());
        zh.insert("open".to_string(), "打开".to_string());
        zh.insert("delete".to_string(), "删除".to_string());
        zh.insert("build_success".to_string(), "构建成功!".to_string());
        zh.insert("build_failed".to_string(), "构建失败".to_string());
        zh.insert("running".to_string(), "运行中...".to_string());
        zh.insert("project_created".to_string(), "项目已创建!".to_string());
        messages.insert("zh".to_string(), zh);
        
        let mut en = HashMap::new();
        en.insert("app_title".to_string(), "Lyco WebView Manager".to_string());
        en.insert("new_project".to_string(), "New Project".to_string());
        en.insert("build".to_string(), "Build".to_string());
        en.insert("run".to_string(), "Run".to_string());
        en.insert("pack".to_string(), "Pack".to_string());
        en.insert("language".to_string(), "Language".to_string());
        en.insert("project_name".to_string(), "Project Name".to_string());
        en.insert("url".to_string(), "URL".to_string());
        en.insert("ai_assist".to_string(), "AI Assist".to_string());
        en.insert("advanced".to_string(), "Advanced".to_string());
        en.insert("beginner".to_string(), "Beginner".to_string());
        en.insert("select_target".to_string(), "Select Target".to_string());
        en.insert("windows".to_string(), "Windows".to_string());
        en.insert("android".to_string(), "Android".to_string());
        en.insert("macos".to_string(), "macOS".to_string());
        en.insert("linux".to_string(), "Linux".to_string());
        en.insert("wasm".to_string(), "WASM".to_string());
        en.insert("settings".to_string(), "Settings".to_string());
        en.insert("github_auth".to_string(), "GitHub Auth".to_string());
        en.insert("openai_key".to_string(), "OpenAI API Key".to_string());
        en.insert("save".to_string(), "Save".to_string());
        en.insert("cancel".to_string(), "Cancel".to_string());
        en.insert("console".to_string(), "Console".to_string());
        en.insert("files".to_string(), "Files".to_string());
        en.insert("preview".to_string(), "Preview".to_string());
        en.insert("new".to_string(), "New".to_string());
        en.insert("open".to_string(), "Open".to_string());
        en.insert("delete".to_string(), "Delete".to_string());
        en.insert("build_success".to_string(), "Build Success!".to_string());
        en.insert("build_failed".to_string(), "Build Failed".to_string());
        en.insert("running".to_string(), "Running...".to_string());
        en.insert("project_created".to_string(), "Project Created!".to_string());
        messages.insert("en".to_string(), en);
        
        Self { lang: lang.to_string(), messages }
    }
    
    pub fn t(&self, key: &str) -> &str {
        self.messages.get(&self.lang)
            .and_then(|m| m.get(key))
            .map(|s| s.as_str())
            .unwrap_or(key)
    }
    
    pub fn set_lang(&mut self, lang: &str) {
        self.lang = lang.to_string();
    }
}

pub fn get_supported_languages() -> Vec<(&'static str, &'static str)> {
    vec![("zh", "中文"), ("en", "English")]
}
