use core::f32;
use std::ops::Add;

use crate::sitter::highlight_lines;
use crate::medit::{ImageInfo, LinkInfo, PghType, CharRect, Cursor, MarkDownImpl, SegmentType, PghView, DoItem, DoCmd, DoMngr, Command, FindReplaceCtx};
use eframe::egui::{Color32, NumExt, Pos2, Rect, Sense, Ui};
use eframe::egui::epaint::text::LayoutJob;
use regex::Regex;
use arboard::Clipboard;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, PartialEq)]
pub struct State {
    top_line: usize,
    bottom_line: usize,
    scroll_to_line: Option<usize>,
    scroll_to_rect: Option<Rect>,

    cursor1: Cursor,
    cursor2: Cursor,
    cursor2_bak: Cursor,
    cursor_show_time: u64, //milliseconds
    cursor_show_bool: bool,
    selecting: bool,
    is_pointer_gone: bool,

    content_change_tick: u64,
    ime_area_changed: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            top_line: 0,
            bottom_line: 0,
            scroll_to_line: None,
            scroll_to_rect: None,
            cursor1: 0.into(),
            cursor2: 0.into(), 
            cursor2_bak: 0.into(),
            cursor_show_time: 0,
            cursor_show_bool: true,
            selecting: false,
            is_pointer_gone: false,
            content_change_tick: 0,
            ime_area_changed: false,
        }
    }
}

pub struct Area {
    max_rect: Rect,
    line_no_rect: Rect,
    edit_rect: Rect,
    scroll_width: f32,
}

impl Default for Area {
    fn default() -> Self {
        Area {
            max_rect: Rect::ZERO,
            line_no_rect: Rect::ZERO,
            edit_rect: Rect::ZERO,
            scroll_width: 0.0,
        }
    }
}

#[derive(Debug)]
pub struct FindCacheItem{
    pub start: Cursor,
    pub end: Cursor,
    pub line_text: Option<String>,
}

pub struct FindCache {
    pub cache: Vec<FindCacheItem>,
}

impl FindCache {
    pub fn new() -> Self {
        Self {
            cache: vec![]
        }
    }
}

pub struct EditColors {
    pub text_color: Color32,
    pub code_bg_color: Color32,
    pub link_color: Color32,
    pub weak_color: Color32,
}
pub struct EditCfg {
    pub font_size: f32,
    pub font_heigh: f32,
    pub dark_mode: bool,
    pub wrap: bool,
    pub show_line_no: bool,
    pub is_markdown: bool,
    pub image_path: Option<String>,     //save image in markdown
    pub lang: Option<String>,
    pub need_line_click_cmd: bool,
    pub hightlight_seleted_word: bool,

    pub dark_color: EditColors,
    pub light_color: EditColors,
}

impl EditCfg {
    pub fn new(font_size: f32, is_markdown: bool, image_path: Option<String>) -> Self {
        Self {
            font_size,
            dark_mode: true,
            font_heigh: 23.0,
            wrap: false,
            show_line_no: false,
            is_markdown,
            image_path,
            lang: None,
            need_line_click_cmd: false,
            hightlight_seleted_word: true,

            dark_color: EditColors {
                text_color: Color32::from_rgb(192,192,192),
                code_bg_color: Color32::from_gray(64),
                link_color: Color32::from_rgb(90, 170, 255),
                weak_color: Color32::from_rgb(100,100,100),
            },

            light_color: EditColors {
                text_color: Color32::from_rgb(0,0,0),
                code_bg_color: Color32::from_gray(230),
                link_color: Color32::from_rgb(0, 155, 255),
                weak_color: Color32::from_rgb(100,100,100),
            },
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
        self.colors().text_color
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
}

pub struct Ctx {
    pgh_views: Vec<PghView>,
    patch_num: usize,
    state: State, //mark somthing has changed after on_event
    state_changed: bool,
    area: Area,
    open_time: u128,
    do_mngr: DoMngr,
    cmd_list: Vec<Command>,
    find_cache: FindCache,
    find_param: FindReplaceCtx,
    same_cache: FindCache,
    clipboard: Clipboard,
    cfg: EditCfg,
}

impl Ctx {
    pub fn new(text: &str, is_markdown: bool, image_path: Option<String>) -> Self {
        let font_size = 17.0;
        let mut ctx = Self {
            pgh_views: vec![],
            patch_num: 80,
            state: State::default(),
            state_changed: false,
            area: Area::default(),
            open_time: 0, 
            do_mngr: DoMngr::new(),
            cmd_list: vec![],
            find_cache: FindCache::new(),
            find_param: FindReplaceCtx::new(),
            same_cache: FindCache::new(),
            clipboard: Clipboard::new().unwrap(),   //todo: unwrap unsafe
            cfg: EditCfg::new(font_size, is_markdown, image_path)
        };

        let markdown_impl = MarkDownImpl::new(
            text,
            is_markdown,
            None,
            false,
            ctx.cfg()
        );

        ctx.pgh_views = markdown_impl.markdown_to_pgh_texts();
        ctx
    }
}

/// impl about cursor
///
impl Ctx {
    pub fn cursor1(&self) -> Cursor {
        self.state.cursor1
    }

    pub fn cursor2(&self) -> Cursor {
        self.state.cursor2
    }

    pub fn cursor_from_pos(&self, pos: &Pos2) -> Option<Cursor> {
        let top_line = self.top_line();
        for (i, pgh_view) in self.pgh_views[top_line..self.patch_end()]
            .iter()
            .enumerate()
        {
            if pgh_view.is_pos_in(pos) {
                if let Some(cursor) = pgh_view.cursor_from_pos(top_line + i, pos) {
                    return Some(cursor);
                }
            }
        }
        None
    }

    pub fn set_cursor2_from_pos(&mut self, pos: &Pos2) {
        if let Some(cursor) = self.cursor_from_pos(pos) {
            self.state.cursor2 = cursor;
        }
    }

    pub fn set_cursor1_reset(&mut self) {
        self.state.cursor1 = self.state.cursor2;
    }

    pub fn set_cursor_switch(&mut self) {
        let c1 = self.cursor1();
        self.state.cursor1 = self.cursor2();
        self.state.cursor2 = c1;
    }

    pub fn set_cursors_to_min(&mut self) {
        let min = std::cmp::min(self.cursor1(), self.cursor2());
        self.state.cursor1 = min;
        self.state.cursor2 = min;
    }

    pub fn set_cursors_select_all(&mut self) {
        self.state.cursor1 = 0.into();
        self.set_cursor2_to_end();
    }

    pub fn set_cursor1(&mut self, cursor: Cursor) {
        self.state.cursor1 = cursor;
    }

    pub fn set_cursor2(&mut self, cursor: Cursor) {
        self.state.cursor2 = cursor;
    }

    pub fn set_cursor2_to_end(&mut self) {
        let max_cursor = (usize::MAX,usize::MAX,usize::MAX).into();
        self.state.cursor2 = self.cursor_check(&max_cursor);
    }

    pub fn cursor2_cmp_and_bakup(&mut self) -> bool {
        let changed = self.state.cursor2_bak != self.state.cursor2;
        self.state.cursor2_bak = self.state.cursor2;
        changed
    }

    pub fn cursor2_bakup_reset(&mut self) {
        self.state.cursor2_bak = 0.into();
    }

    pub fn get_pos_from_cursor(&self, cursor: &Cursor) -> Option<Rect> {
        let line = cursor.line_no;
        let culumn = cursor.culumn;

        if line >= self.pgh_views.len() || line < self.state.top_line {
            return None;
        }
        if let Some(rect) = self.pgh_views[line].pos_from_cursor(cursor) {
            return Some(rect);
        }
        None
    }

    fn get_table_cursor_rect(&self, min: &Cursor, max: &Cursor) -> Option<Rect> {
        if min.line_no != max.line_no {
            return None;
        }
        if let Some(pgh_view) = self.pgh_views.get(min.line_no) {
            if pgh_view.is_table() {
                return pgh_view.table_range_rect(min.segment, max.segment);
            }
        }
        None
    }

    pub fn get_cursor2_line_rect(&self) -> Option<Rect> {
        let cursor = self.cursor2();
        if let Some(pghview) = self.pgh_views.get(cursor.line_no) {
            if let Some(segment_rect) = pghview.get_segment_rect(cursor.segment) {
                return Some(segment_rect)
            }
            pghview.rect()
        } else {
            None
        }
    }

    pub fn get_cursor_rects(&self) -> Option<Vec<Rect>> {
        let mut rects = vec![];
        if self.is_selected() {
            if let Some(mut rc) = self.get_crange_rects(self.state.cursor1, self.state.cursor2) {
                rects.append(&mut rc);
            }
        }

        for rc in &self.same_cache.cache {  
            if rc.start == self.state.cursor1 || rc.start == self.state.cursor2 {
                continue;
            }
            if let Some(mut rc) = self.get_crange_rects(rc.start, rc.end) {
                rects.append(&mut rc);
            }
        }

        Some(rects)
    }

    fn get_crange_rects(&self, c1: Cursor, c2: Cursor) -> Option<Vec<Rect>> {
        if c1 == c2 {
            return None;
        }
        let mut min = std::cmp::min(c1, c2);
        let max = std::cmp::max(c1, c2);
        let mut rects = vec![];

        //is select all
        if min == 0.into() && max.line_no == usize::MAX {
            rects.push(self.edit_rect());
            return Some(rects);
        }

        if min.line_no < self.top_line() {
            min = 0.into();
            min.line_no = self.top_line();
        }

        //if max.line_no >= self.bottom_line() {
        //    max.line_no = self.bottom_line();
        //    max.culmax = self.bottom_pgh().pgh.cursor_max_culumn();
        //}

        let mut left = self.left_top().x + self.line_no_width();
        let mut width = self.edit_width();

        if let Some(min_rect) = self.get_pos_from_cursor(&min) {
            if let Some(max_rect) = self.get_pos_from_cursor(&max) {
                if let Some(table_rect) = self.get_table_cursor_rect(&min, &max) {
                    left = table_rect.left();
                    width = table_rect.width().at_most(self.edit_width());
                }

                if (min_rect.min.y - max_rect.min.y).abs() < 0.1 {
                    //the same line
                    rects.push(Rect::from_min_max(min_rect.min, max_rect.max));
                } else {
                    //first line
                    rects.push(Rect::from_min_max(
                        min_rect.min,
                        Pos2 {
                            x: left + width,
                            y: min_rect.max.y,
                        },
                    ));
                    //middle area
                    rects.push(Rect::from_min_max(
                        Pos2 {
                            x: left,
                            y: min_rect.max.y,
                        },
                        Pos2 {
                            x: left + width,
                            y: max_rect.min.y,
                        },
                    ));
                    //last line
                    rects.push(Rect::from_min_max(
                        Pos2 {
                            x: left,
                            y: max_rect.min.y,
                        },
                        max_rect.max,
                    ));
                }
            }
        }
        Some(rects)
    }

    fn get_word_at_cursor(text: &str, cursor: usize) -> Option<(usize, usize)> {
        let chars: Vec<char> = text.chars().collect();
        let delimiters = " \t~`!@#$%^&*()+-=[]\\{}|;':\",./<>?，。、；：‘’“”";

        if cursor >= chars.len() {
            return None;
        }

        let is_same_char_type = |c1: &char, c2: &char| c1.is_ascii() == c2.is_ascii();
        let is_delimiters = |c1: &char| delimiters.contains(*c1);

        let mut start = cursor;
        let mut end = cursor;

        while start > 0 && 
              !is_delimiters(&chars[start - 1]) &&
              is_same_char_type(&chars[start - 1], &chars[cursor]) {
            start -= 1;
        }
    
        while end < chars.len() && 
              !is_delimiters(&chars[end]) &&
              is_same_char_type(&chars[end], &chars[cursor]) {
            end += 1;
        }
    
        Some((start, end))
    }

    pub fn select_word_at_cursor(&mut self) {
        let line_no = self.cursor2().line_no;
        let segment = self.cursor2().segment;
        let culumn = self.cursor2().culumn;
        if let Some(pghview) = self.pgh_views.get(line_no) {
            if let Some(seg) = pghview.pgh.get(segment) {
                let text = seg.item.text();
                if let Some((start,end)) = Self::get_word_at_cursor(&text, culumn) {
                    self.state.cursor1 = self.cursor2();
                    self.state.cursor1.culumn = start;
                    self.state.cursor2.culumn = end;
                }
            }
        }
    }

    pub fn select_line_at_cursor(&mut self) {
        if let Some(pghview) = self.pgh_views.get(self.cursor2().line_no) {
            self.state.cursor1 = self.cursor2();
            self.state.cursor1.segment = 0;
            self.state.cursor1.culumn = 0;
            self.state.cursor2.segment = pghview.max_segment();
            self.state.cursor2.culumn = pghview.max_culumn(&self.state.cursor2);
        }
    }

    pub fn cursor_pghview(&self, cursor: &Cursor) -> Option<&PghView> {
        let line_no = cursor.line_no;
        if line_no >= self.pgh_views.len() {
            return None;
        }

        Some(&self.pgh_views[line_no])
    }

    pub fn cursor_check(&self, cursor: &Cursor) -> Cursor {
        let mut cursor = cursor.clone();

        //check cursor.line_no
        if cursor.line_no >= self.pgh_views.len() && self.pgh_views.len() > 0 {
            cursor.line_no = self.pgh_views.len() - 1;
        }

        if let Some(pgh_view) = self.cursor_pghview(&cursor) {
            //check cursor.segment
            let max_segment = pgh_view.max_segment();
            if cursor.segment > max_segment {
                cursor.segment = max_segment;
            }

            //check cursor.culumn
            let max_culumn = pgh_view.max_culumn(&cursor);
            if cursor.culumn > max_culumn {
                cursor.culumn = max_culumn;
            }
        }

        cursor
    }

    pub fn cursor2_move_next(&mut self) {
        if let Some(pgh_view) = self.cursor_pghview(&self.state.cursor2) {
            let new = self.state.cursor2.cursor_move_next(pgh_view);
            self.state.cursor2 = self.cursor_check(&new);
        }

        //next node is not text segment, skip over
        if let Some(pgh_view) = self.cursor_pghview(&self.state.cursor2) {
            if pgh_view.get_segment_type(self.state.cursor2.segment) != SegmentType::Text {
                if self.state.cursor2.segment > pgh_view.last_text_segment() {
                    self.state.cursor2.line_no += 1;
                    self.state.cursor2.segment = 0;
                    self.state.cursor2.culumn = 0;
                } else {
                    self.state.cursor2.segment += 1;
                    self.state.cursor2.culumn = 0;
                }
                self.state.cursor2 = self.cursor_check(&self.state.cursor2);
            }
        }
    }

    pub fn cursor2_move_prev(&mut self) {
        if let Some(pgh_view) = self.cursor_pghview(&self.state.cursor2) {
            let new = self.state.cursor2.cursor_move_prev(pgh_view);
            self.state.cursor2 = self.cursor_check(&new);
        }
    }

    pub fn cursor2_move_up(&mut self) {
        if let Some(mut rect) = self.get_pos_from_cursor(&self.state.cursor2) {
            rect.max.y = rect.min.y;
            if let Some(pgh_view) = self.pgh_views.get(self.state.cursor2.line_no) {
                rect.min.y -= pgh_view.spacing_top;
            }
            rect.min.y -= self.font_heigh() / 2.0;
            if let Some(c) = self.cursor_from_pos(&rect.center()) {
                if self.state.cursor2 != c {
                    self.state.cursor2 = c;
                    return;
                }
            }
        }
        let new = self.state.cursor2.cursor_move_up();
        let new = self.cursor_check(&new);
        self.state.cursor2 = new;
    }

    pub fn cursor2_move_down(&mut self) {
        if let Some(mut rect) = self.get_pos_from_cursor(&self.state.cursor2) {
            rect.min.y = rect.max.y;
            if let Some(pgh_view) = self.pgh_views.get(self.state.cursor2.line_no) {
                rect.max.y += pgh_view.spacing_bottom;
            }
            rect.max.y += self.font_heigh() / 2.0;
            if let Some(c) = self.cursor_from_pos(&rect.center()) {
                if self.state.cursor2 != c {
                    self.state.cursor2 = c;
                    return;
                }
            }
        }

        let new = self.state.cursor2.cursor_move_down();
        let new = self.cursor_check(&new);
        self.state.cursor2 = new;
    }

    pub fn cursor2_move_home(&mut self) {
        let new = self.state.cursor2.cursor_move_home();
        self.state.cursor2 = self.cursor_check(&new);
    }

    pub fn cursor2_move_end(&mut self) {
        let new = self.state.cursor2.cursor_move_end();
        self.state.cursor2 = self.cursor_check(&new);
    }

    pub fn patch_end(&self) -> usize {
        if self.state.top_line + self.patch_num >= self.pgh_views.len() {
            self.pgh_views.len()
        } else {
            self.state.top_line + self.patch_num
        }
    }

    pub fn current_range_clone(&self) -> Vec<(usize, PghView)> {
        let mut range = vec![];
        for (l, pgh_view) in self.pgh_views[self.state.top_line..self.patch_end()]
            .iter()
            .enumerate()
        {
            range.push((self.state.top_line + l, pgh_view.clone()));
        }
        range
    }

    fn current_cursor_pghviews(&self) -> Vec<(usize, &PghView)> {
        let mut range = vec![];
        if self.pgh_views.len() == 0 {
            return range;
        }
        let min = std::cmp::min(self.cursor1(), self.cursor2());
        let max = std::cmp::max(self.cursor1(), self.cursor2());
        let first = min.line_no.at_most(self.pgh_views.len() - 1);
        let last = max
            .line_no
            .at_least(first)
            .at_most(self.pgh_views.len() - 1);
        for (i, pgh_view) in self.pgh_views[first..last + 1].iter().enumerate() {
            range.push((first + i, pgh_view));
        }
        range
    }

    fn get_selected_raw_text(&self, is_raw: bool) -> String {
        let mut s = "".to_string();
        for (i, (line_no, pgh_view)) in self.current_cursor_pghviews().iter().enumerate() {
            let selected = pgh_view.select(*line_no, &self.cursor1(), &self.cursor2(), is_raw);
            if i > 0 {
                s += "\n"
            }
            s += &selected;
        }
        s
    }

    pub fn get_selected_text(&self) -> String {
        self.get_selected_raw_text(false)
    }

    pub fn get_all_text(&self) -> String {
        let mut s = "".to_string();
        let mut pre_pgh_type = PghType::UnKnown;
        for (line_no, pgh_view) in self.pgh_views.iter().enumerate() {
            let cursor1: Cursor = 0.into();
            let cursor2: Cursor = usize::MAX.into();
            let mut selected = pgh_view.select(line_no, &cursor1, &cursor2, false);
            if pgh_view.is_code() {
                let lang = pgh_view.code_lang.clone().unwrap_or_else(||"".to_string());
                selected = format!("```{}\n{}\n```\n", lang, selected);
            }
            if line_no > 0 {
                //insert "\n\n" between text line for markdown
                if self.cfg.is_markdown && (pre_pgh_type != pgh_view.pgh_type || pre_pgh_type == PghType::Text && pgh_view.pgh_type ==  PghType::Text) {
                    s += "\n\n"
                } else {
                    s += "\n"
                }
            }
            s += &selected;

            pre_pgh_type = pgh_view.pgh_type.clone();
        }
        s
    }

}

/// impl about update
///
impl Ctx {
    pub fn update_pgh(&mut self, line_no: usize, pghview: &PghView) {
        if let Some(org_pgh) = self.pgh_views.get_mut(line_no) {
            org_pgh.pgh_type = pghview.pgh_type.clone();
            org_pgh.pgh = pghview.pgh.clone();
            org_pgh.table_info = pghview.table_info.clone();
            org_pgh.spacing_top = pghview.spacing_top;
            org_pgh.spacing_bottom = pghview.spacing_bottom;
            org_pgh.code_lang = pghview.code_lang.clone();

            //don't set change_tick
            //org_pgh.change_tick = pghview.change_tick;    
        }
    }

    pub fn update_pgh_segment_job(&mut self, line_no: usize, segment: usize, job: Option<LayoutJob>) {
        if let Some(org_pgh) = self.pgh_views.get_mut(line_no) {
            if let Some(pgh_segment) = org_pgh.pgh.get_mut(segment) {
                pgh_segment.item.layout_job_update(job);
            }
        }
    }

    pub fn update_view(
        &mut self,
        line_no: usize,
        segment: usize,
        rect: Rect,
        char_rect: Vec<CharRect>,
    ) {
        if let Some(pgh_view) = self.pgh_views.get_mut(line_no) {
            pgh_view.update_view_info(segment, rect, char_rect);

            //todo
            //update cursors max_culumn
            if self.state.cursor1.line_no == line_no {
                self.state.cursor1 = self.cursor_check(&self.state.cursor1);
            }
            if self.state.cursor2.line_no == line_no {
                self.state.cursor2 = self.cursor_check(&self.state.cursor2);
            }
        }
    }

    pub fn update_spacing(&mut self, line_no: usize, spacing_top: f32, spacing_bottom: f32) {
        if let Some(pgh_view) = self.pgh_views.get_mut(line_no) {
            pgh_view.spacing_top = spacing_top;
            pgh_view.spacing_bottom = spacing_bottom
        }
    }

    pub fn update_view_mode(&mut self, dark_mode: bool) {
        if self.cfg.dark_mode != dark_mode {
            self.cfg.dark_mode = dark_mode;

            //force update all markdown lines: reset all pghview's change tick
            self.line_change_flash();
        }
    }

    pub fn update_segment_text(&mut self, line_no: usize, segment: usize, s: String) {
        if let Some(pgh_view) = self.pgh_views.get_mut(line_no) {
            pgh_view.update_segment_text(segment, s);
        }
    }

    pub fn truncate_segment(&mut self, line_no: usize, segment_num: usize) {
        if let Some(pgh_view) = self.pgh_views.get_mut(line_no) {
            if pgh_view.pgh.len() > segment_num {
                pgh_view.pgh.truncate(segment_num);
            }
        }
    }

    pub fn update_all_text(&mut self, line_no: usize, s: String) {
        if let Some(pgh_view) = self.pgh_views.get_mut(line_no) {
            pgh_view.update_all_text(s);
        }
    }

    pub fn is_table_line(&self, line_no: usize) -> bool {
        if let Some(pgh_view) = self.pgh_views.get(line_no) {
            return pgh_view.is_table();
        }
        false
    }

    pub fn get_line(&self, line_no: usize) -> Option<&PghView> {
        self.pgh_views.get(line_no)
    }

    pub fn get_line_mut(&mut self, line_no: usize) -> Option<&mut PghView> {
        self.pgh_views.get_mut(line_no)
    }

    pub fn get_line_clone(&mut self, line_no: usize) -> Option<PghView> {
        if let Some(pgh) = self.pgh_views.get(line_no) {
            Some(pgh.clone())
        } else {
            None
        }
    }

    pub fn delete_func(&mut self) -> (DoCmd, DoCmd) {
        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        undo_cmd.set_cursor(self.cursor2());
        redo_cmd.set_cursor(self.cursor1());

        if !self.is_selected() {
            return (undo_cmd, redo_cmd);
        }

        let mut line_set = vec![];
        for (line_no, pgh_view) in self.current_cursor_pghviews() {
            let after_delete = pgh_view.delete(line_no, &self.cursor1(), &self.cursor2());
            line_set.push((line_no, after_delete));
        }

        self.set_cursors_to_min();

        for (i, (line_no, after_delete)) in line_set.iter().enumerate() {
            println!("update {} to {:?}", line_no, after_delete);
            undo_cmd.push_update(*line_no, self.get_line_clone(*line_no));
            for (segment, s) in after_delete.iter().enumerate() {
                self.update_segment_text(*line_no, segment, s.to_string());
            }
            self.truncate_segment(*line_no, after_delete.len());
            redo_cmd.push_update(*line_no, self.get_line_clone(*line_no));
        }

        //delete empty lines
        let mut remain_lines = vec![];
        for (line_no, after_delete) in line_set.iter().rev() {
            let new_s = after_delete.join("");
            if new_s.len() == 0 {
                println!("delete line {}", *line_no);
                undo_cmd.push_insert(*line_no, self.get_line_clone(*line_no));
                self.pgh_views.remove(*line_no);
                redo_cmd.push_delete(*line_no);
            } else {
                remain_lines.push((*line_no, new_s, after_delete));
            }
        }
        //atleat remain one empty line
        if remain_lines.len() == 0 {
            let line_no = self.cursor2().line_no;
            undo_cmd.push_delete(line_no);
            self.insert_line(line_no, "".to_string());
            redo_cmd.push_insert(line_no, self.get_line_clone(line_no));
        }

        //merge remain normal lines
        if remain_lines.len() == 2 {
            println!("merge remain 2 lines");
            let (first_line_no, first_s, first_segments) = remain_lines.last().unwrap();
            let (_, last_s, _) = remain_lines.first().unwrap();
            let last_line_no = first_line_no + 1;
            if let Some(last) = self.pgh_views.get(last_line_no) {
                if let Some(first) = self.pgh_views.get(*first_line_no) {
                    if !last.is_table() && !first.is_table() {
                        let first_last_text = first_segments.last().unwrap();
                        let first_last_news = first_last_text.clone() + last_s;

                        undo_cmd.push_update(*first_line_no, self.get_line_clone(*first_line_no));
                        self.update_segment_text(
                            *first_line_no,
                            first_segments.len() - 1,
                            first_last_news,
                        );
                        redo_cmd.push_update(*first_line_no, self.get_line_clone(*first_line_no));

                        undo_cmd.push_insert(last_line_no, self.get_line_clone(last_line_no));
                        self.pgh_views.remove(last_line_no);
                        redo_cmd.push_delete(last_line_no);
                    }
                }
            }
        }
        println!("cursor after delete: {:?}", self.cursor2());
        return (undo_cmd, redo_cmd);
    }

    pub fn check_to_table_pghview(&mut self, s: &str) -> Option<PghView> {
        if !s.starts_with("|") {
            return None;
        }
        let markdown = MarkDownImpl::new(
            s,
            true,
            None,
            false,
            self.cfg()
        );
        let pghview = markdown.markdown_to_pghview();
        if pghview.is_table() {
            Some(pghview)
        } else {
            None
        }
    }

    pub fn on_change_to_table(&mut self) {
        let cursor = self.cursor2();
        let cur_text = self.get_line_text(cursor.line_no);
        let check_line = if cur_text.starts_with("|") {
            cursor.line_no
        } else if cur_text.is_empty() && cursor.line_no > 0 {
            cursor.line_no - 1
        } else {
            return;
        };

        //collect lines begin with |
        let mut top = vec![];
        for line in (0..=check_line).rev() {
            let txt = self.get_line_text(line);
            if txt.starts_with("|") {
                top.push((line, txt));
            } else {
                break;
            }
        }
        let mut bottom = vec![];
        for line in (check_line + 1)..self.pgh_views.len() {
            let txt = self.get_line_text(line);
            if txt.starts_with("|") {
                bottom.push((line, txt));
            } else {
                break;
            }
        }

        //join lines to one text
        let mut joins = "".to_string();
        let mut need_delete_lines = vec![];
        for (line, txt) in top.iter().rev() {
            joins += txt;
            joins += "\n";
            need_delete_lines.push(*line);
        }
        for (line, txt) in bottom.iter() {
            joins += txt;
            joins += "\n";
            need_delete_lines.push(*line);
        }
        println!("on_content_change table:[{}]", joins);

        //check is table markdown
        if let Some(table) = self.check_to_table_pghview(&joins) {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            undo_cmd.set_cursor(self.cursor2());
            println!("change to table [{}]", table.get_text());
            for i in need_delete_lines.iter().rev() {
                undo_cmd.push_insert(*i, self.get_line_clone(*i));
                self.pgh_views.remove(*i);
                redo_cmd.push_delete(*i);
            }
            let line = need_delete_lines.first().unwrap();
            undo_cmd.push_delete(*line);
            self.pgh_views.insert(*line, table);
            redo_cmd.push_insert(*line, self.get_line_clone(*line));
            
            //change cursor
            self.set_cursor2((*line).into());
            self.set_cursor1_reset();
            redo_cmd.set_cursor(self.cursor2());
            self.push_do(undo_cmd, redo_cmd);
        }
    }

    fn content_change_state(&mut self) {
        self.state.content_change_tick += 1;

        //clean same cache
        self.flash_same_cache_with_seleted();

        //flag need reset ime area
        self.set_ime_area_changed(true);
    }

    pub fn on_content_change(&mut self) {
        self.content_change_state();

        //check change text-lines to table
        let cursor = self.cursor2();
        if !self.is_table_line(cursor.line_no) {
            self.on_change_to_table();
        }
    }

    pub fn clean_change_tick(&mut self) {
        self.state.content_change_tick = 0;
    }

    pub fn is_content_changed(&self) -> bool {
        self.state.content_change_tick != 0
    }

    pub fn delete(&mut self) {
        let c1 = self.cursor1();
        let c2 = self.cursor2();

        if c1.line_no == c2.line_no && self.is_table_line(c1.line_no) {
            let (mut undo_cmd, mut redo_cmd) = self.delete_func();
            if let Some(pgh_view) = self.pgh_views.get_mut(c1.line_no) {
                undo_cmd.push_update(c1.line_no, Some(pgh_view.clone()));
                pgh_view.table_delete_empty_in_range(c1.segment, c2.segment);
                redo_cmd.push_update(c1.line_no, Some(pgh_view.clone()));
                redo_cmd.set_cursor(self.cursor2());
            }
            self.push_do(undo_cmd, redo_cmd);
        } else {
            let (undo_cmd, redo_cmd) = self.delete_func();
            self.push_do(undo_cmd, redo_cmd);
        }
        self.on_content_change();
    }

    fn insert_line(&mut self, line_no: usize, s: String) {
        println!("insert new line: {} {}", line_no, &s);
        let mut new_pgh_view = PghView::new_text();
        new_pgh_view.push_text(s, None);
        self.pgh_views.insert(line_no, new_pgh_view);
    }

    pub fn insert(&mut self, s: String) {
        let (mut undo_cmd, mut redo_cmd) = self.delete_func();

        let org_c: Cursor = self.cursor2();
        let mut new_c = org_c;

        println!("before insert: cursor={:?}", org_c);
        if let Some(pgh_view) = self.pgh_views.get_mut(org_c.line_no) {
            let (ls, rs, seg_text) = pgh_view.insert(&org_c, &s);
            if pgh_view.is_table() {
                undo_cmd.push_update(org_c.line_no, self.get_line_clone(org_c.line_no));
                if let Some(table) = self.check_to_table_pghview(&s) {
                    if let Some(pgh_mut) = self.get_line_mut(org_c.line_no) {
                        new_c.segment = pgh_mut.table_merge(org_c.segment, &table);
                    }
                } else {
                    self.update_segment_text(org_c.line_no, org_c.segment, seg_text);
                    new_c.culumn += s.chars().count();
                }
                redo_cmd.push_update(org_c.line_no, self.get_line_clone(org_c.line_no));
            } else if pgh_view.is_code() {
                undo_cmd.push_update(org_c.line_no, Some(pgh_view.clone()));
                new_c = pgh_view.code_insert(&org_c, &s);
                redo_cmd.push_update(org_c.line_no, Some(pgh_view.clone()));
            } else {
                let new_s = ls + &s + &rs;
                let lines: Vec<&str> = new_s.split('\n').collect();
                for (i, line) in lines.iter().enumerate() {
                    let line_no = org_c.line_no + i;
                    if i == 0 {
                        undo_cmd.push_update(line_no, self.get_line_clone(line_no));
                        self.update_all_text(line_no, line.to_string());
                        redo_cmd.push_update(line_no, self.get_line_clone(line_no));
                        println!("line={}", line);
                    } else {
                        undo_cmd.push_delete(line_no);
                        self.insert_line(line_no, line.to_string());
                        redo_cmd.push_insert(line_no, self.get_line_clone(line_no));
                    }

                    //set last line cursor
                    if i + 1 == lines.len() {
                        new_c.line_no = org_c.line_no + i;
                        new_c.segment = 0;
                        new_c.culumn = line.chars().count() - rs.chars().count();
                    }
                }
            }
            self.set_cursor2(new_c);
            self.set_cursor1_reset();
            println!("after insert: cursor={:?}", self.cursor2());
            redo_cmd.set_cursor(self.cursor2());
        }
        self.push_do(undo_cmd, redo_cmd);

        self.on_content_change();
    }

    pub fn update_line_text(&mut self, line_no: usize, s: String) {
        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        let bak_pghview = self.get_line_clone(line_no);
        if let Some(pgh_view) = self.pgh_views.get_mut(line_no) {
            undo_cmd.push_update(line_no, bak_pghview);
            pgh_view.update_all_text(s);
            redo_cmd.push_update(line_no, self.get_line_clone(line_no));
            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
        }
    }

    pub fn enter_auto_pak_ctrl(left: &str) -> String {
        let re = Regex::new(r"^-[ \t]+\[.*\] ").unwrap();
        if re.is_match(left) {
            return "- [ ] ".to_string()
        }

        if left.starts_with("- ") || left.starts_with("* ") || left.starts_with("> ") {
            let mut s = vec![];
            for x in left.chars() {
                if x == ' ' ||
                   x == '\t' ||
                   x == '-' ||
                   x == '*' ||
                   x == '[' ||
                   x == ']' ||
                   x == '>' {
                    s.push(x);
                } else {
                    break;
                } 
            }
            return s.iter().collect::<String>();
        }

        return "".to_string()
    }

    pub fn enter_insert(&mut self) {
        let (mut undo_cmd, mut redo_cmd) = self.delete_func();

        let c = self.cursor2();
        if let Some(pgh_view) = self.pgh_views.get_mut(c.line_no) {
            if pgh_view.is_table() {
                undo_cmd.push_update(c.line_no, Some(pgh_view.clone()));
                if let Some(cell) = pgh_view.table_segment_to_cell(c.segment) {
                    let new_segments = pgh_view.table_insert_row(cell.row + 1);
                    self.state.cursor2.segment += new_segments;
                }
                redo_cmd.push_update(c.line_no, Some(pgh_view.clone()));
            } else if pgh_view.is_code() {
                undo_cmd.push_update(c.line_no, Some(pgh_view.clone()));
                pgh_view.code_enter(&c);
                redo_cmd.push_update(c.line_no, Some(pgh_view.clone()));
                self.state.cursor2.segment += 1;
                self.state.cursor2.culumn = 0;
            } else {
                let (left, right) = pgh_view.normal_enter(&c);
                let begin_pak = Self::enter_auto_pak_ctrl(&left);
                undo_cmd.push_update(c.line_no, Some(pgh_view.clone()));
                pgh_view.update_all_text(left);
                redo_cmd.push_update(c.line_no, Some(pgh_view.clone()));

                //insert new line
                undo_cmd.push_delete(c.line_no + 1);
                let new_line = begin_pak.clone() + &right;
                self.insert_line(c.line_no + 1, new_line.clone());
                redo_cmd.push_insert(c.line_no + 1, self.get_line_clone(c.line_no + 1));
                self.state.cursor2 = self.state.cursor2.cursor_move_enter();
                self.state.cursor2.culumn += begin_pak.len();
            }
            self.set_cursor1_reset();
            redo_cmd.set_cursor(self.cursor2());
        }
        self.push_do(undo_cmd, redo_cmd);

        self.on_content_change();
    }

    pub fn enter(&mut self, ctrl: bool) {
        if ctrl {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            let c = self.cursor2();
            undo_cmd.push_delete(c.line_no + 1);
            undo_cmd.set_cursor(c);
            self.insert_line(c.line_no + 1, "".to_string());
            redo_cmd.push_insert(c.line_no + 1, self.get_line_clone(c.line_no + 1));

            self.state.cursor2 = 0.into();
            self.state.cursor2.line_no = c.line_no + 1;
            self.set_cursor1_reset();
            redo_cmd.set_cursor(self.cursor2());

            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
        } else {
            self.enter_insert();
        }
    }

    pub fn get_line_text(&self, line_no: usize) -> String {
        if let Some(pgh_view) = self.pgh_views.get(line_no) {
            pgh_view.get_text()
        } else {
            "".to_string()
        }
    }

    pub fn try_get_image_from_clipboard(&mut self) -> Option<String> {
        let uuid = Uuid::now_v7();
        if let Some(image_path) = &self.cfg.image_path {
            let file = format!("image_{}.png", uuid);
            let path = format!("{}/{}", image_path, file);
            if let Some(image_info) = ImageInfo::clipboard_to_file(&mut self.clipboard, "notitle".to_string(), file, path) {
                return Some(format!("![{}]({})", image_info.alt, image_info.url));
            }
        }
        None
    }
}

/// impl about layout info
///
impl Ctx {
    pub fn set_rect(&mut self, max_rect: Rect, line_no_width: f32, scroll_width: f32) {
        self.area.scroll_width = scroll_width;
        self.area.max_rect = max_rect;

        self.area.line_no_rect = max_rect;
        self.area.line_no_rect.set_width(line_no_width);

        self.area.edit_rect = max_rect;
        self.area.edit_rect.set_left(self.area.line_no_rect.right());
        self.area.edit_rect.set_right((max_rect.right() - scroll_width).max(self.area.edit_rect.left()));
        //self.area.edit_rect.set_bottom((max_rect.bottom() - scroll_width).max(self.area.edit_rect.top()));
        self.area.line_no_rect.set_bottom(self.area.edit_rect.bottom());
    }

    pub fn line_num(&self) -> usize {
        self.pgh_views.len()
    }

    pub fn top_line(&self) -> usize {
        self.state.top_line
    }

    pub fn set_scroll_to_line(&mut self, line: usize) {
        self.state.scroll_to_line = Some(line);
    }

    pub fn clean_scroll_to_line(&mut self) -> Option<usize>{
        let line = self.state.scroll_to_line.clone();
        self.state.scroll_to_line = None;
        line
    }

    pub fn set_scroll_to_rect(&mut self, rect: Rect) {
        self.state.scroll_to_rect = Some(rect);
    }

    pub fn clean_scroll_to_rect(&mut self) -> Option<Rect>{
        let rect = self.state.scroll_to_rect.clone();
        self.state.scroll_to_rect = None;
        rect
    }

    pub fn get_top_line_rect(&self) -> Option<Rect> {
        if let Some(pghview) = self.pgh_views.get(self.state.top_line) {
            if let Some(segment_rect) = pghview.get_segment_rect(0) {
                return Some(segment_rect)
            }
            pghview.rect()
        } else {
            None
        }
    }

    pub fn set_top_line(&mut self, top_line: usize) {
        if top_line < self.pgh_views.len() {
            self.state.top_line = top_line;
        }
    }

    pub fn bottom_line(&self) -> usize {
        self.state.bottom_line
    }

    pub fn bottom_pgh(&self) -> &PghView {
        if self.bottom_line() >= self.pgh_views.len() {
            if let Some(last) = self.pgh_views.last() {
                return last;
            }
        }
        return &self.pgh_views[self.bottom_line()];
    }

    pub fn set_bottom_line(&mut self, bottom: usize) {
        if bottom < self.state.top_line {
            return;
        }
        self.state.bottom_line = bottom;
    }

    pub fn left_top(&self) -> Pos2 {
        self.area.line_no_rect.left_top()
    }

    pub fn is_pos_in_edit_area(&self, pos: &Pos2) -> bool {
        let rect = self.edit_rect();
        if pos.x > rect.left()
            && pos.x < rect.right()
            && pos.y > rect.top()
            && pos.y < rect.bottom()
        {
            return true;
        }
        return false;
    }

    pub fn line_no_rect(&self) -> Rect {
        self.area.line_no_rect
    }

    pub fn edit_rect(&self) -> Rect {
        self.area.edit_rect
    }

    pub fn line_no_width(&self) -> f32 {
        self.area.line_no_rect.width()
    }

    pub fn edit_width(&self) -> f32 {
        self.area.edit_rect.width()
    }

    pub fn edit_right(&self) -> f32 {
        self.area.edit_rect.max.x
    }

    pub fn is_pointer_gone(&self) -> bool {
        self.state.is_pointer_gone
    }

    pub fn mark_pointer_gone(&mut self, is_gone: bool) {
        self.state.is_pointer_gone = is_gone;
    }

    pub fn set_ime_area_changed(&mut self, flag: bool) {
        self.state.ime_area_changed = flag;
    }

    pub fn is_ime_area_changed(&self) -> bool {
        self.state.ime_area_changed
    }

    pub fn font_size(&self) -> f32 {
        self.cfg.font_size
    }

    pub fn font_heigh(&self) -> f32 {
        self.cfg.font_heigh
    }

    pub fn set_font_heigh(&mut self, h: f32) {
        self.cfg.font_heigh = h
    }

    pub fn add_font_size(&mut self, delta: f32) {
        self.set_font_size(self.cfg.font_size + delta);
    }

    pub fn set_font_size(&mut self, size: f32) {
        self.cfg.font_size = size;
        if self.cfg.font_size < 6.0 {
            self.cfg.font_size = 6.0
        }
        //force flash all lines view
        self.line_change_flash();
    }

    pub fn set_open_time(&mut self) {
        let now = SystemTime::now();
        if let Ok(duration) = now.duration_since(UNIX_EPOCH) {
            self.open_time = duration.as_millis();
        }
    }

    pub fn get_open_time(&self) -> u128 {
        self.open_time
    }

    pub fn cfg(&self) -> &EditCfg {
        &self.cfg
    }

    pub fn cfg_mut(&mut self) -> &mut EditCfg {
        &mut self.cfg
    }
    
    pub fn sense(&self) -> Sense {
        Sense::click_and_drag()
    }
}

/// impl about state
///
impl Ctx {
    pub fn clone_state(&self) -> State {
        self.state.clone()
    }

    pub fn mark_state_change(&mut self, cmp_state: State) {
        if self.state != cmp_state {
            self.state_changed = true;
        } else {
            self.state_changed = false;
        }
    }

    pub fn is_state_changed(&self) -> bool {
        self.state_changed
    }

    pub fn check_switch_cursor_show(&mut self, milliseconds: u64) -> bool {
        if self.is_state_changed() || milliseconds < self.state.cursor_show_time {
            self.state.cursor_show_time = milliseconds;
            self.state.cursor_show_bool = true;
            return true;
        }
        let diff = milliseconds - self.state.cursor_show_time;
        if diff < 500 {
            return self.state.cursor_show_bool;
        } else {
            self.state.cursor_show_bool = !self.state.cursor_show_bool;
            self.state.cursor_show_time = milliseconds;
            return self.state.cursor_show_bool;
        }
    }

    pub fn mark_selecting(&mut self, selecting: bool) {
        //seleting done
        if self.state.selecting && selecting == false {
            self.line_change_flash();
        }

        self.state.selecting = selecting;
    }

    pub fn is_selecting(&self) -> bool {
        self.state.selecting
    }

    pub fn is_selected(&self) -> bool {
        self.state.cursor1 != self.state.cursor2
    }

    pub fn select_direction(&self) -> Option<f32> {
        if self.state.cursor2 > self.state.cursor1 {
            Some(1.0)
        } else if self.state.cursor2 < self.state.cursor1 {
            Some(-1.0)
        } else {
            None
        }
    }
}

/// command
impl Ctx {
    pub fn insert_cmd(&mut self, cmd: Command) {
        self.cmd_list.insert(0, cmd);      
    }

    pub fn pop_cmd(&mut self) -> Option<Command> {
        self.cmd_list.pop()    
    }

    pub fn insert_link_click_command(&mut self, link_info: LinkInfo) {
        match link_info {
            LinkInfo::File(file) => self.insert_cmd(Command::OpenFile(file)),
            LinkInfo::Link(url) => self.insert_cmd(Command::OpenUrl(url)),
            LinkInfo::Image(image) => {
                println!("todo: flash image: {:?}", image)
            },
        }
    }
}

/// impl about undo/redo
///
impl Ctx {
    pub fn line_change_tick(&mut self, line_no: usize) {
        if let Some(pghview) = self.pgh_views.get_mut(line_no) {
            pghview.change_tick += 1;
        }
    }

    pub fn line_change_reset(&mut self, line_no: usize) -> bool {
        let mut changed = true;
        if let Some(pghview) = self.pgh_views.get_mut(line_no) {
            changed = pghview.change_tick > 0;
            pghview.change_tick = 0;
        }
        changed
    }

    pub fn line_change_flash(&mut self) {
        for x in &mut self.pgh_views {
            x.change_tick += 1;
        }
    }

    pub fn push_do(&mut self, undo: DoCmd, redo: DoCmd) {
        for n in &redo.items {
            match n {
                DoItem::Insert(x) => self.line_change_tick(x.line),
                DoItem::Delete(x) => self.line_change_tick(x.line),
                DoItem::Update(x) => self.line_change_tick(x.line),
            }
        }

        self.do_mngr.do_list.insert(self.do_mngr.index,(undo, redo));
        self.do_mngr.index += 1;
        self.do_mngr.do_list.truncate(self.do_mngr.index);  
    }

    pub fn ondo_item(&mut self, do_item: &DoItem) {
        match do_item {
            DoItem::Insert(do_line) => {
                if let Some(pgh_view) = &do_line.pgh_view {
                    if do_line.line <= self.pgh_views.len() {
                        self.pgh_views.insert(do_line.line, pgh_view.clone());
                    }
                }
                self.line_change_tick(do_line.line);
                println!("Insert {} => {}", do_line.line, (do_line.pgh_view).clone().unwrap().get_text());
            }
            DoItem::Delete(do_line) => {
                if do_line.line < self.pgh_views.len() {
                    self.pgh_views.remove(do_line.line);
                }
                println!("Delete {}", do_line.line);
            }
            DoItem::Update(do_line) => {
                if let Some(pgh_view) = &do_line.pgh_view {
                    self.update_pgh(do_line.line, pgh_view);
                }
                self.line_change_tick(do_line.line);
                println!("Update {} => {}", do_line.line, (do_line.pgh_view).clone().unwrap().get_text())
            }
        }
    }

    pub fn ondo_list(&mut self, do_list: &DoCmd) {
        for do_item in &do_list.items {
            self.ondo_item(do_item);
        }
        self.set_cursor2(do_list.cursor);
        self.set_cursor1_reset();
    }

    pub fn undo(&mut self) {
        if self.do_mngr.index == 0 {
            //self.clean_change_tick();
            return;
        }
        self.do_mngr.index -= 1;
        if let Some((undo_list,_)) = self.do_mngr.do_list.get(self.do_mngr.index) {
            let mut rev_list = undo_list.clone();
            rev_list.items.reverse();
            self.ondo_list(&rev_list);
            self.content_change_state();
        }
    }

    pub fn redo(&mut self) {
        if let Some((_, redo_list)) = self.do_mngr.do_list.get(self.do_mngr.index) {
            self.ondo_list(&redo_list.clone());
            self.do_mngr.index += 1;
            self.content_change_state();
        }
    }
}

/// impl about find/replace
///
impl Ctx {
    fn is_word_boundary(s: &str, r: &std::ops::Range<usize>) -> bool {
        if r.start > s.len() || r.end > s.len() || r.start > r.end {
            return false;
        }

        let is_separator = |c:char| c.is_whitespace() || c.is_ascii_punctuation() && c != '_';
    
        // Convert string to char indices to handle multi-byte characters
        let mut char_indices = s.char_indices().peekable();
    
        // Find the character index just before the start of the range
        let mut prev_char_start = 0;
        while let Some((index, _)) = char_indices.peek() {
            if *index >= r.start {
                break;
            }
            prev_char_start = *index;
            char_indices.next();
        }
    
        // Find the character index at the end of the range
        let mut next_char_end = s.len();
        while let Some((index, _)) = char_indices.next() {
            if index > r.end {
                next_char_end = index;
                break;
            }
        }
    
        // Check if the start of the range is a word boundary
        let start_is_boundary = if r.start == 0 {
            true
        } else {
            let c = s[prev_char_start..r.start].chars().next().unwrap_or(' ');
            is_separator(c)
        };
    
        // Check if the end of the range is a word boundary
        let end_is_boundary = if r.end == s.len() {
            true
        } else {
            let c = s[r.end..next_char_end].chars().next().unwrap_or(' ');
            is_separator(c)
        };
    
        start_is_boundary && end_is_boundary
    }

    fn find_func(s: &str, param: &FindReplaceCtx) -> Vec<std::ops::Range<usize>> {
        if param.is_reg {
            if let Some(re) = &param.regex {
                re.find_iter(s)
                    .map(|mat| mat.range())
                    .filter(|r| !param.is_hole_word || Self::is_word_boundary(s, r))
                    .collect()
            } else {
                vec![]
            }
        } else {
            if param.is_case {
                s.match_indices(&param.find)
                    .map(|(start, _)| start..(start + param.find.len()))
                    .filter(|r| !param.is_hole_word || Self::is_word_boundary(s, r))
                    .collect()
            } else {
                let lower_s = s.to_lowercase();
                let lower_p = param.find.to_lowercase();
                lower_s.match_indices(&lower_p)
                    .map(|(start, _)| start..(start + lower_p.len()))
                    .filter(|r| !param.is_hole_word || Self::is_word_boundary(s, r))
                    .collect()
            }
        }
    }

    fn find_next_cursor(&mut self, param: &FindReplaceCtx) -> Option<(Cursor, Cursor)> {
        let cursor = self.cursor2();
        for line in cursor.line_no..self.pgh_views.len() {
            if let Some(pgh) = self.pgh_views.get(line) {
                let text = pgh.get_text();
                for found in Self::find_func(&text, param) {
                    let start_cursor = pgh.text_byte_index_to_cursor(found.start, line);
                    let end_cursor = pgh.text_byte_index_to_cursor(found.end, line);
                    if end_cursor > cursor {
                        return Some((start_cursor, end_cursor));
                    }
                }
            }
        }

        for line in 0..=cursor.line_no {
            if let Some(pgh) = self.pgh_views.get(line) {
                let text = pgh.get_text();
                for found in Self::find_func(&text, param) {
                    let start_cursor = pgh.text_byte_index_to_cursor(found.start, line);
                    let end_cursor = pgh.text_byte_index_to_cursor(found.end, line);
                    if start_cursor < cursor {
                        return Some((start_cursor, end_cursor));
                    }
                }
            }
        }
        
        None
    }

    pub fn find_and_select(&mut self, param: &FindReplaceCtx) -> bool {
        if param.find.is_empty() {
            return false
        }
        if let Some((c1, c2)) = self.find_next_cursor(param) {
            self.set_cursor1(c1);
            self.set_cursor2(c2);
            true
        } else {
            false
        }
    }

    fn find_all_func(&mut self, param: &FindReplaceCtx, need_text: bool, from_line: usize, end_line: usize) -> FindCache {
        let mut find_cache = FindCache::new();
        let mut list = vec![];
        for line_no in from_line..end_line {
            if let Some(pgh) = self.pgh_views.get(line_no) {
                let text = pgh.get_text();
                for found in Self::find_func(&text, param) {
                    let start = pgh.text_byte_index_to_cursor(found.start, line_no);
                    let end = pgh.text_byte_index_to_cursor(found.end, line_no);
                    let item = FindCacheItem {
                        start,
                        end,
                        line_text: if need_text { Some(text.clone()) } else { None },
                    };

                    list.push(item);
                }
            }
        }
        find_cache.cache = list;
        find_cache
    }

    pub fn find_all(&mut self, param: &FindReplaceCtx) {
        if param.find.is_empty() {
            return;
        }
        self.find_cache = self.find_all_func(param, true, 0, self.pgh_views.len());
        self.find_param = param.clone();
    }

    pub fn get_find_cache(&mut self) -> (&FindCache, &FindReplaceCtx) {
        (&self.find_cache, &self.find_param)
    }

    pub fn flash_same_cache_with_seleted(&mut self) {
        if self.cfg.hightlight_seleted_word {
            if self.is_selected() && self.state.cursor1.line_no == self.state.cursor2.line_no {
                let select_text = self.get_selected_raw_text(true);
                let param = FindReplaceCtx::sample(select_text);
                let from_line = self.top_line().saturating_sub(100);
                let end_line = self.bottom_line().add(100).min(self.pgh_views.len());
                self.same_cache = self.find_all_func(&param, false, from_line, end_line);
            } else {
                self.same_cache.cache = vec![]
            }
        }
    }

    pub fn flash_same_cache_with_param(&mut self, param: &FindReplaceCtx) {
        self.same_cache = self.find_all_func(param, false, 0, self.pgh_views.len());
    }
}


/// impl about hightlight
///
impl Ctx {
    pub fn set_height_lang(&mut self, lang: Option<String>) {
        self.cfg.lang = lang
    }

    pub fn highlight_range_text(&self) -> (usize, String) {
        let top = self.state.top_line.saturating_sub(20);
        let text = self.pgh_views[top..self.patch_end()]
            .iter()
            .map(|x|x.get_text())
            .collect::<Vec<_>>().join("\n");
        
        (top, text)
    }

    pub fn highlight_refresh(&mut self, ui: &Ui) {
        if self.cfg.is_markdown {
            return;
        }
        if let Some(code_lang) = &self.cfg.lang {
            let (top, text) = self.highlight_range_text();
            let source = text.as_bytes();
            if let Ok(lines) = highlight_lines(code_lang.clone(), source) {
                for (lno, line) in lines.iter().enumerate() {
                    if line.len() > 0 {
                        let mut job: LayoutJob = LayoutJob::default();
                        for slice in line {
                            job.append(&String::from_utf8_lossy(slice.slice), 0.0, 
                                PghView::code_format(slice, ui, self));
                        }
                        self.update_pgh_segment_job(top+lno, 0, Some(job));
                    } else {
                        self.update_pgh_segment_job(top+lno, 0, None);
                    }
                }
            }
        }
    }
}


#[test]
pub fn test_is_word_boundary() {
    assert_eq!(Ctx::is_word_boundary("hello abc world", &(0..5)), true);
    assert_eq!(Ctx::is_word_boundary("hello abc world", &(6..9)), true);
    assert_eq!(Ctx::is_word_boundary("hello abc world", &(10..15)), true);
    assert_eq!(Ctx::is_word_boundary("hello abc world中", &(10..15)), false);
    assert_eq!(Ctx::is_word_boundary("hello abc world中", &(0..4)), false);
    assert_eq!(Ctx::is_word_boundary("hello abc world中", &(1..5)), false);
}
