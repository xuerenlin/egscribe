use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// Default value functions
fn default_show_line_no() -> bool { true }
fn default_show_index_window() -> bool { true }
fn default_wrap() -> bool { false }
fn default_font_size() -> f32 { 16.0 }
fn default_indent_size() -> f32 { 16.0 }
fn default_list_item_indent_size() -> f32 { 24.0 }
fn default_dark_mode() -> bool { true }
fn default_current_file() -> String { String::new() }
fn default_opend_files() -> Vec<String> { vec![] }
fn default_tree_open_state() -> HashMap<String, bool> { HashMap::new() }
fn default_tree_open_state_changed() -> bool { false }
fn default_default_charset() -> String { "UTF-8".to_string() }
fn default_auto_detect_encoding() -> bool { true }
fn default_text_color_brightness() -> f32 { 1.0 }
fn default_show_scroll_bar() -> bool { false }
fn default_language() -> String { "zh-CN".to_string() }
fn default_fixed_files() -> Vec<String> { vec![] }
fn default_recent_files() -> Vec<String> { vec![] }
fn default_auto_save_enabled() -> bool { true }
fn default_auto_save_interval() -> u64 { 10 }
fn default_table_frame_style() -> crate::medit::pgh::TableFrameStyle {
    crate::medit::pgh::TableFrameStyle::Horizontal
}
fn default_show_heading_section_numbers() -> bool { true }
fn default_show_table_row_no() -> bool { true }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_show_line_no")]
    pub show_line_no: bool,
    #[serde(default = "default_show_index_window")]
    pub show_index_window: bool,
    #[serde(default = "default_wrap")]
    pub wrap: bool,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_indent_size")]
    pub indent_size: f32,
    #[serde(default = "default_list_item_indent_size")]
    pub list_item_indent_size: f32,
    #[serde(default = "default_dark_mode")]
    pub dark_mode: bool,
    #[serde(default = "default_current_file")]
    pub current_file: String,
    #[serde(default = "default_opend_files")]
    pub opend_files: Vec<String>,
    #[serde(default = "default_tree_open_state")]
    pub tree_open_state: HashMap<String, bool>,
    #[serde(default = "default_tree_open_state_changed")]
    pub tree_open_state_changed: bool,
    #[serde(default = "default_default_charset")]
    pub default_charset: String, // Default charset
    #[serde(default = "default_auto_detect_encoding")]
    pub auto_detect_encoding: bool, // Whether to auto-detect encoding
    #[serde(default = "default_text_color_brightness")]
    pub text_color_brightness: f32, // Text color brightness multiplier (1.0 = no change, >1.0 = brighter, <1.0 = darker)
    #[serde(default = "default_show_scroll_bar")]
    pub show_scroll_bar: bool, // Whether to show scroll bar in tab bar
    #[serde(default = "default_language")]
    pub language: String, // UI language code, e.g. "zh-CN", "en-US"
    #[serde(default = "default_fixed_files")]
    pub fixed_files: Vec<String>, // Fixed files/notes that should remain open
    #[serde(default = "default_recent_files")]
    pub recent_files: Vec<String>, // Recently opened files/notes (max 12)
    #[serde(default = "default_auto_save_enabled")]
    pub auto_save_enabled: bool, // Whether to enable auto-save
    #[serde(default = "default_auto_save_interval")]
    pub auto_save_interval: u64, // Auto-save interval in seconds
    #[serde(default = "default_table_frame_style")]
    pub table_frame_style: crate::medit::pgh::TableFrameStyle, // Table frame style: full, horizontal, none
    #[serde(default = "default_show_heading_section_numbers")]
    pub show_heading_section_numbers: bool, // Markdown 标题行首多级序号（编辑区与侧栏目录）
    #[serde(default = "default_show_table_row_no")]
    pub show_table_row_no: bool, // Markdown 表格左侧行号列（表头格为 #）
}

impl Default for Config {
    fn default() -> Self {
        Self {
            show_line_no: true,
            show_index_window: true,
            wrap: false,
            font_size: 16.0,
            indent_size: 16.0,
            list_item_indent_size: 24.0,
            dark_mode: true,
            current_file: String::new(),
            opend_files: vec![],
            tree_open_state: HashMap::new(),
            tree_open_state_changed: false,
            default_charset: "UTF-8".to_string(),
            auto_detect_encoding: true,
            text_color_brightness: 1.0,
            show_scroll_bar: false,
            language: default_language(),
            fixed_files: vec![],
            recent_files: vec![],
            auto_save_enabled: true,
            auto_save_interval: 30,
            table_frame_style: default_table_frame_style(),
            show_heading_section_numbers: true,
            show_table_row_no: default_show_table_row_no(),
        }
    }
}

impl Config {
    pub fn tree_open_state_update(&mut self, name: &str, is_open: bool) {
        if let Some(old) = self.tree_open_state.insert(name.to_string(), is_open) {
            if old == is_open {
                return;
            }
        }
        self.tree_open_state_changed = true;
    }

    pub fn tree_open_state_is_open(&self, name: &str) -> bool {
        if let Some(is_open) = self.tree_open_state.get(name) {
            *is_open
        } else {
            true
        }
    }
} 