use std::sync::OnceLock;

use eframe::egui::Color32;
use tree_sitter_highlight::Highlighter;
use tree_sitter_highlight::HighlightConfiguration;
use tree_sitter_highlight::HighlightEvent;

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
    Color32::from_rgb	(0, 0, 200),    //	深蓝色，突出控制流关键词	🔵
    Color32::from_rgb	(80, 80, 80),   //	中性深灰，避免视觉干扰	⚫
    Color32::from_rgb	(100, 100, 100),//	浅于运算符，区分括号/逗号	⚫
    Color32::from_rgb	(0, 150, 0),    //	深绿色，清晰区分文本内容	🟢
    Color32::from_rgb	(200, 80, 0),   //	橙红色，强调不可变常量	🟠
    Color32::from_rgb	(128, 0, 128),  //	紫色，与常量区分数值类型	🟣
    Color32::from_rgb	(139, 0, 139),  //	深紫色，标识函数定义	🟣
    Color32::from_rgb	(178, 34, 34),  //	深红色，用于对象属性	🔴
    Color32::from_rgb	(0, 100, 100),  //	深青色，标记跳转标签	🔵
    Color32::from_rgb	(0, 128, 128),  //	青色，表示类型声明	🟢
    Color32::from_rgb	(139, 69, 19),  //	深棕色，普通变量	🟤
    Color32::from_rgb	(128, 128, 128),//	浅灰，降低注释存在感	⚫
];

static HIGHLIGH_COLOR_DARK: [Color32; 12] = [
    Color32::from_rgb	(100, 200, 255),    //	亮蓝色，对比度高且不刺眼	🔵
    Color32::from_rgb	(180, 180, 180),    //	浅灰，保持代码结构清晰	⚪
    Color32::from_rgb	(150, 150, 150),    //	稍暗于运算符，维持层次感	⚪
    Color32::from_rgb	(100, 255, 100),    //	荧光绿，突出字符串内容	🟢
    Color32::from_rgb	(255, 160, 0),      //	亮橙色，强调常量不可变性	🟠
    Color32::from_rgb	(200, 100, 255),    //	亮紫色，区分数值与常量	🟣
    Color32::from_rgb	(255, 105, 180),    //	粉色，醒目标识函数	💖
    Color32::from_rgb	(255, 127, 80),     //	珊瑚色，对象属性高对比度	🟠
    Color32::from_rgb	(0, 255, 255),      //	青色，标签跳转清晰可见	🟢
    Color32::from_rgb	(0, 255, 200),      //	蓝绿色，增强类型声明可读性	🟢
    Color32::from_rgb	(245, 222, 179),    //	米色，避免与背景混淆	🟡
    Color32::from_rgb	(150, 180, 150),    //	灰绿色，柔和且不喧宾夺主	🟢
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

static CONFIG_JS: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_C: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_RUST: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_GO: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_BASH: OnceLock<HighlightConfiguration> = OnceLock::new();
static CONFIG_JSON: OnceLock<HighlightConfiguration> = OnceLock::new();

fn lang_configure(lang: &str) -> &'static HighlightConfiguration {
    if lang.eq_ignore_ascii_case("c") {
        CONFIG_C.get_or_init(||{language_c_config().unwrap()})
    } else if lang.eq_ignore_ascii_case("javascript") {
        CONFIG_JS.get_or_init(||{language_js_config().unwrap()})
    } else if lang.eq_ignore_ascii_case("rust") {
        CONFIG_RUST.get_or_init(||{language_rust_config().unwrap()})
    } else if lang.eq_ignore_ascii_case("go") {
        CONFIG_GO.get_or_init(||{language_go_config().unwrap()})
    } else if lang.eq_ignore_ascii_case("bash") {
        CONFIG_BASH.get_or_init(||{language_bash_config().unwrap()})
    } else if lang.eq_ignore_ascii_case("json") {
        CONFIG_JSON.get_or_init(||{language_json_config().unwrap()})
    } else {
        CONFIG_C.get_or_init(||{language_c_config().unwrap()})
    }
}

pub fn support_lang() -> Vec<&'static str> {
    vec!["C", "Rust", "Go", "Bash", "Json", "JavaScript"]
}

pub fn ext_to_lang(lang: &str) -> Option<String> {
    match lang {
        "c"|"cpp" => Some("c".to_string()),
        "js" => Some("javascript".to_string()),
        "rs" => Some("rust".to_string()),
        "go" => Some("go".to_string()),
        "sh" => Some("bash".to_string()),
        "json" => Some("json".to_string()),
        _ => None
    }
}

fn highlight(lang: String, source: &[u8]) -> SitResult<Vec<LightSlice>> {
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

pub fn highlight_lines(lang: String, source: &[u8]) -> SitResult<Vec<Vec<LightSlice>>> {
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

