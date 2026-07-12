use core::f32;
use std::sync::Arc;
use eframe::egui::{Button, Color32, FontId, Rect, Response, Sense, Ui, Widget, Galley};

use crate::store::Store;
use crate::uicom::{galley_builder, set_ui_button_font};

fn galley_for_path_bar_item(ui: &Ui, text: &str) -> Arc<Galley> {
    galley_builder(ui).text(text).wrap_width(320.0).build()
}

/// Path bar
pub struct PathBar<'a> {
    store: &'a mut Store,
    path: String,
}

impl<'a> PathBar<'a> {
    pub fn new(store: &'a mut Store, path: String) -> Self {
        Self { store, path }
    }
}

impl PathBar<'_> {
    fn sub_menus(store: &mut Store, ui: &mut Ui, name: &str, deep: usize) {
        if deep > 5 {
            return;
        }
        set_ui_button_font(ui);
        
        let childs = store.note_space.get_child_links(name);
        if childs.len() == 0 {
            if ui.button(galley_for_path_bar_item(ui, name)).clicked() {
                ui.close();
                let _ = store.open(name);
            }
        } else {
            let rsp = ui.menu_button(
                galley_for_path_bar_item(ui, name), 
                |ui|{
                    for c in childs {
                        Self::sub_menus(store, ui, &c, deep+1);
                    }
                    ui.separator();
                    if ui.button(galley_for_path_bar_item(ui,"+")).clicked() {
                        ui.close();
                        let _ = store.new_note(Some(name.to_string()));
                        return;
                    }
                });
            if rsp.response.clicked() {
                ui.close();
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
        if ui.button(galley_for_path_bar_item(ui,"+")).clicked() {
            ui.close();
            let parent = if name == "." { None } else { Some(name.to_string())};
            let _ = store.new_note(parent);
            return;
        }
    }

    fn pop_dir_menus(store: &mut Store, ui: &mut Ui, name: &str, dir: &str) {
        //current open is note
        if let Some(_) = store.note_space.get_current_note() {
            Self::pop_note_dir_menus(store, ui, name)
        } 
        //current open is file, todo
        else {
            Self::pop_file_dir_menus(store, ui, dir)
        }
    }

    fn pop_file_dir_menus(store: &mut Store, ui: &mut Ui, dir_path: &str) {
        use std::fs;

        println!("pop_file_dir_menus: {}", dir_path);
        
        // Read directory contents
        if let Ok(dir) = fs::read_dir(&dir_path) {
            let mut entries = Vec::new();
            
            for entry in dir {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if let Some(file_name) = path.file_name() {
                        let name_str = file_name.to_string_lossy().to_string();
                        let is_dir = path.is_dir();
                        entries.push((name_str, is_dir, path.to_string_lossy().to_string()));
                    }
                }
            }
            
            // Sort: directories first, files after, each group sorted by name
            entries.sort_by(|a, b| {
                match (a.1, b.1) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.0.cmp(&b.0)
                }
            });
            
            // Display directories and files
            for (name_str, is_dir, full_path) in entries {
                if is_dir {
                    // Directory: display as menu button
                    let display_name = format!("📁 {}", name_str);
                    let rsp = ui.menu_button(
                        galley_for_path_bar_item(ui, &display_name), 
                        |ui| {
                            Self::pop_file_dir_menus(store, ui, &full_path);
                        });
                    if rsp.response.clicked() {
                        ui.close();
                        let _ = store.open(&full_path);
                    }
                } else {
                    // File: display as clickable button
                    let display_name = format!("📄 {}", name_str);
                    if ui.button(galley_for_path_bar_item(ui, &display_name)).clicked() {
                        ui.close();
                        let _ = store.open(&full_path);
                    }
                }
            }
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

    fn path_bar_impl(path: String, store: &mut Store, ui: &mut Ui) -> f32 {
        let begin_x = ui.cursor().left_top().x;

        let mut font_id = FontId::default();
        font_id.family = eframe::egui::FontFamily::Proportional; 
        ui.style_mut().override_font_id = Some(font_id);

        let spacing = ui.spacing_mut();
        spacing.item_spacing.x = 2.0;
        spacing.button_padding.x = 0.0;

        // TRANSPARENT the botton bg_fill
        let weak_bg_fill = ui.visuals().widgets.inactive.weak_bg_fill;
        ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;

        let names:Vec<&str> = path.split(|c| c == '/' || c == '\\').collect();
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
                let mut builder = galley_builder(ui).text(&dispaly_name).wrap_width(f32::INFINITY);
                if let Some(fg) = fg {
                    builder = builder.fg(fg);
                }
                let button = Button::new(builder.build());
                if button.ui(ui).clicked() {
                    Self::path_bar_name_clicked(name, store);
                }
            }
            //dir
            if i < names.len()-1 {
                let dir = names[0..i+1].join("/") + "/";
                ui.menu_button(
                    galley_for_path_bar_item(ui, ">"), 
                    |ui| {
                        Self::pop_dir_menus(store, ui, *name, &dir)
                    });
            }
        }

        //restore weak_bg_fill
        ui.visuals_mut().widgets.inactive.weak_bg_fill = weak_bg_fill;

        //return the bar width
        let end_x = ui.cursor().left_top().x;
        return end_x - begin_x;
    }
}

impl Widget for PathBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let top = ui.cursor().left_top();
        let response = ui.allocate_rect(Rect::from_pos(top), Sense::hover());

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
            let width = Self::path_bar_impl(self.path, self.store, ui);
            self.store.tool_bar_info.width = Some(width);
        });
        ui.add_space(4.0);
        //ui.separator();

        response
    }
}

