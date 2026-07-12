use core::f32;
use std::ops::Add;
use std::sync::atomic::{AtomicU64, Ordering};
use std::vec;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

static CTX_SCROLL_AREA_ID_SEQ: AtomicU64 = AtomicU64::new(1);

use egscribe_sitter::highlight_lines;
use crate::medit::{CodeInfo, CodeKey, ImageInfo, LinkInfo, PghType, CharRect, Cursor, MarkDownImpl, SegmentType, PghView,
    DoItem, DoCmd, DoMngr, Action, FindReplaceCtx, MergeRedoAndUndoGuard,
    TocEntry, cfg::HeightMode};
use crate::util::{enc_content, dec_content};
use eframe::egui::{Frame, NumExt, Pos2, Rect, Sense, Ui, Vec2};
use eframe::egui::epaint::text::LayoutJob;
use regex::Regex;
use arboard::Clipboard;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::UrlInfo;
use self::cache_linepos::line_scroll_height;
use self::index::{IndexCacheKind, IndexCacheMgr, RebuildMode, RebuildReason};

/// 程序化纵向滚动时目标行的对齐方式。
///
/// - [`ScrollToLineMode::Top`]：将该行顶端对齐视口顶端。
/// - [`ScrollToLineMode::Bottom`]：将该行底端贴近编辑区底端，用于光标下移超出时的「最小」滚动。
/// - [`ScrollToLineMode::Center`]：将该行在垂直方向置于编辑区（视口）中央（按行高缓存推算的行几何中心）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrollToLineMode {
    Top(usize),
    Bottom(usize),
    Center(usize),
}

pub mod index;
pub mod cache_outline;
mod cache_filter;
mod cache_linepos;
mod table;
mod cache_code;
mod cache_table;

#[derive(Clone, Debug)]
pub struct State {
    top_line: usize,
    bottom_line: usize,
    scroll_to_line: Option<ScrollToLineMode>,
    scroll_to_rect: Option<Rect>,

    cursor1: Cursor,
    cursor2: Cursor,
    cursor2_bak: Option<Cursor>,
    cursor_show_time: u64, //milliseconds
    cursor_show_bool: bool,
    selecting: bool,

    change_current_tick: u64,
    change_last_save_tick: u64,
    change_last_swap_tick: u64,
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
            change_last_swap_tick: 0,
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
    state: State, //mark somthing has changed after on_event
    state_cmp: State, 
    area: Area,
    /// 每个编辑器实例唯一，用于 egui ScrollArea 的 id_salt，避免多标签共用滚动记忆
    scroll_area_id: u64,
    open_time: u128,
    request_focus: bool,
    do_mngr: Rc<RefCell<DoMngr>>,
    cmd_list: Vec<Action>,
    /// 插件等无 Ui 路径下发的编辑器 Action，在 layout 中延迟执行
    deferred_editor_actions: Vec<Action>,
    find_cache: FindCache,
    find_param: FindReplaceCtx,
    same_cache: FindCache,
    clipboard: Clipboard,
    expanded_ctx: ExpandedCtx,
    view_height: Option<f32>, // 保存上次布局后的高度，用于下一帧显示设置动态高度
    index_cache_mgr: IndexCacheMgr,
    first_layout_rebuild_requested: bool,
    /// 当前 `draw_all_pgh`  pass 内是否检测到行高变化。
    layout_height_changed_in_pass: bool,
    layout_height_change_start: usize,
    layout_height_change_end: usize,
}


impl Ctx {
    /// Create a new Ctx with default configuration
    pub fn new() -> Self {
        let font_size = 17.0;
        let mut ctx = Self {
            cfg: EditCfg::new(font_size, false, None, HeightMode::fix_max()),
            pgh_views: vec![],
            patch_num: 160,
            state: State::default(),
            state_cmp: State::default(),
            area: Area::default(),
            scroll_area_id: CTX_SCROLL_AREA_ID_SEQ.fetch_add(1, Ordering::Relaxed),
            open_time: 0,
            request_focus: false,
            do_mngr: Rc::new(RefCell::new(DoMngr::new())),
            cmd_list: vec![],
            deferred_editor_actions: vec![],
            find_cache: FindCache::new(),
            find_param: FindReplaceCtx::new(),
            same_cache: FindCache::new(),
            clipboard: Clipboard::new().unwrap(),   //todo: unwrap unsafe
            expanded_ctx: ExpandedCtx::new(),
            view_height: None,
            index_cache_mgr: IndexCacheMgr::default(),
            first_layout_rebuild_requested: false,
            layout_height_changed_in_pass: false,
            layout_height_change_start: 0,
            layout_height_change_end: 0,
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
        // 测试环境常直接 `with_text` 后执行 action；这里补齐 TableRow 元数据，
        // 避免在首帧布局前 `table_info_of_line` 为空导致表格动作退化为普通文本删除。
        if is_markdown {
            self.ensure_table_row_blocks_metadata();
        }
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
                let segment = pgh_view.first_same_y_text_segment(pos);
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
            let _line_no = top_line + i;
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

    fn is_document_fully_selected(&self) -> bool {
        if !self.is_selected() || self.pgh_views.is_empty() {
            return false;
        }
        let min = std::cmp::min(self.cursor1(), self.cursor2());
        if min != (0, 0, 0).into() {
            return false;
        }
        let max = std::cmp::max(self.cursor1(), self.cursor2());
        let doc_end = self.cursor_check(&(usize::MAX, usize::MAX, usize::MAX).into());
        max == doc_end
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
        let _culumn = cursor.culumn;

        if line >= self.pgh_views.len() || line < self.state.top_line {
            return None;
        }
        if let Some(rect) = self.pgh_views[line].pos_from_cursor(cursor) {
            return Some(rect);
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
                            let expanded = if let Some(ti) = self.table_info_of_line(ln) {
                                r.expand2(Vec2::new(ti.spacing_x / 2.0, 0.0))
                            } else {
                                r
                            };

                            let text_len = p.get_segment_text(col).chars().count();
                            let Some((st, en)) = PghView::table_row_column_block_cell_span(
                                ln, col, line_lo, line_hi, col_lo, col_hi, &orig_min, &orig_max, text_len,
                            ) else {
                                continue;
                            };
                            // 空单元格(text_len==0)在列块选区内也应整格高亮；
                            // 仅非空单元格的零宽范围才跳过。
                            if text_len > 0 && st == en {
                                continue;
                            }

                            let mut hl = expanded;
                            if st > 0 {
                                let c_st: Cursor = (ln, col, st).into();
                                if let Some(st_rect) = self.get_pos_from_cursor(&c_st) {
                                    hl.min.x = hl.min.x.max(st_rect.min.x);
                                }
                            }
                            if en < text_len {
                                let c_en: Cursor = (ln, col, en).into();
                                if let Some(en_rect) = self.get_pos_from_cursor(&c_en) {
                                    hl.max.x = hl.max.x.min(en_rect.max.x);
                                }
                            }

                            if hl.max.x > hl.min.x {
                                rects.push(hl);
                            }
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

    pub fn select_line_to_next(&mut self, line_no: usize) {
        if self.pgh_views.is_empty() {
            return;
        }
        let line_no = line_no.min(self.pgh_views.len() - 1);
        if self
            .pgh_views
            .get(line_no)
            .is_some_and(|p| p.is_table_row())
        {
            self.state.cursor2 = self.cursor_check(&(line_no, 0, 0).into());
            self.select_line_at_cursor();
            return;
        }
        self.state.cursor1 = self.cursor_check(&(line_no, 0, 0).into());
        let next_line = line_no + 1;
        if next_line < self.pgh_views.len() {
            self.state.cursor2 = self.cursor_check(&(next_line, 0, 0).into());
        } else if let Some(pghview) = self.pgh_views.get(line_no) {
            self.state.cursor2 = self.cursor_check(
                &(line_no, pghview.max_segment(), usize::MAX).into(),
            );
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

        if self
            .pgh_views
            .get(cursor.line_no)
            .map(|p| p.is_render_hidden())
            .unwrap_or(false)
        {
            cursor.line_no = self.next_visible_line(cursor.line_no, 1);
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
                    let same_block = prev.is_table_row() && prev.table_key == pgh_view.table_key;
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
        let mut new = self.cursor_check(&new);
        if self
            .pgh_views
            .get(new.line_no)
            .map(|p| p.is_render_hidden())
            .unwrap_or(false)
        {
            new.line_no = self.next_visible_line(new.line_no, -1);
            new = self.cursor_check(&new);
        }
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
        let mut new = self.cursor_check(&new);
        if self
            .pgh_views
            .get(new.line_no)
            .map(|p| p.is_render_hidden())
            .unwrap_or(false)
        {
            new.line_no = self.next_visible_line(new.line_no, 1);
            new = self.cursor_check(&new);
        }
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
        let end = self.index_cache_mgr.scroll_layout_patch_end().min(len);
        end.max(self.state.top_line.saturating_add(1))
            .min(len)
    }

    pub(crate) fn set_layout_patch_end(&mut self, end: usize) {
        self.index_cache_mgr.set_scroll_layout_patch_end(end);
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
        true
    }

    #[inline]
    pub(crate) fn scroll_cum_at(&self, i: usize) -> f32 {
        self.index_cache_mgr.scroll_cum_at(i)
    }

    pub(crate) fn scroll_bottom_padding(&self) -> f32 {
        if self.is_dynamic_height() {
            0.0
        } else {
            (self.edit_rect().height() / 2.0).max(0.0)
        }
    }

    /// 行偏移由 `IndexCacheMgr` 在分帧 tick 中刷新。
    pub(crate) fn scroll_offset_y_for_line(&self, line_no: usize) -> f32 {
        let n = self.line_num();
        let line_no = line_no.min(n);
        self.scroll_cum_at(line_no)
    }

    /// 在 [`draw_all_pgh`](crate::medit::layout::Layout::draw_all_pgh) 开始时调用，重置本 pass 的行高变化追踪。
    pub(crate) fn begin_layout_height_pass(&mut self) {
        self.layout_height_changed_in_pass = false;
    }

    /// 记录单行 layout 后的滚动高度；若相对上次有变化则标记本 pass 需要重建索引。
    pub(crate) fn record_line_scroll_height_after_layout(&mut self, line_no: usize) {
        if let Some(pv) = self.pgh_views.get_mut(line_no) {
            if pv.is_render_hidden() {
                let oh = pv.last_scroll_height;
                if oh.abs() < 0.5 {
                    return;
                }
                pv.last_scroll_height = 0.0;
                self.note_layout_height_change(line_no);
                return;
            }
            if let Some(r) = pv.rect() {
                let nh = r.height().at_least(1.0);
                let oh = pv.last_scroll_height;
                let delta = (nh - oh).abs();
                if oh > 0.0 && delta < 0.5 {
                    return;
                }
                log::debug!(
                    "layout_height_changed line={} type={:?} old_h={:.3} new_h={:.3} delta={:.3}",
                    line_no,
                    pv.pgh_type,
                    oh,
                    nh,
                    delta
                );
                pv.last_scroll_height = nh;
                self.note_layout_height_change(line_no);
            }
        }
    }

    fn note_layout_height_change(&mut self, line_no: usize) {
        let n = self.line_num();
        let start = line_no.min(n);
        let end = start
            .saturating_add(self.patch_num().max(1))
            .min(n.max(1));
        if !self.layout_height_changed_in_pass {
            self.layout_height_change_start = start;
            self.layout_height_change_end = end;
            self.layout_height_changed_in_pass = true;
        } else {
            self.layout_height_change_start = self.layout_height_change_start.min(start);
            self.layout_height_change_end = self.layout_height_change_end.max(end);
        }
    }

    /// 在 `draw_all_pgh` 结束后调用：若本 pass 存在行高变化，触发一次 ScrollLayout 索引重建。
    pub(crate) fn rebuild_index_if_layout_heights_changed(&mut self) {
        if !self.layout_height_changed_in_pass {
            return;
        }
        self.layout_height_changed_in_pass = false;
        let total = self.line_num();
        if total == 0 {
            return;
        }
        let start = self.layout_height_change_start.min(total);
        let end = self.layout_height_change_end
            .max(start.saturating_add(1))
            .min(total);
        log::info!(
            "layout_height_rebuild visible=[{}, {}) total_lines={}",
            start,
            end,
            total
        );
        self.request_rebuild_index_kinds(
            start,
            end,
            RebuildReason::LayoutHeightChanged,
            RebuildMode::Deferred,
            &[IndexCacheKind::ScrollLayout],
        );
    }

    /// 可视区行范围；FindFilter 启用时用当前隐藏状态实时推算，避免 LinePos 缓存滞后。
    pub(crate) fn scroll_lines_visible_for_viewport(
        &self,
        viewport: &Rect,
        margin: f32,
    ) -> (usize, usize) {
        let n = self.line_num();
        if n == 0 {
            return (0, 0);
        }
        if self.index_cache_mgr.find_filter_is_active() {
            return self.scroll_lines_visible_for_viewport_live(viewport, margin, n);
        }
        self.index_cache_mgr
            .scroll_lines_visible_for_viewport(viewport, margin, n)
    }

    fn scroll_lines_visible_for_viewport_live(
        &self,
        viewport: &Rect,
        margin: f32,
        n: usize,
    ) -> (usize, usize) {
        let v0 = viewport.min.y - margin;
        let v1 = viewport.max.y + margin;
        if v1 <= 0.0 {
            return (0, 1.min(n));
        }
        let total = self.live_scroll_cum_at(n);
        if v0 >= total {
            let s = n.saturating_sub(1);
            return (s, n);
        }
        let low = v0.max(0.0);
        let high = v1.max(0.0);

        let mut cum = 0.0f32;
        let mut start = 0usize;
        for i in 0..n {
            if cum <= low {
                start = i;
            }
            cum += line_scroll_height(&self.pgh_views, i, self.font_heigh());
        }

        let mut end = (start + 1).min(n);
        cum = self.live_scroll_cum_at(start);
        for i in start..n {
            cum += line_scroll_height(&self.pgh_views, i, self.font_heigh());
            end = i + 1;
            if cum >= high {
                break;
            }
        }
        (start, end.max(start.saturating_add(1)).min(n))
    }

    #[inline]
    fn live_scroll_cum_at(&self, i: usize) -> f32 {
        let n = i.min(self.line_num());
        let font_h = self.font_heigh();
        (0..n)
            .map(|line| line_scroll_height(&self.pgh_views, line, font_h))
            .sum()
    }

    fn live_visible_scroll_height_between(&self, start: usize, end: usize) -> f32 {
        let n = self.line_num();
        if n == 0 || start >= end {
            return 0.0;
        }
        let font_h = self.font_heigh();
        (start..end.min(n))
            .map(|line| line_scroll_height(&self.pgh_views, line, font_h))
            .sum()
    }

    /// 将布局绘制区间 `[start, end)` 向后扩展，直到累计可见高度（隐藏行计 0）至少覆盖 `min_height`。
    /// 用于 FindFilter / 大纲折叠时，避免 `patch_num` 行数上限内大量隐藏行导致视口下方留白。
    pub(crate) fn extend_layout_patch_end(
        &self,
        start: usize,
        initial_end: usize,
        min_height: f32,
    ) -> usize {
        let n = self.line_num();
        if n == 0 || start >= n {
            return start;
        }
        let font_h = self.font_heigh();
        let mut end = initial_end.min(n).max(start.saturating_add(1));
        let mut visible_h = self.live_visible_scroll_height_between(start, end);
        while end < n && visible_h < min_height {
            visible_h += line_scroll_height(&self.pgh_views, end, font_h);
            end += 1;
        }
        end.max(start.saturating_add(1)).min(n)
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

    fn cursor_pghviews(&self, cursor1: &Cursor, cursor2: &Cursor) -> Vec<(usize, &PghView)> {
        let mut range = vec![];
        if self.pgh_views.len() == 0 {
            return range;
        }
        let min = std::cmp::min(*cursor1, *cursor2);
        let max = std::cmp::max(*cursor1, *cursor2);
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

    fn current_cursor_pghviews(&self) -> Vec<(usize, &PghView)> {
        self.cursor_pghviews(&self.cursor1(), &self.cursor2())
    }

    fn get_raw_text_by_cursor_range(&self, cursor1: &Cursor, cursor2: &Cursor, is_raw: bool) -> String {
        let tr_rect = self.table_row_block_column_rect(cursor1, cursor2);
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
                        self,
                        line_no,
                        cursor1,
                        cursor2,
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
        if let Some(code_sel) = self.code_row_block_selected_markdown(cursor1, cursor2, is_raw) {
            return code_sel;
        }
        let mut s = "".to_string();
        let mut prev_selected_table_col_count: Option<usize> = None;
        if self.pgh_views.is_empty() {
            return s;
        }
        let min = std::cmp::min(*cursor1, *cursor2);
        let max = std::cmp::max(*cursor1, *cursor2);
        let first = min.line_no.at_most(self.pgh_views.len() - 1);
        let last = max
            .line_no
            .at_least(first)
            .at_most(self.pgh_views.len() - 1);
        let mut line_no = first;
        let mut i = 0usize;
        while line_no <= last {
            let pgh_view = &self.pgh_views[line_no];
            if self.cfg().is_markdown && pgh_view.is_code_row() {
                if let Some((blk_s, blk_e)) = self.code_row_block_range(line_no) {
                    if line_no == blk_s
                        && self.block_lines_fully_selected_for_fence(cursor1, cursor2, blk_s, blk_e)
                    {
                        let mut body = String::new();
                        for ln in blk_s..=blk_e {
                            if let Some(p) = self.get_line(ln) {
                                if ln > blk_s {
                                    body.push('\n');
                                }
                                body.push_str(&p.get_text());
                            }
                        }
                        let piece = if is_raw {
                            body
                        } else {
                            let info_line = self.markdown_export_code_fence_info_line(blk_s);
                            format!("```{}\n{}\n```", info_line, body)
                        };
                        if i > 0 {
                            s.push('\n');
                        }
                        s.push_str(&piece);
                        i += 1;
                        prev_selected_table_col_count = None;
                        line_no = blk_e.saturating_add(1);
                        continue;
                    }
                }
            }
            let mut selected = pgh_view.select(self, line_no, cursor1, cursor2, is_raw);
            if !is_raw && pgh_view.is_table_row() {
                let col_count = self
                    .table_info_of_line(line_no)
                    .map(|ti| ti.col_count)
                    .unwrap_or(0);
                let first_selected_row_in_block = prev_selected_table_col_count != Some(col_count);
                if first_selected_row_in_block && !selected.contains('\n') {
                    if let Some(sep) = Self::table_markdown_separator_from_pipe_row(&selected) {
                        selected.push('\n');
                        selected.push_str(&sep);
                    }
                }
                prev_selected_table_col_count = Some(col_count);
            } else {
                prev_selected_table_col_count = None;
            }
            if i > 0 {
                s.push('\n');
            }
            s.push_str(&selected);
            i += 1;
            line_no += 1;
        }
        s
    }

    fn get_selected_raw_text(&self, is_raw: bool) -> String {
        self.get_raw_text_by_cursor_range(&self.cursor1(), &self.cursor2(), is_raw)
    }

    pub fn get_text_by_cursor_range(&self, cursor1: Cursor, cursor2: Cursor) -> String {
        self.get_raw_text_by_cursor_range(&cursor1, &cursor2, false)
    }


    pub fn get_selected_text(&self) -> String {
        self.get_selected_raw_text(false)
    }

    fn get_line_select_text(&self, line_no: usize) -> String {
        if let Some(pgh_view) = self.pgh_views.get(line_no) {
            let cursor1: Cursor = (line_no, 0, 0).into();
            let max_segment = pgh_view.max_segment();
            let cursor2: Cursor = (line_no, max_segment, usize::MAX).into();
            pgh_view.select(self, line_no, &cursor1, &cursor2, false)
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
                    let info_line = self.markdown_export_code_fence_info_line(blk_s);
                    let mut body = String::new();
                    for ln in blk_s..=blk_e {
                        if ln > blk_s {
                            body.push('\n');
                        }
                        body.push_str(&self.pgh_views[ln].get_text());
                    }
                    let selected = format!("```{}\n{}\n```", info_line, body);
                    if line_no > 0 {
                        s.push('\n');
                    }
                    s.push_str(&selected);
                    line_no = blk_e + 1;
                    continue;
                } else {
                    let selected = pgh_view.select(self, line_no, &cursor1, &cursor2, false);
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
                s += &pgh_view.select(self, line_no, &cursor1, &cursor2, true);
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
        if let HeightMode::Dynamic { min, max: _ } = self.cfg().height_mode {
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
        let fold_collapsed = self
            .pgh_views
            .get(line_no)
            .map(|org| {
                org.outline_fold_collapsed().unwrap_or(false)
                    || self.outline_section_content_appears_folded(line_no)
            })
            .unwrap_or(false);
        let resync_fold_children = fold_collapsed
            && !self.outline_section_content_appears_folded(line_no);

        if let Some(org_pgh) = self.pgh_views.get_mut(line_no) {
            let hidden_by_level = org_pgh.hidden_by_level;
            let hidden_by_find_filter = org_pgh.hidden_by_find_filter;
            let heading_level = org_pgh.heading_level;
            org_pgh.pgh_type = pghview.pgh_type.clone();
            org_pgh.pgh = pghview.pgh.clone();
            org_pgh.table_key = pghview.table_key;
            org_pgh.code_key = pghview.code_key;
            org_pgh.spacing_top = pghview.spacing_top;
            org_pgh.spacing_bottom = pghview.spacing_bottom;
            org_pgh.code_lang = pghview.code_lang.clone();
            org_pgh.hidden_by_level = hidden_by_level;
            org_pgh.hidden_by_find_filter = hidden_by_find_filter;
            org_pgh.heading_level = pghview.heading_level.or(heading_level);
            if org_pgh.pgh_type == PghType::Heading {
                org_pgh.ensure_outline_fold_segment(fold_collapsed);
            }
            //org_pgh.expanded_text_id = pghview.expanded_text_id;
        }

        if resync_fold_children {
            self.set_outline_folded(line_no, true);
        }
    }

    pub fn update_pgh_segment_job(&mut self, line_no: usize, segment: usize, job: Option<LayoutJob>) {
        if let Some(org_pgh) = self.pgh_views.get_mut(line_no) {
            if let Some(pgh_segment) = org_pgh.pgh.get_mut(segment) {
                pgh_segment.item.layout_job_update(job);
            }
        }
    }

    pub fn merge_line_pgh_rect_from_segments(&mut self, line_no: usize, anchor_rect: Rect) {
        if let Some(pgh_view) = self.pgh_views.get_mut(line_no) {
            pgh_view.merge_pgh_rect_from_segments(anchor_rect);
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
        let total = expanded_ctx.line_num();
        if total > 0 {
            let end = expanded_ctx.patch_num().max(1).min(total);
            expanded_ctx.request_rebuild_index_for_expanded(0, end);
        }

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
        let mut code_key = self.get_line(s).and_then(|p| p.code_key).unwrap_or(0);
        if code_key == 0 {
            code_key = self.alloc_code_key();
        }
        let mut base = self
            .index_cache_mgr
            .code_cache()
            .code_info_cloned_by_key(code_key)
            .unwrap_or_default();
        base.code_key = code_key;
        base.head_line_no = s;
        base.row_count = n;
        base.lang = self.get_line(s).and_then(|p| p.code_lang.clone());
        self.index_cache_mgr
            .code_cache_mut()
            .upsert_code_info_by_key(code_key, base.clone());
        let top = self.cfg().spacing.code.top;
        let bottom = self.cfg().spacing.code.bottom;
        for (i, ln) in (s..=e).enumerate() {
            if let Some(p) = self.get_line_mut(ln) {
                if !p.is_code_row() {
                    continue;
                }
                p.code_key = Some(code_key);
                if ln == s {
                    p.code_lang = base.lang.clone();
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

    pub(crate) fn alloc_code_key(&mut self) -> CodeKey {
        self.index_cache_mgr.code_cache_mut().alloc_code_key()
    }

    pub fn code_key_of_line(&self, line_no: usize) -> Option<CodeKey> {
        self.get_line(line_no).and_then(|p| p.code_key)
    }

    pub(crate) fn code_block_info_by_key(
        &self,
        code_key: CodeKey,
    ) -> Option<&self::cache_code::CodeBlockInfo> {
        self.index_cache_mgr.code_cache().code_info_by_key(code_key)
    }

    pub(crate) fn code_plantuml_image_of_line(&mut self, line_no: usize) -> Option<String> {
        let flash_lines = self.index_cache_mgr.code_cache_mut().poll_render_results();
        for ln in flash_lines {
            self.line_flash_tick(ln);
        }
        let code_key = self.code_key_of_line(line_no)?;
        self.code_block_info_by_key(code_key)
            .and_then(|info| info.plantuml.image_url())
    }

    /// 复制/导出用：围栏首行内容。PlantUML 且 `code_cache` 已有渲染图时，在语言后附加 ` file://...`（与 `plantuml.image_url` 一致，不 poll 异步队列）。
    pub(crate) fn markdown_export_code_fence_info_line(&self, block_start_line: usize) -> String {
        let lang = self
            .get_line(block_start_line)
            .and_then(|p| p.code_lang.as_deref())
            .unwrap_or("");
        if lang.eq_ignore_ascii_case("plantuml") {
            if let Some(key) = self.code_key_of_line(block_start_line) {
                if let Some(url) = self
                    .code_block_info_by_key(key)
                    .and_then(|info| info.plantuml.image_url())
                {
                    return format!("{} {}", lang, url);
                }
            }
        }
        lang.to_string()
    }

    pub fn code_info_of_line(&self, line_no: usize) -> Option<CodeInfo> {
        let p = self.get_line(line_no)?;
        if let Some(code_key) = p.code_key {
            if let Some(info) = self.code_block_info_by_key(code_key) {
                let code_row_index = line_no.saturating_sub(info.head_line_no);
                return Some(CodeInfo {
                    code_row_index,
                    code_total_rows: info.row_count,
                });
            }
        }
        let (s, e) = self.code_row_block_range(line_no)?;
        Some(CodeInfo {
            code_row_index: line_no.saturating_sub(s),
            code_total_rows: e.saturating_sub(s).saturating_add(1),
        })
    }

    /// 选区是否完整包含连续 `CodeRow` 块 `[blk_s, blk_e]`（可与前后正文同属一次复制）。
    /// 与 `code_row_block_selected_markdown` 中「仅含代码块」时的列放宽一致。
    fn block_lines_fully_selected_for_fence(
        &self,
        c1: &Cursor,
        c2: &Cursor,
        blk_s: usize,
        blk_e: usize,
    ) -> bool {
        let c_lo = std::cmp::min(c1, c2);
        let c_hi = std::cmp::max(c1, c2);
        if c_lo.line_no > blk_s || c_hi.line_no < blk_e {
            return false;
        }
        let block_start_c = Cursor {
            line_no: blk_s,
            segment: 0,
            culumn: 0,
        };
        let Some(last_line) = self.get_line(blk_e) else {
            return false;
        };
        let block_end_c = last_line.end_cursor_of_line(blk_e);
        let sel_min = std::cmp::min(c1, c2);
        let sel_max = std::cmp::max(c1, c2);
        let spans_only_block_lines = c_lo.line_no == blk_s && c_hi.line_no == blk_e;
        if spans_only_block_lines {
            return true;
        }
        if c_lo.line_no == blk_s && sel_min > &block_start_c {
            return false;
        }
        if c_hi.line_no == blk_e && sel_max < &block_end_c {
            return false;
        }
        true
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
        // 选区可能从非 CodeRow 行开始；用选区内首行 CodeRow 定位块，与 `pgh_code` 里整段选中判断一致
        let anchor_line = if self.get_line(lo.line_no).is_some_and(|p| p.is_code_row()) {
            lo.line_no
        } else {
            (lo.line_no..=hi.line_no).find(|&ln| self.get_line(ln).is_some_and(|p| p.is_code_row()))?
        };
        let (blk_s, blk_e) = self.code_row_block_range(anchor_line)?;
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
        // 行号已覆盖整块时，不再要求列与 `end_cursor_of_line` 完全对齐（避免复制时缺 ``` 围栏）
        let spans_all_lines_of_block = lo.line_no == blk_s && hi.line_no == blk_e;
        if !spans_all_lines_of_block && (sel_min > &block_start_c || sel_max < &block_end_c) {
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
        let info_line = self.markdown_export_code_fence_info_line(blk_s);
        Some(format!("```{}\n{}\n```", info_line, body))
    }

    /// 物理行 `deleted_line_index` 已从 `pgh_views` 删除后，若其上下仍存在同一 `TableRow` 块，重算该块元数据。

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

    pub(crate) fn request_rebuild_index_on_first_layout(
        &mut self,
        visible_start: usize,
        visible_end: usize,
    ) {
        if self.first_layout_rebuild_requested {
            return;
        }
        self.first_layout_rebuild_requested = true;
        // ScrollLayout 由 draw_all_pgh 结束后的行高 pass 负责，此处不重复入队。
        self.request_rebuild_index_kinds(
            visible_start,
            visible_end,
            RebuildReason::FirstLayout,
            RebuildMode::Deferred,
            Self::index_kinds_for_line_structure(self, false),
        );
    }

    pub(crate) fn request_rebuild_index_for_layout(
        &mut self,
        visible_start: usize,
        visible_end: usize,
    ) {
        let total = self.line_num();
        if total == 0 {
            return;
        }
        self.request_rebuild_index_kinds(
            visible_start,
            visible_end,
            RebuildReason::LayoutHeightChanged,
            RebuildMode::Deferred,
            &[IndexCacheKind::ScrollLayout],
        );
    }

    pub(crate) fn request_rebuild_index_for_content(&mut self) {
        let total = self.line_num();
        if total == 0 {
            return;
        }
        let start = self.top_line();
        let end = self.patch_end().max(start.saturating_add(1)).min(total);
        self.request_rebuild_index_kinds(
            start,
            end,
            RebuildReason::ContentChanged,
            RebuildMode::Deferred,
            Self::index_kinds_for_line_structure(self, true),
        );
    }

    pub(crate) fn request_rebuild_index_for_expanded(
        &mut self,
        visible_start: usize,
        visible_end: usize,
    ) {
        self.request_rebuild_index_kinds(
            visible_start,
            visible_end,
            RebuildReason::ExpandedCtxUpdated,
            RebuildMode::Deferred,
            Self::index_kinds_for_line_structure(self, false),
        );
    }

    /// 行增删等结构变更所需的 cache 组合（Markdown：Table/Code/Toc；FindFilter/ScrollLayout 按需）。
    fn index_kinds_for_line_structure(ctx: &Self, with_scroll_layout: bool) -> &'static [IndexCacheKind] {
        use IndexCacheKind::{Code, FindFilter, ScrollLayout, Table, Toc};
        let is_markdown = ctx.cfg().is_markdown;
        let find_filter = ctx.index_cache_mgr.find_filter_is_active();
        match (is_markdown, find_filter, with_scroll_layout) {
            (true, true, true) => &[Table, Code, Toc, FindFilter, ScrollLayout],
            (true, true, false) => &[Table, Code, Toc, FindFilter],
            (true, false, true) => &[Table, Code, Toc, ScrollLayout],
            (true, false, false) => &[Table, Code, Toc],
            (false, true, true) => &[FindFilter, ScrollLayout],
            (false, true, false) => &[FindFilter],
            (false, false, true) => &[ScrollLayout],
            (false, false, false) => &[],
        }
    }

    pub(crate) fn request_rebuild_index_kinds(
        &mut self,
        visible_start: usize,
        visible_end: usize,
        reason: RebuildReason,
        mode: RebuildMode,
        kinds: &[IndexCacheKind],
    ) {
        let total = self.line_num();
        if total == 0 || kinds.is_empty() {
            return;
        }
        if matches!(mode, RebuildMode::Immediate) {
            let mut mgr = std::mem::take(&mut self.index_cache_mgr);
            mgr.request_rebuild_kinds(
                Some(self),
                visible_start,
                visible_end,
                total,
                reason,
                mode,
                kinds,
            );
            self.index_cache_mgr = mgr;
        } else {
            self.index_cache_mgr.request_rebuild_kinds(
                None,
                visible_start,
                visible_end,
                total,
                reason,
                mode,
                kinds,
            );
        }
    }

    pub fn on_content_change(&mut self) {
        self.content_change_state();
    }

    fn line_basic_changed_by_do_items(items: &[DoItem]) -> bool {
        let mut line_delta: isize = 0;
        for item in items {
            match item {
                DoItem::Insert(_) => {
                    line_delta += 1;
                }
                DoItem::Delete(_) => {
                    line_delta -= 1;
                }
                DoItem::ReplaceAll(_) => {
                    return true;
                }
                DoItem::Update(_) => {
                    continue;
                }
            }
        }
        line_delta != 0
    }

    pub(crate) fn line_basic_changed_by_last_merged_do(&self, is_undo: bool) -> bool {
        let do_mngr = self.do_mngr.borrow();
        let target_idx = if is_undo {
            do_mngr.index
        } else {
            do_mngr.index.saturating_sub(1)
        };
        let Some(target_cmd) = do_mngr.do_list.get(target_idx) else {
            return false;
        };
        Self::line_basic_changed_by_do_items(&target_cmd.redo.items)
    }

    pub(crate) fn request_rebuild_index_if_needed(&mut self, line_basic_changed: bool) {
        if !line_basic_changed {
            return;
        }
        self.request_rebuild_index_for_content();
    }

    /// 立即在本调用内完成索引重建（通过 `take` 临时取得 [`IndexCacheMgr`]，等同单次极大预算的 `tick`）。
    ///
    /// 内部仍走「可视区优先 → 全量收尾」两阶段；可视区间由当前 [`Self::top_line`]、[`Self::patch_end`] 推导，
    /// 仅影响第一阶段扫描顺序，第二阶段仍会扫完整篇以更新大纲等全量索引。
    pub(crate) fn request_rebuild_index_immediate(&mut self, reason: RebuildReason) {
        let total = self.line_num();
        if total == 0 {
            return;
        }
        let vis_lo = self.top_line();
        let vis_hi = self
            .patch_end()
            .max(vis_lo.saturating_add(1))
            .min(total);
        self.request_rebuild_index_kinds(
            vis_lo,
            vis_hi,
            reason,
            RebuildMode::Immediate,
            Self::index_kinds_for_line_structure(self, true),
        );
    }

    pub(crate) fn rebuild_index_tick(&mut self, visible_start: usize, visible_end: usize) {
        let mut manager = std::mem::take(&mut self.index_cache_mgr);
        manager.tick(self, visible_start, visible_end);
        self.index_cache_mgr = manager;
    }

    pub fn has_rebuild_index_task(&self) -> bool {
        self.index_cache_mgr.has_pending_work()
    }

    pub fn is_find_filter_searching(&self) -> bool {
        self.index_cache_mgr.has_active_find_filter_task()
    }

    pub fn find_filter_is_active(&self) -> bool {
        self.index_cache_mgr.find_filter_is_active()
    }

    pub fn find_filter_visible_line_count(&self) -> usize {
        self.index_cache_mgr.find_filter_visible_line_count()
    }

    pub fn find_filter_search_progress(&self) -> f32 {
        self.index_cache_mgr
            .find_filter_task_progress(self.line_num())
            .unwrap_or(0.0)
    }

    pub fn rebuild_index_task_gen(&self) -> Option<u64> {
        self.index_cache_mgr.active_task_gen()
    }

    pub fn rebuild_index_latest_gen(&self) -> u64 {
        self.index_cache_mgr.latest_gen()
    }

    pub fn clean_change_tick(&mut self) {
        self.state.change_last_save_tick = self.state.change_current_tick;
    }

    pub fn clean_swap_tick(&mut self) {
        self.state.change_last_swap_tick = self.state.change_current_tick;
    }

    pub fn is_content_changed(&self) -> bool {
        self.state.change_current_tick != self.state.change_last_save_tick
    }

    pub fn is_swap_stale(&self) -> bool {
        self.state.change_current_tick != self.state.change_last_swap_tick
    }

    /// Content was loaded from a swap file (newer than source on disk).
    /// Mark disk-save as dirty; swap is already up to date.
    pub fn mark_unsaved_from_swap(&mut self) {
        self.content_change_state();
        self.clean_swap_tick();
    }

    pub fn toc_entries(&self) -> &[TocEntry] {
        self.index_cache_mgr.toc_cache().entries.as_slice()
    }

    pub fn find_outline_line_range_by_path(&self, outline_path: &str) -> Option<(usize, usize)> {
        let entries = self.toc_entries();
        if entries.is_empty() {
            return None;
        }

        let mut stack: Vec<&TocEntry> = Vec::new();
        let mut target: Option<&TocEntry> = None;

        for entry in entries.iter() {
            while stack.last().is_some_and(|last| last.level >= entry.level) {
                stack.pop();
            }
            stack.push(entry);

            let path = stack
                .iter()
                .map(|item| item.title.as_str())
                .collect::<Vec<_>>()
                .join(" / ");
            if path == outline_path {
                target = Some(entry);
            }
        }

        let target = target?;
        let end_line = entries
            .iter()
            .find(|entry| entry.line_no > target.line_no && entry.level <= target.level)
            .map(|entry| entry.line_no.saturating_sub(1))
            .unwrap_or_else(|| self.line_num().saturating_sub(1));
        Some((target.line_no, end_line))
    }

    /// 按大纲路径替换章节全文：先立即重建索引再解析路径，写入后再重建（供插件命令与 `Action::execute(set_outline_content)` 共用）。
    pub(crate) fn set_outline_content_by_path(&mut self, outline_path: &str, content: &str) {
        if self.line_num() > 0 {
            self.request_rebuild_index_immediate(RebuildReason::ContentChanged);
        }

        let Some((start_line, end_line)) = self.find_outline_line_range_by_path(outline_path) else {
            log::warn!(
                "Outline path not found for set_outline_content: {}",
                outline_path
            );
            return;
        };

        let start_cursor = self.cursor_check(&(start_line, 0, 0).into());
        let end_cursor = self.cursor_at_logical_line_end(end_line);
        self.set_cursor1(start_cursor);
        self.set_cursor2(end_cursor);
        self.insert(content.to_string());
        self.set_cursor1_reset();
    }

    pub fn cursor_at_logical_line_end(&self, line_no: usize) -> Cursor {
        let line_text = self.get_line_text(line_no);
        let n = line_text.chars().count();
        if let Some(pgh) = self.get_line(line_no) {
            let raw = pgh.text_char_index_to_cursor(n, line_no);
            return self.cursor_check(&raw);
        }
        (line_no, 0, 0).into()
    }

    pub fn toc_entry_for_line(&self, line_no: usize) -> Option<&TocEntry> {
        self.index_cache_mgr.toc_cache().entry_for_line(line_no)
    }

    /// 自 `line_no` 起向上（含本行）第一个 Markdown 标题行。
    pub fn nearest_heading_line_at_or_before(&self, line_no: usize) -> Option<usize> {
        let start = line_no.min(self.pgh_views.len().saturating_sub(1));
        for ln in (0..=start).rev() {
            if self.is_heading_line(ln) {
                return Some(ln);
            }
        }
        None
    }

    /// 自 `line_no` 起最近的目录项：先 [`nearest_heading_line_at_or_before`]，再 [`toc_entry_for_line`]。
    ///
    /// 与仅对 `toc_entries()` 做 `line_no <= cursor` 的反向查找相比，以正文里的标题行为准，避免目录缓存滞后时误选上一节。
    pub fn toc_entry_nearest_at_or_before(&self, line_no: usize) -> Option<&TocEntry> {
        let h_line = self.nearest_heading_line_at_or_before(line_no)?;
        self.toc_entry_for_line(h_line)
    }

    /// 当前光标所在目录 **同级**（`section_number` 点数一致）、且同属一章（首段一致）的标题行（文档序）。
    ///
    /// 向上找到光标所在最近标题的目录项：以其编号首段为章前缀（如 `3.1.2` → `3`），以编号分段数为层级深度；
    /// 只收集首段相同且分段数与当前标题相同的条目（例如在 `3.1.2` 下则收集所有 `3.*.*`，不包含 `3`、`3.1`）。
    pub fn toc_chapter_descendant_heading_lines(&self, cursor_line: usize) -> Option<Vec<usize>> {
        if !self.cfg().is_markdown {
            return None;
        }
        let cur = self.toc_entry_nearest_at_or_before(cursor_line)?;
        let cur_parts: Vec<&str> = cur
            .section_number
            .split('.')
            .filter(|s| !s.is_empty())
            .collect();
        if cur_parts.is_empty() {
            return None;
        }
        let root_first = cur_parts.first()?.to_string();
        let level_depth = cur_parts.len();
        let mut out: Vec<usize> = Vec::new();
        for e in self.toc_entries() {
            let parts: Vec<&str> = e.section_number.split('.').filter(|s| !s.is_empty()).collect();
            if parts.len() != level_depth {
                continue;
            }
            let Some(first) = parts.first().copied() else {
                continue;
            };
            if first != root_first.as_str() {
                continue;
            }
            out.push(e.line_no);
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    /// 光标所在位置向上最近的标题为根，取其大纲子树内全部标题行号（含该根），文档序。
    ///
    /// 依正文逐行判定，**不依赖** [`Self::toc_entries`] 缓存（避免索引尚未重建时子树为空 / `None`）。
    pub fn toc_outline_subtree_heading_lines(&self, cursor_line: usize) -> Option<Vec<usize>> {
        if !self.cfg().is_markdown {
            return None;
        }
        let h_line = self.nearest_heading_line_at_or_before(cursor_line)?;
        if !self.is_heading_line(h_line) {
            return None;
        }
        let root_level =
            cache_outline::parse_heading_level_for_toc(&self.get_line_text(h_line));
        let mut out = vec![h_line];
        let n = self.line_num();
        let mut line_no = h_line.saturating_add(1);
        while line_no < n {
            if self.is_heading_line(line_no) {
                let lev = cache_outline::parse_heading_level_for_toc(&self.get_line_text(line_no));
                if lev <= root_level {
                    break;
                }
                out.push(line_no);
            }
            line_no += 1;
        }
        Some(out)
    }

    /// 光标所在大纲子树内、作为「叶子章节」的标题行（无更深层子标题），文档序。
    ///
    /// 与 [`Self::toc_outline_subtree_heading_lines`] 一致取子树，再依相邻标题级别判定叶子。
    pub fn toc_outline_subtree_leaf_heading_lines(&self, cursor_line: usize) -> Option<Vec<usize>> {
        let lines = self.toc_outline_subtree_heading_lines(cursor_line)?;
        if lines.is_empty() {
            return None;
        }
        let mut leaves: Vec<usize> = Vec::new();
        for i in 0..lines.len() {
            let hi = lines[i];
            let lev_hi =
                cache_outline::parse_heading_level_for_toc(&self.get_line_text(hi));
            let is_leaf = if i + 1 < lines.len() {
                let lev_next = cache_outline::parse_heading_level_for_toc(
                    &self.get_line_text(lines[i + 1]),
                );
                lev_next <= lev_hi
            } else {
                true
            };
            if is_leaf {
                leaves.push(hi);
            }
        }
        if leaves.is_empty() {
            None
        } else {
            Some(leaves)
        }
    }

    /// 当前目录（光标向上最近标题）下 **子级** 叶子标题行：不含根标题本身，文档序。
    pub fn toc_outline_subtree_leaf_descendant_heading_lines(
        &self,
        cursor_line: usize,
    ) -> Option<Vec<usize>> {
        let root_line = self.nearest_heading_line_at_or_before(cursor_line)?;
        self.toc_outline_subtree_leaf_heading_lines(cursor_line)
            .map(|lines| lines.into_iter().filter(|&ln| ln != root_line).collect())
            .filter(|v: &Vec<usize>| !v.is_empty())
    }

    /// 将大纲子树内各标题的 ATX 级别整体加/减 1（夹在 1..=6；降阶时一级标题保持一级）。
    pub fn adjust_heading_levels_in_outline_subtree(&mut self, delta: i8) {
        let cursor_line = self.cursor2().line_no;
        let Some(lines) = self.toc_outline_subtree_heading_lines(cursor_line) else {
            return;
        };
        for line_no in lines {
            if !self.is_heading_line(line_no) {
                continue;
            }
            let line_text = self.get_line_text(line_no);
            let level = cache_outline::parse_heading_level_for_toc(&line_text) as i32;
            let new_level = (level + delta as i32).clamp(1, 6) as usize;
            let prefix = "#".repeat(new_level) + " ";
            let text_without_heading = Self::remove_heading_prefix(&line_text);
            self.update_line_text(line_no, format!("{}{}", prefix, text_without_heading));
        }
        self.request_rebuild_index_immediate(RebuildReason::ContentChanged);
        self.set_cursor1_reset();
    }

    pub fn delete(&mut self) {
        let (undo_cmd, redo_cmd) = self.delete_func();
        self.push_do(undo_cmd, redo_cmd);
        self.on_content_change();
    }

    fn insert_line(&mut self, line_no: usize, s: String) {
        let mut new_pgh_view = PghView::new_text();
        new_pgh_view.push_text(s, None);
        self.pgh_views.insert(line_no, new_pgh_view);
    }

    pub fn insert(&mut self, s: String) {
        let (mut undo_cmd, mut redo_cmd) = self.delete_func();
        let inserted_multiline = s.contains('\n');

        let org_c: Cursor = self.cursor2();
        let mut new_c = org_c;

        if let Some(pgh_view) = self.pgh_views.get_mut(org_c.line_no) {
            let (ls, rs, seg_text) = pgh_view.insert(&org_c, &s);
            if pgh_view.is_table_row() {
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
        if self.cfg().is_markdown {
            if inserted_multiline {
                let line_start = org_c.line_no.min(new_c.line_no);
                let line_end = org_c.line_no.max(new_c.line_no);
                self.change_to_table_in_line_range(line_start, line_end);
            } else if new_c.line_no < self.pgh_views.len()
                && !self.is_table_line(new_c.line_no)
                && self.get_line_text(new_c.line_no).starts_with("|")
            {
                self.change_to_table_by_anchor_line(new_c.line_no);
            }
        }
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
            let (insert_at, col) = self
                .table_cursor_logical_cell()
                .map(|(row, col)| (row + 1, col))
                .unwrap_or((0usize, 0usize));
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
            let insert_line_no = if self
                .get_line(c.line_no)
                .is_some_and(|p| p.is_table_row())
            {
                self.table_row_block_range(c.line_no)
                    .map(|(_, blk_end)| blk_end + 1)
                    .unwrap_or(c.line_no + 1)
            } else if self
                .get_line(c.line_no)
                .is_some_and(|p| p.is_code_row())
            {
                self.code_row_block_range(c.line_no)
                    .map(|(_, blk_end)| blk_end + 1)
                    .unwrap_or(c.line_no + 1)
            } else {
                c.line_no + 1
            };
            undo_cmd.push_delete(insert_line_no);
            undo_cmd.set_cursor(c);
            self.insert_line(insert_line_no, "".to_string());
            redo_cmd.push_insert(insert_line_no, self.get_line_clone(insert_line_no));

            self.state.cursor2 = 0.into();
            self.state.cursor2.line_no = insert_line_no;
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

    /// 文档首行空代码块行首退格：整块还原为一行空文本（与 `replace_lines_with_code_block` 互逆）。
    pub fn backspace_remove_empty_code_block_at_document_start(&mut self) -> bool {
        use crate::medit::{DoCmd, PghType, PghView};

        let c = self.cursor_check(&self.cursor2());
        if c.line_no != 0 || !self.is_line_type(0, PghType::CodeRow) {
            return false;
        }
        let Some((blk_s, blk_e)) = self.code_row_block_range(0) else {
            return false;
        };
        let block_start: Cursor = (blk_s, 0, 0).into();
        if c != self.cursor_check(&block_start) {
            return false;
        }
        // 仅单行空代码块可整块删除；多行空行块保留（退格走常规逻辑）。
        if blk_s != blk_e {
            return false;
        }
        if !self.get_line_text(blk_s).is_empty() {
            return false;
        }

        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        undo_cmd.set_cursor(self.cursor2());
        redo_cmd.set_cursor(self.cursor1());

        for ln in (blk_s..=blk_e).rev() {
            undo_cmd.push_insert(ln, self.get_line_clone(ln));
            self.pgh_views.remove(ln);
            redo_cmd.push_delete(ln);
        }

        let mut empty_line = PghView::new_text();
        empty_line.push_text(String::new(), None);
        undo_cmd.push_delete(blk_s);
        self.pgh_views.insert(blk_s, empty_line);
        redo_cmd.push_insert(blk_s, self.get_line_clone(blk_s));

        let new_cursor = (blk_s, 0, 0).into();
        self.set_cursor2(new_cursor);
        self.set_cursor1_reset();
        redo_cmd.set_cursor(self.cursor2());

        self.push_do(undo_cmd, redo_cmd);
        self.on_content_change();
        true
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
        // 底部留出与 egui ScrollArea 水平滚动条同高的条带，避免命中测试把滚动条当作正文区。
        self.area.edit_rect.set_bottom((max_rect.bottom() - scroll_width).max(self.area.edit_rect.top()));
        self.area.line_no_rect.set_bottom(self.area.edit_rect.bottom());
        self.area.divider_rect.set_bottom(self.area.edit_rect.bottom());
    }

    pub fn line_num(&self) -> usize {
        self.pgh_views.len()
    }

    pub fn top_line(&self) -> usize {
        self.state.top_line
    }

    pub fn set_scroll_to_line_mode(&mut self, mode: ScrollToLineMode) {
        self.state.scroll_to_line = Some(mode);
    }

    pub fn set_scroll_to_line(&mut self, line: usize) {
        self.set_scroll_to_line_mode(ScrollToLineMode::Center(line));
    }

    pub fn clean_scroll_to_line(&mut self) -> Option<ScrollToLineMode> {
        self.state.scroll_to_line.take()
    }

    /// 使第 `line` 行底端贴近编辑区底端的滚动偏移（用于 [`ScrollToLineMode::Bottom`]）。
    pub(crate) fn scroll_offset_y_align_line_bottom(&self, line: usize) -> f32 {
        let n = self.line_num();
        if n == 0 {
            return 0.0;
        }
        let line = line.min(n.saturating_sub(1));
        let end_idx = (line + 1).min(n);
        let line_bottom = self.scroll_cum_at(end_idx);
        let view_h = self.edit_rect().height();
        let total = self.scroll_cum_at(n);
        let max_off = (total - view_h).max(0.0);
        (line_bottom - view_h).clamp(0.0, max_off)
    }

    /// 将第 `line` 行的垂直几何中心对齐编辑区中央的滚动偏移（用于 [`ScrollToLineMode::Center`]）。
    pub(crate) fn scroll_offset_y_center_line(&self, line: usize) -> f32 {
        let n = self.line_num();
        if n == 0 {
            return 0.0;
        }
        let line = line.min(n.saturating_sub(1));
        let y0 = self.scroll_cum_at(line);
        let y1 = self.scroll_cum_at((line + 1).min(n));
        let line_center_y = (y0 + y1) * 0.5;
        let view_h = self.edit_rect().height();
        let total = self.scroll_cum_at(n);
        let max_off = (total - view_h).max(0.0);
        (line_center_y - view_h * 0.5).clamp(0.0, max_off)
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

    pub fn set_request_focus(&mut self) {
        self.request_focus = true;
    }

    pub fn take_request_focus(&mut self) -> bool {
        let pending = self.request_focus;
        self.request_focus = false;
        pending
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

    pub fn defer_editor_action(&mut self, cmd: Action) {
        self.deferred_editor_actions.push(cmd);
    }

    pub fn take_deferred_editor_actions(&mut self) -> Vec<Action> {
        std::mem::take(&mut self.deferred_editor_actions)
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

    fn execute_url(&mut self, line_no: usize, url_info: UrlInfo, is_clicked: bool, _is_line_changed: bool) {
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
        self.pgh_views
            .get_mut(line_no)
            .map(PghView::line_flash_reset)
            .unwrap_or(false)
    }

    pub fn line_flash_tick(&mut self, line_no: usize) {
        if let Some(pghview) = self.pgh_views.get_mut(line_no) {
            pghview.line_flash_tick();
        }
    }

    pub fn line_flash_all(&mut self) {
        for x in &mut self.pgh_views {
            x.line_flash_tick();
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
                    DoItem::ReplaceAll(lines) => {
                        if lines.is_empty() {
                            None
                        } else {
                            Some(0)
                        }
                    }
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
            DoItem::ReplaceAll(pgh_views) => {
                self.pgh_views = pgh_views.clone();
                if self.pgh_views.is_empty() {
                    let mut line = PghView::new_text();
                    line.push_text(String::new(), None);
                    self.pgh_views.push(line);
                }
                self.line_change_tick(0);
                log::debug!("ReplaceAll lines={}", self.pgh_views.len());
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

    pub(crate) fn line_matches_find(text: &str, param: &FindReplaceCtx) -> bool {
        !Self::find_func(text, param).is_empty()
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
                let text = pgh.get_search_text();
                for found in Self::find_func(&text, param) {
                    let start_cursor = pgh.text_byte_index_to_cursor(found.start, line, false);
                    let end_cursor = pgh.text_byte_index_to_cursor(found.end, line, false);
                    if end_cursor > cursor {
                        return Some((start_cursor, end_cursor));
                    }
                }
            }
        }

        for line in 0..=cursor.line_no {
            if let Some(pgh) = self.pgh_views.get(line) {
                let text = pgh.get_search_text();
                for found in Self::find_func(&text, param) {
                    let start_cursor = pgh.text_byte_index_to_cursor(found.start, line, false);
                    let end_cursor = pgh.text_byte_index_to_cursor(found.end, line, false);
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
                let text = pgh.get_search_text();
                for found in Self::find_func(&text, param) {
                    let start = pgh.text_byte_index_to_cursor(found.start, line_no, false);
                    let end = pgh.text_byte_index_to_cursor(found.end, line_no, false);
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

    pub fn set_find_live_filter(&mut self, mut param: FindReplaceCtx, enabled: bool) {
        param.regex_build();
        self.set_scroll_to_line_mode(ScrollToLineMode::Top(0));
        self.index_cache_mgr
            .find_filter_cache_mut()
            .set_filter(param, enabled);
        let n = self.line_num();
        self.request_rebuild_index_kinds(
            0,
            n,
            RebuildReason::FindFilterChanged,
            RebuildMode::Deferred,
            &[IndexCacheKind::FindFilter, IndexCacheKind::ScrollLayout],
        );
    }

    pub fn clear_find_live_filter(&mut self) {
        self.set_find_live_filter(FindReplaceCtx::new(), false);
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
