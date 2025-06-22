use crate::medit::{CharRect, Ctx, Cursor, PghItem, IconName};
use core::f32;
use eframe::egui::epaint::text::{FontFamily, TextFormat, LayoutJob};
use eframe::egui::{
    epaint, Align, Color32, FontSelection, Galley, NumExt, Pos2, Rect, Response, Stroke, StrokeKind, Ui, Vec2
};
use std::sync::Arc;

#[derive(Clone)]
pub struct PghText {
    text: String,
    char_rect: Option<Vec<CharRect>>,
    job: Option<LayoutJob>,
}

impl PghText {
    pub fn new(text: String, job: Option<LayoutJob>) -> Self {
        Self {
            text,
            char_rect: None,
            job,
        }
    }

    pub fn text_galley(ui: &Ui, text: String, fg: Color32, wrap_width: f32) -> Arc<Galley> {
        let font_id = FontSelection::Default.resolve(ui.style());
        let layout_job = LayoutJob::simple(text, font_id.clone(), fg, wrap_width);
        ui.fonts(|f| f.layout_job(layout_job))
    }

    pub fn icon_galley(ui: &Ui, icon_name: IconName, bg: Color32, fg: Color32) -> Arc<Galley> {
        let mut layout_job: LayoutJob = LayoutJob::default();
        let mut format = TextFormat::default();
        format.font_id.size = FontSelection::Default.resolve(ui.style()).size;
        format.font_id.family = FontFamily::Name("icon".into());
        format.background = bg;
        format.color = fg;
        layout_job.append(&icon_name.to_char().to_string(), 0.0, format);
        ui.fonts(|f| f.layout_job(layout_job))
    }

    pub fn layout_text(
        ui: &mut Ui,
        outer_rect: Rect,
        text: String,
        layout_job: &Option<LayoutJob>,
        pos: Pos2,
        fg: Color32,
        bg: Option<Color32>,
        wrap_width: f32,
    ) -> (Arc<Galley>, Rect) {
        let galley = if let Some(mut job) = layout_job.clone() {
            job.wrap.max_width = wrap_width;
            ui.fonts(|f| f.layout_job(job.clone()))
        } else {
            Self::text_galley(ui, text.clone(), fg, wrap_width)
        };

        let galley_rect = Rect::from_min_size(pos, galley.size());

        if ui.is_rect_visible(galley_rect) {
            //gb
            if let Some(bg) = bg {
                ui.painter_at(outer_rect).rect_filled(galley_rect, 0.0, bg);
            }
            //text
            ui.painter_at(outer_rect).add(epaint::TextShape::new(
                galley_rect.left_top(),
                galley.clone(),
                fg,
            ));
        }

        (galley, galley_rect)
    }

    fn layout_get_char_rect(
        outer: Rect,
        spacing_top: f32,
        spacing_bottom: f32,
        galley: Arc<Galley>,
        need_expand: bool,
    ) -> (Vec<CharRect>, Rect) {
        let mut end_rect = outer;
        let mut char_rect_list = vec![];
        let mut next_ch_i = 0;
        let mut pgh_rect = Rect::from_min_max(outer.left_top(), outer.left_top());

        let rnum = galley.rows.len();
        for (i, r) in galley.rows.iter().enumerate() {
            let off_top = if i == 0 { spacing_top } else { 0.0 };
            let off_bottom = if i + 1 == rnum { spacing_bottom } else { 0.0 };
            let mut max_height = 0.0;

            end_rect = r.rect.translate(outer.left_top().to_vec2());
            end_rect.min.y -= spacing_top;
            end_rect.max.y += spacing_bottom;

            for gl in &r.glyphs {
                let min = Pos2 {
                    x: outer.min.x + gl.pos.x,
                    y: outer.min.y + r.rect.min.y - off_top,
                };
                let max = Pos2 {
                    x: outer.min.x + gl.pos.x + gl.advance_width, //gl.uv_rect.size.x,
                    y: outer.min.y + r.rect.min.y + gl.line_height + off_bottom,
                };
                let rect = Rect::from_min_max(min, max);
                //log::debug!("{} {:?} rect-height:{} top:{} bottom:{}", gl.chr, rect, rect.height(), off_top, off_bottom);
                char_rect_list.push(CharRect::new(
                    rect,
                    next_ch_i,
                    gl.chr,
                    off_top,
                    off_bottom,
                ));
                next_ch_i += 1;

                max_height = max_height.at_least(rect.height());
                end_rect = Rect::from_min_size(rect.right_top(), Vec2{x:0.0, y:max_height});
                pgh_rect = pgh_rect.union(rect);
            }

            //end pos for last row
            if need_expand {
                end_rect.set_right(outer.max.x);
            }
            char_rect_list.push(CharRect::new(
                end_rect, next_ch_i, '\0', off_top, off_bottom,
            ));
            pgh_rect = pgh_rect.union(end_rect);
        }

        (char_rect_list, pgh_rect)
    }


    pub fn text_layout_in_ui(
        ctx: &mut Ctx, 
        ui: &mut Ui, 
        text: String,
        job: &Option<LayoutJob>,
        outer: Rect,
        wrap_width: f32,
        need_expand: bool,
        once_allocate: bool,
    ) -> (Pos2, Arc<Galley>, Response) {
        
        let cursor = ui.cursor();
        let pos = Pos2{x: outer.left(), y: cursor.top()};
        let first_row_indentation = (cursor.left() - outer.left()).at_least(0.0);

        let mut layout_job = 
            if let Some(layout_job) = job {
                layout_job.clone()
            } else {
                let text_color = ctx.cfg().text_color();
                let font_id = FontSelection::Default.resolve(ui.style());
                LayoutJob::simple(text, font_id.clone(), text_color, wrap_width)
            };
        
        layout_job.wrap.max_width = wrap_width;
        layout_job.wrap.break_anywhere = true;
        layout_job.first_row_min_height = cursor.height();
        layout_job.halign = Align::Min;
        layout_job.justify = false;
        
        if let Some(first_section) = layout_job.sections.first_mut() {
            first_section.leading_space = first_row_indentation;
        }
        let galley = ui.fonts(|fonts| fonts.layout_job(layout_job));

        
        let mut rsp_rect = Rect::from_min_max(outer.left_top(), outer.left_top());
        let mut row_rects = vec![];
        let row_count = galley.rows.len();
        for (i, row) in galley.rows.iter().enumerate() {
            let mut rect = row.rect.translate(pos.to_vec2());
            if need_expand || (wrap_width.is_finite() && i+1 < row_count) {
                rect.set_right(outer.right());
            }
            row_rects.push(rect);
            rsp_rect = rsp_rect.union(rect);
        }

        //need onlly allocate only one time in table
        if once_allocate {
            let response = ui.allocate_rect(rsp_rect, ctx.sense());
            (pos, galley, response)
        } else {
            let mut response = ui.allocate_rect(Rect::from_min_max(outer.left_top(), outer.left_top()), ctx.sense());
            for rect in row_rects {
                response |= ui.allocate_rect(rect, ctx.sense());
            }
            (pos, galley, response)
        }
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
        outer: Rect,
        warp_width: f32,
        spacing_top: f32,
        spacing_bottom: f32,
        need_expand: bool,
        once_allocate: bool,
        text: String,
        layout_job: &Option<LayoutJob>,
    ) -> Response {
        //layout galley
        let (pos, galley, response) 
            = Self::text_layout_in_ui(ctx, ui, text.clone(), layout_job, outer, warp_width, need_expand, once_allocate);
        
        //paint text
        ui.painter_at(ctx.edit_rect()).add(epaint::TextShape::new(
            pos,
            galley.clone(),
            ctx.cfg().text_color(),
        ));

        let rect = Rect::from_min_size(pos, outer.size());
        let (char_rect_list, pgh_rect) = Self::layout_get_char_rect(
            rect,
            spacing_top,
            spacing_bottom,
            galley,
            need_expand,
        );
        ctx.update_view(line_no, segment, pgh_rect, char_rect_list);

        response
    }

    pub fn guess_text_rect(ui: &Ui, ctx: &Ctx, text: String, wrap_width: f32) -> Rect {
        Self::text_galley(ui, text, ctx.cfg().text_color(), wrap_width).rect
    }

    fn get_cursors_range(
        &self,
        line_no: usize,
        segment: usize,
        c1: &Cursor,
        c2: &Cursor 
    ) -> (Cursor, Cursor) {
        let min = std::cmp::min(c1, c2);
        let max = std::cmp::max(c1, c2);

        let del_min;
        let del_max;
        if line_no == min.line_no && line_no == max.line_no {
            //same line
            del_min = min.clone();
            del_max = max.clone();
        } else if line_no == min.line_no {
            //first line
            del_min = min.clone();
            del_max = (line_no, segment, self.max_culumn()).into();
        } else if line_no == max.line_no {
            //last line
            del_min = line_no.into();
            del_max = max.clone();
        } else {
            //middle line
            del_min = line_no.into();
            del_max = (line_no, segment, self.max_culumn()).into();
        }

        (del_min, del_max)
    }

    // NOTICE: 
    // keep_pos=true return empty string but not None when segment not selected
    // keep_pos=false return None when segment not selected
    fn get_select(
        &self,
        line_no: usize,
        segment: usize,
        c1: &Cursor,
        c2: &Cursor,
        keep_pos: bool
    ) -> Option<String> {
        let (del_min, del_max) = self.get_cursors_range(line_no, segment, c1, c2);
        if line_no < del_min.line_no || line_no > del_max.line_no {
            return None;
        }

        if !keep_pos && (segment < del_min.segment || segment > del_max.segment) {
            return None;
        }

        let after = self
            .text
            .chars()
            .enumerate()
            .filter_map(|(i, chr)| {
                let c_i: Cursor = (line_no, segment, i).into();
                if c_i >= del_min && c_i < del_max {
                    Some(chr)
                } else {
                    None
                }
            })
            .collect::<String>();

        return Some(after);
    }

    // NOTICE: return empty string but not None when segment has delete
    fn get_delete(
        &self,
        line_no: usize,
        segment: usize,
        c1: &Cursor,
        c2: &Cursor
    ) -> Option<String> {
        let (del_min, del_max) = self.get_cursors_range(line_no, segment, c1, c2);
        if line_no < del_min.line_no || line_no > del_max.line_no {
            return None;
        }
        
        let after = self
            .text
            .chars()
            .enumerate()
            .filter_map(|(i, chr)| {
                let c_i: Cursor = (line_no, segment, i).into();
                if c_i >= del_min && c_i < del_max {
                    None
                } else {
                    Some(chr)
                }
            })
            .collect::<String>();

        Some(after)
    }
}


impl PghItem for PghText {
    fn text(&self) -> String {
        self.text.clone()
    }

    fn layout_job(&self) -> Option<LayoutJob> {
        self.job.clone()
    }

    fn layout_job_update(&mut self, job: Option<LayoutJob>) {
        self.job = job;
    }

    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {
        self.char_rect = Some(char_rect);
    }

    fn cursor_from_pos(&self, line_no: usize, segment: usize, pos: &Pos2) -> Option<Cursor> {
        if let Some(plist) = &self.char_rect {
            for (i, c_rect) in plist.into_iter().enumerate() {
                let rect = c_rect.rect;
                let middle = if c_rect.c == '\0' {
                    rect.min.x + rect.width()
                } else {
                    rect.min.x + rect.width() / 2.0
                };
                if middle >= pos.x && rect.min.y <= pos.y && rect.max.y >= pos.y {
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

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        if let Some(plist) = &self.char_rect {
            for c_rect in plist {
                if c_rect.i == cursor.culumn {
                    let mut zero_width_rect = c_rect.rect;
                    zero_width_rect.set_width(0.0);
                    zero_width_rect.min.y += c_rect.top;
                    zero_width_rect.max.y -= c_rect.bottom;
                    return Some(zero_width_rect);
                }
            }
        }
        None
    }

    fn delete(&self, line_no: usize, segment: usize, c1: &Cursor, c2: &Cursor) -> Option<String> {
        self.get_delete(line_no, segment, c1, c2)
    }

    fn select(&self, line_no: usize, segment: usize, c1: &Cursor, c2: &Cursor, keep_pos: bool) -> Option<String> {
        self.get_select(line_no, segment, c1, c2, keep_pos)
    }

    //return (left, right)
    fn insert(&self, c: &Cursor) -> (String, String) {
        let left = self
            .text
            .chars()
            .enumerate()
            .filter_map(|(i, chr)| if i < c.culumn { Some(chr) } else { None })
            .collect::<String>();

        let right = self
            .text
            .chars()
            .enumerate()
            .filter_map(|(i, chr)| if i >= c.culumn { Some(chr) } else { None })
            .collect::<String>();

        (left, right)
    }

    fn enter(&self, c: &Cursor) -> (String, String) {
        let left = self
            .text
            .chars()
            .enumerate()
            .filter_map(|(i, chr)| if i < c.culumn { Some(chr) } else { None })
            .collect::<String>();

        let right = self
            .text
            .chars()
            .enumerate()
            .filter_map(|(i, chr)| if i >= c.culumn { Some(chr) } else { None })
            .collect::<String>();

        (left, right)
    }

    fn update_text(&mut self, new: String) {
        self.text = new;
    }

    fn max_culumn(&self) -> usize {
        return self.text.chars().count();
    }
}

impl PghText {
    //replace tab to space
    fn view_text(&self) -> String {
        let left = self
            .text
            .chars()
            .enumerate()
            .filter_map(|(i, chr)| Some("".to_string()))
            .collect::<String>();

        left
    }
}
