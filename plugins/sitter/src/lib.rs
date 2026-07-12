use std::sync::OnceLock;

use egui::Color32;
use tree_sitter_highlight::Highlighter;
use tree_sitter_highlight::HighlightConfiguration;
use tree_sitter_highlight::HighlightEvent;

// Import newly added language packages
use tree_sitter_php;
use tree_sitter_ruby;
use tree_sitter_scala;
use tree_sitter_html;
//use tree_sitter_css;
use tree_sitter_toml;
use tree_sitter_ocaml;

pub const DARK_TEXT_COLOR: Color32 = Color32::from_rgb(192,192,192);
pub const LIGHT_TEXT_COLOR: Color32 = Color32::from_rgb(0,0,0);

#[derive(Debug)]
pub struct MyErr {
}

impl From<tree_sitter::QueryError> for MyErr {
    fn from(_f: tree_sitter::QueryError) -> MyErr {
        MyErr {}
    }
}

impl From<tree_sitter_highlight::Error> for MyErr {
    fn from(_f: tree_sitter_highlight::Error) -> MyErr {
        MyErr {}
    }
}

type SitResult<T> = Result<T, MyErr>;

#[derive(Clone, Debug)]
pub struct LightSlice<'a>{
    pub type_id: Option<usize>,
    pub slice: &'a [u8],
    pub dark_color: Color32,
    pub light_color: Color32,
}


/// 各语言 `HighlightConfiguration::configure` 使用的捕获名列表，须与该语法包内 `HIGHLIGHT_QUERY`
/// 中出现的 `@…` 捕获一致（顺序决定索引，须与着色时解析所用切片相同）。
mod highlight_names {
    pub const JAVASCRIPT: &[&str] = &[
        "comment",
        "constant",
        "constant.builtin",
        "constructor",
        "embedded",
        "function",
        "function.builtin",
        "function.method",
        "keyword",
        "number",
        "operator",
        "property",
        "punctuation.bracket",
        "punctuation.delimiter",
        "punctuation.special",
        "string",
        "string.special",
        "variable",
        "variable.builtin",
    ];

    pub const C: &[&str] = &[
        "comment",
        "constant",
        "delimiter",
        "function",
        "function.special",
        "keyword",
        "label",
        "number",
        "operator",
        "property",
        "string",
        "type",
        "variable",
    ];

    pub const RUST: &[&str] = &[
        "attribute",
        "comment",
        "constant",
        "constructor",
        "escape",
        "function",
        "function.macro",
        "function.method",
        "keyword",
        "label",
        "operator",
        "property",
        "punctuation.bracket",
        "punctuation.delimiter",
        "string",
        "type",
        "type.builtin",
        "variable",
        "variable.builtin",
        "variable.parameter",
    ];

    pub const GO: &[&str] = &[
        "comment",
        "constant.builtin",
        "escape",
        "function",
        "function.builtin",
        "function.method",
        "keyword",
        "number",
        "operator",
        "property",
        "string",
        "type",
        "variable",
    ];

    pub const BASH: &[&str] = &[
        "comment",
        "constant",
        "embedded",
        "function",
        "keyword",
        "number",
        "operator",
        "property",
        "string",
    ];

    pub const JSON: &[&str] = &[
        "comment",
        "constant.builtin",
        "escape",
        "number",
        "string",
        "string.special.key",
    ];

    pub const PYTHON: &[&str] = &[
        "comment",
        "constant",
        "constant.builtin",
        "constructor",
        "embedded",
        "escape",
        "function",
        "function.builtin",
        "function.method",
        "keyword",
        "number",
        "operator",
        "property",
        "punctuation.special",
        "string",
        "type",
        "variable",
    ];

    pub const JAVA: &[&str] = &[
        "attribute",
        "comment",
        "constant",
        "constant.builtin",
        "function.builtin",
        "function.method",
        "keyword",
        "number",
        "operator",
        "string",
        "string.escape",
        "type",
        "type.builtin",
        "variable",
        "variable.builtin",
    ];

    pub const CPP: &[&str] = &[
        "constant",
        "function",
        "keyword",
        "string",
        "type",
        "variable.builtin",
    ];

    pub const PHP: &[&str] = &[
        "comment",
        "constant",
        "constant.builtin",
        "constructor",
        "function",
        "function.builtin",
        "function.method",
        "keyword",
        "number",
        "operator",
        "property",
        "string",
        "tag",
        "type",
        "type.builtin",
        "variable",
        "variable.builtin",
    ];

    pub const RUBY: &[&str] = &[
        "comment",
        "constant",
        "constant.builtin",
        "constructor",
        "embedded",
        "escape",
        "function.method",
        "function.method.builtin",
        "keyword",
        "number",
        "operator",
        "property",
        "punctuation.bracket",
        "punctuation.delimiter",
        "punctuation.special",
        "string",
        "string.special.regex",
        "string.special.symbol",
        "variable",
        "variable.builtin",
        "variable.parameter",
    ];

    pub const SCALA: &[&str] = &[
        "attribute",
        "boolean",
        "comment",
        "conditional",
        "constant.builtin",
        "constructor",
        "exception",
        "float",
        "function",
        "function.builtin",
        "function.call",
        "include",
        "keyword",
        "keyword.function",
        "keyword.operator",
        "keyword.return",
        "method",
        "method.call",
        "namespace",
        "none",
        "number",
        "operator",
        "parameter",
        "property",
        "punctuation.bracket",
        "punctuation.delimiter",
        "punctuation.special",
        "repeat",
        "spell",
        "storageclass",
        "string",
        "type",
        "type.definition",
        "type.qualifier",
        "variable",
        "variable.builtin",
    ];

    pub const TYPESCRIPT: &[&str] = &[
        "keyword",
        "punctuation.bracket",
        "type",
        "type.builtin",
        "variable.parameter",
    ];

    pub const HTML: &[&str] = &[
        "attribute",
        "comment",
        "constant",
        "punctuation.bracket",
        "string",
        "tag",
        "tag.error",
    ];

    pub const TOML: &[&str] = &[
        "comment",
        "constant.builtin",
        "number",
        "operator",
        "property",
        "punctuation.bracket",
        "punctuation.delimiter",
        "string",
        "string.special",
    ];

    pub const OCAML: &[&str] = &[
        "comment",
        "constant",
        "constructor",
        "escape",
        "function",
        "function.builtin",
        "function.method",
        "keyword",
        "module",
        "number",
        "operator",
        "property",
        "punctuation.bracket",
        "punctuation.delimiter",
        "punctuation.special",
        "string",
        "string.special",
        "tag",
        "type",
        "type.builtin",
        "variable",
        "variable.parameter",
    ];
}

fn lang_key(lang: &str) -> &'static str {
    match lang.to_lowercase().as_str() {
        "c" => "c",
        "javascript" | "js" => "javascript",
        "rust" => "rust",
        "go" => "go",
        "bash" | "sh" => "bash",
        "json" => "json",
        "python" | "py" => "python",
        "java" => "java",
        "cpp" | "c++" | "cc" | "cxx" => "cpp",
        "php" => "php",
        "ruby" | "rb" => "ruby",
        "scala" => "scala",
        "typescript" | "ts" => "typescript",
        "html" | "htm" => "html",
        "toml" => "toml",
        "ocaml" | "ml" => "ocaml",
        "math" => "math",
        _ => "c",
    }
}

fn highlight_names_for_lang_key(key: &str) -> &'static [&'static str] {
    use highlight_names::*;
    match key {
        "javascript" => JAVASCRIPT,
        "rust" => RUST,
        "go" => GO,
        "bash" => BASH,
        "json" => JSON,
        "python" => PYTHON,
        "java" => JAVA,
        "cpp" => CPP,
        "php" => PHP,
        "ruby" => RUBY,
        "scala" => SCALA,
        "typescript" => TYPESCRIPT,
        "html" => HTML,
        "toml" => TOML,
        "ocaml" => OCAML,
        "math" | "c" => C,
        _ => C,
    }
}

/// tree-sitter 捕获名 → 调色板语义键（keyword / operator / delimiter / …）。
/// 先精确匹配常用名；未命中则走 [`capture_to_semantic_fallback`]，按前缀（如 `markup.`、`keyword.`）归类。
fn capture_to_semantic(name: &str) -> &'static str {
    match name {
        // --- 关键字 / 控制流（Helix / nvim-treesitter / Emacs 常见名）---
        "keyword"
        | "conditional"
        | "repeat"
        | "exception"
        | "include"
        | "keyword.function"
        | "keyword.return"
        | "keyword.import"
        | "keyword.export"
        | "keyword.directive"
        | "keyword.coroutine"
        | "keyword.storage"
        | "keyword.repeat"
        | "keyword.conditional"
        | "keyword.exception"
        | "debug"
        | "dedent"
        | "indent"
        | "fold"
        | "title"
        | "environment"
        | "environment.name" => "keyword",

        "keyword.operator" | "operator" | "punctuation.special" => "operator",

        "punctuation.delimiter" | "delimiter" | "punctuation.bracket" => "delimiter",

        // --- 字符串 / 字符 / 嵌入 / 正则 ---
        "string"
        | "string.special"
        | "string.special.key"
        | "string.special.symbol"
        | "string.special.regex"
        | "string.documentation"
        | "string.escape"
        | "string.regexp"
        | "string.regex"
        | "embedded"
        | "embedded.template"
        | "escape"
        | "character"
        | "character.special"
        | "regex"
        | "regexp"
        | "symbol"
        | "quote"
        | "text.literal"
        | "text.uri"
        | "text.emphasis"
        | "text.strike"
        | "text.underline"
        | "text.title"
        | "text.note"
        | "text.warning"
        | "text.danger"
        | "text.todo"
        | "markup.raw"
        | "markup.raw.block"
        | "markup.raw.inline"
        | "markup.quote"
        | "markup.math"
        | "markup.link"
        | "markup.link.url"
        | "markup.link.label"
        | "markup.link.text"
        | "markup.list"
        | "markup.list.checked"
        | "markup.list.unchecked" => "string",

        "constant"
        | "constant.macro"
        | "boolean"
        | "tag.error"
        | "diff.plus"
        | "diff.minus"
        | "diff.delta"
        | "tag.attributename"
        | "attribute.name" => "constant",

        "number" | "float" | "integer" => "number",

        // --- 标准库 / 内建（VS Code：support.* / defaultLibrary，Dark+ 常用 #DCDCAA）---
        "function.builtin"
        | "function.method.builtin"
        | "method.builtin"
        | "constant.builtin"
        | "type.builtin"
        | "variable.builtin"
        | "tag.builtin" => "builtin",

        // --- 函数 / 方法 / 调用 / 标签（HTML/XML）---
        "function"
        | "function.method"
        | "function.macro"
        | "function.call"
        | "function.special"
        | "method"
        | "method.call"
        | "call"
        | "tag"
        | "macro"
        | "markup.heading"
        | "markup.heading.1"
        | "markup.heading.2"
        | "markup.heading.3"
        | "markup.heading.4"
        | "markup.heading.5"
        | "markup.heading.6"
        | "markup.bold"
        | "markup.italic"
        | "markup.underline"
        | "markup.strikethrough"
        | "emphasis"
        | "strong"
        | "underline"
        | "strikethrough" => "function",

        "tag.delimiter" => "delimiter",

        "property"
        | "attribute"
        | "annotation"
        | "storageclass"
        | "field"
        | "member"
        | "variable.member"
        | "property.readonly"
        | "property.writable"
        | "tag.attribute"
        | "tag.attribute.value"
        | "attribute.value" => "property",

        "parameter" | "variable.parameter" | "parameter.reference" => "label",

        "type"
        | "type.definition"
        | "type.qualifier"
        | "namespace"
        | "module"
        | "predefined_type"
        | "class"
        | "struct"
        | "enum"
        | "interface"
        | "implementation"
        | "decorator"
        | "namespace.import"
        | "module.import"
        | "constructor" => "type",

        "label" => "label",

        "variable" | "none" | "identifier" | "text" | "text.reference" => "variable",

        "comment"
        | "spell"
        | "comment.documentation"
        | "comment.error"
        | "comment.warning"
        | "comment.note"
        | "comment.todo"
        | "preproc"
        | "define"
        | "preproc.include"
        | "preproc.define" => "comment",

        _ => capture_to_semantic_fallback(name),
    }
}

/// 未在精确表中列出的捕获名：按 `xxx.yyy` 前缀与常见别名归类。
fn capture_to_semantic_fallback(name: &str) -> &'static str {
    if let Some(tail) = name.strip_prefix("keyword.") {
        return if tail == "operator" {
            "operator"
        } else {
            "keyword"
        };
    }
    if name.starts_with("markup.") {
        return markup_capture_semantic(name);
    }
    if name.starts_with("string.") {
        return "string";
    }
    if name.starts_with("constant.") {
        return if name.ends_with(".builtin") {
            "builtin"
        } else {
            "constant"
        };
    }
    if name.starts_with("function.") {
        return if name.ends_with(".builtin") || name.contains(".method.builtin") {
            "builtin"
        } else {
            "function"
        };
    }
    if name.starts_with("method.") {
        return if name.ends_with(".builtin") {
            "builtin"
        } else {
            "function"
        };
    }
    if name.starts_with("type.") {
        return if name.ends_with(".builtin") {
            "builtin"
        } else {
            "type"
        };
    }
    if name.starts_with("variable.") {
        return match name {
            "variable.member" | "variable.other.member" => "property",
            "variable.parameter" => "label",
            "variable.builtin" => "builtin",
            _ => "variable",
        };
    }
    if name.starts_with("punctuation.") {
        return if name.ends_with("special") {
            "operator"
        } else {
            "delimiter"
        };
    }
    if name.starts_with("comment.") {
        return "comment";
    }
    if name.starts_with("text.") {
        return match name {
            "text.uri" | "text.literal" => "string",
            "text.reference" => "type",
            "text.documentation" => "comment",
            _ => "variable",
        };
    }
    if name.starts_with("tag.") {
        return match name {
            "tag.attribute" | "tag.attributename" => "property",
            "tag.delimiter" => "delimiter",
            "tag.error" => "constant",
            _ => "function",
        };
    }
    if name.starts_with("diff.") {
        return "constant";
    }
    if name.starts_with("definition.") {
        let rest = name.trim_start_matches("definition.");
        let head = rest.split('.').next().unwrap_or("");
        return match head {
            "function" | "method" => "function",
            "type" | "class" | "interface" => "type",
            "var" | "variable" | "parameter" => "label",
            _ => "variable",
        };
    }
    if name.starts_with("module.") || name == "module" {
        return "type";
    }
    if name.starts_with("namespace.") {
        return "type";
    }
    if name.starts_with("character") {
        return "string";
    }
    if matches!(
        name,
        "regex" | "regexp" | "symbol" | "character" | "quote"
    ) {
        return "string";
    }
    "variable"
}

/// `markup.*` 变体（标题、链接、列表等）映射到既有语义色。
fn markup_capture_semantic(name: &str) -> &'static str {
    if name.contains("heading") {
        return "function";
    }
    if name.contains("link") || name.contains("quote") || name.contains("raw") {
        return "string";
    }
    if name.contains("list") {
        return "keyword";
    }
    if name.contains("bold")
        || name.contains("italic")
        || name.contains("underline")
        || name.contains("strike")
        || name.contains("emphasis")
    {
        return "property";
    }
    if name.contains("math") {
        return "number";
    }
    "string"
}

/// 按语义名称返回（浅色主题色, 暗色主题色）。
fn palette_by_semantic(semantic: &str) -> (Color32, Color32) {
    match semantic {
        // keyword — light_vs `keyword` rgb(0, 0, 255)；dark_vs `keyword` rgb(86, 156, 214)（先于 Dark+ 对 `keyword.control` 的覆盖）
        "keyword" => (
            Color32::from_rgb(0, 0, 255),
            Color32::from_rgb(86, 156, 214),
        ),
        // builtin / function — light_plus `entity.name.function` rgb(121, 81, 1)；dark_plus rgb(220, 220, 170)
        "builtin" | "function" => (
            Color32::from_rgb(121, 81, 1),
            Color32::from_rgb(220, 220, 170),
        ),
        // operator — light_vs `keyword.operator` rgb(0, 0, 0)；dark_vs rgb(212, 212, 212)
        "operator" => (
            Color32::from_rgb(0, 0, 0),
            Color32::from_rgb(212, 212, 212),
        ),
        // delimiter — 浅色柔化 rgb(62, 0, 90) 暗色 dark_vs `punctuation.definition.tag` rgb(128, 128, 128)
        "delimiter" => (
            Color32::from_rgb(62, 0, 90),
            Color32::from_rgb(128, 128, 128),
        ),
        // string — light_vs `string` rgb(163, 21, 21)；dark_vs `string` rgb(206, 145, 120)
        "string" => (
            Color32::from_rgb(163, 21, 21),
            Color32::from_rgb(206, 145, 120),
        ),
        // constant — light_plus `variable.other.constant` rgb(0, 112, 193)；dark_plus rgb(79, 193, 255)
        "constant" => (
            Color32::from_rgb(0, 112, 193),
            Color32::from_rgb(79, 193, 255),
        ),
        // number — light_vs `constant.numeric` rgb(9, 134, 88)；dark_vs rgb(181, 206, 168)
        "number" => (
            Color32::from_rgb(9, 134, 88),
            Color32::from_rgb(181, 206, 168),
        ),
        // property — light_plus `meta.object-literal.key` rgb(0, 16, 128)；dark_vs `entity.other.attribute-name` rgb(156, 220, 254)
        "property" => (
            Color32::from_rgb(0, 16, 128),
            Color32::from_rgb(156, 220, 254),
        ),
        // label — light_plus `entity.name.label` #000000；dark_plus #C8C8C8
        "label" => (
            Color32::from_rgb(0, 0, 0),
            Color32::from_rgb(200, 200, 200),
        ),
        // type — light_plus「Types declaration」rgb(0, 119, 155)；dark_plus rgb(78, 201, 176)
        "type" => (
            Color32::from_rgb(0, 119, 155),
            Color32::from_rgb(78, 201, 176),
        ),
        // variable — light_plus `variable` #001080；dark_plus #9CDCFE
        "variable" => (
            Color32::from_rgb(0, 16, 128),
            Color32::from_rgb(156, 220, 254),
        ),
        // comment — 浅色柔化 rgb(70, 107, 53); 暗色 dark_vs rgb(106, 153, 85)
        "comment" => (
            Color32::from_rgb(70, 107, 53),
            Color32::from_rgb(106, 153, 85),
        ),
        // 未归类 — `editor.foreground`：light rgb(0, 0, 0)；dark rgb(212, 212, 212)
        _ => (
            Color32::from_rgb(0, 0, 0),
            Color32::from_rgb(212, 212, 212),
        ),
    }
}

fn colors_for_highlight_name(name: &str) -> (Color32, Color32) {
    palette_by_semantic(capture_to_semantic(name))
}

fn language_js_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_javascript::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_javascript::HIGHLIGHT_QUERY, 
        tree_sitter_javascript::INJECTION_QUERY, 
        tree_sitter_javascript::LOCALS_QUERY)?;

    config.configure(highlight_names::JAVASCRIPT);
    Ok(config)
}

fn language_c_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_c::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_c::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::C);
    Ok(config)
}

fn language_rust_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_rust::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_rust::HIGHLIGHT_QUERY,
        tree_sitter_rust::INJECTIONS_QUERY, 
        "" )?;

    config.configure(highlight_names::RUST);
    Ok(config)
}

fn language_go_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_go::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_go::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::GO);
    Ok(config)
}

fn language_bash_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_bash::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_bash::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::BASH);
    Ok(config)
}

fn language_json_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_json::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_json::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::JSON);
    Ok(config)
}

fn language_python_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_python::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_python::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::PYTHON);
    Ok(config)
}

fn language_java_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_java::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_java::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::JAVA);
    Ok(config)
}

fn language_cpp_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_cpp::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_cpp::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::CPP);
    Ok(config)
}


fn language_php_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_php::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_php::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::PHP);
    Ok(config)
}
// 
fn language_ruby_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_ruby::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_ruby::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::RUBY);
    Ok(config)
}
// 
fn language_scala_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_scala::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_scala::HIGHLIGHTS_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::SCALA);
    Ok(config)
}
// 
// fn language_kotlin_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_kotlin::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_kotlin::HIGHLIGHTS_QUERY,
//         "", 
//         "" )?;
// 
//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }
// 
// fn language_swift_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_swift::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_swift::HIGHLIGHT_QUERY,
//         "", 
//         "" )?;
// 
//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }

fn language_typescript_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_typescript::language_typescript();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_typescript::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::TYPESCRIPT);
    Ok(config)
}

fn language_html_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_html::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_html::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::HTML);
    Ok(config)
}

// fn language_css_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_css::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_css::HIGHLIGHTS_QUERY,
//         "", 
//         "" )?;

//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }

// fn language_yaml_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_yaml::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_yaml::HIGHLIGHT_QUERY,
//         "", 
//         "" )?;

//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }

fn language_toml_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_toml::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_toml::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::TOML);
    Ok(config)
}

// fn language_markdown_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_md::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_md::HIGHLIGHTS_QUERY,
//         "", 
//         "" )?;

//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }

// fn language_lua_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_lua::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_lua::HIGHLIGHT_QUERY,
//         "", 
//         "" )?;
// 
//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }
// 
// fn language_dart_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_dart::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_dart::HIGHLIGHT_QUERY,
//         "", 
//         "" )?;
// 
//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }
// 
// fn language_elixir_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_elixir::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_elixir::HIGHLIGHT_QUERY,
//         "", 
//         "" )?;
// 
//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }

// fn language_clojure_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_clojure::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_clojure::HIGHLIGHT_QUERY,
//         "", 
//         "" )?;
// 
//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }
// 
// fn language_haskell_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_haskell::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_haskell::HIGHLIGHT_QUERY,
//         "", 
//         "" )?;
// 
//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }

fn language_ocaml_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_ocaml::language_ocaml();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_ocaml::HIGHLIGHTS_QUERY,
        "", 
        "" )?;

    config.configure(highlight_names::OCAML);
    Ok(config)
}

// fn language_fsharp_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_fsharp::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_fsharp::HIGHLIGHT_QUERY,
//         "", 
//         "" )?;
// 
//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }
// 
// fn language_nim_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_nim::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_nim::HIGHLIGHT_QUERY,
//         "", 
//         "" )?;
// 
//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }
// 
// fn language_zig_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_zig::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_zig::HIGHLIGHT_QUERY,
//         "", 
//         "" )?;
// 
//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }
// 
// fn language_v_config() -> SitResult<HighlightConfiguration> {
//     let language = tree_sitter_v::language();
//     let mut config: HighlightConfiguration = HighlightConfiguration::new(
//         language, 
//         tree_sitter_v::HIGHLIGHT_QUERY,
//         "", 
//         "" )?;
// 
//     config.configure(&HIGHLIGH_NAMES);
//     Ok(config)
// }

static CONFIG_JS: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_C: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_RUST: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_GO: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_BASH: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_JSON: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_PYTHON: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_JAVA: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_CPP: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_PHP: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_RUBY: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_SCALA: OnceLock<HighlightConfiguration> = OnceLock::new();
// static CONFIG_KOTLIN: OnceLock<HighlightConfiguration> = OnceLock::new();  // Version conflict, temporarily unavailable
// static CONFIG_SWIFT: OnceLock<HighlightConfiguration> = OnceLock::new();   // Temporarily unavailable
static CONFIG_TYPESCRIPT: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_HTML: OnceLock<HighlightConfiguration> = OnceLock::new();
//static CONFIG_CSS: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_TOML: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_OCAML: OnceLock<HighlightConfiguration> = OnceLock::new();
// static CONFIG_YAML: OnceLock<HighlightConfiguration> = OnceLock::new();    // Temporarily unavailable
// static CONFIG_TOML: OnceLock<HighlightConfiguration> = OnceLock::new();    // Temporarily unavailable
// static CONFIG_MARKDOWN: OnceLock<HighlightConfiguration> = OnceLock::new(); // Temporarily unavailable
// static CONFIG_LUA: OnceLock<HighlightConfiguration> = OnceLock::new();     // Temporarily unavailable
// static CONFIG_DART: OnceLock<HighlightConfiguration> = OnceLock::new();    // Temporarily unavailable
// static CONFIG_ELIXIR: OnceLock<HighlightConfiguration> = OnceLock::new();  // Temporarily unavailable
// static CONFIG_CLOJURE: OnceLock<HighlightConfiguration> = OnceLock::new();  // Temporarily unavailable
// static CONFIG_HASKELL: OnceLock<HighlightConfiguration> = OnceLock::new();  // Temporarily unavailable
// static CONFIG_FSHARP: OnceLock<HighlightConfiguration> = OnceLock::new();   // Temporarily unavailable
// static CONFIG_NIM: OnceLock<HighlightConfiguration> = OnceLock::new();      // Temporarily unavailable
// static CONFIG_ZIG: OnceLock<HighlightConfiguration> = OnceLock::new();      // Temporarily unavailable
// static CONFIG_V: OnceLock<HighlightConfiguration> = OnceLock::new();        // Temporarily unavailable

fn lang_configure(lang: &str) -> &'static HighlightConfiguration {
    match lang_key(lang) {
        "c" => CONFIG_C.get_or_init(|| language_c_config().unwrap()),
        "javascript" => CONFIG_JS.get_or_init(|| language_js_config().unwrap()),
        "rust" => CONFIG_RUST.get_or_init(|| language_rust_config().unwrap()),
        "go" => CONFIG_GO.get_or_init(|| language_go_config().unwrap()),
        "bash" => CONFIG_BASH.get_or_init(|| language_bash_config().unwrap()),
        "json" => CONFIG_JSON.get_or_init(|| language_json_config().unwrap()),
        "python" => CONFIG_PYTHON.get_or_init(|| language_python_config().unwrap()),
        "java" => CONFIG_JAVA.get_or_init(|| language_java_config().unwrap()),
        "cpp" => CONFIG_CPP.get_or_init(|| language_cpp_config().unwrap()),
        "php" => CONFIG_PHP.get_or_init(|| language_php_config().unwrap()),
        "ruby" => CONFIG_RUBY.get_or_init(|| language_ruby_config().unwrap()),
        "scala" => CONFIG_SCALA.get_or_init(|| language_scala_config().unwrap()),
        // "kotlin" | "kt" => CONFIG_KOTLIN.get_or_init(||{language_kotlin_config().unwrap()}),  // Version conflict, temporarily unavailable
        // "swift" => CONFIG_SWIFT.get_or_init(||{language_swift_config().unwrap()}),            // Temporarily unavailable
        "typescript" => CONFIG_TYPESCRIPT.get_or_init(|| language_typescript_config().unwrap()),
        "html" => CONFIG_HTML.get_or_init(|| language_html_config().unwrap()),
        //"css" => CONFIG_CSS.get_or_init(||{language_css_config().unwrap()}),
        "toml" => CONFIG_TOML.get_or_init(|| language_toml_config().unwrap()),
        "ocaml" => CONFIG_OCAML.get_or_init(|| language_ocaml_config().unwrap()),
        "math" => CONFIG_C.get_or_init(|| language_c_config().unwrap()),
        // "yaml" | "yml" => CONFIG_YAML.get_or_init(||{language_yaml_config().unwrap()}),      // Temporarily unavailable
        // "toml" => CONFIG_TOML.get_or_init(||{language_toml_config().unwrap()}),              // Temporarily unavailable
        // "markdown" | "md" => CONFIG_MARKDOWN.get_or_init(||{language_markdown_config().unwrap()}), // Temporarily unavailable
        // "lua" => CONFIG_LUA.get_or_init(||{language_lua_config().unwrap()}),                 // Temporarily unavailable
        // "dart" => CONFIG_DART.get_or_init(||{language_dart_config().unwrap()}),              // Temporarily unavailable
        // "elixir" | "ex" => CONFIG_ELIXIR.get_or_init(||{language_elixir_config().unwrap()}), // Temporarily unavailable
        // "clojure" | "clj" => CONFIG_CLOJURE.get_or_init(||{language_clojure_config().unwrap()}),  // Temporarily unavailable
        // "haskell" | "hs" => CONFIG_HASKELL.get_or_init(||{language_haskell_config().unwrap()}),  // Temporarily unavailable
        // "fsharp" | "fs" => CONFIG_FSHARP.get_or_init(||{language_fsharp_config().unwrap()}),     // Temporarily unavailable
        // "nim" => CONFIG_NIM.get_or_init(||{language_nim_config().unwrap()}),                    // Temporarily unavailable
        // "zig" => CONFIG_ZIG.get_or_init(||{language_zig_config().unwrap()}),                    // Temporarily unavailable
        // "v" => CONFIG_V.get_or_init(||{language_v_config().unwrap()}),                          // Temporarily unavailable
        _ => CONFIG_C.get_or_init(|| language_c_config().unwrap()),
    }
}

pub fn support_lang() -> Vec<&'static str> {
    vec![
        "Bash", "C", "C++", "Go", "HTML", "Java", "JavaScript", 
        "Json", "Math", "OCaml", "PHP", "Python", "Ruby", "Rust", "Scala", "TOML", "TypeScript"
    ]
}

pub fn ext_to_lang(lang: &str) -> Option<String> {
    match lang.to_lowercase().as_str() {
        "c" => Some("C".to_string()),
        "cpp" | "cc" | "cxx" | "c++" => Some("C++".to_string()),
        "js" => Some("JavaScript".to_string()),
        "ts" => Some("TypeScript".to_string()),
        "rs" => Some("Rust".to_string()),
        "go" => Some("Go".to_string()),
        "sh" => Some("Bash".to_string()),
        "json" => Some("Json".to_string()),
        "py" => Some("Python".to_string()),
        "java" => Some("Java".to_string()),
        "php" => Some("PHP".to_string()),
        "rb" => Some("Ruby".to_string()),
        "scala" => Some("Scala".to_string()),
        // "kt" => Some("Kotlin".to_string()),  // Version conflict, temporarily unavailable
        // "swift" => Some("Swift".to_string()), // Temporarily unavailable
        "html" | "htm" => Some("HTML".to_string()),
        //"css" => Some("CSS".to_string()),
        "toml" => Some("TOML".to_string()),
        "ml" => Some("OCaml".to_string()),
        "math" => Some("Math".to_string()),
        // "yaml" | "yml" => Some("YAML".to_string()), // Temporarily unavailable
        // "toml" => Some("TOML".to_string()),   // Temporarily unavailable
        // "md" | "markdown" => Some("Markdown".to_string()), // Temporarily unavailable
        // "lua" => Some("Lua".to_string()),     // Temporarily unavailable
        // "dart" => Some("Dart".to_string()),   // Temporarily unavailable
        // "ex" => Some("Elixir".to_string()),   // Temporarily unavailable
        // "clj" => Some("Clojure".to_string()),  // Temporarily unavailable
        // "hs" => Some("Haskell".to_string()),    // Temporarily unavailable
        // "fs" => Some("FSharp".to_string()),     // Temporarily unavailable
        // "nim" => Some("Nim".to_string()),       // Temporarily unavailable
        // "zig" => Some("Zig".to_string()),       // Temporarily unavailable
        // "v" => Some("V".to_string()),           // Temporarily unavailable
        _ => None
    }
}

fn highlight<'a>(lang: String, source: &'a [u8]) -> SitResult<Vec<LightSlice<'a>>> {
    let mut v = vec![];

    let key = lang_key(&lang);
    let names = highlight_names_for_lang_key(key);

    let config = lang_configure(&lang);
    let mut highlighter = Highlighter::new();
    let highlights = highlighter.highlight(config, source, None, |_|None)?;

    let mut type_id = None;
    let mut dark_color = DARK_TEXT_COLOR;
    let mut light_color = LIGHT_TEXT_COLOR;
    highlights.filter(|x| x.is_ok()).for_each(|event|{
        let event = event.unwrap();
        match event {
            HighlightEvent::Source{start, end} => {
                v.push(LightSlice{
                    type_id,
                    slice: &source[start..end],
                    dark_color,
                    light_color
                });
            }
            HighlightEvent::HighlightStart(h) => {
                type_id = Some(h.0);
                let cap = names.get(h.0).copied().unwrap_or("variable");
                let (lc, dc) = colors_for_highlight_name(cap);
                light_color = lc;
                dark_color = dc;
            }
            HighlightEvent::HighlightEnd => {
                type_id = None;
                dark_color = DARK_TEXT_COLOR;
                light_color = LIGHT_TEXT_COLOR;
            }
        }
    });

    Ok(v)    
}

pub fn highlight_lines<'a>(lang: String, source: &'a [u8]) -> SitResult<Vec<Vec<LightSlice<'a>>>> {
    let mut lines = vec![];
    let v =   highlight(lang, source)?;
    let mut line = vec![];
    for node in v {
        let multi: Vec<&[u8]> = node.slice.split(|s| *s == b'\n').collect();
        for (i, n) in multi.iter().enumerate() {
            if i > 0 {
                lines.push(line.clone());
                line.truncate(0);
            }
            if n.len() > 0 {
                line.push(LightSlice{
                    type_id: node.type_id,
                    slice: n,
                    dark_color: node.dark_color,
                    light_color: node.light_color,
                });
            }
        }
    }
    lines.push(line.clone());
    line.truncate(0);

    Ok(lines)
}


#[test]
fn highlight_test() {
    let s = r#"int main() /*{
    return 0;*/
}

"#;
    let source = s.as_bytes();
    if let Ok(lines) = highlight_lines("C".to_string(), source) {
        for line in lines {
            for x in line {
                println!("{:?}[{}]", x.type_id, String::from_utf8_lossy(x.slice));
            }
            println!("-----");
        }
    }

}

