use core::f32;
use std::sync::Arc;
use eframe::egui::{Button, Color32, FontId, Galley, Rect, Response, Sense, Ui, Visuals, Widget};

use crate::medit::{IconName, PghText};
use crate::mem::Store;
use crate::space::CurFile;

pub enum ToolBarType {
    PathBar(String),
    ToolBar,
    WinBar(String),
}

pub struct ToolBar<'a> {
    store: &'a mut Store,
    tool_bar_type: ToolBarType,
}

impl <'a>ToolBar<'a> {
    pub fn new(store: &'a mut Store, tool_bar_type: ToolBarType) -> Self {
        Self {
            store,
            tool_bar_type
        }
    }
}

impl ToolBar<'_> {
    pub fn button_galley(ui: &Ui, text: &str, fg: Option<Color32>) -> Arc<Galley> {
        let fg: Color32 = if let Some(fg) = fg {fg} else {
            ui.style().visuals.text_color()
        };

        PghText::text_galley(ui, String::from(text), fg, 320.0)
    }

    fn sub_menus(store: &mut Store, ui: &mut Ui, name: &str, deep: usize) {
        if deep > 5 {
            return;
        }
        Self::set_ui_button_font(ui);
        
        let childs = store.note_space.get_child_links(name);
        if childs.len() == 0 {
            if ui.button(Self::button_galley(ui, name, None)).clicked() {
                ui.close_menu();
                let _ = store.open(name);
            }
        } else {
            let rsp = ui.menu_button(
                Self::button_galley(ui, name, None), 
                |ui|{
                    for c in childs {
                        Self::sub_menus(store, ui, &c, deep+1);
                    }
                    ui.separator();
                    if ui.button(Self::button_galley(ui, "+", None)).clicked() {
                        ui.close_menu();
                        let _ = store.new_note(Some(name.to_string()));
                        return;
                    }
                });
            if rsp.response.clicked() {
                ui.close_menu();
                let _ = store.open(name);
            }
        }
    }

    fn pop_note_dir_menus(store: &mut Store, ui: &mut Ui, name: &str) {
        let childs = store.note_space.get_child_links(name);
        for c in childs {
            Self::sub_menus(store, ui, &c, 0);
        }
        ui.separator();
        if ui.button(Self::button_galley(ui, "+", None)).clicked() {
            ui.close_menu();
            let parent = if name == "." { None } else { Some(name.to_string())};
            let _ = store.new_note(parent);
            return;
        }
    }

    fn pop_dir_menus(store: &mut Store, ui: &mut Ui, name: &str) {
        //current open is note
        if let Some(_) = store.note_space.get_current_note() {
            Self::pop_note_dir_menus(store, ui, name)
        } 
        //current open is file, todo
        else {
        }
    }

    fn font_size_menus(store: &mut Store, ui: &mut Ui) {
        ui.set_min_width(32.0);
        Self::set_ui_button_font(ui);
        
        let cur_font_size = store.config.font_size;
        let str = format!("{}", cur_font_size as usize);
        let _ = ui.button(Self::button_galley(ui, &str, None));

        let size_list = vec![10,12,14,15,16,17,18,20,24,28,32,36,40,48,56];
        for size in size_list {
            let str = format!("{}", size);
            if ui.button(Self::button_galley(ui, &str, None)).clicked() {
                ui.close_menu();
                store.config_set_font_size(size as f32);
            }
        }
    }

    fn set_ui_button_font(ui: &mut Ui) {
        let mut font_id = FontId::default();
        //font_id.size = 16.0;
        font_id.family = eframe::egui::FontFamily::Monospace; 
        ui.style_mut().override_font_id = Some(font_id);
    }

    pub fn tool_icon_button(ui: &mut Ui, icon_name: IconName, active: bool, strong: bool, hover_text: &str) -> Response {
        let fg = if strong {
            ui.visuals().text_color()
        } else {
            ui.visuals().text_color()
        };
        let bg = if active {
            ui.visuals().selection.bg_fill
        } else {
            Color32::TRANSPARENT
        };
        let button = Button::new(PghText::icon_galley(ui, icon_name, bg, fg)).fill(Color32::TRANSPARENT);
        button.ui(ui).on_hover_text(hover_text)
    }

    fn tool_bar(store: &mut Store, ui: &mut Ui) {
        let spacing = ui.spacing_mut();
        let button_padding_x = spacing.button_padding.x;
        spacing.button_padding.x = spacing.button_padding.y;

        //line_no button
        if Self::tool_icon_button(ui, IconName::icon_sort_numerically, store.config.show_line_no, true, "Line number").clicked() {
            store.config_switch_show_line_no();
        }
        //wrap button
        if Self::tool_icon_button(ui, IconName::icon_wrap_text, store.config.wrap, true, "Wrap text").clicked() {
            store.config_switch_wrap_mode();
        }
        //font size menu
        let bg = Color32::TRANSPARENT;
        let fg = ui.visuals().text_color();
        ui.menu_button(PghText::icon_galley(ui, IconName::icon_format_font_size, bg, fg),
            |ui| {
                Self::font_size_menus(store, ui)
            })
            .response.on_hover_text("Font size");
    

        if !store.config.fixed_files.is_empty() {
            ui.separator();
        }

        //restore padding_x
        ui.spacing_mut().button_padding.x = button_padding_x;
        
        //fixed files button
        let mut need_open = None;
        for file in &store.config.fixed_files {
            let seleted = Some(file.to_string()) == store.note_space.get_current_note();
            let button = Button::new(file).selected(seleted).rounding(3.0);
            let r = button.ui(ui);
            if r.clicked() {
                need_open = Some(file.clone());
            }
        }
        if let Some(file) = need_open {
            let _ = store.open(&file);
        }

        ui.separator();

        //opend files
        need_open = None;
        let mut need_close = None;
        for (file, _) in &store.ectx_map {
            if file.is_file() {
                let seleted = Some(file.path()) == store.note_space.get_current_path();
                let button = Button::new(&file.name()).selected(seleted).rounding(3.0);
                let r = button.ui(ui);
                if r.clicked() {
                    need_open = Some(file.path());
                }
                if r.double_clicked() {
                    need_close = Some(file.path().clone());
                }
            }
        }
        if let Some(path) = need_open {
            let _ = store.open(&path);
        }
        if let Some(path) = need_close {
            let _ = store.close(&CurFile::from(&path));
        }
    }

    fn path_bar_name_clicked(name: &str, store: &mut Store) {
        if name == "." {}
        else if Some(name.to_string()) == store.note_space.get_current_note() { //current note
            //store.note_space.rename_window_active(name);
        } 
        else if let Some(_file) = store.note_space.get_current_file() { //current open is file
            //todo
        }
        else {
            let _ = store.open(name);
        }
    }

    fn home_icon_button(ui: &mut Ui) -> Response {
        let text = PghText::icon_galley(ui, IconName::icon_home, Color32::TRANSPARENT, ui.visuals().text_color());
        let button = Button::new(text).fill(Color32::TRANSPARENT);
        button.ui(ui).on_hover_text("Home page")
    }

    fn close_icon_button(ui: &mut Ui) -> Response {
        let text = PghText::icon_galley(ui, IconName::icon_close, Color32::TRANSPARENT, ui.visuals().text_color());
        let button = Button::new(text).fill(Color32::TRANSPARENT);
        button.ui(ui).on_hover_text("Close")
    }

    //return the bar width
    fn path_bar(path: String, store: &mut Store, ui: &mut Ui) -> f32 {
        let begin_x = ui.cursor().left_top().x;

        let mut font_id = FontId::default();
        font_id.family = eframe::egui::FontFamily::Proportional; 
        ui.style_mut().override_font_id = Some(font_id);

        //home button
        if Self::home_icon_button(ui).clicked() {
            store.config_update_show_index_window(!store.note_space.is_show_index_window());
        }

        let spacing = ui.spacing_mut();
        spacing.item_spacing.x = 2.0;
        spacing.button_padding.x = 0.0;

        // TRANSPARENT the botton bg_fill
        let weak_bg_fill = ui.visuals().widgets.inactive.weak_bg_fill;
        ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;

        let names:Vec<&str> = path.split('/').collect();
        for (i, name) in names.iter().enumerate() {
            //root
            if name == &"." {} 
            //file
            else if name.len() > 0 {
                let mut dispaly_name = name.to_string();
                let mut fg = None;
                if let Some(cur) = store.note_space.get_current_name() {
                    if name == &cur && store.is_cur_content_changed() {
                        dispaly_name = "*".to_string() + &dispaly_name + "*";
                        fg = Some(ui.visuals().strong_text_color());
                    }
                }
                let button = Button::new(Self::button_galley(ui, &dispaly_name, fg));
                if button.ui(ui).clicked() {
                    Self::path_bar_name_clicked(name, store);
                }
            }
            //dir
            if i < names.len()-1 {
                ui.menu_button(
                    Self::button_galley(ui, ">", None), 
                    |ui| {
                        Self::pop_dir_menus(store, ui, *name)
                    });
            }
        }

        //restore weak_bg_fill
        ui.visuals_mut().widgets.inactive.weak_bg_fill = weak_bg_fill;

        //return the bar width
        let end_x = ui.cursor().left_top().x;
        return end_x - begin_x;
    }

    fn window_bar(store: &mut Store, ui: &mut Ui, title: String) {
        if Self::close_icon_button(ui).clicked() {
            store.tool_bar_info.is_show_bottom = false;
        }
        ui.label(title);
    }

}

impl Widget for ToolBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        Self::set_ui_button_font(ui);

        let top = ui.cursor().left_top();
        let response = ui.allocate_rect(Rect::from_pos(top), Sense::hover());

        match self.tool_bar_type {
            ToolBarType::ToolBar => {
                ui.horizontal(|ui|{
                    //view mode button
                    let cfg_visuals = if self.store.config.dark_mode {Visuals::dark()} else {Visuals::light()};
                    if cfg_visuals.dark_mode != ui.style().visuals.dark_mode {
                        ui.ctx().set_visuals(cfg_visuals.clone());
                    }
                    if let Some(new_visuals) = cfg_visuals.light_dark_small_toggle_button(ui) {
                        self.store.config_update_dark_mode(new_visuals.dark_mode);
                        ui.ctx().set_visuals(new_visuals);
                    }
        
                    //tool bar
                    Self::tool_bar(self.store, ui);
                });
                ui.add_space(4.0);
            },
            ToolBarType::PathBar(path) => {
                ui.horizontal(|ui|{
                    //add space, let path-bar in middle
                    /* 
                    let current_x = ui.cursor().left_top().x;
                    let path_bar_with = self.store.path_bar_info.width.unwrap_or_else(||0.0);
                    let max_with = ui.available_width();
                    let fill_space = ((max_with - current_x - path_bar_with)/2.0).at_least(0.0);
                    ui.add_space(fill_space);
                    */
                    ui.add_space(4.0);
                    //path bar
                    let width = Self::path_bar(path, self.store, ui);
                    self.store.tool_bar_info.width = Some(width);
                });
                ui.add_space(4.0);
                //ui.separator();
            },
            ToolBarType::WinBar(title) => {
                ui.horizontal(|ui|{
                    ui.add_space(4.0);
                    Self::window_bar(self.store, ui, title);
                });
                ui.add_space(4.0);
            }
        }

        response
    }


}
