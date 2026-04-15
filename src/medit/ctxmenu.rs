use crate::medit::{Ctx, PghType, Action};
use crate::i18n::tr;
use eframe::egui::{NumExt, Rect, Sense, Ui};

/// 菜单项显示条件
#[derive(Clone)]
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
            MenuCondition::Not(Box::new(MenuCondition::CurrentIsSome(PghType::Table))),
            MenuCondition::Not(Box::new(MenuCondition::CurrentIsSome(PghType::TableRow))),
        ])
    }

    pub fn selected_normal_text_condition() -> MenuCondition {
        MenuCondition::And(vec![
            MenuCondition::HasSelection,
            MenuCondition::Not(Box::new(MenuCondition::SelectIncludeSome(PghType::CodeRow))),
            MenuCondition::Not(Box::new(MenuCondition::SelectIncludeSome(PghType::Table))),
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
            MenuCondition::current_is_not(PghType::Table),
            MenuCondition::current_is_not(PghType::TableRow),
            MenuCondition::NoSelection,
        ])
    }

    /// 光标在 Markdown 表格内（`Table` 或 `TableRow`），且非只读。
    pub fn in_markdown_table_cursor() -> MenuCondition {
        MenuCondition::And(vec![
            MenuCondition::IsMarkdown,
            MenuCondition::IsNotReadOnly,
            MenuCondition::Or(vec![
                MenuCondition::CurrentIsSome(PghType::Table),
                MenuCondition::CurrentIsSome(PghType::TableRow),
            ]),
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
}

impl MenuItem {
    pub fn new(text: String, condition: MenuCondition, action: Action) -> Self {
        Self {
            text,
            condition,
            action: action.clone(),
            shortcut: action.shortcut_string(),
        }
    }

    pub fn with_shortcut(mut self, shortcut: String) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// 检查是否应该显示
    pub fn should_show(&self, ctx: &Ctx) -> bool {
        self.condition.check(ctx)
    }

    /// 显示文本（包含快捷键）
    pub fn display_text(&self) -> String {
        if let Some(shortcut) = &self.shortcut {
            format!("{}\t{}", self.text, shortcut)
        } else {
            self.text.clone()
        }
    }
}

/// 菜单组（可以包含分隔符）
#[derive(Clone)]
pub enum MenuGroupItem {
    /// 普通菜单项
    Item(MenuItem),
    /// 分隔符
    Separator,
}

/// 菜单组
#[derive(Clone)]
pub struct MenuGroup {
    /// 组名（可选，用于插件识别）
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

    /// 检查是否应该显示整个组
    pub fn should_show(&self, ctx: &Ctx) -> bool {
        if let Some(condition) = &self.condition {
            condition.check(ctx)
        } else {
            true
        }
    }
}

/// 上下文菜单管理器
pub struct ContextMenu {
    /// 菜单组列表
    groups: Vec<MenuGroup>,
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
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.italic"),
                MenuCondition::SingleLineOrNoSelection,
                Action::italic(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.strikethrough"),
                MenuCondition::SingleLineOrNoSelection,
                Action::strikethrough(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.link"),
                MenuCondition::SingleLineOrNoSelection,
                Action::link(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.inline_code"),
                MenuCondition::SingleLineOrNoSelection,
                Action::code(),
            ))
            .add_separator()
            .add_item(MenuItem::new(
                tr("ctxmenu.todo_list"),
                MenuCondition::list_condition(),
                Action::todo_list(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.unordered_list"),
                MenuCondition::list_condition(),
                Action::unordered_list(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.ordered_list"),
                MenuCondition::list_condition(),
                Action::ordered_list(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.quote"),
                MenuCondition::list_condition(),
                Action::quote(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.code_block"),
                MenuCondition::code_condition(),
                Action::code_block(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table"),
                MenuCondition::table_condition(),
                Action::table(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.horizontal_rule"),
                MenuCondition::table_condition(),
                Action::horizontal_rule(),
            ))
            .add_separator()
            .add_item(MenuItem::new(
                tr("ctxmenu.heading1"),
                MenuCondition::heading_condition(),
                Action::heading(1),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading2"),
                MenuCondition::heading_condition(),
                Action::heading(2),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading3"),
                MenuCondition::heading_condition(),
                Action::heading(3),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading4"),
                MenuCondition::heading_condition(),
                Action::heading(4),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading5"),
                MenuCondition::heading_condition(),
                Action::heading(5),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.heading6"),
                MenuCondition::heading_condition(),
                Action::heading(6),
            ));

        self.groups.push(markdown_format_group);

        let table_ops_group = MenuGroup::new(Some("markdown_table_ops".to_string()))
            .with_condition(MenuCondition::in_markdown_table_cursor())
            .add_item(MenuItem::new(
                tr("ctxmenu.table_delete_selected_rows"),
                MenuCondition::Always,
                Action::table_delete_selected_rows(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_delete_selected_cols"),
                MenuCondition::Always,
                Action::table_delete_selected_cols(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_insert_row_above"),
                MenuCondition::Always,
                Action::table_insert_row_above(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_insert_row_below"),
                MenuCondition::Always,
                Action::table_insert_row_below(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_insert_col_left"),
                MenuCondition::Always,
                Action::table_insert_col_left(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.table_insert_col_above"),
                MenuCondition::Always,
                Action::table_insert_col_right(),
            ));
        self.groups.push(table_ops_group);

        // 基本编辑组（所有文档）
        let basic_group = MenuGroup::new(Some("basic".to_string()))
            .add_item(MenuItem::new(
                tr("ctxmenu.copy"),
                MenuCondition::HasSelection,
                Action::copy(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.cut"),
                MenuCondition::And(vec![
                    MenuCondition::HasSelection,
                    MenuCondition::IsNotReadOnly,
                ]),
                Action::cut(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.paste"),
                MenuCondition::IsNotReadOnly,
                Action::paste(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.delete"),
                MenuCondition::And(vec![
                    MenuCondition::HasSelection,
                    MenuCondition::IsNotReadOnly,
                ]),
                Action::delete(),
            ))
            .add_item(MenuItem::new(
                tr("ctxmenu.select_all"),
                MenuCondition::Always,
                Action::select_all(),
            ));

        self.groups.push(basic_group);
    
    }

    /// 添加菜单组（用于插件扩展）
    pub fn add_group(&mut self, group: MenuGroup) {
        self.groups.push(group);
    }

    /// 在指定组名后插入菜单组（用于插件扩展）
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
    pub fn replace_group(&mut self, group_name: &str, group: MenuGroup) {
        if let Some(index) = self.groups.iter().position(|g| {
            g.name.as_ref().map(|n| n == group_name).unwrap_or(false)
        }) {
            self.groups[index] = group;
        } else {
            self.groups.push(group);
        }
    }

    /// 显示上下文菜单
    /// 返回 true 表示有菜单项被点击
    pub fn show(&self, ui: &mut Ui, ctx: &mut Ctx) -> bool {
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
                            // 固定菜单按钮宽度
                            const MENU_BUTTON_WIDTH: f32 = 200.0;
                            
                            // 先分配整行的可点击区域
                            let row_height = ui.spacing().interact_size.y;
                            let full_width = ui.available_width().at_most(MENU_BUTTON_WIDTH);
                            let row_rect = Rect::from_min_size(
                                ui.cursor().left_top(),
                                eframe::egui::Vec2::new(full_width, row_height)
                            );
                            let row_response = ui.allocate_rect(row_rect, Sense::click());
                            
                            // 绘制悬停效果
                            if row_response.hovered() {
                                ui.painter().rect_filled(
                                    row_rect,
                                    0.0,
                                    ui.style().visuals.widgets.hovered.weak_bg_fill
                                );
                            }
                            
                            // 在分配的区域内绘制内容
                            // 使用 painter 直接绘制文本，避免使用 child_ui
                            let text_style = ui.style().text_styles.get(&eframe::egui::TextStyle::Body).cloned().unwrap_or_default();
                            let text_color = ui.style().visuals.text_color();
                            let weak_color = ui.style().visuals.weak_text_color();
                            
                            // 绘制菜单文本（左侧，限制在固定宽度内）
                            let text_pos = row_rect.left_center() + eframe::egui::Vec2::new(4.0, 0.0);
                            let text_rect = Rect::from_min_size(
                                row_rect.left_top(),
                                eframe::egui::Vec2::new(MENU_BUTTON_WIDTH, row_height)
                            );
                            ui.painter().text(
                                text_pos,
                                eframe::egui::Align2::LEFT_CENTER,
                                &menu_item.text,
                                text_style.clone(),
                                text_color
                            );
                            
                            // 绘制快捷键（右侧）
                            if let Some(shortcut) = &menu_item.shortcut {
                                let shortcut_pos = row_rect.right_center() - eframe::egui::Vec2::new(4.0, 0.0);
                                ui.painter().text(
                                    shortcut_pos,
                                    eframe::egui::Align2::RIGHT_CENTER,
                                    shortcut,
                                    text_style,
                                    weak_color
                                );
                            }
                            
                            if row_response.clicked() {
                                menu_item.action.execute(ctx, ui);
                                ui.close();
                                item_clicked = true;
                            }
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
                        g.items.iter().any(|item| {
                            if let MenuGroupItem::Item(mi) = item {
                                mi.should_show(ctx)
                            } else {
                                false
                            }
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
           