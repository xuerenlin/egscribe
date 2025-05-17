use crate::medit::{CharRect, Ctx, Cursor, PghItem, IconName};
use core::f32;
use eframe::egui::epaint::text::{FontFamily, TextFormat, LayoutJob};
use eframe::egui::{
    epaint, Color32, FontSelection, Galley, NumExt, Pos2, Rect, Response, Ui,
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
        pgh_rect: Rect,
        spacing_top: f32,
        spacing_bottom: f32,
        galley: Arc<Galley>,
        expand: bool,
    ) -> Vec<CharRect> {
        let mut end_rect = pgh_rect;
        let mut char_rect = vec![];
        let mut next_ch_i = 0;

        end_rect.min.y -= spacing_top;
        end_rect.max.y += spacing_bottom;

        let rnum = galley.rows.len();
        for (i, r) in galley.rows.iter().enumerate() {
            let off_top = if i == 0 { spacing_top } else { 0.0 };
            let off_bottom = if i + 1 == rnum { spacing_bottom } else { 0.0 };

            for gl in &r.glyphs {
                let min = Pos2 {
                    x: pgh_rect.min.x + gl.pos.x,
                    y: pgh_rect.min.y + r.rect.min.y - off_top,
                };
                let max = Pos2 {
                    x: pgh_rect.min.x + gl.pos.x + gl.size.x,
                    y: pgh_rect.min.y + r.rect.max.y + off_bottom,
                };
                char_rect.push(CharRect::new(
                    Rect::from_min_max(min, max),
                    next_ch_i,
                    gl.chr,
                    off_top,
                    off_bottom,
                ));
                next_ch_i += 1;

                end_rect = Rect::from_min_max(Pos2 { x: max.x, y: min.y }, max);
            }

            //end pos for last row
            if expand {
                end_rect.set_right(pgh_rect.max.x);
            }

            char_rect.push(CharRect::new(
                end_rect, next_ch_i, '\0', off_top, off_bottom,
            ));
        }

        char_rect
    }

    pub fn layout_paragraph(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        segment: usize,
        warp_width: f32,
        spacing_top: f32,
        spacing_bottom: f32,
        need_expand_x: bool,
        text: String,
        layout_job: &Option<LayoutJob>,
    ) -> Response {
        let pos = ui.cursor().left_top();
        let text_color = ctx.cfg().text_color();
        let outer_rect = ctx.edit_rect();

        let (galley, pgh_rect) = Self::layout_text(
            ui,
            outer_rect,
            text.clone(),
            layout_job,
            pos,
            text_color,
            None,
            warp_width,
        );

        //expand rect
        let mut expand_rect_x = pgh_rect;
        //Add 8.0, Ensure that clicking on the right side of the last character can locate the cursor
        if expand_rect_x.right() < ctx.edit_right() {
            expand_rect_x.set_right((pos.x + warp_width + 8.0).at_most(ctx.edit_right()));
        }

        let char_rect = Self::layout_get_char_rect(
            expand_rect_x,
            spacing_top,
            spacing_bottom,
            galley,
            need_expand_x,
        );

        let response = if need_expand_x {
            let mut expand_rect_xy = expand_rect_x;
            expand_rect_xy.min.y -= spacing_top;
            expand_rect_xy.max.y += spacing_bottom;
            ctx.update_view(line_no, segment, expand_rect_xy, char_rect);
            ui.allocate_rect(expand_rect_x, ctx.sense())
        } else {
            let mut expand_rect_y = pgh_rect;
            expand_rect_y.min.y -= spacing_top;
            expand_rect_y.min.y += spacing_bottom;
            ctx.update_view(line_no, segment, expand_rect_y, char_rect);
            ui.allocate_rect(pgh_rect, ctx.sense())
        };

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
            //println!("{:?} - {:?}", del_min, del_max);
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

        //println!("after s: {}", after);
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

        //println!("after s: {}", after);
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
                //println!("pos:{} i:{} c_rect:{:?} middle:{}", pos, i, c_rect, middle);
                if middle >= pos.x && rect.min.y <= pos.y && rect.max.y >= pos.y {
                    //println!("from_pos {}->{:?}", pos, self.cursor);
                    return Some(Cursor {
                        line_no,
                        segment,
                        culumn: c_rect.i,
                        //culmax: plist.len()-1
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
                    //println!("from_cursor {:?} -> {}", self.cursor, c_rect.rect);
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
