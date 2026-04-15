use eframe::egui::{Color32, FontId, Frame, FontFamily};
use super::pgh::TableFrameStyle;

/// Color scheme for editor
#[derive(Clone)]
pub struct EditColors {
    pub text_color: Color32,
    pub code_bg_color: Color32,
    pub link_color: Color32,
    pub weak_color: Color32,
    pub select_color: Color32,
    pub same_text_color: Color32,
}

/// Spacing configuration for different paragraph types
#[derive(Clone)]
pub struct ParagraphSpacing {
    pub top: f32,
    pub bottom: f32,
}

impl ParagraphSpacing {
    pub fn new(top: f32, bottom: f32) -> Self {
        Self { top, bottom }
    }
}

/// Spacing configuration for all paragraph types
#[derive(Clone)]
pub struct SpacingConfig {
    pub paragraph: ParagraphSpacing,
    pub heading: ParagraphSpacing,
    pub list: ParagraphSpacing,
    pub code: ParagraphSpacing,
    pub text: ParagraphSpacing,
    pub blockquote: ParagraphSpacing,
    pub thematic_break: ParagraphSpacing,
    pub table: ParagraphSpacing,
}

impl SpacingConfig {
    pub fn new(font_size: f32) -> Self {
        Self {
            paragraph: ParagraphSpacing::new(0.5, 0.5), 
            heading: ParagraphSpacing::new(0.0, 0.0),
            list: ParagraphSpacing::new(0.0, 0.0),
            code: ParagraphSpacing::new(8.0, 4.0),
            text: ParagraphSpacing::new(0.5, 0.5),
            blockquote: ParagraphSpacing::new(0.5, 0.5),
            thematic_break: ParagraphSpacing::new(0.0, 0.0),
            table: ParagraphSpacing::new(0.0, 0.0),
        }
    }

    pub fn update_font_size(&mut self, font_size: f32) {
        self.heading.top = font_size /3.0;
        self.heading.bottom = font_size / 3.0;
        self.list.bottom = font_size / 5.0;
        self.code.bottom = font_size / 3.0;
    }
}

/// Editor height mode
#[derive(Clone, Debug)]
pub enum HeightMode {
    Fixed(f32),
    Dynamic { min: f32, max: f32 },
}

impl HeightMode {
    pub fn fix_max() -> Self {
        Self::Fixed(f32::INFINITY)
    }
    
    pub fn fix_height(height: f32) -> Self {
        Self::Fixed(height)
    }
    
    pub fn dynamic_range(min: f32, max: f32) -> Self {
        Self::Dynamic { min, max }
    }
}

/// Editor configuration
#[derive(Clone)]
pub struct EditCfg {
    pub is_markdown: bool,
    pub image_path: Option<String>,     //save image in markdown
    pub lang: Option<String>,

    pub wrap: bool,
    pub show_line_no: bool,
    pub need_line_click_cmd: bool,
    pub hightlight_seleted_word: bool,
    pub is_read_only: bool,
    
    pub dark_mode: bool,
    pub height_mode: HeightMode,
    pub with_frame: Option<Frame>,
    pub font_size: f32,
    pub font_heigh: f32,
    pub indent_size: f32,
    pub indent_size_of_list: f32,
    pub text_color_brightness: f32, 
    pub is_monospace: bool,
    pub dark_color: EditColors,
    pub light_color: EditColors,
    pub spacing: SpacingConfig,
    pub table_frame_style: TableFrameStyle,
    pub show_heading_section_numbers: bool,
    pub show_table_row_no: bool,
}

impl EditCfg {
    pub fn new(font_size: f32, is_markdown: bool, image_path: Option<String>, height_mode: HeightMode) -> Self {
        Self {
            is_markdown,
            image_path,
            lang: None,
            wrap: false,
            show_line_no: false,
            need_line_click_cmd: false,
            hightlight_seleted_word: true,
            is_read_only: false,
            dark_mode: true,
            height_mode,
            with_frame: None,
            font_size,
            font_heigh: 23.0,
            indent_size: 16.0,
            indent_size_of_list: 24.0,
            text_color_brightness: 1.0,
            is_monospace: true,
            dark_color: EditColors {
                text_color: Color32::from_rgb(192,192,192),
                code_bg_color: Color32::from_gray(64),
                link_color: Color32::from_rgb(90, 170, 255),
                weak_color: Color32::from_rgb(100,100,100),
                select_color: Color32::from_rgb(0, 92, 128),
                same_text_color: Color32::from_rgb(0, 46, 86),
            },
            light_color: EditColors {
                text_color: Color32::from_rgb(0,0,0),
                code_bg_color: Color32::from_gray(230),
                link_color: Color32::from_rgb(0, 155, 255),
                weak_color: Color32::from_rgb(100,100,100),
                select_color: Color32::from_rgb(100, 209, 255),
                same_text_color: Color32::from_rgb(160, 209, 255),
            },
            spacing: SpacingConfig::new(font_size),
            table_frame_style: TableFrameStyle::Horizontal,
            show_heading_section_numbers: true,
            show_table_row_no: true,
        }
    }

    pub fn colors(&self) -> &EditColors {
        if self.dark_mode {
            &self.dark_color
        } else {
            &self.light_color
        }
    }

    pub fn text_color(&self) -> Color32 {
        // Apply brightness adjustment to text color
        self.colors().text_color.linear_multiply(self.text_color_brightness)
    }

    pub fn code_bg_color(&self) -> Color32 {
        self.colors().code_bg_color
    }

    pub fn link_color(&self) -> Color32 {
        self.colors().link_color
    }

    pub fn weak_color(&self) -> Color32 {
        self.colors().weak_color
    }

    pub fn select_color(&self) -> Color32 {
        self.colors().select_color
    }

    pub fn same_text_color(&self) -> Color32 {
        self.colors().same_text_color
    }

    /// Update font size and recalculate spacing
    pub fn set_font_size(&mut self, font_size: f32) {
        self.font_size = font_size;
        self.spacing.update_font_size(font_size);
    }

    pub fn set_frame(&mut self, frame: Frame) {
        self.with_frame = Some(frame);
    }

    /// Get font family based on is_monospace setting
    pub fn font_family(&self) -> FontFamily {
        if self.is_monospace {
            FontFamily::Monospace
        } else {
            FontFamily::Proportional
        }
    }

    /// Markdown ATX 标题（1..=6）的字号与粗体字体，与 [`crate::medit::md::MarkDownImpl::format_head`] 一致。
    pub fn heading_font_id(&self, depth: u8) -> FontId {
        if depth < 1 || depth > 6 {
            return FontId::new(self.font_size, self.font_family());
        }
        let max_font_size = self.font_size * 1.2;
        let delta_font_size = (max_font_size - self.font_size) / 6.0;
        let head_font_size = self.font_size + (7 - depth) as f32 * delta_font_size;
        FontId::new(head_font_size, FontFamily::Name("msyhb".into()))
    }
}

