use std::sync::Arc;

use crate::uicom::{galley_builder, IconName, CONTROL_HIGHLIGHT, icon_button_builder};
use crate::medit::{Ctx, Cursor, ImageInfo, LinkInfo};
use super::{CharRect, PghItem};
use eframe::egui::{
    Align, Color32, CornerRadius, CursorIcon, Galley, Image, Layout, Pos2, Rect, Response, Sense,
    Shape, Stroke, StrokeKind, Ui, Vec2, vec2,
};
use regex::Regex;

const SPACE_X: f32 = 8.0;
pub const QUOTE_INDENT_WIDTH: f32 = 4.0;

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
        for (_i, c_rect) in plist.into_iter().enumerate() {
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
pub struct PghIndent {
    char_rect: Option<Vec<CharRect>>,
}

impl PghIndent {
    pub fn new() -> Self {
        PghIndent {
            char_rect: None,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
        indent_size: f32,
    ) -> Response {
        let response = simple_allocate_rect(ui, ctx, indent_size, ctx.font_heigh());

        let full = response.rect;
        // 设置为右侧没有宽度的 rect，防止光标在左侧显示（segment 边界仍用整块缩进以命中点击）
        let zero_width_right = Rect::from_min_max(
            Pos2::new(full.right(), full.top()),
            Pos2::new(full.right(), full.bottom()),
        );
        ctx.update_view(
            line_no,
            segment,
            full,
            item_char_rect(&zero_width_right),
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

    fn cursor_from_pos(&self, _line_no: usize, _segment: usize, _pos: &Pos2) -> Option<Cursor> {
        None
    }
}

#[derive(Clone)]
pub struct PghCheckBox {
    char_rect: Option<Vec<CharRect>>,
}

impl PghCheckBox {
    pub fn new() -> Self {
        PghCheckBox {
            char_rect: None,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
    ) -> (bool, Response) {
        let text = ctx.get_line_text(line_no);
        let re_checked = Regex::new(r"^[ \t]*[-*+][ \t]+\[x\] ").unwrap();
        let _re_unchecked = Regex::new(r"^[ \t]*[-*+][ \t]+\[ \] ").unwrap();
        let mut checked = re_checked.is_match(&text);
        // 自绘 checkbox，控制大小和颜色
        let row_height = ctx.font_heigh();
        let box_size = vec2(row_height * 0.7, row_height * 0.7);
        let (rect, mut response) = ui.allocate_exact_size(box_size, ctx.sense());

        // 点击交互：切换状态并标记为 changed
        if response.clicked() {
            checked = !checked;
            response.mark_changed();
        }

        // 绘制外框
        let _visuals = ui.style().interact(&response);
        let stroke_color = CONTROL_HIGHLIGHT;
        let rounding = 1.0;
        let rect_inner = rect.shrink(1.0);
        ui.painter()
            .rect_stroke(
                rect_inner,
                rounding,
                Stroke::new(1.0, stroke_color),
                StrokeKind::Outside,
            );

        // 绘制对勾，使用固定紫色并加粗
        if checked {
            let check_color = stroke_color;
            let w = rect_inner.width();
            let h = rect_inner.height();
            let p1 = Pos2::new(rect_inner.left() + w * 0.2, rect_inner.center().y);
            let p2 = Pos2::new(rect_inner.left() + w * 0.45, rect_inner.bottom() - h * 0.2);
            let p3 = Pos2::new(rect_inner.right() - w * 0.2, rect_inner.top() + h * 0.2);

            ui.painter().line_segment([p1, p2], Stroke::new(2.8, check_color));
            ui.painter().line_segment([p2, p3], Stroke::new(2.8, check_color));
        }

        let mut changed = false;
        if response.changed() && text.len() >= 5 {
            let new_s = if checked {
                Regex::new(r"^([ \t]*[-*+][ \t]+)\[ \] ").unwrap()
                    .replace(&text, "$1[x] ")
                    .to_string()
            } else {
                Regex::new(r"^([ \t]*[-*+][ \t]+)\[x\] ").unwrap()
                    .replace(&text, "$1[ ] ")
                    .to_string()
            };
            ctx.update_line_text(line_no, new_s);
            changed = true;
        }

        //update rect info
        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );
        response |= ui.allocate_exact_size(vec2(SPACE_X, response.rect.height()), ctx.sense()).1;

        (changed, response)
    }
}

impl PghItem for PghCheckBox {
    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        pos_from_cursor(&self.char_rect, cursor)
    }

    fn cursor_from_pos(&self, _line_no: usize, _segment: usize, _pos: &Pos2) -> Option<Cursor> {
        None
        //cursor_from_pos(&self.char_rect, line_no, segment, pos)
    }
}

#[derive(Clone)]
pub struct PghOutlineFold {
    collapsed: bool,
    char_rect: Option<Vec<CharRect>>,
}

const OUTLINE_FOLD_CHIP_PAD: f32 = 4.0;
const OUTLINE_FOLD_CHIP_ROUNDING: u8 = 4;
const OUTLINE_FOLD_TRI_GAP: f32 = 3.0;
/// 圆角边框与三角相对设计尺寸的整体缩放（约为原来的 2/3）。
const OUTLINE_FOLD_CHIP_SCALE: f32 = 2.0 / 3.0;

fn outline_fold_hidden_count_label(hidden_lines: usize) -> Option<String> {
    if hidden_lines == 0 {
        None
    } else if hidden_lines > 99 {
        Some("99+".to_string() + "...")
    } else {
        Some(hidden_lines.to_string() + "...")
    }
}

fn outline_fold_chip_pad() -> f32 {
    OUTLINE_FOLD_CHIP_PAD * OUTLINE_FOLD_CHIP_SCALE
}

fn outline_fold_tri_gap() -> f32 {
    OUTLINE_FOLD_TRI_GAP * OUTLINE_FOLD_CHIP_SCALE
}

fn outline_fold_chip_inner_size(
    row_height: f32,
    count_galley: Option<&Arc<Galley>>,
) -> Vec2 {
    let tri_side = (row_height - 6.0).clamp(12.0, 20.0) * OUTLINE_FOLD_CHIP_SCALE;
    let pad = outline_fold_chip_pad();
    let mut w = tri_side + pad * 2.0;
    if let Some(g) = count_galley {
        w += outline_fold_tri_gap() + g.size().x + pad * 0.5;
    }
    let h = tri_side + pad * 1.5;
    vec2(w, h)
}

fn paint_outline_fold_triangle(
    ui: &Ui,
    center: Pos2,
    half: f32,
    collapsed: bool,
    fill: Color32,
) {
    let points: Vec<Pos2> = if collapsed {
        vec![
            Pos2::new(center.x - half * 0.5, center.y - half),
            Pos2::new(center.x - half * 0.5, center.y + half),
            Pos2::new(center.x + half * 0.72, center.y),
        ]
    } else {
        vec![
            Pos2::new(center.x - half, center.y - half * 0.5),
            Pos2::new(center.x + half, center.y - half * 0.5),
            Pos2::new(center.x, center.y + half * 0.72),
        ]
    };
    ui.painter()
        .add(Shape::convex_polygon(points, fill, Stroke::NONE));
}

fn paint_outline_fold_chip(
    ui: &Ui,
    chip_rect: Rect,
    response: &Response,
    collapsed: bool,
    enabled: bool,
    count_galley: Option<&Arc<Galley>>,
) {
    let visuals = ui.style().interact(response);
    let rounding = CornerRadius::same(OUTLINE_FOLD_CHIP_ROUNDING);
    let mut bg = visuals.bg_fill.gamma_multiply(0.92);
    if !enabled {
        bg = ui.visuals().faint_bg_color;
    }
    if collapsed {
        ui.painter().rect_filled(chip_rect, rounding, bg);
    }

    let pad = outline_fold_chip_pad();
    let tri_half = ((chip_rect.height() - pad) * 0.5).clamp(2.5, 6.0);
    let tri_center_x = chip_rect.left() + pad + tri_half;
    let tri_color = if enabled {
        if response.hovered() {
            CONTROL_HIGHLIGHT
        } else {
            ui.visuals().text_color()
        }
    } else {
        ui.visuals().weak_text_color()
    };
    paint_outline_fold_triangle(ui, Pos2::new(tri_center_x, chip_rect.center().y), tri_half, collapsed, tri_color);

    if let Some(galley) = count_galley {
        let count_x = chip_rect.left() + pad + tri_half * 2.0 + outline_fold_tri_gap();
        let count_pos = Pos2::new(
            count_x,
            chip_rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(count_pos, galley.clone(), Color32::WHITE);
    }
}

impl PghOutlineFold {
    pub fn new(collapsed: bool) -> Self {
        Self {
            collapsed,
            char_rect: None,
        }
    }

    /// 大纲折叠后，在标题行右侧空白区底部绘制分隔横线。
    #[allow(dead_code)]
    pub(crate) fn paint_collapsed_section_rule(ui: &Ui, rect: Rect) {
        if !ui.is_rect_visible(rect) {
            return;
        }
        let y = (rect.bottom() - rect.height()/2.0 - 0.5).round() + 0.5;
        let stroke = Stroke::new(0.5, ui.visuals().weak_text_color());
        ui.painter().line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            stroke,
        );
    }

    /// 返回 `(折叠状态是否变化, 是否处于折叠且可折叠, 布局响应)`。
    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
    ) -> (bool, bool, Response) {
        let foldable = ctx.outline_heading_has_foldable_content(line_no);
        let collapsed = ctx
            .get_line(line_no)
            .and_then(|p| p.outline_fold_collapsed())
            .unwrap_or(false);
        let row_height = ctx.font_heigh();
        let hidden_lines = ctx
            .outline_section_content_range(line_no)
            .map(|r| r.len())
            .unwrap_or(0);
        let count_label = if collapsed && foldable {
            outline_fold_hidden_count_label(hidden_lines)
        } else {
            None
        };
        let count_font_size = ctx.font_size() * 0.72 * OUTLINE_FOLD_CHIP_SCALE;
        let count_galley = count_label.as_ref().map(|text| {
            galley_builder(ui)
                .text(text.as_str())
                .font_size(count_font_size)
                .fg(ui.visuals().weak_text_color())
                .build()
        });
        let chip_inner = outline_fold_chip_inner_size(row_height, count_galley.as_ref());
        let trailing_gap = 4.0 * OUTLINE_FOLD_CHIP_SCALE;
        let alloc_w = chip_inner.x + trailing_gap;
        let sense = if foldable {
            Sense::CLICK
        } else {
            Sense::hover()
        };
        let id = ui.id().with(("outline_fold", line_no, segment));

        let left_gap = (segment != 0).then(|| {
            ui.allocate_exact_size(vec2(8.0, row_height), ctx.sense()).1
        });

        let slot = ui.allocate_ui_with_layout(
            vec2(alloc_w, row_height),
            Layout::left_to_right(Align::Center),
            |ui| {
                let (_, inner) = ui.allocate_exact_size(chip_inner, sense);
                inner
            },
        );
        let chip_rect = slot.inner.rect;
        let chip_response = ui.interact(chip_rect, id, sense);
        let mut response = slot.response | chip_response;
        if let Some(gap) = left_gap {
            response = gap | response;
        }

        if ui.is_rect_visible(chip_rect) {
            paint_outline_fold_chip(
                ui,
                chip_rect,
                &response,
                collapsed,
                foldable,
                count_galley.as_ref(),
            );
        }

        let mut changed = false;
        if foldable && response.clicked() {
            ctx.set_outline_folded(line_no, !collapsed);
            changed = true;
        }

        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );

        let pad = ui
            .allocate_exact_size(vec2(2.0 * OUTLINE_FOLD_CHIP_SCALE, row_height), ctx.sense())
            .1;
        let outline_fold_collapsed = collapsed && foldable;
        (changed, outline_fold_collapsed, response | pad)
    }
}

impl PghItem for PghOutlineFold {
    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        pos_from_cursor(&self.char_rect, cursor)
    }

    fn cursor_from_pos(&self, _line_no: usize, _segment: usize, _pos: &Pos2) -> Option<Cursor> {
        None
    }

    fn outline_fold_collapsed(&self) -> Option<bool> {
        Some(self.collapsed)
    }

    fn set_outline_fold_collapsed(&mut self, collapsed: bool) {
        self.collapsed = collapsed;
    }
}

#[derive(Clone)]
pub struct PghPoint {
    char_rect: Option<Vec<CharRect>>,
}

impl PghPoint {
    pub fn new() -> Self {
        PghPoint {
            char_rect: None,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
    ) -> Response {
        let row_height = ctx.font_heigh();
        let mut response = simple_allocate_rect(ui, ctx, row_height, row_height);
        let rect = response.rect;

        // 检查是否是有序列表
        let line_text = ctx.get_line_text(line_no);
        let trimmed = line_text.trim_start();
        let is_ordered_list = {
            let ordered_list_re = Regex::new(r"^(\d+)\.\s+").unwrap();
            ordered_list_re.is_match(trimmed)
        };

        if is_ordered_list {
            // 有序列表：提取编号并显示文本（如 "1."）
            let ordered_list_re = Regex::new(r"^(\d+)\.\s+").unwrap();
            if let Some(caps) = ordered_list_re.captures(trimmed) {
                if let Some(number_str) = caps.get(1) {
                    let number_text = format!("{}.", number_str.as_str());
                    let text_color = ui.visuals().weak_text_color();
                    let font_id = eframe::egui::FontId {
                        size: ctx.cfg().font_size,
                        family: ctx.cfg().font_family(),
                    };
                    let text_galley = ui.painter().layout(
                        number_text.clone(),
                        font_id.clone(),
                        text_color,
                        rect.width(),
                    );
                    let _text_width = text_galley.size().x;
                    let text_x = rect.left();
                    let text_y = rect.center().y - text_galley.size().y / 2.0;
                    
                    ui.painter().galley(
                        eframe::egui::pos2(text_x, text_y),
                        text_galley,
                        text_color,
                    );
                }
            }
        } else {
            // 无序列表：显示圆点
            ui.painter().circle_filled(
                rect.center(),
                rect.height() / 7.0,
                CONTROL_HIGHLIGHT,
            );
        }

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

    fn cursor_from_pos(&self, _line_no: usize, _segment: usize, _pos: &Pos2) -> Option<Cursor> {
        None
        //cursor_from_pos(&self.char_rect, line_no, segment, pos)
    }
}

#[derive(Clone)]
pub struct PghQuoteIndent {
    char_rect: Option<Vec<CharRect>>,
}

impl PghQuoteIndent {
    pub fn new() -> Self {
        PghQuoteIndent {
            char_rect: None,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
    ) -> Response {
        let row_height = ctx.font_heigh();
        let response = simple_allocate_rect(ui, ctx, QUOTE_INDENT_WIDTH+SPACE_X, row_height);
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

impl PghItem for PghQuoteIndent {
    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        pos_from_cursor(&self.char_rect, cursor)
    }

    fn cursor_from_pos(&self, _line_no: usize, _segment: usize, _pos: &Pos2) -> Option<Cursor> {
        None
        //cursor_from_pos(&self.char_rect, line_no, segment, pos)
    }
}

#[derive(Clone)]
pub struct PghBreak {
    char_rect: Option<Vec<CharRect>>,
}

impl PghBreak {
    pub fn new() -> Self {
        PghBreak {
            char_rect: None,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
    ) -> Response {
        let row_height = ctx.font_heigh();
        let cursor_rect = ui.cursor();
        let right = ctx.edit_right().min(ui.max_rect().right());
        let width = (right - cursor_rect.left()).max(0.0);
        let (rect, response) = ui.allocate_exact_size(vec2(width, row_height), ctx.sense());
        Self::paint_groove_line(ui, rect);

        //update rect info
        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );

        response
    }

    pub(crate) fn paint_groove_line(ui: &Ui, rect: Rect) {
        // 双细线凹槽：上暗下亮，模拟嵌入分隔线
        let visuals = ui.visuals();
        let weak = visuals.weak_text_color();
        let shadow = weak.gamma_multiply(0.52);
        let bg = visuals.panel_fill;
        let highlight = weak.lerp_to_gamma(bg, 0.42);
        let stroke = Stroke::new(0.5, shadow);
        let stroke_hi = Stroke::new(0.5, highlight);
        let painter = ui.painter();
        // 像素对齐的相邻双线，避免上下边缘绘制导致中间出现空隙。
        let center_y = rect.center().y.round() + 0.5;
        let top_y = center_y - 0.25;
        let bottom_y = center_y + 0.25;
        let left = Pos2::new(rect.left(), top_y);
        let right = Pos2::new(rect.right(), top_y);
        painter.line_segment([left, right], stroke);
        let left = Pos2::new(rect.left(), bottom_y);
        let right = Pos2::new(rect.right(), bottom_y);
        painter.line_segment([left, right], stroke_hi);
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
            for (_i, c_rect) in plist.into_iter().enumerate() {
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
    char_rect: Option<Vec<CharRect>>,
    icon_name: IconName,
    link_info: Option<LinkInfo>,
}

impl PghIcon {
    pub fn new(icon_name: IconName) -> Self {
        PghIcon {
            char_rect: None,
            icon_name,
            link_info: None,
        }
    }

    pub fn new_with_link(icon_name: IconName, link_info: LinkInfo) -> Self {
        PghIcon {
            char_rect: None,
            icon_name,
            link_info: Some(link_info),
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
    ) -> (Response, bool) {
        let _row_height = ctx.font_heigh();

        //get pos and font-size for diffrent icon_type
        let mut pos = ui.cursor().left_top();
        let mut font_size = ctx.font_size();
        let mut spacing_x = 2.0;
        let mut spacing_y = 0.0;
        let Some(icon_name) = ctx
            .get_line(line_no)
            .and_then(|p| p.pgh.get(segment))
            .and_then(|s| s.item.icon_name())
        else {
            let rect = ui.cursor();
            return (ui.allocate_rect(rect, Sense::hover()), false);
        };
        match icon_name {
            IconName::icon_external_link => {
                spacing_x = 4.0;
                spacing_y = 2.0;
                font_size = ctx.font_size() * 0.7;
            }
            _ => {}
        }
        let mut icon_size_val = icon_button_builder(ui)
            .icon(icon_name.clone())
            .font_size(font_size)
            .size();
        icon_size_val.x += spacing_x * 2.0;
        pos.x += spacing_x;
        pos.y += spacing_y;

        let id = format!("icon_{}_{}", line_no, segment);
        let (_rect, response) = ui.allocate_exact_size(icon_size_val, Sense::CLICK);
        let clicked = icon_button_builder(ui)
            .icon(icon_name)
            .pos(pos)
            .id(id)
            .font_size(font_size)
            .build_inner();
        
        //update rect info
        ctx.update_view(
            line_no,
            segment,
            response.rect,
            item_char_rect(&response.rect),
        );

        (response, clicked)
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

    fn icon_name(&self) -> Option<IconName> {
        Some(self.icon_name.clone())
    }

    fn link_info(&self) -> Option<LinkInfo> {
        self.link_info.clone()
    }
}


#[derive(Clone)]
pub struct PghImage {
    char_rect: Option<Vec<CharRect>>,
    image: ImageInfo,
}

impl PghImage {
    pub fn new(image: ImageInfo) -> Self {
        PghImage {
            char_rect: None,
            image,
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
        available_width: f32,
    ) -> Response {
        let Some(image) = ctx
            .get_line(line_no)
            .and_then(|p| p.pgh.get(segment))
            .and_then(|s| s.item.image_info())
        else {
            let rect = ui.cursor();
            return ui.allocate_rect(rect, Sense::hover());
        };
        let corner_radius = 10.0;
        
        let url = if image.url.starts_with("file://") || image.url.starts_with("http://") || image.url.starts_with("https://") {
            image.url.clone()
        } else {
            if let Some(image_path) = &ctx.cfg().image_path {
                format!("file://{}/{}", image_path, image.url)
            } else {
                image.url.clone()
            }
        };
        
        let response = ui.add(
            Image::new(&url)
                .fit_to_original_size(1.0)
                .max_width(available_width)
                .corner_radius(corner_radius),
        ).on_hover_cursor(CursorIcon::Default);
        
        // Use click_and_drag to support both left and right clicks
        let id = ui.id().with(format!("image_{}_{}", line_no, segment));
        let click_response = ui.interact(response.rect, id, Sense::click_and_drag());
        if click_response.clicked() || click_response.secondary_clicked() {
            if let Some((url_start, url_end)) = image.url_range {
                if let Some(pgh_view) = ctx.get_line(line_no) {
                    let url_start_cursor = pgh_view.text_char_index_to_cursor(url_start, line_no);
                    let url_end_cursor = pgh_view.text_char_index_to_cursor(url_end, line_no);
                    ctx.set_cursor1(url_start_cursor);
                    ctx.set_cursor2(url_end_cursor);
                }
            }
        }
                
        //update zero-rect, because the image cannot has the cursor
        let mut zero_rect = response.rect;
        zero_rect.set_width(0.0);
        ctx.update_view(
            line_no,
            segment,
            zero_rect, 
            item_char_rect(&zero_rect),
        );

        // Check if image url_range is selected and draw selection border
        let mut is_selected = false;
        if let Some((url_start, url_end)) = image.url_range {
            if ctx.is_selected() {
                if let Some(pgh_view) = ctx.get_line(line_no) {
                    let cursor1 = ctx.cursor1();
                    let cursor2 = ctx.cursor2();
                    let url_start_cursor = pgh_view.text_char_index_to_cursor(url_start, line_no);
                    let url_end_cursor = pgh_view.text_char_index_to_cursor(url_end, line_no);
                    let sel_min = std::cmp::min(cursor1, cursor2);
                    let sel_max = std::cmp::max(cursor1, cursor2);
                    is_selected = sel_min <= url_start_cursor && url_end_cursor <= sel_max;
                }
            }
        };
        
        // Draw selection border if url is selected
        if is_selected {
            let stroke = Stroke::new(2.0, CONTROL_HIGHLIGHT);
            ui.painter().rect_stroke(response.rect, corner_radius, stroke, StrokeKind::Inside);
        }

        click_response
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

