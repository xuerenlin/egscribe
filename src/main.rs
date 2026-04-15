#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example

mod sitter;
mod ime_win_bridge;
mod util;
mod uicom;
mod medit;
mod toolbar;
mod space;
mod store;
mod find;
mod config;
mod i18n;
mod plugin;
mod sidepanel;

use std::vec;
use toolbar::{ToolBar, PathBar, WinBar, FileStatusBar, TabButtonBar};
use store::Store;
use eframe::egui::{self, Color32, Stroke, Vec2};
use eframe::egui::{Order, Rect, EventFilter, Ui, Event, Key};
use std::sync::mpsc;
use util::start_process;
use sidepanel::SidePanel;

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();
    let mut file = String::new();
    if args.len() > 1 {
        file = args[1].clone();
    }
    let rx = start_process(&file);
    if rx.is_none() {
        println!("other process has started, exit now !");
        std::process::exit(0);
    }

    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug` or $env:RUST_LOG="debug" in windows).
    let icon = eframe::icon_data::from_png_bytes(&include_bytes!("../desktop/egscribe.png")[..]).unwrap();
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
            Ok(Box::new(MyApp::new(cc, file, rx)))
        }),
    )
}

struct MyApp {
    store: Store,
    dropped_files: Vec<egui::DroppedFile>,
    title: String,
    ipc_rx: Option<mpsc::Receiver<String>>,
    side_panel: SidePanel,
    #[cfg(windows)]
    last_tsf_seq: u64,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>, file: String, ipc_rx: Option<mpsc::Receiver<String>>) -> Self {
        load_fonts(&cc.egui_ctx);
        let mut store = Store::default();
        if !file.is_empty() {
            let _ = store.open_file(&file);
        }
        #[cfg(windows)]
        {
            let ok = crate::ime_win_bridge::tsf_win::install_tsf_monitor();
            log::info!("install tsf monitor: ok={}", ok);
        }

        Self {
            store,
            dropped_files: vec![],
            title: String::new(),
            ipc_rx,
            side_panel: SidePanel::new(),
            #[cfg(windows)]
            last_tsf_seq: 0,
        }
    }

    #[cfg(windows)]
    fn trace_tsf_state(&mut self) {
        if let Some(snapshot) = crate::ime_win_bridge::tsf_win::poll_tsf_snapshot() {
            if snapshot.seq != self.last_tsf_seq {
                self.last_tsf_seq = snapshot.seq;
                log::debug!(
                    "tsf-msg composing={} start={} update={} end={} bound={} ui_open={} ui_begin={} ui_update={} ui_end={} comp_sink_supported={} seq={}",
                    snapshot.composing,
                    snapshot.start_count,
                    snapshot.update_count,
                    snapshot.end_count,
                    snapshot.context_bound,
                    snapshot.ui_open,
                    snapshot.ui_begin_count,
                    snapshot.ui_update_count,
                    snapshot.ui_end_count,
                    snapshot.composition_sink_supported,
                    snapshot.seq
                );
            }
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
                            if let Some((_uni_file,edit_ctx)) = self.store.cur_edit_ctx_mut() {
                                let selected = edit_ctx.get_selected_text();
                                let _ = self.store.find_window.active(selected);
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
            stroke: Stroke::new(0.0, Color32::TRANSPARENT),
            outer_margin: 0.0.into(),
            inner_margin: 0.0.into(),
            ..Default::default()
        };

        let mut a = true;
        egui::Window::new(i18n::tr("window.edit.title"))
            .fixed_rect(in_rect)
            .constrain_to(out_rect)
            .open(&mut a)
            .title_bar(false)
            .resizable([false, false])
            .order(Order::Middle)
            .frame(win_frame)
            .show(ctx, |ui| {
                TabButtonBar::from_note_and_file_tabs(ui, &mut self.store);
                if let Some(cur_path) = self.store.note_space.get_current_path() {
                    ui.horizontal(|ui|{
                        ui.add(PathBar::new(&mut self.store, cur_path));
                    });
                }

                if let Some((_uni_file,edit_ctx)) = self.store.cur_edit_ctx_mut() {
                    ui.add(medit::Edit::new(edit_ctx));
                }
            });
    }

    pub fn update_title(&mut self, ctx: &egui::Context) {
        if let Some(cur_name) = self.store.note_space.get_current_name() {
            let title_prefix = i18n::tr("window.main.title_prefix");
            let title = format!("{title_prefix}{cur_name}");
            if title != self.title {
                self.title = title.clone();
                ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
            }
        }
    }

    fn open_file_command_from_ipc_rx(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.ipc_rx {
            while let Ok(file) = rx.try_recv() {
                if !file.is_empty() {
                    let _ = self.store.open_file(&file);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
            }
        }
    }

    pub fn exe_edit_cmd(&mut self) {
        let mut cmd_list = vec![];
        if let Some((_uni_file, cur_ctx)) = self.store.cur_edit_ctx_mut() {
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
        let cur_ctx = &mut self.store.find_window.edit_ctx;
        while let Some(cmd) = cur_ctx.pop_cmd() {
            cmd_list.insert(0, cmd);
        }
        while let Some(cmd) = cmd_list.pop() {
            self.store.execute_cmd(cmd);
        }
    }
}

// What font is this
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(windows)]
        crate::ime_win_bridge::tsf_win::ensure_tsf_context_bound();
        //#[cfg(windows)]
        //self.trace_tsf_state();

        self.update_title(ctx);
        self.open_file_command_from_ipc_rx(ctx);

        
        egui::TopBottomPanel::top("top")
            .show_separator_line(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui|{
                    ui.add(ToolBar::new(&mut self.store));
                    //TabButtonBar::from_note_and_file_tabs(ui, &mut self.store);
                });
        });
        
        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui|{
            if let Some(_) = self.store.note_space.get_current_path() {
                ui.horizontal(|ui|{
                    ui.add(FileStatusBar::new(&mut self.store));
                });
            }
        });

        // 侧边栏（包含笔记管理和插件管理）
        self.side_panel.show(&mut self.store, ctx);

        if self.store.tool_bar_info.is_show_bottom {
            egui::TopBottomPanel::bottom("bottom_find_result")
                .resizable(true)
                .default_height(360.0)
                .show(ctx, |ui|{
                    let file_path = if let Some(unifile) = &self.store.find_window.find_file {
                        unifile.path()
                    } else {
                        String::new()
                    };
                    let count = self.store.find_window.edit_ctx.line_num();
                    let template = i18n::tr("window.find_result.title");
                    let title = template
                        .replace("{count}", &format!("{count}"))
                        .replace("{file}", &file_path);
                    ui.add(WinBar::new(&mut self.store, title));
                    ui.add(crate::medit::Edit::new(&mut self.store.find_window.edit_ctx));
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

            //find window 
            if let Some(find) = self.store.find_window.show(ui) {
                self.store.execute_find(find);
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
        show_non_text_file_prompt(ctx, &mut self.store);

        // Collect dropped files:
        ctx.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                self.dropped_files = i.raw.dropped_files.clone();
            }
        });

        // process edit command
        self.exe_edit_cmd();
        
        // Handle plugin messages and commands
        self.store.handle_plugin_messages();
        
        // Check and perform auto-save for notes
        self.store.check_auto_save();

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

        let screen_rect = ctx.content_rect();
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

fn show_non_text_file_prompt(ctx: &egui::Context, store: &mut Store) {
    use egui::*;

    let Some(prompt) = store.pending_non_text_file_prompt().cloned() else {
        return;
    };
    Window::new("无法直接打开文件")
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(true)
        .movable(true)
        .min_size([400.0, 100.0])
        .order(Order::Foreground)
        .show(ctx, |ui| {
            ui.label(format!("文件：{}", prompt.file_path));
            ui.add_space(8.0);
            ui.colored_label(Color32::from_rgb(255, 120, 120), &prompt.reason);
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                if ui.button("关闭").clicked() {
                    store.dismiss_non_text_file_prompt();
                }
                if ui.button("调用插件读取文件").clicked() {
                    store.request_read_non_text_with_plugin();
                }
            });
        });
}


fn load_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "msyhl".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/msyhl.ttc")).into(),
    );

    fonts.font_data.insert(
        "msyhb".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/msyhbd.ttc")).into(),
    );

    fonts.font_data.insert(
        "icon".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/icomoon/fonts/icomoon.ttf")).into(),
    );

    fonts.font_data.insert(
        "courier".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/cour.ttf")).into(),
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
