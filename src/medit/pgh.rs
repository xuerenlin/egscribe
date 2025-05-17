use core::f32;

use dyn_clone::DynClone;
use eframe::egui::epaint::text::{LayoutJob, TextFormat};
use eframe::egui::{
    vec2, FontFamily, FontId, Grid, NumExt, Pos2, Rect, Response, Stroke, Ui, Vec2
};

use crate::sitter::{LightSlice, highlight_lines, support_lang};
use crate::medit::{icon, Cursor, Ctx, DoCmd, MarkDownImpl, PghCheckBox, PghText};
use super::items::{PghBreak, PghHead, PghIcon, PghImage, PghIndent, PghPoint, PghQuoteIndent};
use super::image::{ImageInfo};
use super::IconName;

#[derive(Clone, Debug)]
pub struct CharRect {
    pub rect: Rect,
    pub i: usize,
    pub c: char,
    pub top: f32,
    pub bottom: f32,
}

impl CharRect {
    pub fn new(rect: Rect, i: usize, c: char, top: f32, bottom: f32) -> Self {
        CharRect {
            rect,
            i,
            c,
            top,
            bottom,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum SegmentType {
    Text,
    Head,
    Indent,
    CheckBox,
    Point,
    QuoteIndent,
    Break,
    Icon,
    Image,
}

pub trait PghItem: DynClone {
    fn text(&self) -> String {
        "".to_string()
    }

    fn layout_job(&self) -> Option<LayoutJob> {
        None
    }

    fn layout_job_update(&mut self, job: Option<LayoutJob>) {}

    fn update_view_info(&mut self, char_rect: Vec<CharRect>) {}

    fn cursor_from_pos(&self, line_no: usize, segment: usize, pos: &Pos2) -> Option<Cursor> {
        None
    }

    fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        None
    }

    fn delete(&self, line_no: usize, segment: usize, c1: &Cursor, c2: &Cursor) -> Option<String> {
        Some("".to_string())
    }

    fn select(&self, line_no: usize, segment: usize, c1: &Cursor, c2: &Cursor, keep_pos: bool) -> Option<String> {
        Some("".to_string())
    }

    fn insert(&self, c: &Cursor) -> (String, String) {
        ("".to_string(), "".to_string())
    }

    fn enter(&self, c: &Cursor) -> (String, String) {
        ("".to_string(), "".to_string())
    }

    fn update_text(&mut self, text: String) {}

    fn max_culumn(&self) -> usize {
        1
    }

    fn icon_name(&self) -> Option<icon::IconName> {
        None
    }

    fn image_info(&self) -> Option<ImageInfo> {
        None
    }
}

impl Clone for Box<dyn PghItem> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

impl std::fmt::Debug for Box<dyn PghItem> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.text())
    }
}

#[derive(Clone, Debug)]
pub struct PghSegment {
    pub seg_type: SegmentType,
    pub item: Box<dyn PghItem>,
    pub rect: Option<Rect>,
}

impl PghSegment {
    pub fn new(seg_type: SegmentType, item: Box<dyn PghItem>) -> Self {
        PghSegment {
            seg_type,
            item,
            rect: None,
        }
    }

    pub fn is_pos_in(&self, pos: &Pos2) -> bool {
        if let Some(rect) = self.rect {
            if pos.x >= rect.left_top().x
                && pos.x <= rect.right_bottom().x
                && pos.y >= rect.left_top().y
                && pos.y <= rect.right_bottom().y
            {
                return true;
            }
        }
        false
    }
}

#[derive(Clone, Debug)]
pub struct TableCell {
    pub row: usize,
    pub col: usize,
    pub segment: usize,
}

#[derive(Clone, Debug)]
pub struct TableInfo {
    pub row_count: usize,
    pub col_count: usize,
    pub spacing_x: f32,
    pub spacing_y: f32,
    pub spacing_indent: f32,
    pub col_min_width: f32,
    pub has_frame: bool,
}

impl Default for TableInfo {
    fn default() -> Self {
        TableInfo {
            row_count: 0,
            col_count: 0,
            spacing_x: 12.0,
            spacing_y: 12.0,
            spacing_indent: 16.0,
            col_min_width: 64.0,
            has_frame: true,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum PghType {
    Text,
    Heading,
    BreakLine,
    BlockLine,
    ListItem,
    Table,
    Code,
    UnKnown,
}

#[derive(Clone, Debug)]
pub struct PghView {
    pub pgh_type: PghType,
    pub pgh: Vec<PghSegment>,
    pub rect: Option<Rect>,
    pub spacing_top: f32,
    pub spacing_bottom: f32,
    pub table_info: Option<TableInfo>,
    pub code_lang: Option<String>,
    pub change_tick: usize
}

impl PghView {
    pub fn new(pgh_type: PghType) -> Self {
        Self {
            pgh_type,
            pgh: vec![],
            rect: None,
            spacing_top: 0.0,
            spacing_bottom: 0.0,
            table_info: None,
            code_lang: None,
            change_tick: 1,
        }
    }

    pub fn new_text() -> Self {
        PghView::new(PghType::Text)
    }

    pub fn new_heading() -> Self {
        PghView::new(PghType::Heading)
    }

    pub fn new_list_item() -> Self {
        PghView::new(PghType::ListItem)
    }

    pub fn new_break_line() -> Self {
        PghView::new(PghType::BreakLine)
    }

    pub fn new_table() -> Self {
        PghView::new(PghType::Table)
    }

    pub fn new_code() -> Self {
        PghView::new(PghType::Code)
    }

    pub fn new_block_line() -> Self {
        PghView::new(PghType::BlockLine)
    }

    pub fn push(&mut self, segment: PghSegment) {
        self.pgh.push(segment);
    }

    pub fn push_text(&mut self, s: String, job: Option<LayoutJob>) {
        self.pgh.push(PghSegment::new(
            SegmentType::Text,
            Box::new(PghText::new(s, job)),
        ));
    }

    pub fn insert_text(&mut self, i: usize, s: String, job: Option<LayoutJob>) {
        self.pgh.insert(
            i,
            PghSegment::new(SegmentType::Text, Box::new(PghText::new(s, job))),
        );
    }

    pub fn update_text(&mut self, i: usize, s: String, job: Option<LayoutJob>) {
        if let Some(seg) = self.pgh.get_mut(i) {
            seg.item = Box::new(PghText::new(s, job));
        }
    }

    pub fn push_head(&mut self, deep: u8) {
        self.pgh
            .push(PghSegment::new(SegmentType::Head, Box::new(PghHead::new(deep))));
    }

    pub fn push_indent(&mut self) {
        self.pgh
            .push(PghSegment::new(SegmentType::Indent, Box::new(PghIndent::new())));
    }

    pub fn push_checkbox(&mut self) {
        self.pgh.push(PghSegment::new(
            SegmentType::CheckBox,
            Box::new(PghCheckBox::new()),
        ));
    }

    pub fn push_point(&mut self) {
        self.pgh
            .push(PghSegment::new(SegmentType::Point, Box::new(PghPoint::new())));
    }

    pub fn push_quote_indent(&mut self) {
        self.pgh.push(PghSegment::new(
            SegmentType::QuoteIndent,
            Box::new(PghQuoteIndent::new()),
        ));
    }

    pub fn push_break(&mut self) {
        self.pgh
            .push(PghSegment::new(SegmentType::Break, Box::new(PghBreak::new())));
    }

    pub fn push_icon(&mut self, icon_name: icon::IconName) {
        self.pgh
            .push(PghSegment::new(SegmentType::Icon, Box::new(PghIcon::new(icon_name))));
    }

    pub fn push_image(&mut self, image_info: ImageInfo) {
        self.pgh
            .push(PghSegment::new(SegmentType::Image, Box::new(PghImage::new(image_info))));
    }

    pub fn is_pos_in(&self, pos: &Pos2) -> bool {
        if let Some(rect) = self.rect {
            if pos.x >= rect.left_top().x
                && pos.x <= rect.right_bottom().x
                && pos.y >= rect.left_top().y
                && pos.y <= rect.right_bottom().y
            {
                return true;
            }
        }
        false
    }

    pub fn rect(&self) -> Option<Rect> {
        self.rect
    }

    pub fn update_view_info(&mut self, segment: usize, rect: Rect, char_rect: Vec<CharRect>) {
        if let Some(pgh_segment) = self.pgh.get_mut(segment) {
            //segment rect info:
            pgh_segment.rect = Some(rect);
            pgh_segment.item.update_view_info(char_rect.clone());

            //merge rect
            if None != self.rect {
                let mut new_rect = rect;
                for sub_segment in &self.pgh {
                    if let Some(org) = sub_segment.rect {
                        let min_x = if org.min.x < new_rect.min.x {
                            org.min.x
                        } else {
                            new_rect.min.x
                        };
                        let min_y = if org.min.y < new_rect.min.y {
                            org.min.y
                        } else {
                            new_rect.min.y
                        };
                        let max_x = if org.max.x > new_rect.max.x {
                            org.max.x
                        } else {
                            new_rect.max.x
                        };
                        let max_y = if org.max.y > new_rect.max.y {
                            org.max.y
                        } else {
                            new_rect.max.y
                        };

                        new_rect = Rect::from_min_max(
                            Pos2 { x: min_x, y: min_y },
                            Pos2 { x: max_x, y: max_y },
                        );
                    }
                }
                self.rect = Some(new_rect);
            } else {
                self.rect = Some(rect);
            }
        }
    }

    pub fn cursor_from_pos(&self, line_no: usize, pos: &Pos2) -> Option<Cursor> {
        for (i, segment) in self.pgh.iter().enumerate() {
            if !segment.is_pos_in(pos) {
                continue;
            }
            if let Some(c) = segment.item.cursor_from_pos(line_no, i, pos) {
                return Some(c);
            }
        }
        None
    }

    pub fn pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        if let Some(segment) = self.pgh.get(cursor.segment) {
            if let Some(rect) = segment.item.pos_from_cursor(cursor) {
                return Some(rect);
            }
        }
        None
    }

    pub fn max_culumn(&self, cursor: &Cursor) -> usize {
        if let Some(segment) = self.pgh.get(cursor.segment) {
            segment.item.max_culumn()
        } else {
            0
        }
    }

    pub fn max_segment(&self) -> usize {
        if self.pgh.len() > 0 {
            self.pgh.len() - 1
        } else {
            0
        }
    }

    fn is_segment_in_table_select(&self, segment: usize, c1: &Cursor, c2: &Cursor) -> bool {
        if let Some((left_top, right_bottom)) = self.table_range_to_cells(c1.segment, c2.segment) {
            if let Some(cell) = self.table_segment_to_cell(segment) {
                if cell.row >= left_top.row
                    && cell.row <= right_bottom.row
                    && cell.col >= left_top.col
                    && cell.col <= right_bottom.col
                {
                    return true;
                }
            }
        }
        false
    }

    //return all segment text
    pub fn delete(&self, line_no: usize, c1: &Cursor, c2: &Cursor) -> Vec<String> {
        let mut texts = vec![];
        for (i, segment) in self.pgh.iter().enumerate() {
            if c1.line_no == c2.line_no && self.is_table() {
                //in table, select by col-mode
                if self.is_segment_in_table_select(i, c1, c2) {
                    if let Some(s) = segment.item.delete(line_no, i, c1, c2) {
                        texts.push(s);
                    }
                } else {
                    texts.push(segment.item.text());
                }
            } else {
                //out table, select by row-mode
                if let Some(s) = segment.item.delete(line_no, i, c1, c2) {
                    texts.push(s);
                }
            }
        }

        // delete empty segment in range
        if self.is_code() && c1.line_no == c2.line_no && line_no == c1.line_no {
            let min = std::cmp::min(c1, c2);
            let max = std::cmp::max(c1, c2);
            let mut new_texts:Vec<String> = texts.iter().enumerate().filter_map(|(i,text)|{
                if text.is_empty() && i > min.segment && i < max.segment { // delete middle lines
                    None
                } else {
                    Some(text.clone())
                }
            }).collect();

            //merge remain 2 lines
            if max.segment != min.segment && min.segment+1 < new_texts.len() {
                let last = new_texts.remove(min.segment+1);
                if let Some(front) = new_texts.get_mut(min.segment) {
                    *front += &last;
                }
            }   
            return new_texts;
        } else {
            return texts;
        }
    }

    pub fn text_to_vec(&self) -> Vec<String> {
        let mut texts = vec![];
        for segment in &self.pgh {
            texts.push(segment.item.text());
        }
        texts
    }

    //return all segment text
    pub fn select_to_vec(&self, line_no: usize, c1: &Cursor, c2: &Cursor, keep_pos: bool) -> Vec<String> {
        let mut texts = vec![];
        for (i, segment) in self.pgh.iter().enumerate() {
            if let Some(s) = segment.item.select(line_no, i, c1, c2, keep_pos) {
                texts.push(s);
            }
        }
        texts
    }

    //return text that join all segment text
    pub fn select(&self, line_no: usize, c1: &Cursor, c2: &Cursor, is_raw: bool) -> String {
        if is_raw {
            return self.select_to_vec(line_no, c1, c2, false).join("");
        }
        
        if let Some(table_info) = &self.table_info {
            let arr = self.select_to_vec(line_no, c1, c2, true);
            let min = std::cmp::min(c1, c2);
            let max = std::cmp::max(c1, c2);
            let segmax = table_info.row_count * table_info.col_count - 1;

            let range = if line_no < min.line_no || line_no > max.line_no {
                (0, 0)
            } else if line_no == min.line_no && line_no == max.line_no {
                //same line
                (min.segment, max.segment)
            } else if line_no > min.line_no && line_no < max.line_no {
                //middle line
                (0, table_info.row_count * table_info.col_count - 1)
            } else if line_no == max.line_no {
                //last line
                let end = (max.segment + table_info.col_count) / table_info.col_count
                    * table_info.col_count;
                (0, end - 1)
            } else if line_no == min.line_no {
                //first line
                let start = min.segment - (min.segment % table_info.col_count);
                (start, segmax)
            } else {
                (0, 0)
            };

            let mut joins = "".to_string();
            if let Some((c1, c2)) = self.table_range_to_cells(range.0, range.1) {
                for (i, row) in (c1.row..=c2.row).enumerate() {
                    joins += "|";
                    for col in c1.col..=c2.col {
                        joins += arr
                            .get(row * table_info.col_count + col)
                            .unwrap_or(&"".to_string());
                        joins += "|";
                    }
                    joins += "\n";

                    if i == 0 {
                        joins += "|";
                        for col in c1.col..=c2.col {
                            joins += "--|";
                        }
                        joins += "\n";
                    }
                }
            }
            joins
        } else if self.is_code() {
            self.select_to_vec(line_no, c1, c2, false).join("\n")
        } else {
            self.select_to_vec(line_no, c1, c2, false).join("")
        }
    }

    //return the (left, right, segment)
    pub fn insert(&self, c: &Cursor, s: &str) -> (String, String, String) {
        let mut left = "".to_string();
        let mut right = "".to_string();
        let mut this = "".to_string();
        for (i, segment) in self.pgh.iter().enumerate() {
            if i < c.segment {
                left += &segment.item.text();
            } else if i == c.segment {
                let (ls, rs) = segment.item.insert(c);
                left += &ls;
                right += &rs;
                this = ls + s + &rs;
            } else {
                right += &segment.item.text();
            }
        }
        (left, right, this)
    }

    //return left and right texts
    pub fn normal_enter(&self, c: &Cursor) -> (String, String) {
        if let Some(seg) = self.pgh.get(c.segment) {
            let (left, right) = seg.item.enter(c);

            //join left segment text
            let mut left_s = "".to_string();
            for seg in 0..c.segment {
                left_s += &self.get_segment_text(seg);
            }
            left_s += &left;

            //join right segment text
            let mut right_s = right;
            for seg in (c.segment + 1)..self.pgh.len() {
                right_s += &self.get_segment_text(seg);
            }

            (left_s, right_s)
        } else {
            (self.get_text(), "".to_string())
        }
    }

    pub fn update_segment_text(&mut self, segment: usize, new: String) {
        if let Some(seg) = self.pgh.get_mut(segment) {
            seg.item.update_text(new)
        }
    }

    //todo, markdown genarate
    pub fn update_all_text(&mut self, new: String) {
        self.pgh = vec![];
        self.push_text(new, None); //todo
    }

    pub fn remove_segment_from(&mut self, segment: usize) {
        while self.pgh.len() > segment {
            self.pgh.remove(segment);
        }
    }

    pub fn get_segment_text(&self, segment: usize) -> String {
        if let Some(seg) = self.pgh.get(segment) {
            seg.item.text()
        } else {
            "".to_string()
        }
    }

    pub fn get_text(&self) -> String {
        let mut rs: String = Default::default();
        for segment in &self.pgh {
            let s = segment.item.text();
            rs += &s;
        }
        rs
    }

    pub fn get_segment_type(&self, segment: usize) -> SegmentType {
        if let Some(seg) = self.pgh.get(segment) {
            seg.seg_type.clone()
        } else {
            SegmentType::Text
        }
    }

    pub fn get_segment_rect(&self, segment: usize) -> Option<Rect> {
        if let Some(seg) = self.pgh.get(segment) {
            seg.rect
        } else {
            None
        }
    }

    pub fn last_text_segment(&self) -> usize {
        let mut last_text_seg = 0;
        for (i, x) in self.pgh.iter().enumerate() {
            if x.seg_type == SegmentType::Text {
                last_text_seg = i;
            }
        }
        last_text_seg
    }

    pub fn is_last_text_segment(&self, segment: usize) -> bool {
        segment == self.last_text_segment()
    }

    pub fn is_table(&self) -> bool {
        self.pgh_type == PghType::Table
    }

    pub fn is_code(&self) -> bool {
        self.pgh_type == PghType::Code
    }

    //pub fn select_cursor_word()
    pub fn cursor_to_text_char_index(&self, cursor: &Cursor) -> usize {
        let mut index = 0;
        for seg in 0..cursor.segment {
            if let Some(segment) = self.pgh.get(seg) {
                index += segment.item.text().chars().count();
            }
        }

        if let Some(segment) = self.pgh.get(cursor.segment) {
            let seg_max = segment.item.text().chars().count();
            let seg_index = std::cmp::min(cursor.culumn, seg_max);
            index += seg_index;
        } else {
            println!(
                "cursor_to_text_index fail {:?}, text={}, pgh_len={}",
                cursor,
                self.get_text(),
                self.pgh.len()
            );
        }
        index
    }

    pub fn text_char_index_to_cursor(&self, index: usize, line_no: usize) -> Cursor {
        let mut cursor: Cursor = line_no.into();
        let mut sum_index = 0;
        let last_seg = self.last_text_segment();
        'outer: for segment in &self.pgh[..last_seg + 1] {
            for (i, _) in segment.item.text().chars().enumerate() {
                sum_index += 1;
                if sum_index > index {
                    cursor.culumn = i;
                    break 'outer;
                }
            }
            cursor.segment += 1;
            if cursor.segment > last_seg {
                cursor.segment -= 1;
                cursor.culumn = segment.item.max_culumn();
                break;
            }
        }
        cursor
    }

    pub fn text_byte_index_to_cursor(&self, index: usize, line_no: usize) -> Cursor {
        let mut cursor: Cursor = line_no.into();
        let mut sum_index_byte = 0;
        let last_seg = self.last_text_segment();
        'outer: for segment in &self.pgh[..last_seg + 1] {
            for (i, c) in segment.item.text().chars().enumerate() {
                sum_index_byte += c.len_utf8();
                if sum_index_byte > index {
                    cursor.culumn = i;
                    break 'outer;
                }
            }
            cursor.segment += 1;
            if cursor.segment > last_seg {
                cursor.segment -= 1;
                cursor.culumn = segment.item.max_culumn();
                break;
            }
        }
        cursor
    }

    pub fn is_pgh_view_eq(p1: &PghView, p2: &PghView) -> bool {
        if p1.pgh_type != p2.pgh_type {
            return false;
        }

        if p1.pgh.len() != p2.pgh.len() {
            return false;
        }

        for (i, seg1) in p1.pgh.iter().enumerate() {
            if let Some(seg2) = p2.pgh.get(i) {
                if seg1.seg_type != seg2.seg_type {
                    return false;
                }
                if seg1.item.text() != seg2.item.text() {
                    return false;
                }
            }
        }

        true
    }

    pub fn get_text_warp_width(ui: &Ui, ctx: &Ctx, keep_space: f32) -> f32 {
        let pos = ui.cursor().left_top();
        let edit_right = ctx.edit_rect().right();
        if (ctx.cfg().wrap && pos.x <= edit_right) || keep_space > 1.0 {
            edit_right - pos.x - keep_space
        } else {
            f32::INFINITY
        }
    }

    pub fn layout_sigle_line(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        pgh_view: &PghView,
    ) -> Response {
        let text = pgh_view.get_text();
        let style = ui.style();
        let font_id = style
            .override_font_id
            .clone()
            .unwrap_or_else(|| FontId::default());
        let mut curoser_char_index = None;

        //get index of text
        let cursor = ctx.cursor2();
        if cursor.line_no == line_no {
            let text_index = pgh_view.cursor_to_text_char_index(&cursor);
            curoser_char_index = Some(text_index);
            //println!("cursor_to_text_index {:?} -> {}", cursor, text_index);
        }

        //parser markdown when this pgh has changed or is the cursor line
        let new_pghview;
        let mut mk_pghview = pgh_view;
        if ctx.cfg().is_markdown && (cursor.line_no == line_no || ctx.line_change_reset(line_no))  {    //todo
            let markdown = MarkDownImpl::new(
                &text,
                ctx.cfg().is_markdown,
                curoser_char_index,
                ctx.is_selected(),
                ctx.cfg()
            );
    
            //update pgh in ctx after parser markdown
            new_pghview = markdown.markdown_to_pghview();
            //if !Self::is_pgh_view_eq(pgh_view, &new_pghview) {
                ctx.update_pgh(line_no, &new_pghview);
            //}

            mk_pghview = &new_pghview;
            //println!("parser markdown again, line {}", line_no+1);
        }

        ctx.update_spacing(line_no, mk_pghview.spacing_top, mk_pghview.spacing_bottom);
        
        //response with top space
        let mut top_rect = ui.cursor();
        top_rect.set_right(ctx.edit_right());
        top_rect.set_height(mk_pghview.spacing_top);
        let mut response = ui.allocate_rect(top_rect, ctx.sense());

        if mk_pghview.is_code() {
            return response;
        }

        let mut images = vec![];
        ui.horizontal(|ui| {
            //garagraph
            for (segment, pgh_segment) in mk_pghview.pgh.iter().enumerate() {
                let need_expand = segment == mk_pghview.max_segment();
                //let need_expand = mk_pghview.is_last_text_segment(segment);
                match pgh_segment.seg_type {
                    SegmentType::Text => {
                        let warp_width = Self::get_text_warp_width(ui, ctx, 0.0);
                        response |= PghText::layout_paragraph(
                            ui,
                            ctx,
                            line_no,
                            segment,
                            warp_width,
                            mk_pghview.spacing_top,
                            mk_pghview.spacing_bottom,
                            need_expand,
                            pgh_segment.item.text(),
                            &pgh_segment.item.layout_job(),
                        );
                    }
                    SegmentType::Head => {
                        PghHead::layout_paragraph(ui, ctx, line_no, segment, &pgh_segment.item);
                    }
                    SegmentType::Indent => {
                        response |= PghIndent::layout_paragraph(ui, ctx, line_no, segment);
                    }
                    SegmentType::CheckBox => {
                        PghCheckBox::layout_paragraph(ui, ctx, line_no, segment, &pgh_segment.item);
                    }
                    SegmentType::Point => {
                        response |= PghPoint::layout_paragraph(ui, ctx, line_no, segment, &pgh_segment.item);
                    }
                    SegmentType::QuoteIndent => {
                        response |= PghQuoteIndent::layout_paragraph(
                            ui,
                            ctx,
                            line_no,
                            segment,
                            &pgh_segment.item,
                        );
                    }
                    SegmentType::Break => {
                        //println!("break------------------");
                        PghBreak::layout_paragraph(ui, ctx, line_no, segment, &pgh_segment.item);
                    }
                    SegmentType::Icon => {
                        //println!("break------------------");
                        let r = PghIcon::layout_paragraph(ui, ctx, line_no, segment, &pgh_segment.item);
                        if r.clicked() {
                            if let Some(IconName::icon_external_link(link_info)) = pgh_segment.item.icon_name() {
                                ctx.insert_link_click_command(link_info);
                            }
                        }
                    }
                    SegmentType::Image => {
                        images.push((segment,pgh_segment));
                    }
                }
            }
        });

        //draw images
        for (segment, image) in images {
            ui.horizontal(|ui| {
                PghIndent::layout_paragraph(ui, ctx, line_no, segment);
                PghImage::layout_paragraph(ui, ctx, line_no, segment, &image.item);
            });
        }

        //bottom space
        let mut bottom_rect = ui.cursor();
        bottom_rect.set_right(ctx.edit_right());
        bottom_rect.set_height(mk_pghview.spacing_bottom);
        response |= ui.allocate_rect(bottom_rect, ctx.sense());

        //update curosr from text-index
        if let Some(text_index) = curoser_char_index {
            if !ctx.is_selected() {
                let cursor = mk_pghview.text_char_index_to_cursor(text_index, cursor.line_no);
                if cursor != ctx.cursor2() {
                    println!(
                        "old cursor:{:?}, new cursor:{:?} <- {}",
                        ctx.cursor2(),
                        cursor,
                        text_index
                    );
                    ctx.set_cursor2(cursor);
                    ctx.set_cursor1_reset();
                    println!("update cusor2={:?}", ctx.cursor2());
                }
            }
        }

        response
    }

    pub fn layout(ui: &mut Ui, ctx: &mut Ctx, line_no: usize, pgh_view: &PghView) -> Response {
        match pgh_view.pgh_type {
            PghType::Table => {
                Self::layout_table_line(ui, ctx, line_no, pgh_view)
            }
            PghType::Code => {
                Self::layout_code_line(ui, ctx, line_no, pgh_view)
            }
            _=> {
                Self::layout_sigle_line(ui, ctx, line_no, pgh_view)
            }
        }
    }
}

/// impl code
impl PghView {
    pub fn code_enter(&mut self, c: &Cursor) {
        if let Some(seg) = self.pgh.get(c.segment) {
            let (left, right) = seg.item.enter(c);
            self.update_segment_text(c.segment, left);
            self.insert_text(c.segment+1, right, None);
        }
    }

    //return the new Cursor
    pub fn code_insert(&mut self, c: &Cursor, s: &str) -> Cursor {
        let mut new_c = c.clone();
        if let Some(seg) = self.pgh.get(c.segment) {
            let (left, right) = seg.item.insert(c);
            let new_s =  left + &s + &right;
            let mut seg_idx = c.segment;
            let lines: Vec<&str> = new_s.split('\n').collect();
            for (i, line) in lines.iter().enumerate() {
                if i == 0 {
                    self.update_segment_text(seg_idx, line.to_string());
                } else {
                    self.insert_text(seg_idx, line.to_string(), None);
                }
                //set last line cursor
                if i + 1 == lines.len() {
                    new_c.segment = seg_idx;
                    new_c.culumn = line.chars().count() - right.chars().count();
                }

                seg_idx += 1;
            }
        }
        new_c
    }

    pub fn code_format(slice: &LightSlice, ui: &Ui, ctx: &Ctx) -> TextFormat {
        let color = if ctx.cfg().dark_mode{
            slice.dark_color
        } else {
            slice.light_color
        };

        let mut format = TextFormat::default();
        format.font_id.size = ctx.font_size();
        format.font_id.family = FontFamily::Monospace;
        format.color = color;
        format
    }

    fn code_highlight_job(ui: &Ui, ctx: &mut Ctx, line_no: usize, pgh_view: &PghView) {
        if !ctx.line_change_reset(line_no) {
            return;
        }
        if let Some(code_lang) = &pgh_view.code_lang {
            let text = pgh_view.text_to_vec().join("\n");
            let source = text.as_bytes();
            if let Ok(lines) = highlight_lines(code_lang.clone(), source) {
                for (segment, line) in lines.iter().enumerate() {
                    if line.len() > 0 {
                        let mut job: LayoutJob = LayoutJob::default();
                        for slice in line {
                            job.append(&String::from_utf8_lossy(slice.slice), 0.0, 
                                Self::code_format(slice, ui, ctx));
                        }
                        ctx.update_pgh_segment_job(line_no, segment, Some(job));
                    } else {
                        ctx.update_pgh_segment_job(line_no, segment, None);
                    }
                }
            }
            //println!("code_highlight_job updated, line:{}", line_no+1);
        }
    }

    fn font_size_menus(ctx: &mut Ctx, ui: &mut Ui, pghview: &PghView, line_no: usize) {
        let cur_lang = pghview.code_lang.clone().unwrap_or_else(||"Lang".to_string());
        ui.menu_button( cur_lang, |ui| {
            for lang in support_lang() {
                if ui.button(lang).clicked() {
                    if let Some(update) = ctx.get_line_mut(line_no) {
                        update.code_lang = Some(lang.to_string());
                        update.change_tick += 1;
                    }
                    ctx.on_content_change();
                    ui.close_menu();
                }
            }
        });
    }

    pub fn layout_code_line(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        pgh_view: &PghView,
    ) -> Response {
        //top space
        let mut top_rect = ui.cursor();
        top_rect.set_right(ctx.edit_right());
        top_rect.set_height(pgh_view.spacing_top);
        let mut response = ui.allocate_rect(top_rect, ctx.sense());

        //highlight
        Self::code_highlight_job(ui, ctx, line_no, pgh_view);

        //layout
        for (segment, pgh_segment) in pgh_view.pgh.iter().enumerate() {
            ui.horizontal(|ui|{
                response |= PghIndent::layout_paragraph(ui, ctx, line_no, segment);
                let need_expand = true;
                let keep_space = if segment == 0 {80.0} else {0.0};
                match pgh_segment.seg_type {
                    SegmentType::Text => {
                        let warp_width = Self::get_text_warp_width(ui, ctx, keep_space);
                        response |= PghText::layout_paragraph(
                            ui,
                            ctx,
                            line_no,
                            segment,
                            warp_width,
                            pgh_view.spacing_top,
                            pgh_view.spacing_bottom,
                            need_expand,
                            pgh_segment.item.text(),
                            &pgh_segment.item.layout_job(), 
                        );
                    }
                    _ => {}
                }
                if segment == 0 {
                    Self::font_size_menus(ctx, ui, pgh_view, line_no);
                }
            });
        }

        //bottom space
        let mut bottom_rect = ui.cursor();
        bottom_rect.set_right(ctx.edit_right());
        bottom_rect.set_height(pgh_view.spacing_bottom);
        response |= ui.allocate_rect(bottom_rect, ctx.sense());

        //frame
        let mut rect = response.rect;
        rect.min.x += 12.0;
        let painter = ui.painter();
        //let stroke = Stroke::new(1.0, ui.visuals().weak_text_color());
        //painter.rect_stroke(rect, 3.0, stroke);
        //painter.line_segment([rect.left_top(), rect.right_top()], stroke);
        //painter.line_segment([rect.left_bottom(), rect.right_bottom()], stroke);
        painter.rect_filled(rect, 3.0, ui.style().visuals.faint_bg_color);
        response
    }
}

/// impl tables
impl PghView {
    pub fn table_segment_to_cell(&self, segment: usize) -> Option<TableCell> {
        if let Some(table_info) = &self.table_info {
            Some(TableCell {
                row: segment / table_info.col_count,
                col: segment % table_info.col_count,
                segment,
            })
        } else {
            None
        }
    }

    pub fn table_cell_to_segment(&self, cell: &TableCell) -> usize {
        if let Some(table_info) = &self.table_info {
            cell.row * table_info.col_count + cell.col
        } else {
            0
        }
    }

    //return left-top,right-bottom
    pub fn table_range_to_cells(&self, s1: usize, s2: usize) -> Option<(TableCell, TableCell)> {
        if let Some(table_info) = &self.table_info {
            let c1 = self.table_segment_to_cell(s1).unwrap();
            let c2 = self.table_segment_to_cell(s2).unwrap();
            let row_min = std::cmp::min(c1.row, c2.row);
            let row_max = std::cmp::max(c1.row, c2.row);
            let col_min = std::cmp::min(c1.col, c2.col);
            let col_max = std::cmp::max(c1.col, c2.col);
            Some((
                TableCell {
                    row: row_min,
                    col: col_min,
                    segment: row_min * table_info.col_count + col_min,
                },
                TableCell {
                    row: row_max,
                    col: col_max,
                    segment: row_max * table_info.col_count + col_max,
                },
            ))
        } else {
            None
        }
    }

    pub fn table_range_rect(&self, s1: usize, s2: usize) -> Option<Rect> {
        if let Some((c1, c2)) = self.table_range_to_cells(s1, s2) {
            if let Some(rect1) = self.get_segment_rect(c1.segment) {
                if let Some(rect2) = self.get_segment_rect(c2.segment) {
                    //println!("2 {}-{} c1:{:?} c2:{:?}", s1, s2, &c1, &c2);
                    return Some(Rect::from_two_pos(rect1.left_top(), rect2.right_bottom()));
                }
            }
        }
        None
    }

    pub fn table_is_empty_row(&self, row: usize) -> bool {
        if let Some(table_info) = &self.table_info {
            for col in 0..table_info.col_count {
                let segment = row * table_info.col_count + col;
                if let Some(pgh_segment) = self.pgh.get(segment) {
                    if pgh_segment.item.text().len() > 0 {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn table_is_empty_col(&self, col: usize) -> bool {
        if let Some(table_info) = &self.table_info {
            for row in 0..table_info.row_count {
                let segment = row * table_info.col_count + col;
                if let Some(pgh_segment) = self.pgh.get(segment) {
                    if pgh_segment.item.text().len() > 0 {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn table_delete_row(&mut self, row: usize) {
        if let Some(table_info) = &mut self.table_info {
            for col in 0..table_info.col_count {
                let segment = row * table_info.col_count;
                self.pgh.remove(segment);
            }
            table_info.row_count -= 1;
        }
    }

    pub fn table_delete_col(&mut self, col: usize) {
        if let Some(table_info) = &mut self.table_info {
            for row in (0..table_info.row_count).rev() {
                let segment = row * table_info.col_count + col;
                self.pgh.remove(segment);
            }
            table_info.col_count -= 1;
        }
    }

    pub fn table_delete_empty_in_range(&mut self, s1: usize, s2: usize) {
        let mut empty_row = vec![];
        let mut empty_col = vec![];
        if let Some(table_info) = &self.table_info {
            if let Some((min, max)) = self.table_range_to_cells(s1, s2) {
                println!("min:{:?} max:{:?}", min, max);
                if min.col == 0 && max.col + 1 == table_info.col_count {
                    for row in min.row..=max.row {
                        if self.table_is_empty_row(row) {
                            empty_row.push(row);
                        }
                    }
                }
                if min.row == 0 && max.row + 1 == table_info.row_count {
                    for col in min.col..=max.col {
                        if self.table_is_empty_col(col) {
                            empty_col.push(col);
                        }
                    }
                }
            }
            for row in empty_row.iter().rev() {
                self.table_delete_row(*row);
            }
            for col in empty_col.iter().rev() {
                self.table_delete_col(*col);
            }
        }
    }

    ///return: segments inserted
    pub fn table_insert_row(&mut self, row: usize) -> usize {
        let mut segment = 0;
        let mut col_count = 0;
        if let Some(table_info) = &mut self.table_info {
            segment = table_info.col_count * row;
            col_count = table_info.col_count;
            table_info.row_count += 1;
        }
        for i in 0..col_count {
            self.insert_text(segment, "".to_string(), None);
        }
        return col_count;
    }

    ///return: segments inserted
    pub fn table_insert_col(&mut self, col: usize) -> usize {
        let mut segments = vec![];
        if let Some(table_info) = &mut self.table_info {
            for row in (0..table_info.row_count).rev() {
                segments.push(table_info.col_count * row + col);
            }
            table_info.col_count += 1;
        }
        for i in &segments {
            self.insert_text(*i, "".to_string(), None);
        }
        return segments.len();
    }

    //return new segment after change
    pub fn table_merge(&mut self, segment: usize, change: &PghView) -> usize {
        let mut min_cell = TableCell {
            row: 0,
            col: 0,
            segment: 0,
        };
        let mut new_seg = segment;
        if let Some(table_info) = self.table_info.clone() {
            if let Some(change_info) = &change.table_info {
                min_cell = self.table_segment_to_cell(segment).unwrap();
                let max_cell = TableCell {
                    row: min_cell.row + change_info.row_count,
                    col: min_cell.col + change_info.col_count,
                    segment: 0,
                };
                for r in table_info.row_count..max_cell.row {
                    self.table_insert_row(table_info.row_count);
                }
                for c in table_info.col_count..max_cell.col {
                    self.table_insert_col(table_info.col_count);
                }
            }
        }

        if let Some(table_info) = self.table_info.clone() {
            new_seg = min_cell.row * table_info.col_count + min_cell.col;
            if let Some(change_info) = &change.table_info {
                for r in 0..change_info.row_count {
                    for c in 0..change_info.col_count {
                        let org_seg = r * change_info.col_count + c;
                        let org_txt = change.get_segment_text(org_seg);
                        let dst_seg =
                            (min_cell.row + r) * table_info.col_count + (min_cell.col + c);
                        self.update_segment_text(dst_seg, org_txt);
                    }
                }
            }
        }

        new_seg
    }

    pub fn table_head_job(ui: &Ui, ctx: &Ctx, text: &str) -> LayoutJob {
        let mut job: LayoutJob = LayoutJob::default();
        let mut format = TextFormat::default();
        format.font_id.size = ctx.font_size();
        format.font_id.family = FontFamily::Name("msyhb".into());
        format.color = ctx.cfg().text_color();
        job.append(text, 0.0, format);
        job
    }

    pub fn table_cell_job(ui: &Ui, ctx: &Ctx, text: &str) -> LayoutJob {
        let mut job: LayoutJob = LayoutJob::default();
        let mut format = TextFormat::default();
        format.font_id.size = ctx.font_size();
        format.color = ctx.cfg().text_color();
        job.append(text, 0.0, format);
        job
    }

    pub fn table_guess_text_width(ui: &Ui, ctx: &Ctx, row: usize, text: String) -> f32 {
        let job = if row == 0 {
            Self::table_head_job(ui, ctx, &text)
        } else {
            Self::table_cell_job(ui, ctx, &text)
        };
        ui.fonts(|f| f.layout_job(job)).rect.width()
    }

    pub fn table_guess_width(&self, ui: &Ui, ctx: &Ctx) -> Vec<f32> {
        let mut width_info = vec![];
        let mut max_width = ctx.edit_width();

        if let Some(table_info) = &self.table_info {
            max_width -= table_info.spacing_indent;
            max_width -= 64.0; //left right buttons space

            for c in 0..table_info.col_count {
                let mut c_width = 0.0;
                for r in 0..table_info.row_count {
                    let cell_i = r * table_info.col_count + c;
                    if let Some(pgh_segment) = self.pgh.get(cell_i) {
                        let text = pgh_segment.item.text();
                        c_width = Self::table_guess_text_width(ui, ctx, r, text).at_least(c_width);
                    }
                }
                width_info.push(c_width);
                if c != 0 {
                    max_width -= table_info.spacing_x;
                }
            }

            let total: f32 = width_info.iter().sum();
            let warp_total: f32 = width_info
                .iter()
                .filter(|w| **w > table_info.col_min_width)
                .sum();
            let keep_total: f32 = total - warp_total;
            let max_warp_width = max_width - keep_total;

            if total > max_width && max_width > 0.0 && max_warp_width > 0.0 {
                let new_info: Vec<f32> = width_info
                    .iter()
                    .map(|w| {
                        if *w <= table_info.col_min_width {
                            *w
                        } else {
                            (w / warp_total * max_warp_width).at_least(table_info.col_min_width)
                        }
                    })
                    .collect();
                width_info = new_info;
            }
            //let total:f32 = width_info.iter().sum();
            //println!("longest col:{:?} total:{} max_width:{}", width_info, total, max_width);
        };

        width_info
    }

    fn table_draw_frame(
        ui: &mut Ui,
        ctx: &mut Ctx,
        table_info: &TableInfo,
        cell_rects: &Vec<Vec<Rect>>,
    ) {
        if table_info.has_frame {
            for (r, row) in cell_rects.iter().enumerate() {
                for (c, cell) in row.iter().enumerate() {
                    let rect = cell.expand2(Vec2 {
                        x: table_info.spacing_x / 2.0,
                        y: table_info.spacing_y / 2.0,
                    });
                    ui.painter().rect_stroke(
                        rect,
                        1.0,
                        Stroke::new(0.5, ui.visuals().weak_text_color()),
                    );
                }
            }
        }
    }

    fn table_reset_cursor(pgh: &PghView, ctx: &mut Ctx, row: usize, col: usize, cursor: &Cursor) {
        let mut new_cursor = *cursor;
        if let Some(info) = &pgh.table_info {
            new_cursor.segment = row * info.col_count + col;
            ctx.set_cursor2(new_cursor);
            ctx.set_cursor1_reset();
        }
    }

    fn table_draw_buttons(
        ui: &mut Ui,
        ctx: &mut Ctx,
        cursor: &Cursor,
        table_info: &TableInfo,
        cell_rects: &Vec<Vec<Rect>>,
    ) {
        let segment = cursor.segment;
        let r = segment / table_info.col_count;
        let c = segment % table_info.col_count;
        let mut insert_col: Option<usize> = None;
        let mut insert_row: Option<usize> = None;

        //top buttons
        if let Some(row) = cell_rects.get(0) {
            if let Some(cell) = row.get(c) {
                let size = icon::icon_size(ui, icon::IconName::icon_chevron_down, 12.0);
                let mut rect = cell.expand2(Vec2 {
                    x: table_info.spacing_x / 2.0,
                    y: table_info.spacing_y / 2.0,
                });
                rect.min.x -= size.x / 2.0;
                rect.min.y -= size.y;
                rect.max.x -= size.x / 2.0;

                let id: String = format!("{}.left", segment);
                if icon::icon_button(ui, id, rect.left_top(), icon::IconName::icon_chevron_down, 12.0) {
                    insert_col = Some(c);
                }
                let id: String = format!("{}.right", segment);
                if icon::icon_button(ui, id, rect.right_top(), icon::IconName::icon_chevron_down, 12.0) {
                    insert_col = Some(c+1);
                }
            }
        }

        //right buttons
        if let Some(row) = cell_rects.get(r) {
            if let Some(cell) = row.get(table_info.col_count - 1) {
                let size = icon::icon_size(ui, icon::IconName::icon_chevron_left, 12.0);
                let mut rect = cell.expand2(Vec2 {
                    x: table_info.spacing_x / 2.0,
                    y: table_info.spacing_y / 2.0,
                });
                rect.min.y -= size.y / 2.0 - 2.0;
                rect.max.y -= size.y / 2.0 - 2.0;
                rect.max.x += table_info.spacing_x / 4.0;

                let id: String = format!("{}.top", segment);
                if icon::icon_button(ui, id, rect.right_top(), icon::IconName::icon_chevron_left, 12.0) {
                    insert_row = Some(r);
                }
                let id: String = format!("{}.bottom", segment);
                if icon::icon_button(ui, id, rect.right_bottom(), icon::IconName::icon_chevron_left, 12.0) {
                    insert_row = Some(r+1);
                }
            }
        }

        //insert row/col
        if insert_col != None || insert_row != None {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            undo_cmd.set_cursor(ctx.cursor2());
            undo_cmd.push_update(cursor.line_no, ctx.get_line_clone(cursor.line_no));
            if let Some(pgh) = ctx.get_line_mut(cursor.line_no) {
                if let Some(row) = insert_row {
                    pgh.table_insert_row(row);
                }
                if let Some(col) = insert_col {
                    pgh.table_insert_col(col);
                    if let Some(info) = &pgh.table_info {
                        let mut new_cursor = *cursor;
                        new_cursor.segment = r * info.col_count + c;
                        ctx.set_cursor2(new_cursor);
                        ctx.set_cursor1_reset();
                    }
                }
            }
            redo_cmd.push_update(cursor.line_no, ctx.get_line_clone(cursor.line_no));
            redo_cmd.set_cursor(ctx.cursor2());
            ctx.push_do(undo_cmd, redo_cmd);
        }
        
    }

    pub fn layout_table(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        pgh_view: &PghView,
    ) -> Response {
        let mut response = ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1;

        let width_info = pgh_view.table_guess_width(ui, ctx);
        let max_col_width = ctx.edit_width();
        if let Some(table_info) = &pgh_view.table_info {
            let table_id = format!("table_id_{}", line_no);
            let iner_rsp = Grid::new(&table_id)
                .striped(!table_info.has_frame)
                .min_col_width(0.0)
                .min_row_height(0.0)
                .max_col_width(max_col_width)
                .spacing(Vec2 {
                    x: table_info.spacing_x,
                    y: table_info.spacing_y,
                })
                .show(ui, |ui| {
                    let mut all_cell_rects = vec![];
                    for r in 0..table_info.row_count {
                        let mut row_cell_rects = vec![];
                        for c in 0..table_info.col_count {
                            let cell_i = r * table_info.col_count + c;
                            if let Some(pgh_segment) = pgh_view.pgh.get(cell_i) {
                                let text = pgh_segment.item.text();
                                let job = if r == 0 {
                                    Self::table_head_job(ui, ctx, &text)
                                } else {
                                    Self::table_cell_job(ui, ctx, &text)
                                };
                                let warp_width =
                                    width_info.get(c).unwrap_or_else(|| &max_col_width);
                                let begin_pos = ui.cursor().left_top().x;
                                let rsp = PghText::layout_paragraph(
                                    ui,
                                    ctx,
                                    line_no,
                                    cell_i,
                                    *warp_width,
                                    table_info.spacing_y / 2.0,
                                    table_info.spacing_y / 2.0,
                                    true,
                                    text,
                                    &Some(job),
                                );

                                row_cell_rects.push(rsp.rect);
                                response |= rsp;
                            }
                        }
                        ui.end_row();

                        //update the row max height for each cell
                        let mut max_height = 0.0;
                        for r in &row_cell_rects {
                            max_height = max_height.at_least(r.height());
                        }
                        for crect in &mut row_cell_rects {
                            crect.set_height(max_height);
                        }
                        all_cell_rects.push(row_cell_rects.clone());
                    }

                    // draw frame
                    Self::table_draw_frame(ui, ctx, &table_info, &all_cell_rects);

                    // draw button
                    let cursor = ctx.cursor2();
                    if cursor.line_no == line_no && !ctx.is_selected() {
                        Self::table_draw_buttons(ui, ctx, &cursor, &table_info, &all_cell_rects);
                    }
                });
            response |= iner_rsp.response;
        };

        response
    }

    pub fn layout_table_line(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        pgh_view: &PghView,
    ) -> Response {
        let mut response = ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1;
        if let Some(table_info) = &pgh_view.table_info {
            //top space
            let size = icon::icon_size(ui, icon::IconName::icon_chevron_down, 12.0);
            ui.allocate_exact_size(vec2(0.0, size.y), ctx.sense()).1;

            ctx.update_spacing(
                line_no,
                table_info.spacing_y / 2.0,
                table_info.spacing_y / 2.0,
            );

            ui.horizontal(|ui| {
                ui.allocate_exact_size(vec2(table_info.spacing_indent, 0.0), ctx.sense());
                response |= Self::layout_table(ui, ctx, line_no, pgh_view);
            });

            //bottom space
            let mut bottom_rect = ui.cursor();
            bottom_rect.set_right(ctx.edit_right());
            bottom_rect.set_height(size.y);
            response |= ui.allocate_rect(bottom_rect, ctx.sense());
            //response |= ui.allocate_exact_size(vec2(0.0, size.y), ctx.sense()).1;
        }
        response
    }
}
