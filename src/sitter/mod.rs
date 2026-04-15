use std::sync::OnceLock;

use eframe::egui::Color32;
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


static HIGHLIGH_NAMES: [&str; 12] = [
    "keyword",
    "operator",
    "delimiter",
    "string",
    "constant",
    "number",
    "function",
    "property",
    "label",
    "type",
    "variable",
    "comment",
];

static HIGHLIGH_COLOR_LIGHT: [Color32; 12] = [
    Color32::from_rgb	(0, 0, 200),    //	Dark blue, highlight control flow keywords	🔵
    Color32::from_rgb	(80, 80, 80),   //	Neutral dark gray, avoid visual interference	⚫
    Color32::from_rgb	(100, 100, 100),//	Lighter than operators, distinguish brackets/commas	⚫
    Color32::from_rgb	(0, 150, 0),    //	Dark green, clearly distinguish text content	🟢
    Color32::from_rgb	(200, 80, 0),   //	Orange-red, emphasize immutable constants	🟠
    Color32::from_rgb	(128, 0, 128),  //	Purple, distinguish numeric types from constants	🟣
    Color32::from_rgb	(139, 0, 139),  //	Dark purple, identify function definitions	🟣
    Color32::from_rgb	(178, 34, 34),  //	Dark red, for object properties	🔴
    Color32::from_rgb	(0, 100, 100),  //	Dark cyan, mark jump labels	🔵
    Color32::from_rgb	(0, 128, 128),  //	Cyan, represent type declarations	🟢
    Color32::from_rgb	(139, 69, 19),  //	Dark brown, ordinary variables	🟤
    Color32::from_rgb	(128, 128, 128),//	Light gray, reduce comment presence	⚫
];

static HIGHLIGH_COLOR_DARK: [Color32; 12] = [
    Color32::from_rgb	(100, 200, 255),    //	Bright blue, high contrast and not harsh	🔵
    Color32::from_rgb	(180, 180, 180),    //	Light gray, maintain clear code structure	⚪
    Color32::from_rgb	(150, 150, 150),    //	Slightly darker than operators, maintain hierarchy	⚪
    Color32::from_rgb	(100, 255, 100),    //	Fluorescent green, highlight string content	🟢
    Color32::from_rgb	(255, 160, 0),      //	Bright orange, emphasize constant immutability	🟠
    Color32::from_rgb	(200, 100, 255),    //	Bright purple, distinguish numbers from constants	🟣
    Color32::from_rgb	(255, 105, 180),    //	Pink, prominently identify functions	💖
    Color32::from_rgb	(255, 127, 80),     //	Coral, high contrast for object properties	🟠
    Color32::from_rgb	(0, 255, 255),      //	Cyan, clearly visible jump labels	🟢
    Color32::from_rgb	(0, 255, 200),      //	Blue-green, enhance type declaration readability	🟢
    Color32::from_rgb	(245, 222, 179),    //	Beige, avoid confusion with background	🟡
    Color32::from_rgb	(150, 180, 150),    //	Gray-green, soft and not overwhelming	🟢
];

fn language_js_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_javascript::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_javascript::HIGHLIGHT_QUERY, 
        tree_sitter_javascript::INJECTION_QUERY, 
        tree_sitter_javascript::LOCALS_QUERY)?;

    config.configure(&HIGHLIGH_NAMES);
    Ok(config)
}

fn language_c_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_c::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_c::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(&HIGHLIGH_NAMES);
    Ok(config)
}

fn language_rust_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_rust::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_rust::HIGHLIGHT_QUERY,
        tree_sitter_rust::INJECTIONS_QUERY, 
        "" )?;

    config.configure(&HIGHLIGH_NAMES);
    Ok(config)
}

fn language_go_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_go::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_go::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(&HIGHLIGH_NAMES);
    Ok(config)
}

fn language_bash_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_bash::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_bash::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(&HIGHLIGH_NAMES);
    Ok(config)
}

fn language_json_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_json::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_json::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(&HIGHLIGH_NAMES);
    Ok(config)
}

fn language_python_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_python::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_python::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(&HIGHLIGH_NAMES);
    Ok(config)
}

fn language_java_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_java::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_java::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(&HIGHLIGH_NAMES);
    Ok(config)
}

fn language_cpp_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_cpp::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_cpp::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(&HIGHLIGH_NAMES);
    Ok(config)
}


fn language_php_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_php::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_php::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(&HIGHLIGH_NAMES);
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

    config.configure(&HIGHLIGH_NAMES);
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

    config.configure(&HIGHLIGH_NAMES);
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

    config.configure(&HIGHLIGH_NAMES);
    Ok(config)
}

fn language_html_config() -> SitResult<HighlightConfiguration> {
    let language = tree_sitter_html::language();
    let mut config: HighlightConfiguration = HighlightConfiguration::new(
        language, 
        tree_sitter_html::HIGHLIGHT_QUERY,
        "", 
        "" )?;

    config.configure(&HIGHLIGH_NAMES);
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

    config.configure(&HIGHLIGH_NAMES);
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

    config.configure(&HIGHLIGH_NAMES);
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
    match lang.to_lowercase().as_str() {
        "c" => CONFIG_C.get_or_init(||{language_c_config().unwrap()}),
        "javascript" | "js" => CONFIG_JS.get_or_init(||{language_js_config().unwrap()}),
        "rust" => CONFIG_RUST.get_or_init(||{language_rust_config().unwrap()}),
        "go" => CONFIG_GO.get_or_init(||{language_go_config().unwrap()}),
        "bash" | "sh" => CONFIG_BASH.get_or_init(||{language_bash_config().unwrap()}),
        "json" => CONFIG_JSON.get_or_init(||{language_json_config().unwrap()}),
        "python" | "py" => CONFIG_PYTHON.get_or_init(||{language_python_config().unwrap()}),
        "java" => CONFIG_JAVA.get_or_init(||{language_java_config().unwrap()}),
        "cpp" | "c++" | "cc" | "cxx" => CONFIG_CPP.get_or_init(||{language_cpp_config().unwrap()}),
        "php" => CONFIG_PHP.get_or_init(||{language_php_config().unwrap()}),
        "ruby" | "rb" => CONFIG_RUBY.get_or_init(||{language_ruby_config().unwrap()}),
        "scala" => CONFIG_SCALA.get_or_init(||{language_scala_config().unwrap()}),
        // "kotlin" | "kt" => CONFIG_KOTLIN.get_or_init(||{language_kotlin_config().unwrap()}),  // Version conflict, temporarily unavailable
        // "swift" => CONFIG_SWIFT.get_or_init(||{language_swift_config().unwrap()}),            // Temporarily unavailable
        "typescript" | "ts" => CONFIG_TYPESCRIPT.get_or_init(||{language_typescript_config().unwrap()}),
        "html" | "htm" => CONFIG_HTML.get_or_init(||{language_html_config().unwrap()}),
        //"css" => CONFIG_CSS.get_or_init(||{language_css_config().unwrap()}),
        "toml" => CONFIG_TOML.get_or_init(||{language_toml_config().unwrap()}),
        "ocaml" | "ml" => CONFIG_OCAML.get_or_init(||{language_ocaml_config().unwrap()}),
        "math" => CONFIG_C.get_or_init(||{language_c_config().unwrap()}),  // Use C config as fallback for math expressions
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
        _ => CONFIG_C.get_or_init(||{language_c_config().unwrap()})
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
                dark_color = HIGHLIGH_COLOR_DARK[h.0];
                light_color = HIGHLIGH_COLOR_LIGHT[h.0];
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

