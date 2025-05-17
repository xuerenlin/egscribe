#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

mod sitter;
mod medit;
mod toolbar;
mod space;
mod mem;
mod find;

use std::vec;
use toolbar::{ToolBar, ToolBarType};
use mem::Store;
use find::FindWindow;
use eframe::egui::{self, Color32, Stroke, Vec2};
use eframe::egui::{Order, Rect, EventFilter, Ui, Event, Key, ScrollArea};

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();
    let mut file = String::new();
    if args.len() > 1 {
        file = args[1].clone();
    }

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let icon = eframe::icon_data::from_png_bytes(&include_bytes!("../fonts/egscribe.png")[..]).unwrap();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(icon)
            .with_inner_size([1240.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egscribe",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp::new(cc, file)))
        }),
    )
}

struct MyApp {
    store: Store,
    find_window: FindWindow,
    dropped_files: Vec<egui::DroppedFile>
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>, file: String) -> Self {
        load_fonts(&cc.egui_ctx);
        Self::default(file)
    }

    fn default(file: String) -> Self {
        let mut store = Store::default();
        if !file.is_empty() {
            let _ = store.open_file(&file);
        }
        Self {
            store,
            find_window: FindWindow::new(),
            dropped_files: vec![],
        }
    }

        
    fn hot_keys(&mut self, ui: &Ui) {
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
                    modifiers,
                    key,
                    pressed: true,
                    ..
                } => {
                    match key {
                        Key::S if modifiers.ctrl => {   
                            //ctrl+s save
                            let _ = self.store.save();
                        }
                        Key::F if modifiers.ctrl => {   
                            //ctrl+f find
                            if let Some(edit_ctx) = self.store.cur_edit_ctx_mut() {
                                let selected = edit_ctx.get_selected_text();
                                let _ = self.find_window.active(selected);
                            }
                        }
                        Key::Escape => {
                            self.store.config_update_show_index_window(!self.store.note_space.is_show_index_window());
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

        
    fn edit_sub_window(&mut self, ctx: &egui::Context, in_rect: Rect, out_rect: Rect) {
        let win_frame = egui::Frame {
            fill: ctx.style().visuals.window_fill(),
            rounding: 0.0.into(),
            stroke: Stroke::new(0.0, Color32::TRANSPARENT),
            outer_margin: 0.0.into(),
            inner_margin: 0.0.into(),
            ..Default::default()
        };

        let mut a = true;
        egui::Window::new("title")
            .fixed_rect(in_rect)
            .constrain_to(out_rect)
            .open(&mut a)
            .title_bar(false)
            .resizable([false, false])
            .order(Order::Middle)
            .frame(win_frame)
            .show(ctx, |ui| {
                if let Some(cur_path) = self.store.note_space.get_current_path() {
                    let path_bar = ToolBarType::PathBar(cur_path);
                    ui.add(ToolBar::new(&mut self.store, path_bar));
                }

                if let Some(edit_ctx) = self.store.cur_edit_ctx_mut() {
                    ui.add(medit::Edit::new(edit_ctx));
                }
            });
    }

    pub fn exe_edit_cmd(&mut self) {
        let mut cmd_list = vec![];
        if let Some(cur_ctx) = self.store.cur_edit_ctx_mut() {
            while let Some(cmd) = cur_ctx.pop_cmd() {
                cmd_list.insert(0, cmd);
            }
        }
        while let Some(cmd) = cmd_list.pop() {
            self.store.execute_cmd(cmd);
        }
    }

    pub fn exe_find_edit_cmd(&mut self) {
        let mut cmd_list = vec![];
        let cur_ctx = &mut self.find_window.edit_ctx;
        while let Some(cmd) = cur_ctx.pop_cmd() {
            cmd_list.insert(0, cmd);
        }
        while let Some(cmd) = cmd_list.pop() {
            self.store.execute_cmd(cmd);
        }
    }
}

//这是什么字体
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        egui::TopBottomPanel::top("top")
            .show_separator_line(true)
            .show(ctx, |ui| {
            ui.add(ToolBar::new(&mut self.store, ToolBarType::ToolBar));
        });

        //egui::TopBottomPanel::bottom("bottom").show(ctx, |ui|{
        //    ui.horizontal(|ui| {
        //        ui.label("status bar");
        //    });
        //});

        //index window
        if self.store.note_space.is_show_index_window() {
            egui::SidePanel::left("options")
                .resizable(true)
                .default_width(260.0)
                .show_separator_line(true)
                .show(ctx, |ui| {

                ScrollArea::both().auto_shrink(false).show(ui, |ui| {
                    let mut outer_rect = ui.cursor();

                    outer_rect.set_width(ui.available_width());
                    outer_rect.set_height(ui.available_height());
                    let in_rect = outer_rect.expand(-10.0);
                    let mut config = self.store.config.clone();
                    if let Some(cmd) = self.store.note_space.show_index_view(&mut config, ui, in_rect, outer_rect) {
                        self.store.execute_cmd(cmd);
                    }
                    //save tree state
                    if config.tree_open_state_changed {
                        self.store.config = config;
                        self.store.config_save();
                    }
                });
            });
        }

        if self.store.tool_bar_info.is_show_bottom {
            egui::TopBottomPanel::bottom("bottom_find_result")
                .resizable(true)
                .default_height(360.0)
                .show(ctx, |ui|{
                let title = format!("Find result: match {} items", self.find_window.edit_ctx.line_num());
                ui.add(ToolBar::new(&mut self.store, ToolBarType::WinBar(title)));
                ui.add(crate::medit::Edit::new(&mut self.find_window.edit_ctx));
                self.exe_find_edit_cmd();
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            //test_clipboard(ui);
            //ui.image("file://E:/rustspace/medit/fonts/M.png");
            
            let mut outer_rect = ui.cursor();
            outer_rect.set_width(ui.available_width());
            outer_rect.set_height(ui.available_height());
            
            //edit window
            //ui.add(medit::Edit::new(&mut self.store.edit_ctx));
            let edit_rect = outer_rect.expand2(Vec2::new(0.0, 0.0));
            self.edit_sub_window( ui.ctx(), edit_rect, outer_rect);

            //rename window
            if self.store.note_space.rename_window_show(ui) {
                let (org_name, new_name) = self.store.note_space.rename_from_to();
                if org_name != new_name {
                    let _ = self.store.rename_file(&org_name, &new_name);
                }
            }

            //find window as top window 
            if let Some(find) = self.find_window.show(ui) {
                self.store.execute_cmd(medit::Command::FindReplace(find));
                if let Some(edit_ctx) = self.store.cur_edit_ctx_mut() {
                    let (find_cache, find_param) = edit_ctx.get_find_cache();
                    self.find_window.set_find_result(find_cache, find_param);
                }
            }

            //hot keys
            self.hot_keys(ui);
        });

        

        // open dropped files:
        while let Some(dropped_file) = self.dropped_files.pop(){
            println!("{:?}", dropped_file.path);
            if let Some(file) = dropped_file.path {
                let _ = self.store.open(&file.to_string_lossy());
            }
        }
        
        // preview files dropped
        preview_files_being_dropped(ctx);

        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.dropped_files = i.raw.dropped_files.clone();
            }
        });

        // process edit command
        self.exe_edit_cmd();

    }
}

/// Preview hovering files:
fn preview_files_being_dropped(ctx: &egui::Context) {
    use egui::*;
    use std::fmt::Write as _;

    if !ctx.input(|i| i.raw.hovered_files.is_empty()) {
        let text = ctx.input(|i| {
            let mut text = "".to_owned();
            for file in &i.raw.hovered_files {
                if let Some(path) = &file.path {
                    write!(text, "\n{}", path.display()).ok();
                } else if !file.mime.is_empty() {
                    write!(text, "\n{}", file.mime).ok();
                } else {
                    text += "\n???";
                }
            }
            text
        });

        let painter =
            ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("file_drop_target")));

        let screen_rect = ctx.screen_rect();
        painter.rect_filled(screen_rect, 0.0, Color32::from_black_alpha(192));
        painter.text(
            screen_rect.center(),
            Align2::CENTER_CENTER,
            text,
            TextStyle::Heading.resolve(&ctx.style()),
            Color32::WHITE,
        );
    }
}


fn load_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "msyhl".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/msyhl.ttc")),
    );

    fonts.font_data.insert(
        "msyhb".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/msyhbd.ttc")),
    );

    fonts.font_data.insert(
        "icon".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/icomoon/fonts/icomoon.ttf")),
    );

    fonts.font_data.insert(
        "courier".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/cour.ttf")),
    );


    //Monospace
    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .insert(0, "courier".to_owned());

    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .push("msyhl".to_owned());

    //Proportional
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "msyhl".to_owned());

    //Strong
    fonts.families.insert(
        egui::FontFamily::Name("msyhb".into()),
        vec!["msyhb".to_owned()],
    );

    //Icon
    fonts.families.insert(
        egui::FontFamily::Name("icon".into()),
        vec!["icon".to_owned(), "msyhl".to_owned()],
    );

    ctx.set_fonts(fonts);
}
