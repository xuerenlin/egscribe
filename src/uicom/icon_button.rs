use eframe::egui::epaint;
use eframe::egui::{Button, Color32, FontId, Pos2, Rect, Response, Sense, Ui, Vec2, Widget, FontSelection};
use eframe::egui::epaint::text::{FontFamily, LayoutJob, TextFormat};
use crate::uicom::IconName;

/// Icon button builder with chainable calls
pub struct IconButtonBuilder<'a> {
    ui: &'a mut Ui,
    icon_name: Option<IconName>,
    icon_name_hovered: Option<IconName>,
    font_size: Option<f32>,
    pos: Option<Pos2>,
    id: Option<String>,
    fg_normal: Option<Color32>,
    fg_hovered: Option<Color32>,
    fg_actived: Option<Color32>,
    bg_normal: Option<Color32>,
    bg_hovered: Option<Color32>,
    bg_actived: Option<Color32>,
    hover_text: Option<String>,
    active: Option<bool>,
    expand: Option<f32>,
}

impl<'a> IconButtonBuilder<'a> {
    /// Create new builder
    pub fn new(ui: &'a mut Ui) -> Self {
        Self {
            ui,
            icon_name: None,
            icon_name_hovered: None,
            font_size: None,
            pos: None,
            id: None,
            fg_normal: None,
            fg_hovered: None,
            fg_actived: None,
            bg_normal: None,
            bg_hovered: None,
            bg_actived: None,
            hover_text: None,
            active: None,
            expand: None,
        }
    }

    /// Set icon
    pub fn icon(mut self, icon_name: IconName) -> Self {
        self.icon_name = Some(icon_name);
        self
    }

    /// Set icon for hovered state
    pub fn icon_hovered(mut self, icon_name: IconName) -> Self {
        self.icon_name_hovered = Some(icon_name);
        self
    }

    /// Set font size
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Set position (for inner buttons)
    pub fn pos(mut self, pos: Pos2) -> Self {
        self.pos = Some(pos);
        self
    }

    /// Set ID (for inner buttons)
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set foreground color (normal state)
    pub fn fg(mut self, color: Color32) -> Self {
        self.fg_normal = Some(color);
        self
    }

    /// Set foreground color (hovered state)
    pub fn fg_hovered(mut self, color: Color32) -> Self {
        self.fg_hovered = Some(color);
        self
    }

    /// Set foreground color (actived state)
    pub fn fg_actived(mut self, color: Color32) -> Self {
        self.fg_actived = Some(color);
        self
    }

    /// Set background color (normal state)
    pub fn bg(mut self, color: Color32) -> Self {
        self.bg_normal = Some(color);
        self
    }

    /// Set background color (hovered state)
    pub fn bg_hovered(mut self, color: Color32) -> Self {
        self.bg_hovered = Some(color);
        self
    }

    /// Set background color (actived state)
    pub fn bg_actived(mut self, color: Color32) -> Self {
        self.bg_actived = Some(color);
        self
    }

    /// Set hover text
    pub fn hover_text(mut self, text: impl Into<String>) -> Self {
        self.hover_text = Some(text.into());
        self
    }

    /// Set active state
    pub fn active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    /// Set expand size (for inner button click area)
    pub fn expand(mut self, expand: f32) -> Self {
        self.expand = Some(expand);
        self
    }

    /// Build inner icon button (draw at specified position, return whether clicked)
    pub fn build_inner(self) -> bool {
        let icon_name = self.icon_name.expect("icon is required");
        let pos = self.pos.expect("pos is required");
        let id = self.id.expect("id is required");
        let font_size = self.font_size.unwrap_or_else(|| {
            FontSelection::Default.resolve(self.ui.style()).size
        });
        let expand = self.expand.unwrap_or(2.0);

        // Get default colors
        let default_fg_normal = self.ui.visuals().weak_text_color();
        let default_fg_hovered = self.ui.visuals().text_color();
        
        // Use colors set by builder, or use defaults if not set
        let fg_normal = self.fg_normal.unwrap_or(default_fg_normal);
        let fg_hovered = self.fg_hovered.unwrap_or(default_fg_hovered);

        let icon_char = icon_name.to_char();
        let icon_char_hovered = self.icon_name_hovered.map(|icon| icon.to_char()).unwrap_or(icon_char);
        
        let job_normal = layout_job_from_icon(self.ui, icon_char, font_size, fg_normal);
        let job_hovered = layout_job_from_icon(self.ui, icon_char_hovered, font_size, fg_hovered);
        let galley = self.ui.fonts_mut(|f| f.layout_job(job_normal));
        let galley_rect = Rect::from_min_size(pos, galley.size());
        let galley_rect = galley_rect.expand(expand);

        let response = self.ui.interact(galley_rect, id.clone().into(), Sense::click());
        if response.hovered() {
            let galley = self.ui.fonts_mut(|f| f.layout_job(job_hovered));
            self.ui.painter().add(epaint::TextShape::new(pos, galley, fg_hovered));
        } else {
            self.ui.painter().add(epaint::TextShape::new(pos, galley, fg_normal));
        }
        response.clicked()
    }

    /// Build tool icon button (returns Response)
    pub fn build_tool(self) -> Response {
        let icon_name = self.icon_name.expect("icon is required");
        let is_active = self.active.unwrap_or(false);
        let icon_size = self.font_size;
        
        // Get default colors
        let default_fg_normal = self.ui.visuals().text_color();
        let default_bg_normal = Color32::TRANSPARENT;
        let default_bg_actived = self.ui.visuals().selection.bg_fill;
        
        // Select color based on state
        // Note: Button component's hover effect is automatically handled by egui
        // If custom hover colors are needed, can use fg_hovered and bg_hovered
        // But need to redraw on hover, which requires more complex implementation
        let fg = if is_active {
            self.fg_actived.unwrap_or(self.fg_normal.unwrap_or(default_fg_normal))
        } else {
            self.fg_normal.unwrap_or(default_fg_normal)
        };
        
        let bg = if is_active {
            self.bg_actived.unwrap_or(self.bg_normal.unwrap_or(default_bg_actived))
        } else {
            self.bg_normal.unwrap_or(default_bg_normal)
        };

        let button = Button::new(super::galley_builder(self.ui)
            .icon(icon_name)
            .icon_size(icon_size.unwrap_or_else(|| FontSelection::Default.resolve(self.ui.style()).size))
            .bg(bg)
            .fg(fg)
            .build())
            .fill(Color32::TRANSPARENT);
        
        let mut response = button.ui(self.ui);
        
        if let Some(hover_text) = self.hover_text {
            response = response.on_hover_text(hover_text);
        }
        response
    }

    /// Get icon size
    pub fn size(self) -> Vec2 {
        let icon_name = self.icon_name.expect("icon is required");
        let font_size = self.font_size.unwrap_or_else(|| {
            FontSelection::Default.resolve(self.ui.style()).size
        });

        let icon_char = icon_name.to_char();
        let default_color = self.ui.visuals().weak_text_color();
        let job = layout_job_from_icon(self.ui, icon_char, font_size, default_color);
        let galley = self.ui.fonts_mut(|f| f.layout_job(job));
        galley.size()
    }
}

/// Convenience function to create icon button builder
pub fn icon_button_builder(ui: &mut Ui) -> IconButtonBuilder<'_> {
    IconButtonBuilder::new(ui)
}

// Helper function
fn layout_job_from_icon(ui: &Ui, icon_char: char, font_size: f32, color: Color32) -> LayoutJob {
    let mut job: LayoutJob = LayoutJob::default();
    let mut format = TextFormat::default();
    format.font_id.size = font_size;
    format.font_id.family = FontFamily::Name("icon".into());
    format.color = color;

    job.append(&icon_char.to_string(), 0.0, format);
    job
}

/// Set UI button font
pub fn set_ui_button_font(ui: &mut Ui) {
    let mut font_id = FontId::default();
    //font_id.size = 16.0;
    font_id.family = eframe::egui::FontFamily::Monospace; 
    ui.style_mut().override_font_id = Some(font_id);
}


