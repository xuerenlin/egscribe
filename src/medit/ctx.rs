use core::f32;
use std::ops::Add;
use std::sync::atomic::{AtomicU64, Ordering};
use std::vec;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

static CTX_SCROLL_AREA_ID_SEQ: AtomicU64 = AtomicU64::new(1);

use crate::sitter::highlight_lines;
use crate::medit::{CodeInfo, ImageInfo, LinkInfo, PghType, CharRect, Cursor, MarkDownImpl, SegmentType, PghView,
    DoItem, DoCmd, DoMngr, Action, FindReplaceCtx, MarkdownOutline, MergeRedoAndUndoGuard,
    TocCache, TocEntry, cfg::HeightMode};
use crate::util::{enc_content, dec_content};
use eframe::egui::{NumExt, Pos2, Rect, Sense, Ui, Frame};
use eframe::egui::epaint::text::LayoutJob;
use regex::Regex;
use arboard::Clipboard;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::UrlInfo;
use super::scroll_layout::{EditScrollLayout, scroll_line_height_estimate};

#[derive(Clone, Debug)]
pub struct State {
    top_line: usize,
    bottom_line: usize,
    scroll_to_line: Option<usize>,
    scroll_to_rect: Option<Rect>,

    cursor1: Cursor,
    cursor2: Cursor,
    cursor2_bak: Option<Cursor>,
    cursor_show_time: u64, //milliseconds
    cursor_show_bool: bool,
    selecting: bool,

    change_current_tick: u64,
    change_last_save_tick: u64,
    ime_area_changed: bool,
    ime_actived: bool,
    ime_preedit_selected: bool,
    last_auto_scroll_time: u64, //milliseconds, 上次自动调整光标的时间
}

impl PartialEq for State {
    /// 只比较内容和光标的变化，忽略UI状态（如滚动位置、光标闪烁等）
    fn eq(&self, other: &Self) -> bool {
        self.change_current_tick == other.change_current_tick
            && self.cursor1 == other.cursor1
            && self.cursor2 == other.cursor2
            && self.selecting == other.selecting
    }
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
            cursor2_bak: None,
            cursor_show_time: 0,
            cursor_show_bool: true,
            selecting: false,
            change_current_tick: 0,
            change_last_save_tick: 0,
            ime_area_changed: false,
            ime_actived: false,
            ime_preedit_selected: false,
            last_auto_scroll_time: 0,
        }
    }
}

pub struct Area {
    max_rect: Rect,
    line_no_rect: Rect,
    divider_rect: Rect,
    edit_rect: Rect,
    scroll_width: f32,
}

impl Default for Area {
    fn default() -> Self {
        Area {
            max_rect: Rect::ZERO,
            line_no_rect: Rect::ZERO,
            divider_rect: Rect::ZERO,
            edit_rect: Rect::ZERO,
            scroll_width: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FindCacheItem{
    pub start: Cursor,
    pub end: Cursor,
    pub line_text: Option<String>,
}

#[derive(Clone)]
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

// EditCfg and EditColors moved to cfg.rs
use super::cfg::EditCfg;

pub enum HighlightRect {
    Select(Rect),
    SameText(Rect),
}

pub struct ExpandedCtx {
    current_id: u64,
    ctx_map: HashMap<u64, Box<Ctx>>,
    order: Vec<u64>, 
}

impl ExpandedCtx {
    pub fn new() -> Self {
        Self {
            current_id: 0,
            ctx_map: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn current_id(&self) -> u64 {
        self.current_id
    }

    pub fn ctx(&self, id: u64) -> Option<&Ctx> {
        self.ctx_map.get(&id).map(|c| c.as_ref())
    }

    pub fn ctx_mut(&mut self, id: u64) -> Option<&mut Ctx> {
        self.ctx_map.get_mut(&id).map(|c| c.as_mut())
    }

    pub fn set_ctx(&mut self, ctx: Ctx) -> u64 {
        self.current_id += 1;
        let new_id = self.current_id;
        
        // 如果已经有8个，删除最老的（order 中的第一个）
        const MAX_SIZE: usize = 8;
        if self.ctx_map.len() >= MAX_SIZE {
            if let Some(oldest_id) = self.order.first().copied() {
                self.ctx_map.remove(&oldest_id);
                self.order.remove(0);
            }
        }
        
        // 插入新的 ctx
        self.ctx_map.insert(new_id, Box::new(ctx));
        self.order.push(new_id);
        
        new_id
    }

    pub fn clear(&mut self) {
        self.ctx_map.clear();
        self.order.clear();
    }

    pub fn has_ctx(&self) -> bool {
        !self.ctx_map.is_empty()
    }

    pub fn has_id(&self, id: u64) -> bool {
        self.ctx_map.contains_key(&id)
    }
}

pub struct Ctx {
    cfg: EditCfg,
    pgh_views: Vec<PghView>,
    patch_num: usize,
    scroll_layout: EditScrollLayout,
    state: State, //mark somthing has changed after on_event
    state_cmp: State, 
    area: Area,
    /// 每个编辑器实例唯一，用于 egui ScrollArea 的 id_salt，避免多标签共用滚动记忆
    scroll_area_id: u64,
    open_time: u128,
    do_mngr: Rc<RefCell<DoMngr>>,
    cmd_list: Vec<Action>,
    find_cache: FindCache,
    find_param: FindReplaceCtx,
    same_cache: FindCache,
    clipboard: Clipboard,
    expanded_ctx: ExpandedCtx,
    view_height: Option<f32>, // 保存上次布局后的高度，用于下一帧显示设置动态高度
    toc_cache: TocCache,
}

impl Ctx {
    /// Create a new Ctx with default configuration
    pub fn new() -> Self {
        let font_size = 17.0;
        let mut ctx = Self {
            cfg: EditCfg::new(font_size, false, None, HeightMode::fix_max()),
            pgh_views: vec![],
            patch_num: 80,
            scroll_layout: EditScrollLayout::new(),
            state: State::default(),
            state_cmp: State::default(),
            area: Area::default(),
            scroll_area_id: CTX_SCROLL_AREA_ID_SEQ.fetch_add(1, Ordering::Relaxed),
            open_time: 0, 
            do_mngr: Rc::new(RefCell::new(DoMngr::new())),
            cmd_list: vec![],
            find_cache: FindCache::new(),
            find_param: FindReplaceCtx::new(),
            same_cache: FindCache::new(),
            clipboard: Clipboard::new().unwrap(),   //todo: unwrap unsafe
            expanded_ctx: ExpandedCtx::new(),
            view_height: None,
            toc_cache: TocCache::new(),
        };
        // Initialize with empty text
        ctx.pgh_views = MarkDownImpl::new("", false, None, false, ctx.cfg())
            .markdown_to_pgh_texts();
        ctx
    }

    /// Set text content and reparse with specified markdown mode
    pub fn with_text(mut self, text: &str, is_markdown: bool) -> Self {
        self.cfg.is_markdown = is_markdown;
        let markdown_impl = MarkDownImpl::new(
            text,
            is_markdown,
            None,
            false,
            self.cfg()
        );
        self.pgh_views = markdown_impl.markdown_to_pgh_texts();
        self.scroll_cum_invalidate_full();
        self.toc_replace_entries();
        self
    }

    pub fn image_path(mut self, image_path: Option<String>) -> Self {
        self.cfg.image_path = image_path;
        self
    }

    pub fn height_mode(mut self, height_mode: HeightMode) -> Self {
        self.cfg.height_mode = height_mode;
        self
    }

    pub fn show_line_no(mut self, show_line_no: bool) -> Self {
        self.cfg.show_line_no = show_line_no;
        self
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.cfg.wrap = wrap;
        self
    }

    pub fn show_heading_section_numbers(mut self, show: bool) -> Self {
        self.cfg.show_heading_section_numbers = show;
        self
    }

    pub fn dark_mode(mut self, dark_mode: bool) -> Self {
        self.cfg.dark_mode = dark_mode;
        self
    }

    pub fn set_font_size_chain(mut self, font_size: f32) -> Self {
        self.set_font_size(font_size);
        self
    }

    pub fn indent_size(mut self, indent_size: f32) -> Self {
        self.set_indent_size(indent_size);
        self
    }

    pub fn list_item_indent_size(mut self, list_item_indent_size: f32) -> Self {
        self.set_list_item_indent_size(list_item_indent_size);
        self
    }

    pub fn text_color_brightness(mut self, text_color_brightness: f32) -> Self {
        self.cfg.text_color_brightness = text_color_brightness;
        self
    }

    pub fn need_line_click_cmd(mut self, need_line_click_cmd: bool) -> Self {
        self.cfg.need_line_click_cmd = need_line_click_cmd;
        self
    }

    pub fn hightlight_seleted_word(mut self, hightlight_seleted_word: bool) -> Self {
        self.cfg.hightlight_seleted_word = hightlight_seleted_word;
        self
    }

    pub fn read_only(mut self, is_read_only: bool) -> Self {
        self.cfg.is_read_only = is_read_only;
        self
    }

    pub fn monospace(mut self, is_monospace: bool) -> Self {
        self.cfg.is_monospace = is_monospace;
        self
    }

    pub fn with_cfg(mut self, cfg: &EditCfg) -> Self {
        self.cfg = cfg.clone();
        self
    }

    pub fn with_frame(mut self, frame: Frame) -> Self {
        self.cfg.with_frame = Some(frame);
        self
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
            } else if pgh_view.is_pos_left(pos) {
                let segment = pgh_view.first_same_y_segment(pos);
                return Some((top_line + i, segment, 0).into());
            } else if pgh_view.is_pos_right(pos) {
                let mut segment = pgh_view.last_same_y_segment(pos);
                let mut culumn = pgh_view.max_culumn(&(top_line+1, segment, 0).into());
                //the last segment is not normal text (it has not culumns)
                log::debug!("segment={} culumn={}", segment, culumn);
                if segment > 0 && culumn == 0 {
                    segment -= 1;
                    culumn = pgh_view.max_culumn(&(top_line+1, segment, 0).into());
                }
                return Some((top_line + i, segment, culumn).into());
            }
        }
        None
    }

    //判断pos是否在SegmentType::Icon或者SegmentType::CheckBox rect内，用于鼠标点击不切换光标
    pub fn is_pos_in_icon_or_checkbox(&self, pos: &Pos2) -> bool {
        let top_line = self.top_line();
        for (i, pgh_view) in self.pgh_views[top_line..self.patch_end()]
            .iter()
            .enumerate()
        {
            let line_no = top_line + i;
            // 遍历该行的所有 segment
            for segment in 0..pgh_view.pgh.len() {
                let seg_type = pgh_view.get_segment_type(segment);
                if seg_type == SegmentType::Icon || seg_type == SegmentType::CheckBox {
                    if let Some(rect) = pgh_view.get_segment_rect(segment) {
                        if rect.contains(*pos) {
                            return true;
                        }
                    }
                }
            }
        }
        false
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
        let changed = if let Some(cur_bak) = self.state.cursor2_bak {
            if cur_bak != self.state.cursor2 {true} else {false}
        } else {
            true
        };
        self.state.cursor2_bak = Some(self.state.cursor2);
        changed
    }

    pub fn cursor2_cmp_reset(&mut self) {
        self.state.cursor2_bak = None;
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
            if pgh_view.is_table() || pgh_view.is_table_row() {
                return pgh_view.table_range_rect(min.segment, max.segment);
            }
        }
        None
    }

    fn get_code_cursor_left(&self, min: &Cursor, max: &Cursor) -> Option<f32> {
        if min.line_no != max.line_no {
            return None;
        }
        if let Some(pgh_view) = self.pgh_views.get(min.line_no) {
            if pgh_view.is_code_row() {
                return Some(self.left_top().x + self.line_no_width() + self.cfg.indent_size);
            }
        }
        None
    }

    pub fn get_line_rect(&self, line_no: usize) -> Option<Rect> {
        if let Some(pghview) = self.pgh_views.get(line_no) {
            pghview.rect()
        } else {
            None
        }
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

    pub fn get_heighlight_rects(&self) -> Option<Vec<HighlightRect>> {
        let mut rects = vec![];
        if self.is_selected() {
            if let Some(vec_rc) = self.get_crange_rects(self.state.cursor1, self.state.cursor2) {
                for rc in vec_rc {
                    rects.push(HighlightRect::Select(rc));
                }
            }
        }

        if !self.same_cache.cache.is_empty() {
            let min = std::cmp::min(self.state.cursor1, self.state.cursor2);
            let max = std::cmp::max(self.state.cursor1, self.state.cursor2);
            for rc in &self.same_cache.cache {
                let same_min = std::cmp::min(rc.start, rc.end);
                let same_max = std::cmp::max(rc.start, rc.end);
                // 判断min/max和same_min/same_max是否存在交集，如果存在交集，则不添加到rects中
                if max >= same_min && same_max >= min {
                    continue;
                }
                if same_min.line_no() < self.top_line() {
                    continue;
                }
                if same_max.line_no() > self.bottom_line() {
                    break;
                }
                if let Some(vec_rc) = self.get_crange_rects(same_min, same_max) {
                    for rc in vec_rc {
                        rects.push(HighlightRect::SameText(rc));
                    }
                }
            }
        }
        Some(rects)
    }

    fn get_crange_rects(&self, c1: Cursor, c2: Cursor) -> Option<Vec<Rect>> {
        if c1 == c2 {
            return None;
        }
        let orig_min = std::cmp::min(c1, c2);
        let orig_max = std::cmp::max(c1, c2);
        if let Some((line_lo, line_hi, col_lo, col_hi)) =
            self.table_row_block_column_rect(&orig_min, &orig_max)
        {
            let mut rects = vec![];
            for ln in line_lo..=line_hi {
                if let Some(p) = self.pgh_views.get(ln) {
                    for col in col_lo..=col_hi {
                        if let Some(r) = p.get_segment_rect(col) {
                            rects.push(r);
                        }
                    }
                }
            }
            if !rects.is_empty() {
                return Some(rects);
            }
        }

        let mut min = orig_min;
        let max = orig_max;
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
                if let Some(code_left) = self.get_code_cursor_left(&min, &max){
                    left = code_left;
                }

                if (min_rect.min.y - max_rect.min.y).abs() < self.font_heigh()/2.0 {
                    //the same line
                    let min = Pos2::new(min_rect.min.x, min_rect.min.y.min(max_rect.min.y));
                    let max = Pos2::new(max_rect.max.x, min_rect.max.y.max(max_rect.max.y));
                    rects.push(Rect::from_min_max(min, max));
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
        let delimiters = " \t~`!@#$%^&()+-=[]\\{}|;':\",./<>?，。、；：‘’“”";

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
        let old = self.state.cursor2;
        if let Some(pgh_view) = self.cursor_pghview(&self.state.cursor2) {
            let new = self.state.cursor2.cursor_move_prev(pgh_view);
            let mut new = self.cursor_check(&new);
            // `TableRow` 行首跨到上一行：`cursor_move_prev` 落在末格「尾后」(culumn==len)，
            // 与段内退格落在「待删字符」上的语义不一致，会导致退格选区为空。对齐到末字符。
            if old.line_no > 0
                && old.segment == 0
                && old.culumn == 0
                && pgh_view.is_table_row()
                && new.line_no + 1 == old.line_no
            {
                if let Some(prev) = self.get_line(new.line_no) {
                    let same_block = prev.is_table_row()
                        && prev
                            .table_info
                            .as_ref()
                            .zip(pgh_view.table_info.as_ref())
                            .is_some_and(|(a, b)| a.col_count == b.col_count);
                    if same_block && new.culumn > 0 {
                        new.culumn -= 1;
                        new = self.cursor_check(&new);
                    }
                }
            }
            self.state.cursor2 = new;
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
        let len = self.pgh_views.len();
        if len == 0 {
            return 0;
        }
        let end = self.scroll_layout.layout_patch_end.min(len);
        end.max(self.state.top_line.saturating_add(1))
            .min(len)
    }

    pub(crate) fn set_layout_patch_end(&mut self, end: usize) {
        self.scroll_layout.layout_patch_end = end;
    }

    pub(crate) fn patch_num(&self) -> usize {
        self.patch_num
    }

    /// 兜底保证编辑器至少有一行，返回是否发生了插入。
    pub(crate) fn ensure_non_empty_line_for_layout(&mut self) -> bool {
        if !self.pgh_views.is_empty() {
            return false;
        }
        self.insert_line(0, String::new());
        self.state.top_line = 0;
        self.state.bottom_line = 0;
        self.state.cursor1 = 0.into();
        self.state.cursor2 = 0.into();
        self.scroll_cum_invalidate_full();
        true
    }

    /// 未测量行用 `font_heigh`，否则用上一帧实际高度
    pub(crate) fn line_scroll_height_estimate(&self, line: usize) -> f32 {
        scroll_line_height_estimate(&self.pgh_views, line, self.font_heigh())
    }

    pub(crate) fn scroll_cum_invalidate_full(&mut self) {
        self.scroll_layout.invalidate_cum_full();
    }

    /// 保证前缀和与当前行数、字高及各行高度估计一致。
    pub(crate) fn ensure_scroll_cumulative_offsets(&mut self) {
        let n = self.line_num();
        let fh = self.font_heigh();
        self.scroll_layout
            .ensure_cumulative_offsets(n, fh, &self.pgh_views);
    }

    #[inline]
    pub(crate) fn scroll_cum_at(&self, i: usize) -> f32 {
        self.scroll_layout.cum_at(i)
    }

    pub(crate) fn scroll_bottom_padding(&self) -> f32 {
        if self.is_dynamic_height() {
            0.0
        } else {
            (self.edit_rect().height() / 2.0).max(0.0)
        }
    }

    /// 须先调用 [`Self::ensure_scroll_cumulative_offsets`]
    pub(crate) fn scroll_offset_y_for_line(&self, line_no: usize) -> f32 {
        let n = self.line_num();
        let line_no = line_no.min(n);
        self.scroll_cum_at(line_no)
    }

    pub(crate) fn record_line_scroll_height_after_layout(&mut self, line_no: usize) {
        if let Some(pv) = self.pgh_views.get_mut(line_no) {
            if let Some(r) = pv.rect() {
                let nh = r.height().at_least(1.0);
                let oh = pv.last_scroll_height;
                if oh > 0.0 && (nh - oh).abs() < 0.5 {
                    return;
                }
                pv.last_scroll_height = nh;
                let n = self.line_num();
                self.scroll_layout.note_line_height_changed(line_no, n);
            }
        }
    }

    /// 须先 [`Self::ensure_scroll_cumulative_offsets`]
    pub(crate) fn scroll_lines_visible_for_viewport(
        &self,
        viewport: &Rect,
        margin: f32,
    ) -> (usize, usize) {
        let n = self.line_num();
        self.scroll_layout
            .lines_visible_for_viewport(viewport, margin, n)
    }

    pub fn current_range(&self) -> std::ops::Range<usize> {
        self.state.top_line..self.patch_end()
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

    pub fn get_line_pghview(&self, line_no: usize) -> Option<&PghView> {
        self.pgh_views.get(line_no)
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
        let tr_rect = self.table_row_block_column_rect(&self.cursor1(), &self.cursor2());
        if let Some((lo, hi, cl, ch)) = tr_rect {
            if !is_raw {
                return self.table_row_block_column_copy_markdown(lo, hi, cl, ch);
            }
            let mut s = String::new();
            for (i, line_no) in (lo..=hi).enumerate() {
                if let Some(p) = self.get_line(line_no) {
                    if i > 0 {
                        s.push('\n');
                    }
                    s.push_str(&p.select_table_row_column_block(
                        line_no,
                        &self.cursor1(),
                        &self.cursor2(),
                        lo,
                        hi,
                        cl,
                        ch,
                        true,
                    ));
                }
            }
            return s;
        }
        if let Some(code_sel) =
            self.code_row_block_selected_markdown(&self.cursor1(), &self.cursor2(), is_raw)
        {
            return code_sel;
        }
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

    fn get_line_select_text(&self, line_no: usize) -> String {
        if let Some(pgh_view) = self.pgh_views.get(line_no) {
            let cursor1: Cursor = (line_no, 0, 0).into();
            let max_segment = pgh_view.max_segment();
            let cursor2: Cursor = (line_no, max_segment, usize::MAX).into();
            pgh_view.select(line_no, &cursor1, &cursor2, false)
        } else {
            "".to_string()
        }
    }

    pub fn get_all_text(&self) -> String {
        let mut s = String::new();
        let cursor1: Cursor = 0.into();
        let cursor2: Cursor = usize::MAX.into();
        let mut line_no = 0usize;
        while line_no < self.pgh_views.len() {
            let pgh_view = &self.pgh_views[line_no];
            if self.cfg().is_markdown {
                if pgh_view.is_code_row() {
                    let (blk_s, blk_e) = self
                        .code_row_block_range(line_no)
                        .unwrap_or((line_no, line_no));
                    let lang = self
                        .get_line(blk_s)
                        .and_then(|p| p.code_lang.clone())
                        .unwrap_or_default();
                    let mut body = String::new();
                    for ln in blk_s..=blk_e {
                        if ln > blk_s {
                            body.push('\n');
                        }
                        body.push_str(&self.pgh_views[ln].get_text());
                    }
                    let selected = format!("```{}\n{}\n```", lang, body);
                    if line_no > 0 {
                        s.push('\n');
                    }
                    s.push_str(&selected);
                    line_no = blk_e + 1;
                    continue;
                } else {
                    let selected = pgh_view.select(line_no, &cursor1, &cursor2, false);
                    if line_no > 0 {
                        s.push('\n');
                    }
                    s.push_str(&selected);
                    line_no += 1;
                }
            } else {
                if line_no > 0 {
                    s.push('\n');
                }
                s += &pgh_view.select(line_no, &cursor1, &cursor2, true);
                line_no += 1;
            }
        }
        s
    }

    pub fn get_selected_status_info(&self) -> String {
        let cursor2 = self.cursor2();
        let mut cursor_text = format!("Ln {},Col {}", cursor2.line_no + 1, cursor2.culumn);
        if self.is_selected() {
            let cursor1 = self.cursor1();
            let start_line = cursor1.line_no.min(cursor2.line_no);
            let end_line = cursor1.line_no.max(cursor2.line_no);
            let selection_text = if start_line == end_line {
                format!("{} chars",  self.get_selected_text().len())
            } else {
                format!("{} lines", end_line - start_line + 1)
            };
            cursor_text = format!("{} ({} selected)", cursor_text, selection_text);
        } 
        cursor_text
    }

    pub fn get_view_height(&self) -> f32 {
        match self.cfg().height_mode {
            HeightMode::Fixed(height) => {
                if height == f32::INFINITY {
                    self.edit_rect().height()
                } else {
                    height
                }
            }
            HeightMode::Dynamic { min: _, max } => {
                self.view_height.unwrap_or(max)
            }
        }
    }

    /// This should be called after layout/rendering to adjust height based on actual content
    pub fn set_height_mode_to_actually(&mut self) {
        let actual_view_height = self.get_view_height();
        if let HeightMode::Dynamic { min, max } = self.cfg().height_mode {
            self.cfg_mut().height_mode = HeightMode::Dynamic { min, max: actual_view_height }
        }
    }

    pub fn is_dynamic_height(&self) -> bool {
        matches!(self.cfg().height_mode, HeightMode::Dynamic { .. })
    }

    /// 获取保存的视图高度
    pub fn saved_view_height(&self) -> Option<f32> {
        self.view_height
    }

    /// 设置保存的视图高度
    pub fn set_saved_view_height(&mut self, height: Option<f32>) {
        self.view_height = height;
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
            org_pgh.code_info = pghview.code_info.clone();
            //org_pgh.expanded_text_id = pghview.expanded_text_id; 
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
            self.line_flash_all();
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

    pub fn set_expanded_text(&mut self, line_no: usize, expanded_text: Option<String>) {
        // 处理清除情况（None 或空字符串）
        let text = match expanded_text {
            Some(ref t) if !t.is_empty() => t,
            _ => {
                self.expanded_ctx.clear();
                if let Some(pgh_view) = self.pgh_views.get_mut(line_no) {
                    pgh_view.expanded_text_id = None;
                }
                self.on_content_change();
                self.line_flash_tick(line_no);
                return;
            }
        };

        let cfg_clone = self.cfg().clone();
        let expanded_text_id = self.pgh_views.get(line_no)
            .and_then(|pgh_view| pgh_view.expanded_text_id);
        
        // 检查 expanded_text_id 是否在 map 中
        let is_exists_ctx = expanded_text_id
            .map(|id| self.expanded_ctx().has_id(id))
            .unwrap_or(false);

        if is_exists_ctx {
            if let Some(id) = expanded_text_id {
                self.update_existing_expanded_ctx(text, id);
            }
        } else {
            let new_id = self.create_new_expanded_ctx(text, &cfg_clone);
            if let Some(pgh_view) = self.pgh_views.get_mut(line_no) {
                pgh_view.expanded_text_id = Some(new_id);
            }
        }

        self.on_content_change();
        self.line_flash_tick(line_no);
    }

    /// 更新现有的 expanded_ctx 内容
    fn update_existing_expanded_ctx(&mut self, text: &str, id: u64) {
        let expanded_ctx = match self.expanded_ctx_mut().ctx_mut(id) {
            Some(ctx) => ctx,
            None => return,
        };
        // 直接解析 markdown 并设置 pgh_views
        let markdown_impl = MarkDownImpl::new(
            text,
            true,
            None,
            false,
            expanded_ctx.cfg()
        );
        expanded_ctx.pgh_views = markdown_impl.markdown_to_pgh_texts();
        expanded_ctx.scroll_cum_invalidate_full();
        expanded_ctx.toc_replace_entries();

        // 重置光标到开始位置
        expanded_ctx.set_cursor1(0.into());
        expanded_ctx.set_cursor2(0.into());
        
        // 触发内容变化
        expanded_ctx.on_content_change();
    }

    /// 创建新的 expanded_ctx
    fn create_new_expanded_ctx(&mut self, text: &str, cfg: &EditCfg) -> u64 {
        let frame = Frame::default();
            //.stroke(Stroke::new(1.0, cfg.colors().weak_color))
            //.corner_radius(3.0)
            //.fill(cfg.colors().code_bg_color);
            //.outer_margin(Vec2::splat(3.0));

        let nex_ctx = Ctx::new()
            .with_text(text, true)
            .with_cfg(cfg)
            .read_only(true)
            .monospace(true)   //expanded ctx always use monospace font
            .show_line_no(false)
            .height_mode(HeightMode::dynamic_range(cfg.font_heigh, cfg.font_heigh))
            .with_frame(frame);
        
        self.expanded_ctx.set_ctx(nex_ctx)
    }

    pub fn is_table_line(&self, line_no: usize) -> bool {
        if let Some(pgh_view) = self.pgh_views.get(line_no) {
            return pgh_view.is_table_like();
        }
        false
    }

    /// 与 `line_no` 同属一块的连续 `TableRow`（列数相同）的 `[start, end]` 行号（含端点）
    pub fn table_row_block_range(&self, line_no: usize) -> Option<(usize, usize)> {
        let p = self.get_line(line_no)?;
        if !p.is_table_row() {
            return None;
        }
        let ti = p.table_info.as_ref()?;
        let col_count = ti.col_count;
        let mut start = line_no;
        while start > 0 {
            let prev = self.get_line(start - 1)?;
            if !prev.is_table_row() {
                break;
            }
            let Some(pt) = prev.table_info.as_ref() else {
                break;
            };
            if pt.col_count != col_count {
                break;
            }
            start -= 1;
        }
        let mut end = line_no;
        while end + 1 < self.pgh_views.len() {
            let next = self.get_line(end + 1)?;
            if !next.is_table_row() {
                break;
            }
            let Some(nt) = next.table_info.as_ref() else {
                break;
            };
            if nt.col_count != col_count {
                break;
            }
            end += 1;
        }
        Some((start, end))
    }

    /// 同一块连续 `TableRow` 上、跨物理行的列矩形选区 `(line_lo, line_hi, col_lo, col_hi)`（闭区间）。
    /// 单行表内选区仍走 `PghView` + `get_table_cursor_rect`，此处仅 `c1.line_no != c2.line_no` 时返回。
    ///
    /// 行首 Backspace 会先选「上一行末格末尾 → 本行首格开头」，两端 segment 恰好为 `0..nc-1`，
    /// 若仍当列矩形会误走 `delete_table_row_column_block` 掏空整行。此类不视为列矩形。
    fn table_row_block_column_rect(&self, c1: &Cursor, c2: &Cursor) -> Option<(usize, usize, usize, usize)> {
        if c1.line_no == c2.line_no {
            return None;
        }
        let line_lo = c1.line_no.min(c2.line_no);
        let line_hi = c1.line_no.max(c2.line_no);
        let p_lo = self.get_line(line_lo)?;
        let p_hi = self.get_line(line_hi)?;
        if !p_lo.is_table_row() || !p_hi.is_table_row() {
            return None;
        }
        let ti_lo = p_lo.table_info.as_ref()?;
        let nc = ti_lo.col_count;
        if nc == 0 || c1.segment >= nc || c2.segment >= nc {
            return None;
        }
        let (blk_s, blk_e) = self.table_row_block_range(line_lo)?;
        if line_lo < blk_s || line_hi > blk_e {
            return None;
        }
        for ln in line_lo..=line_hi {
            let p = self.get_line(ln)?;
            if !p.is_table_row() {
                return None;
            }
            let ti = p.table_info.as_ref()?;
            if ti.col_count != nc {
                return None;
            }
        }
        let col_lo = c1.segment.min(c2.segment).min(nc.saturating_sub(1));
        let col_hi = c1.segment.max(c2.segment).min(nc.saturating_sub(1));
        if line_hi == line_lo + 1
            && col_lo == 0
            && col_hi == nc.saturating_sub(1)
        {
            let mn = std::cmp::min(*c1, *c2);
            let mx = std::cmp::max(*c1, *c2);
            let last_seg = nc.saturating_sub(1);
            if mn.line_no == line_lo
                && mx.line_no == line_hi
                && mn.segment == last_seg
                && mx.segment == 0
                && mx.culumn == 0
            {
                return None;
            }
        }
        Some((line_lo, line_hi, col_lo, col_hi))
    }

    /// 跨行 `TableRow` 列矩形复制为独立 GFM 小表：首行 + 分隔行 `|---|` + 后续数据行（单元格可为部分文本）。
    fn table_row_block_column_copy_markdown(
        &self,
        line_lo: usize,
        line_hi: usize,
        col_lo: usize,
        col_hi: usize,
    ) -> String {
        let c1 = self.cursor1();
        let c2 = self.cursor2();
        let ncol = col_hi.saturating_sub(col_lo).saturating_add(1).max(1);
        let mut rows: Vec<String> = Vec::new();
        for ln in line_lo..=line_hi {
            let Some(p) = self.get_line(ln) else {
                continue;
            };
            if !p.is_table_row() {
                continue;
            }
            rows.push(p.table_row_column_block_pipe_data_row(
                ln, &c1, &c2, line_lo, line_hi, col_lo, col_hi,
            ));
        }
        if rows.is_empty() {
            return String::new();
        }
        let sep = format!(
            "|{}|",
            std::iter::repeat("---")
                .take(ncol)
                .collect::<Vec<_>>()
                .join("|")
        );
        let mut out = String::new();
        out.push_str(&rows[0]);
        out.push('\n');
        out.push_str(&sep);
        for r in rows.iter().skip(1) {
            out.push('\n');
            out.push_str(r);
        }
        out
    }

    /// `TableRow` 块列宽估计（与整表 `table_guess_width` 行为对齐）
    pub fn table_guess_width_for_table_row_block(&self, anchor_line_no: usize, ui: &Ui) -> Vec<f32> {
        const TABLE_WIDTH_SAMPLE_ROWS: usize = 15;
        let Some((blk_start, blk_end)) = self.table_row_block_range(anchor_line_no) else {
            return self
                .get_line(anchor_line_no)
                .map(|p| p.table_guess_width(ui, self))
                .unwrap_or_default();
        };
        let Some(table_info) = self
            .get_line(anchor_line_no)
            .and_then(|p| p.table_info.clone())
        else {
            return vec![];
        };
        let total_rows = table_info.table_total_rows.max(1);
        let col_count = table_info.col_count;
        let mut max_width = self.edit_width();
        max_width -= table_info.spacing_indent;
        max_width -= 64.0;
        if self.cfg().show_table_row_no {
            max_width -= PghView::table_index_col_width(total_rows, table_info.col_min_width);
            max_width -= table_info.spacing_x;
        }
        let sample_rows = total_rows.min(TABLE_WIDTH_SAMPLE_ROWS);
        let mut width_info = vec![];
        for c in 0..col_count {
            let mut c_width = 0.0f32;
            for i in 0..sample_rows {
                let ln = blk_start + i;
                if ln > blk_end {
                    break;
                }
                let Some(line_p) = self.get_line(ln) else {
                    continue;
                };
                let Some(ti) = &line_p.table_info else {
                    continue;
                };
                let row_style = ti.table_row_index;
                let text = line_p.get_segment_text(c);
                let w = PghView::table_guess_text_width(ui, self, row_style, text);
                c_width = w.at_least(c_width);
            }
            if total_rows > TABLE_WIDTH_SAMPLE_ROWS {
                c_width = c_width.at_least(table_info.col_min_width);
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
            width_info = width_info
                .iter()
                .map(|w| {
                    if *w <= table_info.col_min_width {
                        *w
                    } else {
                        (w / warp_total * max_warp_width).at_least(table_info.col_min_width)
                    }
                })
                .collect();
        }
        width_info
    }

    /// 重算连续 `TableRow` 块内各行的 `table_row_index` / `table_total_rows`
    pub fn refresh_table_row_block_metadata(&mut self, any_line_in_block: usize) {
        let Some((s, e)) = self.table_row_block_range(any_line_in_block) else {
            return;
        };
        let n = e.saturating_sub(s) + 1;
        for (i, ln) in (s..=e).enumerate() {
            if let Some(p) = self.get_line_mut(ln) {
                if !p.is_table_row() {
                    continue;
                }
                if let Some(ti) = &mut p.table_info {
                    ti.table_row_index = i;
                    ti.table_total_rows = n;
                    ti.row_count = 1;
                }
            }
        }
    }

    /// 与 `line_no` 同属一块的连续 `CodeRow` 的 `[start, end]` 行号（含端点，松散匹配）
    pub fn code_row_block_range(&self, line_no: usize) -> Option<(usize, usize)> {
        let p = self.get_line(line_no)?;
        if !p.is_code_row() {
            return None;
        }
        let mut start = line_no;
        while start > 0 {
            let prev = self.get_line(start - 1)?;
            if !prev.is_code_row() {
                break;
            }
            start -= 1;
        }
        let mut end = line_no;
        while end + 1 < self.pgh_views.len() {
            let next = self.get_line(end + 1)?;
            if !next.is_code_row() {
                break;
            }
            end += 1;
        }
        Some((start, end))
    }

    /// 重算连续 `CodeRow` 块内各行的 `code_row_index` / `code_total_rows`，语言仅保留在块首行；
    /// 并同步 `spacing_top` / `spacing_bottom`（仅块首行有上间隙、仅末行有下间隙，与 `code_to_code_row_pghviews` 一致）。
    pub fn refresh_code_row_block_metadata(&mut self, any_line_in_block: usize) {
        let Some((s, e)) = self.code_row_block_range(any_line_in_block) else {
            return;
        };
        let n = e.saturating_sub(s) + 1;
        let lang = self.get_line(s).and_then(|p| p.code_lang.clone());
        let top = self.cfg().spacing.code.top;
        let bottom = self.cfg().spacing.code.bottom;
        for (i, ln) in (s..=e).enumerate() {
            if let Some(p) = self.get_line_mut(ln) {
                if !p.is_code_row() {
                    continue;
                }
                p.code_info = Some(CodeInfo {
                    code_row_index: i,
                    code_total_rows: n,
                });
                if ln == s {
                    p.code_lang = lang.clone();
                } else {
                    p.code_lang = None;
                }
                p.spacing_top = if i == 0 { top } else { 0.0 };
                p.spacing_bottom = if i + 1 == n { bottom } else { 0.0 };
            }
        }
    }

    /// 物理行删除后，若邻近仍存在 `CodeRow` 块则重算元数据
    fn refresh_code_row_block_after_physical_line_deleted(&mut self, deleted_line_index: usize) {
        if self
            .get_line(deleted_line_index)
            .is_some_and(|p| p.is_code_row())
        {
            self.refresh_code_row_block_metadata(deleted_line_index);
        } else if deleted_line_index > 0
            && self
                .get_line(deleted_line_index - 1)
                .is_some_and(|p| p.is_code_row())
        {
            self.refresh_code_row_block_metadata(deleted_line_index - 1);
        }
    }

    /// 选中整块连续 `CodeRow` 时：`is_raw` 为原文拼接；否则带 ``` 围栏
    fn code_row_block_selected_markdown(
        &self,
        c1: &Cursor,
        c2: &Cursor,
        is_raw: bool,
    ) -> Option<String> {
        let lo = *std::cmp::min(c1, c2);
        let hi = *std::cmp::max(c1, c2);
        let (blk_s, blk_e) = self.code_row_block_range(lo.line_no)?;
        if lo.line_no < blk_s || hi.line_no > blk_e {
            return None;
        }
        let block_start_c = Cursor {
            line_no: blk_s,
            segment: 0,
            culumn: 0,
        };
        let block_end_c = self.get_line(blk_e)?.end_cursor_of_line(blk_e);
        let sel_min = std::cmp::min(c1, c2);
        let sel_max = std::cmp::max(c1, c2);
        if sel_min > &block_start_c || sel_max < &block_end_c {
            return None;
        }
        let mut body = String::new();
        for ln in blk_s..=blk_e {
            let p = self.get_line(ln)?;
            if ln > blk_s {
                body.push('\n');
            }
            body.push_str(&p.get_text());
        }
        if is_raw {
            return Some(body);
        }
        let lang = self.get_line(blk_s)?.code_lang.as_deref().unwrap_or("");
        Some(format!("```{}\n{}\n```", lang, body))
    }

    /// 物理行 `deleted_line_index` 已从 `pgh_views` 删除后，若其上下仍存在同一 `TableRow` 块，重算该块元数据。
    fn refresh_table_row_block_after_physical_line_deleted(&mut self, deleted_line_index: usize) {
        if self
            .get_line(deleted_line_index)
            .is_some_and(|p| p.is_table_row())
        {
            self.refresh_table_row_block_metadata(deleted_line_index);
        } else if deleted_line_index > 0
            && self
                .get_line(deleted_line_index - 1)
                .is_some_and(|p| p.is_table_row())
        {
            self.refresh_table_row_block_metadata(deleted_line_index - 1);
        }
    }

    /// 在块内逻辑行 `logical_insert_at` 处插入一空行（`0..=当前行数`）
    pub fn table_row_block_insert_logical_row(
        &mut self,
        anchor_line_no: usize,
        logical_insert_at: usize,
        cursor_col: usize,
    ) {
        let Some((blk_start, blk_end)) = self.table_row_block_range(anchor_line_no) else {
            return;
        };
        let tmpl = match self.get_line(anchor_line_no).cloned() {
            Some(p) => p,
            None => return,
        };
        let Some(ti_src) = tmpl.table_info.clone() else {
            return;
        };
        let col_count = ti_src.col_count;
        let mut nti = ti_src.clone();
        nti.row_count = 1;
        nti.table_row_index = 0;
        nti.table_total_rows = 0;
        let mut new_row = PghView::new_table_row();
        for _ in 0..col_count {
            new_row.push_text(String::new(), None);
        }
        new_row.table_info = Some(nti);
        new_row.spacing_top = tmpl.spacing_top;
        new_row.spacing_bottom = tmpl.spacing_bottom;
        let n_old = blk_end - blk_start + 1;
        let physical = blk_start + logical_insert_at.min(n_old);
        self.pgh_views.insert(physical, new_row);
        self.refresh_table_row_block_metadata(blk_start);
        let c = cursor_col.min(col_count.saturating_sub(1));
        self.state.cursor2.line_no = physical;
        self.state.cursor2.segment = c;
        self.state.cursor2.culumn = 0;
        self.set_cursor1_reset();
    }

    /// 在光标所在 `TableRow` 块的每一行同一列前插入空列
    pub fn table_row_block_insert_col(&mut self, col: usize) {
        let line_no = self.cursor2().line_no;
        let Some((s, e)) = self.table_row_block_range(line_no) else {
            return;
        };
        for ln in s..=e {
            if let Some(p) = self.get_line_mut(ln) {
                p.table_row_insert_col(col);
            }
        }
        self.refresh_table_row_block_metadata(line_no);
        let mut c2 = self.cursor2();
        c2.segment = col;
        c2.culumn = 0;
        self.set_cursor2(self.cursor_check(&c2));
        self.set_cursor1_reset();
    }

    /// 光标所在逻辑单元格 `(row, col)`（`Table` 与 `TableRow` 块统一为整张表的逻辑坐标）。
    pub fn table_cursor_logical_cell(&self) -> Option<(usize, usize)> {
        let c = self.cursor2();
        let p = self.get_line(c.line_no)?;
        let ti = p.table_info.as_ref()?;
        if p.is_table_row() {
            let nc = ti.col_count.max(1);
            let col = c.segment.min(nc - 1);
            Some((ti.table_row_index, col))
        } else if p.is_table() {
            let nc = ti.col_count.max(1);
            let seg = c.segment.min(p.pgh.len().saturating_sub(1));
            Some((seg / nc, seg % nc))
        } else {
            None
        }
    }

    /// 删除当前选区涉及的物理行（`TableRow` 块）；至少保留一块内一行。
    pub fn table_delete_selected_rows(&mut self) {
        if !self.cfg().is_markdown || self.cfg().is_read_only {
            return;
        }
        let anchor = self.cursor2().line_no;
        let Some((blk_s, blk_e)) = self.table_row_block_range(anchor) else {
            if self.get_line(anchor).is_some_and(|p| p.is_table()) {
                self.table_delete_rows_single_table(anchor);
            }
            return;
        };
        let block_len = blk_e - blk_s + 1;
        let mut lines: Vec<usize> = if self.is_selected() {
            self.get_selected_line_nos()
                .into_iter()
                .filter(|ln| *ln >= blk_s && *ln <= blk_e)
                .collect()
        } else {
            vec![anchor]
        };
        lines.sort_unstable();
        lines.dedup();
        if lines.is_empty() {
            return;
        }
        if lines.len() >= block_len {
            lines.retain(|&ln| ln != anchor);
        }
        if lines.len() >= block_len {
            lines.pop();
        }
        if lines.is_empty() {
            return;
        }
        lines.sort_by(|a, b| b.cmp(a));
        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        undo_cmd.set_cursor(self.cursor2());
        for &ln in &lines {
            let Some(clone) = self.get_line_clone(ln) else {
                continue;
            };
            undo_cmd.push_insert(ln, Some(clone));
            self.pgh_views.remove(ln);
            redo_cmd.push_delete(ln);
            self.refresh_table_row_block_after_physical_line_deleted(ln);
        }
        let rest = blk_s.min(self.pgh_views.len().saturating_sub(1));
        self.refresh_table_row_block_metadata(rest);
        let c2 = self.cursor_check(&self.cursor2());
        self.set_cursor2(c2);
        self.set_cursor1_reset();
        redo_cmd.set_cursor(self.cursor2());
        self.push_do(undo_cmd, redo_cmd);
        self.on_content_change();
    }

    fn table_delete_rows_single_table(&mut self, line_no: usize) {
        let c1 = self.cursor1();
        let c2 = self.cursor2();
        let (rmin, rmax, row_count) = {
            let Some(p) = self.get_line(line_no) else {
                return;
            };
            if !p.is_table() {
                return;
            }
            let Some(ti) = p.table_info.clone() else {
                return;
            };
            let row_count = ti.row_count;
            if row_count <= 1 {
                return;
            }
            let rr = if self.is_selected() {
                let s1 = c1.segment.min(c2.segment);
                let s2 = c1.segment.max(c2.segment);
                if let Some((a, b)) = p.table_range_to_cells(s1, s2) {
                    (a.row.min(b.row), a.row.max(b.row))
                } else {
                    let seg = c2.segment;
                    let nc = ti.col_count.max(1);
                    (seg / nc, seg / nc)
                }
            } else {
                let seg = c2.segment;
                let nc = ti.col_count.max(1);
                (seg / nc, seg / nc)
            };
            (rr.0, rr.1, row_count)
        };
        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        undo_cmd.set_cursor(self.cursor2());
        undo_cmd.push_update(line_no, self.get_line_clone(line_no));
        if let Some(p) = self.get_line_mut(line_no) {
            let mut r_hi = rmax.min(row_count - 1);
            let mut r_lo = rmin.min(r_hi);
            let span = r_hi - r_lo + 1;
            if span >= row_count {
                r_lo = 1;
                r_hi = row_count - 1;
            }
            for r in (r_lo..=r_hi).rev() {
                if p.table_info.as_ref().is_some_and(|t| t.row_count > 1) {
                    p.table_delete_row(r);
                }
            }
        }
        redo_cmd.push_update(line_no, self.get_line_clone(line_no));
        let c2 = self.cursor_check(&self.cursor2());
        self.set_cursor2(c2);
        self.set_cursor1_reset();
        redo_cmd.set_cursor(self.cursor2());
        self.push_do(undo_cmd, redo_cmd);
        self.on_content_change();
    }

    /// 删除当前选区涉及的列；至少保留一列。
    pub fn table_delete_selected_cols(&mut self) {
        if !self.cfg().is_markdown || self.cfg().is_read_only {
            return;
        }
        let anchor = self.cursor2().line_no;
        let c1 = self.cursor1();
        let c2 = self.cursor2();
        if let Some((lo, hi, cl, ch)) = self.table_row_block_column_rect(&c1, &c2) {
            let mut cols: Vec<usize> = (cl..=ch).collect();
            cols.sort_unstable();
            cols.dedup();
            cols.reverse();
            self.table_row_block_delete_cols_undoable(lo, &cols);
            return;
        }
        if self.table_row_block_range(anchor).is_some() {
            let col = self
                .table_cursor_logical_cell()
                .map(|(_, c)| c)
                .unwrap_or(0);
            self.table_row_block_delete_cols_undoable(anchor, &[col]);
            return;
        }
        if self.get_line(anchor).is_some_and(|p| p.is_table()) {
            self.table_delete_cols_single_table(anchor, &c1, &c2);
        }
    }

    fn table_row_block_delete_cols_undoable(&mut self, blk_anchor: usize, cols: &[usize]) {
        let Some((s, e)) = self.table_row_block_range(blk_anchor) else {
            return;
        };
        let nc = self
            .get_line(s)
            .and_then(|p| p.table_info.as_ref())
            .map(|t| t.col_count)
            .unwrap_or(0);
        if nc <= 1 {
            return;
        }
        let mut cols: Vec<usize> = cols
            .iter()
            .copied()
            .filter(|&c| c < nc)
            .collect();
        cols.sort_unstable();
        cols.dedup();
        cols.reverse();
        while cols.len() >= nc {
            cols.pop();
        }
        if cols.is_empty() {
            return;
        }
        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        undo_cmd.set_cursor(self.cursor2());
        for ln in s..=e {
            undo_cmd.push_update(ln, self.get_line_clone(ln));
        }
        for &c in &cols {
            for ln in s..=e {
                if let Some(p) = self.get_line_mut(ln) {
                    if p.is_table_row() {
                        let ti = p.table_info.as_ref().map(|t| t.col_count).unwrap_or(0);
                        if ti > 1 && c < ti {
                            p.table_row_delete_col(c);
                        }
                    }
                }
            }
            self.refresh_table_row_block_metadata(s);
        }
        for ln in s..=e {
            redo_cmd.push_update(ln, self.get_line_clone(ln));
        }
        let c2 = self.cursor_check(&self.cursor2());
        self.set_cursor2(c2);
        self.set_cursor1_reset();
        redo_cmd.set_cursor(self.cursor2());
        self.push_do(undo_cmd, redo_cmd);
        self.on_content_change();
    }

    fn table_delete_cols_single_table(&mut self, line_no: usize, c1: &Cursor, c2: &Cursor) {
        let (mut cols, nc) = {
            let Some(p) = self.get_line(line_no) else {
                return;
            };
            if !p.is_table() {
                return;
            }
            let Some(ti) = p.table_info.clone() else {
                return;
            };
            let nc = ti.col_count;
            if nc <= 1 {
                return;
            }
            let mut cols: Vec<usize> = if self.is_selected() {
                let s1 = c1.segment.min(c2.segment);
                let s2 = c1.segment.max(c2.segment);
                if let Some((a, b)) = p.table_range_to_cells(s1, s2) {
                    (a.col.min(b.col)..=a.col.max(b.col)).collect()
                } else {
                    vec![c2.segment % nc.max(1)]
                }
            } else {
                let seg = c2.segment;
                vec![seg % nc.max(1)]
            };
            cols.sort_unstable();
            cols.dedup();
            cols.reverse();
            (cols, nc)
        };
        while cols.len() >= nc {
            cols.pop();
        }
        if cols.is_empty() {
            return;
        }
        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        undo_cmd.set_cursor(self.cursor2());
        undo_cmd.push_update(line_no, self.get_line_clone(line_no));
        if let Some(p) = self.get_line_mut(line_no) {
            for &c in &cols {
                if p.table_info.as_ref().is_some_and(|t| t.col_count > 1) {
                    p.table_delete_col(c);
                }
            }
        }
        redo_cmd.push_update(line_no, self.get_line_clone(line_no));
        let c2 = self.cursor_check(&self.cursor2());
        self.set_cursor2(c2);
        self.set_cursor1_reset();
        redo_cmd.set_cursor(self.cursor2());
        self.push_do(undo_cmd, redo_cmd);
        self.on_content_change();
    }

    /// 在光标所在逻辑行上方插入空行。
    pub fn table_insert_row_above(&mut self) {
        if !self.cfg().is_markdown || self.cfg().is_read_only {
            return;
        }
        let anchor = self.cursor2().line_no;
        let (r, col) = self
            .table_cursor_logical_cell()
            .unwrap_or((0usize, 0usize));
        if self.get_line(anchor).is_some_and(|p| p.is_table_row()) {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            undo_cmd.set_cursor(self.cursor2());
            if let Some((s, e)) = self.table_row_block_range(anchor) {
                for ln in s..=e {
                    undo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            self.table_row_block_insert_logical_row(anchor, r, col);
            let inserted_line = self.cursor2().line_no;
            let ins_clone = self.get_line_clone(inserted_line);
            undo_cmd.push_delete(inserted_line);
            if let Some((s2, e2)) = self.table_row_block_range(self.cursor2().line_no) {
                if let Some(ref row) = ins_clone {
                    redo_cmd.push_insert(inserted_line, Some(row.clone()));
                }
                for ln in s2..=e2 {
                    if ln == inserted_line {
                        continue;
                    }
                    redo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            self.set_cursor1_reset();
            redo_cmd.set_cursor(self.cursor2());
            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
            return;
        }
        if self.get_line(anchor).is_some_and(|p| p.is_table()) {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            undo_cmd.set_cursor(self.cursor2());
            undo_cmd.push_update(anchor, self.get_line_clone(anchor));
            if let Some(p) = self.get_line_mut(anchor) {
                p.table_insert_row(r);
            }
            redo_cmd.push_update(anchor, self.get_line_clone(anchor));
            let c2 = self.cursor_check(&self.cursor2());
            self.set_cursor2(c2);
            self.set_cursor1_reset();
            redo_cmd.set_cursor(self.cursor2());
            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
        }
    }

    /// 在光标所在逻辑行下方插入空行。
    pub fn table_insert_row_below(&mut self) {
        if !self.cfg().is_markdown || self.cfg().is_read_only {
            return;
        }
        let anchor = self.cursor2().line_no;
        let (r, col) = self
            .table_cursor_logical_cell()
            .unwrap_or((0usize, 0usize));
        if self.get_line(anchor).is_some_and(|p| p.is_table_row()) {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            undo_cmd.set_cursor(self.cursor2());
            if let Some((s, e)) = self.table_row_block_range(anchor) {
                for ln in s..=e {
                    undo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            self.table_row_block_insert_logical_row(anchor, r + 1, col);
            let inserted_line = self.cursor2().line_no;
            let ins_clone = self.get_line_clone(inserted_line);
            undo_cmd.push_delete(inserted_line);
            if let Some((s2, e2)) = self.table_row_block_range(self.cursor2().line_no) {
                if let Some(ref row) = ins_clone {
                    redo_cmd.push_insert(inserted_line, Some(row.clone()));
                }
                for ln in s2..=e2 {
                    if ln == inserted_line {
                        continue;
                    }
                    redo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            self.set_cursor1_reset();
            redo_cmd.set_cursor(self.cursor2());
            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
            return;
        }
        if self.get_line(anchor).is_some_and(|p| p.is_table()) {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            undo_cmd.set_cursor(self.cursor2());
            undo_cmd.push_update(anchor, self.get_line_clone(anchor));
            if let Some(p) = self.get_line_mut(anchor) {
                p.table_insert_row(r + 1);
            }
            redo_cmd.push_update(anchor, self.get_line_clone(anchor));
            let c2 = self.cursor_check(&self.cursor2());
            self.set_cursor2(c2);
            self.set_cursor1_reset();
            redo_cmd.set_cursor(self.cursor2());
            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
        }
    }

    /// 在当前列左侧插入空列（`col` 位置前插入）。
    pub fn table_insert_col_left(&mut self) {
        if !self.cfg().is_markdown || self.cfg().is_read_only {
            return;
        }
        let anchor = self.cursor2().line_no;
        let col = self.table_cursor_logical_cell().map(|(_, c)| c).unwrap_or(0);
        if self.get_line(anchor).is_some_and(|p| p.is_table_row()) {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            undo_cmd.set_cursor(self.cursor2());
            if let Some((s, e)) = self.table_row_block_range(anchor) {
                for ln in s..=e {
                    undo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            self.table_row_block_insert_col(col);
            if let Some((s2, e2)) = self.table_row_block_range(self.cursor2().line_no) {
                for ln in s2..=e2 {
                    redo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            redo_cmd.set_cursor(self.cursor2());
            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
            return;
        }
        if self.get_line(anchor).is_some_and(|p| p.is_table()) {
            let (r, _) = self.table_cursor_logical_cell().unwrap_or((0, 0));
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            let cur_before = self.cursor2();
            undo_cmd.set_cursor(cur_before);
            undo_cmd.push_update(anchor, self.get_line_clone(anchor));
            let new_seg = if let Some(p) = self.get_line_mut(anchor) {
                p.table_insert_col(col);
                p.table_info
                    .as_ref()
                    .map(|info| r * info.col_count + col)
            } else {
                None
            };
            if let Some(seg) = new_seg {
                let mut new_cursor = cur_before;
                new_cursor.segment = seg;
                self.set_cursor2(self.cursor_check(&new_cursor));
            }
            redo_cmd.push_update(anchor, self.get_line_clone(anchor));
            redo_cmd.set_cursor(self.cursor2());
            self.set_cursor1_reset();
            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
        }
    }

    /// 在当前列右侧插入空列（与单元格顶部「右」侧插入列按钮一致）。
    pub fn table_insert_col_right(&mut self) {
        if !self.cfg().is_markdown || self.cfg().is_read_only {
            return;
        }
        let anchor = self.cursor2().line_no;
        let (r, col) = self.table_cursor_logical_cell().unwrap_or((0usize, 0usize));
        let insert_at = col + 1;
        if self.get_line(anchor).is_some_and(|p| p.is_table_row()) {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            undo_cmd.set_cursor(self.cursor2());
            if let Some((s, e)) = self.table_row_block_range(anchor) {
                for ln in s..=e {
                    undo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            self.table_row_block_insert_col(insert_at);
            if let Some((s2, e2)) = self.table_row_block_range(self.cursor2().line_no) {
                for ln in s2..=e2 {
                    redo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            redo_cmd.set_cursor(self.cursor2());
            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
            return;
        }
        if self.get_line(anchor).is_some_and(|p| p.is_table()) {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            let cur_before = self.cursor2();
            undo_cmd.set_cursor(cur_before);
            undo_cmd.push_update(anchor, self.get_line_clone(anchor));
            let new_seg = if let Some(p) = self.get_line_mut(anchor) {
                p.table_insert_col(insert_at);
                p.table_info
                    .as_ref()
                    .map(|info| r * info.col_count + col)
            } else {
                None
            };
            if let Some(seg) = new_seg {
                let mut new_cursor = cur_before;
                new_cursor.segment = seg;
                self.set_cursor2(self.cursor_check(&new_cursor));
            }
            redo_cmd.push_update(anchor, self.get_line_clone(anchor));
            redo_cmd.set_cursor(self.cursor2());
            self.set_cursor1_reset();
            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
        }
    }

    /// 在 `TableRow` 连续块内将剪贴板中的整张 GFM 表（`PghType::Table`）按单元格合并到当前锚点，行为与单行 `Table` 的 `table_merge` 对齐。
    /// 返回 `(光标, 合并后块最后一行的物理行号)`，供 undo 中 `push_delete` 使用。
    fn table_row_block_merge_paste(
        &mut self,
        anchor_line_no: usize,
        anchor_segment: usize,
        table: &PghView,
    ) -> Option<(Cursor, usize)> {
        let p = self.get_line(anchor_line_no)?;
        if !p.is_table_row() {
            return None;
        }
        let change_info = table.table_info.as_ref()?;
        let (blk_start, mut blk_end) = self.table_row_block_range(anchor_line_no)?;
        let min_cell = p.table_segment_to_cell(anchor_segment)?;
        let mut col_count = p.table_info.as_ref()?.col_count;
        let mut n_rows = blk_end.saturating_sub(blk_start) + 1;
        let need_rows = min_cell.row + change_info.row_count;
        let need_cols = min_cell.col + change_info.col_count;

        let saved_cursor = self.cursor2();
        let mut c_tmp = saved_cursor;
        c_tmp.line_no = anchor_line_no;
        self.set_cursor2(self.cursor_check(&c_tmp));

        while col_count < need_cols {
            self.table_row_block_insert_col(col_count);
            col_count += 1;
        }

        while n_rows < need_rows {
            self.table_row_block_insert_logical_row(blk_start, n_rows, 0);
            let (_, e) = self.table_row_block_range(blk_start)?;
            blk_end = e;
            n_rows = blk_end.saturating_sub(blk_start) + 1;
        }

        self.set_cursor2(saved_cursor);

        for r in 0..change_info.row_count {
            for c in 0..change_info.col_count {
                let org_seg = r * change_info.col_count + c;
                let org_txt = table.get_segment_text(org_seg);
                let dst_ln = blk_start + min_cell.row + r;
                let dst_col = min_cell.col + c;
                if let Some(dst_p) = self.get_line_mut(dst_ln) {
                    dst_p.update_segment_text(dst_col, org_txt);
                }
            }
        }
        self.refresh_table_row_block_metadata(blk_start);

        let (_, blk_end_final) = self.table_row_block_range(blk_start)?;
        Some((
            Cursor {
                line_no: blk_start + min_cell.row,
                segment: min_cell.col,
                culumn: 0,
            },
            blk_end_final,
        ))
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

        // 在删除前保存cursor2所在行的类型，用于在所有行被删除时保留类型
        let cursor2_line_type = self.pgh_views.get(self.cursor2().line_no)
            .map(|pgh_view| pgh_view.pgh_type.clone());

        let tr_rect = self.table_row_block_column_rect(&self.cursor1(), &self.cursor2());
        let mut line_set = vec![];
        for (line_no, pgh_view) in self.current_cursor_pghviews() {
            let after_delete = if let Some((lo, hi, cl, ch)) = tr_rect {
                if pgh_view.is_table_row() && line_no >= lo && line_no <= hi {
                    pgh_view.delete_table_row_column_block(
                        line_no,
                        &self.cursor1(),
                        &self.cursor2(),
                        lo,
                        hi,
                        cl,
                        ch,
                    )
                } else {
                    pgh_view.delete(line_no, &self.cursor1(), &self.cursor2())
                }
            } else {
                pgh_view.delete(line_no, &self.cursor1(), &self.cursor2())
            };
            line_set.push((line_no, after_delete));
        }

        self.set_cursors_to_min();

        for (i, (line_no, after_delete)) in line_set.iter().enumerate() {
            log::debug!("update {} to {:?}", line_no, after_delete);
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
            // if the line is code and the line is empty, do not delete it
            if new_s.len() == 0 && !(line_no == &self.cursor2().line_no && cursor2_line_type == Some(PghType::CodeRow)) {
                log::debug!("delete line {}", *line_no);
                undo_cmd.push_insert(*line_no, self.get_line_clone(*line_no));
                self.pgh_views.remove(*line_no);
                redo_cmd.push_delete(*line_no);
                self.refresh_table_row_block_after_physical_line_deleted(*line_no);
                self.refresh_code_row_block_after_physical_line_deleted(*line_no);
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
            log::debug!("merge remain 2 lines");
            let (first_line_no, first_s, first_segments) = remain_lines.last().unwrap();
            let (_, last_s, _) = remain_lines.first().unwrap();
            let last_line_no = first_line_no + 1;
            if let Some(last) = self.pgh_views.get(last_line_no) {
                if let Some(first) = self.pgh_views.get(*first_line_no) {
                    if !last.is_table_like()
                        && !first.is_table_like()
                        && !first.is_code_row()
                        && !last.is_code_row()
                    {
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
        self.collapse_empty_text_between_table_rows(&mut undo_cmd, &mut redo_cmd);
        self.collapse_empty_cols_in_table_row_blocks(&mut undo_cmd, &mut redo_cmd);
        let c2 = self.cursor_check(&self.cursor2());
        self.set_cursor2(c2);
        self.set_cursor1_reset();
        log::debug!("cursor after delete: {:?}", self.cursor2());
        return (undo_cmd, redo_cmd);
    }

    /// 连续 `TableRow` 块中：若某列在块内每一行上均为空白，则从模型中去掉该列（不保留空列管道）。
    fn collapse_empty_cols_in_table_row_blocks(
        &mut self,
        undo_cmd: &mut DoCmd,
        redo_cmd: &mut DoCmd,
    ) {
        let len = self.pgh_views.len();
        let mut line_no = 0usize;
        while line_no < len {
            if !self.get_line(line_no).is_some_and(|p| p.is_table_row()) {
                line_no += 1;
                continue;
            }
            let Some((s, e)) = self.table_row_block_range(line_no) else {
                line_no += 1;
                continue;
            };
            loop {
                let nc = self
                    .get_line(s)
                    .and_then(|p| p.table_info.as_ref())
                    .map(|t| t.col_count)
                    .unwrap_or(0);
                if nc <= 1 {
                    break;
                }
                let mut empty_cols: Vec<usize> = Vec::new();
                'scan_col: for c in 0..nc {
                    for ln in s..=e {
                        let Some(row) = self.get_line(ln) else {
                            continue 'scan_col;
                        };
                        if !row.is_table_row() {
                            continue 'scan_col;
                        }
                        let row_nc = row.table_info.as_ref().map(|t| t.col_count).unwrap_or(0);
                        if row_nc != nc {
                            continue 'scan_col;
                        }
                        if !row.get_segment_text(c).trim().is_empty() {
                            continue 'scan_col;
                        }
                    }
                    empty_cols.push(c);
                }
                if empty_cols.is_empty() {
                    break;
                }
                for &c in empty_cols.iter().rev() {
                    self.table_row_block_delete_col(s, c, undo_cmd, redo_cmd);
                }
            }
            line_no = e + 1;
        }
    }

    /// 在块内每一行删除同一列（`col` 为 `0..col_count`），至少保留一列。
    fn table_row_block_delete_col(
        &mut self,
        blk_start: usize,
        col: usize,
        undo_cmd: &mut DoCmd,
        redo_cmd: &mut DoCmd,
    ) {
        let Some((s, e)) = self.table_row_block_range(blk_start) else {
            return;
        };
        let nc = self
            .get_line(s)
            .and_then(|p| p.table_info.as_ref())
            .map(|t| t.col_count)
            .unwrap_or(0);
        if nc <= 1 || col >= nc {
            return;
        }
        for ln in s..=e {
            if self.get_line(ln).is_some_and(|p| p.is_table_row()) {
                undo_cmd.push_update(ln, self.get_line_clone(ln));
            }
        }
        for ln in s..=e {
            if let Some(p) = self.get_line_mut(ln) {
                if p.is_table_row() {
                    p.table_row_delete_col(col);
                }
            }
        }
        for ln in s..=e {
            if self.get_line(ln).is_some_and(|p| p.is_table_row()) {
                redo_cmd.push_update(ln, self.get_line_clone(ln));
            }
        }
        self.refresh_table_row_block_metadata(s);
    }

    /// 删除整表行后有时会留下无正文的 `Text` 行夹在两块 `TableRow` 之间；从模型中去掉该行，避免版面与导出上表被拆成两段。
    fn collapse_empty_text_between_table_rows(
        &mut self,
        undo_cmd: &mut DoCmd,
        redo_cmd: &mut DoCmd,
    ) {
        let mut i = 1;
        while i + 1 < self.pgh_views.len() {
            let prev_tr = self
                .pgh_views
                .get(i - 1)
                .is_some_and(|p| p.is_table_row());
            let next_tr = self
                .pgh_views
                .get(i + 1)
                .is_some_and(|p| p.is_table_row());
            let mid_empty = self.pgh_views.get(i).is_some_and(|p| {
                p.pgh_type == PghType::Text && p.get_text().trim().is_empty()
            });
            if prev_tr && mid_empty && next_tr {
                if let Some(clone) = self.get_line_clone(i) {
                    undo_cmd.push_insert(i, Some(clone));
                }
                self.pgh_views.remove(i);
                redo_cmd.push_delete(i);
                self.refresh_table_row_block_after_physical_line_deleted(i);
                continue;
            }
            i += 1;
        }
    }

    //检查从line_no_start到line_no_end的文本，如果文本中存在代码块，则需转换为代码块
    pub fn check_change_to_code(&mut self, line_no_start: usize, line_no_end: usize, undo_cmd: &mut DoCmd, redo_cmd: &mut DoCmd) {
        if !self.cfg().is_markdown {
            return;
        }
        // 确保范围有效
        if line_no_start > line_no_end || line_no_end >= self.pgh_views.len() {
            return;
        }

        // 收集范围内的所有行文本，并记录行号
        let mut lines_with_numbers = vec![];
        for line_no in line_no_start..=line_no_end {
            // 跳过已经是代码块的行
            if self.is_line_type(line_no, PghType::CodeRow) {
                continue;
            }
            let txt = self.get_line_text(line_no);
            lines_with_numbers.push((line_no, txt));
        }

        if lines_with_numbers.is_empty() {
            return;
        }

        // 查找所有代码块（```...```）
        // 逐行检查，找到 ``` 开头的行，然后找到对应的 ``` 结尾的行
        let mut code_blocks = vec![];
        let mut i = 0;
        while i < lines_with_numbers.len() {
            let (line_no, txt) = &lines_with_numbers[i];
            let trimmed = txt.trim_start();
            
            // 检查是否是代码块开始标记 ```
            if trimmed.starts_with("```") {
                let start_line = *line_no;
                let mut code_lines = vec![txt.clone()];
                let mut found_end = false;
                
                // 查找结束标记 ```
                for j in (i + 1)..lines_with_numbers.len() {
                    let (next_line_no, next_txt) = &lines_with_numbers[j];
                    code_lines.push(next_txt.clone());
                    
                    let next_trimmed = next_txt.trim_start();
                    if next_trimmed.starts_with("```") {
                        // 找到结束标记
                        let end_line = *next_line_no;
                        let code_text = code_lines.join("\n");
                        code_blocks.push((start_line, end_line, code_text));
                        i = j + 1;
                        found_end = true;
                        break;
                    }
                }
                
                if !found_end {
                    // 没有找到结束标记，跳过
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        // 从后往前处理代码块，避免行号变化
        if !code_blocks.is_empty() {
            let current_cursor = self.cursor2();
            let mut new_cursor_line = current_cursor.line_no;
            let mut cursor_in_code_block = false;
            
            // 先找到光标所在的代码块（如果有）
            for (start_line, end_line, _) in code_blocks.iter() {
                if current_cursor.line_no >= *start_line && current_cursor.line_no <= *end_line {
                    new_cursor_line = *start_line;
                    cursor_in_code_block = true;
                    break;
                }
            }
            
            // 如果没有在代码块内，计算需要调整的行号
            if !cursor_in_code_block {
                let mut deleted_before_cursor = 0;
                for (start_line, end_line, _) in code_blocks.iter() {
                    if *end_line < current_cursor.line_no {
                        // 被删除的行在光标之前，需要减去被删除的行数（不包括起始行，因为会被替换）
                        deleted_before_cursor += end_line - start_line;
                    }
                }
                new_cursor_line = current_cursor.line_no - deleted_before_cursor;
            }
            
            for (start_line, end_line, code_text) in code_blocks.iter().rev() {
                if let Some(rows) = self.check_to_code_pghviews(&code_text) {
                    if rows.is_empty() {
                        continue;
                    }
                    log::debug!("change to code block rows={}", rows.len());

                    let mut need_delete_lines = vec![];
                    for line_no in *start_line..=*end_line {
                        need_delete_lines.push(line_no);
                    }

                    for i in need_delete_lines.iter().rev() {
                        undo_cmd.push_insert(*i, self.get_line_clone(*i));
                        self.pgh_views.remove(*i);
                        redo_cmd.push_delete(*i);
                    }

                    let insert_line = *need_delete_lines.first().unwrap();
                    for (j, row) in rows.into_iter().enumerate() {
                        let at = insert_line + j;
                        undo_cmd.push_delete(at);
                        self.pgh_views.insert(at, row);
                        redo_cmd.push_insert(at, self.get_line_clone(at));
                    }
                    self.refresh_code_row_block_metadata(insert_line);
                }
            }
            
            // 更新光标位置
            if new_cursor_line != current_cursor.line_no {
                let mut new_cursor = current_cursor;
                new_cursor.line_no = new_cursor_line;
                new_cursor.segment = 0;
                new_cursor.culumn = 0;
                self.set_cursor2(self.cursor_check(&new_cursor));
            }
        }
        
    }

    pub fn check_to_code_pghviews(&self, s: &str) -> Option<Vec<PghView>> {
        if !s.trim_start().starts_with("```") {
            return None;
        }
        let markdown = MarkDownImpl::new(s, true, None, false, self.cfg());
        markdown.markdown_to_code_rows_if_single_code()
    }

    /// 移除代码块标记（头尾的 ```）
    /// 删除所有匹配 `^\s*```\w*$` 的行
    fn remove_code_block_markers(s: String) -> String {
        if !s.contains("```") {
            return s;
        }
        let re = Regex::new(r"^\s*```\w*$").unwrap();
        s.lines()
            .filter(|line| !re.is_match(line))
            .collect::<Vec<_>>()
            .join("\n")
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

    /// 管道块解析为多个 `PghType::TableRow`（GFM 单表根节点）
    pub fn check_to_table_row_pghviews(&self, s: &str) -> Option<Vec<PghView>> {
        if !s.starts_with("|") {
            return None;
        }
        let markdown = MarkDownImpl::new(s, true, None, false, self.cfg());
        markdown.markdown_to_table_rows_if_single_table()
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
        log::debug!("on_content_change table:[{}]", joins);

        //check is table markdown → 多行 TableRow
        if let Some(rows) = self.check_to_table_row_pghviews(&joins) {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            undo_cmd.set_cursor(self.cursor2());
            log::debug!("change to table rows count {}", rows.len());
            for i in need_delete_lines.iter().rev() {
                undo_cmd.push_insert(*i, self.get_line_clone(*i));
                self.pgh_views.remove(*i);
                redo_cmd.push_delete(*i);
            }
            let line = *need_delete_lines.first().unwrap();
            for (k, row) in rows.iter().enumerate() {
                let ln = line + k;
                undo_cmd.push_delete(ln);
                self.pgh_views.insert(ln, row.clone());
                redo_cmd.push_insert(ln, self.get_line_clone(ln));
            }
            self.refresh_table_row_block_metadata(line);

            //change cursor
            self.set_cursor2(line.into());
            self.set_cursor1_reset();
            redo_cmd.set_cursor(self.cursor2());
            self.push_do(undo_cmd, redo_cmd);
        }
    }

    fn content_change_state(&mut self) {
        self.state.change_current_tick += 1;

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
        self.state.change_last_save_tick = self.state.change_current_tick;
    }

    pub fn is_content_changed(&self) -> bool {
        self.state.change_current_tick != self.state.change_last_save_tick
    }

    fn toc_replace_entries(&mut self) {
        if self.cfg.is_markdown {
            self.toc_cache.replace_entries(MarkdownOutline::collect_from_ctx(self));
        } else {
            self.toc_cache.clear_all();
        }
    }

    pub fn toc_ensure_updated(&mut self, now_secs: f64, scan_interval_secs: f64) {
        if !self.cfg.is_markdown {
            self.toc_cache.clear_all();
            self.toc_cache.last_scan_secs = now_secs;
            return;
        }

        if now_secs < scan_interval_secs + self.toc_cache.last_scan_secs {
            return;
        }

        let tick = self.state.change_current_tick;
        if self.toc_cache.last_build_content_tick == tick {
            self.toc_cache.last_scan_secs = now_secs;
            return;
        }

        self.toc_cache.replace_entries(MarkdownOutline::collect_from_ctx(self));
        self.toc_cache.last_build_content_tick = tick;
        self.toc_cache.last_scan_secs = now_secs;
        self.line_flash_all();
    }

    pub fn toc_entries(&self) -> &[TocEntry] {
        &self.toc_cache.entries
    }

    pub fn toc_entry_for_line(&self, line_no: usize) -> Option<&TocEntry> {
        self.toc_cache.by_line.get(&line_no)
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
        let mut new_pgh_view = PghView::new_text();
        new_pgh_view.push_text(s, None);
        self.pgh_views.insert(line_no, new_pgh_view);
    }

    pub fn insert(&mut self, s: String) {
        let (mut undo_cmd, mut redo_cmd) = self.delete_func();

        let org_c: Cursor = self.cursor2();
        let mut new_c = org_c;

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
                self.set_cursor2(new_c);
                self.set_cursor1_reset();
                redo_cmd.set_cursor(self.cursor2());
                self.push_do(undo_cmd, redo_cmd);
            } else if pgh_view.is_table_row() {
                'table_row_paste: {
                    if let Some(table) = self.check_to_table_pghview(&s) {
                        if let Some((bs, be)) = self.table_row_block_range(org_c.line_no) {
                            let old_snapshots: Vec<Option<PghView>> =
                                (bs..=be).map(|ln| self.get_line_clone(ln)).collect();
                            if let Some((merged_c, be2)) = self.table_row_block_merge_paste(
                                org_c.line_no,
                                org_c.segment,
                                &table,
                            ) {
                                new_c = self.cursor_check(&merged_c);
                                for ln in bs..=be {
                                    undo_cmd.push_update(ln, old_snapshots[ln - bs].clone());
                                }
                                let added = be2.saturating_sub(be);
                                if added > 0 {
                                    // undo 倒序执行：须先 push 较小行号，后删较大行号，否则会因索引下移漏删
                                    for ln in be + 1..=be + added {
                                        undo_cmd.push_delete(ln);
                                    }
                                }
                                for ln in bs..=be {
                                    redo_cmd.push_update(ln, self.get_line_clone(ln));
                                }
                                for ln in be + 1..=be2 {
                                    redo_cmd.push_insert(ln, self.get_line_clone(ln));
                                }
                                self.set_cursor2(new_c);
                                self.set_cursor1_reset();
                                redo_cmd.set_cursor(self.cursor2());
                                self.push_do(undo_cmd, redo_cmd);
                                break 'table_row_paste;
                            }
                        }
                    }
                    undo_cmd.push_update(org_c.line_no, self.get_line_clone(org_c.line_no));
                    self.update_segment_text(org_c.line_no, org_c.segment, seg_text);
                    new_c.culumn += s.chars().count();
                    redo_cmd.push_update(org_c.line_no, self.get_line_clone(org_c.line_no));
                    self.set_cursor2(new_c);
                    self.set_cursor1_reset();
                    redo_cmd.set_cursor(self.cursor2());
                    self.push_do(undo_cmd, redo_cmd);
                }
            } else if pgh_view.is_code_row() {
                if let Some((bs, be)) = self.code_row_block_range(org_c.line_no) {
                    for ln in bs..=be {
                        undo_cmd.push_update(ln, self.get_line_clone(ln));
                    }
                }
                let cleaned_s = Ctx::remove_code_block_markers(s);
                let new_s = ls + &cleaned_s + &rs;
                let lines: Vec<&str> = new_s.split('\n').collect();
                for (i, line) in lines.iter().enumerate() {
                    let line_no = org_c.line_no + i;
                    if i == 0 {
                        self.update_segment_text(line_no, org_c.segment, (*line).to_string());
                    } else {
                        undo_cmd.push_delete(line_no);
                        let mut nr = PghView::new_code_row();
                        nr.push_text((*line).to_string(), None);
                        nr.spacing_top = 0.0;
                        nr.spacing_bottom = 0.0;
                        self.pgh_views.insert(line_no, nr);
                        redo_cmd.push_insert(line_no, self.get_line_clone(line_no));
                    }
                    if i + 1 == lines.len() {
                        new_c.line_no = org_c.line_no + i;
                        new_c.segment = 0;
                        new_c.culumn = line
                            .chars()
                            .count()
                            .saturating_sub(rs.chars().count());
                    }
                }
                self.refresh_code_row_block_metadata(org_c.line_no);
                if let Some((bs2, be2)) = self.code_row_block_range(org_c.line_no) {
                    for ln in bs2..=be2 {
                        redo_cmd.push_update(ln, self.get_line_clone(ln));
                    }
                }
                self.set_cursor2(new_c);
                self.set_cursor1_reset();
                redo_cmd.set_cursor(self.cursor2());
                self.push_do(undo_cmd, redo_cmd);
            } else {
                let new_s = ls + &s + &rs;
                let lines: Vec<&str> = new_s.split('\n').collect();
                for (i, line) in lines.iter().enumerate() {
                    let line_no = org_c.line_no + i;
                    if i == 0 {
                        undo_cmd.push_update(line_no, self.get_line_clone(line_no));
                        self.update_all_text(line_no, line.to_string());
                        redo_cmd.push_update(line_no, self.get_line_clone(line_no));
                    } else {
                        undo_cmd.push_delete(line_no);
                        self.insert_line(line_no, line.to_string());
                        redo_cmd.push_insert(line_no, self.get_line_clone(line_no));
                    }

                    //set last line cursor
                    if i + 1 == lines.len() {
                        new_c.line_no = org_c.line_no + i;
                        new_c.segment = 0;
                        new_c.culumn = line.chars().count().saturating_sub(rs.chars().count());
                    }
                }
                self.set_cursor2(new_c);
                self.set_cursor1_reset();

                //check change to code
                self.check_change_to_code(org_c.line_no, org_c.line_no+lines.len()-1, &mut undo_cmd, &mut redo_cmd);
                redo_cmd.set_cursor(self.cursor2());
                self.push_do(undo_cmd, redo_cmd);
            }
        }

        self.on_content_change();
    }

    pub fn insert_tab(&mut self) {
        // 1. 如果没有选中内容，直接插入 tab
        if !self.is_selected() {
            self.insert("\t".to_string());
            return;
        }

        // 2. 如果有选择行，判断如果是 code 或者 table，也是直接插入 tab
        let selected_line_nos: Vec<usize> = {
            let selected_lines = self.current_cursor_pghviews();
            for (_, pgh_view) in &selected_lines {
                if pgh_view.is_code_row() || pgh_view.is_table_like() {
                    self.insert("\t".to_string());
                    return;
                }
            }
            // 收集行号列表
            selected_lines.into_iter().map(|(line_no, _)| line_no).collect()
        };

        // 3. 否则获取所有选择的行，在行首插入 tab
        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        undo_cmd.set_cursor(self.cursor2());
        redo_cmd.set_cursor(self.cursor1());
        
        // 先收集需要更新的行号和文本（此时已确认没有 code 或 table 行）
        let lines_to_update: Vec<(usize, String)> = selected_line_nos
            .into_iter()
            .map(|line_no| (line_no, self.get_line_text(line_no)))
            .collect();
        
        // 然后更新所有行
        for (line_no, line_text) in lines_to_update {
            undo_cmd.push_update(line_no, self.get_line_clone(line_no));
            self.update_all_text(line_no, "\t".to_string() + &line_text);
            redo_cmd.push_update(line_no, self.get_line_clone(line_no));
        }
        
        self.set_cursor1_reset();
        redo_cmd.set_cursor(self.cursor2());
        self.push_do(undo_cmd, redo_cmd);
        self.on_content_change();
    }

    pub fn ime_preedit(&mut self, s: String) {
        //非PreEdit选中，先正常删除选中内容，需要记录redo/undo
        if !self.state.ime_preedit_selected && self.is_selected() {
            self.delete();
        }

        if s.is_empty() {
            return;
        }

        //关闭redo/undo记录
        self.do_mngr.borrow_mut().disable();
        self.insert(s.clone());
        self.do_mngr.borrow_mut().enable();

        //选中preedit内容
        for _ in s.chars() {
            self.cursor2_move_prev();
        }
        self.set_cursor_switch();
        self.state.ime_preedit_selected = true;
    }

    pub fn ime_commit(&mut self, s: String) {
        if self.is_selected() {
            if self.state.ime_preedit_selected {
                //PreEdit选中内容，需要关闭redo/undo
                self.do_mngr.borrow_mut().disable();
                self.delete();
                self.do_mngr.borrow_mut().enable();
                self.state.ime_preedit_selected = false;
            } else {
                self.delete();
            }
        }
        self.insert(s);
        self.set_ime_actived(false);
    }

    pub fn ime_enable(&mut self) {
        self.set_ime_actived(true);
    }

    pub fn ime_disable(&mut self) {
        self.set_ime_actived(false);
    }

    pub fn update_line_text(&mut self, line_no: usize, s: String) {
        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        let bak_pghview = self.get_line_clone(line_no);
        let cursor_before = self.cursor2();
        if let Some(pgh_view) = self.pgh_views.get_mut(line_no) {
            undo_cmd.set_cursor(cursor_before);
            undo_cmd.push_update(line_no, bak_pghview);
            pgh_view.update_all_text(s);
            redo_cmd.push_update(line_no, self.get_line_clone(line_no));
        }
        let cursor_after = self.cursor2();
        redo_cmd.set_cursor(cursor_after);
        self.push_do(undo_cmd, redo_cmd);
        self.on_content_change();
    }

    pub fn enter_auto_pak_ctrl(left: &str) -> String {
        // 提取前导空白字符（空格和制表符）
        let mut indent = String::new();
        let mut pos = 0;
        for ch in left.chars() {
            if ch == ' ' || ch == '\t' {
                indent.push(ch);
                pos += ch.len_utf8();
            } else {
                break;
            }
        }
        
        // 获取去除缩进后的内容
        let content = &left[pos..];
        
        // 处理任务列表：- [ ]、- [x]、- [X]、* [ ]、* [x] 等
        let task_list_re = Regex::new(r"^([-*+])\s+\[([ xX])\]\s+").unwrap();
        if let Some(caps) = task_list_re.captures(content) {
            let marker = caps.get(1).unwrap().as_str();
            return format!("{}{} [ ] ", indent, marker);
        }
        
        // 处理无序列表：-、*、+
        if content.starts_with("- ") {
            return format!("{}- ", indent);
        }
        if content.starts_with("* ") {
            return format!("{}* ", indent);
        }
        if content.starts_with("+ ") {
            return format!("{}+ ", indent);
        }
        
        // 处理有序列表：1.、2.、10. 等
        let ordered_list_re = Regex::new(r"^(\d+)\.\s+").unwrap();
        if let Some(caps) = ordered_list_re.captures(content) {
            // 尝试递增数字
            if let Ok(num) = caps.get(1).unwrap().as_str().parse::<u32>() {
                let next_num = num + 1;
                return format!("{}{}. ", indent, next_num);
            }
            // 如果解析失败，返回相同格式
            return format!("{}{}. ", indent, caps.get(1).unwrap().as_str());
        }
        
        // 处理多级引用：>、>>、>>> 等
        let blockquote_re = Regex::new(r"^(>+\s*)").unwrap();
        if let Some(caps) = blockquote_re.captures(content) {
            return format!("{}{}", indent, caps.get(1).unwrap().as_str());
        }
        
        // 如果只包含空白字符，返回空白
        if content.trim().is_empty() {
            return indent;
        }
        
        // 其他情况返回空字符串
        "".to_string()
    }

    /// 检测并删除自动插入的前缀（用于退格键）
    pub fn backspace_auto_prefix(&mut self) -> bool {
        let c = self.cursor2();
        
        // 检查 do_mngr 中最后一个命令的 action_name 必须是 "enter"
        /* 
        let last_action_is_enter = {
            let do_mngr = self.do_mngr.borrow();
            do_mngr.last_action_name().as_ref().map(|s| s == "enter").unwrap_or(false)
        };
        if !last_action_is_enter {
            return false;
        }
        */
        
        // 只处理普通文本段落，不处理表格和代码块
        if let Some(pgh_view) = self.pgh_views.get(c.line_no) {
            if pgh_view.is_table_like() || pgh_view.is_code_row() {
                return false;
            }
            
            // 获取光标前的文本和光标后的文本
            let (left, right) = pgh_view.normal_enter(&c);
            
            // 如果光标前没有内容，不需要删除前缀
            if left.trim().is_empty() {
                return false;
            }
            
            // 检查光标后的内容是否为空或只有空白
            if !right.trim().is_empty() {
                return false;
            }
            
            // 获取上一行的内容，用于调用 enter_auto_pak_ctrl
            let prev_line_text = if c.line_no > 0 {
                self.get_line_text(c.line_no - 1)
            } else {
                "".to_string()
            };
            
            // 调用 enter_auto_pak_ctrl 获取应该自动插入的前缀
            let expected_prefix = Self::enter_auto_pak_ctrl(&prev_line_text);
            
            // 如果返回的前缀为空，表示不是自动前缀，不需要删除
            if expected_prefix.is_empty() {
                return false;
            }
            
            if left == expected_prefix {
                // 删除从行首到光标位置的所有内容
                let start_cursor = pgh_view.start_cursor_of_line(c.line_no);
                self.set_cursor1(start_cursor);
                self.set_cursor2(c);
                self.delete();
                return true;
            }
        }
        
        false
    }
    

    pub fn enter_insert(&mut self) {
        let (mut undo_cmd, mut redo_cmd) = self.delete_func();

        let c = self.cursor2();
        let is_table_row = self
            .pgh_views
            .get(c.line_no)
            .is_some_and(|p| p.is_table_row());
        if is_table_row {
            let (insert_at, col) = match self.get_line(c.line_no) {
                Some(p) => match p.table_segment_to_cell(c.segment) {
                    Some(cell) => (cell.row + 1, cell.col),
                    None => (0usize, 0usize),
                },
                None => (0usize, 0usize),
            };
            if let Some((s, e)) = self.table_row_block_range(c.line_no) {
                for ln in s..=e {
                    undo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            self.table_row_block_insert_logical_row(c.line_no, insert_at, col);
            let inserted_line = self.cursor2().line_no;
            let ins_clone = self.get_line_clone(inserted_line);
            // 撤销时须先删掉插入的物理行，否则仅 restore 块内旧行会在表格外留下多余一行
            undo_cmd.push_delete(inserted_line);
            if let Some((s2, e2)) = self.table_row_block_range(self.cursor2().line_no) {
                if let Some(ref row) = ins_clone {
                    redo_cmd.push_insert(inserted_line, Some(row.clone()));
                }
                for ln in s2..=e2 {
                    if ln == inserted_line {
                        continue;
                    }
                    redo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            self.set_cursor1_reset();
            redo_cmd.set_cursor(self.cursor2());
            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
            return;
        }

        let is_code_row = self
            .pgh_views
            .get(c.line_no)
            .is_some_and(|p| p.is_code_row());
        if is_code_row {
            if let Some((s, e)) = self.code_row_block_range(c.line_no) {
                for ln in s..=e {
                    undo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            let (left, right) = self
                .pgh_views
                .get(c.line_no)
                .map(|p| p.normal_enter(&c))
                .unwrap_or_default();
            let inserted_line = c.line_no + 1;
            undo_cmd.push_delete(inserted_line);
            if let Some(cur) = self.get_line_mut(c.line_no) {
                cur.update_all_text(left);
            }
            let mut new_row = PghView::new_code_row();
            new_row.push_text(right, None);
            new_row.spacing_top = 0.0;
            new_row.spacing_bottom = 0.0;
            self.pgh_views.insert(inserted_line, new_row);
            self.refresh_code_row_block_metadata(c.line_no);
            if let Some((s2, e2)) = self.code_row_block_range(inserted_line) {
                redo_cmd.push_insert(inserted_line, self.get_line_clone(inserted_line));
                for ln in s2..=e2 {
                    if ln == inserted_line {
                        continue;
                    }
                    redo_cmd.push_update(ln, self.get_line_clone(ln));
                }
            }
            self.state.cursor2.line_no = inserted_line;
            self.state.cursor2.segment = 0;
            self.state.cursor2.culumn = 0;
            self.set_cursor1_reset();
            redo_cmd.set_cursor(self.cursor2());
            self.push_do(undo_cmd, redo_cmd);
            self.on_content_change();
            return;
        }

        if let Some(pgh_view) = self.pgh_views.get_mut(c.line_no) {
            if pgh_view.is_table() {
                undo_cmd.push_update(c.line_no, Some(pgh_view.clone()));
                if let Some(cell) = pgh_view.table_segment_to_cell(c.segment) {
                    let new_segments = pgh_view.table_insert_row(cell.row + 1);
                    self.state.cursor2.segment += new_segments;
                }
                redo_cmd.push_update(c.line_no, Some(pgh_view.clone()));
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

    /// 从剪贴板获取文本
    pub fn get_clipboard_text(&mut self) -> Option<String> {
        self.clipboard.get_text().ok()
    }

    /// 检查是否是单行选中
    pub fn is_single_line_selected(&self) -> bool {
        self.is_selected() && self.cursor1().line_no == self.cursor2().line_no
    }

    /// 检查指定行是否是标题
    pub fn is_heading_line(&self, line_no: usize) -> bool {
        if let Some(pgh_view) = self.pgh_views.get(line_no) {
            pgh_view.pgh_type == PghType::Heading
        } else {
            false
        }
    }

    /// 检查指定行是否是某种类型
    pub fn is_line_type(&self, line_no: usize, pgh_type: PghType) -> bool {
        if let Some(pgh_view) = self.pgh_views.get(line_no) {
            pgh_view.pgh_type == pgh_type
        } else {
            false
        }
    }

    /// 检查选中的行中是否包含某种类型
    pub fn has_selected_line_type(&self, pgh_type: PghType) -> bool {
        if !self.is_selected() {
            return false;
        }
        let cursor1 = self.cursor1();
        let cursor2 = self.cursor2();
        let min_line = std::cmp::min(cursor1.line_no, cursor2.line_no);
        let max_line = std::cmp::max(cursor1.line_no, cursor2.line_no);
        
        for line_no in min_line..=max_line {
            if self.is_line_type(line_no, pgh_type.clone()) {
                return true;
            }
        }
        false
    }

    /// 移除标题标记（# 字符和空格）
    /// 返回移除标记后的文本
    pub fn remove_heading_prefix(text: &str) -> String {
        let trimmed = text.trim_start();
        let mut chars = trimmed.chars();
        let mut count = 0;
        // 计算开头的 # 数量
        while chars.next() == Some('#') {
            count += 1;
            if count >= 6 {
                break;
            }
        }
        // 移除 # 和后面的空格
        if count > 0 {
            trimmed[count.min(trimmed.len())..].trim_start().to_string()
        } else {
            trimmed.to_string()
        }
    }

    /// 获取当前光标所在行的文本（去除标题标记）
    pub fn get_current_line_text_without_heading(&self) -> String {
        let line_no = self.cursor2().line_no;
        let line_text = self.get_line_text(line_no);
        if self.is_heading_line(line_no) {
            Self::remove_heading_prefix(&line_text)
        } else {
            line_text
        }
    }

    /// 获取选中行的完整文本（包括首尾行未被选中的部分）
    pub fn get_selected_lines_full_text(&self) -> Vec<String> {
        let selected_lines = self.current_cursor_pghviews();
        selected_lines
            .iter()
            .map(|(line_no, _)| self.get_line_text(*line_no))
            .collect()
    }

    /// 删除指定的行并插入代码块 PghView
    /// 返回插入的行号
    pub fn replace_lines_with_code_block(&mut self, line_nos: Vec<usize>, code_text: &str) -> usize {
        use crate::medit::{DoCmd, MarkDownImpl, PghView};

        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        undo_cmd.set_cursor(self.cursor2());
        redo_cmd.set_cursor(self.cursor1());

        for line_no in line_nos.iter().rev() {
            undo_cmd.push_insert(*line_no, self.get_line_clone(*line_no));
            self.pgh_views.remove(*line_no);
            redo_cmd.push_delete(*line_no);
        }

        let insert_line_no = line_nos.first().copied().unwrap_or(0);
        let fenced = format!("```\n{}\n```", code_text);
        let md = MarkDownImpl::new(&fenced, true, None, false, self.cfg());
        let mut rows = md.markdown_to_code_rows_if_single_code().unwrap_or_default();
        if rows.is_empty() {
            let mut row = PghView::new_code_row();
            row.push_text(code_text.to_string(), None);
            row.code_info = Some(CodeInfo {
                code_row_index: 0,
                code_total_rows: 1,
            });
            row.spacing_top = self.cfg().spacing.code.top;
            row.spacing_bottom = self.cfg().spacing.code.bottom;
            rows.push(row);
        }
        for (j, row) in rows.into_iter().enumerate() {
            let at = insert_line_no + j;
            undo_cmd.push_delete(at);
            self.pgh_views.insert(at, row);
            redo_cmd.push_insert(at, self.get_line_clone(at));
        }
        self.refresh_code_row_block_metadata(insert_line_no);
        
        // 设置光标
        let new_cursor = (insert_line_no, 0, 0).into();
        self.set_cursor2(new_cursor);
        self.set_cursor1_reset();
        redo_cmd.set_cursor(self.cursor2());
        
        // 执行 undo/redo
        self.push_do(undo_cmd, redo_cmd);
        self.on_content_change();
        
        insert_line_no
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

        // 计算分割区域：位于 line_no_rect 和 edit_rect 之间，宽度为 1.0 像素
        self.area.divider_rect = max_rect;
        self.area.divider_rect.set_left(self.area.line_no_rect.right());
        self.area.divider_rect.set_width(1.0);
        self.area.divider_rect.set_bottom(max_rect.bottom());

        self.area.edit_rect = max_rect;
        self.area.edit_rect.set_left(self.area.divider_rect.right());
        self.area.edit_rect.set_right((max_rect.right() - scroll_width).max(self.area.edit_rect.left()));
        //self.area.edit_rect.set_bottom((max_rect.bottom() - scroll_width).max(self.area.edit_rect.top()));
        self.area.line_no_rect.set_bottom(self.area.edit_rect.bottom());
        self.area.divider_rect.set_bottom(self.area.edit_rect.bottom());
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

    pub fn divider_rect(&self) -> Rect {
        self.area.divider_rect
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

    pub fn scroll_width(&self) -> f32 {
        self.area.scroll_width
    }

    pub fn set_ime_area_changed(&mut self, flag: bool) {
        self.state.ime_area_changed = flag;
    }

    pub fn is_ime_area_changed(&self) -> bool {
        self.state.ime_area_changed
    }

    pub fn set_ime_actived(&mut self, flag: bool) {
        log::debug!("set_ime_actived {}", flag);
        self.state.ime_actived = flag;
    }

    pub fn is_ime_actived(&self) -> bool {
        self.state.ime_actived
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

    pub fn set_text_color_brightness(&mut self, text_color_brightness: f32) {
        self.cfg.text_color_brightness = text_color_brightness;
        //force flash all lines view
        self.line_flash_all();
    }

    pub fn set_font_size(&mut self, size: f32) {
        let mut font_size = size;
        if font_size < 6.0 {
            font_size = 6.0;
        }
        self.cfg.set_font_size(font_size);
        //force flash all lines view
        self.line_flash_all();
    }

    pub fn set_indent_size(&mut self, size: f32) {
        self.cfg.indent_size = size.at_least(0.0);
        self.line_flash_all();
    }

    pub fn set_list_item_indent_size(&mut self, size: f32) {
        self.cfg.indent_size_of_list = size.at_least(0.0);
        self.line_flash_all();
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

    pub fn scroll_area_id(&self) -> u64 {
        self.scroll_area_id
    }

    pub fn cfg(&self) -> &EditCfg {
        &self.cfg
    }

    pub fn cfg_mut(&mut self) -> &mut EditCfg {
        &mut self.cfg
    }

    /// 将 `cfg.table_frame_style` 写入已解析的表格段落（解析时会把该值快照到 `TableInfo`）。
    pub fn sync_table_views_frame_style(&mut self) {
        let style = self.cfg.table_frame_style.clone();
        for pgh in &mut self.pgh_views {
            if pgh.is_table() || pgh.is_table_row() {
                if let Some(ref mut ti) = pgh.table_info {
                    ti.frame_style = style.clone();
                }
            }
        }
    }

    pub fn expanded_ctx(&self) -> &ExpandedCtx {
        &self.expanded_ctx
    }

    pub fn expanded_ctx_mut(&mut self) -> &mut ExpandedCtx {
        &mut self.expanded_ctx
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

    pub fn is_state_changed(&mut self) -> bool {
        let changed = if self.state != self.state_cmp {
            self.state_cmp = self.state.clone();
            true
        } else {
            false
        };
        changed
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
            self.line_flash_all();
        }

        self.state.selecting = selecting;
    }

    pub fn is_selecting(&self) -> bool {
        self.state.selecting
    }

    pub fn is_selected(&self) -> bool {
        self.state.cursor1 != self.state.cursor2
    }

    pub fn is_ime_preedit_selected(&self) -> bool {
        self.state.ime_preedit_selected
    }

    pub fn last_auto_scroll_time(&self) -> u64 {
        self.state.last_auto_scroll_time
    }

    pub fn set_last_auto_scroll_time(&mut self, milliseconds: u64) {
        self.state.last_auto_scroll_time = milliseconds;
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

    pub fn is_selected_line(&self, line_no: usize) -> bool {
        if !self.is_selected() {
            return false;
        }
        if self.state.cursor2 > self.state.cursor1 {
            line_no >= self.state.cursor1.line_no && line_no <= self.state.cursor2.line_no
        } else {
            line_no >= self.state.cursor2.line_no && line_no <= self.state.cursor1.line_no
        }
    }

    pub fn get_selected_line_nos(&self) -> Vec<usize> {
        if !self.is_selected() {
            return Vec::new();
        }
        let cursor1 = self.cursor1();
        let cursor2 = self.cursor2();
        let min_line = std::cmp::min(cursor1.line_no, cursor2.line_no);
        let max_line = std::cmp::max(cursor1.line_no, cursor2.line_no);
        (min_line..=max_line).collect()
    }

    pub fn get_selected_and_current_line_nos(&self) -> Vec<usize> {
        if !self.is_selected() {
            return vec![self.cursor2().line_no];
        }
        return self.get_selected_line_nos();
    }
}

/// command
impl Ctx {
    pub fn insert_cmd(&mut self, cmd: Action) {
        self.cmd_list.insert(0, cmd);      
    }

    pub fn pop_cmd(&mut self) -> Option<Action> {
        self.cmd_list.pop()    
    }

    fn replace_passwd_url(&mut self, line_no: usize, url_info: UrlInfo, passwd: &str, new_title: &str) {
        const URL_PREFIX: &str = "passwd:";
        if let Some(pos) = url_info.text.find(URL_PREFIX) {
            let url_text_prefix = url_info.text[0..pos+URL_PREFIX.len()].to_string();
            let line_text = self.get_line_text(line_no);
            if let Some(start) = line_text.find(&url_text_prefix) {
                let end = start + url_info.text.len();
                if end > line_text.len() {
                    return;
                }
                let new_url_text = format!("{}{} \"{}\")", url_text_prefix, passwd, new_title);
                let left = line_text[0..start].to_string();
                let tail = line_text[end..].to_string();
                let new_line_text = left + &new_url_text + &tail;
                self.update_line_text(line_no, new_line_text);
            }
        }
    }

    fn execute_url(&mut self, line_no: usize, url_info: UrlInfo, is_clicked: bool, is_line_changed: bool) {
        const URL_PREFIX: &str = "passwd:";
        const ENC_PREFIX: &str = "cipher:";
        
        if url_info.url.starts_with(URL_PREFIX) && url_info.title.is_some() {
            let passwd = url_info.url[URL_PREFIX.len()..].to_string();
            let passwd_hided = passwd.chars().all(|c| c == '*');
            let title = url_info.title.clone().unwrap();
            if !passwd_hided {
                //decrypt
                if title.starts_with(ENC_PREFIX) {
                    log::debug!("try decrypt content");
                    let cipher = &title[ENC_PREFIX.len()..];
                    if let Ok(title) = dec_content(cipher, &passwd) {
                        self.replace_passwd_url(line_no, url_info, &passwd, &title)
                    }
                }
                //encrypt
                else if is_clicked {
                    if let Ok(cipher) = enc_content(&title, &passwd) {
                        let new_title = ENC_PREFIX.to_string() + &cipher;
                        self.replace_passwd_url(line_no, url_info, "******", &new_title);
                    }
                }
            }
        } else if is_clicked {
            self.insert_cmd(Action::open_url(url_info))
        }
    }

    pub fn insert_link_click_command(&mut self, line_no: usize, link_info: LinkInfo, is_clicked: bool, is_line_changed: bool) {
        match link_info {
            LinkInfo::File(file) => if is_clicked { self.insert_cmd(Action::open_file(file)) },
            LinkInfo::Url(url_info) => self.execute_url(line_no, url_info, is_clicked, is_line_changed),
            LinkInfo::Image(image) => if is_clicked {
                log::debug!("todo: flash image: {:?}", image)
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
            pghview.refresh_tick += 1;
        }
        
        // 插入 LineChanged 触发命令到 cmd_list
        let line_text = self.get_line_select_text(line_no);
        let mut params = std::collections::HashMap::new();
        params.insert("line_no".to_string(), serde_json::Value::Number(serde_json::Number::from(line_no)));
        params.insert("line_text".to_string(), serde_json::Value::String(line_text));
        
        if let Ok(action) = Action::from_command("line_changed", &params) {
            self.insert_cmd(action);
        }
    }

    pub fn line_change_reset(&mut self, line_no: usize) -> bool {
        let mut changed = false;
        if let Some(pghview) = self.pgh_views.get_mut(line_no) {
            changed = pghview.change_tick > 0;
            pghview.change_tick = 0;
        }
        changed
    }

    pub fn line_flash_reset(&mut self, line_no: usize) -> bool {
        let mut changed = false;
        if let Some(pghview) = self.pgh_views.get_mut(line_no) {
            changed = pghview.refresh_tick > 0;
            pghview.refresh_tick = 0;
        }
        changed
    }

    pub fn line_flash_tick(&mut self, line_no: usize) {
        if let Some(pghview) = self.pgh_views.get_mut(line_no) {
            pghview.refresh_tick += 1;
        }
    }

    pub fn line_flash_all(&mut self) {
        for x in &mut self.pgh_views {
            x.refresh_tick += 1;
        }
    }

    pub fn push_do(&mut self, undo: DoCmd, redo: DoCmd) {
        // 先检查是否激活，并收集需要更新 tick 的行号
        // 注意：只收集 Insert 和 Update 的行号，因为 Delete 操作对应的行已经被删除
        let lines_to_update = {
            let do_mngr = self.do_mngr.borrow();
            if !do_mngr.active {
                return;
            }
            redo.items.iter().filter_map(|n| {
                match n {
                    DoItem::Insert(x) => Some(x.line),
                    DoItem::Update(x) => Some(x.line),
                    DoItem::Delete(_) => None, // Delete 操作的行已被删除，不需要更新 tick
                }
            }).collect::<Vec<usize>>()
        };

        // 更新行的 change_tick（此时没有持有 do_mngr 的 borrow）
        for line in &lines_to_update {
            self.line_change_tick(*line);
        }

        // 现在可以安全地更新 do_mngr（action_name 会在 guard drop 时设置）
        self.do_mngr.borrow_mut().push(undo, redo, None);
    }

    //实现一个guard，在guard的作用域离开后，自动合并作用域内的redo和undo命令
    pub fn merge_redo_and_undo_guard(&mut self, action_name: Option<String>) -> MergeRedoAndUndoGuard {
        let start_index = self.do_mngr.borrow().index;
        MergeRedoAndUndoGuard {
            start_index,
            do_mngr: Rc::clone(&self.do_mngr),
            action_name,
        }
    }
    pub fn ondo_item(&mut self, do_item: &DoItem) {
        match do_item {
            DoItem::Insert(do_line) => {
                if let Some(pgh_view) = &do_line.pgh_view {
                    if do_line.line <= self.pgh_views.len() {
                        let is_tr = pgh_view.is_table_row();
                        let is_cr = pgh_view.is_code_row();
                        self.pgh_views.insert(do_line.line, pgh_view.clone());
                        if is_tr {
                            self.refresh_table_row_block_metadata(do_line.line);
                        }
                        if is_cr {
                            self.refresh_code_row_block_metadata(do_line.line);
                        }
                    }
                }
                self.line_change_tick(do_line.line);
                log::debug!("Insert {} => {}", do_line.line, (do_line.pgh_view).clone().unwrap().get_text());
            }
            DoItem::Delete(do_line) => {
                let ln = do_line.line;
                if ln < self.pgh_views.len() {
                    self.pgh_views.remove(ln);
                    self.refresh_table_row_block_after_physical_line_deleted(ln);
                    self.refresh_code_row_block_after_physical_line_deleted(ln);
                }
                log::debug!("Delete {}", ln);
            }
            DoItem::Update(do_line) => {
                if let Some(pgh_view) = &do_line.pgh_view {
                    self.update_pgh(do_line.line, pgh_view);
                }
                self.line_change_tick(do_line.line);
                log::debug!("Update {} => {}", do_line.line, (do_line.pgh_view).clone().unwrap().get_text())
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
        let mut do_mngr = self.do_mngr.borrow_mut();
        if do_mngr.index == 0 {
            //self.clean_change_tick();
            return;
        }
        do_mngr.index -= 1;
        if let Some(cmd) = do_mngr.do_list.get(do_mngr.index) {
            let mut rev_list = cmd.undo.clone();
            rev_list.items.reverse();
            drop(do_mngr); // 释放 borrow，以便调用其他方法
            self.ondo_list(&rev_list);
            self.content_change_state();
        }
    }

    pub fn redo(&mut self) {
        let mut do_mngr = self.do_mngr.borrow_mut();
        if let Some(cmd) = do_mngr.do_list.get(do_mngr.index) {
            let redo_list_clone = cmd.redo.clone();
            do_mngr.index += 1;
            drop(do_mngr); // 释放 borrow，以便调用其他方法
            self.ondo_list(&redo_list_clone);
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

    /// Directly set find cache and parameters (for scenarios like FindNotes, no need to search in current text)
    pub fn set_find_cache(&mut self, find_cache: FindCache, find_param: FindReplaceCtx) {
        self.find_cache = find_cache;
        self.find_param = find_param;
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
        if lang.is_none() && self.cfg.lang.is_some() {
            for pgh_view in self.pgh_views.iter_mut() {
                for segment in pgh_view.pgh.iter_mut() {
                    segment.item.layout_job_update(None);
                }
            }
        }

        self.cfg.lang = lang;
    }

    pub fn get_heighlight_lang(&self) -> &str {
        if let Some(lang) = &self.cfg.lang {
            lang
        } else {
            "Plain Text"
        }
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
