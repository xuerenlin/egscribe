use crate::medit::{icon, ImageInfo, CharRect, Ctx, Cursor, PghItem};
use eframe::egui::{vec2, Image, Pos2, Rect, Response, Ui, Vec2};
use regex::Regex;

const SPACE_X: f32 = 8.0;
const SPACE_INDENT_X: f32 = 16.0;

fn pos_from_cursor(char_rect: &Option<Vec<CharRect>>, cursor: &Cursor) -> Option<Rect> {
    if let Some(char_rect) = char_rect {
        if let Some(c_rect) = char_rect.get(cursor.culumn) {
            let mut zero_width_rect = c_rect.rect;
            zero_width_rect.set_width(0.0);
            return Some(zero_width_rect);
        }
    }
    None
}

fn cursor_from_pos(char_rect: &Option<Vec<CharRect>>, line_no: usize, segment: usize, pos: &Pos2) -> Option<Cursor> {
    if let Some(plist) = char_rect {
        for (i, c_rect) in plist.into_iter().enumerate() {
            let rect = c_rect.rect;
            if rect.min.x <= pos.x && rect.max.x >= pos.x && rect.min.y <= pos.y && rect.max.y >= pos.y {
                return Some(Cursor {
                    line_no,
                    segment,
                    culumn: c_rect.i,
                });
            }
        }
    }
    None
}

fn item_char_rect(rect: &Rect) -> Vec<CharRect> {
    let min = Pos2 {
        x: rect.max.x,
        y: rect.min.y,
    };
    let max = Pos2 {
        x: rect.max.x,
        y: rect.max.y,
    };
    let end_rect = Rect::from_min_max(min, max);

    vec![
        CharRect {
            rect: rect.clone(),
            i: 0,
            c: '\0',
            top: 0.0,
            bottom: 0.0,
        },
        CharRect {
            rect: end_rect,
            i: 1,
            c: '\0',
            top: 0.0,
            bottom: 0.0,
        },
    ]
}

fn simple_allocate_rect(ui: &mut Ui, ctx: &mut Ctx, w:f32, h:f32) -> Response {
    let mut space_rect = ui.cursor();
    space_rect.set_width(w);
    space_rect.set_height(h);
    ui.allocate_rect(space_rect, ctx.sense())
}


#[derive(Clone)]
pub struct PghHead {
    text: String,
    char_rect: Option<Vec<CharRect>>,
    deep: u8,
}

impl PghHead {
    pub fn new(deep: u8) -> Self {
        PghHead {
            text: "X".to_string(),
            char_rect: None,
            deep,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
        pgh_text: &Box<dyn PghItem>,
    ) -> Response {
        //let row_height = ctx.font_heigh();
        let (rect, mut response) = ui.allocate_exact_size(vec2(1.0, 0.1), ctx.sense());
        ui.painter().line_segment(
            [rect.center_top(), rect.center_bottom()],
            (0.1, ui.visuals().weak_text_color()),
        );

        //update rect info
        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );

        //space_x
        response |= ui.allocate_exact_size(vec2(SPACE_X, rect.height()), ctx.sense()).1;

        response
    }
}

impl PghItem for PghHead {
    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        pos_from_cursor(&self.char_rect, cursor)
    }

    fn cursor_from_pos(&self, line_no: usize, segment: usize, pos: &Pos2) -> Option<Cursor> {
        cursor_from_pos(&self.char_rect, line_no, segment, pos)
    }
}

#[derive(Clone)]
pub struct PghIndent {
    text: String,
    char_rect: Option<Vec<CharRect>>,
}

impl PghIndent {
    pub fn new() -> Self {
        PghIndent {
            text: "X".to_string(),
            char_rect: None,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
    ) -> Response {
        let response = simple_allocate_rect(ui, ctx, SPACE_INDENT_X, ctx.font_heigh());
        //update rect info
        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );
        response
    }
}

impl PghItem for PghIndent {
    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        pos_from_cursor(&self.char_rect, cursor)
    }

    fn cursor_from_pos(&self, line_no: usize, segment: usize, pos: &Pos2) -> Option<Cursor> {
        cursor_from_pos(&self.char_rect, line_no, segment, pos)
    }
}

#[derive(Clone)]
pub struct PghCheckBox {
    text: String,
    char_rect: Option<Vec<CharRect>>,
}

impl PghCheckBox {
    pub fn new() -> Self {
        PghCheckBox {
            text: "X".to_string(),
            char_rect: None,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
        pgh_text: &Box<dyn PghItem>,
    ) -> Response {
        let text = ctx.get_line_text(line_no);
        let re = Regex::new(r"^-[ \t]+\[x\] ").unwrap();
        let mut checked = re.is_match(&text);
        let mut response = ui.checkbox(&mut checked, "");
        if response.changed() && text.len() >= 5 {
            let new_s = if checked {
                text.replace(" [ ] ", " [x] ")
            } else {
                text.replace(" [x] ", " [ ] ")
            };
            ctx.update_line_text(line_no, new_s);
        }

        //update rect info
        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );
        response |= ui.allocate_exact_size(vec2(SPACE_X, response.rect.height()), ctx.sense()).1;

        response
    }
}

impl PghItem for PghCheckBox {
    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        pos_from_cursor(&self.char_rect, cursor)
    }

    fn cursor_from_pos(&self, line_no: usize, segment: usize, pos: &Pos2) -> Option<Cursor> {
        cursor_from_pos(&self.char_rect, line_no, segment, pos)
    }
}

#[derive(Clone)]
pub struct PghPoint {
    text: String,
    char_rect: Option<Vec<CharRect>>,
}

impl PghPoint {
    pub fn new() -> Self {
        PghPoint {
            text: "X".to_string(),
            char_rect: None,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
        pgh_text: &Box<dyn PghItem>,
    ) -> Response {
        let row_height = ctx.font_heigh();
        let mut response = simple_allocate_rect(ui, ctx, row_height, row_height);
        let rect = response.rect;

        ui.painter().circle_filled(
            rect.center(),
            rect.height() / 7.0,
            ctx.cfg().text_color(),
        );

        //update rect info
        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );

        //space_x
        response |= simple_allocate_rect(ui, ctx, SPACE_X, rect.height());

        response
    }
}

impl PghItem for PghPoint {
    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        pos_from_cursor(&self.char_rect, cursor)
    }

    fn cursor_from_pos(&self, line_no: usize, segment: usize, pos: &Pos2) -> Option<Cursor> {
        cursor_from_pos(&self.char_rect, line_no, segment, pos)
    }
}

#[derive(Clone)]
pub struct PghQuoteIndent {
    text: String,
    char_rect: Option<Vec<CharRect>>,
}

impl PghQuoteIndent {
    pub fn new() -> Self {
        PghQuoteIndent {
            text: "X".to_string(),
            char_rect: None,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
        pgh_text: &Box<dyn PghItem>,
    ) -> Response {
        let row_height = ctx.font_heigh();
        let mut response = simple_allocate_rect(ui, ctx, 3.0, row_height);
        let rect = response.rect;
        let fill_rect = rect
            .expand2(ui.style().spacing.item_spacing * 0.5)
            .expand2(Vec2 { x: 0.0, y: 1.0 });
        ui.painter()
            .rect_filled(fill_rect, 1.0, ui.visuals().weak_text_color());

        //update rect info
        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );

        //space_x
        response |= simple_allocate_rect(ui, ctx, SPACE_X, row_height);

        response
    }
}

impl PghItem for PghQuoteIndent {
    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        pos_from_cursor(&self.char_rect, cursor)
    }

    fn cursor_from_pos(&self, line_no: usize, segment: usize, pos: &Pos2) -> Option<Cursor> {
        cursor_from_pos(&self.char_rect, line_no, segment, pos)
    }
}

#[derive(Clone)]
pub struct PghBreak {
    text: String,
    char_rect: Option<Vec<CharRect>>,
}

impl PghBreak {
    pub fn new() -> Self {
        PghBreak {
            text: "X".to_string(),
            char_rect: None,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
        pgh_text: &Box<dyn PghItem>,
    ) -> Response {
        let row_height = ctx.font_heigh();
        let (rect, response) = ui.allocate_exact_size(vec2(ctx.edit_width(), row_height), ctx.sense());
        let line_rect = rect.expand2(Vec2 {
            x: 0.0,
            y: -rect.height() / 2.0 + 0.75,
        });
        ui.painter()
            .rect_filled(line_rect, 1.0, ui.visuals().weak_text_color());

        //update rect info
        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );

        response
    }
}

impl PghItem for PghBreak {
    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        pos_from_cursor(&self.char_rect, cursor)
    }

    fn cursor_from_pos(&self, line_no: usize, segment: usize, pos: &Pos2) -> Option<Cursor> {
        if let Some(plist) = &self.char_rect {
            for (i, c_rect) in plist.into_iter().enumerate() {
                let rect = c_rect.rect;
                if rect.max.x >= pos.x && rect.min.y <= pos.y && rect.max.y >= pos.y {
                    return Some(Cursor {
                        line_no,
                        segment,
                        culumn: c_rect.i,
                    });
                }
            }
        }
        None
    }
}



#[derive(Clone)]
pub struct PghIcon {
    text: String,
    char_rect: Option<Vec<CharRect>>,
    icon_name: icon::IconName,
}

impl PghIcon {
    pub fn new(icon_name: icon::IconName) -> Self {
        PghIcon {
            text: "".to_string(),
            char_rect: None,
            icon_name,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
        pgh_text: &Box<dyn PghItem>,
    ) -> Response {
        let row_height = ctx.font_heigh();

        //get pos and font-size for diffrent icon_type
        let mut pos = ui.cursor().left_top();
        let mut font_size = ctx.font_size();
        let mut spacing_x = 2.0;
        let mut spacing_y = 0.0;
        let icon_name = pgh_text.icon_name().unwrap();
        match icon_name {
            icon::IconName::icon_external_link(_) => {
                spacing_x = 4.0;
                spacing_y = 2.0;
                font_size = ctx.font_size() * 0.7;
            }
            _ => {}
        }
        let mut icon_size = icon::icon_size(ui, icon_name.clone(), font_size);
        icon_size.x += spacing_x * 2.0;
        pos.x += spacing_x;
        pos.y += spacing_y;

        let id = format!("icon_{}_{}", line_no, segment);
        let (rect, mut response) = ui.allocate_exact_size(icon_size, ctx.sense());
        if icon::icon_button(ui, id, pos, icon_name, font_size) {
            println!("icon clicked");
            response.clicked = true;
        }
        
        //update rect info
        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );

        response
    }
}

impl PghItem for PghIcon {
    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        pos_from_cursor(&self.char_rect, cursor)
    }

    fn cursor_from_pos(&self, line_no: usize, segment: usize, pos: &Pos2) -> Option<Cursor> {
        cursor_from_pos(&self.char_rect, line_no, segment, pos)
    }

    fn icon_name(&self) -> Option<icon::IconName> {
        Some(self.icon_name.clone())
    }
}


#[derive(Clone)]
pub struct PghImage {
    text: String,
    char_rect: Option<Vec<CharRect>>,
    image: ImageInfo,
}

impl PghImage {
    pub fn new(image: ImageInfo) -> Self {
        PghImage {
            text: "".to_string(),
            char_rect: None,
            image,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
        pgh_text: &Box<dyn PghItem>,
    ) -> Response {
        let image = pgh_text.image_info().unwrap();
        
        let url = if image.url.starts_with("file://") || image.url.starts_with("http://") || image.url.starts_with("https://") {
            image.url.clone()
        } else {
            if let Some(image_path) = &ctx.cfg().image_path {
                format!("file://{}/{}", image_path, image.url)
            } else {
                image.url.clone()
            }
        };
        
        //println!("show image:[{}] is_ok:{}", image.url, std::fs::metadata(&image.url).is_ok());
        let width = ui.available_width();
        let response = ui.add(
            Image::new(&url)
                .fit_to_original_size(1.0)
                .max_width(width*0.95)
                .rounding(10.0),
        );
        
        //update rect info
        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );

        response
    }
}

impl PghItem for PghImage {
    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        pos_from_cursor(&self.char_rect, cursor)
    }

    fn cursor_from_pos(&self, line_no: usize, segment: usize, pos: &Pos2) -> Option<Cursor> {
        cursor_from_pos(&self.char_rect, line_no, segment, pos)
    }

    fn image_info(&self) -> Option<ImageInfo> {
        Some(self.image.clone())
    }
}

