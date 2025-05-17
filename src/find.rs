
use eframe::egui::{Button, Event, EventFilter, Key, Order, Rect, ScrollArea, TextEdit, Ui, Vec2, Widget, Window};
use crate::medit::{ctx::FindCache, Ctx, FindCmd, FindReplaceCtx};

pub struct FindWindow {
    is_show: bool,
    is_window: bool,
    need_focus: bool,
    replace_ready: bool,
    is_open_replace: bool,
    param: FindReplaceCtx,
    pub edit_ctx: Ctx,
}

impl FindWindow {
    pub fn new() -> Self {
        let mut edit_ctx = Ctx::new("", false, None);
        edit_ctx.cfg_mut().need_line_click_cmd = true;
        Self {
            is_show: false,
            is_window: true,
            need_focus: false,
            replace_ready: false,
            edit_ctx,
            is_open_replace: false,
            param: FindReplaceCtx::new(),
        }
    }

    pub fn active(&mut self, find_str: String) {
        self.param.find = find_str;
        self.is_show = true;
        self.need_focus = true;
        self.replace_ready = false;
    }

    pub fn _close(&mut self) {
        self.is_show = false;
        self.need_focus = false;
    }

    pub fn _is_window(&self) -> bool {
        self.is_window
    }

    pub fn set_find_result(&mut self, result: &FindCache, find_param: &FindReplaceCtx) {
        let last_line_no = if let Some(last) = result.cache.last() {
            last.start.line_no.to_string().len()
        } else {
            1
        };
        
        let text = result.cache.iter().map(|item|{
                let line_text = item.line_text.clone().unwrap_or(String::new());
                format!("{:>last_line_no$} {}", item.start.line_no+1, line_text)
            })
            .collect::<Vec<_>>()
            .join("\n");

        self.edit_ctx = Ctx::new(&text, false, None);
        self.edit_ctx.cfg_mut().need_line_click_cmd = true;
        self.edit_ctx.cfg_mut().hightlight_seleted_word = false;
        self.edit_ctx.flash_same_cache_with_param(find_param);
    }

    pub fn show_content(&mut self, ui: &mut Ui) -> Option<FindReplaceCtx> {
        let mut param = None;

        ui.add_space(4.0);
        ui.horizontal(|ui|{
            let case_button = Button::new("Aa").selected(self.param.is_case).rounding(3.0);
            if case_button.ui(ui).clicked() {
                self.param.is_case = !self.param.is_case;
            }
            let word_button = Button::new("__").selected(self.param.is_hole_word).rounding(3.0);
            if word_button.ui(ui).clicked() {
                self.param.is_hole_word = !self.param.is_hole_word;
            }
            let regex_button = Button::new("/.*/").selected(self.param.is_reg).rounding(3.0);
            if regex_button.ui(ui).clicked() {
                self.param.is_reg = !self.param.is_reg;
            }

            ui.separator();
            if ui.button("find").clicked() {
                self.replace_ready =  true;
                let mut ctx: FindReplaceCtx = self.param.clone();
                ctx.cmd = Some(FindCmd::Find);
                param = Some(ctx)
            }

            if ui.button("find all").clicked() {
                let mut ctx: FindReplaceCtx = self.param.clone();
                ctx.cmd = Some(FindCmd::FindAll);
                param = Some(ctx)
            }

            ui.separator();
            let open_text = if self.is_open_replace {"<"} else {">"};
            let open_replace_button = Button::new(open_text).selected(self.is_open_replace).rounding(3.0);
            if open_replace_button.ui(ui).clicked() {
                self.is_open_replace = !self.is_open_replace;
            }

            if self.is_open_replace {
                if ui.button("replace").clicked() {
                    let mut ctx: FindReplaceCtx = self.param.clone();
                    if self.replace_ready {
                        ctx.cmd = Some(FindCmd::Replace);
                    } else {
                        self.replace_ready = true;
                        ctx.cmd = Some(FindCmd::Find);
                    }
                    param = Some(ctx)
                }
                if ui.button("replace all").clicked() {
                    let mut ctx: FindReplaceCtx = self.param.clone();
                    ctx.cmd = Some(FindCmd::ReplaceAll);
                    param = Some(ctx)
                }
            }
        });
        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);
        
        let max_width = ui.available_width();
        ui.horizontal(|ui|{
            //ui.label("F");
            let mut edit = TextEdit::singleline(&mut self.param.find)
                .hint_text("find")
                .desired_width(max_width);
            if self.need_focus {
                edit = edit.cursor_at_end(true);
            }
            let r = edit.ui(ui);
            if self.need_focus {
                r.request_focus();
                self.need_focus = false;
            }
        });
        ui.add_space(4.0);
        if self.is_open_replace {
            ui.horizontal(|ui|{
                //ui.label("R");
                let edit = TextEdit::singleline(&mut self.param.replace)
                    .hint_text("replace")
                    .desired_width(max_width);
                edit.ui(ui);
            });
        }

        param
    }

    pub fn show_all(&mut self, ui: &mut Ui) -> Option<FindReplaceCtx> {
        let mut param = None;
        if self.is_window {
            param = self.show_content(ui);
        } else {
            ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                //ui.button("text");
                param = self.show_content(ui);
                //ui.separator();
                //ui.add(crate::medit::Edit::new(&mut self.edit_ctx));
            });
        }

        //Enter hot key
        if Self::enter_hot_keys(&ui) {
            self.replace_ready =  true;
            self.need_focus = true;
            let mut ctx: FindReplaceCtx = self.param.clone();
            ctx.cmd = Some(FindCmd::Find);
            param = Some(ctx)
        }
        param
    }

    pub fn show_window(&mut self, ui: &mut Ui) -> Option<FindReplaceCtx> {
        let mut param = None;
        if self.is_show == false {
            return None;
        }

        let size = Vec2::new(380.0, 200.0);
        let mut rect = Rect::from_min_size(ui.cursor().left_top(), size);
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            rect = rect.translate(Vec2::new(pointer_pos.x, pointer_pos.y));
        }
        let title = format!("find/replace");
        let egui_ctx = ui.ctx();
        let mut is_show = self.is_show;
        Window::new(title)
            .default_rect(rect)
            .open(&mut is_show)
            //.resizable([true, true])
            .enabled(true)
            .order(Order::TOP)
            .show(egui_ctx, |ui| {
                param = self.show_all(ui);
            }
        );
        self.is_show = is_show;
        param
    }

    pub fn show(&mut self, ui: &mut Ui) -> Option<FindReplaceCtx> {
        if self.is_window {
            self.show_window(ui)
        } else {
            self.show_all(ui)
        }
    }

    fn enter_hot_keys(ui: &Ui) -> bool {
        //hot keys
        let event_filter = EventFilter {
            tab: false,
            horizontal_arrows: false,
            vertical_arrows: false,
            escape: true,
        };
        let events = ui.input(|i| i.filtered_events(&event_filter));
        for event in &events {
            match event {
                Event::Key {
                    key,
                    pressed: true,
                    ..
                } => {
                    match key {
                        Key::Enter => {   
                            return true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        return false;
    }
}
