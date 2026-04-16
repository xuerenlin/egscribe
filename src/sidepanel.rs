use crate::medit::{TocNode, toc_entries_to_forest};
use crate::store::Store;
use crate::uicom::{IconName, icon_button_builder};
use crate::i18n::tr;
use eframe::egui::collapsing_header;
use eframe::egui::{Color32, Frame, Rect, RichText, ScrollArea, Ui};

// Side panel size tuning constants.
const SIDE_PANEL_TOP_SPACE: f32 = 8.0;
const SIDE_PANEL_TAB_ICON_FONT_SIZE: f32 = 15.0;
const SIDE_PANEL_TAB_INNER_MARGIN: f32 = 4.0;
const SIDE_PANEL_INDEX_DEFAULT_WIDTH: f32 = 260.0;
const SIDE_PANEL_COLLAPSED_DEFAULT_WIDTH: f32 = SIDE_PANEL_TAB_ICON_FONT_SIZE + SIDE_PANEL_TAB_INNER_MARGIN * 2.0;

/// 侧边栏页面类型
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SidePanelPage {
    /// 笔记管理页面
    Notes,
    /// 插件管理页面
    Plugins,
    /// 当前文档 Markdown 目录
    Outline,
}

/// Tab：`(页面, 图标, i18n key（传给 tr）)`
type TabInfo = (SidePanelPage, IconName, &'static str);

/// 侧边栏管理器
pub struct SidePanel {
    /// 当前选中的页面
    current_page: SidePanelPage,
    /// Tab 按钮信息列表
    tabs: Vec<TabInfo>,
}

impl SidePanel {
    pub fn new() -> Self {
        Self {
            current_page: SidePanelPage::Notes,
            tabs: vec![
                (
                    SidePanelPage::Notes,
                    IconName::icon_documents,
                    "sidepanel.tab.notes",
                ),
                (
                    SidePanelPage::Outline,
                    IconName::icon_bookmark_outline,
                    "sidepanel.tab.outline",
                ),
                (
                    SidePanelPage::Plugins,
                    IconName::icon_puzzle,
                    "sidepanel.tab.plugins",
                ),
            ],
        }
    }
    
    /// 显示侧边栏
    pub fn show(&mut self, store: &mut Store, ctx: &eframe::egui::Context) {
        if store.note_space.is_show_index_window() {
            eframe::egui::SidePanel::left("side_panel_index_window")
                .resizable(true)
                .default_width(SIDE_PANEL_INDEX_DEFAULT_WIDTH)
                .show_separator_line(true)
                .show(ctx, |ui| {
                    ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                    ui.add_space(SIDE_PANEL_TOP_SPACE);
                    let available_height = ui.available_height();
                    ui.horizontal(|ui| {
                        self.show_vertical_tab_bar(ui, available_height, store);
                        self.show_content_area(store, ui);
                    });
                });
            });
        } else {
            eframe::egui::SidePanel::left("side_panel_no_index_window")
                .resizable(false)
                .default_width(SIDE_PANEL_COLLAPSED_DEFAULT_WIDTH)
                .show_separator_line(true)
                .show(ctx, |ui| {
                    ui.add_space(SIDE_PANEL_TOP_SPACE);
                    let available_height = ui.available_height();
                    self.show_vertical_tab_bar(ui, available_height, store);
                });
        };
    }
    
    /// 显示左侧垂直 Tab bar
    fn show_vertical_tab_bar(&mut self, ui: &mut Ui, available_height: f32, store: &mut Store) {
        ui.vertical(|ui| {
            // Home 按钮（第一个按钮）
            let is_show_index_window = store.note_space.is_show_index_window();
            let color = if is_show_index_window {
                ui.style().visuals.selection.bg_fill
            } else {
                ui.style().visuals.text_color()
            };
            let frame = Frame::default().inner_margin(SIDE_PANEL_TAB_INNER_MARGIN);
            frame.show(ui, |ui| {
                let button = icon_button_builder(ui)
                    .icon(IconName::icon_home)
                    .font_size(SIDE_PANEL_TAB_ICON_FONT_SIZE)
                    .hover_text(tr("toolbar.home.tooltip"))
                    .fg(color)
                    .build_tool();
                
                if button.clicked() {
                    store.config_update_show_index_window(!store.note_space.is_show_index_window());
                }
            });

            for (page, icon, hover_text) in &self.tabs {
                let is_selected = self.current_page == *page;
                let frame = if is_selected {
                    Frame::default().fill(ui.style().visuals.selection.bg_fill)
                } else {
                    Frame::default().fill(Color32::TRANSPARENT)
                };
                let frame = frame.inner_margin(SIDE_PANEL_TAB_INNER_MARGIN);
                
                frame.show(ui, |ui| {
                    let button = icon_button_builder(ui)
                        .icon(icon.clone())
                        .font_size(SIDE_PANEL_TAB_ICON_FONT_SIZE)
                        .hover_text(tr(hover_text))
                        .build_tool();
                    
                    if button.clicked() {
                        self.current_page = *page;
                        store.config_update_show_index_window(true);
                    }
                });
            }

            // 填充剩余空间：使用 allocate_space
            let used_height = ui.cursor().top() - ui.min_rect().top();
            let remaining_height = available_height - used_height;
            if remaining_height > 0.0 {
                ui.allocate_space(eframe::egui::Vec2::new(0.0, remaining_height));
            }
        });
    }
    
    /// 显示右侧内容区域
    fn show_content_area(&mut self, store: &mut Store, ui: &mut Ui) {
        ScrollArea::both().auto_shrink(false).show(ui, |ui| {
            ui.vertical(|ui| {
                let mut outer_rect = ui.cursor();
                outer_rect.set_width(ui.available_width());
                outer_rect.set_height(ui.available_height());
                let in_rect = outer_rect.expand(-10.0);
                // 根据当前页面显示内容
                match self.current_page {
                    SidePanelPage::Notes => {
                        self.show_notes_page(store, ui, in_rect, outer_rect);
                    }
                    SidePanelPage::Outline => {
                        self.show_outline_page(store, ui);
                    }
                    SidePanelPage::Plugins => {
                        self.show_plugins_page(store, ui);
                    }
                }
            });
        });
    }

    /// Markdown 目录：TOC 由编辑器 [`crate::medit::Edit`] 内定期刷新；此处只读缓存，点击跳转标题行。
    fn show_outline_page(&mut self, store: &mut Store, ui: &mut Ui) {
        let mut jump_line = None::<usize>;
        {
            let Some((f, ctx)) = store.cur_edit_ctx_mut() else {
                ui.label(tr("sidepanel.toc.no_editor"));
                return;
            };

            if !ctx.cfg().is_markdown {
                ui.label(tr("sidepanel.toc.not_markdown"));
                return;
            }

            ui.label(RichText::new(tr("sidepanel.toc.title")).strong());
            ui.add_space(4.0);

            if ctx.toc_entries().is_empty() {
                ui.label(tr("sidepanel.toc.no_headings"));
                return;
            }

            let storage_key = f.path();
            let show_section_numbers = ctx.cfg().show_heading_section_numbers;
            let forest = toc_entries_to_forest(ctx.toc_entries());
            self.render_toc_nodes(
                ui,
                &storage_key,
                &forest,
                &mut jump_line,
                show_section_numbers,
            );
        }

        if let Some(line) = jump_line {
            if let Some((_, ctx)) = store.cur_edit_ctx_mut() {
                ctx.set_scroll_to_line(line);
                ctx.set_cursor2(line.into());
                ctx.set_cursor1_reset();
            }
        }
    }

    /// 目录树：有子标题的节点用 [`collapsing_header::CollapsingState`]，与 `space::show_sub_index` 相同套路。
    fn render_toc_nodes(
        &self,
        ui: &mut Ui,
        storage_key: &str,
        nodes: &[TocNode],
        jump_line: &mut Option<usize>,
        show_section_numbers: bool,
    ) {
        for node in nodes {
            let label = if show_section_numbers {
                format!("{}  {}", node.entry.section_number, node.entry.title)
            } else {
                node.entry.title.clone()
            };
            let id = ui.make_persistent_id(format!(
                "egscribe_toc:{}:{}",
                storage_key,
                node.entry.line_no
            ));

            if node.children.is_empty() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    // 与 `CollapsingState::show_button_indented` 占用的 `spacing.indent` 宽度对齐
                    ui.add_space(ui.spacing().indent);
                    if ui.selectable_label(false, &label).clicked() {
                        *jump_line = Some(node.entry.line_no);
                    }
                });
            } else {
                let mut state =
                    collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true);
                let header_res = ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    state.show_toggle_button(ui, collapsing_header::paint_default_icon);
                    if ui.selectable_label(false, &label).clicked() {
                        *jump_line = Some(node.entry.line_no);
                    }
                });
                state.show_body_indented(&header_res.response, ui, |ui| {
                    self.render_toc_nodes(
                        ui,
                        storage_key,
                        &node.children,
                        jump_line,
                        show_section_numbers,
                    );
                });
            }
        }
    }
    
    /// 显示笔记管理页面
    fn show_notes_page(&mut self, store: &mut Store, ui: &mut Ui, in_rect: Rect, outer_rect: Rect) {
        let mut config = store.config.clone();
        if let Some(cmd) = store.note_space.show_index_view(&mut config, ui, in_rect, outer_rect) {
            store.execute_cmd(cmd);
        }
        // 保存树状态
        if config.tree_open_state_changed {
            store.config = config;
            store.config_save();
        }
    }
    
    /// 显示插件管理页面
    fn show_plugins_page(&mut self, store: &mut Store, ui: &mut Ui) {
        use crate::plugin::manager::PluginAction;
        
        let actions = store.plugin_manager.show_ui(ui);
        
        // 处理插件操作
        for action in actions {
            match action {
                PluginAction::StartStop(plugin_id, start) => {
                    if start {
                        let _ = store.plugin_manager.start_plugin(&plugin_id);
                    } else {
                        let _ = store.plugin_manager.stop_plugin(&plugin_id);
                    }
                }
                PluginAction::ShowLog(plugin_id) => {
                    Self::show_plugin_log(store, &plugin_id);
                }
                PluginAction::ShowConfig(plugin_id) => {
                    Self::show_plugin_config(store, &plugin_id);
                }
            }
        }
    }
    
    /// 显示插件日志（保存为 .log 文件并打开）
    fn show_plugin_log(store: &mut Store, plugin_id: &str) {
        // 获取插件日志
        let logs = match store.plugin_manager.get_plugin_logs(plugin_id) {
            Some(logs) => logs,
            None => {
                log::warn!("No logs found for plugin: {}", plugin_id);
                return;
            }
        };
        
        // 获取插件目录
        let plugin_dir = store.plugin_manager.plugin_dir();
        
        // 创建 logs 子目录（如果不存在）
        let logs_dir = plugin_dir.join("logs");
        if let Err(e) = std::fs::create_dir_all(&logs_dir) {
            log::error!("Failed to create logs directory: {}", e);
            return;
        }
        
        // 生成日志文件路径
        let log_file_name = format!("{}.log", plugin_id);
        let log_file_path = logs_dir.join(&log_file_name);
        
        // 将日志内容合并为字符串
        let log_content = if logs.is_empty() {
            format!("# 插件日志: {}\n暂无日志记录。\n", plugin_id)
        } else {
            let mut content = format!("# 插件日志: {}\n", plugin_id);
            for log_entry in logs {
                content.push_str(&log_entry);
                content.push_str("\n");
            }
            content
        };
        
        // 写入日志文件
        if let Err(e) = std::fs::write(&log_file_path, log_content) {
            log::error!("Failed to write plugin log file: {}", e);
            return;
        }
        
        // 关闭所有已打开的日志文件
        let logs_dir_str = logs_dir.to_string_lossy().to_string();
        let opened_files: Vec<crate::space::UniFile> = store.opened_files.all().iter()
            .filter(|file| {
                // 只处理普通文件（不是笔记）
                if file.is_file() {
                    let file_path = file.path();
                    // 检查文件路径是否在 logs 目录下，并且是 .log 文件
                    file_path.contains(&logs_dir_str) && file_path.ends_with(".log")
                } else {
                    false
                }
            })
            .cloned()
            .collect();
        
        for file in opened_files {
            store.close(&file);
        }
        
        // 打开日志文件
        let log_file_path_str = log_file_path.to_string_lossy().to_string();
        if let Err(e) = store.open_file(&log_file_path_str) {
            log::error!("Failed to open plugin log file: {}", e);
        }
    }

    /// 显示插件配置（打开对应 desc.json 便于实时修改）
    fn show_plugin_config(store: &mut Store, plugin_id: &str) {
        let Some(config_path) = store.plugin_manager.get_plugin_config_path(plugin_id) else {
            log::warn!("No config found for plugin: {}", plugin_id);
            return;
        };

        let config_path_str = config_path.to_string_lossy().to_string();
        if let Err(e) = store.open_file(&config_path_str) {
            log::error!("Failed to open plugin config file: {}", e);
        }
    }
}

