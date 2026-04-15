use eframe::egui::{Align, Button, Color32, Layout, Rect, Response, Sense, Ui, Widget};

use crate::store::Store;
use crate::uicom::{IconName, galley_builder};
use crate::i18n::tr;

/// Window bar
pub struct WinBar<'a> {
    store: &'a mut Store,
    title: String,
}

impl<'a> WinBar<'a> {
    pub fn new(store: &'a mut Store, title: String) -> Self {
        Self { store, title }
    }
}

impl WinBar<'_> {
    fn close_icon_button(ui: &mut Ui) -> Response {
        let text = galley_builder(ui)
            .icon(IconName::icon_clear)
            .bg(Color32::TRANSPARENT)
            .fg(ui.visuals().text_color())
            .build();
        let button = Button::new(text).fill(Color32::TRANSPARENT);
        button.ui(ui).on_hover_text(tr("winbar.close.tooltip"))
    }
}

impl Widget for WinBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let top = ui.cursor().left_top();
        let response = ui.allocate_rect(Rect::from_pos(top), Sense::hover());

        ui.horizontal(|ui|{
            ui.add_space(4.0);
            // Title left-aligned
            ui.label(self.title);
            // Use remaining space to push close button to the right
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if Self::close_icon_button(ui).clicked() {
                    self.store.tool_bar_info.is_show_bottom = false;
                }
            });
        });
        ui.add_space(4.0);

        response
    }
}

