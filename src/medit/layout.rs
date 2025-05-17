use core::f32;
use std::usize;

use eframe::egui::{
    Align, CursorIcon, Event, EventFilter, FontId, ImeEvent, Key, Layout, Order, 
    PointerButton, Rect, Response, ScrollArea, Ui, Vec2, ViewportCommand, Widget
};

use crate::medit::{Ctx, Command, PghText, PghView, TEXT_TOP_SPACE, TEXT_BOTTOM_SPACE};

pub struct Edit<'a> {
    ctx: &'a mut Ctx,
}

impl<'a> Edit<'a> {
    pub fn new(ctx: &'a mut Ctx) -> Self {
        Self { ctx }
    }
}

impl Edit<'_> {
    fn draw_text_cursor(ui: &mut Ui, ctx: &mut Ctx, has_focus: bool) {
        let cursor = ctx.cursor2();
        if let Some(cursor_rect) = ctx.get_pos_from_cursor(&cursor) {
            let start = std::time::SystemTime::now();
            let since_the_epoch = start
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Time went backwards");
            let milliseconds =
                since_the_epoch.as_secs() * 1000 + u64::from(since_the_epoch.subsec_millis());
            if !has_focus || ctx.check_switch_cursor_show(milliseconds) {
                let cursor_rect = cursor_rect.expand2([1.0, 0.0].into());
                ui.painter().rect_filled(
                    cursor_rect,
                    0.0,
                    ui.style().visuals.text_cursor.stroke.color,
                );
            }
            ui.ctx().request_repaint_after_secs(0.5);
        }
    }

    fn draw_select_rect(ui: &mut Ui, ctx: &Ctx) {
        if let Some(rects) = ctx.get_cursor_rects() {
            //println!("get_cursor_rects:{:?}", rects);
            for rect in rects {
                //let rect = rect.expand2(Vec2 { x: 0.0, y: 1.0 });
                //let color = Color32::from_rgb(111, 111, 11);
                let color = ui.style().visuals.selection.bg_fill.linear_multiply(0.5);
                //let color = visuals.selection.bg_fill.linear_multiply(0.5);
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

    fn draw_line_no_text(ui: &mut Ui, ctx: &Ctx, pgh_rect: &Rect, line_no: usize, active: bool, sub_line: bool) {
        //line_no text
        let max_no_str = format!(" {}  ", ctx.line_num());
        let mut line_no_str = format!(" {}  ", line_no + 1);
        for _ in line_no_str.len()..max_no_str.len() {
            line_no_str.insert(0, ' ');
        }

        //line_no rect
        let mut line_no_rect = ctx.line_no_rect();
        line_no_rect.set_top(pgh_rect.top());
        line_no_rect.set_bottom(pgh_rect.bottom());

        //println!("line_no:{} top:{}", line_no, line_no_rect.top());

        //color
        let color = if sub_line {
            ui.style().visuals.weak_text_color()
        } else {
            ui.style().visuals.weak_text_color()
        };

        PghText::layout_text(
            ui,
            ctx.line_no_rect(),
            line_no_str,
            &None,
            line_no_rect.left_center() - Vec2::new(0.0, ctx.font_heigh()/2.0),
            color,
            None,
            core::f32::INFINITY,
        );

        //height current cursor line
        if active {
            //let stroke = (1.0, ui.style().visuals.selection.bg_fill);
            line_no_rect.set_width(2.0);
            //ui.painter().rect_stroke(line_no_rect, 0.0, stroke);
            ui.painter().rect_filled(line_no_rect, 0.0, ui.style().visuals.selection.bg_fill);
        }
    }

    fn draw_line_no_rect(ui: &mut Ui, ctx: &mut Ctx) {
        ui.with_layout(Layout::top_down(Align::Max), |ui| {
            let line_no_rect = ctx.line_no_rect();
            //ui.painter()
            //    .rect_filled(line_no_rect, 0.0, ui.style().visuals.faint_bg_color);

            for (line_no, pgh_view) in ctx.current_range_clone() {
                if pgh_view.is_code() {
                    for segment in 0..=pgh_view.max_segment() {
                        let active = ctx.cursor2().line_no() == line_no && ctx.cursor2().segment == segment;
                        if let Some(seg_rect) = pgh_view.get_segment_rect(segment) {
                            Self::draw_line_no_text(ui, ctx, &seg_rect, segment, active, true);
                        }
                    }
                } else if let Some(table_info) = &pgh_view.table_info {
                    for row in 0..table_info.row_count {
                        let row_min = row * table_info.col_count;
                        let row_end = (row+1) * table_info.col_count;
                        if let Some(ref seg_rect) = pgh_view.get_segment_rect(row_min) {
                            let active = ctx.cursor2().line_no() == line_no && (row_min..row_end).contains(&ctx.cursor2().segment);
                            Self::draw_line_no_text(ui, ctx, seg_rect, row, active, true);
                        }
                    }
                }
                else if let Some(pgh_rect) = pgh_view.rect {
                    let active = ctx.cursor2().line_no() == line_no;
                    Self::draw_line_no_text(ui, ctx, &pgh_rect, line_no, active, false);
                }
            }
        });
    }

    fn draw_all_pgh(ui: &mut Ui, ctx: &mut Ctx, response: &mut Response) {
        ui.vertical(|ui| {
            let mut bottom_line = 0;
            for (line_no, pgh_view) in ctx.current_range_clone() {
                let r = PghView::layout(ui, ctx, line_no, &pgh_view);
                if ui.is_rect_visible(r.rect) {
                    if line_no > bottom_line {
                        bottom_line = line_no;
                    }
                }
                //ui.painter().rect_stroke(r.rect, 0.0, Stroke::new(1.0, Color32::RED));  //todo
                *response |= r;
            }
            ctx.set_bottom_line(bottom_line);
            let space = (ctx.edit_rect().height()/2.0).max(0.0);
            ui.allocate_space(Vec2::new(0.0, space));
        });
    }

    fn draw_edit_area(ui: &mut Ui, ctx: &mut Ctx, response: &mut Response) {
        ctx.highlight_refresh(ui);
        Self::draw_all_pgh(ui, ctx, response);

        Self::draw_select_rect(ui, ctx);
    }

    fn draw_edit_erea_in_scroll_rows(ui: &mut Ui, ctx: &mut Ctx, response: &mut Response) {
        ui.with_layout(Layout::top_down(Align::TOP), |ui| {
            let scroll_area = if let Some(scroll_to_line) = ctx.clean_scroll_to_line() {
                let offset_y = scroll_to_line as f32 * ctx.font_heigh();
                ScrollArea::both().vertical_scroll_offset(offset_y)
            } else {
                ScrollArea::both()
            };
            scroll_area.auto_shrink(false).show_rows(
                ui,
                ctx.font_heigh(),
                ctx.line_num(),
                |ui, row_range| {
                    ctx.set_top_line(row_range.start);
                    Self::draw_edit_area(ui, ctx, response);
                },
            );
        });
    }

    fn draw_edit_erea_in_scroll_normal(ui: &mut Ui, ctx: &mut Ctx, response: &mut Response) {
        ui.with_layout(Layout::top_down(Align::TOP), |ui| {
            ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                Self::draw_edit_area(ui, ctx, response);
            });
        });
    }

    fn scroll_check(ui: &mut Ui, ctx: &mut Ctx) {
        if let Some(rect) = ctx.clean_scroll_to_rect() {
            println!("need scroll_to_rect: {:?}", rect);
            ui.scroll_to_rect(rect, Some(Align::TOP));
        }
        
        if ctx.is_selected() && ctx.is_selecting() && ctx.cursor1().line_no != ctx.cursor2().line_no {
            if let Some(curosr_line_rect) = ctx.get_cursor2_line_rect() {
                let top_min = curosr_line_rect.top() - curosr_line_rect.height();
                let bottom_max = curosr_line_rect.bottom() + curosr_line_rect.height();
                if top_min < ctx.edit_rect().top() {
                    ctx.cursor2_move_up();
                    let rect = curosr_line_rect.translate(Vec2::new(0.0, -ctx.font_heigh()));
                    ui.scroll_to_rect(rect, None);
                    return;
                } else if bottom_max > ctx.edit_rect().bottom() {
                    ctx.cursor2_move_down();
                    let rect = curosr_line_rect.translate(Vec2::new(0.0, ctx.font_heigh()));
                    ui.scroll_to_rect(rect, None);
                    return;
                }
            }
        }
        
        let cursor_changed = ctx.cursor2_cmp_and_bakup();
        if cursor_changed {
            if !ctx.cfg().is_markdown {
                let c = ctx.cursor2();
                if c.line_no < ctx.top_line() || c.line_no > ctx.bottom_line() {
                    let line_no = c.line_no.saturating_sub((ctx.edit_rect().height()/2.0/ctx.font_heigh()) as usize);
                    ctx.set_scroll_to_line(line_no);
                }
            } else {
                if let Some(mut curosr_line_rect) = ctx.get_cursor2_line_rect() {
                    curosr_line_rect.set_left(ctx.edit_rect().left());
                    curosr_line_rect.set_right(ctx.edit_rect().right());
                    let is_fully_visible = ui.clip_rect().contains_rect(curosr_line_rect);
                    if !is_fully_visible {
                        ui.scroll_to_rect(curosr_line_rect, None);
                        return;
                    }
                }
            }
        }
    }
}

impl Widget for Edit<'_> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        //zero spacing between lines
        let spacing = ui.spacing_mut();
        spacing.item_spacing.x = 0.0;
        spacing.item_spacing.y = 0.0;
        spacing.icon_width = self.ctx.font_heigh() - 8.0;

        //set font size
        let mut font_id = FontId::default();
        font_id.size = self.ctx.font_size();
        font_id.family = eframe::egui::FontFamily::Monospace; //todo, set in self.ctx
        ui.style_mut().override_font_id = Some(font_id);

        //get current positon
        let line_no_rect = Self::cal_line_no_rect(ui, self.ctx);
        let scroll_style = ui.style().spacing.scroll;
        let scroll_bar_width = scroll_style.bar_width + scroll_style.bar_inner_margin + scroll_style.bar_outer_margin;
        let mut max_rect = ui.max_rect();
        max_rect.min.y = ui.cursor().top();
        self.ctx.set_rect(max_rect, line_no_rect.width(), scroll_bar_width);
        self.ctx.set_font_heigh(line_no_rect.height() + TEXT_TOP_SPACE + TEXT_BOTTOM_SPACE);

        //scroll to the cursor pos
        Self::scroll_check(ui, self.ctx);
        
        //layout
        let top = ui.cursor().left_top();
        let mut response = ui.allocate_rect(Rect::from_pos(top), self.ctx.sense());
        ui.horizontal(|ui| {
            //allocate line no rect
            ui.allocate_rect(self.ctx.line_no_rect(), self.ctx.sense());

            //edit
            if self.ctx.cfg().is_markdown {
                Self::draw_edit_erea_in_scroll_normal(ui, self.ctx, &mut response);
            } else {
                Self::draw_edit_erea_in_scroll_rows(ui, self.ctx, &mut response);
            }

            //draw line_no
            if self.ctx.cfg().show_line_no {
                Self::draw_line_no_rect(ui, self.ctx);
            }
        });
        
        //left space
        let mut left_space_rect = max_rect;
        left_space_rect.set_top(response.rect.bottom());
        let r = ui.allocate_rect(left_space_rect, self.ctx.sense());
        if r.clicked() {
            self.ctx.set_cursor2_to_end();
            self.ctx.set_cursor1_reset();
        }
        response |= r;

        //cursor icon
        response.clone().on_hover_cursor(CursorIcon::Text);

        //request focus
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            if response.clicked() || 
               (response.hovered() && ui.input(|i| i.pointer.any_pressed())) {   
                //let focused = ui.memory(|mem| mem.focused());
                //println!("clicked or any_pressed, focused:{:?}, my-id:{:?} {}", focused, response.id, response.has_focus());
                ui.memory_mut(|mem| mem.request_focus(response.id));
            }
            if response.double_clicked() {
                self.ctx.set_cursor2_from_pos(&pointer_pos);
                self.ctx.select_word_at_cursor();
                self.ctx.flash_same_cache_with_seleted();
            } else if response.triple_clicked() {
                self.ctx.set_cursor2_from_pos(&pointer_pos);
                self.ctx.select_line_at_cursor();
            }
        }

        //context_menu, todo
        if self.ctx.is_selected(){
            response.context_menu(|ui|{
                let _ = ui.button("todo");
                let _ = ui.button("todo");
            });
        }

        //somthing when has focus
        let has_focus = ui.memory(|mem| mem.has_focus(response.id));
        if has_focus {
            //change image to image-link in clipboard
            if ui.input(|i| i.modifiers.ctrl) {
                if let Some(image_link) = self.ctx.try_get_image_from_clipboard() {
                    println!("change image to image-link({}) in clipboard", image_link);
                    ui.ctx().copy_text(image_link);
                }
            }

            //process events
            let bak_state = self.ctx.clone_state();
            let event_filter = EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            };
            ui.memory_mut(|mem| mem.set_focus_lock_filter(response.id, event_filter));
            let events = ui.input(|i| i.filtered_events(&event_filter));
            for event in &events {
                //println!("has focus, do envent: {:?}", event);
                Self::on_event(ui, &mut self.ctx, event);
            }
            //compare state and mark_state_change after process event
            self.ctx.mark_state_change(bak_state);

            //draw frame, todo
            //ui.painter().rect_stroke(self.ctx.edit_rect(), 0.0, Stroke::new(1.0, Color32::RED));
            //ui.painter().rect_stroke(self.ctx.line_no_rect(), 0.0, Stroke::new(1.0, Color32::RED));

            
        }

        //draw edit cursor
        Self::draw_text_cursor(ui, self.ctx, has_focus);

        return response;
    }
}

impl Edit<'_> {
    fn set_ime_cursor_area(ui: &mut Ui, ctx: &Ctx) {
        if let Some(rect) = ctx.get_pos_from_cursor(&ctx.cursor2()) {
            ui.ctx().send_viewport_cmd(ViewportCommand::IMEAllowed(true));
            ui.ctx().send_viewport_cmd(ViewportCommand::IMERect(rect));
        }
    }

    fn on_mouse_event(ui: &mut Ui, ctx: &mut Ctx, event: &Event) -> bool {
        match event {
            Event::MouseMoved(v) => true,
            Event::PointerMoved(pos) => {
                //println!("{:?}", event);
                ctx.mark_pointer_gone(false);
                if ctx.is_selecting() {
                    //selecting
                    if ctx.is_pos_in_edit_area(pos) {
                        ctx.set_cursor2_from_pos(pos);
                        ctx.flash_same_cache_with_seleted();
                    }
                }
                true
            }
            Event::PointerGone => {
                //println!("{:?}", event);
                ctx.mark_pointer_gone(true);
                true
            }
            Event::PointerButton {
                pos,
                button,
                pressed,
                modifiers,
            } => {
                //println!("{:?}", event);
                if *button == PointerButton::Primary && *pressed && ctx.is_pos_in_edit_area(pos) {
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

                    //line click command
                    if ctx.cfg().need_line_click_cmd {
                        let line_txt = ctx.get_line_text(ctx.cursor2().line_no);
                        ctx.insert_cmd(Command::ClickEditLine(line_txt));
                    }
                }
                true
            }
            Event::MouseWheel {
                unit,
                delta,
                modifiers,
            } => {
                if modifiers.ctrl {
                    println!("{:?}", event);
                    ctx.add_font_size(delta.y * 1.5);
                }

                true
            }
            _ => false,
        }
    }

    fn on_key_event(ui: &mut Ui, ctx: &mut Ctx, event: &Event) -> bool {
        match event {
            Event::Key {
                modifiers,
                key,
                pressed: true,
                ..
            } => {
                if ctx.is_ime_area_changed() {
                    Self::set_ime_cursor_area(ui, ctx);
                    ctx.set_ime_area_changed(false);
                }
                match key {
                    Key::Backspace => {
                        if ctx.is_selected() {
                            ctx.delete();
                        } else {
                            ctx.cursor2_move_prev();
                            ctx.set_cursor_switch();
                            ctx.delete();
                        }
                    }
                    Key::Delete => {
                        if ctx.is_selected() {
                            ctx.delete();
                        } else {
                            ctx.cursor2_move_next();
                            ctx.set_cursor_switch();
                            ctx.delete();
                        }
                    }
                    Key::ArrowLeft => {
                        ctx.cursor2_move_prev();
                        if !modifiers.shift {
                            ctx.set_cursor1_reset();
                        }
                    }
                    Key::ArrowRight => {
                        ctx.cursor2_move_next();
                        if !modifiers.shift {
                            ctx.set_cursor1_reset();
                        }
                    }
                    Key::ArrowUp => {
                        ctx.cursor2_move_up();
                        if !modifiers.shift {
                            ctx.set_cursor1_reset();
                        }
                    }
                    Key::ArrowDown => {
                        ctx.cursor2_move_down();
                        if !modifiers.shift {
                            ctx.set_cursor1_reset();
                        }
                    }
                    Key::Home => {
                        ctx.cursor2_move_home();
                        if !modifiers.shift {
                            ctx.set_cursor1_reset();
                        }
                    }
                    Key::End => {
                        ctx.cursor2_move_end();
                        if !modifiers.shift {
                            ctx.set_cursor1_reset();
                        }
                    }
                    Key::PageDown => {
                        let mut rect = ui.cursor();
                        rect.set_height(ctx.font_heigh());
                        ctx.set_scroll_to_rect(rect);
                    }
                    Key::PageUp => {
                        let mut rect = ui.cursor();
                        rect.set_height(ctx.font_heigh());
                        let rect = rect.translate(Vec2::new(0.0, -ctx.edit_rect().height()*2.0));
                        ctx.set_scroll_to_rect(rect);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        false
    }

    //window.set_ime_cursor_area(LogicalPosition::new(cursor_pos[0], cursor_pos[1]), LogicalSize::new(100, 100));
    fn on_ime_event(ui: &mut Ui, ctx: &mut Ctx, event: &Event) -> bool {
        match event {
            Event::Ime(ImeEvent::Commit(s)) => {
                println!("{:?}", event);
                ctx.insert(s.clone());
                return true;
            }
            Event::Ime(ImeEvent::Enabled) => {
                println!("{:?}", event);
                return true;
            }
            Event::Ime(ImeEvent::Preedit(s)) => {
                println!("{:?}", event);
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
                let text = ctx.get_selected_text();
                if text.len() > 0 {
                    //println!("Copy [{}]", &text);
                    ui.ctx().copy_text(text);
                }
            }
            Event::Cut => {
                let text = ctx.get_selected_text();
                if text.len() > 0 {
                    //println!("Cut [{}]", &text);
                    ui.ctx().copy_text(text);
                }
                ctx.delete();
            }
            Event::Paste(text_to_insert) => {
                //println!("Paste [{}]", text_to_insert);
                ctx.insert(text_to_insert.clone());
            }
            Event::Text(text_to_insert) => {
                ctx.insert(text_to_insert.clone());
            }

            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                match key {
                    Key::Tab => {
                        //todo
                        //1.multi-line
                        //2.tab always 4 space
                        ctx.insert("\t".to_string());
                    }
                    Key::Enter => {
                        ctx.enter(modifiers.ctrl);
                    }
                    Key::A if modifiers.ctrl => {
                        //ctrl+a select all
                        ctx.set_cursors_select_all();
                    }
                    Key::Z if modifiers.ctrl => {   
                        //ctrl+z undo
                        ctx.undo();
                    }
                    Key::Y if modifiers.ctrl => {   
                        //ctrl+y redo
                        ctx.redo();
                    }
                    Key::S if modifiers.ctrl => {   
                        //ctrl+s save
                        //todo
                        println!("ctrl+s in editor");
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        false
    }

    fn is_on_top(ui: &mut Ui) -> bool {
        //todo, don't know how to contrl edit focus
        let top_layer_id = ui.ctx().memory(|mem|mem.areas().top_layer_id(Order::Middle));
        let self_layer_id = ui.layer_id();
        //println!("top_layer_id:{:?} self_layer_id:{:?}", top_layer_id, self_layer_id);
        return top_layer_id == Some(self_layer_id);
    }

    fn on_event(ui: &mut Ui, ctx: &mut Ctx, event: &Event) {
        Self::on_mouse_event(ui, ctx, event);
        Self::on_key_event(ui, ctx, event);
        Self::on_ime_event(ui, ctx, event);
        Self::on_text_event(ui, ctx, event); 
    }
}
