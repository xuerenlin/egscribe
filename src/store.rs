use egscribe_sitter as sitter;
use crate::space::{UniFile, NoteSpace};
use crate::medit::{Action, Ctx, FindCmd, FindReplaceCtx, Cursor, Trigger, cfg::HeightMode};
use crate::medit::ctxmenu::EditorPluginMenuEntry;
use crate::medit::pgh::TableFrameStyle;
use crate::find::FindWindow;
use crate::util::encoding::{EncodingManager, Charset, FileEncoding, LineEnding};
use crate::util::{open_url, show_save_file_dialog, delete_swap, is_untitled_path, resolve_content_on_open, write_swap};
use crate::config::Config;
use crate::i18n;
use crate::plugin::dynamic_params;
use crate::plugin::manager::UiNotification;
use crate::plugin::PluginManager;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::usize;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn path_is_markdown_file(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown")
        })
}

/// Manager for the order of opened files
/// Ensures notes are before files
pub struct OpenedFilesManager {
    /// All opened files (notes first, files after)
    order: Vec<UniFile>,
}

impl OpenedFilesManager {
    pub fn new() -> Self {
        Self {
            order: Vec::new(),
        }
    }

    /// Get all opened files (in order)
    pub fn all(&self) -> &[UniFile] {
        &self.order
    }

    /// Get all notes
    pub fn notes(&self) -> Vec<&UniFile> {
        self.order.iter().filter(|f| f.is_note()).collect()
    }

    /// Get all notes sorted by name
    pub fn notes_sorted(&self) -> Vec<UniFile> {
        let mut notes: Vec<UniFile> = self.order.iter()
            .filter(|f| f.is_note())
            .cloned()
            .collect();
        notes.sort_by(|a, b| a.name().cmp(&b.name()));
        notes
    }

    /// Get all files
    pub fn files(&self) -> Vec<&UniFile> {
        self.order.iter().filter(|f| f.is_file()).collect()
    }

    /// Get all files sorted by name
    pub fn files_sorted(&self) -> Vec<UniFile> {
        let mut files: Vec<UniFile> = self.order.iter()
            .filter(|f| f.is_file())
            .cloned()
            .collect();
        files.sort_by(|a, b| a.name().cmp(&b.name()));
        files
    }

    /// Find the position of a file in the order
    pub fn position(&self, file: &UniFile) -> Option<usize> {
        self.order.iter().position(|f| f == file)
    }

    /// Check if a file is already opened
    pub fn contains(&self, file: &UniFile) -> bool {
        self.position(file).is_some()
    }

    /// Add a file to the order list
    /// If the file already exists, move it to the specified position
    /// Notes are automatically placed before files
    pub fn add(&mut self, file: UniFile, after: Option<&UniFile>) {
        let file_is_note = file.is_note();
        
        // If the file already exists
        if let Some(existing_pos) = self.position(&file) {
            // If after is the file itself, no need to move
            if let Some(after_file) = after {
                if after_file == &file {
                    return; // File is already in the correct position, no need to move
                }
            }
            // Remove from old position
            self.order.remove(existing_pos);
        }

        let insert_pos = if let Some(after_file) = after {
            // If after is specified, find its position
            if let Some(after_pos) = self.position(after_file) {
                let after_is_note = after_file.is_note();
                
                if file_is_note && after_is_note {
                    // Both are notes, insert after (but not beyond notes region)
                    let last_note_pos = self.order.iter().rposition(|f| f.is_note())
                        .unwrap_or(0);
                    (after_pos + 1).min(last_note_pos + 1)
                } else if !file_is_note && !after_is_note {
                    // Both are files, insert after (but within files region)
                    let first_file_pos = self.order.iter().position(|f| f.is_file())
                        .unwrap_or(self.order.len());
                    (after_pos + 1).max(first_file_pos)
                } else if file_is_note && !after_is_note {
                    // Note cannot be inserted after file, insert at end of all notes
                    self.order.iter().rposition(|f| f.is_note())
                        .map(|p| p + 1)
                        .unwrap_or(0)
                } else {
                    // File cannot be inserted before note, insert at start of all files
                    self.order.iter().position(|f| f.is_file())
                        .unwrap_or(self.order.len())
                }
            } else {
                // After file doesn't exist, insert based on type
                if file_is_note {
                    // Note: insert at end of all notes
                    self.order.iter().rposition(|f| f.is_note())
                        .map(|p| p + 1)
                        .unwrap_or(0)
                } else {
                    // File: insert at end of all files
                    self.order.len()
                }
            }
        } else {
            // No after specified, insert based on type
            if file_is_note {
                // Note: insert at end of all notes
                self.order.iter().rposition(|f| f.is_note())
                    .map(|p| p + 1)
                    .unwrap_or(0)
            } else {
                // File: insert at end of all files
                self.order.len()
            }
        };

        self.order.insert(insert_pos, file);
    }

    /// Remove a file
    pub fn remove(&mut self, file: &UniFile) -> bool {
        if let Some(pos) = self.position(file) {
            self.order.remove(pos);
            true
        } else {
            false
        }
    }

    /// Move a file to a new position
    pub fn move_to(&mut self, file: &UniFile, target_pos: usize) -> bool {
        if let Some(source_pos) = self.position(file) {
            if source_pos == target_pos {
                return true; // Already at target position
            }

            let file = self.order.remove(source_pos);
            
            // Calculate new target position (if source is before target, target position needs to be decremented by 1)
            let new_target_pos = if source_pos < target_pos {
                target_pos - 1
            } else {
                target_pos
            };
            
            // Ensure not out of bounds and conforms to the rule: notes before files
            let file_is_note = file.is_note();
            let final_pos = if file_is_note {
                // Note cannot be moved to files region
                new_target_pos.min(
                    self.order.iter().position(|f| f.is_file())
                        .unwrap_or(self.order.len())
                )
            } else {
                // File cannot be moved to notes region
                new_target_pos.max(
                    self.order.iter().rposition(|f| f.is_note())
                        .map(|p| p + 1)
                        .unwrap_or(0)
                )
            }.min(self.order.len());
            
            self.order.insert(final_pos, file);
            true
        } else {
            false
        }
    }

    /// Update a file (for scenarios like renaming)
    pub fn update(&mut self, old_file: &UniFile, new_file: UniFile) -> bool {
        if let Some(pos) = self.position(old_file) {
            self.order[pos] = new_file;
            true
        } else {
            false
        }
    }

    /// Clean up files not in ectx_map
    #[allow(dead_code)]
    pub fn cleanup(&mut self, ectx_map: &HashMap<UniFile, Ctx>) {
        self.order.retain(|f| ectx_map.contains_key(f));
    }
}


pub struct ToolBarInfo {
    pub width: Option<f32>,
    pub is_show_bottom: bool,
}

#[derive(Clone)]
pub struct NonTextFilePrompt {
    pub file_path: String,
    pub reason: String,
}

struct AsyncOpenTask {
    file_path: String,
    started_at: Instant,
    rx: Receiver<AsyncOpenResult>,
}

enum AsyncOpenResult {
    Loaded {
        file_path: String,
        text: String,
        parse_markdown: bool,
        loaded_from_swap: bool,
    },
    NonText {
        file_path: String,
        reason: String,
    },
    Error {
        file_path: String,
        reason: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenExecutionsViewRequest {
    Tasks,
    Notifications,
}

impl ToolBarInfo {
    pub fn default() -> Self {
        Self {
            width: None,
            is_show_bottom: false,
        }
    }
}

pub struct Store {
    pub config: Config,
    pub ectx_map: HashMap<UniFile, Ctx>,
    pub note_space: NoteSpace,
    pub tool_bar_info: ToolBarInfo,
    pub find_window: FindWindow,
    pub encoding_manager: EncodingManager,
    /// Manager for the order of opened files (notes before files)
    pub opened_files: OpenedFilesManager,
    /// Last auto-save time (Unix timestamp in seconds)
    last_auto_save_time: u64,
    /// Plugin manager
    pub plugin_manager: PluginManager,
    /// Prompt shown when opening a likely non-text file
    pending_non_text_file_prompt: Option<NonTextFilePrompt>,
    /// Pending async open requests
    pending_open_requests: VecDeque<String>,
    /// Currently running async open task
    active_open_task: Option<AsyncOpenTask>,
    /// Pending UI notifications generated by plugin executions
    pub ui_notifications: VecDeque<UiNotification>,
    /// Recent notifications for UI views
    recent_notifications: VecDeque<UiNotification>,
    /// Next auto-increment id for app-generated notifications
    next_app_notification_id: u64,
    /// Request to open executions view in side panel
    open_executions_view_requested: Option<OpenExecutionsViewRequest>,
}

impl Store {
    pub fn default() -> Self {
        // 先确定工作目录，再让插件目录跟随工作目录（work_dir 同级）
        let note_space = NoteSpace::new();
        let plugin_dir = note_space
            .work_dir()
            .parent()
            .map(|p| p.join("plugins"))
            .unwrap_or_else(|| PathBuf::from("./plugins"));
        
        // 创建插件目录（如果不存在）
        if !plugin_dir.exists() {
            let _ = std::fs::create_dir_all(&plugin_dir);
        }
        
        // 创建插件管理器
        let mut plugin_manager = PluginManager::new(plugin_dir);
        
        // 加载插件
        if let Err(e) = plugin_manager.load_plugins() {
            log::warn!("Failed to load plugins: {}", e);
        }
        
        let mut store = Self {
            ectx_map: HashMap::new(),
            note_space,
            config: Config::default(),
            tool_bar_info: ToolBarInfo::default(),
            find_window: FindWindow::new(),
            encoding_manager: EncodingManager::new(),
            opened_files: OpenedFilesManager::new(),
            last_auto_save_time: 0,
            plugin_manager,
            pending_non_text_file_prompt: None,
            pending_open_requests: VecDeque::new(),
            active_open_task: None,
            ui_notifications: VecDeque::new(),
            recent_notifications: VecDeque::new(),
            next_app_notification_id: 1,
            open_executions_view_requested: None,
        };
        store.config_restore();
        // Sync i18n language from config
        i18n::set_language_code(&store.config.language);
        // Initialize last_auto_save_time
        store.last_auto_save_time = store.current_timestamp();
        store
    }
    
    /// 处理插件消息和命令
    pub fn handle_plugin_messages(&mut self) {
        // 处理插件消息，获取插件发送的命令
        let commands = self.plugin_manager.handle_messages();
        let notifications = self.plugin_manager.take_ui_notifications();
        for item in notifications {
            self.push_ui_notification(item);
        }
        
        // 执行插件命令
        for cmd in commands {
            self.execute_cmd(cmd);
        }
    }

    /// 获取编辑器右键菜单中的插件命令动作
    pub fn editor_plugin_context_menu_actions(&self) -> Vec<EditorPluginMenuEntry> {
        self.plugin_manager
            .editor_context_menu_items()
            .into_iter()
            .map(|item| EditorPluginMenuEntry {
                text: item.name,
                action: Action::execute_plugin_command(item.plugin_id, item.command, item.params),
                supports_batch_concurrent: item.supports_batch_concurrent,
            })
            .collect()
    }

    pub fn latest_notification(&mut self) -> Option<&UiNotification> {
        if self.recent_notifications.is_empty() {
            return None;
        }
        let idx = self.recent_notifications.len() - 1;
        if !self.recent_notifications[idx].ensure_status_shown_at() {
            return None;
        }
        self.recent_notifications.back()
    }

    fn push_ui_notification(&mut self, notification: UiNotification) {
        self.ui_notifications.push_back(notification.clone());
        if self.ui_notifications.len() > 100 {
            self.ui_notifications.pop_front();
        }
        self.recent_notifications.push_back(notification);
        if self.recent_notifications.len() > 100 {
            self.recent_notifications.pop_front();
        }
    }

    fn push_app_notification(&mut self, level: &str, message: String) {
        let id = self.next_app_notification_id;
        self.next_app_notification_id = id.saturating_add(1);
        self.push_ui_notification(UiNotification {
            plugin_id: "egscribe".to_string(),
            command: None,
            request_id: Some(id.to_string()),
            level: level.to_string(),
            message,
            status_shown_at: None,
        });
    }

    pub fn recent_notifications(&self) -> &VecDeque<UiNotification> {
        &self.recent_notifications
    }

    pub fn request_open_notifications_view(&mut self) {
        self.open_executions_view_requested = Some(OpenExecutionsViewRequest::Notifications);
    }

    pub fn request_open_execution_tasks_view(&mut self) {
        self.open_executions_view_requested = Some(OpenExecutionsViewRequest::Tasks);
    }

    pub fn take_open_executions_view_request(&mut self) -> Option<OpenExecutionsViewRequest> {
        let requested = self.open_executions_view_requested;
        self.open_executions_view_requested = None;
        requested
    }

    /// Get current Unix timestamp in seconds
    fn current_timestamp(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    pub fn cur_edit_ctx_mut(&mut self) -> Option<(UniFile, &mut Ctx)> {
        if let Some(curfile) = self.note_space.get_current_cur() {
            if let Some(ctx) = self.ectx_map.get_mut(&curfile) {
                return Some((curfile, ctx))
            }
        }
        None
    }

    pub fn is_cur_content_changed(&self) -> bool {
        if let Some(curfile) = self.note_space.get_current_cur() {
            if let Some(ctx) = self.ectx_map.get(&curfile) {
                return ctx.is_content_changed();
            }
        }
        false
    }

    fn set_edit_cfg(config: &Config, edit_ctx: &mut Ctx) {
        edit_ctx.cfg_mut().show_line_no = config.show_line_no;
        edit_ctx.cfg_mut().wrap = config.wrap;
        edit_ctx.cfg_mut().dark_mode = config.dark_mode;
        edit_ctx.cfg_mut().text_color_brightness = config.text_color_brightness;
        edit_ctx.set_font_size(config.font_size);
        edit_ctx.set_indent_size(config.indent_size);
        edit_ctx.set_list_item_indent_size(config.list_item_indent_size);
        edit_ctx.cfg_mut().table_frame_style = config.table_frame_style.clone();
        edit_ctx.cfg_mut().show_heading_section_numbers = config.show_heading_section_numbers;
        edit_ctx.cfg_mut().show_table_row_no = config.show_table_row_no;
        edit_ctx.cfg_mut().show_table_head_checkbox = config.show_table_head_checkbox;
        edit_ctx.cfg_mut().plantuml_jar_path = config.plantuml_jar_path.clone();
        edit_ctx.sync_table_views_frame_style();
    }

    fn new_ctx_with_cfg(&self) -> Ctx {
        let mut new_ctx = Ctx::new();
        Self::set_edit_cfg(&self.config, &mut new_ctx);
        new_ctx
    }

    pub fn open_set_ctx(&mut self, curfile: &UniFile) {
        if let Some(edit_ctx) = self.ectx_map.get_mut(&curfile) {
            edit_ctx.set_open_time();
            edit_ctx.set_request_focus();
            edit_ctx.cursor2_cmp_reset();
            Self::set_edit_cfg(&self.config, edit_ctx);
            self.note_space.set_current_file(&curfile);
            self.config_set_current_file(&curfile);   
        }
    }

    pub fn open_note(&mut self, name: &str) -> std::io::Result<String> {
        // file isn't exist, create first
        if !self.note_space.is_file_exist(name) {
            self.note_space.write_note(&name, "")?;
            self.note_space.flash_data();
        }

        // Get curfile after flash_data to ensure path is correct
        let curfile = self.note_space.note_name_to_unifile(name);

        // Only close non-fixed notes when opening a new note (not switching)
        // If the note is already opened, don't close other notes
        let note_already_opened = self.opened_files.contains(&curfile);
        if !note_already_opened {
            // Close all non-fixed notes except the current one being opened
            let notes_to_close: Vec<UniFile> = self
                .opened_files
                .notes()
                .iter()
                .filter(|note| {
                    let note_name = note.name();
                    // Keep fixed notes and the note being opened
                    note_name != name && !self.is_fixed(&note_name)
                })
                .map(|note| (*note).clone())
                .collect();
            
            for note in notes_to_close {
                self.close(&note);
            }
        }

        // If note is not in ectx_map, create new ctx
        if self.ectx_map.get(&curfile).is_none() {
            let text = self.note_space.read_note(name)?;
            let new_ctx = self
                .new_ctx_with_cfg()
                .with_text(&text, true)
                .monospace(false)  // Markdown notes use proportional font by default
                .image_path(Some(self.note_space.image_path()))
                .height_mode(HeightMode::fix_max());
            self.ectx_map.insert(curfile.clone(), new_ctx);
        }

        // Add to opened files order list
        // If note is not fixed, place it at the end of all notes
        let is_fixed_note = self.is_fixed(name);
        if !self.opened_files.contains(&curfile) {
            if is_fixed_note {
                // Fixed note: insert after current file
                let after = self.note_space.get_current_cur();
                self.opened_files.add(curfile.clone(), after.as_ref());
            } else {
                // Non-fixed note: insert at the end of all notes
                self.opened_files.add(curfile.clone(), None);
            }
            // Update opened files list in config
            self.update_opened_files_config();
        } else if !is_fixed_note {
            // Note already exists but is not fixed, move it to the end of all notes
            if let Some(pos) = self.opened_files.position(&curfile) {
                // Find the last note position
                let last_note_pos = self.opened_files.all().iter()
                    .rposition(|f| f.is_note())
                    .unwrap_or(0);
                // Only move if not already at the last position
                if pos < last_note_pos {
                    self.opened_files.move_to(&curfile, last_note_pos);
                    self.update_opened_files_config();
                }
            }
        }
        
        // Set ctx
        self.open_set_ctx(&curfile);

        // Update recent files
        self.update_recent_files(&curfile);

        Ok(String::new())
    }  

    pub fn open_file(&mut self, name: &str) -> std::io::Result<String> {
        let normalized = match Self::normalize_open_path(name) {
            Some(path) => path,
            None => return Ok(String::new()),
        };
        if is_untitled_path(&normalized) {
            return self.open_untitled_file(&normalized);
        }
        let curfile = UniFile::from(&normalized);
        if self.ectx_map.get(&curfile).is_some() {
            self.open_set_ctx(&curfile);
            self.update_recent_files(&curfile);
            return Ok(String::new());
        }
        self.enqueue_open_request(&normalized);
        self.start_next_open_task_if_idle();
        Ok(String::new())
    }

    fn open_untitled_file(&mut self, path: &str) -> std::io::Result<String> {
        let curfile = UniFile::from(path);
        if self.ectx_map.get(&curfile).is_some() {
            self.open_set_ctx(&curfile);
            self.update_recent_files(&curfile);
            return Ok(String::new());
        }

        let work_dir = self.note_space.work_dir().to_path_buf();
        let (text, loaded_from_swap) = resolve_content_on_open(path, &work_dir, "")?;
        let parse_markdown = path_is_markdown_file(path);
        self.apply_opened_file_text(path, text, parse_markdown, loaded_from_swap)?;
        Ok(String::new())
    }

    fn parse_untitled_id(path: &str) -> Option<u64> {
        path.strip_prefix("untitled/Untitled-")
            .and_then(|s| s.parse().ok())
    }

    fn next_untitled_id(&self) -> u64 {
        let mut max_id = 0u64;
        for file in self.opened_files.all() {
            if let Some(id) = Self::parse_untitled_id(&file.path()) {
                max_id = max_id.max(id);
            }
        }
        for path in &self.config.opend_files {
            if let Some(id) = Self::parse_untitled_id(path) {
                max_id = max_id.max(id);
            }
        }
        max_id + 1
    }

    pub fn tick_async_open(&mut self) {
        if let Some(task) = self.active_open_task.take() {
            match task.rx.try_recv() {
                Ok(result) => {
                    match result {
                        AsyncOpenResult::Loaded {
                            file_path,
                            text,
                            parse_markdown,
                            loaded_from_swap,
                        } => {
                            if let Err(e) = self.apply_opened_file_text(
                                &file_path,
                                text,
                                parse_markdown,
                                loaded_from_swap,
                            ) {
                                log::error!("Failed to apply async opened file {}: {}", file_path, e);
                            }
                        }
                        AsyncOpenResult::NonText { file_path, reason } => {
                            self.pending_non_text_file_prompt = Some(NonTextFilePrompt {
                                file_path,
                                reason,
                            });
                        }
                        AsyncOpenResult::Error { file_path, reason } => {
                            log::error!("Failed to async open {}: {}", file_path, reason);
                        }
                    }
                }
                Err(TryRecvError::Empty) => {
                    self.active_open_task = Some(task);
                }
                Err(TryRecvError::Disconnected) => {
                    log::error!("Async open task channel disconnected");
                }
            }
        }
        self.start_next_open_task_if_idle();
    }

    pub fn async_open_progress(&self) -> Option<f32> {
        self.active_open_task.as_ref().map(|task| {
            let t = task.started_at.elapsed().as_secs_f32();
            let wave = ((t * 3.0).sin() * 0.5 + 0.5) * 0.7;
            (0.15 + wave).clamp(0.0, 0.95)
        })
    }

    fn is_likely_binary(content: &[u8]) -> bool {
        if content.is_empty() {
            return false;
        }

        let sample_len = content.len().min(4096);
        let sample = &content[..sample_len];

        if sample.contains(&0) {
            return true;
        }

        let suspicious = sample
            .iter()
            .filter(|&&b| {
                !(b == b'\n'
                    || b == b'\r'
                    || b == b'\t'
                    || (0x20..=0x7e).contains(&b)
                    || b >= 0x80)
            })
            .count();

        suspicious as f32 / sample_len as f32 > 0.3
    }

    pub fn pending_non_text_file_prompt(&self) -> Option<&NonTextFilePrompt> {
        self.pending_non_text_file_prompt.as_ref()
    }

    pub fn dismiss_non_text_file_prompt(&mut self) {
        self.pending_non_text_file_prompt = None;
    }

    pub fn request_read_non_text_with_plugin(&mut self) {
        const HEX_FILE_TO_MD: &str = "hex_file_to_md";

        if let Some(prompt) = self.pending_non_text_file_prompt.as_ref().cloned() {
            let mut params = HashMap::new();
            params.insert(
                "path".to_string(),
                serde_json::Value::String(prompt.file_path.clone()),
            );

            match self.plugin_manager.execute_file_command_with_auto_start(
                HEX_FILE_TO_MD,
                &prompt.file_path,
                params,
            ) {
                Ok(result) => {
                    log::info!(
                        "Plugin handled {} successfully: {:?}",
                        HEX_FILE_TO_MD,
                        result
                    );
                }
                Err(e) => {
                    log::error!("Plugin failed {}: {}", HEX_FILE_TO_MD, e);
                    self.pending_non_text_file_prompt = Some(NonTextFilePrompt {
                        file_path: prompt.file_path,
                        reason: format!("插件转换失败：{}", e),
                    });
                    return;
                }
            }
        }
        self.pending_non_text_file_prompt = None;
    }

    /// filename - open filename.md in note space
    /// path - open file in file system
    pub fn open(&mut self, name: &str) -> std::io::Result<String> {
        println!("open: {}", name);
        if name == "" {
            return Ok(String::new())
        }
        if name == "." {
            return Ok(String::new())
        } else if name.contains("/") || name.contains("\\") {
            let new_name = name.replace("\\", "/");
            self.open_file(&new_name)
        } else {
            self.open_note(name)
        }
    }

    pub fn open_unifile(&mut self, unifile: &UniFile) -> std::io::Result<String> {
        match unifile {
            UniFile::File(_) => {
                self.open_file(&unifile.path())
            }
            UniFile::Note(_) => {
                self.open_note(&unifile.name())
            }
        }
    }

    fn enqueue_open_request(&mut self, name: &str) {
        let Some(normalized) = Self::normalize_open_path(name) else {
            return;
        };
        if self
            .active_open_task
            .as_ref()
            .is_some_and(|task| task.file_path == normalized)
        {
            return;
        }
        if self.pending_open_requests.iter().any(|path| path == &normalized) {
            return;
        }
        self.pending_open_requests.push_back(normalized);
    }

    fn start_next_open_task_if_idle(&mut self) {
        if self.active_open_task.is_some() {
            return;
        }
        let Some(file_path) = self.pop_next_pending_open_request() else {
            return;
        };
        let auto_detect = self.config.auto_detect_encoding;
        let default_charset = self.config.default_charset.clone();
        let work_dir = self.note_space.work_dir().to_path_buf();
        let (tx, rx) = mpsc::channel();
        let task_file_path = file_path.clone();
        thread::spawn(move || {
            let result = Self::load_file_for_open(&file_path, auto_detect, &default_charset, work_dir);
            let _ = tx.send(result);
        });
        self.active_open_task = Some(AsyncOpenTask {
            file_path: task_file_path,
            started_at: Instant::now(),
            rx,
        });
    }

    fn pop_next_pending_open_request(&mut self) -> Option<String> {
        while let Some(file_path) = self.pending_open_requests.pop_front() {
            let curfile = UniFile::from(&file_path);
            if self.ectx_map.get(&curfile).is_none() {
                return Some(file_path);
            }
        }
        None
    }

    fn normalize_open_path(name: &str) -> Option<String> {
        if name.is_empty() || name == "." {
            return None;
        }
        Some(name.replace("\\", "/"))
    }

    fn load_file_for_open(
        file_path: &str,
        auto_detect: bool,
        default_charset: &str,
        work_dir: PathBuf,
    ) -> AsyncOpenResult {
        const MARKDOWN_PARSE_LIMIT_BYTES: usize = 1024 * 1024;

        let raw_content = match std::fs::read(file_path) {
            Ok(raw) => raw,
            Err(e) => {
                return AsyncOpenResult::Error {
                    file_path: file_path.to_string(),
                    reason: e.to_string(),
                };
            }
        };
        if Self::is_likely_binary(&raw_content) {
            return AsyncOpenResult::NonText {
                file_path: file_path.to_string(),
                reason: "The file appears to contain many binary bytes and may not be plain text.".to_string(),
            };
        }

        let mut encoding_manager = EncodingManager::new();
        let text = if auto_detect {
            match encoding_manager.read_file_as_utf8(file_path) {
                Ok(content) => content,
                Err(e) => {
                    log::error!("Failed to read file with encoding detection: {}", e);
                    match std::fs::read_to_string(file_path) {
                        Ok(content) => content,
                        Err(read_err) => {
                            return AsyncOpenResult::Error {
                                file_path: file_path.to_string(),
                                reason: read_err.to_string(),
                            };
                        }
                    }
                }
            }
        } else {
            let charset = Charset::from_str(default_charset);
            match encoding_manager.read_file_with_charset(file_path, &charset) {
                Ok(content) => content,
                Err(e) => {
                    log::error!("Failed to read file with default charset: {}", e);
                    match std::fs::read_to_string(file_path) {
                        Ok(content) => content,
                        Err(read_err) => {
                            return AsyncOpenResult::Error {
                                file_path: file_path.to_string(),
                                reason: read_err.to_string(),
                            };
                        }
                    }
                }
            }
        };
        let is_markdown_file = path_is_markdown_file(file_path);
        let parse_markdown = if is_markdown_file && raw_content.len() > MARKDOWN_PARSE_LIMIT_BYTES {
            log::info!(
                "Skip markdown parsing for large file: {} ({} bytes)",
                file_path,
                raw_content.len()
            );
            false
        } else {
            is_markdown_file
        };

        let (text, loaded_from_swap) = match resolve_content_on_open(file_path, &work_dir, &text) {
            Ok(result) => result,
            Err(e) => {
                return AsyncOpenResult::Error {
                    file_path: file_path.to_string(),
                    reason: e.to_string(),
                };
            }
        };

        AsyncOpenResult::Loaded {
            file_path: file_path.to_string(),
            text,
            parse_markdown,
            loaded_from_swap,
        }
    }

    fn apply_opened_file_text(
        &mut self,
        name: &str,
        text: String,
        parse_markdown: bool,
        loaded_from_swap: bool,
    ) -> std::io::Result<String> {
        let curfile = UniFile::from(name);
        if self.ectx_map.get(&curfile).is_none() {
            let mut new_ctx = self
                .new_ctx_with_cfg()
                .with_text(&text, parse_markdown)
                .height_mode(HeightMode::fix_max());
            if let Some(ext) = PathBuf::from(name).extension() {
                let ext = ext.to_string_lossy().to_string();
                new_ctx.set_height_lang(sitter::ext_to_lang(&ext));
            }
            if loaded_from_swap {
                new_ctx.mark_unsaved_from_swap();
            }
            self.ectx_map.insert(curfile.clone(), new_ctx);
        }

        if !self.opened_files.contains(&curfile) {
            let after = self.note_space.get_current_cur();
            self.opened_files.add(curfile.clone(), after.as_ref());
            self.update_opened_files_config();
        }

        self.open_set_ctx(&curfile);
        self.update_recent_files(&curfile);
        self.pending_open_requests.retain(|path| path != name);
        Ok(String::new())
    }

    pub fn close(&mut self, file: &UniFile) {
        log::debug!("close {:?}", file);
        if self.ectx_map.len() > 1 {
            if file.is_file() && is_untitled_path(&file.path()) {
                let work_dir = self.note_space.work_dir().to_path_buf();
                if let Err(e) = delete_swap(&file.path(), &work_dir) {
                    log::error!("Failed to delete untitled swap on close: {}", e);
                }
            }

            // remove firstly
            self.ectx_map.remove(file);
            
            // Remove from opened files order list
            self.opened_files.remove(file);

            let last_file = self.ectx_map.iter().max_by(|x, y|{
                let time1 = x.1.get_open_time();
                let time2 = y.1.get_open_time();
                time1.cmp(&time2)
            });
            if let Some((last_file,_)) = last_file {
                log::debug!("open {:?}", last_file);
                self.open_set_ctx(&last_file.clone());
            }
            
            // Update opened files list in config and save
            self.update_opened_files_config();
        }
    }

    /// Close all notes
    pub fn close_all_notes(&mut self) {
        let notes_to_close: Vec<UniFile> = self
            .opened_files
            .notes()
            .into_iter()
            .map(|f| (*f).clone())
            .collect();
        for note in notes_to_close {
            self.close(&note);
        }
    }

    /// Close all files
    pub fn close_all_files(&mut self) {
        let files_to_close: Vec<UniFile> = self
            .opened_files
            .files()
            .into_iter()
            .map(|f| (*f).clone())
            .collect();
        for file in files_to_close {
            self.close(&file);
        }
    }

    /// Close all files and notes
    pub fn close_all(&mut self) {
        let all_files: Vec<UniFile> = self.opened_files.all().iter().cloned().collect();
        for file in all_files {
            self.close(&file);
        }
    }

    /// untitled 文件通过另存为对话框保存到新路径。
    /// 返回 `Ok(false)` 表示用户取消。
    fn save_file_as(&mut self, curfile: &UniFile, text: &str) -> Result<bool, std::io::Error> {
        let file_path = curfile.path();
        let Some(save_path) =
            show_save_file_dialog(&curfile.name(), Some(self.note_space.work_dir()))
        else {
            return Ok(false);
        };
        self.encoding_manager
            .write_file_with_encoding(&save_path, text, &Charset::UTF8, &LineEnding::LF)?;
        let new_file = UniFile::from(&save_path);
        let old_file = curfile.clone();
        let is_markdown = path_is_markdown_file(&save_path);
        let mut new_ctx = self
            .new_ctx_with_cfg()
            .with_text(text, is_markdown)
            .height_mode(HeightMode::fix_max());
        new_ctx.set_open_time();
        self.ectx_map.remove(&old_file);
        self.ectx_map.insert(new_file.clone(), new_ctx);
        self.opened_files.update(&old_file, new_file.clone());
        self.update_opened_files_config();
        self.note_space.set_current_file(&new_file);
        if let Some(ctx) = self.ectx_map.get_mut(&new_file) {
            Self::set_edit_cfg(&self.config, ctx);
            ctx.clean_change_tick();
            ctx.clean_swap_tick();
        }
        let work_dir = self.note_space.work_dir().to_path_buf();
        if let Err(e) = delete_swap(&file_path, &work_dir) {
            log::error!("Failed to delete untitled swap after save: {}", e);
        }
        if let Err(e) = write_swap(&save_path, &work_dir, text) {
            log::error!("Failed to sync swap after save: {}", e);
        }
        Ok(true)
    }

    /// 保存外部文件（含 untitled 另存为与普通路径）。
    /// 返回 `Ok(false)` 表示用户在另存为对话框中取消；`save()` 需据此提前返回且不更新自动保存时间。
    fn save_file(&mut self, curfile: &UniFile, text: &str) -> Result<bool, std::io::Error> {
        let file_path = curfile.path();
        if is_untitled_path(&file_path) {
            return self.save_file_as(curfile, text);
        }

        // Get file's original encoding
        let (charset, line_ending) =
            if let Some(encoding_info) = self.encoding_manager.get_file_encoding(&file_path) {
                (&encoding_info.charset, &encoding_info.line_ending)
            } else {
                (&Charset::UTF8, &LineEnding::LF)
            };
        // Save file with original encoding and line ending format
        let saved = match self
            .encoding_manager
            .write_file_with_encoding(&file_path, text, charset, line_ending)
        {
            Ok(_) => true,
            Err(e) => {
                log::error!("Failed to save file with encoding: {}", e);
                // If encoding save fails, try direct save
                match self.note_space.write_file(&file_path, text) {
                    Ok(_) => true,
                    Err(e) => {
                        log::error!(
                            "Failed to save file (e.g. no write permission): {}, try save as",
                            e
                        );
                        false
                    }
                }
            }
        };
        if !saved {
            return self.save_file_as(curfile, text);
        }
        // Clear change flag
        if let Some(ctx) = self.ectx_map.get_mut(curfile) {
            ctx.clean_change_tick();
            ctx.clean_swap_tick();
        }
        let work_dir = self.note_space.work_dir().to_path_buf();
        if let Err(e) = write_swap(&file_path, &work_dir, text) {
            log::error!("Failed to sync swap after save: {}", e);
        }
        Ok(true)
    }

    fn save_note(&mut self, curfile: &UniFile, text: &str) -> std::io::Result<()> {
        self.note_space.write_note(&curfile.name(), text)?;
        if let Some(ctx) = self.ectx_map.get_mut(curfile) {
            ctx.clean_change_tick();
        }
        Ok(())
    }

    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(curfile) = self.note_space.get_current_cur() {
            // Get text first to avoid borrow conflict
            let text = if let Some(ctx) = self.ectx_map.get(&curfile) {
                ctx.get_all_text()
            } else {
                return Ok(());
            };

            if curfile.is_file() {
                if !self.save_file(&curfile, &text)? {
                    return Ok(());
                }
            } else {
                self.save_note(&curfile, &text)?;
            }
            // Update last auto-save time after successful save
            self.last_auto_save_time = self.current_timestamp();
        }
        Ok(())
    }
    
    /// Check and perform auto-save if enabled and interval has passed.
    /// Notes are saved directly; external and untitled files are saved to swap files.
    pub fn check_auto_save(&mut self) {
        // Check if auto-save is enabled
        if !self.config.auto_save_enabled {
            return;
        }
        
        // Check if enough time has passed since last save
        let current_time = self.current_timestamp();
        let time_since_last_save = current_time.saturating_sub(self.last_auto_save_time);
        
        if time_since_last_save < self.config.auto_save_interval {
            return;
        }
        
        let notes_to_save: Vec<UniFile> = self.opened_files.notes()
            .iter()
            .filter_map(|note| {
                if let Some(ctx) = self.ectx_map.get(note) {
                    if ctx.is_content_changed() {
                        return Some((*note).clone());
                    }
                }
                None
            })
            .collect();

        let files_to_save: Vec<UniFile> = self.opened_files.files()
            .iter()
            .filter_map(|file| {
                if let Some(ctx) = self.ectx_map.get(file) {
                    if ctx.is_swap_stale() {
                        return Some((*file).clone());
                    }
                }
                None
            })
            .collect();
        
        let mut has_saved = false;
        for note in notes_to_save {
            let note_name = note.name();
            if let Err(e) = self.auto_save_note(&note) {
                log::error!("Auto-save failed for note {}: {}", note_name, e);
            } else {
                has_saved = true;
                self.push_app_notification(
                    "info",
                    format!("{}: {}", i18n::tr("autosave.note"), note_name),
                );
            }
        }
        for file in files_to_save {
            let file_path = file.path();
            if let Err(e) = self.auto_save_file(&file) {
                log::error!("Auto-save failed for file {}: {}", file_path, e);
            } else {
                has_saved = true;
                self.push_app_notification(
                    "info",
                    format!("{}: {}", i18n::tr("autosave.swap"), file_path),
                );
            }
        }
        
        if has_saved {
            self.last_auto_save_time = current_time;
        }
    }
    
    /// Auto-save a note (internal method, only saves notes)
    fn auto_save_note(&mut self, curfile: &UniFile) -> std::io::Result<()> {
        // Get text first to avoid borrow conflict
        let text = if let Some(ctx) = self.ectx_map.get(curfile) {
            ctx.get_all_text()
        } else {
            return Ok(());
        };
        
        // Only save notes
        if !curfile.is_note() {
            return Ok(());
        }
        
        // Save the note
        self.note_space.write_note(&curfile.name(), &text)?;
        
        // Clear change flag
        if let Some(ctx) = self.ectx_map.get_mut(curfile) {
            ctx.clean_change_tick();
        }
        
        Ok(())
    }

    /// Auto-save an external file to its swap file (internal method)
    fn auto_save_file(&mut self, curfile: &UniFile) -> std::io::Result<()> {
        if !curfile.is_file() {
            return Ok(());
        }

        let file_path = curfile.path();

        let text = if let Some(ctx) = self.ectx_map.get(curfile) {
            ctx.get_all_text()
        } else {
            return Ok(());
        };

        let work_dir = self.note_space.work_dir().to_path_buf();
        write_swap(&file_path, &work_dir, &text)?;

        if let Some(ctx) = self.ectx_map.get_mut(curfile) {
            ctx.clean_swap_tick();
        }

        Ok(())
    }

    /// Create an unsaved file
    pub fn create_untitled_file(&mut self) {
        let counter = self.next_untitled_id();
        let untitled_path = format!("untitled/Untitled-{}", counter);
        
        // Create new context
        let new_ctx = self
            .new_ctx_with_cfg()
            .with_text("", false)
            .height_mode(HeightMode::fix_max());
        let untitled_file = UniFile::from(&untitled_path);
        
        // Add to ectx_map
        self.ectx_map.insert(untitled_file.clone(), new_ctx);
        
        // Insert into opened files order list: find current file's position, insert after it
        let after = self.note_space.get_current_cur();
        self.opened_files.add(untitled_file.clone(), after.as_ref());
        // Update opened files list in config
        self.update_opened_files_config();
        
        // Set as current file
        self.open_set_ctx(&untitled_file);
    }

    pub fn new_note(&mut self, parent: Option<String>) -> std::io::Result<()> {
        if let Some(new_name) = self.note_space.new_file_name() {
            //create new file
            self.note_space.write_note(&new_name, "")?;

            //add link to parent
            if let Some(parent_name) = parent {
                if Some(parent_name.to_string()) == self.note_space.get_current_note() {
                    let _ = self.save();
                } 
                
                let text = self.note_space.read_note(&parent_name)?;
                let text = text + "\n\n[[" + &new_name + "]]";
                self.note_space.write_note(&parent_name, &text)?;
            }
            //flash data
            self.note_space.flash_data();

            //open new file
            self.open(&new_name)?;
        }
        Ok(())
    }

    pub fn rename_file(&mut self, org_name: &str, new_name: &str) -> std::io::Result<()> {
        self.note_space.rename(org_name, new_name)?;

        for parent in self.note_space.get_parents(org_name) {
            //change line content in parent file
            let text = self.note_space.read_note(&parent)?;
            let org_links = format!("[[{}]]", org_name);
            let new_links = format!("[[{}]]", new_name);
            let new_text = text.replace(&org_links, &new_links);
            self.note_space.write_note(&parent, &new_text)?;
        }
        //flash data
        self.note_space.flash_data();

        //open new file
        self.open(new_name)?;
        Ok(())
    }

    pub fn delete_file(&mut self, file: &str) -> std::io::Result<()> {
        self.note_space.delete_file(file)?;
        let mut to_open= "help".to_string();

        for parent in self.note_space.get_parents(file) {
            //change line content in parent file
            let text = self.note_space.read_note(&parent)?;
            let org_links = format!("[[{}]]\n", file);
            let new_text = text.replace(&org_links, "");

            let org_links = format!("[[{}]]", file);
            let new_text = new_text.replace(&org_links, "");
            self.note_space.write_note(&parent, &new_text)?;

            to_open = parent;
        }

        //flash data
        self.note_space.flash_data();

        //open parent file
        self.open(&to_open)?;
        Ok(())
    }

    pub fn execute_goto(&mut self, line_text: String, dead_loop: usize) {
        if dead_loop > 1 {
            return;
        }
        if let Some(find_file) = self.find_window.find_file.clone() {
            let arr: Vec<&str> = line_text.trim().split('.').collect();
            if arr.len() < 3 {
                return;
            }
            
            // Check if it's FindNotes virtual file
            if find_file.name() == "__all_notes__" {
                // Extract note name from line_text (format: {line_no}.{segment}.{column}. [note_name] line_text)
                // First find [note_name] part
                let note_name = if let Some(bracket_start) = line_text.find('[') {
                    if let Some(bracket_end) = line_text[bracket_start+1..].find(']') {
                        Some(line_text[bracket_start+1..bracket_start+1+bracket_end].to_string())
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                if let Some(note_name) = note_name {
                    // Find matching item from find_cache (match by note name and line number)
                    let original_line_no = arr[0].parse::<usize>().unwrap_or(0);
                    let original_segment = arr[1].parse::<usize>().unwrap_or(0);
                    let original_culumn = arr[2].parse::<usize>().unwrap_or(0);
                    
                    // Get needed data first to avoid borrow conflict
                    let cursor_pos = {
                        let (find_cache, _) = self.find_window.edit_ctx.get_find_cache();
                        // Find matching item: note name matches, and line_no, segment, column match
                        find_cache.cache.iter().find(|item| {
                            if let Some(ref item_line_text) = item.line_text {
                                if let Some(item_note_name) = Self::extract_note_name_from_line_text(item_line_text) {
                                    if item_note_name == note_name {
                                        return item.end.line_no == original_line_no 
                                            && item.end.segment == original_segment 
                                            && item.end.culumn == original_culumn;
                                    }
                                }
                            }
                            false
                        }).map(|item| item.end)
                    };
                    
                    // Open real note and set cursor
                    if let Some(cursor_pos) = cursor_pos {
                        if self.open_note(&note_name).is_ok() {
                            // Use original cursor position from FindCacheItem (relative to this note)
                            if let Some((_uni_file, cur_edit)) = self.cur_edit_ctx_mut() {
                                cur_edit.set_cursor2(cursor_pos);
                                cur_edit.set_cursor1_reset();
                            }
                        }
                    }
                }
                return;
            }
            
            // Normal file/note handling
            let curosr:Cursor = (arr[0].parse::<usize>().unwrap(), arr[1].parse::<usize>().unwrap(), arr[2].parse::<usize>().unwrap()).into();
            if let Some((uni_file,cur_edit)) = self.cur_edit_ctx_mut() {
                if find_file == uni_file {
                    cur_edit.set_cursor2(curosr);
                    cur_edit.set_cursor1_reset();
                } else {
                    if self.open_unifile(&find_file).is_ok() {
                        self.execute_goto(line_text, dead_loop+1);
                    }
                }
            }
        }
    }
    
    /// Extract note name from line text (format: [note_name] line_text)
    fn extract_note_name_from_line_text(line_text: &str) -> Option<String> {
        if let Some(start) = line_text.find('[') {
            if let Some(end) = line_text[start+1..].find(']') {
                let note_name = &line_text[start+1..start+1+end];
                return Some(note_name.to_string());
            }
        }
        None
    }
    
    pub fn clear_find_live_filter_on_current_ctx(&mut self) {
        if let Some((_uni_file, edit_ctx)) = self.cur_edit_ctx_mut() {
            edit_ctx.clear_find_live_filter();
        }
    }

    pub fn handle_find_window(&mut self, ui: &mut eframe::egui::Ui) {
        if let Some(find) = self.find_window.show(ui) {
            self.execute_find(find);
        }
        if self.find_window.drain_clear_filter() {
            self.clear_find_live_filter_on_current_ctx();
        }
    }

    pub fn execute_find(&mut self, find: FindReplaceCtx) {
        let skip_find_result = if let Some(cmd) = &find.cmd {
            matches!(cmd, FindCmd::FindNotes | FindCmd::LiveDisplay)
        } else {
            false
        };

        self.execute_cmd(Action::find_replace(find));

        if skip_find_result {
            return;
        }
        
        let dark_mode = self.config.dark_mode;
        
        let (uni_file, find_cache, find_param) = if let Some((uni_file, edit_ctx)) = self.cur_edit_ctx_mut() {
            let (find_cache, find_param) = edit_ctx.get_find_cache();
               (uni_file, find_cache.clone(), find_param.clone())
        } else {
            return;
        };
        self.find_window.set_find_result(uni_file, &find_cache, &find_param, dark_mode);
    }

    pub fn execute_cmd(&mut self, cmd: Action) {
        if cmd.is_editor_action() {
            if let Some((_unifile, edit_ctx)) = self.cur_edit_ctx_mut() {
                edit_ctx.defer_editor_action(cmd);
            } else {
                log::warn!(
                    "No active editor context for editor action: {}",
                    cmd.command
                );
            }
            return;
        }

        match cmd.command.as_str() {
            "open_file" => {
                if let Ok(file) = cmd.get_string_param("path") {
                    let _ = self.open(&file);
                } else {
                    log::warn!("Failed to get 'path' parameter for open_file command");
                }
            }
            "path_list" => {
                if let Ok(parent) = cmd.get_string_param("path") {
                    let links = self.note_space.get_child_links(&parent);
                    log::debug!("{:?}", links);
                } else {
                    log::warn!("Failed to get 'path' parameter for path_list command");
                }
            }
            "delete_file" => {
                if let Ok(file) = cmd.get_string_param("path") {
                    let _ = self.delete_file(&file);
                } else {
                    log::warn!("Failed to get 'path' parameter for delete_file command");
                }
            }
            "new_file" => {
                let parent = cmd.get_optional_string_param("parent")
                    .unwrap_or(None);
                let _ = self.new_note(parent);
            }
            "rename_file" => {
                if let Ok(file) = cmd.get_string_param("path") {
                    self.note_space.rename_window_active(&file);
                } else {
                    log::warn!("Failed to get 'path' parameter for rename_file command");
                }
            }
            "click_edit_line" => {
                if let Ok(line) = cmd.get_string_param("line") {
                    self.execute_goto(line, 0);
                } else {
                    log::warn!("Failed to get 'line' parameter for click_edit_line command");
                }
            }
            "goto_editor_line" => {
                if let Ok(line_no) = cmd.get_number_param("line_no") {
                    let line_no = line_no as usize;
                    if let Some((_, ctx)) = self.cur_edit_ctx_mut() {
                        let max_ln = ctx.line_num().saturating_sub(1);
                        let line_no = line_no.min(max_ln);
                        let c: Cursor = line_no.into();
                        let c = ctx.cursor_check(&c);
                        ctx.set_cursor2(c);
                        ctx.set_cursor1_reset();
                    }
                } else {
                    log::warn!("Failed to get 'line_no' parameter for goto_editor_line command");
                }
            }
            "open_url" => {
                if let Ok(url) = cmd.get_string_param("url") {
                    open_url(&url);
                } else {
                    log::warn!("Failed to get 'url' parameter for open_url command");
                }
            }
            "fixed_file" => {
                if let Ok(name) = cmd.get_string_param("path") {
                    self.fix_file(&name);
                } else {
                    log::warn!("Failed to get 'path' parameter for fixed_file command");
                }
            }
            "unfixed_file" => {
                if let Ok(name) = cmd.get_string_param("path") {
                    self.unfix_file(&name);
                } else {
                    log::warn!("Failed to get 'path' parameter for unfixed_file command");
                }
            }
            "find_replace" => {
                // 使用 FindReplaceCtx::from_action 从 Action 创建 FindReplaceCtx
                let param = match FindReplaceCtx::from_action(&cmd) {
                    Ok(ctx) => ctx,
                    Err(e) => {
                        log::warn!("Failed to parse find_replace action: {}", e);
                        return;
                    }
                };
                if let Some((_unifile, edit_ctx)) = self.cur_edit_ctx_mut() {
                    if let Some(find_cmd) = param.cmd.clone() {
                        match find_cmd {
                            FindCmd::Find => {
                                edit_ctx.find_and_select(&param);
                            },
                            FindCmd::Replace => {
                                if edit_ctx.is_selected() {
                                    edit_ctx.insert(param.replace.clone());
                                }
                                edit_ctx.find_and_select(&param);
                            },
                            FindCmd::ReplaceAll => {
                                while edit_ctx.find_and_select(&param) {
                                    edit_ctx.insert(param.replace.clone());
                                }
                            },
                            FindCmd::FindAll => {
                                edit_ctx.find_all(&param);
                                self.tool_bar_info.is_show_bottom = true;
                            },
                            FindCmd::FindNotes => {
                                self.find_in_all_notes(&param);
                                self.tool_bar_info.is_show_bottom = true;
                            },
                            FindCmd::LiveDisplay => {
                                edit_ctx.set_find_live_filter(param.clone(), true);
                            },
                        }
                    }
                }
            }
            "execute_plugin_command" => {
                self.request_open_execution_tasks_view();
                let plugin_id = match cmd.get_string_param("plugin_id") {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!("Failed to get 'plugin_id' parameter: {}", e);
                        return;
                    }
                };
                let plugin_command = match cmd.get_string_param("command") {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!("Failed to get 'command' parameter: {}", e);
                        return;
                    }
                };
                let params = cmd
                    .params
                    .get("plugin_params")
                    .and_then(|v| v.as_object())
                    .map(|map| dynamic_params::resolve_params(map, self))
                    .unwrap_or_default();
                if let Err(e) = self.plugin_manager.execute_plugin_command_async_with_auto_start(
                    &plugin_id,
                    &plugin_command,
                    params,
                ) {
                    log::warn!(
                        "Failed to execute plugin command '{}:{}': {}",
                        plugin_id,
                        plugin_command,
                        e
                    );
                }
            }
            // 触发器事件：行内容改变
            "line_changed" => {
                if let (Ok(line_no), Ok(line_text)) = (cmd.get_number_param("line_no"), cmd.get_string_param("line_text")) {
                    let mut event_data = HashMap::new();
                    event_data.insert("line_no".to_string(), serde_json::Value::Number(serde_json::Number::from(line_no)));
                    event_data.insert("line_text".to_string(), serde_json::Value::String(line_text));
                    self.plugin_manager.notify_event(Trigger::LineChanged, event_data);
                } else {
                    log::warn!("Failed to get parameters for line_changed command");
                }
            }
            _ => {
                log::warn!("Unknown command: {}", cmd.command);
            }
        }
    }

    /// Search in all notes
    fn find_in_all_notes(&mut self, param: &FindReplaceCtx) {
        use crate::medit::ctx::{FindCache, FindCacheItem};
        use crate::space::FilePath;
        
        if param.find.is_empty() {
            return;
        }
        
        let mut all_find_cache = FindCache::new();
        let mut all_results: Vec<(String, FindCacheItem)> = Vec::new();
        
        // Recursively get all note names
        let mut all_note_names = Vec::new();
        let root_files = self.note_space.get_child_links(".");
        for root in root_files {
            self.collect_all_notes(&root, &mut all_note_names);
        }
        
        // Iterate through all note files
        for note_name in all_note_names {
            // Read note content
            if let Ok(content) = self.note_space.read_note(&note_name) {
                // Create temporary Ctx for each note to search
                let mut temp_ctx = Ctx::new()
                    .with_text(&content, false)
                    .image_path(Some(self.note_space.image_path()))
                    .height_mode(HeightMode::fix_max());
                temp_ctx.find_all(param);
                
                // Get search results
                let (find_cache, _) = temp_ctx.get_find_cache();
                
                // Add results to total results and mark source note
                for item in &find_cache.cache {
                    // Create new FindCacheItem with note name information
                    let mut new_item = item.clone();
                    // Add note name before line text
                    if let Some(ref mut line_text) = new_item.line_text {
                        *line_text = format!("[{}] {}", note_name, line_text);
                    } else {
                        new_item.line_text = Some(format!("[{}]", note_name));
                    }
                    all_results.push((note_name.clone(), new_item));
                }
            }
        }
        
        // Sort by note name and line number
        all_results.sort_by(|a, b| {
            let name_cmp = a.0.cmp(&b.0);
            if name_cmp != std::cmp::Ordering::Equal {
                return name_cmp;
            }
            a.1.end.line_no.cmp(&b.1.end.line_no)
        });
        
        // Convert to FindCache
        all_find_cache.cache = all_results.into_iter().map(|(_, item)| item).collect();
        println!("all_find_cache count: {}", all_find_cache.cache.len());
        
        // Set to find_window (using a virtual UniFile)
        // Since it's results from multiple notes, we use a special identifier
        let virtual_file = UniFile::Note(FilePath {
            name: "__all_notes__".to_string(),
            path: "__all_notes__".to_string(),
        });
        
        // Ensure result is set
        self.find_window.set_find_result(
            virtual_file,
            &all_find_cache,
            param,
            self.config.dark_mode
        );
        
        // Ensure bottom panel is shown
        self.tool_bar_info.is_show_bottom = true;
    }
    
    /// Recursively collect all note names
    fn collect_all_notes(&self, name: &str, result: &mut Vec<String>) {
        result.push(name.to_string());
        let children = self.note_space.get_child_links(name);
        for child in children {
            self.collect_all_notes(&child, result);
        }
    }

    pub fn config_save(&self) {
        let json_str = serde_json::to_string_pretty(&self.config).unwrap();
        let config_file = self.note_space.config_file();
        let _ = std::fs::write(&config_file, json_str);
    }

    /// Update opened files list in config based on OpenedFilesManager
    pub fn config_update_opend_files(&mut self) {
        // Use OpenedFilesManager to get ordered file list
        let opend_files: Vec<String> = self.opened_files
            .all()
            .iter()
            .map(|file| file.name4open())
            .collect();
        self.config.opend_files = opend_files;
    }

    /// Update opened files list in config and save
    /// Call this after modifying opened_files (add, remove, update, move)
    fn update_opened_files_config(&mut self) {
        self.config_update_opend_files();
        self.config_save();
    }

    /// Update recent files list in config
    /// Add the file/note to the beginning of recent_files, remove duplicates, keep max 32 items
    fn update_recent_files(&mut self, file: &UniFile) {
        let file_key = file.name4open();
        
        // Remove if already exists
        self.config.recent_files.retain(|f| f != &file_key);
        
        // Insert at the beginning
        self.config.recent_files.insert(0, file_key);
        
        // Keep only the first 32 items
        if self.config.recent_files.len() > 32 {
            self.config.recent_files.truncate(32);
        }
        
        self.config_save();
    }

    /// Move file/note to new position
    /// file_path: File path to move (name for notes, path for files)
    /// target_idx: Target position (index in corresponding type, notes or files)
    /// is_note: Whether it's a note
    pub fn move_file_to_position(&mut self, file_path: &str, target_idx: usize, is_note: bool) -> bool {
        // Find the file to move
        let file = if is_note {
            self.opened_files.all().iter()
                .find(|f| f.is_note() && f.name() == file_path)
                .cloned()
        } else {
            self.opened_files.all().iter()
                .find(|f| f.is_file() && f.path() == file_path)
                .cloned()
        };

        if let Some(file) = file {
            // Calculate target position in all files
            let target_pos_in_all = if is_note {
                // For notes, target_idx is the target position (notes are in front)
                target_idx
            } else {
                // For files, need to add the number of notes
                let notes_count = self.opened_files.notes().len();
                notes_count + target_idx
            };

            // Move file to new position
            if self.opened_files.move_to(&file, target_pos_in_all) {
                // If move successful, update opened files list in config
                self.update_opened_files_config();
                return true;
            }
        }
        false
    }


    pub fn config_set_current_file(&mut self, curfile: &UniFile) {
        if curfile.is_file() {
            self.config.current_file = curfile.path();
        } else {
            self.config.current_file = curfile.name();
        }
        // Only save config, don't update opened files list (current file change doesn't affect opened files)
        self.config_save();
    }

    pub fn config_switch_wrap_mode(&mut self) {
        self.config.wrap = !self.config.wrap;
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.cfg_mut().wrap = self.config.wrap;
        }
        self.config_save();
    }

    pub fn config_switch_show_line_no(&mut self) {
        self.config.show_line_no = !self.config.show_line_no;
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.cfg_mut().show_line_no = self.config.show_line_no;
        }
        self.config_save();
    }

    pub fn config_switch_heading_section_numbers(&mut self) {
        self.config.show_heading_section_numbers = !self.config.show_heading_section_numbers;
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.cfg_mut().show_heading_section_numbers = self.config.show_heading_section_numbers;
        }
        self.config_save();
    }

    pub fn config_switch_table_row_no(&mut self) {
        self.config.show_table_row_no = !self.config.show_table_row_no;
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.cfg_mut().show_table_row_no = self.config.show_table_row_no;
        }
        self.config_save();
    }

    pub fn config_switch_table_head_checkbox(&mut self) {
        self.config.show_table_head_checkbox = !self.config.show_table_head_checkbox;
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.cfg_mut().show_table_head_checkbox = self.config.show_table_head_checkbox;
        }
        self.config_save();
    }

    pub fn config_update_dark_mode(&mut self, dark_mode: bool) {
        self.config.dark_mode = dark_mode;
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.update_view_mode(self.config.dark_mode);
        }
        self.config_save();
    }

    pub fn config_set_font_size(&mut self, size: f32) {
        self.config.font_size = size;
        if self.config.font_size < 6.0 {
            self.config.font_size = 6.0
        }
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.set_font_size(size as f32);
        }
        self.config_save();
    }

    pub fn config_add_indent_size(&mut self, delta: f32) {
        self.config.indent_size += delta;
        if self.config.indent_size < 0.0 || self.config.indent_size > 100.0 {
            self.config.indent_size = 0.0
        }
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.set_indent_size(self.config.indent_size);
        }
        self.config_save();
    }

    #[allow(dead_code)]
    pub fn config_add_list_item_indent_size(&mut self, delta: f32) {
        self.config.list_item_indent_size += delta;
        if self.config.list_item_indent_size < 0.0 || self.config.list_item_indent_size > 100.0 {
            self.config.list_item_indent_size = 0.0
        }
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.set_list_item_indent_size(self.config.list_item_indent_size);
        }
        self.config_save();
    }

    pub fn config_set_text_color_brightness(&mut self, brightness: f32) {
        // Clamp brightness to reasonable range (0.1 to 2.0)
        let clamped_brightness = if brightness < 0.1 {
            0.1
        } else if brightness > 2.0 {
            2.0
        } else {
            brightness
        };
        self.config.text_color_brightness = clamped_brightness;
        let brightness_value = self.config.text_color_brightness;
        if let Some((_uni_file, cur_edit)) = self.cur_edit_ctx_mut() {
            cur_edit.set_text_color_brightness(brightness_value);
        }
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.set_text_color_brightness(brightness_value);
        }
        self.config_save();
    }

    pub fn config_update_show_index_window(&mut self, is_show: bool) {
        self.config.show_index_window = is_show;
        self.note_space.set_show_index_window(is_show);
        self.config_save();
    }

    pub fn config_set_table_frame_style(&mut self, style: TableFrameStyle) {
        self.config.table_frame_style = style.clone();
        for (_, ctx) in self.ectx_map.iter_mut() {
            ctx.cfg_mut().table_frame_style = style.clone();
            ctx.sync_table_views_frame_style();
        }
        self.config_save();
    }

    pub fn config_restore(&mut self) {
        let config_file = self.note_space.config_file();
        if let Ok(json_str) = std::fs::read_to_string(&config_file) {
            // Use serde_json::from_str Result handling, use default if parsing fails
            match serde_json::from_str::<Config>(&json_str) {
                Ok(config) => {
                    self.config = config;
                }
                Err(e) => {
                    log::warn!("Failed to parse config file: {}. Using default config.", e);
                    // Keep current default config
                }
            }
        }
        self.note_space.set_show_index_window(self.config.show_index_window);

        //restore current file
        if self.config.current_file.is_empty() {
            let curfile = self.note_space.note_name_to_unifile("untitled_1");
            self.config_set_current_file(&curfile);
        }
        let current_file = self.config.current_file.clone();

        //restore opend files
        for file in self.config.opend_files.clone() {
            let _= self.open(&file);
        }
        let _= self.open(&current_file);
    }

    /// Get current file's encoding information
    pub fn get_current_file_encoding(&self) -> Option<&FileEncoding> {
        if let Some(curfile) = self.note_space.get_current_cur() {
            if curfile.is_file() {
                return self.encoding_manager.get_file_encoding(&curfile.path());
            }
        }
        None
    }

    /// Set default charset
    #[allow(dead_code)]
    pub fn config_set_default_charset(&mut self, charset: &str) {
        self.config.default_charset = charset.to_string();
        self.config_save();
    }

    /// Toggle automatic encoding detection
    #[allow(dead_code)]
    pub fn config_toggle_auto_detect_encoding(&mut self) {
        self.config.auto_detect_encoding = !self.config.auto_detect_encoding;
        self.config_save();
    }

    /// Redetect current file's encoding
    #[allow(dead_code)]
    pub fn redetect_current_file_encoding(&mut self) -> Result<(), std::io::Error> {
        if let Some(curfile) = self.note_space.get_current_cur() {
            if curfile.is_file() {
                // Clear cache and redetect
                self.encoding_manager.clear_cache();
                let _ = self.encoding_manager.detect_file_encoding(&curfile.path())?;
            }
        }
        Ok(())
    }

    /// Reopen current file with specified encoding
    pub fn reopen_with_encoding(&mut self, charset: &Charset) -> std::io::Result<()> {
        if let Some(curfile) = self.note_space.get_current_cur() {
            if curfile.is_file() {
                let file_path = curfile.path();
                // Read file with specified encoding
                let text = self.encoding_manager.read_file_with_charset(&file_path, charset)?;
                
                // Recreate edit context
                let is_markdown = path_is_markdown_file(&file_path);
                let mut new_ctx = self
                    .new_ctx_with_cfg()
                    .with_text(&text, is_markdown)
                    .height_mode(HeightMode::fix_max())
                    .show_line_no(self.config.show_line_no)
                    .wrap(self.config.wrap)
                    .dark_mode(self.config.dark_mode)
                    .set_font_size_chain(self.config.font_size)
                    .indent_size(self.config.indent_size)
                    .list_item_indent_size(self.config.list_item_indent_size)
                    .text_color_brightness(self.config.text_color_brightness)
                    .show_heading_section_numbers(self.config.show_heading_section_numbers);
                if let Some(ext) = PathBuf::from(&file_path).extension(){
                    let ext = ext.to_string_lossy().to_string();
                    new_ctx.set_height_lang(sitter::ext_to_lang(&ext));
                }
                
                // Update edit context
                if let Some(ctx) = self.ectx_map.get_mut(&curfile) {
                    *ctx = new_ctx;
                    ctx.clean_change_tick();
                }
                
                // Cache encoding information
                self.encoding_manager.set_file_encoding(&file_path, charset.clone(), false);
            }
        }
        Ok(())
    }

    /// Save current file with specified encoding
    pub fn save_with_encoding(&mut self, charset: &Charset) -> std::io::Result<()> {
        if let Some(curfile) = self.note_space.get_current_cur() {
            if let Some(ctx) = self.ectx_map.get_mut(&curfile) {
                let text = ctx.get_all_text();
                if curfile.is_file() {
                    // Get file's original line ending format
                    let line_ending = if let Some(encoding_info) = self.encoding_manager.get_file_encoding(&curfile.path()) {
                        &encoding_info.line_ending
                    } else {
                        // If no cache, default to LF
                        &LineEnding::LF
                    };
                    
                    // Save file with specified encoding and original line ending format
                    self.encoding_manager.write_file_with_encoding(&curfile.path(), &text, charset, line_ending)?;
                    
                    // Update cache
                    self.encoding_manager.set_file_encoding(&curfile.path(), charset.clone(), false);
                    
                    ctx.clean_change_tick();
                }
            }
        }
        Ok(())
    }

    /// Save current file with specified line ending format
    pub fn save_with_line_ending(&mut self, line_ending: &LineEnding) -> std::io::Result<()> {
        if let Some(curfile) = self.note_space.get_current_cur() {
            if let Some(ctx) = self.ectx_map.get_mut(&curfile) {
                let text = ctx.get_all_text();
                if curfile.is_file() {
                    // Get file's original encoding
                    let charset = if let Some(encoding_info) = self.encoding_manager.get_file_encoding(&curfile.path()) {
                        &encoding_info.charset
                    } else {
                        // If no cache, default to UTF-8
                        &Charset::UTF8
                    };
                    
                    // Save file with original encoding and specified line ending format
                    self.encoding_manager.write_file_with_encoding(&curfile.path(), &text, charset, line_ending)?;
                    
                    // Update line ending format in cache
                    if let Some(encoding_info) = self.encoding_manager.get_file_encoding(&curfile.path()) {
                        self.encoding_manager.set_file_encoding_with_line_ending(
                            &curfile.path(), 
                            encoding_info.charset.clone(), 
                            encoding_info.has_bom, 
                            line_ending.clone()
                        );
                    }
                    
                    ctx.clean_change_tick();
                }
            }
        }
        Ok(())
    }

    /// Get current file's line ending format
    pub fn get_current_file_line_ending(&self) -> Option<&LineEnding> {
        if let Some(curfile) = self.note_space.get_current_cur() {
            if curfile.is_file() {
                if let Some(encoding_info) = self.encoding_manager.get_file_encoding(&curfile.path()) {
                    return Some(&encoding_info.line_ending);
                }
            }
        }
        None
    }

    pub fn get_current_file_line_ending_name(&self) -> &str {
        if let Some(line_ending) = self.get_current_file_line_ending() {
            line_ending.display_name()
        } else {
            "LF (Unix)"
        }
    }

    /// Check if a file/note is fixed
    pub fn is_fixed(&self, name: &str) -> bool {
        self.config.fixed_files.contains(&name.to_string())
    }

    /// Fix a file/note to toolbar
    pub fn fix_file(&mut self, name: &str) {
        let name_str = name.to_string();
        if !self.config.fixed_files.contains(&name_str) {
            self.config.fixed_files.push(name_str);
            self.config_save();
        }
        
        // If the file/note is not opened, open it
        let curfile = if name.contains("/") || name.contains("\\") {
            UniFile::from(name)
        } else {
            self.note_space.note_name_to_unifile(name)
        };
        
        if !self.opened_files.contains(&curfile) {
            let _ = self.open(name);
        }
    }

    /// Unfix a file/note from toolbar
    pub fn unfix_file(&mut self, name: &str) {
        self.config.fixed_files.retain(|f| f != name);
        self.config_save();
        
        // If there are multiple notes open, close this note
        let notes_count = self.opened_files.notes().len();
        if notes_count > 1 {
            if let Some(note) = self.opened_files.all().iter().find(|f| f.is_note() && f.name() == name).cloned() {
                self.close(&note);
            }
        }
    }
}
