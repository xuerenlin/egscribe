use crate::medit::{Ctx, Cursor, PghType, UrlInfo};
use eframe::egui::{Key, Ui, Vec2};
use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// 触发器类型
/// 定义插件需要监听的事件类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    /// 行内容发生变化
    LineChanged,
    /// 文件打开
    FileOpened,
    /// 文件保存
    FileSaved,
    /// 文件关闭
    FileClosed,
    /// 光标位置变化
    CursorChanged,
    /// 选择文本变化
    SelectionChanged,
}

impl Trigger {
    /// 将触发器转换为事件类型字符串
    pub fn to_event_type(&self) -> &'static str {
        match self {
            Trigger::LineChanged => "line_changed",
            Trigger::FileOpened => "file_opened",
            Trigger::FileSaved => "file_saved",
            Trigger::FileClosed => "file_closed",
            Trigger::CursorChanged => "cursor_changed",
            Trigger::SelectionChanged => "selection_changed",
        }
    }
    
    /// 从事件类型字符串创建触发器
    pub fn from_event_type(event_type: &str) -> Option<Self> {
        match event_type {
            "line_changed" => Some(Trigger::LineChanged),
            "file_opened" => Some(Trigger::FileOpened),
            "file_saved" => Some(Trigger::FileSaved),
            "file_closed" => Some(Trigger::FileClosed),
            "cursor_changed" => Some(Trigger::CursorChanged),
            "selection_changed" => Some(Trigger::SelectionChanged),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub enum FindCmd {
    Find,
    Replace,
    ReplaceAll,
    FindAll,
    FindNotes,
}

#[derive(Clone)]
pub struct FindReplaceCtx {
    pub find: String,
    pub replace: String,
    pub is_case: bool,
    pub is_hole_word: bool,
    pub is_reg: bool,
    pub cmd: Option<FindCmd>,
    pub regex: Option<Regex>,
}

impl FindReplaceCtx {
    pub fn new() -> Self {
        FindReplaceCtx {
            find: "".to_string(),
            replace: "".to_string(),
            is_case: false,
            is_hole_word: false,
            is_reg: false,
            cmd: None,
            regex: None,
        }
    }

    pub fn sample(find: String) -> Self {
        let mut s = FindReplaceCtx::new();
        s.find = find;
        s.is_case = true;
        s
    }

    pub fn regex_build(&mut self) {
        if self.is_reg {
            let mut builder = RegexBuilder::new(&self.find);
            builder.case_insensitive(!self.is_case);
            if let Ok(re) = builder.build() {
                self.regex = Some(re);
                return;
            }
        }
        self.regex = None;
    }

    /// 获取 find_replace 命令的参数定义
    pub fn param_info() -> Vec<ParamInfo> {
        vec![
            ParamInfo {
                name: "find",
                param_type: ParamType::String,
                required: false,
                description: "Search text",
            },
            ParamInfo {
                name: "replace",
                param_type: ParamType::String,
                required: false,
                description: "Replace text",
            },
            ParamInfo {
                name: "is_case",
                param_type: ParamType::Boolean,
                required: false,
                description: "Case sensitive",
            },
            ParamInfo {
                name: "is_hole_word",
                param_type: ParamType::Boolean,
                required: false,
                description: "Whole word match",
            },
            ParamInfo {
                name: "is_reg",
                param_type: ParamType::Boolean,
                required: false,
                description: "Use regular expression",
            },
            ParamInfo {
                name: "cmd",
                param_type: ParamType::OptionalString,
                required: false,
                description: "Find command type (Find, Replace, ReplaceAll, FindAll, FindNotes)",
            },
        ]
    }

    /// 将 FindReplaceCtx 转换为 Action
    pub fn to_action(&self) -> Action {
        let mut params = std::collections::HashMap::new();
        params.insert("find".to_string(), serde_json::Value::String(self.find.clone()));
        params.insert("replace".to_string(), serde_json::Value::String(self.replace.clone()));
        params.insert("is_case".to_string(), serde_json::Value::Bool(self.is_case));
        params.insert("is_hole_word".to_string(), serde_json::Value::Bool(self.is_hole_word));
        params.insert("is_reg".to_string(), serde_json::Value::Bool(self.is_reg));
        // 保存 cmd 字段（序列化为字符串）
        if let Some(cmd) = &self.cmd {
            let cmd_str = match cmd {
                FindCmd::Find => "Find",
                FindCmd::Replace => "Replace",
                FindCmd::ReplaceAll => "ReplaceAll",
                FindCmd::FindAll => "FindAll",
                FindCmd::FindNotes => "FindNotes",
            };
            params.insert("cmd".to_string(), serde_json::Value::String(cmd_str.to_string()));
        }
        Action::new("find_replace".to_string(), params)
    }

    /// 从 Action 创建 FindReplaceCtx
    pub fn from_action(action: &Action) -> Result<Self, String> {
        if action.command != "find_replace" {
            return Err(format!("Action command '{}' is not 'find_replace'", action.command));
        }

        let find = action.get_optional_string_param("find")
            .ok()
            .flatten()
            .unwrap_or_default();
        let replace = action.get_optional_string_param("replace")
            .ok()
            .flatten()
            .unwrap_or_default();
        let is_case = action.get_optional_bool_param("is_case", false)
            .unwrap_or(false);
        let is_hole_word = action.get_optional_bool_param("is_hole_word", false)
            .unwrap_or(false);
        let is_reg = action.get_optional_bool_param("is_reg", false)
            .unwrap_or(false);
        
        // 提取并反序列化 cmd 字段
        let find_cmd = action.get_optional_string_param("cmd")
            .ok()
            .flatten()
            .and_then(|cmd_str| {
                match cmd_str.as_str() {
                    "Find" => Some(FindCmd::Find),
                    "Replace" => Some(FindCmd::Replace),
                    "ReplaceAll" => Some(FindCmd::ReplaceAll),
                    "FindAll" => Some(FindCmd::FindAll),
                    "FindNotes" => Some(FindCmd::FindNotes),
                    _ => None,
                }
            });
        
        let mut ctx = FindReplaceCtx {
            find,
            replace,
            is_case,
            is_hole_word,
            is_reg,
            cmd: find_cmd,
            regex: None,
        };
        ctx.regex_build();
        Ok(ctx)
    }
}

/// 快捷键定义
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShortcutKey {
    pub key: Key,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl ShortcutKey {
    pub fn new(key: Key, ctrl: bool, shift: bool, alt: bool) -> Self {
        Self { key, ctrl, shift, alt }
    }

    /// 检查是否匹配给定的按键和修饰符
    pub fn matches(&self, key: &Key, modifiers: &eframe::egui::Modifiers) -> bool {
        self.key == *key
            && self.ctrl == modifiers.ctrl
            && self.shift == modifiers.shift
            && self.alt == modifiers.alt
    }

    /// 转换为字符串（用于显示）
    pub fn to_string(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl".to_string());
        }
        if self.shift {
            parts.push("Shift".to_string());
        }
        if self.alt {
            parts.push("Alt".to_string());
        }
        // 使用 egui 的 Key::name() 方法获取键的名称
        let key_str = self.key.name();
        parts.push(key_str.to_string());
        parts.join("+")
    }
}

/// 参数类型
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParamType {
    String,
    Number,
    Boolean,
    OptionalString,
}

/// 参数信息
#[derive(Clone, Debug)]
pub struct ParamInfo {
    /// 参数名称
    pub name: &'static str,
    /// 参数类型
    pub param_type: ParamType,
    /// 是否必需
    pub required: bool,
    /// 参数描述
    pub description: &'static str,
}

/// 动作执行函数类型（用于编辑器级别的动作）
pub type ActionExecutor = fn(&mut Ctx, &mut Ui, &Action);

/// 动作信息（包含元数据）
#[derive(Clone)]
pub struct ActionInfo {
    /// 命令名称（用于插件系统）
    pub command: &'static str,
    /// 快捷键（可选，仅编辑器级别的动作有）
    pub shortcut_key: Option<ShortcutKey>,
    /// 参数列表信息
    pub params: Vec<ParamInfo>,
    /// 执行函数（仅编辑器级别的动作有）
    pub executor: Option<ActionExecutor>,
    /// 触发器事件类型（仅触发器事件有）
    pub trigger: Option<Trigger>,
    /// 动作描述
    pub description: &'static str,
    /// 是否为编辑命令（需要检查 is_read_only）
    pub is_update: bool,
}

impl ActionInfo {
    /// 创建应用级别的动作信息
    pub fn app_command(
        command: &'static str,
        params: Vec<ParamInfo>,
        description: &'static str,
    ) -> Self {
        Self {
            command,
            shortcut_key: None,
            params,
            executor: None,
            trigger: None,
            description,
            is_update: false,
        }
    }

    /// 创建编辑器级别的动作信息
    pub fn editor_action(
        command: &'static str,
        shortcut_key: Option<ShortcutKey>,
        params: Vec<ParamInfo>,
        executor: ActionExecutor,
        description: &'static str,
        is_update: bool,
    ) -> Self {
        Self {
            command,
            shortcut_key,
            params,
            executor: Some(executor),
            trigger: None,
            description,
            is_update,
        }
    }

    /// 创建触发器事件类型的动作信息
    pub fn trigger_event(
        trigger: Trigger,
        params: Vec<ParamInfo>,
        description: &'static str,
    ) -> Self {
        // 触发器事件的命令名称从触发器类型生成
        let command = trigger.to_event_type();
        
        Self {
            command,
            shortcut_key: None,
            params,
            executor: None,
            trigger: Some(trigger),
            description,
            is_update: false,
        }
    }

    /// 验证参数是否符合定义
    pub fn validate_params(&self, params: &std::collections::HashMap<String, serde_json::Value>) -> Result<(), String> {
        for param_info in &self.params {
            let value = params.get(param_info.name);
            
            // 检查必需参数
            if param_info.required && value.is_none() {
                return Err(format!("Missing required parameter: {}", param_info.name));
            }
            
            // 检查参数类型
            if let Some(v) = value {
                match param_info.param_type {
                    ParamType::String => {
                        if !v.is_string() {
                            return Err(format!("Parameter '{}' must be a string", param_info.name));
                        }
                    }
                    ParamType::Number => {
                        if !v.is_number() {
                            return Err(format!("Parameter '{}' must be a number", param_info.name));
                        }
                    }
                    ParamType::Boolean => {
                        if !v.is_boolean() {
                            return Err(format!("Parameter '{}' must be a boolean", param_info.name));
                        }
                    }
                    ParamType::OptionalString => {
                        if !v.is_string() && !v.is_null() {
                            return Err(format!("Parameter '{}' must be a string or null", param_info.name));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// 从命令名和参数创建 Action（带验证）
    pub fn create_action(&self, params: &std::collections::HashMap<String, serde_json::Value>) -> Result<Action, String> {
        self.validate_params(params)?;
        Ok(Action::new(
            self.command.to_string(),
            params.clone(),
        ))
    }
}

/// 统一的动作类型（合并了原来的Command和EditAction）
/// 现在使用 ActionInfo 和参数数据来表示动作
#[derive(Clone)]
pub struct Action {
    /// 命令名称
    pub command: String,
    /// 参数数据（JSON格式）
    pub params: std::collections::HashMap<String, serde_json::Value>,
}

/// 静态的动作信息注册表（使用 OnceLock 确保只初始化一次）
static ACTION_INFOS: OnceLock<std::collections::HashMap<&'static str, ActionInfo>> = OnceLock::new();

impl Action {
    /// 创建新的 Action
    pub fn new(command: String, params: std::collections::HashMap<String, serde_json::Value>) -> Self {
        Self {
            command,
            params,
        }
    }

    /// 获取所有动作信息的注册表
    pub fn action_infos() -> &'static std::collections::HashMap<&'static str, ActionInfo> {
        ACTION_INFOS.get_or_init(|| {
            use std::collections::HashMap;
            let mut infos = HashMap::new();

        // 应用级别的命令
        infos.insert("open_file", ActionInfo::app_command(
            "open_file",
            vec![ParamInfo {
                name: "path",
                param_type: ParamType::String,
                required: true,
                description: "File path",
            }],
            "Open file",
        ));

        infos.insert("path_list", ActionInfo::app_command(
            "path_list",
            vec![ParamInfo {
                name: "path",
                param_type: ParamType::String,
                required: true,
                description: "Path",
            }],
            "Path list",
        ));

        infos.insert("new_file", ActionInfo::app_command(
            "new_file",
            vec![ParamInfo {
                name: "parent",
                param_type: ParamType::OptionalString,
                required: false,
                description: "Parent directory path (optional)",
            }],
            "New file",
        ));

        infos.insert("delete_file", ActionInfo::app_command(
            "delete_file",
            vec![ParamInfo {
                name: "path",
                param_type: ParamType::String,
                required: true,
                description: "File path",
            }],
            "Delete file",
        ));

        infos.insert("rename_file", ActionInfo::app_command(
            "rename_file",
            vec![ParamInfo {
                name: "path",
                param_type: ParamType::String,
                required: true,
                description: "File path",
            }],
            "Rename file",
        ));

        infos.insert("find_replace", ActionInfo::app_command(
            "find_replace",
            FindReplaceCtx::param_info(),
            "Find and replace",
        ));

        infos.insert("click_edit_line", ActionInfo::app_command(
            "click_edit_line",
            vec![ParamInfo {
                name: "line",
                param_type: ParamType::String,
                required: true,
                description: "Line number",
            }],
            "Click edit line",
        ));

        infos.insert("open_url", ActionInfo::app_command(
            "open_url",
            vec![
                ParamInfo {
                    name: "url",
                    param_type: ParamType::String,
                    required: true,
                    description: "URL address",
                },
                ParamInfo {
                    name: "title",
                    param_type: ParamType::OptionalString,
                    required: false,
                    description: "Title (optional)",
                },
                ParamInfo {
                    name: "text",
                    param_type: ParamType::String,
                    required: false,
                    description: "Text",
                },
                ParamInfo {
                    name: "pos_line",
                    param_type: ParamType::Number,
                    required: false,
                    description: "Position line number",
                },
                ParamInfo {
                    name: "pos_col",
                    param_type: ParamType::Number,
                    required: false,
                    description: "Position column number",
                },
            ],
            "Open URL",
        ));

        infos.insert("fixed_file", ActionInfo::app_command(
            "fixed_file",
            vec![ParamInfo {
                name: "path",
                param_type: ParamType::String,
                required: true,
                description: "File path",
            }],
            "Pin file to toolbar",
        ));

        infos.insert("unfixed_file", ActionInfo::app_command(
            "unfixed_file",
            vec![ParamInfo {
                name: "path",
                param_type: ParamType::String,
                required: true,
                description: "File path",
            }],
            "Unpin file",
        ));

        // 编辑器级别的动作
        infos.insert("copy", ActionInfo::editor_action(
            "copy",
            Some(ShortcutKey::new(Key::C, true, false, false)),
            vec![],
            Self::execute_copy,
            "Copy",
            false, // 非编辑命令
        ));

        infos.insert("cut", ActionInfo::editor_action(
            "cut",
            Some(ShortcutKey::new(Key::X, true, false, false)),
            vec![],
            Self::execute_cut,
            "Cut",
            true, // 编辑命令
        ));

        infos.insert("paste", ActionInfo::editor_action(
            "paste",
            Some(ShortcutKey::new(Key::V, true, false, false)),
            vec![],
            Self::execute_paste,
            "Paste",
            true, // 编辑命令
        ));

        infos.insert("insert_text", ActionInfo::editor_action(
            "insert_text",
            None,
            vec![ParamInfo {
                name: "text",
                param_type: ParamType::String,
                required: true,
                description: "Text to insert at cursor (from Paste/Text events)",
            }],
            Self::execute_insert_text,
            "Insert text at cursor",
            true, // 编辑命令
        ));

        infos.insert("delete", ActionInfo::editor_action(
            "delete",
            Some(ShortcutKey::new(Key::Delete, false, false, false)),
            vec![],
            Self::execute_delete,
            "Delete",
            true, // 编辑命令
        ));

        infos.insert("backspace", ActionInfo::editor_action(
            "backspace",
            Some(ShortcutKey::new(Key::Backspace, false, false, false)),
            vec![],
            Self::execute_backspace,
            "Backspace",
            true, // 编辑命令
        ));

        infos.insert("arrow_left", ActionInfo::editor_action(
            "arrow_left",
            None,
            vec![],
            Self::execute_arrow_left,
            "Move Left",
            false, // 非编辑命令
        ));

        infos.insert("arrow_right", ActionInfo::editor_action(
            "arrow_right",
            None,
            vec![],
            Self::execute_arrow_right,
            "Move Right",
            false, // 非编辑命令
        ));

        infos.insert("arrow_up", ActionInfo::editor_action(
            "arrow_up",
            None,
            vec![],
            Self::execute_arrow_up,
            "Move Up",
            false, // 非编辑命令
        ));

        infos.insert("arrow_down", ActionInfo::editor_action(
            "arrow_down",
            None,
            vec![],
            Self::execute_arrow_down,
            "Move Down",
            false, // 非编辑命令
        ));

        infos.insert("home", ActionInfo::editor_action(
            "home",
            None,
            vec![],
            Self::execute_home,
            "Move to Home",
            false, // 非编辑命令
        ));

        infos.insert("end", ActionInfo::editor_action(
            "end",
            None,
            vec![],
            Self::execute_end,
            "Move to End",
            false, // 非编辑命令
        ));

        infos.insert("page_down", ActionInfo::editor_action(
            "page_down",
            None,
            vec![],
            Self::execute_page_down,
            "Page Down",
            false, // 非编辑命令
        ));

        infos.insert("page_up", ActionInfo::editor_action(
            "page_up",
            None,
            vec![],
            Self::execute_page_up,
            "Page Up",
            false, // 非编辑命令
        ));

        infos.insert("undo", ActionInfo::editor_action(
            "undo",
            Some(ShortcutKey::new(Key::Z, true, false, false)),
            vec![],
            Self::execute_undo,
            "Undo",
            true, // 编辑命令
        ));

        infos.insert("redo", ActionInfo::editor_action(
            "redo",
            Some(ShortcutKey::new(Key::Y, true, false, false)),
            vec![],
            Self::execute_redo,
            "Redo",
            true, // 编辑命令
        ));

        infos.insert("insert_tab", ActionInfo::editor_action(
            "insert_tab",
            None,
            vec![],
            Self::execute_insert_tab,
            "Insert Tab",
            true, // 编辑命令
        ));

        infos.insert("set_expanded_text", ActionInfo::editor_action(
            "set_expanded_text",
            None,
            vec![
                ParamInfo {
                    name: "line_no",
                    param_type: ParamType::Number,
                    required: true,
                    description: "Line number",
                },
                ParamInfo {
                    name: "expanded_text",
                    param_type: ParamType::OptionalString,
                    required: false,
                    description: "Expanded text content (optional, null to clear)",
                },
            ],
            Self::execute_set_expanded_text,
            "Set expanded text for a line",
            true, // 编辑命令
        ));

        infos.insert("enter", ActionInfo::editor_action(
            "enter",
            None,
            vec![],
            Self::execute_enter,
            "Enter",
            true, // 编辑命令
        ));

        infos.insert("select_all", ActionInfo::editor_action(
            "select_all",
            Some(ShortcutKey::new(Key::A, true, false, false)),
            vec![],
            Self::execute_select_all,
            "Select all",
            false, // 非编辑命令
        ));

        infos.insert("bold", ActionInfo::editor_action(
            "bold",
            Some(ShortcutKey::new(Key::B, true, false, false)),
            vec![],
            Self::execute_bold,
            "Markdown: Bold",
            true, // 编辑命令
        ));

        infos.insert("italic", ActionInfo::editor_action(
            "italic",
            Some(ShortcutKey::new(Key::I, true, false, false)),
            vec![],
            Self::execute_italic,
            "Markdown: Italic",
            true, // 编辑命令
        ));

        infos.insert("strikethrough", ActionInfo::editor_action(
            "strikethrough",
            Some(ShortcutKey::new(Key::S, true, true, false)),
            vec![],
            Self::execute_strikethrough,
            "Markdown: Strikethrough",
            true, // 编辑命令
        ));

        infos.insert("code", ActionInfo::editor_action(
            "code",
            Some(ShortcutKey::new(Key::Backtick, true, false, false)),
            vec![],
            Self::execute_code,
            "Markdown: Code",
            true, // 编辑命令
        ));

        infos.insert("code_block", ActionInfo::editor_action(
            "code_block",
            Some(ShortcutKey::new(Key::Backtick, true, true, false)),
            vec![],
            Self::execute_code_block,
            "Markdown: Code block",
            true, // 编辑命令
        ));

        infos.insert("link", ActionInfo::editor_action(
            "link",
            Some(ShortcutKey::new(Key::K, true, false, false)),
            vec![],
            Self::execute_link,
            "Markdown: Link",
            true, // 编辑命令
        ));

        infos.insert("table", ActionInfo::editor_action(
            "table",
            Some(ShortcutKey::new(Key::T, true, true, false)),
            vec![],
            Self::execute_table,
            "Markdown: Table",
            true, // 编辑命令
        ));

        // Heading 1-6
        infos.insert("heading_1", ActionInfo::editor_action(
            "heading_1",
            Some(ShortcutKey::new(Key::Num1, true, false, false)),
            vec![],
            Self::execute_heading_1,
            "Markdown: Heading 1",
            true, // 编辑命令
        ));
        infos.insert("heading_2", ActionInfo::editor_action(
            "heading_2",
            Some(ShortcutKey::new(Key::Num2, true, false, false)),
            vec![],
            Self::execute_heading_2,
            "Markdown: Heading 2",
            true, // 编辑命令
        ));
        infos.insert("heading_3", ActionInfo::editor_action(
            "heading_3",
            Some(ShortcutKey::new(Key::Num3, true, false, false)),
            vec![],
            Self::execute_heading_3,
            "Markdown: Heading 3",
            true, // 编辑命令
        ));
        infos.insert("heading_4", ActionInfo::editor_action(
            "heading_4",
            Some(ShortcutKey::new(Key::Num4, true, false, false)),
            vec![],
            Self::execute_heading_4,
            "Markdown: Heading 4",
            true, // 编辑命令
        ));
        infos.insert("heading_5", ActionInfo::editor_action(
            "heading_5",
            Some(ShortcutKey::new(Key::Num5, true, false, false)),
            vec![],
            Self::execute_heading_5,
            "Markdown: Heading 5",
            true, // 编辑命令
        ));
        infos.insert("heading_6", ActionInfo::editor_action(
            "heading_6",
            Some(ShortcutKey::new(Key::Num6, true, false, false)),
            vec![],
            Self::execute_heading_6,
            "Markdown: Heading 6",
            true, // 编辑命令
        ));

        infos.insert("quote", ActionInfo::editor_action(
            "quote",
            Some(ShortcutKey::new(Key::Q, true, true, false)),
            vec![],
            Self::execute_quote,
            "Markdown: Quote",
            true, // 编辑命令
        ));

        infos.insert("unordered_list", ActionInfo::editor_action(
            "unordered_list",
            Some(ShortcutKey::new(Key::U, true, true, false)),
            vec![],
            Self::execute_unordered_list,
            "Markdown: Unordered list",
            true, // 编辑命令
        ));

        infos.insert("ordered_list", ActionInfo::editor_action(
            "ordered_list",
            Some(ShortcutKey::new(Key::O, true, true, false)),
            vec![],
            Self::execute_ordered_list,
            "Markdown: Ordered list",
            true, // 编辑命令
        ));

        infos.insert("todo_list", ActionInfo::editor_action(
            "todo_list",
            Some(ShortcutKey::new(Key::L, true, true, false)),
            vec![],
            Self::execute_todo_list,
            "Markdown: TODO list",
            true, // 编辑命令
        ));

        infos.insert("horizontal_rule", ActionInfo::editor_action(
            "horizontal_rule",
            Some(ShortcutKey::new(Key::H, true, true, false)),
            vec![],
            Self::execute_horizontal_rule,
            "Markdown: Horizontal rule",
            true, // 编辑命令
        ));

        infos.insert("table_delete_selected_rows", ActionInfo::editor_action(
            "table_delete_selected_rows",
            None,
            vec![],
            Self::execute_table_delete_selected_rows,
            "Markdown: Table delete selected rows",
            true,
        ));
        infos.insert("table_delete_selected_cols", ActionInfo::editor_action(
            "table_delete_selected_cols",
            None,
            vec![],
            Self::execute_table_delete_selected_cols,
            "Markdown: Table delete selected columns",
            true,
        ));
        infos.insert("table_insert_row_above", ActionInfo::editor_action(
            "table_insert_row_above",
            None,
            vec![],
            Self::execute_table_insert_row_above,
            "Markdown: Table insert row above",
            true,
        ));
        infos.insert("table_insert_row_below", ActionInfo::editor_action(
            "table_insert_row_below",
            None,
            vec![],
            Self::execute_table_insert_row_below,
            "Markdown: Table insert row below",
            true,
        ));
        infos.insert("table_insert_col_left", ActionInfo::editor_action(
            "table_insert_col_left",
            None,
            vec![],
            Self::execute_table_insert_col_left,
            "Markdown: Table insert column left",
            true,
        ));
        infos.insert("table_insert_col_right", ActionInfo::editor_action(
            "table_insert_col_right",
            None,
            vec![],
            Self::execute_table_insert_col_right,
            "Markdown: Table insert column right",
            true,
        ));

        // 触发器事件
        infos.insert("line_changed", ActionInfo::trigger_event(
            Trigger::LineChanged,
            vec![
                ParamInfo {
                    name: "line_no",
                    param_type: ParamType::Number,
                    required: true,
                    description: "Line number",
                },
                ParamInfo {
                    name: "line_text",
                    param_type: ParamType::String,
                    required: true,
                    description: "Line text content",
                },
            ],
            "Line content changed event",
        ));

        infos.insert("file_opened", ActionInfo::trigger_event(
            Trigger::FileOpened,
            vec![
                ParamInfo {
                    name: "file_path",
                    param_type: ParamType::String,
                    required: true,
                    description: "File path",
                },
            ],
            "File opened event",
        ));

        infos.insert("file_saved", ActionInfo::trigger_event(
            Trigger::FileSaved,
            vec![
                ParamInfo {
                    name: "file_path",
                    param_type: ParamType::String,
                    required: true,
                    description: "File path",
                },
            ],
            "File saved event",
        ));

        infos.insert("file_closed", ActionInfo::trigger_event(
            Trigger::FileClosed,
            vec![
                ParamInfo {
                    name: "file_path",
                    param_type: ParamType::String,
                    required: true,
                    description: "File path",
                },
            ],
            "File closed event",
        ));

        infos.insert("cursor_changed", ActionInfo::trigger_event(
            Trigger::CursorChanged,
            vec![
                ParamInfo {
                    name: "line_no",
                    param_type: ParamType::Number,
                    required: true,
                    description: "Line number",
                },
                ParamInfo {
                    name: "column",
                    param_type: ParamType::Number,
                    required: true,
                    description: "Column number",
                },
            ],
            "Cursor position changed event",
        ));

        infos.insert("selection_changed", ActionInfo::trigger_event(
            Trigger::SelectionChanged,
            vec![
                ParamInfo {
                    name: "selected_text",
                    param_type: ParamType::String,
                    required: false,
                    description: "Selected text",
                },
            ],
            "Selection changed event",
        ));

        infos
        })
    }

    /// 所有标记为编辑类（`ActionInfo::is_update == true`）的命令名，与 [`action_infos`](Self::action_infos) 注册表一致。
    pub fn editing_command_names() -> Vec<&'static str> {
        let mut names: Vec<_> = Self::action_infos()
            .values()
            .filter(|info| info.is_update)
            .map(|info| info.command)
            .collect();
        names.sort_unstable();
        names
    }

    /// 获取动作信息
    pub fn info(&self) -> Option<ActionInfo> {
        let infos = Self::action_infos();
        infos.get(self.command.as_str()).cloned()
    }

    /// 获取动作对应的命令名（用于插件系统）
    pub fn command_name(&self) -> &str {
        &self.command
    }

    /// 从命令名和参数创建 Action（用于插件系统，使用注册表，带验证）
    pub fn from_command(command: &str, params: &std::collections::HashMap<String, serde_json::Value>) -> Result<Self, String> {
        let infos = Self::action_infos();
        if let Some(info) = infos.get(command) {
            info.create_action(params)
        } else {
            Err(format!("Unknown command: {}", command))
        }
    }

    /// 类型安全地获取字符串参数
    pub fn get_string_param(&self, name: &str) -> Result<String, String> {
        let info = self.info().ok_or_else(|| format!("Action info not found for command: {}", self.command))?;
        
        // 查找参数定义
        let param_info = info.params.iter()
            .find(|p| p.name == name)
            .ok_or_else(|| format!("Parameter '{}' not defined for command '{}'", name, self.command))?;
        
        // 检查参数类型
        match param_info.param_type {
            ParamType::String | ParamType::OptionalString => {
                match self.params.get(name) {
                    Some(v) => {
                        if let Some(s) = v.as_str() {
                            Ok(s.to_string())
                        } else {
                            Err(format!("Parameter '{}' is not a valid string", name))
                        }
                    }
                    None => {
                        if param_info.required {
                            Err(format!("Required parameter '{}' is missing", name))
                        } else {
                            Ok("".to_string()) // 可选参数返回空字符串
                        }
                    }
                }
            }
            _ => Err(format!("Parameter '{}' is not a string type", name))
        }
    }

    /// 类型安全地获取可选字符串参数
    pub fn get_optional_string_param(&self, name: &str) -> Result<Option<String>, String> {
        let info = self.info().ok_or_else(|| format!("Action info not found for command: {}", self.command))?;
        
        let param_info = info.params.iter()
            .find(|p| p.name == name)
            .ok_or_else(|| format!("Parameter '{}' not defined for command '{}'", name, self.command))?;
        
        match param_info.param_type {
            ParamType::String | ParamType::OptionalString => {
                Ok(self.params.get(name)
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()))
            }
            _ => Err(format!("Parameter '{}' is not a string type", name))
        }
    }

    /// 类型安全地获取数字参数
    pub fn get_number_param(&self, name: &str) -> Result<u64, String> {
        let info = self.info().ok_or_else(|| format!("Action info not found for command: {}", self.command))?;
        
        let param_info = info.params.iter()
            .find(|p| p.name == name)
            .ok_or_else(|| format!("Parameter '{}' not defined for command '{}'", name, self.command))?;
        
        match param_info.param_type {
            ParamType::Number => {
                self.params.get(name)
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        if param_info.required {
                            format!("Required parameter '{}' is missing or invalid", name)
                        } else {
                            format!("Parameter '{}' is invalid", name)
                        }
                    })
            }
            _ => Err(format!("Parameter '{}' is not a number type", name))
        }
    }

    /// 类型安全地获取布尔参数
    pub fn get_bool_param(&self, name: &str) -> Result<bool, String> {
        let info = self.info().ok_or_else(|| format!("Action info not found for command: {}", self.command))?;
        
        let param_info = info.params.iter()
            .find(|p| p.name == name)
            .ok_or_else(|| format!("Parameter '{}' not defined for command '{}'", name, self.command))?;
        
        match param_info.param_type {
            ParamType::Boolean => {
                match self.params.get(name) {
                    Some(v) => {
                        if let Some(b) = v.as_bool() {
                            Ok(b)
                        } else {
                            Err(format!("Parameter '{}' is not a valid boolean", name))
                        }
                    }
                    None => {
                        if param_info.required {
                            Err(format!("Required parameter '{}' is missing", name))
                        } else {
                            Ok(false) // 可选布尔参数默认为 false
                        }
                    }
                }
            }
            _ => Err(format!("Parameter '{}' is not a boolean type", name))
        }
    }

    /// 类型安全地获取可选布尔参数（带默认值）
    pub fn get_optional_bool_param(&self, name: &str, default: bool) -> Result<bool, String> {
        let info = self.info().ok_or_else(|| format!("Action info not found for command: {}", self.command))?;
        
        let param_info = info.params.iter()
            .find(|p| p.name == name)
            .ok_or_else(|| format!("Parameter '{}' not defined for command '{}'", name, self.command))?;
        
        match param_info.param_type {
            ParamType::Boolean => {
                Ok(self.params.get(name)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(default))
            }
            _ => Err(format!("Parameter '{}' is not a boolean type", name))
        }
    }

    /// 获取动作对应的快捷键（从注册表中获取）
    pub fn shortcut_key(&self) -> Option<ShortcutKey> {
        self.info().and_then(|info| info.shortcut_key.clone())
    }

    /// 获取动作对应的快捷键字符串（用于显示）
    pub fn shortcut_string(&self) -> Option<String> {
        self.shortcut_key().map(|sk| sk.to_string())
    }

    /// 创建默认的动作实例（带默认快捷键）
    pub fn copy() -> Self {
        Self::new(
            "copy".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn cut() -> Self {
        Self::new(
            "cut".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn paste() -> Self {
        Self::new(
            "paste".to_string(),
            std::collections::HashMap::new(),
        )
    }

    /// 在光标处插入给定文本（`Event::Paste` / `Event::Text` 使用；`Action::paste()` 仍从剪贴板读取）
    pub fn insert_text(text: String) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("text".to_string(), serde_json::Value::String(text));
        Self::new("insert_text".to_string(), params)
    }

    pub fn delete() -> Self {
        Self::new(
            "delete".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn backspace() -> Self {
        Self::new(
            "backspace".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn arrow_left(shift: bool) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("shift".to_string(), serde_json::Value::Bool(shift));
        Self::new("arrow_left".to_string(), params)
    }

    pub fn arrow_right(shift: bool) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("shift".to_string(), serde_json::Value::Bool(shift));
        Self::new("arrow_right".to_string(), params)
    }

    pub fn arrow_up(shift: bool) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("shift".to_string(), serde_json::Value::Bool(shift));
        Self::new("arrow_up".to_string(), params)
    }

    pub fn arrow_down(shift: bool) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("shift".to_string(), serde_json::Value::Bool(shift));
        Self::new("arrow_down".to_string(), params)
    }

    pub fn home(shift: bool) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("shift".to_string(), serde_json::Value::Bool(shift));
        Self::new("home".to_string(), params)
    }

    pub fn end(shift: bool) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("shift".to_string(), serde_json::Value::Bool(shift));
        Self::new("end".to_string(), params)
    }

    pub fn page_down() -> Self {
        Self::new(
            "page_down".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn page_up() -> Self {
        Self::new(
            "page_up".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn undo() -> Self {
        Self::new(
            "undo".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn redo() -> Self {
        Self::new(
            "redo".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn insert_tab() -> Self {
        Self::new(
            "insert_tab".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn enter(ctrl: bool) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("ctrl".to_string(), serde_json::Value::Bool(ctrl));
        Self::new("enter".to_string(), params)
    }

    pub fn select_all() -> Self {
        Self::new(
            "select_all".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn bold() -> Self {
        Self::new(
            "bold".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn italic() -> Self {
        Self::new(
            "italic".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn strikethrough() -> Self {
        Self::new(
            "strikethrough".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn code() -> Self {
        Self::new(
            "code".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn code_block() -> Self {
        Self::new(
            "code_block".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn link() -> Self {
        Self::new(
            "link".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn table() -> Self {
        Self::new(
            "table".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn heading(level: usize) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("level".to_string(), serde_json::Value::Number(level.into()));
        Self::new(
            format!("heading_{}", level),
            params,
        )
    }

    pub fn quote() -> Self {
        Self::new(
            "quote".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn unordered_list() -> Self {
        Self::new(
            "unordered_list".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn ordered_list() -> Self {
        Self::new(
            "ordered_list".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn todo_list() -> Self {
        Self::new(
            "todo_list".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn horizontal_rule() -> Self {
        Self::new(
            "horizontal_rule".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn table_delete_selected_rows() -> Self {
        Self::new(
            "table_delete_selected_rows".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn table_delete_selected_cols() -> Self {
        Self::new(
            "table_delete_selected_cols".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn table_insert_row_above() -> Self {
        Self::new(
            "table_insert_row_above".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn table_insert_row_below() -> Self {
        Self::new(
            "table_insert_row_below".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn table_insert_col_left() -> Self {
        Self::new(
            "table_insert_col_left".to_string(),
            std::collections::HashMap::new(),
        )
    }

    pub fn table_insert_col_right() -> Self {
        Self::new(
            "table_insert_col_right".to_string(),
            std::collections::HashMap::new(),
        )
    }

    // 应用级别的命令创建方法
    pub fn open_file(path: String) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("path".to_string(), serde_json::Value::String(path));
        Self::new("open_file".to_string(), params)
    }

    pub fn path_list(path: String) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("path".to_string(), serde_json::Value::String(path));
        Self::new("path_list".to_string(), params)
    }

    pub fn new_file(parent: Option<String>) -> Self {
        let mut params = std::collections::HashMap::new();
        if let Some(p) = parent {
            params.insert("parent".to_string(), serde_json::Value::String(p));
        }
        Self::new("new_file".to_string(), params)
    }

    pub fn delete_file(path: String) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("path".to_string(), serde_json::Value::String(path));
        Self::new("delete_file".to_string(), params)
    }

    pub fn rename_file(path: String) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("path".to_string(), serde_json::Value::String(path));
        Self::new("rename_file".to_string(), params)
    }

    pub fn find_replace(ctx: FindReplaceCtx) -> Self {
        ctx.to_action()
    }

    pub fn click_edit_line(line: String) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("line".to_string(), serde_json::Value::String(line));
        Self::new("click_edit_line".to_string(), params)
    }

    pub fn open_url(url_info: UrlInfo) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("url".to_string(), serde_json::Value::String(url_info.url));
        if let Some(title) = url_info.title {
            params.insert("title".to_string(), serde_json::Value::String(title));
        }
        params.insert("text".to_string(), serde_json::Value::String(url_info.text));
        params.insert("pos_line".to_string(), serde_json::Value::Number(url_info.pos.0.into()));
        params.insert("pos_col".to_string(), serde_json::Value::Number(url_info.pos.1.into()));
        Self::new("open_url".to_string(), params)
    }

    pub fn fixed_file(path: String) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("path".to_string(), serde_json::Value::String(path));
        Self::new("fixed_file".to_string(), params)
    }

    pub fn unfixed_file(path: String) -> Self {
        let mut params = std::collections::HashMap::new();
        params.insert("path".to_string(), serde_json::Value::String(path));
        Self::new("unfixed_file".to_string(), params)
    }

    // 执行函数（用于 ActionInfo）
    fn execute_copy(ctx: &mut Ctx, ui: &mut Ui, _action: &Action) {
        let text = ctx.get_selected_text();
        if !text.is_empty() {
            ui.ctx().copy_text(text);
        }
    }

    fn execute_cut(ctx: &mut Ctx, ui: &mut Ui, _action: &Action) {
        let text = ctx.get_selected_text();
        if !text.is_empty() {
            ui.ctx().copy_text(text.clone());
            ctx.delete();
        }
    }

    fn execute_paste(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        if let Some(text) = ctx.get_clipboard_text() {
            ctx.insert(text);
        }
    }

    fn execute_insert_text(ctx: &mut Ctx, _ui: &mut Ui, action: &Action) {
        match action.get_string_param("text") {
            Ok(text) => ctx.insert(text),
            Err(e) => log::error!("insert_text: {}", e),
        }
    }

    fn execute_delete(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        if ctx.is_selected() {
            ctx.delete();
        } else {
            ctx.cursor2_move_next();
            ctx.set_cursor_switch();
            ctx.delete();
        }
    }

    fn execute_backspace(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        if ctx.is_selected() {
            ctx.delete();
        } else {
            // 先尝试删除自动插入的前缀
            if ctx.backspace_auto_prefix() {
                return;
            }
            // 如果不需要删除前缀，执行正常的退格操作
            ctx.cursor2_move_prev();
            ctx.set_cursor_switch();
            ctx.delete();
        }
    }

    fn execute_arrow_left(ctx: &mut Ctx, _ui: &mut Ui, action: &Action) {
        let shift = action.params.get("shift")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        ctx.cursor2_move_prev();
        if !shift {
            ctx.set_cursor1_reset();
        }
    }

    fn execute_arrow_right(ctx: &mut Ctx, _ui: &mut Ui, action: &Action) {
        let shift = action.params.get("shift")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        ctx.cursor2_move_next();
        if !shift {
            ctx.set_cursor1_reset();
        }
    }

    fn execute_arrow_up(ctx: &mut Ctx, _ui: &mut Ui, action: &Action) {
        let shift = action.params.get("shift")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        ctx.cursor2_move_up();
        if !shift {
            ctx.set_cursor1_reset();
        }
    }

    fn execute_arrow_down(ctx: &mut Ctx, _ui: &mut Ui, action: &Action) {
        let shift = action.params.get("shift")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        ctx.cursor2_move_down();
        if !shift {
            ctx.set_cursor1_reset();
        }
    }

    fn execute_home(ctx: &mut Ctx, _ui: &mut Ui, action: &Action) {
        let shift = action.params.get("shift")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        ctx.cursor2_move_home();
        if !shift {
            ctx.set_cursor1_reset();
        }
    }

    fn execute_end(ctx: &mut Ctx, _ui: &mut Ui, action: &Action) {
        let shift = action.params.get("shift")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        ctx.cursor2_move_end();
        if !shift {
            ctx.set_cursor1_reset();
        }
    }

    fn execute_page_down(ctx: &mut Ctx, ui: &mut Ui, _action: &Action) {
        let mut rect = ui.cursor();
        rect.set_height(ctx.font_heigh());
        ctx.set_scroll_to_rect(rect);
    }

    fn execute_page_up(ctx: &mut Ctx, ui: &mut Ui, _action: &Action) {
        let mut rect = ui.cursor();
        rect.set_height(ctx.font_heigh());
        let rect = rect.translate(Vec2::new(0.0, -ctx.edit_rect().height()*2.0));
        ctx.set_scroll_to_rect(rect);
    }

    fn execute_undo(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        ctx.undo();
    }

    fn execute_redo(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        ctx.redo();
    }

    fn execute_insert_tab(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        ctx.insert_tab();
    }

    fn execute_set_expanded_text(ctx: &mut Ctx, _ui: &mut Ui, action: &Action) {
        let line_no = match action.get_number_param("line_no") {
            Ok(n) => n as usize,
            Err(e) => {
                log::error!("Failed to get line_no parameter: {}", e);
                return;
            }
        };
        
        let expanded_text = match action.get_optional_string_param("expanded_text") {
            Ok(Some(text)) => Some(text),
            Ok(None) => None,
            Err(e) => {
                log::error!("Failed to get expanded_text parameter: {}", e);
                return;
            }
        };
        
        ctx.set_expanded_text(line_no, expanded_text);
    }

    fn execute_enter(ctx: &mut Ctx, _ui: &mut Ui, action: &Action) {
        let ctrl = action.params.get("ctrl")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        ctx.enter(ctrl);
    }

    fn execute_select_all(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        ctx.set_cursors_select_all();
    }

    fn execute_bold(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::wrap_selection(ctx, "**", "**");
    }

    fn execute_italic(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::wrap_selection(ctx, "*", "*");
    }

    fn execute_strikethrough(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::wrap_selection(ctx, "~~", "~~");
    }

    fn execute_code(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::wrap_selection(ctx, "`", "`");
    }

    fn execute_code_block(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::wrap_code_block(ctx);
    }

    fn execute_link(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_link(ctx);
    }

    fn execute_table(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_table(ctx);
    }

    fn execute_heading_1(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_heading(ctx, 1);
    }

    fn execute_heading_2(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_heading(ctx, 2);
    }

    fn execute_heading_3(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_heading(ctx, 3);
    }

    fn execute_heading_4(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_heading(ctx, 4);
    }

    fn execute_heading_5(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_heading(ctx, 5);
    }

    fn execute_heading_6(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_heading(ctx, 6);
    }

    fn execute_quote(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_quote(ctx);
    }

    fn execute_unordered_list(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_list(ctx, false);
    }

    fn execute_ordered_list(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_list(ctx, true);
    }

    fn execute_todo_list(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_todo_list(ctx);
    }

    fn execute_horizontal_rule(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        Self::insert_horizontal_rule(ctx);
    }

    fn execute_table_delete_selected_rows(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        ctx.table_delete_selected_rows();
    }

    fn execute_table_delete_selected_cols(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        ctx.table_delete_selected_cols();
    }

    fn execute_table_insert_row_above(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        ctx.table_insert_row_above();
    }

    fn execute_table_insert_row_below(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        ctx.table_insert_row_below();
    }

    fn execute_table_insert_col_left(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        ctx.table_insert_col_left();
    }

    fn execute_table_insert_col_right(ctx: &mut Ctx, _ui: &mut Ui, _action: &Action) {
        ctx.table_insert_col_right();
    }

    /// 执行动作（仅编辑器级别的动作）
    pub fn execute(&self, ctx: &mut Ctx, ui: &mut Ui) {
        let _guard = ctx.merge_redo_and_undo_guard(Some(self.command.clone()));   //自动合并redo和undo命令
        
        // 统一判断 is_read_only
        if let Some(info) = self.info() {
            if info.is_update && ctx.cfg().is_read_only {
                return;
            }
        }
        
        // 对于 Heading，需要从 params 中提取 level
        if self.command.starts_with("heading_") {
            let level = self.params.get("level")
                .and_then(|v| v.as_u64())
                .map(|n| n as usize)
                .unwrap_or_else(|| {
                    self.command.strip_prefix("heading_")
                        .and_then(|s| s.parse::<usize>().ok())
                        .unwrap_or(1)
                });
            Self::insert_heading(ctx, level);
            return;
        }
        
        // 尝试使用 ActionInfo 的执行器
        if let Some(info) = self.info() {
            if let Some(executor) = info.executor {
                executor(ctx, ui, self);
                return;
            }
        }
        
        // 应用级别的命令不在这里处理
    }

    /// 包装选中文本（智能切换：如果已有格式标记则移除，否则添加）
    fn wrap_selection(ctx: &mut Ctx, prefix: &str, suffix: &str) {
        if ctx.is_selected() {
            let line_nos = ctx.get_selected_line_nos();
            if line_nos.len() > 1 {
                let nonempty: Vec<usize> = line_nos
                    .iter()
                    .copied()
                    .filter(|&ln| !ctx.get_line_text(ln).trim().is_empty())
                    .collect();
                let all_wrapped = !nonempty.is_empty()
                    && nonempty.iter().all(|&ln| {
                        let lt = ctx.get_line_text(ln);
                        let t = lt.trim();
                        t.starts_with(prefix) && t.ends_with(suffix)
                    });
                if all_wrapped {
                    for &line_no in &nonempty {
                        let line_text = ctx.get_line_text(line_no);
                        let trimmed = line_text.trim();
                        let unwrapped = trimmed
                            .strip_prefix(prefix)
                            .and_then(|s| s.strip_suffix(suffix))
                            .unwrap_or(trimmed)
                            .to_string();
                        ctx.update_line_text(line_no, unwrapped);
                    }
                } else {
                    for &line_no in &nonempty {
                        let line_text = ctx.get_line_text(line_no);
                        let trimmed = line_text.trim();
                        if trimmed.starts_with(prefix) && trimmed.ends_with(suffix) {
                            let unwrapped = trimmed
                                .strip_prefix(prefix)
                                .and_then(|s| s.strip_suffix(suffix))
                                .unwrap_or(trimmed)
                                .to_string();
                            ctx.update_line_text(line_no, unwrapped);
                        } else {
                            let wrapped = format!("{}{}{}", prefix, trimmed, suffix);
                            ctx.update_line_text(line_no, wrapped);
                        }
                    }
                }
                ctx.set_cursor1_reset();
                return;
            }
            let selected = ctx.get_selected_text();
            let trimmed = selected.trim();
            
            // 检查是否已经有格式标记
            if trimmed.starts_with(prefix) && trimmed.ends_with(suffix) {
                // 移除格式标记
                let unwrapped = trimmed
                    .strip_prefix(prefix)
                    .and_then(|s| s.strip_suffix(suffix))
                    .unwrap_or(trimmed)
                    .to_string();
                ctx.insert(unwrapped);
            } else {
                // 添加格式标记
                let wrapped = format!("{}{}{}", prefix, selected.trim(), suffix);
                ctx.insert(wrapped);
            }
        } else {
            // 如果没有选中，插入格式标记并定位光标在中间
            let text = format!("{}{}", prefix, suffix);
            ctx.insert(text);
            // 将光标移动到中间位置
            let cursor = ctx.cursor2();
            let new_column = cursor.culumn.saturating_sub(suffix.len());
            let mut new_cursor = cursor;
            new_cursor.culumn = new_column;
            ctx.set_cursor2(new_cursor);
        }
        ctx.set_cursor1_reset();
    }

    /// 插入代码块（获取选中行的完整文本，删除这些行，插入新的代码块 PghView）
    fn wrap_code_block(ctx: &mut Ctx) {
        // 检查当前行是否是代码块
        let current_line_no = ctx.cursor2().line_no;
        if ctx.is_line_type(current_line_no, PghType::CodeRow) {
            return; 
        }
        
        // 检查选中的行中是否包含代码块
        if ctx.is_selected() && ctx.has_selected_line_type(PghType::CodeRow) {
            return; 
        }
        
        if ctx.is_selected() {
            // 获取所有选中行的完整文本（包括首尾行未被选中的部分）
            let code_text_lines = ctx.get_selected_lines_full_text();
            let code_text = code_text_lines.join("\n");
            let line_nos = ctx.get_selected_line_nos();
            ctx.replace_lines_with_code_block(line_nos, &code_text);
        } else {
            // 没有选中：将当前行替换为代码块
            let current_line_no = ctx.cursor2().line_no;
            let current_line_text = ctx.get_line_text(current_line_no);
            let current_line_text = current_line_text;
            ctx.replace_lines_with_code_block(vec![current_line_no], &current_line_text);
        }
    }

    /// 插入链接
    fn insert_link(ctx: &mut Ctx) {
        if ctx.is_single_line_selected() {
            // 匹配 Markdown 链接格式：[文本](url) 或 [文本](url "title")
            let link_regex = Regex::new(r"\[[^\]]+\]\([^\)]+\)").unwrap();
            let selected = ctx.get_selected_text().trim().to_string();
            if link_regex.is_match(&selected) {
                return;
            }
            let link = format!("[{}](url)", selected);
            ctx.insert(link);
        } else if ctx.is_selected() {
            let line_nos = ctx.get_selected_line_nos();
            let link_regex = Regex::new(r"\[[^\]]+\]\([^\)]+\)").unwrap();
            let mut touched = false;
            for &line_no in &line_nos {
                let line_text = ctx.get_line_text(line_no);
                if line_text.trim().is_empty() {
                    continue;
                }
                if link_regex.is_match(line_text.trim()) {
                    continue;
                }
                let linked = format!("[{}](url)", line_text.trim());
                ctx.update_line_text(line_no, linked);
                touched = true;
            }
            if touched {
                ctx.set_cursor1_reset();
            }
        } else {
            let link = "[text](url)".to_string();
            ctx.insert(link);
        }
    }

    /// 插入表格
    fn insert_table(ctx: &mut Ctx) {
        ctx.set_cursor1_reset();
        let line_no = ctx.cursor2().line_no;
        let line_text = ctx.get_line_text(line_no);
        let table = "| ColA | ColB | ColC |\n|-----|-----|-----|\n|     |     |     |".to_string();
        if line_text.trim().is_empty() {
            ctx.insert(table);
        } else if let Some(pgh) = ctx.get_line(line_no) {
            let end = ctx.cursor_check(&pgh.end_cursor_of_line(line_no));
            ctx.set_cursor2(end);
            // 与 insert() 同理：仅移动 cursor2 会留下选区，enter/insert 会先删选区导致截断本行
            ctx.set_cursor1_reset();
            ctx.enter(false);
            ctx.insert(table);
        }
    }

    /// 插入标题（如果已经是标题，先移除旧的标题标记）
    fn insert_heading(ctx: &mut Ctx, level: usize) {
        let level = level.min(6).max(1);
        let prefix = "#".repeat(level) + " ";
        
        if ctx.is_selected() {
            let line_nos = ctx.get_selected_line_nos();
            for line_no in line_nos {
                if ctx.is_heading_line(line_no) {
                    let line_text = ctx.get_line_text(line_no);
                    let text_without_heading = Ctx::remove_heading_prefix(&line_text);
                    ctx.update_line_text(line_no, format!("{}{}", prefix, text_without_heading));
                } else {
                    let line_text = ctx.get_line_text(line_no);
                    ctx.update_line_text(line_no, format!("{}{}", prefix, line_text.trim_start()));
                }
            }
            ctx.set_cursor1_reset();
        } else {
            let line_no = ctx.cursor2().line_no;
            if ctx.is_heading_line(line_no) {
                let line_text = ctx.get_current_line_text_without_heading();
                ctx.update_line_text(line_no, format!("{}{}", prefix, line_text));
            } else {
                let line_text = ctx.get_line_text(line_no);
                ctx.update_line_text(line_no, format!("{}{}", prefix, line_text.trim_start()));
            }
        }
    }

    /// 移除引用标记（支持 \s*>\s* 模式）
    fn remove_quote_prefix(text: &str) -> String {
        let trimmed = text.trim_start();
        // 使用正则表达式匹配 \s*>\s* 模式
        let re = Regex::new(r"^\s*>\s*").unwrap();
        re.replace(trimmed, "").to_string()
    }

    /// 检查文本是否以引用标记开头（支持 \s*>\s* 模式）
    fn has_quote_prefix(text: &str) -> bool {
        let trimmed = text.trim_start();
        let re = Regex::new(r"^\s*>\s*").unwrap();
        re.is_match(trimmed)
    }

    /// 插入引用（支持多行，如果已有引用标记则移除）
    fn insert_quote(ctx: &mut Ctx) {
        let line_nos = ctx.get_selected_and_current_line_nos();
        let all_quoted = !line_nos.is_empty() && line_nos.iter().all(|&line_no| {
            let line_text = ctx.get_line_text(line_no);
            Self::has_quote_prefix(&line_text)
        });
        
        if all_quoted {
            // 移除所有行的引用标记
            for line_no in line_nos {
                let line_text = ctx.get_line_text(line_no);
                let unquoted = Self::remove_quote_prefix(&line_text);
                ctx.update_line_text(line_no, unquoted);
            }
        } else {
            // 添加引用标记
            for line_no in line_nos {
                let line_text = ctx.get_line_text(line_no);
                let text_without_quote =if Self::has_quote_prefix(&line_text) {
                    line_text
                } else {
                    let line_text = line_text.trim_start().to_string();
                    format!("> {}", line_text)
                };
                ctx.update_line_text(line_no, text_without_quote);
            }
        }
        ctx.set_cursor1_reset();
    }

    /// 检查文本是否以列表标记开头
    fn has_list_prefix(text: &str, ordered: bool) -> bool {
        let trimmed = text.trim_start();
        if ordered {
            // 检查是否以 "数字. " 开头
            trimmed.chars().next().map_or(false, |c| c.is_ascii_digit()) &&
            trimmed.chars().nth(1).map_or(false, |c| c == '.')
        } else {
            // 检查是否以 "- " 或 "* " 开头
            trimmed.starts_with("- ") || trimmed.starts_with("* ")
        }
    }

    /// 移除列表标记
    fn remove_list_prefix(text: &str, ordered: bool) -> String {
        let trimmed = text.trim_start();
        if ordered {
            // 移除 "数字. " 格式
            if let Some(pos) = trimmed.find(". ") {
                trimmed[pos + 2..].to_string()
            } else {
                trimmed.to_string()
            }
        } else {
            // 移除 "- " 或 "* " 格式
            trimmed.strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .unwrap_or(trimmed)
                .to_string()
        }
    }

    /// 插入列表（支持多行，如果已有列表标记则移除）
    fn insert_list(ctx: &mut Ctx, ordered: bool) {
        let line_nos = ctx.get_selected_and_current_line_nos();
        let all_list = !line_nos.is_empty() && line_nos.iter().all(|&line_no| {
            let line_text = ctx.get_line_text(line_no);
            Self::has_list_prefix(&line_text, ordered)
        });
        
        if all_list {
            // 移除所有行的列表标记
            for line_no in line_nos {
                let line_text = ctx.get_line_text(line_no);
                let unlisted = Self::remove_list_prefix(&line_text, ordered);
                ctx.update_line_text(line_no, unlisted);
            }
        } else {
            // 添加列表标记
            let ordered_re = Regex::new(r"^(\d+)\.\s+").unwrap();
            let mut start_num = 1usize;
            if ordered && !line_nos.is_empty() {
                let first = line_nos[0];
                if first > 0 {
                    let prev = ctx.get_line_text(first - 1);
                    if let Some(caps) = ordered_re.captures(prev.trim()) {
                        if let Ok(n) = caps[1].parse::<usize>() {
                            start_num = n + 1;
                        }
                    }
                }
            }
            for (index, line_no) in line_nos.iter().enumerate() {
                let line_text = ctx.get_line_text(*line_no);
                let num = start_num + index;
                let text_without_list = if Self::has_list_prefix(&line_text, ordered) {
                    // 如果已经有列表标记，先移除再添加，确保格式统一
                    let trimmed = Self::remove_list_prefix(&line_text, ordered);
                    if ordered {
                        format!("{}. {}", num, trimmed)
                    } else {
                        format!("- {}", trimmed)
                    }
                } else {
                    let trimmed = line_text.trim_start().to_string();
                    if ordered {
                        format!("{}. {}", num, trimmed)
                    } else {
                        format!("- {}", trimmed)
                    }
                };
                ctx.update_line_text(*line_no, text_without_list);
            }
        }
        ctx.set_cursor1_reset();
    }

    /// 检查文本是否以TODO列表标记开头（- [ ] 或 - [x]）
    fn has_todo_list_prefix(text: &str) -> bool {
        let trimmed = text.trim_start();
        // 检查是否以 "- [ ] " 或 "- [x] " 或 "- [X] " 开头
        let re = Regex::new(r"^[-*+]\s+\[[ xX]\]\s+").unwrap();
        re.is_match(trimmed)
    }

    /// 移除TODO列表标记
    fn remove_todo_list_prefix(text: &str) -> String {
        let trimmed = text.trim_start();
        // 移除 "- [ ] " 或 "- [x] " 或 "- [X] " 格式
        let re = Regex::new(r"^[-*+]\s+\[[ xX]\]\s+").unwrap();
        re.replace(trimmed, "").to_string()
    }

    /// 插入TODO列表（支持多行，如果已有TODO列表标记则移除）
    fn insert_todo_list(ctx: &mut Ctx) {
        let line_nos = ctx.get_selected_and_current_line_nos();
        let all_todo = !line_nos.is_empty() && line_nos.iter().all(|&line_no| {
            let line_text = ctx.get_line_text(line_no);
            Self::has_todo_list_prefix(&line_text)
        });
        
        if all_todo {
            // 移除所有行的TODO列表标记
            for line_no in line_nos {
                let line_text = ctx.get_line_text(line_no);
                let untodo = Self::remove_todo_list_prefix(&line_text);
                ctx.update_line_text(line_no, untodo);
            }
        } else {
            // 添加TODO列表标记
            for line_no in line_nos.iter() {
                let line_text = ctx.get_line_text(*line_no);
                let text_with_todo = if Self::has_todo_list_prefix(&line_text) {
                    line_text
                } else {
                    let trimmed = if Self::has_list_prefix(&line_text, false) {
                        Self::remove_list_prefix(&line_text, false)
                    } else {
                        line_text.trim_start().to_string()
                    };
                    format!("- [ ] {}", trimmed)
                };
                ctx.update_line_text(*line_no, text_with_todo);
            }
        }
        ctx.set_cursor1_reset();
    }

    /// 将主光标置于「当前行逻辑正文末尾」（与 `PghView::text_char_index_to_cursor(len)` / 测试里 `cursor_at_line_char` 一致）。
    fn cursor_at_logical_line_end(ctx: &Ctx, line_no: usize) -> Cursor {
        let line_text = ctx.get_line_text(line_no);
        let n = line_text.chars().count();
        if let Some(pgh) = ctx.get_line(line_no) {
            let raw = pgh.text_char_index_to_cursor(n, line_no);
            return ctx.cursor_check(&raw);
        }
        (line_no, 0, 0).into()
    }

    /// 插入水平线
    fn insert_horizontal_rule(ctx: &mut Ctx) {
        let line_no = ctx.cursor2().line_no;
        let line_text = ctx.get_line_text(line_no);
        
        // 检查当前行是否为空行（去除空白字符后为空）
        if line_text.trim().is_empty() {
            // 如果当前行是空行，直接替换为水平线
            ctx.update_line_text(line_no, "---".to_string());
        } else {
            let c = ctx.cursor2();
            let idx = ctx
                .get_line(line_no)
                .map(|p| p.cursor_to_text_char_index(&c))
                .unwrap_or(0);
            let chars: Vec<char> = line_text.chars().collect();
            let idx = idx.min(chars.len());
            let left: String = chars[..idx].iter().collect();
            // 光标在逻辑行首：整行保留，仅在正文末尾下插水平线（须用逻辑字符尾，勿用落在非 Text 段上的 end_cursor）
            if idx == 0 {
                let end = Self::cursor_at_logical_line_end(ctx, line_no);
                ctx.set_cursor2(end);
                // insert() 会先 delete_func：若仅移动了 cursor2 而 cursor1 仍在行首，会形成选区并删掉正文
                ctx.set_cursor1_reset();
                ctx.insert("\n---".to_string());
            } else {
                let right: String = chars[idx..].iter().collect();
                let right = right.trim_start().to_string();
                ctx.update_line_text(line_no, left);
                let end = Self::cursor_at_logical_line_end(ctx, line_no);
                ctx.set_cursor2(end);
                ctx.set_cursor1_reset();
                if right.is_empty() {
                    ctx.insert("\n---".to_string());
                } else {
                    ctx.insert(format!("\n---\n{}", right));
                }
            }
        }
        ctx.set_cursor1_reset();
    }
}
