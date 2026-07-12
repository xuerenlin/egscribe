use crate::medit::{Ctx, PghType, Action};
use crate::i18n::tr;
use crate::uicom::{galley_builder, IconName, CONTROL_HIGHLIGHT};
use eframe::egui::{Color32, NumExt, Pos2, Rect, Sense, Ui, Vec2};
use log::info;

/// 与 `toolbar::tabbar::TAB_FONT_SIZE` 一致，下拉 / 右键菜单统一字号
const CTX_MENU_FONT_SIZE: f32 = 16.0;
/// 列宽下限（过窄时与 [`ContextMenu::compute_max_content_width`] 取较大值）
const CTX_MENU_MIN_WIDTH: f32 = 200.0;
/// 列宽上限（极长插件名等场景避免弹层过宽）
const CTX_MENU_MAX_WIDTH: f32 = 600.0;
/// 子菜单 `menu_button` 标题行在文本右侧预留（箭头等）
const CTX_SUBMENU_TITLE_EXTRA: f32 = 28.0;

/// 主标签 galley（可选图标 + 文案，与绘制一致）
fn menu_item_main_galley(ui: &Ui, menu_item: &MenuItem) -> std::sync::Arc<eframe::egui::Galley> {
    match &menu_item.icon {
        Some(icon) => galley_builder(ui)
            .icon(icon.clone())
            .icon_fg(CONTROL_HIGHLIGHT)
            .text(menu_item.text.as_str())
            .font_size(CTX_MENU_FONT_SIZE)
            .build(),
        None => galley_builder(ui)
            .text(menu_item.text.as_str())
            .font_size(CTX_MENU_FONT_SIZE)
            .build(),
    }
}

/// 与 [`ctx_menu_item_row`] 绘制一致：左 4px + 主文案 +（可选）12px + 快捷键 + 右 4px
fn measure_menu_item_row_width(ui: &Ui, menu_item: &MenuItem) -> f32 {
    let weak_color = ui.style().visuals.weak_text_color();
    let main_w = menu_item_main_galley(ui, menu_item).size().x;
    let sc_w = menu_item
        .shortcut
        .as_ref()
        .map(|sc| {
            galley_builder(ui)
                .text(sc.as_str())
                .font_size(CTX_MENU_FONT_SIZE)
                .fg(weak_color)
                .build()
                .size()
                .x
        })
        .unwrap_or(0.0);
    let mut w = 4.0 + main_w + 4.0;
    if sc_w > 0.0 {
        w += 12.0 + sc_w;
    }
    w.max(32.0)
}

/// 子菜单 `menu_button` 标题 galley（可选 `Submenu::title_icon`）
fn submenu_title_galley(ui: &Ui, sub: &Submenu) -> std::sync::Arc<eframe::egui::Galley> {
    match &sub.title_icon {
        Some(icon) => galley_builder(ui)
            .icon(icon.clone())
            .icon_fg(CONTROL_HIGHLIGHT)
            .text(sub.text.as_str())
            .font_size(CTX_MENU_FONT_SIZE)
            .build(),
        None => galley_builder(ui)
            .text(sub.text.as_str())
            .font_size(CTX_MENU_FONT_SIZE)
            .build(),
    }
}

fn measure_submenu_title_row_width(ui: &Ui, sub: &Submenu) -> f32 {
    submenu_title_galley(ui, sub).size().x + CTX_SUBMENU_TITLE_EXTRA + 8.0
}

/// 与 tab 栏「更多」菜单一致：galley 排版 + 整行可点；仅悬停时高亮底色
fn ctx_menu_item_row(ui: &mut Ui, menu_item: &MenuItem, column_width: f32) -> bool {
    let row_height = ui.spacing().interact_size.y;
    let full_width = ui.available_width().at_most(column_width);
    let row_rect = Rect::from_min_size(ui.cursor().left_top(), Vec2::new(full_width, row_height));
    let row_response = ui.allocate_rect(row_rect, Sense::click());

    if row_response.hovered() {
        let w = &ui.style().visuals.widgets.hovered;
        ui.painter()
            .rect_filled(row_rect, w.corner_radius, w.weak_bg_fill);
    }

    let weak_color = ui.style().visuals.weak_text_color();
    let text_color = ui.style().visuals.text_color();

    let shortcut_galley = menu_item.shortcut.as_ref().map(|sc| {
        galley_builder(ui)
            .text(sc.as_str())
            .font_size(CTX_MENU_FONT_SIZE)
            .fg(weak_color)
            .build()
    });
    // 不设 wrap_width：列宽被 clamp 或 available 略小于测量时，仍保持主文案单行（避免最长项被误换行）
    let main_galley = menu_item_main_galley(ui, menu_item);

    let main_x = row_rect.left() + 4.0;
    let main_y = row_rect.center().y - main_galley.size().y / 2.0;
    ui.painter()
        .galley(Pos2::new(main_x, main_y), main_galley, text_color);

    if let Some(sg) = shortcut_galley {
        let sc_x = row_rect.right() - sg.size().x - 4.0;
        let sc_y = row_rect.center().y - sg.size().y / 2.0;
        ui.painter()
            .galley(Pos2::new(sc_x, sc_y), sg, Color32::WHITE);
    }

    row_response.clicked()
}

/// 菜单项显示条件
#[derive(Clone)]
#[allow(dead_code)]
pub enum MenuCondition {
    /// 总是显示
    Always,
    /// 仅当有选中文本时显示
    HasSelection,
    /// 仅当没有选中文本时显示
    NoSelection,
    /// 仅当是单行选中时显示
    HasSingleLineSelected,
    /// 单行选中或没有选中（用于格式操作，如加粗、斜体等）
    SingleLineOrNoSelection,
    /// 当前行是某种类型
    CurrentIsSome(PghType),
    /// 选中的行中包含某种类型
    SelectIncludeSome(PghType),
    /// 取反条件
    Not(Box<MenuCondition>),
    /// 仅当是markdown文档时显示
    IsMarkdown,
    /// 仅当不是markdown文档时显示
    IsNotMarkdown,
    /// 仅当是只读模式时显示
    IsReadOnly,
    /// 仅当不是只读模式时显示
    IsNotReadOnly,
    /// 光标所在表格块有列选择（表头 checkbox）
    HasTableSelectedCols,
    /// 当前章下存在可批处理的标题行（见 [`Ctx::toc_chapter_descendant_heading_lines`]）
    OutlineSectionHasDescendants,
    /// 当前目录子树下存在可批处理的 **子级** 叶子标题（见 [`Ctx::toc_outline_subtree_leaf_descendant_heading_lines`]）
    OutlineSubtreeHasLeafDescendants,
    /// 光标所在大纲子树内存在可升高一级的标题（&lt; 6 级）
    OutlineSubtreeCanIncreaseHeadingLevel,
    /// 光标所在大纲子树内存在可降低一级的标题（&gt; 1 级）
    OutlineSubtreeCanDecreaseHeadingLevel,
    /// 组合条件（AND）
    And(Vec<MenuCondition>),
    /// 组合条件（OR）
    Or(Vec<MenuCondition>),
}

impl MenuCondition {
    /// 检查条件是否满足
    pub fn check(&self, ctx: &Ctx) -> bool {
        match self {
            MenuCondition::Always => true,
            MenuCondition::HasSelection => ctx.is_selected(),
            MenuCondition::NoSelection => !ctx.is_selected(),
            MenuCondition::HasSingleLineSelected => ctx.is_single_line_selected(),
            MenuCondition::SingleLineOrNoSelection => {
                ctx.is_single_line_selected() || !ctx.is_selected()
            }
            MenuCondition::CurrentIsSome(pgh_type) => {
                ctx.is_line_type(ctx.cursor2().line_no, pgh_type.clone())
            }
            MenuCondition::SelectIncludeSome(pgh_type) => {
                ctx.has_selected_line_type(pgh_type.clone())
            }
            MenuCondition::Not(condition) => !condition.check(ctx),
            MenuCondition::IsMarkdown => ctx.cfg().is_markdown,
            MenuCondition::IsNotMarkdown => !ctx.cfg().is_markdown,
            MenuCondition::IsReadOnly => ctx.cfg().is_read_only,
            MenuCondition::IsNotReadOnly => !ctx.cfg().is_read_only,
            MenuCondition::HasTableSelectedCols => {
                ctx.table_row_block_has_selected_cols(ctx.cursor2().line_no)
            }
            MenuCondition::OutlineSectionHasDescendants => ctx
                .toc_chapter_descendant_heading_lines(ctx.cursor2().line_no)
                .map(|v| !v.is_empty())
                .unwrap_or(false),
            MenuCondition::OutlineSubtreeHasLeafDescendants => ctx
                .toc_outline_subtree_leaf_descendant_heading_lines(ctx.cursor2().line_no)
                .is_some(),
            MenuCondition::OutlineSubtreeCanIncreaseHeadingLevel => ctx
                .toc_outline_subtree_heading_lines(ctx.cursor2().line_no)
                .map(|lines| {
                    lines.iter().any(|&ln| {
                        ctx.is_heading_line(ln)
                            && crate::medit::ctx::cache_outline::parse_heading_level_for_toc(
                                &ctx.get_line_text(ln),
                            ) < 6
                    })
                })
                .unwrap_or(false),
            MenuCondition::OutlineSubtreeCanDecreaseHeadingLevel => ctx
                .toc_outline_subtree_heading_lines(ctx.cursor2().line_no)
                .map(|lines| {
                    lines.iter().any(|&ln| {
                        ctx.is_heading_line(ln)
                            && crate::medit::ctx::cache_outline::parse_heading_level_for_toc(
                                &ctx.get_line_text(ln),
                            ) > 1
                    })
                })
                .unwrap_or(false),
            MenuCondition::And(conditions) => {
                conditions.iter().all(|c| c.check(ctx))
            }
            MenuCondition::Or(conditions) => {
                conditions.iter().any(|c| c.check(ctx))
            }
        }
    }

    pub fn heading_condition() -> MenuCondition {
        MenuCondition::And(vec![
            MenuCondition::Or(vec![
                MenuCondition::CurrentIsSome(PghType::Text),
                MenuCondition::CurrentIsSome(PghType::Heading),
            ]),
            MenuCondition::SingleLineOrNoSelection,
        ])
    }

    pub fn noselected_current_is_normal_text_condition() -> MenuCondition {
        MenuCondition::And(vec![
            MenuCondition::NoSelection,
            MenuCondition::Not(Box::new(MenuCondition::CurrentIsSome(PghType::CodeRow))),
            MenuCondition::Not(Box::new(MenuCondition::CurrentIsSome(PghType::TableRow))),
        ])
    }

    pub fn selected_normal_text_condition() -> MenuCondition {
        MenuCondition::And(vec![
            MenuCondition::HasSelection,
            MenuCondition::Not(Box::new(MenuCondition::SelectIncludeSome(PghType::CodeRow))),
            MenuCondition::Not(Box::new(MenuCondition::SelectIncludeSome(PghType::TableRow))),
        ])
    }

    pub fn current_is_not(pgytype: PghType) -> MenuCondition {
        MenuCondition::Not(Box::new(MenuCondition::CurrentIsSome(pgytype)))
    }

    pub fn code_condition() -> MenuCondition {
        MenuCondition::And(vec![
            MenuCondition::current_is_not(PghType::CodeRow),
            MenuCondition::Or(vec![
                MenuCondition::noselected_current_is_normal_text_condition(),
                MenuCondition::selected_normal_text_condition(),
            ]),
        ])
    }

    pub fn list_condition() -> MenuCondition {
        MenuCondition::Or(vec![
            MenuCondition::noselected_current_is_normal_text_condition(),
            MenuCondition::selected_normal_text_condition(),
        ])
    }

    pub fn table_condition() -> MenuCondition {
        MenuCondition::And(vec![
            MenuCondition::current_is_not(PghType::TableRow),
            MenuCondition::NoSelection,
        ])
    }

    /// 光标在 Markdown 表格内（`TableRow`），且非只读。
    pub fn in_markdown_table_cursor() -> MenuCondition {
        MenuCondition::And(vec![
            MenuCondition::IsMarkdown,
            MenuCondition::IsNotReadOnly,
            MenuCondition::CurrentIsSome(PghType::TableRow),
        ])
    }
}

/// 菜单项
#[derive(Clone)]
pub struct MenuItem {
    /// 显示文本
    pub text: String,
    /// 显示条件
    pub condition: MenuCondition,
    /// 执行的动作
    pub action: Action,
    /// 快捷键提示（可选）
    pub shortcut: Option<String>,
    /// 主标签前图标（可选）
    pub icon: Option<IconName>,
}

impl MenuItem {
    pub fn new(text: String, condition: MenuCondition, action: Action, icon: IconName) -> Self {
        Self {
            text,
            condition,
            action: action.clone(),
            shortcut: action.shortcut_string(),
            icon: Some(icon),
        }
    }

    #[allow(dead_code)]
    pub fn with_shortcut(mut self, shortcut: String) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// 检查是否应该显示
    pub fn should_show(&self, ctx: &Ctx) -> bool {
        self.condition.check(ctx)
    }

    /// 显示文本（包含快捷键）
    #[allow(dead_code)]
    pub fn display_text(&self) -> String {
        if let Some(shortcut) = &self.shortcut {
            format!("{}\t{}", self.text, shortcut)
        } else {
            self.text.clone()
        }
    }
}

/// 插件子菜单：在大纲上批量跳转并执行时的范围模式。
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum OutlineBatchMode {
    #[default]
    None,
    /// 当前章内同级标题（[`Ctx::toc_chapter_descendant_heading_lines`]）
    SiblingSections,
    /// 当前目录子树内全部 **子级** 叶子标题，不含光标所在根标题（[`Ctx::toc_outline_subtree_leaf_descendant_heading_lines`]）
    LeafDescendantsUnderSection,
}

/// 子菜单（用于 egui `menu_button`）
#[derive(Clone)]
pub struct Submenu {
    pub text: String,
    pub condition: MenuCondition,
    pub items: Vec<MenuItem>,
    /// 非 [`OutlineBatchMode::None`] 时：子项点击后按模式在大纲上逐段跳转再执行插件命令。
    pub outline_batch_mode: OutlineBatchMode,
    /// 标题前图标（如 `ctxmenu.plugin_run_all_in_section` 使用 `icon_functions`）
    pub title_icon: Option<IconName>,
}

impl Submenu {
    pub fn should_show(&self, ctx: &Ctx) -> bool {
        if !self.condition.check(ctx) {
            return false;
        }
        self.items.iter().any(|i| i.should_show(ctx))
    }
}

/// 菜单组（可以包含分隔符）
#[derive(Clone)]
pub enum MenuGroupItem {
    /// 普通菜单项
    Item(MenuItem),
    /// 分隔符
    Separator,
    /// 子菜单
    Submenu(Submenu),
}

/// 菜单组
#[derive(Clone)]
pub struct MenuGroup {
    /// 组名（可选，用于插件识别）
    #[allow(dead_code)]
    pub name: Option<String>,
    /// 菜单项列表
    pub items: Vec<MenuGroupItem>,
    /// 显示条件（整个组的条件）
    pub condition: Option<MenuCondition>,
}

impl MenuGroup {
    pub fn new(name: Option<String>) -> Self {
        Self {
            name,
            items: vec![],
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: MenuCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn add_item(mut self, item: MenuItem) -> Self {
        self.items.push(MenuGroupItem::Item(item));
        self
    }

    pub fn add_separator(mut self) -> Self {
        self.items.push(MenuGroupItem::Separator);
        self
    }

    pub fn add_submenu(mut self, submenu: Submenu) -> Self {
        self.items.push(MenuGroupItem::Submenu(submenu));
        self
    }

    /// 检查是否应该显示整个组
    pub fn should_show(&self, ctx: &Ctx) -> bool {
        if let Some(condition) = &self.condition {
            condition.check(ctx)
        } else {
            true
        }
    }
}

/// 插件在编辑器上下文菜单中注册的一行：主列表显示全部；批量子菜单仅包含 `supports_batch_concurrent` 为 true 的项。
#[derive(Clone)]
pub struct EditorPluginMenuEntry {
    pub text: String,
    pub action: Action,
    pub supports_batch_concurrent: bool,
}

/// 上下文菜单管理器
pub struct ContextMenu {
    /// 菜单组列表
    groups: Vec<MenuGroup>,
}

/// 将「在所有段落中执行」所需的跳转与插件命令按顺序压入 `Ctx::cmd_list`。
fn enqueue_plugin_all_in_outline_section(ctx: &mut Ctx, plugin_action: Action) {
    let Some(lines) = ctx.toc_chapter_descendant_heading_lines(ctx.cursor2().line_no) else {
        return;
    };
    if lines.is_empty() {
        return;
    }
    for line_no in &lines {
        info!("enqueue_plugin_all_in_outline_section: {} {}", line_no, ctx.get_line_text(*line_no));
        ctx.insert_cmd(Action::goto_editor_line(*line_no));
        ctx.insert_cmd(plugin_action.clone());
    }
}

fn enqueue_plugin_all_leaf_descendants_under_section(ctx: &mut Ctx, plugin_action: Action) {
    let Some(lines) =
        ctx.toc_outline_subtree_leaf_descendant_heading_lines(ctx.cursor2().line_no)
    else {
        return;
    };
    for line_no in &lines {
        info!(
            "enqueue_plugin_all_leaf_descendants_under_section: {} {}",
            line_no,
            ctx.get_line_text(*line_no)
        );
        ctx.insert_cmd(Action::goto_editor_line(*line_no));
        ctx.insert_cmd(plugin_action.clone());
    }
}

impl ContextMenu {
    /// 获取所有默认的 MenuAction 列表（用于快捷键处理）
    pub fn default_actions() -> Vec<Action> {
        vec![
            Action::copy(),
            Action::cut(),
            Action::paste(),
            Action::delete(),
            Action::select_all(),
            Action::bold(),
            Action::italic(),
            Action::strikethrough(),
            Action::code(),
            Action::code_block(),
            Action::link(),
            Action::table(),
            Action::heading(1),
            Action::heading(2),
            Action::heading(3),
            Action::heading(4),
            Action::heading(5),
            Action::heading(6),
            Action::quote(),
            Action::unordered_list(),
            Action::ordered_list(),
            Action::todo_list(),
            Action::horizontal_rule(),
            Action::table_delete_selected_rows(),
            Action::table_delete_selected_cols(),
            Action::table_insert_row_above(),
            Action::table_insert_row_below(),
            Action::table_insert_col_left(),
            Action::table_insert_col_right(),
            Action::table_split_by_selected_cols(),
            Action::table_merge_under_current_heading(),
            Action::heading_subtree_increase(),
            Action::heading_subtree_decrease(),
        ]
    }

    pub fn new() -> Self {
        let mut menu = Self {
            groups: vec![],
        };
        menu.init_default_menu();
        menu
    }

    /// 初始化默认菜单
    fn init_default_menu(&mut self) {
        // Markdown格式组
        let markdown_format_group = MenuGroup::new(Some("markdown_format".to_string()))
            .with_condition(MenuCondition::And(vec![
                MenuCondition::IsMarkdown,
                MenuCondition::IsNotReadOnly,
            ]))
            .add_item(MenuItem::new(
                tr("ctxmenu.bold"),
                MenuCondition::SingleLineOrNoSelection,
                Action::bold(),
                IconName::icon_format_bold,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.italic"),
                MenuCondition::SingleLineOrNoSelection,
                Action::italic(),
                IconName::icon_format_italic,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.strikethrough"),
                MenuCondition::SingleLineOrNoSelection,
                Action::strikethrough(),
                IconName::icon_strikethrough,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.link"),
                MenuCondition::SingleLineOrNoSelection,
                Action::link(),
                IconName::icon_link,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.inline_code"),
                MenuCondition::SingleLineOrNoSelection,
                Action::code(),
                IconName::icon_code1,
            ))
            .add_separator()
            .add_item(MenuItem::new(
                tr("ctxmenu.todo_list"),
                MenuCondition::list_condition(),
                Action::todo_list(),
                IconName::icon_todo1,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.unordered_list"),
                MenuCondition::list_condition(),
                Action::unordered_list(),
                IconName::icon_list2,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.ordered_list"),
                MenuCondition::list_condition(),
                Action::ordered_list(),
                IconName::icon_list_numbered,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.quote"),
                MenuCondition::list_condition(),
                Action::quote(),
                IconName::icon_embed,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.code_block"),
                MenuCondition::code_condition(),
                Action::code_block(),
                IconName::icon_code1,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table"),
                MenuCondition::table_condition(),
                Action::table(),
                IconName::icon_table,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.horizontal_rule"),
                MenuCondition::table_condition(),
                Action::horizontal_rule(),
                IconName::icon_border_horizontal,
            ))
            .add_separator()
            .add_item(MenuItem::new(
                tr("ctxmenu.heading1"),
                MenuCondition::heading_condition(),
                Action::heading(1),
                IconName::icon_h_square,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading2"),
                MenuCondition::heading_condition(),
                Action::heading(2),
                IconName::icon_h_square,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading3"),
                MenuCondition::heading_condition(),
                Action::heading(3),
                IconName::icon_h_square,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading4"),
                MenuCondition::heading_condition(),
                Action::heading(4),
                IconName::icon_h_square,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading5"),
                MenuCondition::heading_condition(),
                Action::heading(5),
                IconName::icon_h_square,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading6"),
                MenuCondition::heading_condition(),
                Action::heading(6),
                IconName::icon_h_square,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading_subtree_increase"),
                MenuCondition::And(vec![
                    MenuCondition::heading_condition(),
                    MenuCondition::OutlineSubtreeCanIncreaseHeadingLevel,
                ]),
                Action::heading_subtree_increase(),
                IconName::icon_plus,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading_subtree_decrease"),
                MenuCondition::And(vec![
                    MenuCondition::heading_condition(),
                    MenuCondition::OutlineSubtreeCanDecreaseHeadingLevel,
                ]),
                Action::heading_subtree_decrease(),
                IconName::icon_minus,
            ));

        self.groups.push(markdown_format_group);

        let table_ops_group = MenuGroup::new(Some("markdown_table_ops".to_string()))
            .with_condition(MenuCondition::in_markdown_table_cursor())
            .add_item(MenuItem::new(
                tr("ctxmenu.table_delete_selected_rows"),
                MenuCondition::Always,
                Action::table_delete_selected_rows(),
                IconName::icon_trash,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_delete_selected_cols"),
                MenuCondition::Always,
                Action::table_delete_selected_cols(),
                IconName::icon_trash,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_insert_row_above"),
                MenuCondition::Always,
                Action::table_insert_row_above(),
                IconName::icon_plus,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_insert_row_below"),
                MenuCondition::Always,
                Action::table_insert_row_below(),
                IconName::icon_plus,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_insert_col_left"),
                MenuCondition::Always,
                Action::table_insert_col_left(),
                IconName::icon_plus,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_insert_col_above"),
                MenuCondition::Always,
                Action::table_insert_col_right(),
                IconName::icon_plus,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_split_by_selected_cols"),
                MenuCondition::HasTableSelectedCols,
                Action::table_split_by_selected_cols(),
                IconName::icon_table_view,
            ));
        self.groups.push(table_ops_group);

        let table_merge_group = MenuGroup::new(Some("markdown_table_merge".to_string()))
            .with_condition(MenuCondition::And(vec![
                MenuCondition::IsMarkdown,
                MenuCondition::IsNotReadOnly,
            ]))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_merge_under_current_heading"),
                MenuCondition::Always,
                Action::table_merge_under_current_heading(),
                IconName::icon_border_all,
            ));
        self.groups.push(table_merge_group);

        // 基本编辑组（所有文档）
        let basic_group = MenuGroup::new(Some("basic".to_string()))
            .add_item(MenuItem::new(
                tr("ctxmenu.copy"),
                MenuCondition::HasSelection,
                Action::copy(),
                IconName::icon_copy,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.cut"),
                MenuCondition::And(vec![
                    MenuCondition::HasSelection,
                    MenuCondition::IsNotReadOnly,
                ]),
                Action::cut(),
                IconName::icon_edit_crop,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.paste"),
                MenuCondition::IsNotReadOnly,
                Action::paste(),
                IconName::icon_clipboard,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.delete"),
                MenuCondition::And(vec![
                    MenuCondition::HasSelection,
                    MenuCondition::IsNotReadOnly,
                ]),
                Action::delete(),
                IconName::icon_trash,
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.select_all"),
                MenuCondition::Always,
                Action::select_all(),
                IconName::icon_text_document,
            ));

        self.groups.push(basic_group);
    
    }

    /// 添加菜单组（用于插件扩展）
    #[allow(dead_code)]
    pub fn add_group(&mut self, group: MenuGroup) {
        self.groups.push(group);
    }

    /// 添加插件注册的上下文菜单项
    pub fn add_plugin_command_items(&mut self, items: Vec<EditorPluginMenuEntry>) {
        if items.is_empty() {
            return;
        }
        let mut group = MenuGroup::new(Some("plugin_commands".to_string()));
        for entry in &items {
            group = group.add_item(MenuItem::new(
                entry.text.clone(),
                MenuCondition::Always,
                entry.action.clone(),
                IconName::icon_functions,
            ));
        }
        let outline_items: Vec<MenuItem> = items
            .iter()
            .filter(|e| e.supports_batch_concurrent)
            .map(|e| {
                MenuItem::new(
                    e.text.clone(),
                    MenuCondition::Always,
                    e.action.clone(),
                    IconName::icon_functions,
                )
            })
            .collect();
        group = group.add_submenu(Submenu {
            text: tr("ctxmenu.plugin_run_all_in_section"),
            condition: MenuCondition::And(vec![
                MenuCondition::IsMarkdown,
                MenuCondition::OutlineSectionHasDescendants,
            ]),
            items: outline_items.clone(),
            outline_batch_mode: OutlineBatchMode::SiblingSections,
            title_icon: Some(IconName::icon_media_fast_forward),
        });
        group = group.add_submenu(Submenu {
            text: tr("ctxmenu.plugin_run_all_leaf_descendants"),
            condition: MenuCondition::And(vec![
                MenuCondition::IsMarkdown,
                MenuCondition::OutlineSubtreeHasLeafDescendants,
            ]),
            items: outline_items,
            outline_batch_mode: OutlineBatchMode::LeafDescendantsUnderSection,
            title_icon: Some(IconName::icon_media_fast_forward_outline),
        });
        self.groups.insert(0, group);
    }

    /// 在指定组名后插入菜单组（用于插件扩展）
    #[allow(dead_code)]
    pub fn insert_group_after(&mut self, after_group_name: &str, group: MenuGroup) {
        if let Some(index) = self.groups.iter().position(|g| {
            g.name.as_ref().map(|n| n == after_group_name).unwrap_or(false)
        }) {
            self.groups.insert(index + 1, group);
        } else {
            self.groups.push(group);
        }
    }

    /// 在指定组名前插入菜单组（用于插件扩展）
    #[allow(dead_code)]
    pub fn insert_group_before(&mut self, before_group_name: &str, group: MenuGroup) {
        if let Some(index) = self.groups.iter().position(|g| {
            g.name.as_ref().map(|n| n == before_group_name).unwrap_or(false)
        }) {
            self.groups.insert(index, group);
        } else {
            self.groups.push(group);
        }
    }

    /// 根据组名查找并替换菜单组（用于插件扩展）
    #[allow(dead_code)]
    pub fn replace_group(&mut self, group_name: &str, group: MenuGroup) {
        if let Some(index) = self.groups.iter().position(|g| {
            g.name.as_ref().map(|n| n == group_name).unwrap_or(false)
        }) {
            self.groups[index] = group;
        } else {
            self.groups.push(group);
        }
    }

    /// 按当前可见项估算根菜单列宽（最长一行：普通项、子菜单标题、空菜单占位文案）。
    pub fn compute_max_content_width(&self, ui: &Ui, ctx: &Ctx) -> f32 {
        let mut m = 0.0f32;
        let mut has_visible = false;
        for group in &self.groups {
            if !group.should_show(ctx) {
                continue;
            }
            for item in &group.items {
                match item {
                    MenuGroupItem::Item(mi) => {
                        if mi.should_show(ctx) {
                            has_visible = true;
                            m = m.max(measure_menu_item_row_width(ui, mi));
                        }
                    }
                    MenuGroupItem::Submenu(sub) => {
                        if sub.should_show(ctx) {
                            has_visible = true;
                            m = m.max(measure_submenu_title_row_width(ui, sub));
                        }
                    }
                    MenuGroupItem::Separator => {}
                }
            }
        }
        if !has_visible {
            m = galley_builder(ui)
                .text(tr("ctxmenu.no_action"))
                .font_size(CTX_MENU_FONT_SIZE)
                .build()
                .size()
                .x
                + 16.0;
        }
        m.max(ui.spacing().interact_size.x)
    }

    fn compute_submenu_items_max_width(ui: &Ui, sub: &Submenu, ctx: &Ctx) -> f32 {
        let mut m = 0.0f32;
        for item in &sub.items {
            if item.should_show(ctx) {
                m = m.max(measure_menu_item_row_width(ui, item));
            }
        }
        m.max(ui.spacing().interact_size.x)
    }

    /// 显示上下文菜单
    /// 返回 true 表示有菜单项被点击
    pub fn show(&self, ui: &mut Ui, ctx: &mut Ctx) -> bool {
        let root_w = self
            .compute_max_content_width(ui, ctx)
            .clamp(CTX_MENU_MIN_WIDTH, CTX_MENU_MAX_WIDTH);
        ui.set_min_width(root_w);
        ui.set_max_width(root_w);

        let mut item_clicked = false;
        let mut can_view_separator = false;
        let mut has_visible_items = false;

        for group in &self.groups {
            // 检查组是否应该显示
            if !group.should_show(ctx) {
                continue;
            }

            for item in &group.items {
                match item {
                    MenuGroupItem::Separator => {
                        // 只在前面已经显示过 Item 且当前组有可见项时才显示分隔符
                        if can_view_separator {
                            ui.separator();
                            can_view_separator = false;
                        }
                    }
                    MenuGroupItem::Item(menu_item) => {
                        if menu_item.should_show(ctx) {
                            if ctx_menu_item_row(ui, menu_item, root_w) {
                                // 编辑器命令直接执行；应用级/插件命令进入 cmd 队列，交给 Store 统一处理。
                                let should_enqueue = menu_item
                                    .action
                                    .info()
                                    .map(|info| info.executor.is_none())
                                    .unwrap_or(true);
                                if should_enqueue {
                                    ctx.insert_cmd(menu_item.action.clone());
                                } else {
                                    menu_item.action.execute(ctx, ui);
                                }
                                ui.close();
                                item_clicked = true;
                            }
                            can_view_separator = true;
                            has_visible_items = true;
                        }
                    }
                    MenuGroupItem::Submenu(sub) => {
                        if sub.should_show(ctx) {
                            let sub_w = Self::compute_submenu_items_max_width(ui, sub, ctx)
                                .clamp(CTX_MENU_MIN_WIDTH, CTX_MENU_MAX_WIDTH);
                            let sub_galley = submenu_title_galley(ui, sub);
                            let _ = ui.menu_button(sub_galley, |ui| {
                                ui.set_min_width(sub_w);
                                ui.set_max_width(sub_w);
                                for item in &sub.items {
                                    if !item.should_show(ctx) {
                                        continue;
                                    }
                                    if ctx_menu_item_row(ui, item, sub_w) {
                                        match sub.outline_batch_mode {
                                            OutlineBatchMode::SiblingSections => {
                                                enqueue_plugin_all_in_outline_section(
                                                    ctx,
                                                    item.action.clone(),
                                                );
                                            }
                                            OutlineBatchMode::LeafDescendantsUnderSection => {
                                                enqueue_plugin_all_leaf_descendants_under_section(
                                                    ctx,
                                                    item.action.clone(),
                                                );
                                            }
                                            OutlineBatchMode::None => {
                                                let should_enqueue = item
                                                    .action
                                                    .info()
                                                    .map(|info| info.executor.is_none())
                                                    .unwrap_or(true);
                                                if should_enqueue {
                                                    ctx.insert_cmd(item.action.clone());
                                                } else {
                                                    item.action.execute(ctx, ui);
                                                }
                                            }
                                        }
                                        ui.close();
                                        item_clicked = true;
                                    }
                                }
                            });
                            can_view_separator = true;
                            has_visible_items = true;
                        }
                    }
                }
            }

            if can_view_separator {
                // 检查下一个组是否有可见项
                let next_has_visible = self.groups.iter()
                    .skip_while(|g| !std::ptr::eq(*g, group))
                    .skip(1)
                    .any(|g| {
                        if !g.should_show(ctx) {
                            return false;
                        }
                        g.items.iter().any(|item| match item {
                            MenuGroupItem::Item(mi) => mi.should_show(ctx),
                            MenuGroupItem::Submenu(s) => s.should_show(ctx),
                            MenuGroupItem::Separator => false,
                        })
                    });

                if next_has_visible {
                    ui.separator();
                    can_view_separator = false;
                }
            }
        }

        // 如果没有任何可见项，显示一个占位项
        if !has_visible_items {
            ui.label(tr("ctxmenu.no_action"));
        }

        item_clicked
    }
}

impl Default for ContextMenu {
    fn default() -> Self {
        Self::new()
    }
}
