use eframe::egui::{
    Align, Button, Color32, Frame, Id, Layout, Pos2, Rect, Response, ScrollArea, Sense, Stroke, StrokeKind, Ui, Visuals,
    Widget,
};
use eframe::egui::containers::scroll_area::ScrollBarVisibility;
use eframe::egui::epaint;
use core::f32;
use std::collections::HashMap;

use crate::uicom::{IconName, icon_button_builder, galley_builder, icon_name_from_filepath};
use crate::store::Store;
use crate::space::UniFile;
use crate::i18n::tr;

const TAB_FONT_SIZE: f32 = 16.0;
const CLOSE_BUTTON_SIZE: f32 = 14.0;

/// Tab button item
pub struct TabItem {
    /// Display text
    pub text: String,
    /// Whether selected
    pub selected: bool,
    /// Whether to show close button
    pub show_close: bool,
    /// Associated data (for click callback)
    pub data: String,
    /// Icon (to distinguish notes and file types)
    pub icon: Option<IconName>,
    /// Whether this is a note (to show fixed button instead of close button)
    pub is_note: bool,
    /// Whether this note is fixed
    pub is_fixed: bool,
}

/// Tab button response result
pub struct TabButtonResponse {
    /// Main button response
    pub response: Response,
    /// Whether close button was clicked
    pub close_clicked: bool,
}

/// Tab button group widget
pub struct TabButtonBar<'a> {
    store: &'a mut Store,
    items: Vec<TabItem>,
    extend_space: bool,
    on_click: Box<dyn Fn(&mut Store, &str) + 'a>,
    on_close: Box<dyn Fn(&mut Store, &str) + 'a>,
    id: String,
}

impl<'a> TabButtonBar<'a> {
    pub fn new(
        store: &'a mut Store,
        items: Vec<TabItem>,
        extend_space: bool,
        on_click: impl Fn(&mut Store, &str) + 'a,
        on_close: impl Fn(&mut Store, &str) + 'a,
        id: String,
    ) -> Self {
        Self {
            store,
            items,
            extend_space,
            on_click: Box::new(on_click),
            on_close: Box::new(on_close),
            id,
        }
    }

    /// Build note tab bar
    pub fn from_note_tabs(store: &'a mut Store, extend_space: bool) -> Option<Self> {
        // Get note list from opened_files.notes()
        let note_items: Vec<TabItem> = store
            .opened_files
            .notes()
            .iter()
            .filter_map(|note| {
                // Ensure note exists in ectx_map
                if !store.ectx_map.contains_key(note) {
                    return None;
                }
                
                let note_name = note.name();
                let note_path = note.path();
                let is_fixed = store.is_fixed(&note_name);
                Some(TabItem {
                    text: note_name.clone(),
                    selected: Some(note_path.clone()) == store.note_space.get_current_path(),
                    show_close: true,
                    data: note_name,
                    icon: Some(IconName::icon_bookmark1), // Notes use bookmark icon
                    is_note: true,
                    is_fixed,
                })
            })
            .collect();
        
        if note_items.is_empty() {
            return None;
        }
        Some(Self::new(
            store,
            note_items,
            extend_space,
            |store, name| {
                let _ = store.open(name);
            },
            |store, name| {
                if store.is_fixed(name) {
                    store.unfix_file(name);
                } else {
                    if let Some(note) = store.opened_files.all().iter().find(|f| f.is_note() && f.name() == name).cloned() {
                        store.close(&note);
                    }
                }
            },
            "note_tabbar".to_string()
        ))
    }

    /// Build file tab bar
    pub fn from_file_tabs(store: &'a mut Store, extend_space: bool) -> Option<Self> {
        // Display files according to opened files order list (only files, not notes)
        let opened_items: Vec<TabItem> = store
            .opened_files
            .files()
            .iter()
            .filter_map(|file| {
                // Ensure file exists in ectx_map
                if !store.ectx_map.contains_key(file) {
                    return None;
                }
                
                let file_path = file.path();
                // Check if it's an unsaved file
                let (text, icon) = if file_path.starts_with("untitled/") {
                    // Unsaved file: display file name (remove "untitled/" prefix)
                    let display_name = file_path.strip_prefix("untitled/").unwrap_or(&file_path).to_string();
                    (display_name, Some(IconName::icon_file_text))
                } else {
                    // Saved file: display file name and corresponding icon
                    (file.name(), Some(crate::uicom::icon_name_from_filepath(&file_path)))
                };
                
                let file_path_clone = file_path.clone();
                Some(TabItem {
                    text,
                    selected: Some(file_path_clone.clone()) == store.note_space.get_current_path(),
                    show_close: true,
                    data: file_path_clone,
                    icon,
                    is_note: false,
                    is_fixed: false,
                })
            })
            .collect();

        Some(Self::new(
            store,
            opened_items,
            extend_space,
            |store, path| {
                let _ = store.open(path);
            },
            |store, path| {
                let uni_file = UniFile::from(path);
                let _ = store.close(&uni_file);
            },
            "file_tabbar".to_string()
        ))
    }

    pub fn from_note_and_file_tabs(ui: &mut Ui, store: &'a mut Store) {
        ui.horizontal(|ui| {
            let total_width = ui.available_width();
            let menu_width = 36.0;
            let scroll_width = (total_width - menu_width).max(0.0);

            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.set_width(scroll_width);
                Self::tab_scroll_area(ui, store);
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.set_width(menu_width);
                ui.add_space(4.0);
                Self::tab_more_menu(ui, store);
            });
        });
    }

    /// Set target file/note for scrolling when tab button or menu item is clicked
    fn set_scroll_target(ui: &mut Ui, target: String) {
        let scroll_target_id = Id::new("tabbar_scroll_target");
        ui.memory_mut(|mem| {
            mem.data.insert_temp(scroll_target_id, target);
        });
    }

    /// Get scroll target and clear it
    fn get_and_clear_scroll_target(ui: &mut Ui) -> Option<String> {
        let scroll_target_id = Id::new("tabbar_scroll_target");
        let target: Option<String> = ui.memory(|mem| {
            mem.data.get_temp::<String>(scroll_target_id)
        });
        
        if target.is_some() {
            ui.memory_mut(|mem| {
                mem.data.remove::<String>(scroll_target_id);
            });
        }
        
        target
    }

    /// Store tab rect in memory map (keyed by item.data)
    fn store_tab_rect(ui: &mut Ui, tab_id: &str, item_data: &str, rect: Rect) {
        let tab_rects_map_id = Id::new(tab_id.to_string() + "_tab_rects_map");
        ui.memory_mut(|mem| {
            let mut rects_map: HashMap<String, Rect> = mem.data.get_temp::<HashMap<String, Rect>>(tab_rects_map_id)
                .unwrap_or_default();
            rects_map.insert(item_data.to_string(), rect);
            mem.data.insert_temp(tab_rects_map_id, rects_map);
        });
    }

    /// Get target tab rect from memory based on scroll target
    fn get_target_tab_rect(ui: &mut Ui, target: &str) -> Option<Rect> {
        let note_tab_rects_map_id = Id::new("note_tabbar_tab_rects_map");
        let file_tab_rects_map_id = Id::new("file_tabbar_tab_rects_map");
        
        ui.memory(|mem| {
            // Try to find in note tabs first
            if let Some(rects_map) = mem.data.get_temp::<HashMap<String, Rect>>(note_tab_rects_map_id) {
                if let Some(rect) = rects_map.get(target) {
                    return Some(*rect);
                }
            }
            // Then try file tabs
            if let Some(rects_map) = mem.data.get_temp::<HashMap<String, Rect>>(file_tab_rects_map_id) {
                if let Some(rect) = rects_map.get(target) {
                    return Some(*rect);
                }
            }
            None
        })
    }

    fn tab_scroll_area(ui: &mut Ui, store: &mut Store) {
        let old = ui.style().spacing.scroll.bar_width;
        ui.style_mut().spacing.scroll.bar_width = 2.0;           // Scroll bar width
        
        let scroll_bar_visibility = if store.config.show_scroll_bar {
            ScrollBarVisibility::VisibleWhenNeeded
        } else {
            ScrollBarVisibility::AlwaysHidden
        };
        
        ScrollArea::horizontal()
            .id_salt("tabbar_note_and_file_scroll")
            .auto_shrink(true)
            .scroll_bar_visibility(scroll_bar_visibility)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(note_tab_bar) = TabButtonBar::from_note_tabs(store, false) {
                        ui.add(note_tab_bar);
                    }
                    ui.separator();
                    if let Some(file_tab_bar) = TabButtonBar::from_file_tabs(store, true) {
                        ui.add(file_tab_bar);
                    }
                });
                ui.add_space(2.0);
                
                // Get scroll target (set when tab button or menu item is clicked)
                let scroll_target = Self::get_and_clear_scroll_target(ui);
                if let Some(target) = scroll_target {
                    if let Some(target_rect) = Self::get_target_tab_rect(ui, &target) {
                        ui.scroll_to_rect(target_rect, Some(Align::Center));
                    }
                }
            });
        ui.style_mut().spacing.scroll.bar_width = old;
    }

    fn tab_more_menu(ui: &mut Ui, store: &mut Store) {
        ui.menu_button("...", |ui| {
            let close_notes_galley = galley_builder(ui)
                .icon(IconName::icon_clear)
                .text(tr("tabbar.close_all_notes"))
                .font_size(TAB_FONT_SIZE)
                .build();
            let close_notes_button = Button::new(close_notes_galley);
            if close_notes_button.ui(ui).clicked() {
                store.close_all_notes();
                ui.close();
            }
            
            let close_files_galley = galley_builder(ui)
                .icon(IconName::icon_clear)
                .text(tr("tabbar.close_all_files"))
                .font_size(TAB_FONT_SIZE)
                .build();
            let close_files_button = Button::new(close_files_galley);
            if close_files_button.ui(ui).clicked() {
                store.close_all_files();
                ui.close();
            }
            
            let close_all_galley = galley_builder(ui)
                .icon(IconName::icon_clear)
                .text(tr("tabbar.close_all"))
                .font_size(TAB_FONT_SIZE)
                .build();
            let close_all_button = Button::new(close_all_galley);
            if close_all_button.ui(ui).clicked() {
                store.close_all();
                ui.close();
            }
            ui.separator();
            
            // Show scroll bar toggle
            let show_scroll_bar_galley = galley_builder(ui)
            .text(tr("tabbar.show_scroll_bar"))
            .font_size(TAB_FONT_SIZE)
            .build();
            let mut show_scroll_bar = store.config.show_scroll_bar;
            if ui.checkbox(&mut show_scroll_bar, show_scroll_bar_galley).changed() {
                store.config.show_scroll_bar = show_scroll_bar;
                store.config_save();
            }
            
            ui.separator();

            // Scrollable area for file list
            ScrollArea::vertical()
                .show(ui, |ui| {
                    // Show notes first (sorted by name)
                    let notes = store.opened_files.notes_sorted();
                    for note in &notes {
                        let icon_galley = galley_builder(ui)
                            .icon(IconName::icon_bookmark1)
                            .text(note.name())
                            .font_size(TAB_FONT_SIZE)
                            .build();
                        let button = Button::new(icon_galley);
                        if button.ui(ui).clicked() {
                            let _ = store.open(&note.name4open());
                            Self::set_scroll_target(ui, note.name());
                            ui.close();
                        }
                    }
                    // Add separator between notes and files if both exist
                    if !notes.is_empty() && !store.opened_files.files().is_empty() {
                        ui.separator();
                    }
                    // Show files (sorted by name)
                    let files = store.opened_files.files_sorted();
                    for file in &files {
                        let file_path = file.path();
                        let icon = if file_path.starts_with("untitled/") {
                            IconName::icon_file_text
                        } else {
                            icon_name_from_filepath(&file_path)
                        };
                        let icon_galley = galley_builder(ui)
                            .icon(icon)
                            .text(file.name())
                            .font_size(TAB_FONT_SIZE)
                            .build();
                        let button = Button::new(icon_galley);
                        if button.ui(ui).on_hover_text(file.path()).clicked() {
                            let file_path = file.path();
                            let _ = store.open(&file.name4open());
                            // Use file path as key to match with tab rects map
                            Self::set_scroll_target(ui, file_path);
                            ui.close();
                        }
                    }
                });
            
            ui.separator();
            
            // Recent files submenu
            let recent_files_galley = galley_builder(ui)
                .text(tr("tabbar.recent_files"))
                .font_size(TAB_FONT_SIZE)
                .build();
            ui.menu_button(recent_files_galley, |ui| {
                // Get recent files and filter out already opened ones
                let opened_file_keys: std::collections::HashSet<String> = store.opened_files
                    .all()
                    .iter()
                    .map(|f| f.name4open())
                    .collect();
                
                let recent_files: Vec<String> = store.config.recent_files
                    .iter()
                    .filter(|key| !opened_file_keys.contains(*key))
                    .take(12)
                    .cloned()
                    .collect();
                
                if recent_files.is_empty() {
                    ui.label(tr("tabbar.no_recent_files"));
                } else {
                    for file_key in &recent_files {
                        let unifile = UniFile::from(file_key);
                        let icon = if unifile.is_note() {
                            IconName::icon_bookmark1
                        } else {
                            let file_path = unifile.path();
                            if file_path.starts_with("untitled/") {
                                IconName::icon_file_text
                            } else {
                                icon_name_from_filepath(&file_path)
                            }
                        };
                        let icon_galley = galley_builder(ui)
                            .icon(icon)
                            .text(unifile.name())
                            .font_size(TAB_FONT_SIZE)
                            .build();
                        let hover_text = if unifile.is_file() {
                            unifile.path()
                        } else {
                            unifile.name()
                        };
                        let button = Button::new(icon_galley);
                        if button.ui(ui).on_hover_text(&hover_text).clicked() {
                            let _ = store.open(&unifile.name4open());
                            if unifile.is_file() {
                                Self::set_scroll_target(ui, unifile.path());
                            } else {
                                Self::set_scroll_target(ui, unifile.name());
                            }
                            ui.close();
                        }
                    }
                }
            });
        });
    }

}

impl TabButtonBar<'_> {
    pub fn tab_button(ui: &mut Ui, item: &TabItem, font_size: f32) -> TabButtonResponse {
        let mut close_clicked = false;
        let visuals = ui.visuals().clone();
        let style = TabStyle::from_visuals(&visuals, item.selected);
        
        let response = ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            let frame = Frame::new()
                .fill(style.bg_fill)
                .stroke(style.stroke)
                .corner_radius(style.corner_radius);

            let inner = frame.show(ui, |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                //icon+text
                let mut builder = galley_builder(ui)
                    .text(&item.text)
                    .fg(ui.visuals().text_color())
                    .font_size(font_size);
                if let Some(icon) = &item.icon {
                    builder = builder.icon(icon.clone()).icon_size(font_size+4.0);
                }
                let combined_galley = builder.build();
                let button = Button::new(combined_galley).frame(false);
                let button_response = button.ui(ui).on_hover_text(&item.data);
                //close_button or fixed_button
                if item.show_close {
                    let button_size = icon_button_builder(ui)
                        .icon(IconName::icon_clear)
                        .font_size(CLOSE_BUTTON_SIZE)
                        .size();
                    let button_pos = Pos2::new(
                        button_response.rect.right() + button_size.x / 4.0,
                        button_response.rect.center().y - button_size.y / 2.0,
                    );
                    ui.add_space(button_size.x * 1.5);
                    let id: String = format!("tab_bar_{}_{}_close", item.text, item.data);
                    if item.is_note {
                        let icon = if item.is_fixed {
                            IconName::icon_fixed
                        } else {
                            IconName::icon_clear
                        };
                        close_clicked = icon_button_builder(ui)
                            .icon(icon)
                            .icon_hovered(IconName::icon_clear)
                            .pos(button_pos)
                            .id(id)
                            .font_size(CLOSE_BUTTON_SIZE)
                            .fg_hovered(Color32::RED)
                            .build_inner();
                    } else {
                        // For files, show close button
                        close_clicked = icon_button_builder(ui)
                            .icon(IconName::icon_clear)
                            .pos(button_pos)
                            .id(id)
                            .font_size(CLOSE_BUTTON_SIZE)
                            .fg_hovered(Color32::RED)
                            .build_inner();
                    }
                } 
                button_response
            });
            (inner.response, inner.inner)
        });

        let frame_response = response.inner.0;
        let button_response = response.inner.1;
        let mut union_response = button_response.union(frame_response);
        
        // Merge drag response instead of overwriting
        let drag_response = ui.interact(union_response.rect, union_response.id, Sense::click_and_drag());
        union_response = union_response.union(drag_response);
        
        if union_response.hovered() && !union_response.dragged() {
            let hover_color = ui.visuals().selection.bg_fill.linear_multiply(0.3);
            ui.painter().rect_filled(union_response.rect, style.corner_radius, hover_color);
        }

        // Draw top highlight line for selected tab
        /* 
        if item.selected {
            let line_color = visuals.selection.bg_fill;
            let line_thickness = 1.0;
            let line_rect = Rect::from_min_max(
                Pos2::new(union_response.rect.left(), union_response.rect.top()),
                Pos2::new(union_response.rect.right(), union_response.rect.top() + line_thickness)
            );
            ui.painter().rect_filled(line_rect, 0.0, line_color);
        }
        */

        TabButtonResponse {
            response: union_response,
            close_clicked,
        }
    }

    /// Detect drag target index
    fn detect_drag_target_index(
        ui: &mut Ui,
        tab_rects: &[Rect],
        drag_source_index: Option<usize>,
        pointer_pos: Pos2,
    ) -> Option<usize> {
        let mut drag_target_index = None;
        
        if let Some(source_idx) = drag_source_index {
            // Iterate through all tabs to find insertion point corresponding to mouse position
            for (index, rect) in tab_rects.iter().enumerate() {
                if index == source_idx {
                    continue; 
                }
                
                // Check if mouse is within tab's rect
                if rect.contains(pointer_pos) {
                    // Determine if mouse is on left or right side of tab
                    let rect_center_x = rect.center().x;
                    let pointer_x = pointer_pos.x;
                    
                    let is_left = pointer_x < rect_center_x;
                    if is_left {
                        drag_target_index = Some(index);
                    } else {
                        drag_target_index = Some(index + 1);
                    }
                    
                    //println!("drag_target_index updated: {:?} (index: {}, left: {})", 
                    //    drag_target_index, index, is_left);
                    
                    // Draw vertical line at insertion position
                    let stroke = Stroke::new(2.0, ui.visuals().selection.bg_fill);
                    let line_x = if is_left {
                        rect.left()
                    } else {
                        rect.right()
                    };
                    ui.painter().vline(line_x, rect.y_range(), stroke);
                    break;
                }
            }
            
            // If mouse is not in any tab, check if it's on left or right side of tab bar
            if drag_target_index.is_none() {
                if !tab_rects.is_empty() {
                    let first_rect = &tab_rects[0];
                    let last_rect = &tab_rects[tab_rects.len() - 1];
                    let stroke = Stroke::new(2.0, ui.visuals().selection.bg_fill);
                    if pointer_pos.x < first_rect.left() {
                        drag_target_index = Some(0);
                        ui.painter().vline(first_rect.left(), first_rect.y_range(), stroke);
                    } else if pointer_pos.x > last_rect.right() {
                        drag_target_index = Some(tab_rects.len());
                        ui.painter().vline(last_rect.right(), last_rect.y_range(), stroke);
                    }
                }
            }
        }
        
        drag_target_index
    }

    /// Draw dragging tab (follows mouse)
    fn draw_dragging_tab(
        ui: &mut Ui,
        source_item: &TabItem,
        source_rect: Rect,
        pointer_pos: Pos2,
        font_size: f32,
    ) {
        let visuals = ui.visuals();
        let drag_rect = Rect::from_min_size(pointer_pos, source_rect.size());
        let drag_color = visuals.selection.bg_fill.linear_multiply(0.7);
        
        // Draw dragging tab
        let style = TabStyle::from_visuals(visuals, source_item.selected);
        ui.painter().rect_filled(drag_rect, style.corner_radius, drag_color);
        ui.painter().rect_stroke(drag_rect, style.corner_radius, style.stroke, StrokeKind::Outside);
        
        // Draw text
        let mut builder = galley_builder(ui)
            .text(&source_item.text)
            .fg(ui.visuals().text_color())
            .font_size(font_size);
        if let Some(icon) = &source_item.icon {
            builder = builder.icon(icon.clone()).icon_size(font_size);
        }
        let galley = builder.build();
        let text_pos = Pos2::new(
            drag_rect.left() + 4.0,
            drag_rect.center().y - galley.size().y / 2.0
        );
        ui.painter().add(epaint::TextShape::new(text_pos, galley, ui.visuals().text_color()));
    }

    #[allow(dead_code)]
    fn draw_tab_bottom_line(ui: &mut Ui, rect: Rect) {
        let visuals = ui.visuals();
        let line_color = visuals.selection.bg_fill;
        
            let available_width = ui.available_rect_before_wrap().right();
            if rect.right() < available_width {
                let line_rect = eframe::egui::Rect::from_min_max(
                    eframe::egui::Pos2::new(rect.left(), rect.bottom()),
                    eframe::egui::Pos2::new(available_width, rect.bottom() + 1.0)
                );
                ui.painter().rect_filled(line_rect, 0.0, line_color);
            }
    }
}

impl Widget for TabButtonBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let top = ui.cursor().left_top();
        let mut response = ui.allocate_rect(Rect::from_pos(top), Sense::click());

        // Use memory to save drag state, avoid being cleared on each call
        let drag_state_id = Id::new(self.id.clone() + "_drag_state");
        let drag_state: (Option<usize>, Option<usize>) = ui.memory(|mem| {
            mem.data.get_temp::<(Option<usize>, Option<usize>)>(drag_state_id)
                .unwrap_or((None, None))
        });
        
        let mut drag_source_index = drag_state.0;
        let mut drag_target_index = drag_state.1;

        let mut tab_rects = Vec::new();
        let mut need_click = None;
        let mut need_close = None;
        let mut need_new_file = false;

        let horizontal_response = ui.with_layout(Layout::left_to_right(Align::Center), |ui|{
        //let horizontal_response = ui.horizontal(|ui|{
            ui.spacing_mut().item_spacing.x = 2.0;
            for (index, item) in self.items.iter().enumerate() {
                let tab_resp = Self::tab_button(ui, item, TAB_FONT_SIZE);
                tab_rects.push(tab_resp.response.rect);
                
                // Store all tab rects for scrolling (keyed by item.data)
                Self::store_tab_rect(ui, &self.id, &item.data, tab_resp.response.rect);
                
                // Detect drag start
                if tab_resp.response.drag_started() {
                    drag_source_index = Some(index);
                }
                // Detect click (not drag)
                if tab_resp.response.clicked() && !tab_resp.response.dragged() {
                    need_click = Some(item.data.clone());
                    Self::set_scroll_target(ui, item.data.clone());
                }
                if tab_resp.response.double_clicked() && item.show_close {
                    need_close = Some(item.data.clone());
                }
                if tab_resp.close_clicked {
                    need_close = Some(item.data.clone());
                }

                response |= tab_resp.response;
            }
        });

        // Allocate interaction area for right blank space
        if self.extend_space {
            let available_rect = ui.available_rect_before_wrap();
            let available_right = available_rect.right() - 36.0;
            let tabs_end_x = if tab_rects.is_empty() {
                top.x
            } else {
                horizontal_response.response.rect.right()
            };
            let tab_row_height = if horizontal_response.response.rect.height() > 1.0 {
                horizontal_response.response.rect.height()
            } else {
                    // Fallback height when there are no tabs rendered
                    let spacing = ui.spacing();
                    let padding = spacing.button_padding.y * 2.0;
                    (TAB_FONT_SIZE + padding).max(spacing.interact_size.y)
            };
            if tabs_end_x < available_right{
                let blank_rect = Rect::from_min_max(
                    Pos2::new(tabs_end_x, top.y),
                    Pos2::new(available_right, top.y + tab_row_height)
                );
                let blank_response = ui.allocate_rect(blank_rect, Sense::click());
                
                // Detect double click on blank area
                if blank_response.double_clicked() {
                    need_new_file = true;
                }
                
                response |= blank_response;
            }
        }
        
        // Draw bottom horizontal line
        //Self::draw_tab_bottom_line(ui, response.rect);

        // During dragging, need to continuously detect target position
        let is_dragging = drag_source_index.is_some() && ui.input(|i| i.pointer.primary_down());
        if is_dragging {
            if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                drag_target_index = Self::detect_drag_target_index(
                    ui,
                    &tab_rects,
                    drag_source_index,
                    pointer_pos,
                );
            }
        }

        // Update drag state to memory (before detecting drag end)
        ui.memory_mut(|mem| {
            mem.data.insert_temp(drag_state_id, (drag_source_index, drag_target_index));
        });

        // Detect drag end: mouse release
        if let Some(source_idx) = drag_source_index {
            let drag_stopped = response.drag_stopped() || !is_dragging;
            if drag_stopped {
                //println!("drag_stopped, source_idx: {:?}, target_idx: {:?}", source_idx, drag_target_index);
                // Clear drag state
                ui.memory_mut(|mem| {
                    mem.data.remove::<(Option<usize>, Option<usize>)>(drag_state_id);
                });
                
                if let Some(target_idx) = drag_target_index {
                    // Drag ended, reorder
                    // target_idx is already the calculated insertion position (considering left/right side)
                    if source_idx != target_idx {
                        if let Some(file_path) = self.items.get(source_idx).map(|item| item.data.clone()) {
                            // Determine if it's a note or file (distinguish by id)
                            let is_note = self.id == "note_tabbar";
                            // Use Store's unified method to move
                            self.store.move_file_to_position(&file_path, target_idx, is_note);
                        }
                    }
                }
            }
            
            // Draw dragging tab (follows mouse)
            // Display throughout the entire drag process, as long as mouse is still pressed
            if is_dragging {
                if let Some(source_item) = self.items.get(source_idx) {
                    if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                        Self::draw_dragging_tab(
                            ui,
                            source_item,
                            tab_rects[source_idx],
                            pointer_pos,
                            TAB_FONT_SIZE,
                        );
                    }
                }
            }
        }

        // Handle click events
        if let Some(data) = need_click {
            (self.on_click)(self.store, &data);
        }
        if let Some(data) = need_close {
            (self.on_close)(self.store, &data);
        }
        if need_new_file {
            self.store.create_untitled_file();
        }

        response
    }
}

struct TabStyle {
    bg_fill: Color32,
    stroke: Stroke,
    corner_radius: f32,
}

impl TabStyle {
    fn from_visuals(visuals: &Visuals, selected: bool) -> Self {
        if selected {
            Self {
                bg_fill: visuals.selection.bg_fill,
                stroke: Stroke::NONE,
                corner_radius: 1.0,
            }
        } else {
            Self {
                bg_fill: Color32::TRANSPARENT,
                stroke: Stroke::NONE,
                corner_radius: 1.0,
            }
        }
    }
}

