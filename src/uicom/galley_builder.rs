use std::sync::Arc;
use eframe::egui::{Color32, Galley, Ui, FontSelection};
use eframe::egui::epaint::text::{FontFamily, LayoutJob, TextFormat};
use crate::uicom::IconName;

/// Galley builder with chainable calls
pub struct GalleyBuilder<'a> {
    ui: &'a Ui,
    text: Option<String>,
    icon: Option<IconName>,
    fg: Option<Color32>,
    bg: Option<Color32>,
    wrap_width: Option<f32>,
    font_size: Option<f32>,
    icon_size: Option<f32>,
}

impl<'a> GalleyBuilder<'a> {
    /// Create new builder
    pub fn new(ui: &'a Ui) -> Self {
        Self {
            ui,
            text: None,
            icon: None,
            fg: None,
            bg: None,
            wrap_width: None,
            font_size: None,
            icon_size: None,
        }
    }

    /// Set text
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set icon
    pub fn icon(mut self, icon_name: IconName) -> Self {
        self.icon = Some(icon_name);
        self
    }

    /// Set foreground color
    pub fn fg(mut self, color: Color32) -> Self {
        self.fg = Some(color);
        self
    }

    /// Set background color
    pub fn bg(mut self, color: Color32) -> Self {
        self.bg = Some(color);
        self
    }

    /// Set wrap width
    pub fn wrap_width(mut self, width: f32) -> Self {
        self.wrap_width = Some(width);
        self
    }

    /// Set font size
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Set icon size
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = Some(size);
        self
    }

    /// Build final Galley
    pub fn build(self) -> Arc<Galley> {
        let mut layout_job = LayoutJob::default();
        let default_font_id = FontSelection::Default.resolve(self.ui.style());
        let default_fg = self.fg.unwrap_or_else(|| self.ui.style().visuals.text_color());
        let default_bg = self.bg.unwrap_or(Color32::TRANSPARENT);
        let default_font_size = self.font_size.unwrap_or(default_font_id.size);
        let default_icon_size = self.icon_size.unwrap_or(default_font_size);

        // If there's an icon, add it first
        if let Some(icon) = self.icon {
            let mut icon_format = TextFormat::default();
            icon_format.font_id.size = default_icon_size;
            icon_format.font_id.family = FontFamily::Name("icon".into());
            icon_format.color = default_fg;
            icon_format.background = default_bg;
            layout_job.append(&icon.to_char().to_string(), 0.0, icon_format);
        }

        // If there's text, add text
        if let Some(text) = self.text {
            let mut text_format = TextFormat::default();
            text_format.font_id = default_font_id;
            text_format.font_id.size = default_font_size;
            text_format.color = default_fg;
            layout_job.append(&text, 0.0, text_format);
        }

        // Set wrap width
        if let Some(wrap_width) = self.wrap_width {
            layout_job.wrap.max_width = wrap_width;
        }

        self.ui.fonts_mut(|f| f.layout_job(layout_job))
    }
}

/// Convenience function to create Galley builder
pub fn galley_builder(ui: &Ui) -> GalleyBuilder<'_> {
    GalleyBuilder::new(ui)
}
