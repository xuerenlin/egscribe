use core::f32;
use std::cell::RefCell;
use dyn_clone::DynClone;
use eframe::egui::epaint::text::LayoutJob;
use eframe::egui::{
    Label, NumExt, Pos2, Rect, Response, RichText, Sense, Ui, Widget,
    vec2, CursorIcon,
};
use crate::medit::{Cursor, Ctx, MarkDownImpl, LinkInfo, Edit};
use super::image::{ImageInfo};
use crate::uicom::IconName;
mod pgh_text;
pub use pgh_text::{PghText, TextSpacing};
mod pgh_items;
mod pgh_code;
mod pgh_table;
pub use pgh_items::{
    PghBreak, PghCheckBox, PghIcon, PghImage, PghIndent, PghOutlineFold, PghPoint, PghQuoteIndent,
    QUOTE_INDENT_WIDTH,
};
#[allow(dead_code)]
pub type CodeLangMenu<'a> = pgh_code::CodeLangMenu<'a>;
pub use pgh_table::{TableFrameStyle, TableInfo};
pub type TableKey = u64;
pub type CodeKey = u64;

/// fenced 代码块内一行（`PghType::CodeRow`），与 `TableInfo` 行下标语义对齐
#[derive(Clone, Debug, PartialEq)]
pub struct CodeInfo {
    pub code_row_index: usize,
    pub code_total_rows: usize,
}


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
    Indent,
    ListItemIndent,
    CheckBox,
    Point,
    QuoteIndent,
    Break,
    Icon,
    Image,
    OutlineFold,
}

pub trait PghItem: DynClone {
    fn text(&self) -> String {
        "".to_string()
    }

    fn layout_job(&self) -> Option<LayoutJob> {
        None
    }

    fn layout_job_update(&mut self, _job: Option<LayoutJob>) {}

    fn update_view_info(&mut self, _char_rect: Vec<CharRect>) {}

    fn cursor_from_pos(&self, _line_no: usize, _segment: usize, _pos: &Pos2) -> Option<Cursor> {
        None
    }

    fn pos_from_cursor(&self, _cursor: &Cursor) -> Option<Rect> {
        None
    }

    fn delete(&self, _line_no: usize, _segment: usize, _c1: &Cursor, _c2: &Cursor) -> Option<String> {
        Some("".to_string())
    }

    fn select(&self, _line_no: usize, _segment: usize, _c1: &Cursor, _c2: &Cursor, _keep_pos: bool) -> Option<String> {
        Some("".to_string())
    }

    fn insert(&self, _c: &Cursor) -> (String, String) {
        ("".to_string(), "".to_string())
    }

    fn enter(&self, _c: &Cursor) -> (String, String) {
        ("".to_string(), "".to_string())
    }

    fn update_text(&mut self, _text: String) {}

    fn max_culumn(&self) -> usize {
        0
    }

    fn icon_name(&self) -> Option<IconName> {
        None
    }

    fn image_info(&self) -> Option<ImageInfo> {
        None
    }

    fn link_info(&self) -> Option<LinkInfo> {
        None
    }

    fn outline_fold_collapsed(&self) -> Option<bool> {
        None
    }

    fn set_outline_fold_collapsed(&mut self, _collapsed: bool) {}
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

/// 布局函数的返回类型，包含 Response 和事件处理标志
#[derive(Clone, Debug)]
pub struct LayoutResponse {
    pub response: Response,
    /// 独立控件（OutlineFold / CheckBox / Break 等）的响应，不并入 `response` 以免干扰正文光标；
    /// 与 `response` 合并后用于申请编辑器焦点。
    pub focus_response: Response,
    /// 标志子控件是否已经响应事件，上层不需要再响应事件
    pub handled: bool,
}

impl LayoutResponse {
    pub fn new(response: Response, focus_response: Response, handled: bool) -> Self {
        LayoutResponse {
            response,
            focus_response,
            handled,
        }
    }

    /// 从 Response 创建，默认 handled 为 false
    pub fn from_response(response: Response) -> Self {
        LayoutResponse {
            response: response.clone(),
            focus_response: response,
            handled: false,
        }
    }

    /// 用于申请焦点的合并响应（正文区 + 独立控件 + 行号等）
    pub fn focus_target(&self) -> Response {
        self.response.clone() | self.focus_response.clone()
    }
}


#[derive(Clone, PartialEq, Debug)]
pub enum PghType {
    Text,
    Heading,
    BreakLine,
    BlockLine,
    ListItem,
    /// 表格的一行，对应编辑器中的一行 `PghView`，`pgh` 仅含 `col_count` 个单元格 segment
    TableRow,
    /// 代码块的一行，对应编辑器中的一行 `PghView`，通常仅含一个文本 segment
    CodeRow,
    UnKnown,
}

#[derive(Clone, Debug)]
pub struct PghView {
    pub pgh_type: PghType,
    pub pgh: Vec<PghSegment>,
    pub rect: Option<Rect>,
    pub spacing_top: f32,
    pub spacing_bottom: f32,
    pub table_key: Option<TableKey>,
    pub code_key: Option<CodeKey>,
    pub code_lang: Option<String>,
    pub change_tick: usize,
    pub refresh_tick: usize,
    pub expanded_text_id: Option<u64>,
    /// 行号列 checkbox（当前行）勾选状态，仅用于 `TableRow`
    pub row_index_checked: bool,
    /// 表格行高缓存（仅用于可见区虚拟化占位，不参与业务状态）
    pub(crate) table_row_heights: RefCell<Vec<f32>>,
    /// 上一帧该行布局高度，用于 `ScrollArea::show_viewport` 前缀和；0 表示尚未测量，用 `font_heigh` 估计
    pub(crate) last_scroll_height: f32,
    /// 被各级标题折叠隐藏（下标 0 = 被 1 级标题折叠，依此类推）
    pub hidden_by_level: [bool; 6],
    /// 被实时搜索过滤隐藏
    pub hidden_by_find_filter: bool,
    /// 标题级别缓存（1..=6），仅 `PghType::Heading`
    pub heading_level: Option<u8>,
}

impl PghView {
    pub fn new(pgh_type: PghType) -> Self {
        Self {
            pgh_type,
            pgh: vec![],
            rect: None,
            spacing_top: 0.0,
            spacing_bottom: 0.0,
            table_key: None,
            code_key: None,
            code_lang: None,
            change_tick: 1,
            refresh_tick: 0,
            expanded_text_id: None,
            row_index_checked: false,
            table_row_heights: RefCell::new(Vec::new()),
            last_scroll_height: 0.0,
            hidden_by_level: [false; 6],
            hidden_by_find_filter: false,
            heading_level: None,
        }
    }

    pub fn is_outline_hidden(&self) -> bool {
        self.hidden_by_level.iter().any(|&h| h)
    }

    pub fn is_render_hidden(&self) -> bool {
        self.is_outline_hidden() || self.hidden_by_find_filter
    }

    pub fn line_flash_reset(&mut self) -> bool {
        let changed = self.refresh_tick > 0;
        self.refresh_tick = 0;
        changed
    }

    pub fn line_flash_tick(&mut self) {
        self.refresh_tick += 1;
    }

    pub fn set_find_filter_hidden(&mut self, hidden: bool) {
        if self.hidden_by_find_filter != hidden {
            if hidden {
                self.rect = None;
                self.last_scroll_height = 0.0;
            }
            self.line_flash_tick();
            self.hidden_by_find_filter = hidden;
        }
        
    }

    pub fn set_hidden_at_level(&mut self, level: u8, hidden: bool) {
        if !(1..=6).contains(&level) {
            return;
        }
        let idx = (level - 1) as usize;
        if self.hidden_by_level[idx] != hidden {
            if hidden {
                self.rect = None;
                self.last_scroll_height = 0.0;
            }
            self.line_flash_tick();
            self.hidden_by_level[idx] = hidden;
        }
    }

    pub fn outline_fold_segment_index(&self) -> Option<usize> {
        self.pgh
            .iter()
            .position(|s| s.seg_type == SegmentType::OutlineFold)
    }

    pub fn outline_fold_collapsed(&self) -> Option<bool> {
        self.pgh
            .iter()
            .find(|s| s.seg_type == SegmentType::OutlineFold)
            .and_then(|s| s.item.outline_fold_collapsed())
    }

    pub fn set_outline_fold_collapsed_on_segment(&mut self, collapsed: bool) {
        if let Some(i) = self.outline_fold_segment_index() {
            if let Some(seg) = self.pgh.get_mut(i) {
                seg.item.set_outline_fold_collapsed(collapsed);
            }
        }
    }

    pub fn ensure_outline_fold_segment(&mut self, collapsed: bool) {
        if self.pgh_type != PghType::Heading {
            return;
        }
        if let Some(i) = self.outline_fold_segment_index() {
            if i + 1 != self.pgh.len() {
                let seg = self.pgh.remove(i);
                self.pgh.push(seg);
            }
        } else {
            self.pgh.push(PghSegment::new(
                SegmentType::OutlineFold,
                Box::new(PghOutlineFold::new(collapsed)),
            ));
        }
        self.set_outline_fold_collapsed_on_segment(collapsed);
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

    pub fn new_code_row() -> Self {
        PghView::new(PghType::CodeRow)
    }

    pub fn new_block_line() -> Self {
        PghView::new(PghType::BlockLine)
    }

    /// 将 `PghType::Text` 中含字面 `\n` 的内容拆成多行 `PghView`（每行仍带行首连续的缩进/列表等前缀段）。
    /// 其它 `PghType` 不处理，原样返回单元素向量。
    ///
    /// 若某 `Text` 段内出现换行且该段带有 `LayoutJob`，拆出的各文本段一律使用 `None`（暂不支持跨行富文本）。
    pub fn split_text_by_embedded_newlines(self) -> Vec<PghView> {
        fn segment_is_line_prefix(seg: &PghSegment) -> bool {
            matches!(
                seg.seg_type,
                SegmentType::Indent
                    | SegmentType::ListItemIndent
                    | SegmentType::CheckBox
                    | SegmentType::Point
                    | SegmentType::QuoteIndent
            )
        }

        if self.pgh_type != PghType::Text {
            return vec![self];
        }
        let has_embedded_nl = self.pgh.iter().any(|s| {
            s.seg_type == SegmentType::Text && s.item.text().contains('\n')
        });
        if !has_embedded_nl {
            return vec![self];
        }

        let spacing_top_first = self.spacing_top;
        let spacing_bottom_last = self.spacing_bottom;
        let change_tick = self.change_tick;
        let refresh_tick = self.refresh_tick;
        let expanded_text_id = self.expanded_text_id;
        let table_key = self.table_key;
        let code_key = self.code_key;
        let code_lang = self.code_lang.clone();

        let pgh = self.pgh;
        let prefix_len = pgh
            .iter()
            .take_while(|s| segment_is_line_prefix(s))
            .count();
        let prefix: Vec<PghSegment> = pgh[..prefix_len].to_vec();
        let body = &pgh[prefix_len..];

        let mut out_lines: Vec<PghView> = vec![];
        let mut current_suffix: Vec<PghSegment> = vec![];

        let mut flush_line = |suffix: Vec<PghSegment>| {
            let mut line = PghView::new(PghType::Text);
            line.rect = None;
            line.spacing_top = spacing_top_first;
            line.spacing_bottom = spacing_bottom_last;
            line.change_tick = change_tick;
            line.refresh_tick = refresh_tick;
            line.expanded_text_id = expanded_text_id;
            line.table_key = table_key;
            line.code_key = code_key;
            line.code_lang = code_lang.clone();
            line.pgh = prefix.clone();
            line.pgh.extend(suffix);
            out_lines.push(line);
        };

        for seg in body {
            if seg.seg_type != SegmentType::Text || !seg.item.text().contains('\n') {
                current_suffix.push(seg.clone());
                continue;
            }
            let full_text = seg.item.text();
            let parts: Vec<&str> = full_text.split('\n').collect();
            for (i, part) in parts.iter().enumerate() {
                if i > 0 {
                    flush_line(std::mem::take(&mut current_suffix));
                }
                current_suffix.push(PghSegment::new(
                    SegmentType::Text,
                    Box::new(PghText::new((*part).to_string(), None)),
                ));
            }
        }

        flush_line(std::mem::take(&mut current_suffix));

        let n = out_lines.len();
        for (i, line) in out_lines.iter_mut().enumerate() {
            if i > 0 {
                line.spacing_top = 0.0;
            }
            if i + 1 < n {
                line.spacing_bottom = 0.0;
            }
        }

        out_lines
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

    pub fn insert_text_before_next_text(&mut self, i: usize, s: String, job: Option<LayoutJob>) {
        let insert_pos = self.pgh
            .iter()
            .enumerate()
            .skip(i + 1)
            .find(|(_, segment)| segment.seg_type == SegmentType::Text)
            .map(|(pos, _)| pos);
        
        if let Some(pos) = insert_pos {
            self.pgh.insert(
                pos,
                PghSegment::new(SegmentType::Text, Box::new(PghText::new(s, job))),
            );
        } else {
            self.pgh.push(PghSegment::new(
                SegmentType::Text,
                Box::new(PghText::new(s, job)),
            ));
        }
    }

    pub fn update_text(&mut self, i: usize, s: String, job: Option<LayoutJob>) {
        if let Some(seg) = self.pgh.get_mut(i) {
            seg.item = Box::new(PghText::new(s, job));
        }
    }

    pub fn push_indent(&mut self) {
        self.pgh
            .push(PghSegment::new(SegmentType::Indent, Box::new(PghIndent::new())));
    }

    pub fn push_list_item_indent(&mut self) {
        self.pgh
            .push(PghSegment::new(SegmentType::ListItemIndent, Box::new(PghIndent::new())));
    }

    pub fn insert_list_item_indent(&mut self, i: usize) {
        self.pgh.insert(i, PghSegment::new(SegmentType::ListItemIndent, Box::new(PghIndent::new())));
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

    pub fn push_icon(&mut self, icon_name: IconName) {
        self.pgh
            .push(PghSegment::new(SegmentType::Icon, Box::new(PghIcon::new(icon_name))));
    }

    pub fn push_icon_with_link(&mut self, icon_name: IconName, link_info: LinkInfo) {
        self.pgh
            .push(PghSegment::new(SegmentType::Icon, Box::new(PghIcon::new_with_link(icon_name, link_info))));
    }

    pub fn push_image(&mut self, image_info: ImageInfo) {
        self.pgh
            .push(PghSegment::new(SegmentType::Image, Box::new(PghImage::new(image_info))));
    }

    pub fn push_outline_fold(&mut self, collapsed: bool) {
        self.ensure_outline_fold_segment(collapsed);
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

    pub fn is_pos_left(&self, pos: &Pos2) -> bool {
        if let Some(rect) = self.rect {
            if pos.x < rect.left()
                && pos.y >= rect.left_top().y
                && pos.y <= rect.right_bottom().y
            {
                return true;
            }
        }
        false
    }

    pub fn is_pos_right(&self, pos: &Pos2) -> bool {
        if let Some(rect) = self.rect {
            if pos.x >= rect.right()
                && pos.y >= rect.left_top().y
                && pos.y <= rect.right_bottom().y
            {
                return true;
            }
        }
        false
    }

    pub fn first_same_y_text_segment(&self, pos: &Pos2) -> usize {
        for (i, segment) in self.pgh.iter().enumerate() {
            if let Some(rect) = segment.rect {
                if rect.left_top().y <= pos.y && rect.right_bottom().y >= pos.y 
                    && segment.seg_type == SegmentType::Text {
                    return i
                }
            }
        }
        0
    }

    pub fn last_same_y_segment(&self, pos: &Pos2) -> usize {
        let mut last = self.max_segment();
        for (i, segment) in self.pgh.iter().enumerate() {
            if let Some(rect) = segment.rect {
                if rect.left_top().y <= pos.y && rect.right_bottom().y >= pos.y {
                    last = i
                }
            }
        }
        last
    }

    pub fn rect(&self) -> Option<Rect> {
        self.rect
    }

    /// 用 `anchor_rect` 与各行内已有片段的 [`PghSegment::rect`] 求并集，写回本行 [`Self::rect`]。
    pub fn merge_pgh_rect_from_segments(&mut self, anchor_rect: Rect) {
        if self.rect.is_some() {
            let mut new_rect = anchor_rect;
            for sub_segment in &self.pgh {
                if let Some(seg_rect) = sub_segment.rect {
                    new_rect = new_rect.union(seg_rect);
                }
            }
            self.rect = Some(new_rect);
        } else {
            self.rect = Some(anchor_rect);
        }
    }

    pub fn update_view_info(&mut self, segment: usize, rect: Rect, char_rect: Vec<CharRect>) {
        if let Some(pgh_segment) = self.pgh.get_mut(segment) {
            //segment rect info:
            pgh_segment.rect = Some(rect);
            pgh_segment.item.update_view_info(char_rect.clone());

            self.merge_pgh_rect_from_segments(rect);
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

    pub fn end_cursor_of_line(&self, line_no: usize) -> Cursor {
        // 行尾应对齐「正文」末尾：最后一格若是 Icon 等非 Text，`max_segment` 会落在该段上，insert 会得到空串。
        let seg = self.last_text_segment();
        let max_culumn = self.max_culumn(&(line_no, seg, 0).into());
        (line_no, seg, max_culumn).into()
    }

    pub fn start_cursor_of_line(&self, line_no: usize) -> Cursor {
        (line_no, 0, 0).into()
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
            if c1.line_no == c2.line_no && self.is_table_like() {
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

        texts
    }

    fn extrema_culumn_on_cell(c1: &Cursor, c2: &Cursor, line: usize, seg: usize) -> (Option<usize>, Option<usize>) {
        let mut vs: Vec<usize> = Vec::new();
        if c1.line_no == line && c1.segment == seg {
            vs.push(c1.culumn);
        }
        if c2.line_no == line && c2.segment == seg {
            vs.push(c2.culumn);
        }
        (vs.iter().min().copied(), vs.iter().max().copied())
    }

    /// 列矩形与单元格相交时的字符范围，语义与 `PghText::get_delete` 一致（`en` 为较大一端光标，含于半开区间判定）。
    pub(crate) fn table_row_column_block_cell_span(
        line_no: usize,
        seg: usize,
        line_lo: usize,
        line_hi: usize,
        col_lo: usize,
        col_hi: usize,
        c1: &Cursor,
        c2: &Cursor,
        text_len: usize,
    ) -> Option<(usize, usize)> {
        if line_no < line_lo || line_no > line_hi || seg < col_lo || seg > col_hi {
            return None;
        }
        let row_single = line_lo == line_hi;
        let col_single = col_lo == col_hi;

        if row_single && col_single {
            let (mn, mx) = Self::extrema_culumn_on_cell(c1, c2, line_no, seg);
            let a = mn.unwrap_or(0);
            let b = mx.unwrap_or(text_len);
            let (a, b) = if a <= b { (a, b) } else { (b, a) };
            return Some((a, b));
        }

        // 单列跨多行：左右边界为同一列，首行只截左、末行只截右，中间整格；不能用「顶行取 max_on」
        // 否则另一端光标不在本格时会得到 en=0，复制成 `||`。
        if col_single && !row_single {
            if seg != col_lo {
                return None;
            }
            let (mut st, mut en) = if line_no == line_lo {
                let st = Self::extrema_culumn_on_cell(c1, c2, line_no, seg)
                    .0
                    .unwrap_or(0);
                (st, text_len)
            } else if line_no == line_hi {
                let en = Self::extrema_culumn_on_cell(c1, c2, line_no, seg)
                    .1
                    .unwrap_or(text_len);
                (0, en)
            } else {
                (0, text_len)
            };
            if st > en {
                std::mem::swap(&mut st, &mut en);
            }
            return Some((st, en));
        }

        let st = if seg == col_lo {
            if line_no == line_lo {
                Self::extrema_culumn_on_cell(c1, c2, line_no, seg)
                    .0
                    .unwrap_or(0)
            } else if line_no == line_hi && line_hi > line_lo {
                Self::extrema_culumn_on_cell(c1, c2, line_no, seg)
                    .0
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        };

        let en = if seg == col_hi {
            if line_no == line_hi {
                Self::extrema_culumn_on_cell(c1, c2, line_no, seg)
                    .1
                    .unwrap_or(text_len)
            } else if line_no == line_lo && line_hi > line_lo {
                Self::extrema_culumn_on_cell(c1, c2, line_no, seg)
                    .1
                    .unwrap_or(text_len)
            } else {
                text_len
            }
        } else {
            text_len
        };

        let (mut st, mut en) = (st, en);
        if st > en {
            std::mem::swap(&mut st, &mut en);
        }
        Some((st, en))
    }

    /// 列矩形内某一单元格的选中文本（不含 `|`）
    pub fn cell_text_table_row_column_block(
        &self,
        line_no: usize,
        seg: usize,
        c1: &Cursor,
        c2: &Cursor,
        line_lo: usize,
        line_hi: usize,
        col_lo: usize,
        col_hi: usize,
    ) -> String {
        let text_len = self
            .pgh
            .get(seg)
            .map(|s| s.item.text().chars().count())
            .unwrap_or(0);
        let Some((st, en)) = Self::table_row_column_block_cell_span(
            line_no, seg, line_lo, line_hi, col_lo, col_hi, c1, c2, text_len,
        ) else {
            return String::new();
        };
        let ec1: Cursor = (line_no, seg, st).into();
        let ec2: Cursor = (line_no, seg, en).into();
        let lo = std::cmp::min(ec1, ec2);
        let hi = std::cmp::max(ec1, ec2);
        self.pgh
            .get(seg)
            .and_then(|s| s.item.select(line_no, seg, &lo, &hi, false))
            .unwrap_or_default()
    }

    /// 同一块 `TableRow` 跨物理行的列矩形：删除后各 segment 新文本
    pub fn delete_table_row_column_block(
        &self,
        line_no: usize,
        c1: &Cursor,
        c2: &Cursor,
        line_lo: usize,
        line_hi: usize,
        col_lo: usize,
        col_hi: usize,
    ) -> Vec<String> {
        let mut texts = vec![];
        for (i, segment) in self.pgh.iter().enumerate() {
            if i < col_lo || i > col_hi {
                texts.push(segment.item.text());
                continue;
            }
            let orig = segment.item.text();
            let text_len = orig.chars().count();
            let Some((st, en)) = Self::table_row_column_block_cell_span(
                line_no, i, line_lo, line_hi, col_lo, col_hi, c1, c2, text_len,
            ) else {
                texts.push(orig);
                continue;
            };
            let ec1: Cursor = (line_no, i, st).into();
            let ec2: Cursor = (line_no, i, en).into();
            let lo = std::cmp::min(ec1, ec2);
            let hi = std::cmp::max(ec1, ec2);
            if let Some(ns) = segment.item.delete(line_no, i, &lo, &hi) {
                texts.push(ns);
            } else {
                texts.push(orig);
            }
        }
        texts
    }

    /// 列矩形内当前物理行的一行管道表单元格（`|a|b|`，不含分隔行）
    pub fn table_row_column_block_pipe_data_row(
        &self,
        line_no: usize,
        c1: &Cursor,
        c2: &Cursor,
        line_lo: usize,
        line_hi: usize,
        col_lo: usize,
        col_hi: usize,
    ) -> String {
        if !self.is_table_row() {
            return String::new();
        }
        let mut parts: Vec<String> = Vec::new();
        for col in col_lo..=col_hi {
            parts.push(self.cell_text_table_row_column_block(
                line_no, col, c1, c2, line_lo, line_hi, col_lo, col_hi,
            ));
        }
        format!("|{}|", parts.join("|"))
    }

    /// 同一块 `TableRow` 跨物理行的列矩形：`is_raw` 为拼接单元格原文；`false` 时仅一行 `|...|`（完整 GFM 小表由 `Ctx` 组装）
    pub fn select_table_row_column_block(
        &self,
        ctx: &Ctx,
        line_no: usize,
        c1: &Cursor,
        c2: &Cursor,
        line_lo: usize,
        line_hi: usize,
        col_lo: usize,
        col_hi: usize,
        is_raw: bool,
    ) -> String {
        if !self.is_table_row() {
            return self.select(ctx, line_no, c1, c2, is_raw);
        }
        if is_raw {
            let mut out = String::new();
            for col in col_lo..=col_hi {
                out.push_str(&self.cell_text_table_row_column_block(
                    line_no, col, c1, c2, line_lo, line_hi, col_lo, col_hi,
                ));
            }
            return out;
        }
        self.table_row_column_block_pipe_data_row(line_no, c1, c2, line_lo, line_hi, col_lo, col_hi)
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
    pub fn select(&self, ctx: &Ctx, line_no: usize, c1: &Cursor, c2: &Cursor, is_raw: bool) -> String {
        if is_raw {
            return self.select_to_vec(line_no, c1, c2, false).join("");
        }
        
        if let Some(table_info) = ctx.table_info_of_line(line_no) {
            let arr = self.select_to_vec(line_no, c1, c2, true);
            let min = std::cmp::min(c1, c2);
            let max = std::cmp::max(c1, c2);

            if self.is_table_row() {
                let col_count = self.pgh.len().max(1);
                let segmax_row = col_count.saturating_sub(1);
                let range = if line_no < min.line_no || line_no > max.line_no {
                    (0, 0)
                } else if line_no == min.line_no && line_no == max.line_no {
                    let s0 = min.segment.min(max.segment);
                    let s1 = min.segment.max(max.segment);
                    (s0, s1)
                } else if line_no > min.line_no && line_no < max.line_no {
                    (0, segmax_row)
                } else if line_no == max.line_no {
                    (0, max.segment)
                } else if line_no == min.line_no {
                    (min.segment, segmax_row)
                } else {
                    (0, 0)
                };

                let mut joins = String::new();
                if let Some((c1c, c2c)) = self.table_range_to_cells(range.0, range.1) {
                    // `get_all_text` 等使用 (0..) 到 (usize::MAX,..) 伪光标：min.line_no 恒为 0，不能单靠 line_no==min 判断表头行
                    let whole_document_select = max.line_no == usize::MAX;
                    // 仅框选单个单元格：只要单元格内原文，不要 `|...|` / 分隔行（整篇导出仍走表格拼接）
                    if c1c.col == c2c.col && !whole_document_select {
                        return arr.get(c1c.col).cloned().unwrap_or_default();
                    }
                    joins.push('|');
                    for col in c1c.col..=c2c.col {
                        joins.push_str(arr.get(col).unwrap_or(&String::new()));
                        joins.push('|');
                    }
                    // 表头后补分隔行（整块导出 / 局部选表头）
                    let (blk_s, blk_e) = ctx
                        .table_row_block_range(line_no)
                        .unwrap_or((line_no, line_no));
                    let row_no = line_no.saturating_sub(blk_s);
                    let row_count = blk_e.saturating_sub(blk_s).saturating_add(1);
                    let append_after_header = row_no == 0
                        && row_count > 1
                        && (line_no == min.line_no || whole_document_select);
                    // 从表中间起选（单行或多行）：在选中块「首行」单元格之后补 `|---|`（单行时也在内容下面）
                    let append_after_mid_first_line = !whole_document_select
                        && row_no >= 1
                        && line_no == min.line_no;
                    if append_after_header || append_after_mid_first_line {
                        joins.push('\n');
                        joins.push('|');
                        for _col in c1c.col..=c2c.col {
                            joins.push_str("--|");
                        }
                        // 单行选中（表头一行或中间单行）时在分隔行后再换行
                        if max.line_no == min.line_no && !whole_document_select {
                            joins.push('\n');
                        }
                    }
                }
                return joins;
            }

            let row_count = self.pgh.len() / table_info.col_count.max(1);
            if row_count == 0 || table_info.col_count == 0 {
                return String::new();
            }
            let segmax = row_count * table_info.col_count - 1;

            let range = if line_no < min.line_no || line_no > max.line_no {
                (0, 0)
            } else if line_no == min.line_no && line_no == max.line_no {
                //same line
                (min.segment, max.segment)
            } else if line_no > min.line_no && line_no < max.line_no {
                //middle line
                (0, row_count * table_info.col_count - 1)
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
                        for _col in c1.col..=c2.col {
                            joins += "--|";
                        }
                        joins += "\n";
                    }
                }
            }
            joins
        } else if self.is_code_row() {
            let text = self.select_to_vec(line_no, c1, c2, false).join("\n");
            let min_cursor = std::cmp::min(c1, c2);
            let max_cursor = std::cmp::max(c1, c2);
            let line_end_cursor = self.end_cursor_of_line(line_no);
            let line_start_cursor = self.start_cursor_of_line(line_no);
            let is_full_line = min_cursor <= &line_start_cursor && max_cursor >= &line_end_cursor;
            // 多行 fenced 块在整篇导出时由 `Ctx::get_all_text` 合并输出围栏；此处仅单行块或局部选区
            let single_row_block = ctx
                .code_info_of_line(line_no)
                .map(|c| c.code_total_rows <= 1)
                .unwrap_or(true);
            if is_full_line && single_row_block {
                let info_line = ctx.markdown_export_code_fence_info_line(line_no);
                format!("```{}\n{}\n```", info_line, text)
            } else {
                text
            }
        } else if self.pgh_type == PghType::BreakLine {
            // 前导 `\n`：`get_all_text` 行首会再补一行间 `\n`，与标记内本 `\n` 合成为 `正文\n\n---`，满足 GFM 主题线；根级 gap 对 ThematicBreak 已减算，不会在打开时多还原空行。
            let s = self.get_text();
            let t = s.trim();
            let marker = if t.is_empty() {
                "---".to_string()
            } else {
                t.to_string()
            };
            format!("\n{marker}")
        } else {
            let mut out = self.select_to_vec(line_no, c1, c2, false).join("");
            // 插入水平线后、重解析前常为 `Text` 的 `---`，与 `BreakLine` 导出一致
            if self.pgh_type == PghType::Text {
                let min_cursor = std::cmp::min(c1, c2);
                let max_cursor = std::cmp::max(c1, c2);
                let line_start_cursor = self.start_cursor_of_line(line_no);
                let line_end_cursor = self.end_cursor_of_line(line_no);
                let whole_line =
                    min_cursor <= &line_start_cursor && max_cursor >= &line_end_cursor;
                let whole_doc = max_cursor.line_no == usize::MAX;
                if whole_line && whole_doc && matches!(out.trim(), "---" | "***" | "___") {
                    out.insert(0, '\n');
                }
            }
            out
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
        //TODO：存疑，table_row是不是不需要|?
        if self.is_table_row() {
            let mut parts: Vec<String> = vec![];
            for segment in &self.pgh {
                parts.push(segment.item.text());
            }
            return format!("|{}|", parts.join("|"));
        }
        let mut rs: String = Default::default();
        for segment in &self.pgh {
            let s = segment.item.text();
            rs += &s;
        }
        rs
    }

    /// 文本查找使用的线性文本。`TableRow` 不包含管道分隔符，以对齐 `text_*_index_to_cursor` 的 segment 拼接语义。
    pub fn get_search_text(&self) -> String {
        if self.is_table_row() {
            let mut rs: String = Default::default();
            for segment in &self.pgh {
                rs += &segment.item.text();
            }
            rs
        } else {
            self.get_text()
        }
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

    pub fn is_code_row(&self) -> bool {
        self.pgh_type == PghType::CodeRow
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
            log::debug!(
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

    pub fn text_byte_index_to_cursor(&self, index: usize, line_no: usize, greedy: bool) -> Cursor {
        let mut cursor: Cursor = line_no.into();
        let mut sum_index_byte = 0;
        let last_seg = self.last_text_segment();
        'outer: for segment in &self.pgh[..last_seg + 1] {
            for (i, c) in segment.item.text().chars().enumerate() {
                sum_index_byte += c.len_utf8();
                if !greedy && sum_index_byte == index {
                    cursor.culumn = i+1;
                    break 'outer;
                }
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

    pub fn get_text_warp_width_base_cursor(ui: &Ui, ctx: &Ctx, keep_space: f32) -> f32 {
        let pos = ui.cursor().left_top();
        let edit_right = ctx.edit_rect().right();
        if (ctx.cfg().wrap && pos.x <= edit_right) || keep_space > 1.0 {
            edit_right - pos.x - keep_space
        } else {
            f32::INFINITY
        }
    }

    pub fn get_text_warp_width_base_rect(ctx: &Ctx, rect_width: f32) -> f32 {
        if ctx.cfg().wrap {
            rect_width
        } else {
            f32::INFINITY
        }
    }

    pub fn parse_markdown_if_needed(ctx: &mut Ctx, line_no: usize) -> Option<bool> {
        if ctx.get_line(line_no).is_none() {
            return None;
        }
        if ctx
            .get_line(line_no)
            .map(|p| p.is_render_hidden())
            .unwrap_or(false)
        {
            return Some(false);
        }
        let is_line_content_changed = ctx.line_change_reset(line_no);
        let is_flash_changed = ctx.line_flash_reset(line_no);
        let is_changed = is_line_content_changed || is_flash_changed;

        let skip_markdown_parse = {
            let line = ctx.get_line(line_no)?;
            line.pgh_type == PghType::CodeRow || line.pgh_type == PghType::TableRow
        };
        if skip_markdown_parse {
            // `line_flash_all` 只 bump `refresh_tick`：须把 flash 并入返回值，否则 CodeRow
            // 不会重跑 `code_highlight_job`（主题色不随 dark/light 更新）。
            return Some(is_changed);
        }

        if !ctx.cfg().is_markdown
            || (ctx.cursor2().line_no != line_no && !is_changed && !ctx.is_selected_line(line_no))
        {
            return Some(is_line_content_changed);
        }

        let is_selected = ctx.is_selected();
        let cursor2 = ctx.cursor2();
        let cursor1 = ctx.cursor1();
        let (cursor1_char_index, cursor2_char_index, text) = {
            let pgh_view = ctx.get_line(line_no)?;
            let cursor2_char_index = if cursor2.line_no == line_no {
                Some(pgh_view.cursor_to_text_char_index(&cursor2))
            } else {
                None
            };
            let cursor1_char_index = if cursor1.line_no == line_no && is_selected {
                Some(pgh_view.cursor_to_text_char_index(&cursor1))
            } else {
                None
            };
            let text = pgh_view.get_text();
            (cursor1_char_index, cursor2_char_index, text)
        };

        let markdown = MarkDownImpl::new(
            &text,
            ctx.cfg().is_markdown,
            cursor2_char_index,
            ctx.is_selected_line(line_no) && !ctx.is_ime_preedit_selected(),
            ctx.cfg(),
        );
        let new_pghview = markdown.markdown_to_pghview();
        ctx.update_pgh(line_no, &new_pghview);

        let (new_cursor1_opt, new_cursor2_opt) = if let Some(updated_pgh_view) = ctx.get_line(line_no) {
            let new_cursor1_opt = cursor1_char_index.map(|text_index| {
                updated_pgh_view.text_char_index_to_cursor(text_index, cursor1.line_no)
            });
            let new_cursor2_opt = cursor2_char_index.map(|text_index| {
                updated_pgh_view.text_char_index_to_cursor(text_index, cursor2.line_no)
            });
            (new_cursor1_opt, new_cursor2_opt)
        } else {
            (None, None)
        };

        if let Some(new_cursor1) = new_cursor1_opt {
            if new_cursor1 != cursor1 {
                log::debug!("old cursor1:{:?}, new cursor1:{:?}", cursor1, new_cursor1);
                ctx.set_cursor1(new_cursor1);
            }
        }
        if let Some(new_cursor2) = new_cursor2_opt {
            if new_cursor2 != cursor2 {
                log::debug!("old cursor2:{:?}, new cursor2:{:?}", cursor2, new_cursor2);
                ctx.set_cursor2(new_cursor2);
                if !is_selected {
                    ctx.set_cursor1_reset();
                }
            }
        }

        Some(is_line_content_changed)
    }

    pub fn layout_sigle_line(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        is_line_changed: bool,
    ) -> LayoutResponse {
        let (spacing_top, spacing_bottom, max_segment, num_segments, is_heading) = {
            let Some(p) = ctx.get_line(line_no) else {
                return LayoutResponse::from_response(
                    ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1,
                );
            };
            (
                p.spacing_top,
                p.spacing_bottom,
                p.max_segment(),
                p.pgh.len(),
                p.pgh_type == PghType::Heading,
            )
        };

        ctx.update_spacing(line_no, spacing_top, spacing_bottom);

        //response with top space
        let mut top_rect = ui.cursor();
        top_rect.set_right(ctx.edit_right());
        top_rect.set_height(spacing_top);
        let mut response = ui.allocate_rect(top_rect, ctx.sense());
        let mut focus_response = ui.allocate_rect(
            Rect::from_min_max(top_rect.left_top(), top_rect.left_top()),
            ctx.sense(),
        );
        let mut handled = false;

        let mut images = vec![];
        let mut quote_indent_rect = None;
        let mut outline_fold_collapsed = false;
        ui.horizontal_wrapped(|ui| {
            let mut row_rect = ui.cursor();
            row_rect.set_right(ctx.edit_right());

            // 标题行：行首绘制多级序号（字号/字体与 Markdown 标题一致）
            if is_heading && ctx.cfg().show_heading_section_numbers {
                if let Some(entry) = ctx.toc_entry_for_line(line_no) {
                    let font = ctx.cfg().heading_font_id(entry.level);
                    focus_response |= ui.add(
                        Label::new(
                            RichText::new(format!("{} ", entry.section_number))
                                .font(font)
                                .color(ctx.cfg().weak_color()),
                        )
                        .sense(Sense::click()),
                    );
                }
            }

            for segment in 0..num_segments {
                let seg_type = ctx
                    .get_line(line_no)
                    .and_then(|p| p.pgh.get(segment))
                    .map(|s| s.seg_type.clone());
                let Some(seg_type) = seg_type else {
                    continue;
                };
                match seg_type {
                    SegmentType::OutlineFold => {
                        let (_fold_changed, collapsed, r) =
                            PghOutlineFold::layout_paragraph(ui, ctx, line_no, segment);
                        outline_fold_collapsed = collapsed;
                        focus_response |= r;
                    }
                    SegmentType::Text => {
                        let (text, job) = {
                            let p = ctx.get_line(line_no).unwrap();
                            let seg = &p.pgh[segment];
                            (seg.item.text(), seg.item.layout_job())
                        };
                        let need_expand = segment == max_segment;
                        let warp_width = Self::get_text_warp_width_base_rect(ctx, row_rect.width());
                        let spacing = TextSpacing::text_spacing_in_rect(row_rect, warp_width)
                            .with_spacing_top_bottom(spacing_top, spacing_bottom)
                            .with_need_expand(need_expand)
                            .with_once_allocate(false)
                            .with_first_row_indentation(ui);
                        let r = PghText::layout_paragraph(
                            ui,
                            ctx,
                            line_no,
                            segment,
                            spacing,
                            text,
                            &job,
                        );
                        response |= r;
                    }
                    SegmentType::Indent => {
                        response |= PghIndent::layout_paragraph(ui, ctx, line_no, segment, ctx.cfg().indent_size);
                    }
                    SegmentType::ListItemIndent => {
                        response |= PghIndent::layout_paragraph(ui, ctx, line_no, segment, ctx.cfg().indent_size_of_list);
                    }
                    SegmentType::CheckBox => {
                        let (check_box_changed, r) =
                            PghCheckBox::layout_paragraph(ui, ctx, line_no, segment);
                        focus_response |= r;
                        if check_box_changed {
                            handled = true;
                        }
                    }
                    SegmentType::Point => {
                        response |= PghPoint::layout_paragraph(ui, ctx, line_no, segment);
                    }
                    SegmentType::QuoteIndent => {
                        let quote_response =
                            PghQuoteIndent::layout_paragraph(ui, ctx, line_no, segment);
                        // 收集引用块的 rect 信息，用于后续统一绘制
                        quote_indent_rect = Some(quote_response.rect);
                        response |= quote_response;
                    }
                    SegmentType::Break => {
                        focus_response |=
                            PghBreak::layout_paragraph(ui, ctx, line_no, segment);
                    }
                    SegmentType::Icon => {
                        let (r, is_clicked) = PghIcon::layout_paragraph(ui, ctx, line_no, segment);
                        if is_clicked || is_line_changed {
                            let is_external = matches!(
                                ctx.get_line(line_no)
                                    .and_then(|p| p.pgh.get(segment))
                                    .and_then(|s| s.item.icon_name()),
                                Some(IconName::icon_external_link)
                            );
                            if is_external {
                                let link_info = ctx
                                    .get_line(line_no)
                                    .and_then(|p| p.pgh.get(segment))
                                    .and_then(|s| s.item.link_info());
                                if let Some(link_info) = link_info {
                                    ctx.insert_link_click_command(line_no, link_info, is_clicked, is_line_changed);
                                }
                            }
                        }
                        if is_clicked {
                            handled = true;
                        }
                        response |= r;
                    }
                    SegmentType::Image => {
                        images.push(segment);
                    }
                }
            }

            //right space
            let mut right_rect = ui.cursor();
            if right_rect.left() < ctx.edit_right() {
                right_rect.set_right(ctx.edit_right());
                right_rect.set_height(response.rect.height() - spacing_top);
                response |= ui.allocate_rect(right_rect, ctx.sense());
                //if outline_fold_collapsed {
                //    PghOutlineFold::paint_collapsed_section_rule(ui, right_rect);
                //}
            }
        });

        //bottom space
        let mut bottom_rect = ui.cursor();
        bottom_rect.set_right(ctx.edit_right());
        bottom_rect.set_height(spacing_bottom);
        response |= ui.allocate_rect(bottom_rect, ctx.sense());

        //在当前行绘制完成后，统一绘制引用块填充
        if let Some(quote_indent_rect) = quote_indent_rect {
            let line_rect = response.rect;
            let fill_rect = Rect::from_min_max(
                Pos2::new(quote_indent_rect.center().x - QUOTE_INDENT_WIDTH / 2.0, line_rect.top()),
                Pos2::new(quote_indent_rect.center().x + QUOTE_INDENT_WIDTH / 2.0, line_rect.bottom()),
            );
            ui.painter().rect_filled(fill_rect, 0.0, ui.visuals().weak_text_color());
        }
        //设置光标，必须在image之前，否则图片光标不准确
        let mut response = response.on_hover_cursor(CursorIcon::Text);
        //draw images
        Self::layout_images(ui, ctx, line_no, &images, &mut response);
        LayoutResponse::new(response, focus_response, handled)
    }

    fn layout_images(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        image_segments: &[usize],
        response: &mut Response,
    ) {
        if image_segments.is_empty() {
            return;
        }

        let max_width = (ctx.edit_right() - ui.cursor().min.x).at_least(10.0);
        let available_width = max_width / image_segments.len() as f32;
        ui.horizontal(|ui| {
            *response |= PghIndent::layout_paragraph(ui, ctx, line_no, 0, ctx.cfg().indent_size);
            for &segment in image_segments {
                *response |= PghImage::layout_paragraph(ui, ctx, line_no, segment, available_width);
            }
        });
    }

    fn layout_expanded_text(
        ui: &mut Ui,
        ctx: &mut Ctx,
        _line_no: usize,
        expanded_id: u64,
    ) -> Option<Response> {
        // 检查expanded_id是否在map中，只有存在的才显示
        if !ctx.expanded_ctx().has_id(expanded_id) {
            return None;
        }

        let indent_size = ctx.cfg().indent_size;
        let expanded_ctx = match ctx.expanded_ctx_mut().ctx_mut(expanded_id) {
            Some(expanded_ctx) => expanded_ctx,
            None => return None,
        };

        let r = ui.push_id(expanded_id, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(indent_size);
                let edit = Edit::new(expanded_ctx);
                edit.ui(ui);
            })
        });
        
        // 显示后，调整为真实高度
        if let Some(expanded_ctx) = ctx.expanded_ctx_mut().ctx_mut(expanded_id) {
            expanded_ctx.set_height_mode_to_actually();
        }   
        Some(r.response)
    }

    pub fn layout(ui: &mut Ui, ctx: &mut Ctx, line_no: usize, is_line_changed: bool) -> LayoutResponse {
        let pgh_type = ctx
            .get_line(line_no)
            .map(|p| p.pgh_type.clone())
            .unwrap_or(PghType::UnKnown);
        let expanded_text_id = ctx.get_line(line_no).and_then(|p| p.expanded_text_id);

        let mut layout_response = match pgh_type {
            PghType::TableRow => Self::layout_table_row_line(ui, ctx, line_no),
            PghType::CodeRow => {
                Self::layout_code_line(ui, ctx, line_no, is_line_changed)
            }
            _ => Self::layout_sigle_line(ui, ctx, line_no, is_line_changed),
        };
        if let Some(expanded_id) = expanded_text_id {
            if let Some(r) = Self::layout_expanded_text(ui, ctx, line_no, expanded_id) {
                layout_response.response |= r;
            }
        }
        layout_response
    }
}
