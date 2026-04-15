use eframe::egui::{Button, Color32, Rect, Response, Sense, Ui, Widget, Slider, Label};

use crate::store::Store;
use crate::uicom::{IconName, galley_builder, set_ui_button_font, icon_button_builder};
use crate::i18n::{self, Language, tr};
use crate::medit::pgh::TableFrameStyle;

/// Main toolbar
pub struct ToolBar<'a> {
    store: &'a mut Store,
}

impl<'a> ToolBar<'a> {
    pub fn new(store: &'a mut Store) -> Self {
        Self { store }
    }
}

impl ToolBar<'_> {
    fn font_size_menus(store: &mut Store, ui: &mut Ui) {
        ui.set_min_width(32.0);
        set_ui_button_font(ui);
        
        let cur_font_size = store.config.font_size;
        let str = format!("{}", cur_font_size as usize);
        let _ = ui.button(galley_builder(ui).text(&str).build());

        let size_list = vec![10,12,14,15,16,17,18,20,24,28,32,36,40,48,56];
        for size in size_list {
            let str = format!("{}", size);
            if ui.button(galley_builder(ui).text(&str).build()).clicked() {
                ui.close();
                store.config_set_font_size(size as f32);
            }
        }
    }

    fn tool_bar(store: &mut Store, ui: &mut Ui) {
        let spacing = ui.spacing_mut();
        let button_padding_x = spacing.button_padding.x;
        spacing.button_padding.x = spacing.button_padding.y;

        // language switcher
        ui.menu_button(tr("toolbar.language.button"), |ui| {
            if ui.button(tr("toolbar.language.zh-CN")).clicked() {
                i18n::set_language(Language::ZhCn);
                store.config.language = i18n::current_language_code();
                store.config_save();
                ui.close();
            }
            if ui.button(tr("toolbar.language.en-US")).clicked() {
                i18n::set_language(Language::EnUs);
                store.config.language = i18n::current_language_code();
                store.config_save();
                ui.close();
            }
        });

        //theme mode
        if store.config.dark_mode != ui.style().visuals.dark_mode {
            let theme = if store.config.dark_mode { eframe::egui::Theme::Dark } else { eframe::egui::Theme::Light };
            ui.ctx().set_theme(theme);
        }
        if store.config.dark_mode {
            if ui
                .add(Button::new("☀").frame(false))
                .on_hover_text(tr("toolbar.theme.to_light"))
                .clicked()
            {
                ui.ctx().set_theme(eframe::egui::Theme::Dark);
                store.config_update_dark_mode(false);
            }
        } else {
            if ui
                .add(Button::new("🌙").frame(false))
                .on_hover_text(tr("toolbar.theme.to_dark"))
                .clicked()
            {
                ui.ctx().set_theme(eframe::egui::Theme::Light);
                store.config_update_dark_mode(true);
            }
        }

        //text brightness slider
        let bg = Color32::TRANSPARENT;
        let fg = ui.visuals().text_color();
        ui.horizontal(|ui| {
            let icon_galley = galley_builder(ui)
                .icon(IconName::icon_sun)
                .text(&tr("toolbar.text_brightness.label"))
                .bg(bg)
                .fg(fg)
                .build();
            ui.add(Label::new(icon_galley).selectable(false));
            let brightness = store.config.text_color_brightness;
            let brightness_percent = (brightness * 100.0) as i32;
            let mut brightness_value = brightness_percent as f32;
            let slider = Slider::new(&mut brightness_value, 50.0..=200.0)
                //.text(format!("{}%", brightness_percent))
                .show_value(false);
            if ui.add(slider).changed() {
                store.config_set_text_color_brightness(brightness_value / 100.0);
            }
        });

        //font size
        ui.menu_button(galley_builder(ui)
        .icon(IconName::icon_format_font_size)
        .bg(bg)
        .fg(fg)
        .build(),
        |ui| {
            Self::font_size_menus(store, ui)
        })
        .response
        .on_hover_text(tr("toolbar.font_size.tooltip"));

        // table frame style
        ui.menu_button(tr("toolbar.table_frame.button"), |ui| {
            let mut choose = |label: &str, style: TableFrameStyle| {
                let selected = store.config.table_frame_style == style;
                if ui.selectable_label(selected, label).clicked() {
                    store.config_set_table_frame_style(style);
                    ui.close();
                }
            };
            choose(&tr("toolbar.table_frame.full"), TableFrameStyle::Full);
            choose(&tr("toolbar.table_frame.horizontal"), TableFrameStyle::Horizontal);
            choose(&tr("toolbar.table_frame.none"), TableFrameStyle::None);
            ui.separator();
            if ui
                .selectable_label(
                    store.config.show_table_row_no,
                    tr("toolbar.table_index.toggle"),
                )
                .clicked()
            {
                store.config_switch_table_row_no();
                ui.close();
            }
        });

        //line_no button
        if icon_button_builder(ui)
            .icon(IconName::icon_sort_numerically)
            .active(store.config.show_line_no)
            .hover_text(&tr("toolbar.line_number.tooltip"))
            .build_tool()
            .clicked() {
            store.config_switch_show_line_no();
        }
        
        // Markdown 标题多级序号（编辑区与侧栏目录）
        if icon_button_builder(ui)
            .icon(IconName::icon_hash)
            .active(store.config.show_heading_section_numbers)
            .hover_text(&tr("toolbar.heading_section_numbers.tooltip"))
            .build_tool()
            .clicked()
        {
            store.config_switch_heading_section_numbers();
        }
        //wrap button
        if icon_button_builder(ui)
            .icon(IconName::icon_wrap_text)
            .active(store.config.wrap)
            .hover_text(&tr("toolbar.wrap_text.tooltip"))
            .build_tool()
            .clicked() {
            store.config_switch_wrap_mode();
        }

        //indent decrease
        if icon_button_builder(ui)
            .icon(IconName::icon_indent_decrease)
            .hover_text(&tr("toolbar.indent_decrease.tooltip"))
            .build_tool()
            .clicked() {
            store.config_add_indent_size(-16.0);
        }
        //indent increase
        if icon_button_builder(ui)
            .icon(IconName::icon_indent_increase)
            .hover_text(&tr("toolbar.indent_increase.tooltip"))
            .build_tool()
            .clicked() {
            store.config_add_indent_size(16.0);
        }

        //new file
        if icon_button_builder(ui)
        .icon(IconName::icon_new)
        .hover_text(&tr("toolbar.new_file.tooltip"))
        .build_tool()
        .clicked() {
            store.create_untitled_file();
        }
        
        //restore padding_x
        ui.spacing_mut().button_padding.x = button_padding_x;
    }
}

impl Widget for ToolBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        set_ui_button_font(ui);

        let top = ui.cursor().left_top();
        let response = ui.allocate_rect(Rect::from_pos(top), Sense::hover());

        ui.horizontal(|ui|{            
            //tool bar
            Self::tool_bar(self.store, ui);
        });
        ui.add_space(4.0);

        response
    }
}

