use crate::store::Store;
use crate::util::show_save_file_dialog;
use crate::medit::TocEntry;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
/// 通过系统「另存为」对话框选择的新文件路径（取消则为 null）
pub const PARAM_NEW_FILE_PATH: &str = "new_file_path";
/// 当前文件路径（仅当当前标签是文件时有值）
pub const PARAM_CURRENT_FILE_PATH: &str = "current_file_path";
/// 当前文件名（仅当当前标签是文件时有值）
pub const PARAM_CURRENT_FILE_NAME: &str = "current_file_name";
/// 当前文件扩展名（仅当当前标签是文件时有值）
pub const PARAM_CURRENT_FILE_EXT: &str = "current_file_ext";
/// 当前文件主名（不含扩展名，仅当当前标签是文件时有值）
pub const PARAM_CURRENT_FILE_STEM: &str = "current_file_stem";
/// 当前标签路径（文件或笔记）
pub const PARAM_CURRENT_PATH: &str = "current_path";
/// 当前标签名称（文件名或笔记名）
pub const PARAM_CURRENT_NAME: &str = "current_name";
/// 当前标签是否是文件
pub const PARAM_CURRENT_IS_FILE: &str = "current_is_file";
/// 当前标签是否是笔记
pub const PARAM_CURRENT_IS_NOTE: &str = "current_is_note";
/// 当前笔记路径（仅当当前标签是笔记时有值）
pub const PARAM_CURRENT_NOTE_PATH: &str = "current_note_path";
/// 当前笔记名称（仅当当前标签是笔记时有值）
pub const PARAM_CURRENT_NOTE_NAME: &str = "current_note_name";
/// 当前大纲标题（当前光标所属标题）
pub const PARAM_CURRENT_OUTLINE_NAME: &str = "current_outline_name";
/// 当前大纲路径（按标题层级拼接）
pub const PARAM_CURRENT_OUTLINE_PATH: &str = "current_outline_path";
/// 当前大纲内容（从当前标题到下一个同级或更高级标题之前）
pub const PARAM_CURRENT_OUTLINE_CONTENT: &str = "current_outline_content";
/// 当前大纲下合并后的表格（仅聚合子标题范围内 TableRow，忽略普通文本）
pub const PARAM_CURRENT_OUTLINE_MERGED_TABLE: &str = "current_outline_merged_table";
/// 当前笔记工作目录
pub const PARAM_NOTE_WORK_DIR: &str = "note_work_dir";
/// 当前笔记配置文件路径
pub const PARAM_NOTE_CONFIG_FILE: &str = "note_config_file";
/// 当前笔记图片目录路径
pub const PARAM_NOTE_IMAGE_DIR: &str = "note_image_dir";
/// 当前应用语言代码（如 zh-CN / en-US）
pub const PARAM_APP_LANGUAGE: &str = "app_language";
/// 当前已打开标签总数
pub const PARAM_OPENED_TOTAL_COUNT: &str = "opened_total_count";
/// 当前已打开笔记数
pub const PARAM_OPENED_NOTE_COUNT: &str = "opened_note_count";
/// 当前已打开文件数
pub const PARAM_OPENED_FILE_COUNT: &str = "opened_file_count";
/// 光标所在行号（1-based）
pub const PARAM_EDITOR_CURSOR_LINE_NO: &str = "editor_cursor_line_no";
/// 光标所在行号（0-based）
pub const PARAM_EDITOR_CURSOR_LINE_INDEX: &str = "editor_cursor_line_index";
/// 光标所在段索引（segment）
pub const PARAM_EDITOR_CURSOR_SEGMENT: &str = "editor_cursor_segment";
/// 光标所在列号（column）
pub const PARAM_EDITOR_CURSOR_COLUMN: &str = "editor_cursor_column";
/// 当前编辑器总行数
pub const PARAM_EDITOR_LINE_COUNT: &str = "editor_line_count";
/// 当前行文本
pub const PARAM_EDITOR_CURRENT_LINE_TEXT: &str = "editor_current_line_text";
/// 当前行文本（若为标题则去掉 # 前缀）
pub const PARAM_EDITOR_CURRENT_LINE_TEXT_PLAIN: &str = "editor_current_line_text_plain";
/// 是否存在选区
pub const PARAM_EDITOR_HAS_SELECTION: &str = "editor_has_selection";
/// 选区文本
pub const PARAM_EDITOR_SELECTED_TEXT: &str = "editor_selected_text";
/// 选区行数
pub const PARAM_EDITOR_SELECTED_LINE_COUNT: &str = "editor_selected_line_count";
/// 选区行号列表（1-based）
pub const PARAM_EDITOR_SELECTED_LINE_NOS: &str = "editor_selected_line_nos";
/// 选区行完整文本列表
pub const PARAM_EDITOR_SELECTED_LINES_TEXT: &str = "editor_selected_lines_text";
/// 光标所在行是否是表格行
pub const PARAM_EDITOR_IS_TABLE_LINE: &str = "editor_is_table_line";
/// 表格当前逻辑行（1-based，仅在表格内有值）
pub const PARAM_EDITOR_TABLE_ROW_NO: &str = "editor_table_row_no";
/// 表格当前逻辑列（1-based，仅在表格内有值）
pub const PARAM_EDITOR_TABLE_COL_NO: &str = "editor_table_col_no";
/// 当前表格总行数（仅在表格内有值）
pub const PARAM_EDITOR_TABLE_ROW_COUNT: &str = "editor_table_row_count";
/// 当前表格总列数（仅在表格内有值）
pub const PARAM_EDITOR_TABLE_COL_COUNT: &str = "editor_table_col_count";
/// 当前表格块内容（markdown 文本，仅在表格内有值）
pub const PARAM_EDITOR_TABLE_CONTENT: &str = "editor_table_content";
/// 当前表格块逐行内容（仅在表格内有值）
pub const PARAM_EDITOR_TABLE_LINES: &str = "editor_table_lines";

/// 常用、受支持的动态参数名（用于统一维护）
/// `new_file_path` 不在此列表中：须通过「另存为」对话框取值，仅在模板中出现 `$new_file_path` 时惰性解析。
pub const COMMON_SUPPORTED_PARAM_NAMES: &[&str] = &[
    PARAM_CURRENT_FILE_PATH,
    PARAM_CURRENT_FILE_NAME,
    PARAM_CURRENT_FILE_EXT,
    PARAM_CURRENT_FILE_STEM,
    PARAM_CURRENT_PATH,
    PARAM_CURRENT_NAME,
    PARAM_CURRENT_IS_FILE,
    PARAM_CURRENT_IS_NOTE,
    PARAM_CURRENT_NOTE_PATH,
    PARAM_CURRENT_NOTE_NAME,
    PARAM_CURRENT_OUTLINE_NAME,
    PARAM_CURRENT_OUTLINE_PATH,
    PARAM_CURRENT_OUTLINE_CONTENT,
    PARAM_CURRENT_OUTLINE_MERGED_TABLE,
    PARAM_NOTE_WORK_DIR,
    PARAM_NOTE_CONFIG_FILE,
    PARAM_NOTE_IMAGE_DIR,
    PARAM_APP_LANGUAGE,
    PARAM_OPENED_TOTAL_COUNT,
    PARAM_OPENED_NOTE_COUNT,
    PARAM_OPENED_FILE_COUNT,
    PARAM_EDITOR_CURSOR_LINE_NO,
    PARAM_EDITOR_CURSOR_LINE_INDEX,
    PARAM_EDITOR_CURSOR_SEGMENT,
    PARAM_EDITOR_CURSOR_COLUMN,
    PARAM_EDITOR_LINE_COUNT,
    PARAM_EDITOR_CURRENT_LINE_TEXT,
    PARAM_EDITOR_CURRENT_LINE_TEXT_PLAIN,
    PARAM_EDITOR_HAS_SELECTION,
    PARAM_EDITOR_SELECTED_TEXT,
    PARAM_EDITOR_SELECTED_LINE_COUNT,
    PARAM_EDITOR_SELECTED_LINE_NOS,
    PARAM_EDITOR_SELECTED_LINES_TEXT,
    PARAM_EDITOR_IS_TABLE_LINE,
    PARAM_EDITOR_TABLE_ROW_NO,
    PARAM_EDITOR_TABLE_COL_NO,
    PARAM_EDITOR_TABLE_ROW_COUNT,
    PARAM_EDITOR_TABLE_COL_COUNT,
    PARAM_EDITOR_TABLE_CONTENT,
    PARAM_EDITOR_TABLE_LINES,
];

fn current_edit_ctx<'a>(store: &'a Store) -> Option<&'a crate::medit::Ctx> {
    let current_cur = store.note_space.get_current_cur()?;
    store.ectx_map.get(&current_cur)
}

fn current_table_line_range(ctx: &crate::medit::Ctx) -> Option<(usize, usize)> {
    let cursor = ctx.cursor2();
    let line_no = cursor.line_no;
    let line = ctx.get_line(line_no)?;
    if line.is_table_row() {
        ctx.table_row_block_range(line_no)
    } else {
        None
    }
}

fn current_outline_entry<'a>(ctx: &'a crate::medit::Ctx) -> Option<&'a TocEntry> {
    ctx.toc_entry_nearest_at_or_before(ctx.cursor2().line_no)
}

fn current_outline_path(ctx: &crate::medit::Ctx, current: &TocEntry) -> String {
    let mut stack: Vec<&TocEntry> = Vec::new();
    for entry in ctx
        .toc_entries()
        .iter()
        .take_while(|entry| entry.line_no <= current.line_no)
    {
        while stack
            .last()
            .is_some_and(|last| last.level >= entry.level)
        {
            stack.pop();
        }
        stack.push(entry);
    }
    stack
        .into_iter()
        .map(|entry| entry.title.as_str())
        .collect::<Vec<_>>()
        .join(" / ")
}

/// 通过系统「另存为」对话框得到的目标路径；取消则 `None`。
fn new_file_path_value(store: &Store) -> Option<Value> {
    let current_cur = store.note_space.get_current_cur();
    let default_name = current_cur
        .as_ref()
        .map(|cur| cur.name())
        .unwrap_or_else(|| "Untitled".to_string());
    let parent_dir: Option<PathBuf> = current_cur.as_ref().and_then(|cur| {
        let p = cur.path();
        Path::new(&p)
            .parent()
            .filter(|d| d.is_dir())
            .map(|d| d.to_path_buf())
    });
    let initial_dir = parent_dir
        .as_ref()
        .map(|pb| pb.as_path())
        .or_else(|| {
            let wd = store.note_space.work_dir();
            wd.is_dir().then_some(wd)
        });
    show_save_file_dialog(&default_name, initial_dir).map(Value::String)
}

fn current_outline_content(ctx: &crate::medit::Ctx, current: &TocEntry) -> String {
    let entries = ctx.toc_entries();
    let end_line = entries
        .iter()
        .find(|entry| entry.line_no > current.line_no && entry.level <= current.level)
        .map(|entry| entry.line_no.saturating_sub(1))
        .unwrap_or_else(|| ctx.line_num().saturating_sub(1));

    if current.line_no > end_line {
        return String::new();
    }

    let start: crate::medit::Cursor = (current.line_no, 0, 0).into();
    let end_segment = ctx
        .get_line_pghview(end_line)
        .map(|view| view.max_segment())
        .unwrap_or(0);
    let end: crate::medit::Cursor = (end_line, end_segment, usize::MAX).into();
    ctx.get_text_by_cursor_range(start, end)
}

fn value_by_param_name(param_name: &str, store: &Store) -> Option<Value> {
    let current_cur = store.note_space.get_current_cur();

    match param_name {
        PARAM_CURRENT_FILE_PATH => current_cur
            .filter(|cur| cur.is_file())
            .map(|cur| Value::String(cur.path())),
        PARAM_CURRENT_FILE_NAME => current_cur
            .filter(|cur| cur.is_file())
            .map(|cur| Value::String(cur.name())),
        PARAM_CURRENT_FILE_EXT => current_cur
            .filter(|cur| cur.is_file())
            .and_then(|cur| {
                Path::new(&cur.path())
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| Value::String(ext.to_string()))
            }),
        PARAM_CURRENT_FILE_STEM => current_cur
            .filter(|cur| cur.is_file())
            .and_then(|cur| {
                Path::new(&cur.path())
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .map(|stem| Value::String(stem.to_string()))
            }),
        PARAM_CURRENT_PATH => current_cur.map(|cur| Value::String(cur.path())),
        PARAM_CURRENT_NAME => current_cur.map(|cur| Value::String(cur.name())),
        PARAM_CURRENT_IS_FILE => Some(Value::Bool(
            current_cur.as_ref().is_some_and(|cur| cur.is_file()),
        )),
        PARAM_CURRENT_IS_NOTE => Some(Value::Bool(
            current_cur.as_ref().is_some_and(|cur| cur.is_note()),
        )),
        PARAM_CURRENT_NOTE_PATH => current_cur
            .filter(|cur| cur.is_note())
            .map(|cur| Value::String(cur.path())),
        PARAM_CURRENT_NOTE_NAME => current_cur
            .filter(|cur| cur.is_note())
            .map(|cur| Value::String(cur.name())),
        PARAM_CURRENT_OUTLINE_NAME => current_edit_ctx(store).and_then(|ctx| {
            current_outline_entry(ctx).map(|entry| Value::String(entry.title.clone()))
        }),
        PARAM_CURRENT_OUTLINE_PATH => current_edit_ctx(store).and_then(|ctx| {
            current_outline_entry(ctx)
                .map(|entry| Value::String(current_outline_path(ctx, entry)))
        }),
        PARAM_CURRENT_OUTLINE_CONTENT => current_edit_ctx(store).and_then(|ctx| {
            current_outline_entry(ctx)
                .map(|entry| Value::String(current_outline_content(ctx, entry)))
        }),
        PARAM_CURRENT_OUTLINE_MERGED_TABLE => current_edit_ctx(store).and_then(|ctx| {
            ctx.current_outline_merged_table().map(Value::String)
        }),
        PARAM_NOTE_WORK_DIR => Some(Value::String(
            store
                .note_space
                .work_dir()
                .to_string_lossy()
                .replace('\\', "/"),
        )),
        PARAM_NOTE_CONFIG_FILE => Some(Value::String(
            store.note_space.config_file().replace('\\', "/"),
        )),
        PARAM_NOTE_IMAGE_DIR => Some(Value::String(store.note_space.image_path())),
        PARAM_APP_LANGUAGE => Some(Value::String(store.config.language.clone())),
        PARAM_OPENED_TOTAL_COUNT => Some(Value::from(store.opened_files.all().len() as u64)),
        PARAM_OPENED_NOTE_COUNT => Some(Value::from(store.opened_files.notes().len() as u64)),
        PARAM_OPENED_FILE_COUNT => Some(Value::from(store.opened_files.files().len() as u64)),
        PARAM_EDITOR_CURSOR_LINE_NO => current_edit_ctx(store).map(|ctx| {
            let line_no = ctx.cursor2().line_no + 1;
            Value::from(line_no as u64)
        }),
        PARAM_EDITOR_CURSOR_LINE_INDEX => {
            current_edit_ctx(store).map(|ctx| Value::from(ctx.cursor2().line_no as u64))
        }
        PARAM_EDITOR_CURSOR_SEGMENT => {
            current_edit_ctx(store).map(|ctx| Value::from(ctx.cursor2().segment as u64))
        }
        PARAM_EDITOR_CURSOR_COLUMN => {
            current_edit_ctx(store).map(|ctx| Value::from(ctx.cursor2().culumn as u64))
        }
        PARAM_EDITOR_LINE_COUNT => {
            current_edit_ctx(store).map(|ctx| Value::from(ctx.line_num() as u64))
        }
        PARAM_EDITOR_CURRENT_LINE_TEXT => current_edit_ctx(store).map(|ctx| {
            let line_no = ctx.cursor2().line_no;
            Value::String(ctx.get_line_text(line_no))
        }),
        PARAM_EDITOR_CURRENT_LINE_TEXT_PLAIN => current_edit_ctx(store)
            .map(|ctx| Value::String(ctx.get_current_line_text_without_heading())),
        PARAM_EDITOR_HAS_SELECTION => {
            current_edit_ctx(store).map(|ctx| Value::Bool(ctx.is_selected()))
        }
        PARAM_EDITOR_SELECTED_TEXT => {
            current_edit_ctx(store).map(|ctx| Value::String(ctx.get_selected_text()))
        }
        PARAM_EDITOR_SELECTED_LINE_COUNT => current_edit_ctx(store).map(|ctx| {
            let count = ctx.get_selected_and_current_line_nos().len();
            Value::from(count as u64)
        }),
        PARAM_EDITOR_SELECTED_LINE_NOS => current_edit_ctx(store).map(|ctx| {
            let line_nos = ctx
                .get_selected_and_current_line_nos()
                .into_iter()
                .map(|line_no| Value::from((line_no + 1) as u64))
                .collect();
            Value::Array(line_nos)
        }),
        PARAM_EDITOR_SELECTED_LINES_TEXT => current_edit_ctx(store).map(|ctx| {
            let lines = if ctx.is_selected() {
                ctx.get_selected_lines_full_text()
            } else {
                vec![ctx.get_line_text(ctx.cursor2().line_no)]
            };
            Value::Array(lines.into_iter().map(Value::String).collect())
        }),
        PARAM_EDITOR_IS_TABLE_LINE => current_edit_ctx(store).map(|ctx| {
            let line_no = ctx.cursor2().line_no;
            let is_table = ctx.get_line(line_no).is_some_and(|line| line.is_table_like());
            Value::Bool(is_table)
        }),
        PARAM_EDITOR_TABLE_ROW_NO => current_edit_ctx(store).and_then(|ctx| {
            ctx.table_cursor_logical_cell()
                .map(|(row, _)| Value::from((row + 1) as u64))
        }),
        PARAM_EDITOR_TABLE_COL_NO => current_edit_ctx(store).and_then(|ctx| {
            ctx.table_cursor_logical_cell()
                .map(|(_, col)| Value::from((col + 1) as u64))
        }),
        PARAM_EDITOR_TABLE_ROW_COUNT => current_edit_ctx(store).and_then(|ctx| {
            let cursor_line = ctx.cursor2().line_no;
            let table_info = ctx.table_info_of_line(cursor_line)?;
            let row_count = table_info.logical_row_count_for_ui();
            Some(Value::from(row_count as u64))
        }),
        PARAM_EDITOR_TABLE_COL_COUNT => current_edit_ctx(store).and_then(|ctx| {
            let cursor_line = ctx.cursor2().line_no;
            let table_info = ctx.table_info_of_line(cursor_line)?;
            Some(Value::from(table_info.col_count as u64))
        }),
        PARAM_EDITOR_TABLE_CONTENT => current_edit_ctx(store).and_then(|ctx| {
            let (start, end) = current_table_line_range(ctx)?;
            let lines: Vec<String> = (start..=end).map(|line_no| ctx.get_line_text(line_no)).collect();
            Some(Value::String(lines.join("\n")))
        }),
        PARAM_EDITOR_TABLE_LINES => current_edit_ctx(store).and_then(|ctx| {
            let (start, end) = current_table_line_range(ctx)?;
            let lines: Vec<Value> = (start..=end)
                .map(|line_no| Value::String(ctx.get_line_text(line_no)))
                .collect();
            Some(Value::Array(lines))
        }),
        _ => None,
    }
}

/// 基于 Store 上下文构建“系统预置动态参数池”。
fn supported_params(store: &Store) -> HashMap<String, Value> {
    let mut params = HashMap::new();
    for &name in COMMON_SUPPORTED_PARAM_NAMES {
        let value = value_by_param_name(name, store).unwrap_or(Value::Null);
        params.insert(name.to_string(), value);
    }
    params
}

/// 根据参数名解析值（优先直接命中；否则走常用别名）。
pub fn resolve_param_by_name(
    param_name: &str,
    supported_params: &HashMap<String, Value>,
    store: &Store,
) -> Option<Value> {
    if param_name == PARAM_NEW_FILE_PATH {
        return new_file_path_value(store);
    }
    if let Some(v) = supported_params.get(param_name) {
        return Some(v.clone());
    }
    match param_name {
        // 常用别名：插件命令参数 `path` 默认映射到 `current_file_path`。
        "path" => supported_params
            .get(PARAM_CURRENT_PATH)
            .cloned()
            .or_else(|| supported_params.get(PARAM_CURRENT_FILE_PATH).cloned()),
        "file_path" => supported_params.get(PARAM_CURRENT_FILE_PATH).cloned(),
        "note_path" => supported_params.get(PARAM_CURRENT_NOTE_PATH).cloned(),
        "name" => supported_params.get(PARAM_CURRENT_NAME).cloned(),
        "file_name" => supported_params.get(PARAM_CURRENT_FILE_NAME).cloned(),
        "note_name" => supported_params.get(PARAM_CURRENT_NOTE_NAME).cloned(),
        "work_dir" => supported_params.get(PARAM_NOTE_WORK_DIR).cloned(),
        "language" => supported_params.get(PARAM_APP_LANGUAGE).cloned(),
        "line" | "line_no" => supported_params.get(PARAM_EDITOR_CURSOR_LINE_NO).cloned(),
        "line_text" => supported_params.get(PARAM_EDITOR_CURRENT_LINE_TEXT).cloned(),
        "selected_text" => supported_params.get(PARAM_EDITOR_SELECTED_TEXT).cloned(),
        "table_content" | "table_markdown" => {
            supported_params.get(PARAM_EDITOR_TABLE_CONTENT).cloned()
        }
        "table_lines" => supported_params.get(PARAM_EDITOR_TABLE_LINES).cloned(),
        "table_col" | "table_col_no" => supported_params.get(PARAM_EDITOR_TABLE_COL_NO).cloned(),
        "table_row" | "table_row_no" => supported_params.get(PARAM_EDITOR_TABLE_ROW_NO).cloned(),
        "table_col_count" => supported_params.get(PARAM_EDITOR_TABLE_COL_COUNT).cloned(),
        "table_row_count" => supported_params.get(PARAM_EDITOR_TABLE_ROW_COUNT).cloned(),
        _ => None,
    }
}

/// 解析单个参数模板值。
pub fn resolve_param_value(
    param_name: &str,
    value: &Value,
    supported_params: &HashMap<String, Value>,
    store: &Store,
) -> Value {
    match value {
        Value::String(s) => {
            if let Some(key) = s.strip_prefix('$') {
                if key == PARAM_NEW_FILE_PATH {
                    return new_file_path_value(store).unwrap_or(Value::Null);
                }
                return supported_params
                    .get(key)
                    .cloned()
                    .unwrap_or(Value::String(s.clone()));
            }
            Value::String(s.clone())
        }
        Value::Null => {
            resolve_param_by_name(param_name, supported_params, store).unwrap_or(Value::Null)
        }
        Value::Array(arr) => Value::Array(
            arr.iter()
                .map(|v| resolve_param_value(param_name, v, supported_params, store))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), resolve_param_value(k, v, supported_params, store)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

/// 解析整组参数模板（来自 desc.json 的 context_menus.params）。
/// 所有系统动态参数取值都在本模块内部完成。
pub fn resolve_params(params_template: &Map<String, Value>, store: &Store) -> HashMap<String, Value> {
    let supported_params = supported_params(store);

    params_template
        .iter()
        .map(|(k, v)| (k.clone(), resolve_param_value(k, v, &supported_params, store)))
        .collect()
}
