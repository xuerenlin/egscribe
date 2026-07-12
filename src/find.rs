
use eframe::egui::{Button, Event, EventFilter, Key, Order, Rect, ScrollArea, TextEdit, Ui, Vec2, Widget, Window};
use crate::medit::{ctx::FindCache, Ctx, FindCmd, FindReplaceCtx, cfg::HeightMode};
use crate::space::UniFile;
use crate::i18n::tr;

pub struct FindWindow {
    is_show: bool,
    is_window: bool,
    need_focus: bool,
    replace_ready: bool,
    is_open_replace: bool,
    is_live_display: bool,
    need_clear_filter: bool,
    last_live_trigger: FindReplaceCtx,
    param: FindReplaceCtx,
    pub find_file: Option<UniFile>,
    pub edit_ctx: Ctx,
}

impl FindWindow {
    pub fn new() -> Self {
        let edit_ctx = Ctx::new()
            .with_text("", false)
            .height_mode(HeightMode::fix_max())
            .need_line_click_cmd(true);
        Self {
            is_show: false,
            is_window: true,
            need_focus: false,
            replace_ready: false,
            is_open_replace: false,
            is_live_display: false,
            need_clear_filter: false,
            last_live_trigger: FindReplaceCtx::new(),
            param: FindReplaceCtx::new(),
            find_file: None,
            edit_ctx,
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

    pub fn set_find_result(&mut self, find_file: UniFile, result: &FindCache, find_param: &FindReplaceCtx, dark_mode: bool) {
        let cursor_text_size = if let Some(last) = result.cache.last() {
            last.end.line_no.to_string().len() + 8
        } else {
            1
        };
        
        let text = result.cache.iter().map(|item|{
                let line_text = item.line_text.clone().unwrap_or(String::new());
                let cursor_text = format!("{}.{}.{}.", item.end.line_no, item.end.segment, item.end.culumn);
                format!("{:>cursor_text_size$} {}", cursor_text, line_text)
            })
            .collect::<Vec<_>>()
            .join("\n");

        println!("set_find_result: text length = {}, cache count = {}", text.len(), result.cache.len());
        
        self.edit_ctx = Ctx::new()
            .with_text(&text, false)
            .height_mode(HeightMode::fix_max())
            .need_line_click_cmd(true)
            .hightlight_seleted_word(false);
        self.edit_ctx.cfg_mut().dark_mode = dark_mode;
        
        // Directly set find cache and parameters instead of re-searching
        // This allows FindNotes results to be cached correctly
        self.edit_ctx.set_find_cache(result.clone(), find_param.clone());
        
        // Also set same_cache for highlighting
        self.edit_ctx.flash_same_cache_with_param(find_param);
        self.find_file = Some(find_file);
    }

    pub fn drain_clear_filter(&mut self) -> bool {
        std::mem::take(&mut self.need_clear_filter)
    }

    fn live_filter_changed(&self) -> bool {
        self.param.find != self.last_live_trigger.find
            || self.param.is_case != self.last_live_trigger.is_case
            || self.param.is_hole_word != self.last_live_trigger.is_hole_word
            || self.param.is_reg != self.last_live_trigger.is_reg
    }

    fn live_display_param(&self) -> FindReplaceCtx {
        let mut ctx = self.param.clone();
        ctx.cmd = Some(FindCmd::LiveDisplay);
        ctx
    }

    pub fn show_content(&mut self, ui: &mut Ui) -> Option<FindReplaceCtx> {
        let mut param = None;

        ui.add_space(4.0);
        ui.horizontal(|ui|{
            let case_button = Button::new("Aa").selected(self.param.is_case).corner_radius(3.0);
            if case_button.ui(ui).clicked() {
                self.param.is_case = !self.param.is_case;
            }
            let word_button = Button::new("__").selected(self.param.is_hole_word).corner_radius(3.0);
            if word_button.ui(ui).clicked() {
                self.param.is_hole_word = !self.param.is_hole_word;
            }
            let regex_button = Button::new("/.*/").selected(self.param.is_reg).corner_radius(3.0);
            if regex_button.ui(ui).clicked() {
                self.param.is_reg = !self.param.is_reg;
            }

            let live_button = Button::new(tr("find.live_display"))
                .selected(self.is_live_display)
                .corner_radius(3.0);
            if live_button.ui(ui).clicked() {
                self.is_live_display = !self.is_live_display;
                if self.is_live_display {
                    param = Some(self.live_display_param());
                    self.last_live_trigger = self.param.clone();
                } else {
                    self.need_clear_filter = true;
                }
            }

            ui.separator();
            if ui.button(tr("find.button")).clicked() {
                self.replace_ready =  true;
                let mut ctx: FindReplaceCtx = self.param.clone();
                ctx.cmd = Some(FindCmd::Find);
                param = Some(ctx)
            }

            if ui.button(tr("find.all")).clicked() {
                let mut ctx: FindReplaceCtx = self.param.clone();
                ctx.cmd = Some(FindCmd::FindAll);
                param = Some(ctx)
            }

            if ui.button(tr("find.notes")).clicked() {
                let mut ctx: FindReplaceCtx = self.param.clone();
                ctx.cmd = Some(FindCmd::FindNotes);
                param = Some(ctx)
            }

            ui.separator();
            let open_text = if self.is_open_replace {"<"} else {">"};
            let open_replace_button = Button::new(open_text).selected(self.is_open_replace).corner_radius(3.0);
            if open_replace_button.ui(ui).clicked() {
                self.is_open_replace = !self.is_open_replace;
            }

            if self.is_open_replace {
                if ui.button(tr("find.replace")).clicked() {
                    let mut ctx: FindReplaceCtx = self.param.clone();
                    if self.replace_ready {
                        ctx.cmd = Some(FindCmd::Replace);
                    } else {
                        self.replace_ready = true;
                        ctx.cmd = Some(FindCmd::Find);
                    }
                    param = Some(ctx)
                }
                if ui.button(tr("find.replace_all")).clicked() {
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
                .hint_text(tr("find.hint"))
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
                    .hint_text(tr("find.replace.hint"))
                    .desired_width(max_width);
                edit.ui(ui);
            });
        }

        if param.is_none() && self.is_live_display && self.live_filter_changed() {
            param = Some(self.live_display_param());
            self.last_live_trigger = self.param.clone();
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

        let was_show = self.is_show;
        let size = Vec2::new(380.0, 200.0);
        let mut rect = Rect::from_min_size(ui.cursor().left_top(), size);
        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            rect = rect.translate(Vec2::new(pointer_pos.x, pointer_pos.y));
        }
        let title = tr("find.title");
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
        if was_show && !is_show {
            self.is_live_display = false;
            self.need_clear_filter = true;
        }
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
