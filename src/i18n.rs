use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    ZhCn,
    EnUs,
}

impl Language {
    pub fn code(&self) -> &'static str {
        match self {
            Language::ZhCn => "zh-CN",
            Language::EnUs => "en-US",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code {
            "en-US" => Language::EnUs,
            _ => Language::ZhCn,
        }
    }
}

type StrMap = HashMap<String, String>;

static CURRENT_LANG: RwLock<Language> = RwLock::new(Language::ZhCn);
static ZH_CN_STRINGS: OnceLock<StrMap> = OnceLock::new();
static EN_US_STRINGS: OnceLock<StrMap> = OnceLock::new();

fn load_lang_json(json: &str) -> StrMap {
    serde_json::from_str(json).unwrap_or_default()
}

fn strings(lang: Language) -> &'static StrMap {
    match lang {
        Language::ZhCn => ZH_CN_STRINGS.get_or_init(|| load_lang_json(include_str!("../locales/zh-CN.json"))),
        Language::EnUs => EN_US_STRINGS.get_or_init(|| load_lang_json(include_str!("../locales/en-US.json"))),
    }
}

pub fn set_language(lang: Language) {
    if let Ok(mut guard) = CURRENT_LANG.write() {
        *guard = lang;
    }
}

pub fn set_language_code(code: &str) {
    set_language(Language::from_code(code));
}

pub fn current_language() -> Language {
    match CURRENT_LANG.read() {
        Ok(guard) => *guard,
        Err(_) => Language::ZhCn,
    }
}

pub fn current_language_code() -> String {
    current_language().code().to_string()
}

pub fn tr_with_lang(lang: Language, key: &str) -> String {
    let map = strings(lang);
    map.get(key).cloned().unwrap_or_else(|| key.to_string())
}

pub fn tr(key: &str) -> String {
    let lang = current_language();
    tr_with_lang(lang, key)
}


