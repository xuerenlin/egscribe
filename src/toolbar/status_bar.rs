use eframe::egui::{Color32, Rect, Response, RichText, Sense, Ui, Widget};

use crate::store::Store;
use crate::util::encoding::EncodingManager;
use egscribe_sitter as sitter;
use crate::i18n::tr;

/// File status bar
pub struct FileStatusBar<'a> {
    store: &'a mut Store,
}

impl<'a> FileStatusBar<'a> {
    pub fn new(store: &'a mut Store) -> Self {
        Self { store }
    }
}

impl FileStatusBar<'_> {
    fn file_status_bar_impl(store: &mut Store, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // TRANSPARENT the botton bg_fill
            let weak_bg_fill = ui.visuals().widgets.inactive.weak_bg_fill;
            ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;

            // Left: text status information
            ui.horizontal(|ui| {
                // File status information
                if let Some((uni_file, _)) = store.cur_edit_ctx_mut() {
                    if uni_file.is_file() {
                        ui.label(tr("filestatusbar.file"));
                    } else {
                        ui.label(tr("filestatusbar.note"));
                    }
                }
            });

            // Right: cursor position and selection information (right-aligned)
            ui.with_layout(eframe::egui::Layout::right_to_left(eframe::egui::Align::Center), |ui| {
                // LANG menu button (files only; notes are always Markdown)
                if let Some((uni_file, edit_ctx)) = store.cur_edit_ctx_mut() {
                    if uni_file.is_file() {
                        let lang_text = edit_ctx.get_heighlight_lang();
                        ui.menu_button(&lang_text.to_string(), |ui| {
                            if ui.button("Plain Text").clicked() {
                                if let Some((_uni_file, edit_ctx)) = store.cur_edit_ctx_mut() {
                                    edit_ctx.set_height_lang(None);
                                    edit_ctx.highlight_refresh(ui);
                                }
                                ui.close();
                            }

                            ui.separator();

                            for lang in sitter::support_lang() {
                                if ui.button(lang).clicked() {
                                    if let Some((_uni_file, edit_ctx)) = store.cur_edit_ctx_mut() {
                                        edit_ctx.set_height_lang(Some(lang.to_string()));
                                        edit_ctx.highlight_refresh(ui);
                                    }
                                    ui.close();
                                }
                            }
                        });

                        ui.separator();
                    }
                }
                
                // Selected status info
                if let Some((_uni_file, edit_ctx)) = store.cur_edit_ctx_mut() {
                    ui.label(&edit_ctx.get_selected_status_info());
                }

                if let Some((_uni_file, edit_ctx)) = store.cur_edit_ctx_mut() {
                    if edit_ctx.find_filter_is_active() {
                        ui.separator();
                        let total = edit_ctx.line_num();
                        let progress_pct = if edit_ctx.is_find_filter_searching() {
                            (edit_ctx.find_filter_search_progress() * 100.0).round() as u32
                        } else {
                            100
                        };
                        let visible = edit_ctx.find_filter_visible_line_count();
                        ui.horizontal(|ui| {
                            ui.label(format!(" total: {total}"));
                            ui.label(RichText::new(visible.to_string()).strong());
                            ui.label(format!("progress:{progress_pct}% visible: "));
                        });
                    }
                }

                // Line ending & charset menus (files only)
                if let Some((uni_file, _)) = store.cur_edit_ctx_mut() {
                    if uni_file.is_file() {

                        ui.separator();

                        let line_ending_text = store.get_current_file_line_ending_name();
                        ui.menu_button(&line_ending_text.to_string(), |ui| {
                            let line_endings = EncodingManager::get_supported_line_endings();
                            for line_ending in line_endings {
                                if ui.button(line_ending.display_name()).clicked() {
                                    ui.close();
                                    let _ = store.save_with_line_ending(&line_ending);
                                    break;
                                }
                            }
                        });

                        ui.separator();

                        let current_charset = if let Some(encoding_info) = store.get_current_file_encoding() {
                            encoding_info.charset.display_name()
                        } else {
                            "UTF-8"
                        };

                        let charset_text = format!("{}", current_charset);
                        ui.menu_button(&charset_text, |ui| {
                            ui.menu_button(tr("filestatusbar.encoding.reopen"), |ui| {
                                let charsets = EncodingManager::get_supported_charsets();
                                for charset in charsets {
                                    if ui.button(charset.display_name()).clicked() {
                                        ui.close();
                                        let _ = store.reopen_with_encoding(&charset);
                                        break;
                                    }
                                }
                            });
                            ui.menu_button(tr("filestatusbar.encoding.save"), |ui| {
                                let charsets = EncodingManager::get_supported_charsets();
                                for charset in charsets {
                                    if ui.button(charset.display_name()).clicked() {
                                        ui.close();
                                        let _ = store.save_with_encoding(&charset);
                                        break;
                                    }
                                }
                            });
                        });
                    }
                }

                ui.separator();
                if let Some(latest) = store.latest_notification() {
                    let title = format!(
                        "{} {}",
                        tr("notifications.status.latest"),
                        latest.status_message()
                    );
                    if ui.button(title).clicked() {
                        store.request_open_notifications_view();
                    }
                } else {
                    ui.label(tr("notifications.status.empty"));
                }
            });

            //restore weak_bg_fill
            ui.visuals_mut().widgets.inactive.weak_bg_fill = weak_bg_fill;
        });
    }
}

impl Widget for FileStatusBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let top = ui.cursor().left_top();
        let response = ui.allocate_rect(Rect::from_pos(top), Sense::hover());

        Self::file_status_bar_impl(self.store, ui);

        response
    }
}

