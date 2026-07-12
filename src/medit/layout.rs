use core::f32;
use std::usize;

use eframe::egui::{
    Align, Event, EventFilter, FontId, ImeEvent, Key, Layout, CursorIcon,
    PointerButton, Rect, Response, ScrollArea, Sense, Ui, Vec2, ViewportCommand, Widget,
};

use crate::medit::{
    ctx::{HighlightRect, ScrollToLineMode},
    Action, Ctx, PghText, PghView, TextSpacing,
    TEXT_BOTTOM_SPACE, TEXT_TOP_SPACE, ctxmenu::{ContextMenu, EditorPluginMenuEntry},
    cfg::HeightMode,
};
use crate::medit::pgh::LayoutResponse;
use crate::uicom::CONTROL_HIGHLIGHT;

pub struct Edit<'a> {
    ctx: &'a mut Ctx,
    context_menu: ContextMenu,
}

impl<'a> Edit<'a> {
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self::new_with_plugin_command_items(ctx, Vec::new())
    }

    pub fn new_with_plugin_command_items(
        ctx: &'a mut Ctx,
        plugin_menu_items: Vec<EditorPluginMenuEntry>,
    ) -> Self {
        let mut context_menu = ContextMenu::new();
        context_menu.add_plugin_command_items(plugin_menu_items);
        Self { 
            ctx,
            context_menu,
        }
    }
}

impl Edit<'_> {
    #[cfg(windows)]
    fn should_suppress_backspace_after_ime_close() -> bool {
        // 避免“最后一次用于关闭候选窗的 Backspace”穿透到正文删除。
        crate::ime_win_bridge::tsf_win::should_suppress_backspace_after_ui_end(80)
    }

    #[cfg(not(windows))]
    fn should_suppress_backspace_after_ime_close() -> bool {
        false
    }

    fn draw_text_cursor(ui: &mut Ui, ctx: &mut Ctx, has_focus: bool) {
        if ctx.cfg().is_read_only {
            return;
        }
        let cursor = ctx.cursor2();
        if let Some(cursor_rect) = ctx.get_pos_from_cursor(&cursor) {
            let start = std::time::SystemTime::now();
            let since_the_epoch = start
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards");
            let milliseconds =
                since_the_epoch.as_secs() * 1000 + u64::from(since_the_epoch.subsec_millis());
            let should_show_cursor = !has_focus
                || ctx.is_ime_actived()
                || ctx.check_switch_cursor_show(milliseconds);
            if should_show_cursor {
                let cursor_rect = cursor_rect.expand2([1.0, 0.0].into());
                let paint_rect = ctx.edit_rect() | ctx.divider_rect();
                ui.painter_at(paint_rect).rect_filled(
                    cursor_rect,
                    0.0,
                    ui.style().visuals.text_cursor.stroke.color,
                );
            }
            ui.ctx().request_repaint_after_secs(0.5);
        }
    }

    fn draw_select_rect(ui: &mut Ui, ctx: &Ctx) {
        if let Some(highlight_rects) = ctx.get_heighlight_rects() {
            for hl_rect in highlight_rects {
                let (rect, color) = match hl_rect {
                    HighlightRect::Select(rect) => (rect, ctx.cfg().select_color().linear_multiply(0.5)),
                    HighlightRect::SameText(rect) => (rect, ctx.cfg().same_text_color().linear_multiply(0.5)),
                };
                ui.painter_at(ctx.edit_rect()).rect_filled(rect, 0.0, color);
            }
        }
    }

    fn cal_line_no_rect(ui: &mut Ui, ctx: &Ctx) -> Rect {
        let line_no_text = format!(" {}  ", ctx.line_num());
        let mut rect = PghText::guess_text_rect(ui, ctx, line_no_text, f32::INFINITY);
        if !ctx.cfg().show_line_no {
            rect.set_width(0.0);
        }
        rect
    }

    fn draw_line_no_text(
        ui: &mut Ui,
        ctx: &Ctx,
        pgh_rect: &Rect,
        line_no: &str,
        active: bool,
        sub_line: bool,
    ) -> Response {
        let mut line_no_rect = ctx.line_no_rect();
        line_no_rect.set_top(pgh_rect.top());
        line_no_rect.set_bottom(pgh_rect.bottom());

        let sense = if ctx.cfg().show_line_no && !sub_line {
            Sense::click()
        } else {
            Sense::hover()
        };
        let response = ui
            .allocate_rect(line_no_rect, sense)
            .on_hover_cursor(CursorIcon::Default);

        if !ctx.cfg().show_line_no {
            return response;
        }

        //line_no text
        let max_no_str = format!(" {}  ", ctx.line_num());
        let mut line_no_str = format!(" {}  ", line_no);
        for _ in line_no_str.len()..max_no_str.len() {
            line_no_str.insert(0, ' ');
        }

        //color
        let color = ui.style().visuals.weak_text_color();

        let spacing = TextSpacing::text_spacing_in_rect(ctx.line_no_rect(), core::f32::INFINITY);
        PghText::layout_text(
            ui,
            spacing,
            line_no_str,
            &None,
            line_no_rect.left_center() - Vec2::new(0.0, ctx.font_heigh()/2.0),
            color,
            None,
        );

        //hightlight current cursor line
        if active {
            let painter = ui.painter_at(ctx.line_no_rect());
            painter.rect_filled(line_no_rect, 0.0, ui.style().visuals.faint_bg_color);
            line_no_rect.set_width(2.0);
            painter.rect_filled(line_no_rect, 0.0, ui.style().visuals.selection.bg_fill);
        }

        response
    }

    fn draw_divider_line_for_rect(ui: &mut Ui, ctx: &Ctx, pgh_rect: &Rect) -> Response {
        let mut divider_rect = ctx.divider_rect();
        divider_rect.set_top(pgh_rect.top());
        divider_rect.set_bottom(pgh_rect.bottom());
        let response = ui.allocate_rect(divider_rect, Sense::hover());
        let painter = ui.painter_at(divider_rect);
        painter.rect_filled(divider_rect, 0.0, ui.style().visuals.faint_bg_color);
        response
    }

    fn draw_line_no_for_pgh(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        pgh_view: &PghView,
        pgh_rect: &Rect,
    ) -> Response {
        let mut response = ui.allocate_rect(
            Rect::from_min_max(pgh_rect.left_top(), pgh_rect.left_top()),
            Sense::hover(),
        );
        if pgh_view.is_code_row() {
            for segment in 0..=pgh_view.max_segment() {
                let active = ctx.cursor2().line_no() == line_no && ctx.cursor2().segment == segment;
                if let Some(seg_rect) = pgh_view.get_segment_rect(segment) {
                    response |= Self::draw_line_no_text(ui, ctx, &seg_rect, "", active, true);
                }
            }
        } else if pgh_view.is_table_row() {
            if let Some(ref seg_rect) = pgh_view.get_segment_rect(0) {
                let row_end = pgh_view.max_segment() + 1;
                let active = ctx.cursor2().line_no() == line_no
                    && (0..row_end).contains(&ctx.cursor2().segment);
                response |= Self::draw_line_no_text(ui, ctx, seg_rect, "", active, true);
            }
        } else if let Some(table_info) = ctx.table_info_of_line(line_no) {
            let row_count = if pgh_view.is_table_row() {
                1
            } else {
                pgh_view.pgh.len() / table_info.col_count.max(1)
            };
            for row in 0..row_count {
                let row_min = row * table_info.col_count;
                let row_end = (row+1) * table_info.col_count;
                if let Some(ref seg_rect) = pgh_view.get_segment_rect(row_min) {
                    let active = ctx.cursor2().line_no() == line_no && (row_min..row_end).contains(&ctx.cursor2().segment);
                    response |= Self::draw_line_no_text(ui, ctx, seg_rect, "", active, true);
                }
            }
        }
        {
            let active = ctx.cursor2().line_no() == line_no;
            let line_no_str = format!("{}", line_no+1);
            response |= Self::draw_line_no_text(ui, ctx, pgh_rect, &line_no_str, active, false);
        }
        response
    }

    fn draw_all_pgh(ui: &mut Ui, ctx: &mut Ctx, response: &mut LayoutResponse) {
        ctx.begin_layout_height_pass();
        let mut bottom_line = 0;
        ui.vertical(|ui| {
            for line_no in ctx.current_range() {
                if ctx.prepare_render_hidden_line_for_draw(line_no) {
                    continue;
                }
                ui.horizontal(|ui| {
                    ui.add_space(ctx.line_no_width());
                    ui.add_space(ctx.divider_rect().width());
                    ui.vertical(|ui| {
                        if let Some(is_line_changed) = PghView::parse_markdown_if_needed(ctx, line_no) {
                            let layout_response = PghView::layout(ui, ctx, line_no, is_line_changed);
                            ctx.record_line_scroll_height_after_layout(line_no);
                            if ui.is_rect_visible(layout_response.response.rect) {
                                if line_no > bottom_line {
                                    bottom_line = line_no;
                                }
                            }
                            response.response |= layout_response.response;
                            response.focus_response |= layout_response.focus_response;
                            if layout_response.handled {
                                response.handled = true;
                            }
                        }
                    });

                    // Get pgh_view and rect, then clone to avoid borrow checker issues
                    let (pgh_view_opt, rect_opt) = {
                        if let Some(layout_pgh_view) = ctx.get_line_pghview(line_no) {
                            (Some(layout_pgh_view.clone()), layout_pgh_view.rect())
                        } else {
                            (None, None)
                        }
                    };
                    if let (Some(pgh_view), Some(rect)) = (pgh_view_opt, rect_opt) {
                        let line_no_response =
                            Self::draw_line_no_for_pgh(ui, ctx, line_no, &pgh_view, &rect);
                        let divider_response = Self::draw_divider_line_for_rect(ui, ctx, &rect);
                        let line_no_clicked = line_no_response.clicked();
                        response.response |= line_no_response;
                        response.response |= divider_response;
                        if line_no_clicked {
                            ctx.select_line_to_next(line_no);
                            ctx.flash_same_cache_with_seleted();
                            response.handled = true;
                        }
                    }
                });
            }
            ctx.set_bottom_line(bottom_line);

            // 仅在绘制区间触达文档末尾时补底部留白，避免插在未绘制的中间区间
            if !ctx.is_dynamic_height() && ctx.patch_end() >= ctx.line_num() {
                let space = (ctx.edit_rect().height() / 2.0).max(0.0);
                ui.allocate_space(Vec2::new(0.0, space));
            }

            //scroll to the cursor pos
            Self::scroll_check(ui, ctx);
        });
        ctx.rebuild_index_if_layout_heights_changed();
    }

    fn draw_searching_highlight(ui: &mut Ui, ctx: &Ctx) {
        if !ctx.is_find_filter_searching() {
            return;
        }
        let base = ctx.line_no_rect();
        let progress = ctx.find_filter_search_progress().clamp(0.0, 1.0);
        let bar_h = base.height() * progress;
        if bar_h <= 0.0 {
            return;
        }
        let mut bar_rect = base;
        bar_rect.set_width(2.0);
        bar_rect.set_height(bar_h);
        ui.painter_at(bar_rect).rect_filled(bar_rect, 0.0, CONTROL_HIGHLIGHT);
    }

    fn draw_edit_area(ui: &mut Ui, ctx: &mut Ctx, response: &mut LayoutResponse) {
        Self::draw_searching_highlight(ui, ctx);
        ctx.highlight_refresh(ui);
        Self::draw_all_pgh(ui, ctx, response);
        Self::draw_select_rect(ui, ctx);

        //left space
        if !ctx.is_dynamic_height() {
            let mut left_space_rect = ctx.edit_rect();
            left_space_rect.set_top(response.response.rect.bottom());
            let r = ui.allocate_rect(left_space_rect, ctx.sense()).on_hover_cursor(CursorIcon::Text);
            if r.clicked() {
                ctx.set_cursor2_to_end();
                ctx.set_cursor1_reset();
            }
            response.response |= r;
        }
    }

    fn draw_edit_erea_in_scroll_viewport(ui: &mut Ui, ctx: &mut Ctx, response: &mut LayoutResponse) {
        ui.with_layout(Layout::top_down(Align::TOP), |ui| {
            let n = ctx.line_num();
            let total_lines_h = if n > 0 { ctx.scroll_cum_at(n) } else { 0.0 };
            let total_target = total_lines_h + ctx.scroll_bottom_padding();

            let scroll_area = if let Some(mode) = ctx.clean_scroll_to_line() {
                let offset_y = match mode {
                    ScrollToLineMode::Top(line) => ctx.scroll_offset_y_for_line(line),
                    ScrollToLineMode::Bottom(line) => ctx.scroll_offset_y_align_line_bottom(line),
                    ScrollToLineMode::Center(line) => ctx.scroll_offset_y_center_line(line),
                };
                ScrollArea::both()
                    .id_salt(("medit_edit_scroll", ctx.scroll_area_id()))
                    .vertical_scroll_offset(offset_y)
            } else {
                ScrollArea::both().id_salt(("medit_edit_scroll", ctx.scroll_area_id()))
            };
            scroll_area
                .min_scrolled_height(ctx.font_heigh())
                .auto_shrink(false)
                .animated(false)
                .show_viewport(ui, |ui, viewport| {
                    if n == 0 {
                        if ctx.ensure_non_empty_line_for_layout() {
                            ui.ctx().request_repaint();
                        }
                        return;
                    }

                    let margin = ctx.edit_rect().height().max(ctx.font_heigh() * 4.0);
                    //let (start_pre, end_pre) = ctx.scroll_lines_visible_for_viewport(&viewport, margin);
                    //ctx.rebuild_index_tick(start_pre, end_pre);
                    let (start, end_viewport) = ctx.scroll_lines_visible_for_viewport(&viewport, margin);
                    let min_visible_height = viewport.height() + 2.0 * margin;
                    let mut end = ctx.extend_layout_patch_end(start, end_viewport, min_visible_height);
                    end = end.max(start.saturating_add(1)).min(n);
                    ctx.set_top_line(start);
                    ctx.set_layout_patch_end(end);
                    ctx.request_rebuild_index_on_first_layout(start, end);
                    ctx.rebuild_index_tick(start, end);

                    let full_w = ctx.line_no_width() + ctx.edit_width(); //ui.available_width();
                    ui.vertical(|ui| {
                        ui.set_width(full_w);
                        let top_before = ui.cursor().top();
                        if start > 0 {
                            ui.add_space(ctx.scroll_cum_at(start));
                        }
                        Self::draw_edit_area(ui, ctx, response);
                        let tail = (ctx.scroll_cum_at(n) - ctx.scroll_cum_at(end)).max(0.0);
                        if tail > 0.0 {
                            ui.add_space(tail);
                        }
                        let gap = total_target - (ui.cursor().top() - top_before);
                        if gap > 0.0 {
                            ui.add_space(gap);
                        }
                    });
                });
        });
    }

    fn scroll_check(ui: &mut Ui, ctx: &mut Ctx) {
        //当选择到顶部/底部时，如果光标行接近或超出可见区域边缘，自动调整光标位置
        if ctx.is_selected() && ctx.is_selecting() && ctx.cursor1().line_no != ctx.cursor2().line_no {
            if let Some(curosr_line_rect) = ctx.get_cursor2_line_rect() {
                // 获取当前时间（毫秒）
                let start = std::time::SystemTime::now();
                let since_the_epoch = start
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("Time went backwards");
                let current_time = since_the_epoch.as_secs() * 1000 + u64::from(since_the_epoch.subsec_millis());
                
                // 检查时间间隔，防止自动滚动太快（至少间隔100毫秒）
                const AUTO_SCROLL_INTERVAL_MS: u64 = 100;
                let last_scroll_time = ctx.last_auto_scroll_time();
                let time_since_last_scroll = if current_time >= last_scroll_time {
                    current_time - last_scroll_time
                } else {
                    // 时间回退的情况，允许执行
                    AUTO_SCROLL_INTERVAL_MS + 1
                };
                
                // 只有当时间间隔足够时才执行自动调整
                if time_since_last_scroll >= AUTO_SCROLL_INTERVAL_MS {
                    let edit_rect = ctx.edit_rect();
                    let line_height = curosr_line_rect.height();
                    // 使用0.3倍行高作为接近边缘的阈值，避免在行完全可见时过早触发
                    let edge_threshold = line_height * 0.3;
                    
                    let line_top = curosr_line_rect.top();
                    let line_bottom = curosr_line_rect.bottom();
                    let edit_top = edit_rect.top();
                    let edit_bottom = edit_rect.bottom();
                    
                    // 向上滚动：当行的顶部超出或非常接近编辑区域顶部时
                    if line_top < edit_top || (line_top < edit_top + edge_threshold) {
                        ctx.cursor2_move_up();
                        ctx.set_last_auto_scroll_time(current_time);
                    } 
                    // 向下滚动：当行的底部超出或非常接近编辑区域底部时
                    else if line_bottom > edit_bottom || (line_bottom > edit_bottom - edge_threshold) {
                        ctx.cursor2_move_down();
                        ctx.set_last_auto_scroll_time(current_time);
                    }
                }
            }
        }

        //scroll to rect seted at pagedown/pageup event
        if let Some(rect) = ctx.clean_scroll_to_rect() {
            ui.scroll_to_rect(rect, Some(Align::TOP));
        }
        
        //cusror changed, ensure the corsor visible 
        if ctx.cursor2_cmp_and_bakup() {
            let c = ctx.cursor2();
            let edit_rect = ctx.edit_rect();
            // `top_line`/`bottom_line` 与用于推算可视行的 margin 组合后，行号上仍可能落在
            // 「名义可视」区间内，但插入符像素已经超出编辑区（常见：上移时光标贴在可视区上沿外）。
            let outside_line_window =
                c.line_no < ctx.top_line() || c.line_no > ctx.bottom_line();
            let caret = ctx.get_pos_from_cursor(&c);
            let caret_outside_v = caret.is_some_and(|r| {
                r.top() < edit_rect.top() || r.bottom() > edit_rect.bottom()
            });
            let caret_outside_h = caret.is_some_and(|r| {
                r.left() < edit_rect.left() || r.right() > edit_rect.right()
            });

            let need_v = outside_line_window || caret_outside_v;
            let need_h = caret_outside_h;

            if need_v || need_h {
                // 水平超出（以及同时垂直超出）：用 egui 在 ScrollArea 内滚动，两轴一次到位。
                // 若同时再 schedule `vertical_scroll_offset`，下一帧会覆盖纵向状态导致打架。
                if need_h {
                    if let Some(caret_rect) = caret {
                        log::info!("caret_rect: {:?}", caret_rect);
                        ui.scroll_to_rect(caret_rect, Some(Align::Center));
                    }
                }
                // 仅垂直超出：沿用 Top/Bottom/Center，减少跳跃感。
                if need_v && !need_h {
                    let outside_top = c.line_no < ctx.top_line();
                    let outside_bottom = c.line_no > ctx.bottom_line();
                    let mut scroll_up = outside_top;
                    let mut scroll_down = outside_bottom;
                    if let Some(cr) = caret {
                        if cr.top() < edit_rect.top() {
                            scroll_up = true;
                        }
                        if cr.bottom() > edit_rect.bottom() {
                            scroll_down = true;
                        }
                    }
                    if scroll_up && scroll_down {
                        ctx.set_scroll_to_line_mode(ScrollToLineMode::Center(c.line_no));
                    } else if scroll_up {
                        ctx.set_scroll_to_line_mode(ScrollToLineMode::Top(
                            c.line_no.saturating_sub(2),
                        ));
                    } else if scroll_down {
                        ctx.set_scroll_to_line_mode(ScrollToLineMode::Bottom(
                            c.line_no.saturating_add(2),
                        ));
                    } else {
                        ctx.set_scroll_to_line_mode(ScrollToLineMode::Center(c.line_no));
                    }
                }
            }
        }
    }
}

impl Widget for Edit<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let response = match self.ctx.cfg().height_mode {
            HeightMode::Fixed(h) => {
                if h == f32::INFINITY {
                    self.ui_with_frame_height(ui, None)
                } else {
                    self.ui_with_frame_height(ui, Some((h,h)))
                }
            }
            HeightMode::Dynamic { min, max } => {
                let max_height = self.ctx.get_view_height();
                let max_height = max_height.min(max);
                let min_height = min.min(max_height);
                self.ui_with_frame_height(ui, Some((min_height, max_height)))
            }
        };

        // layout 完成后保存计算出的高度
        let calculated_height = response.rect.height();
        self.ctx.set_saved_view_height(Some(calculated_height));

        response
    }
}

impl Edit<'_> {
    fn ui_with_frame_height(&mut self, ui: &mut Ui, view_height: Option<(f32,f32)>) -> Response {
        if let Some(frame) = self.ctx.cfg().with_frame.clone() {
            frame.show(ui, |ui| {
                if let Some((min, max)) =  view_height {
                    ui.set_min_height(min);
                    ui.set_max_height(max);
                }
                self.ui_impl(ui)
            }).inner
        } else {
            if let Some((min, max)) =  view_height {
                ui.set_min_height(min);
                ui.set_max_height(max);
            }
            self.ui_impl(ui)
        }
    }

    fn execute_deferred_editor_actions(ui: &mut Ui, ctx: &mut Ctx) {
        for action in ctx.take_deferred_editor_actions() {
            action.execute(ctx, ui);
        }
    }

    fn ui_impl(&mut self, ui: &mut Ui) -> Response {
        Self::execute_deferred_editor_actions(ui, self.ctx);

        //zero spacing between lines
        let spacing = ui.spacing_mut();
        spacing.item_spacing.x = 0.0;
        spacing.item_spacing.y = 0.0;
        spacing.icon_width = self.ctx.font_heigh() - 8.0;

        //get max rect
        let mut max_rect = ui.max_rect();
        max_rect.min.y = ui.cursor().top();

        //set font size
        let mut font_id = FontId::default();
        font_id.size = self.ctx.font_size();
        font_id.family = self.ctx.cfg().font_family();
        ui.style_mut().override_font_id = Some(font_id);

        //get current positon
        let line_no_rect = Self::cal_line_no_rect(ui, self.ctx);
        let scroll_style = ui.style().spacing.scroll;
        let scroll_bar_width = scroll_style.bar_width + scroll_style.bar_inner_margin + scroll_style.bar_outer_margin;
        self.ctx.set_rect(max_rect, line_no_rect.width(), scroll_bar_width);
        self.ctx.set_font_heigh(line_no_rect.height() + TEXT_TOP_SPACE + TEXT_BOTTOM_SPACE);

        //layout edit
        let top = ui.cursor().left_top();
        let initial_response = ui.allocate_rect(Rect::from_pos(top), self.ctx.sense());
        let mut layout_response = LayoutResponse::from_response(initial_response);
        Self::draw_edit_erea_in_scroll_viewport(ui, self.ctx, &mut layout_response);

        let event_handled = layout_response.handled;
        let focus_target = layout_response.focus_target();
        let response = &mut layout_response.response;

        //request focus
        if self.ctx.take_request_focus()
            || focus_target.clicked()
            || (focus_target.hovered() && ui.input(|i| i.pointer.any_pressed())) {
            ui.memory_mut(|mem| mem.request_focus(response.id));
            ui.ctx().send_viewport_cmd(ViewportCommand::IMEAllowed(true));
        }
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            if response.double_clicked() {
                self.ctx.set_cursor2_from_pos(&pointer_pos);
                self.ctx.select_word_at_cursor();
                self.ctx.flash_same_cache_with_seleted();
            } else if response.triple_clicked() {
                self.ctx.set_cursor2_from_pos(&pointer_pos);
                self.ctx.select_line_at_cursor();
            }
        }

        //context_menu
        let mut menu_item_clicked = false;
        response.context_menu(|ui|{
            menu_item_clicked = self.context_menu.show(ui, self.ctx);
        });
        
        // 如果菜单项被点击，重新请求焦点以确保编辑器可以继续接收输入
        if menu_item_clicked {
            ui.memory_mut(|mem| mem.request_focus(response.id));
        }

        //somthing when has focus
        let has_focus = ui.memory(|mem| mem.has_focus(response.id));
        if has_focus && !event_handled {
            //change image to image-link in clipboard
            if ui.input(|i| i.modifiers.ctrl) {
                if let Some(image_link) = self.ctx.try_get_image_from_clipboard() {
                    log::debug!("change image to image-link({}) in clipboard", image_link);
                    ui.ctx().copy_text(image_link);
                }
            }

            //process events
            let event_filter = EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            };
            ui.memory_mut(|mem| mem.set_focus_lock_filter(response.id, event_filter));
            let events = ui.input(|i| i.filtered_events(&event_filter));
            Self::process_all_event(ui, &mut self.ctx, events);

            //draw frame, todo
            //ui.painter().rect_stroke(self.ctx.edit_rect(), 0.0, Stroke::new(1.0, Color32::RED));
            //ui.painter().rect_stroke(self.ctx.line_no_rect(), 0.0, Stroke::new(1.0, Color32::RED));
        }

        //draw edit cursor
        Self::draw_text_cursor(ui, self.ctx, has_focus);

        return layout_response.response;
    }

    fn set_ime_cursor_area(ui: &mut Ui, ctx: &Ctx) {
        if let Some(rect) = ctx.get_pos_from_cursor(&ctx.cursor2()) {
            //ui.ctx().send_viewport_cmd(ViewportCommand::IMEAllowed(true));
            ui.ctx().send_viewport_cmd(ViewportCommand::IMERect(rect));
        }
    }

    fn on_mouse_event(ui: &mut Ui, ctx: &mut Ctx, event: &Event) -> bool {
        match event {
            Event::MouseMoved(_v) => true,
            Event::PointerMoved(pos) => {
                if ctx.is_selecting() {
                    //selecting
                    ctx.set_cursor2_from_pos(pos);
                    ctx.flash_same_cache_with_seleted();
                }
                true
            }
            Event::PointerGone => {
                true
            }
            Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            } => {
                if *button == PointerButton::Primary && *pressed && ctx.is_pos_in_edit_area(pos) && !ctx.is_pos_in_icon_or_checkbox(pos)  {
                    //left-button down
                    ctx.set_cursor2_from_pos(pos);
                    if !modifiers.shift {
                        ctx.set_cursor1_reset();
                    }
                    ctx.mark_selecting(true);
                } else if *button == PointerButton::Primary && !*pressed {
                    //left-button up
                    ctx.mark_selecting(false);
                    ctx.flash_same_cache_with_seleted();
                    Self::set_ime_cursor_area(ui, ctx);

                    //line click command（仅在正文区内松开，避免水平滚动条等区域误触）
                    if ctx.cfg().need_line_click_cmd && ctx.is_pos_in_edit_area(pos) && !ctx.is_selected() {
                        let line_txt = ctx.get_line_text(ctx.cursor2().line_no);
                        ctx.insert_cmd(Action::click_edit_line(line_txt));
                    }
                }
                true
            }
            Event::MouseWheel {
                unit: _,
                delta,
                modifiers,
            } => {
                if modifiers.ctrl {
                    log::debug!("{:?}", event);
                    ctx.add_font_size(delta.y * 1.5);
                }

                true
            }
            _ => false,
        }
    }

    fn on_key_event(ui: &mut Ui, ctx: &mut Ctx, event: &Event) -> bool {
        if ctx.is_ime_actived() {
            return false;
        }
        match event {
            Event::Key {
                modifiers,
                key,
                pressed: true,
                ..
            } => {
                log::debug!("{:?}", event);
                if ctx.is_ime_area_changed() {
                    Self::set_ime_cursor_area(ui, ctx);
                    ctx.set_ime_area_changed(false);
                }
                match key {
                    Key::Backspace => {
                        if !Self::should_suppress_backspace_after_ime_close() {
                            Action::backspace().execute(ctx, ui);
                        }
                    }
                    Key::Delete => {
                        Action::delete().execute(ctx, ui);
                    }
                    Key::ArrowLeft => {
                        Action::arrow_left(modifiers.shift).execute(ctx, ui);
                    }
                    Key::ArrowRight => {
                        Action::arrow_right(modifiers.shift).execute(ctx, ui);
                    }
                    Key::ArrowUp => {
                        Action::arrow_up(modifiers.shift).execute(ctx, ui);
                    }
                    Key::ArrowDown => {
                        Action::arrow_down(modifiers.shift).execute(ctx, ui);
                    }
                    Key::Home => {
                        Action::home(modifiers.shift).execute(ctx, ui);
                    }
                    Key::End => {
                        Action::end(modifiers.shift).execute(ctx, ui);
                    }
                    Key::PageDown => {
                        Action::page_down().execute(ctx, ui);
                    }
                    Key::PageUp => {
                        Action::page_up().execute(ctx, ui);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        false
    }

    //window.set_ime_cursor_area(LogicalPosition::new(cursor_pos[0], cursor_pos[1]), LogicalSize::new(100, 100));
    fn on_ime_event(_ui: &mut Ui, ctx: &mut Ctx, event: &Event) -> bool {
        match event {
            Event::Ime(ImeEvent::Commit(s)) => {
                log::debug!("{:?}", event);
                ctx.ime_commit(s.clone());
                return true;
            }
            Event::Ime(ImeEvent::Enabled) => {
                log::debug!("{:?}", event);
                ctx.ime_enable();
                return true;
            }
            Event::Ime(ImeEvent::Preedit(s)) => {
                log::debug!("{:?}", event);
                ctx.ime_preedit(s.clone());
                ctx.set_ime_actived(true);
                return true;
            }
            Event::Ime(ImeEvent::Disabled) => {
                log::debug!("{:?}", event);
                ctx.ime_disable();
                return true;
            }
            _ => {
                return false;
            }
        }
    }

    fn on_text_event(ui: &mut Ui, ctx: &mut Ctx, event: &Event) -> bool {
        match event {
            Event::Copy => {
                Action::copy().execute(ctx, ui);
            }
            Event::Cut => {
                Action::cut().execute(ctx, ui);
            }
            Event::Paste(text_to_insert) => {
                // Event::Paste 直接提供文本；Action::paste() 从剪贴板读取
                log::debug!("{:?}", event);
                Action::insert_text(text_to_insert.clone()).execute(ctx, ui);
            }
            Event::Text(text_to_insert) => {
                log::debug!("{:?}", event);
                Action::insert_text(text_to_insert.clone()).execute(ctx, ui);
            }

            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                if modifiers.ctrl {
                    match key {
                        Key::A => {
                            Action::select_all().execute(ctx, ui);
                        }
                        Key::Z => {
                            Action::undo().execute(ctx, ui);
                        }
                        Key::Y => {
                            Action::redo().execute(ctx, ui);
                        }
                        Key::S => {} //ctrl+s save
                        _=> {}
                    }
                }
                match key {
                    Key::Tab => {
                        Action::insert_tab().execute(ctx, ui);
                    }
                    Key::Enter => {
                        Action::enter(modifiers.ctrl).execute(ctx, ui);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        false
    }

    fn on_shortcut_event(ui: &mut Ui, ctx: &mut Ctx, event: &Event) -> bool {
        // 检查是否是按键事件
        if let Event::Key {
            modifiers,
            key,
            pressed: true,
            ..
        } = event {
            // 使用 ContextMenu 定义的默认 actions
            let actions = ContextMenu::default_actions();

            for action in actions {
                if let Some(shortcut_key) = action.shortcut_key() {
                    if shortcut_key.matches(key, modifiers) {
                        // 检查动作的条件是否满足（通过 ContextMenu 的 should_show 逻辑）
                        // 这里我们简化处理，直接执行动作
                        action.execute(ctx, ui);
                        return true;
                    }
                }
            }
        }
        false
    }

    fn on_event(ui: &mut Ui, ctx: &mut Ctx, event: &Event) {
        // 先处理快捷键，如果快捷键被处理了，就不继续处理其他事件
        if Self::on_shortcut_event(ui, ctx, event) {
            return;
        }
        Self::on_mouse_event(ui, ctx, event);
        Self::on_text_event(ui, ctx, event); 
        Self::on_key_event(ui, ctx, event);
        Self::on_ime_event(ui, ctx, event);
    }

    #[cfg(windows)]
    fn sync_ime_state_from_tsf(ctx: &mut Ctx) {
        if let Some(snapshot) = crate::ime_win_bridge::tsf_win::poll_tsf_snapshot() {
            let tsf_active = if snapshot.composition_sink_supported {
                snapshot.composing || snapshot.ui_open
            } else {
                // 在搜狗环境里 composition sink 不可连接时，UI 元素生命周期是更稳定信号。
                snapshot.ui_open
            };
            if ctx.is_ime_actived() != tsf_active {
                ctx.set_ime_actived(tsf_active);
            }
        }
    }

    #[cfg(not(windows))]
    fn sync_ime_state_from_tsf(_ctx: &mut Ctx) {}

    #[allow(unused_assignments)]
    fn process_all_event(ui: &mut Ui, ctx: &mut Ctx, events: Vec<Event>) {
        if events.len() == 0 {
            return;
        }

        let has_key_event = events
            .iter()
            .any(|event| matches!(event, Event::Key { .. }));

        if has_key_event {
            Self::sync_ime_state_from_tsf(ctx);
        }

        for event in &events {
            Self::on_event(ui, ctx, event);
        }

        // 某些 TSF 回调会在本帧事件处理后到达，末尾再同步一次可减少一帧延迟。
        if has_key_event {
            Self::sync_ime_state_from_tsf(ctx);
        }

    }
}
