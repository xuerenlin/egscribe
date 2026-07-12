use super::*;
use crate::medit::{TableInfo, TableKey};

#[derive(Clone, Debug)]
pub(super) struct ParsedTable {
    col_count: usize,
    row_count: usize,
    cells: Vec<String>,
}

impl Ctx {
    pub(super) fn get_table_cursor_rect(&self, min: &Cursor, max: &Cursor) -> Option<Rect> {
        if min.line_no != max.line_no {
            return None;
        }
        if let Some(pgh_view) = self.pgh_views.get(min.line_no) {
            if pgh_view.is_table_row() {
                let mut rect = pgh_view.table_range_rect(min.segment, max.segment)?;
                if let Some(ti) = self.table_info_of_line(min.line_no) {
                    rect = rect.expand2(Vec2::new(ti.spacing_x / 2.0, 0.0));
                }
                return Some(rect);
            }
        }
        None
    }

    pub(super) fn table_markdown_separator_from_pipe_row(row: &str) -> Option<String> {
        if !row.starts_with('|') || !row.ends_with('|') {
            return None;
        }
        let col_count = row.matches('|').count().saturating_sub(1);
        if col_count == 0 {
            return None;
        }
        Some(format!(
            "|{}|",
            std::iter::repeat("--")
                .take(col_count)
                .collect::<Vec<_>>()
                .join("|")
        ))
    }

    pub fn is_table_line(&self, line_no: usize) -> bool {
        if let Some(pgh_view) = self.pgh_views.get(line_no) {
            return pgh_view.is_table_like();
        }
        false
    }

    /// 为当前文档中所有连续 `TableRow` 块补齐元数据（`table_key` / `table_info`）。
    /// 适用于解析后但尚未进入首帧布局前的场景（例如测试直接 `with_text` 后执行 action）。
    pub(super) fn ensure_table_row_blocks_metadata(&mut self) {
        let mut line_no = 0usize;
        while line_no < self.pgh_views.len() {
            if self
                .pgh_views
                .get(line_no)
                .is_some_and(|p| p.is_table_row())
            {
                self.refresh_table_row_block_metadata(line_no);
                if let Some((_, end)) = self.table_row_block_range(line_no) {
                    line_no = end.saturating_add(1);
                    continue;
                }
            }
            line_no += 1;
        }
    }

    /// 与 `line_no` 同属一块的连续 `TableRow` 的 `[start, end]` 行号（含端点）。
    /// 优先按 `table_key` 聚合；缺失 key 时回退到“连续 + 同列数”。
    pub fn table_row_block_range(&self, line_no: usize) -> Option<(usize, usize)> {
        let p = self.get_line(line_no)?;
        if !p.is_table_row() {
            return None;
        }
        let table_key = p.table_key;
        let col_count = self.table_info_of_line(line_no).map(|ti| ti.col_count).unwrap_or(p.pgh.len());
        if col_count == 0 {
            return None;
        }
        let mut start = line_no;
        while start > 0 {
            let prev = self.get_line(start - 1)?;
            if !prev.is_table_row() {
                break;
            }
            if let Some(key) = table_key {
                if prev.table_key != Some(key) {
                    break;
                }
            } else {
                if prev.pgh.len() != col_count {
                    break;
                }
            }
            start -= 1;
        }
        let mut end = line_no;
        while end + 1 < self.pgh_views.len() {
            let next = self.get_line(end + 1)?;
            if !next.is_table_row() {
                break;
            }
            if let Some(key) = table_key {
                if next.table_key != Some(key) {
                    break;
                }
            } else {
                if next.pgh.len() != col_count {
                    break;
                }
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
    pub(super) fn table_row_block_column_rect(&self, c1: &Cursor, c2: &Cursor) -> Option<(usize, usize, usize, usize)> {
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
        let ti_lo = self.table_info_of_line(line_lo)?;
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
            let Some(ti) = self.table_info_of_line(ln) else {
                return None;
            };
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
    pub(super) fn table_row_block_column_copy_markdown(
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
        let Some(table_info) = self.table_info_of_line(anchor_line_no).cloned() else {
            return vec![];
        };
        let total_rows = table_info.row_count.max(1);
        let col_count = table_info.col_count;
        let mut max_width = self.edit_width();
        max_width -= table_info.spacing_indent;
        max_width -= 64.0;
        if self.cfg().show_table_row_no {
            max_width -= PghView::table_index_col_width(total_rows, table_info.col_min_width);
            if self.cfg().show_table_head_checkbox {
                max_width -= PghView::table_head_checkbox_total_width(self);
            }
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
                let Some(ti) = self.table_info_of_line(ln) else {
                    continue;
                };
                let row_style = ln.saturating_sub(ti.head_line_no);
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
        let mut table_key = self
            .get_line(s)
            .and_then(|p| p.table_key)
            .unwrap_or(0);
        if table_key == 0 {
            table_key = self.alloc_table_key();
        }
        let mut base = self
            .index_cache_mgr
            .table_cache()
            .table_info_cloned_by_key(table_key)
            .unwrap_or_default();
        base.row_index = 0;
        base.row_count = n;
        base.col_count = self.get_line(s).map(|p| p.pgh.len()).unwrap_or(base.col_count);
        base.head_line_no = s;
        base.frame_style = self.cfg().table_frame_style.clone();
        base.ensure_head_col_checked_len();
        self.index_cache_mgr.table_cache_mut().upsert_table_info(table_key, base.clone());
        for (i, ln) in (s..=e).enumerate() {
            if let Some(p) = self.get_line_mut(ln) {
                if !p.is_table_row() {
                    continue;
                }
                let mut ti = base.clone();
                ti.row_index = i;
                p.table_key = Some(table_key);
            }
        }
        //这里打印head_line_no信息
        log::info!("refresh_table_row_block_metadata table_key: {}, head_line_no: {}", table_key, base.head_line_no);
    }

    pub fn table_row_block_set_head_col_checked(
        &mut self,
        anchor_line_no: usize,
        col: usize,
        checked: bool,
    ) {
        let Some((s, _e)) = self.table_row_block_range(anchor_line_no) else {
            return;
        };
        let Some(table_key) = self.get_line(s).and_then(|p| p.table_key) else {
            return;
        };
        let Some(mut base) = self.index_cache_mgr.table_cache_mut().table_info_cloned_by_key(table_key) else {
            return;
        };
        base.ensure_head_col_checked_len();
        if col < base.col_count && col < base.head_col_checked.len() {
            base.head_col_checked[col] = checked;
        }
        self.index_cache_mgr.table_cache_mut().upsert_table_info(table_key, base.clone());
    }

    /// 当前光标所在 `TableRow` 连续块是否至少有一列被勾选（表头列选择）。
    pub fn table_row_block_has_selected_cols(&self, anchor_line_no: usize) -> bool {
        let Some((s, _e)) = self.table_row_block_range(anchor_line_no) else {
            return false;
        };
        let Some(row) = self.get_line(s) else {
            return false;
        };
        let ti = row
            .table_key
            .and_then(|k| self.index_cache_mgr.table_cache().table_info_by_key(k));
        let Some(ti) = ti else { return false; };
        ti.head_col_checked.iter().any(|checked| *checked)
    }

    pub fn table_row_block_set_head_index_checked(
        &mut self,
        anchor_line_no: usize,
        checked: bool,
    ) {
        let Some((s, _e)) = self.table_row_block_range(anchor_line_no) else {
            return;
        };
        let Some(table_key) = self.get_line(s).and_then(|p| p.table_key) else {
            return;
        };
        let Some(mut base) = self.index_cache_mgr.table_cache_mut().table_info_cloned_by_key(table_key) else {
            return;
        };
        base.head_index_checked = checked;
        self.index_cache_mgr.table_cache_mut().upsert_table_info(table_key, base.clone());
    }

    pub fn table_row_block_set_row_index_checked(
        &mut self,
        anchor_line_no: usize,
        row: usize,
        checked: bool,
    ) {
        let Some((s, e)) = self.table_row_block_range(anchor_line_no) else {
            return;
        };
        let n = e.saturating_sub(s).saturating_add(1);
        if row >= n {
            return;
        }
        let target_line = s.saturating_add(row);
        if let Some(p) = self.get_line_mut(target_line) {
            if p.is_table_row() {
                p.row_index_checked = checked;
            }
        }
    }

    pub(super) fn refresh_table_row_block_after_physical_line_deleted(&mut self, deleted_line_index: usize) {
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
        let Some(ti_src) = self.table_info_of_line(anchor_line_no).cloned() else {
            return;
        };
        let col_count = ti_src.col_count;
        let mut nti = ti_src.clone();
        nti.row_index = 0;
        nti.row_count = 0;
        let mut new_row = PghView::new_table_row();
        for _ in 0..col_count {
            new_row.push_text(String::new(), None);
        }
        new_row.table_key = tmpl.table_key;
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

    /// 光标所在逻辑单元格 `(row, col)`（`TableRow` 块统一为整张表的逻辑坐标）。
    pub fn table_cursor_logical_cell(&self) -> Option<(usize, usize)> {
        let c = self.cursor2();
        let p = self.get_line(c.line_no)?;
        let ti = p
            .table_key
            .and_then(|k| self.index_cache_mgr.table_cache().table_info_by_key(k))?;
        if p.is_table_row() {
            let nc = ti.col_count.max(1);
            let col = c.segment.min(nc - 1);
            let row = p
                .table_key
                .and_then(|k| self.table_row_no(c.line_no, k))
                .unwrap_or_else(|| c.line_no.saturating_sub(ti.head_line_no));
            Some((row, col))
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

    /// 删除当前选区涉及的列；至少保留一列。
    pub fn table_delete_selected_cols(&mut self) {
        if !self.cfg().is_markdown || self.cfg().is_read_only {
            return;
        }
        let anchor = self.cursor2().line_no;
        let c1 = self.cursor1();
        let c2 = self.cursor2();
        if let Some((lo, _hi, cl, ch)) = self.table_row_block_column_rect(&c1, &c2) {
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
        }
    }

    pub(super) fn table_row_block_delete_cols_undoable(&mut self, blk_anchor: usize, cols: &[usize]) {
        let Some((s, e)) = self.table_row_block_range(blk_anchor) else {
            return;
        };
        let nc = self
            .table_info_of_line(s)
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
                let ti = self
                    .table_info_of_line(ln)
                    .map(|t| t.col_count)
                    .unwrap_or(0);
                if let Some(p) = self.get_line_mut(ln) {
                    if p.is_table_row() && ti > 1 && c < ti {
                        p.table_row_delete_col(c);
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
    }

    /// 在当前列右侧插入空列（与单元格顶部「右」侧插入列按钮一致）。
    pub fn table_insert_col_right(&mut self) {
        if !self.cfg().is_markdown || self.cfg().is_read_only {
            return;
        }
        let anchor = self.cursor2().line_no;
        let (_r, col) = self.table_cursor_logical_cell().unwrap_or((0usize, 0usize));
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
    }

    /// 按选中列拆分当前表格块：最后选中列作为分组列，之前选中列作为父标题，之后列作为内容表格。
    pub fn table_split_by_selected_cols(&mut self) {
        if !self.cfg().is_markdown || self.cfg().is_read_only {
            return;
        }
        let anchor = self.cursor2().line_no;
        let Some((blk_start, blk_end)) = self.table_row_block_range(anchor) else {
            return;
        };
        let col_count = self
            .table_info_of_line(blk_start)
            .map(|ti| ti.col_count)
            .unwrap_or(0);
        if col_count <= 1 {
            return;
        }

        let mut selected_cols: Vec<usize> = self
            .table_info_of_line(anchor)
            .map(|ti| {
                ti.head_col_checked
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, checked)| {
                        if *checked && idx < col_count {
                            Some(idx)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        if selected_cols.is_empty() {
            let c1 = self.cursor1();
            let c2 = self.cursor2();
            if let Some((line_lo, line_hi, col_lo, col_hi)) = self.table_row_block_column_rect(&c1, &c2) {
                if line_lo >= blk_start && line_hi <= blk_end {
                    selected_cols = (col_lo..=col_hi).collect();
                }
            }
        }

        if selected_cols.is_empty() {
            let col = self
                .table_cursor_logical_cell()
                .map(|(_, c)| c)
                .unwrap_or(0)
                .min(col_count.saturating_sub(1));
            selected_cols.push(col);
        }

        selected_cols.sort_unstable();
        selected_cols.dedup();
        if selected_cols.is_empty() {
            return;
        }

        let split_col = *selected_cols.last().unwrap_or(&0);
        let parent_cols: Vec<usize> = selected_cols
            .iter()
            .copied()
            .filter(|&c| c < split_col)
            .collect();
        let content_cols: Vec<usize> = ((split_col + 1)..col_count).collect();
        if content_cols.is_empty() {
            return;
        }
        let base_heading_level = self.nearest_parent_heading_level_for_line(blk_start);

        let markdown = self.table_row_block_split_markdown(
            blk_start,
            blk_end,
            &parent_cols,
            split_col,
            &content_cols,
            base_heading_level,
        );
        if markdown.trim().is_empty() {
            return;
        }
        self.table_row_block_replace_with_markdown(blk_start, blk_end, &markdown);
    }

    /// 以当前标题（或最近父标题）为根，汇总其子标题下的 Markdown 表格为一张总表。
    /// 汇总列 = 子标题路径列 + 表格列；路径列名使用 `X级目录` / `XX级目录`...。
    pub fn table_merge_under_current_heading(&mut self) {
        if !self.cfg().is_markdown || self.cfg().is_read_only {
            return;
        }
        let Some((scope_start, scope_end, markdown)) = self.current_outline_merged_table_data() else {
            return;
        };
        self.replace_line_range_with_markdown(scope_start, scope_end, &markdown);
    }

    /// 返回当前标题范围下合并后的表格（仅计算，不改动文档）。
    pub fn current_outline_merged_table(&self) -> Option<String> {
        if !self.cfg().is_markdown {
            return None;
        }
        self.current_outline_merged_table_data()
            .map(|(_, _, markdown)| markdown)
    }

    fn current_outline_merged_table_data(&self) -> Option<(usize, usize, String)> {
        let anchor = self.cursor2().line_no;
        let Some((root_line, root_level)) = self.nearest_heading_at_or_before(anchor) else {
            return None;
        };
        let Some((scope_start, scope_end)) = self.heading_scope_range(root_line, root_level) else {
            return None;
        };
        if scope_start > scope_end || scope_end >= self.pgh_views.len() {
            return None;
        }
        let markdown = self.merge_heading_scope_tables_markdown(scope_start, scope_end, root_level);
        if markdown.trim().is_empty() {
            return None;
        }
        Some((scope_start, scope_end, markdown))
    }

    pub(super) fn table_row_block_split_markdown(
        &self,
        blk_start: usize,
        blk_end: usize,
        parent_cols: &[usize],
        split_col: usize,
        content_cols: &[usize],
        base_heading_level: usize,
    ) -> String {
        let Some(header_row) = self.get_line(blk_start) else {
            return String::new();
        };
        let col_count = header_row
            .table_key
            .and_then(|k| self.table_info_by_key(k))
            .map(|ti| ti.col_count)
            .unwrap_or(0);
        if col_count == 0 || split_col >= col_count || content_cols.is_empty() {
            return String::new();
        }

        #[derive(Clone)]
        struct GroupBlock {
            parent_vals: Vec<String>,
            group_title: String,
            rows: Vec<Vec<String>>,
        }

        let col_titles: Vec<String> = (0..col_count)
            .map(|c| header_row.get_segment_text(c).trim().to_string())
            .collect();
        let content_headers: Vec<String> = content_cols
            .iter()
            .map(|&c| col_titles.get(c).cloned().unwrap_or_default())
            .collect();

        let mut carry_vals = vec![String::new(); col_count];
        let mut blocks: Vec<GroupBlock> = Vec::new();
        for ln in (blk_start + 1)..=blk_end {
            let Some(row) = self.get_line(ln) else {
                continue;
            };
            if !row.is_table_row() {
                continue;
            }
            let row_cells: Vec<String> = (0..col_count)
                .map(|c| row.get_segment_text(c).trim().to_string())
                .collect();
            let is_separator = !row_cells.is_empty()
                && row_cells.iter().all(|cell| {
                    let s = cell.trim();
                    !s.is_empty() && s.chars().all(|ch| ch == '-' || ch == ':')
                });
            if is_separator {
                continue;
            }

            let mut effective_vals = vec![String::new(); col_count];
            for c in 0..col_count {
                if row_cells[c].is_empty() {
                    effective_vals[c] = carry_vals[c].clone();
                } else {
                    effective_vals[c] = row_cells[c].clone();
                    carry_vals[c] = row_cells[c].clone();
                }
            }
            let group_title = effective_vals
                .get(split_col)
                .cloned()
                .unwrap_or_default()
                .trim()
                .to_string();
            if group_title.is_empty() {
                continue;
            }
            let parent_vals: Vec<String> = parent_cols
                .iter()
                .map(|&c| effective_vals.get(c).cloned().unwrap_or_default())
                .collect();
            let content_row: Vec<String> = content_cols
                .iter()
                .map(|&c| row_cells.get(c).cloned().unwrap_or_default())
                .collect();

            if let Some(last) = blocks.last_mut() {
                if last.parent_vals == parent_vals && last.group_title == group_title {
                    last.rows.push(content_row);
                    continue;
                }
            }
            blocks.push(GroupBlock {
                parent_vals,
                group_title,
                rows: vec![content_row],
            });
        }

        if blocks.is_empty() {
            return String::new();
        }

        let mut out_lines: Vec<String> = Vec::new();
        let mut last_parent_vals: Vec<String> = Vec::new();
        for block in blocks {
            let mut prefix_len = 0usize;
            while prefix_len < last_parent_vals.len()
                && prefix_len < block.parent_vals.len()
                && last_parent_vals[prefix_len] == block.parent_vals[prefix_len]
            {
                prefix_len += 1;
            }
            for idx in prefix_len..block.parent_vals.len() {
                let title = block.parent_vals[idx].trim();
                if title.is_empty() {
                    continue;
                }
                let level = (base_heading_level + idx + 1).clamp(1, 6);
                out_lines.push(format!("{} {}", "#".repeat(level), title));
            }
            last_parent_vals = block.parent_vals.clone();

            let group_level = (base_heading_level + block.parent_vals.len() + 1).clamp(1, 6);
            out_lines.push(format!("{} {}", "#".repeat(group_level), block.group_title));
            out_lines.push(format!("|{}|", content_headers.join("|")));
            out_lines.push(format!(
                "|{}|",
                std::iter::repeat("--")
                    .take(content_headers.len())
                    .collect::<Vec<_>>()
                    .join("|")
            ));
            for row in block.rows {
                out_lines.push(format!("|{}|", row.join("|")));
            }
            out_lines.push(String::new());
        }
        while out_lines.last().is_some_and(|l| l.is_empty()) {
            out_lines.pop();
        }
        out_lines.join("\n")
    }

    fn nearest_heading_at_or_before(&self, line_no: usize) -> Option<(usize, usize)> {
        let start = line_no.min(self.pgh_views.len().saturating_sub(1));
        for ln in (0..=start).rev() {
            let Some(pgh) = self.get_line(ln) else {
                continue;
            };
            if pgh.pgh_type != PghType::Heading {
                continue;
            }
            return Some((ln, self.heading_level_from_pghview(pgh)));
        }
        None
    }

    fn heading_scope_range(&self, root_line: usize, root_level: usize) -> Option<(usize, usize)> {
        if self.pgh_views.is_empty() || root_line >= self.pgh_views.len() {
            return None;
        }
        let start = root_line.saturating_add(1);
        if start >= self.pgh_views.len() {
            return None;
        }
        let mut end = self.pgh_views.len().saturating_sub(1);
        for ln in start..self.pgh_views.len() {
            let Some(pgh) = self.get_line(ln) else {
                continue;
            };
            if pgh.pgh_type != PghType::Heading {
                continue;
            }
            let lvl = self.heading_level_from_pghview(pgh);
            if lvl <= root_level {
                end = ln.saturating_sub(1);
                break;
            }
        }
        if start > end {
            None
        } else {
            Some((start, end))
        }
    }

    fn merge_heading_scope_tables_markdown(
        &self,
        scope_start: usize,
        scope_end: usize,
        root_level: usize,
    ) -> String {
        #[derive(Clone)]
        struct MergeRow {
            path: Vec<String>,
            cells: Vec<String>,
        }

        let mut rows: Vec<MergeRow> = Vec::new();
        let mut table_header: Option<Vec<String>> = None;
        let mut heading_stack: Vec<(usize, String)> = Vec::new();
        let mut ln = scope_start;
        while ln <= scope_end {
            let Some(pgh) = self.get_line(ln) else {
                ln += 1;
                continue;
            };
            if pgh.pgh_type == PghType::Heading {
                let level = self.heading_level_from_pghview(pgh);
                while heading_stack.last().is_some_and(|(lvl, _)| *lvl >= level) {
                    heading_stack.pop();
                }
                if level > root_level {
                    heading_stack.push((level, pgh.get_text().trim().trim_start_matches('#').trim().to_string()));
                } else {
                    heading_stack.clear();
                }
                ln += 1;
                continue;
            }
            if pgh.is_table_row() {
                let Some((blk_s, blk_e)) = self.table_row_block_range(ln) else {
                    ln += 1;
                    continue;
                };
                let col_count = self
                    .table_info_of_line(blk_s)
                    .map(|ti| ti.col_count)
                    .unwrap_or(0);
                if col_count == 0 {
                    ln = blk_e.saturating_add(1);
                    continue;
                }
                let header: Vec<String> = (0..col_count)
                    .map(|c| self.get_line(blk_s).map(|r| r.get_segment_text(c).trim().to_string()).unwrap_or_default())
                    .collect();
                if table_header.is_none() {
                    table_header = Some(header);
                }
                for rln in (blk_s + 1)..=blk_e {
                    let Some(row) = self.get_line(rln) else {
                        continue;
                    };
                    if !row.is_table_row() {
                        continue;
                    }
                    if !row.get_text().trim_start().starts_with('|') {
                        continue;
                    }
                    let row_cells: Vec<String> = (0..col_count)
                        .map(|c| row.get_segment_text(c).trim().to_string())
                        .collect();
                    let is_separator = !row_cells.is_empty()
                        && row_cells.iter().all(|cell| {
                            let s = cell.trim();
                            !s.is_empty() && s.chars().all(|ch| ch == '-' || ch == ':')
                        });
                    if is_separator {
                        continue;
                    }
                    let has_data = row_cells.iter().any(|c| !c.trim().is_empty());
                    if !has_data {
                        continue;
                    }
                    // 忽略夹在子表间的普通段落被“抬升”为首列单值行的情况。
                    // 这类行通常只有首列有值，其余列全空，不应进入合并结果。
                    let non_empty_count = row_cells
                        .iter()
                        .filter(|c| !c.trim().is_empty())
                        .count();
                    if non_empty_count == 1
                        && row_cells
                            .iter()
                            .enumerate()
                            .all(|(idx, c)| idx == 0 || c.trim().is_empty())
                    {
                        continue;
                    }
                    rows.push(MergeRow {
                        path: heading_stack.iter().map(|(_, t)| t.clone()).collect(),
                        cells: row_cells,
                    });
                }
                ln = blk_e.saturating_add(1);
                continue;
            }
            ln += 1;
        }
        let Some(header) = table_header else {
            return String::new();
        };
        if rows.is_empty() {
            return String::new();
        }
        let max_depth = rows.iter().map(|r| r.path.len()).max().unwrap_or(0);
        let mut merged_header: Vec<String> = (0..max_depth)
            .map(|idx| format!("{}级目录", "X".repeat(idx + 1)))
            .collect();
        merged_header.extend(header.iter().cloned());

        let mut out_lines = Vec::<String>::new();
        out_lines.push(format!("|{}|", merged_header.join("|")));
        out_lines.push(format!(
            "|{}|",
            std::iter::repeat("--")
                .take(merged_header.len())
                .collect::<Vec<_>>()
                .join("|")
        ));
        for row in rows {
            let mut path_cells = vec![String::new(); max_depth];
            for (idx, val) in row.path.iter().enumerate().take(max_depth) {
                path_cells[idx] = val.clone();
            }
            let mut cells = row.cells;
            if cells.len() < header.len() {
                cells.extend(std::iter::repeat(String::new()).take(header.len() - cells.len()));
            }
            if cells.len() > header.len() {
                cells.truncate(header.len());
            }
            path_cells.extend(cells);
            out_lines.push(format!("|{}|", path_cells.join("|")));
        }
        out_lines.join("\n")
    }

    pub(super) fn nearest_parent_heading_level_for_line(&self, line_no: usize) -> usize {
        if line_no == 0 {
            return 1;
        }
        for ln in (0..line_no).rev() {
            let Some(pgh) = self.get_line(ln) else {
                continue;
            };
            if pgh.pgh_type != PghType::Heading {
                continue;
            }
            return self.heading_level_from_pghview(pgh);
        }
        1
    }

    pub(super) fn heading_level_from_pghview(&self, pgh: &PghView) -> usize {
        let line = pgh.get_text();
        let s = line.trim_start();
        let mut level = 0usize;
        for ch in s.chars() {
            if ch != '#' {
                break;
            }
            level += 1;
            if level >= 6 {
                return 6;
            }
        }
        if level == 0 { 1 } else { level }
    }

    pub(super) fn table_row_block_replace_with_markdown(
        &mut self,
        blk_start: usize,
        blk_end: usize,
        markdown: &str,
    ) {
        if markdown.is_empty() || blk_start > blk_end || blk_end >= self.pgh_views.len() {
            return;
        }
        let before_snapshot = self.pgh_views.clone();
        let old_cursor = self.cursor2();
        let parsed_ctx = Ctx::new().with_cfg(self.cfg()).with_text(markdown, true);
        let mut replacement = parsed_ctx.pgh_views.clone();
        if replacement.is_empty() {
            return;
        }
        // replacement 来自临时 Ctx，其 table/code key 空间与当前文档无关。
        // 若直接带入，可能与当前缓存 key 冲突，导致分帧重建期间元数据互相覆盖并抖动。
        for p in &mut replacement {
            if p.is_table_row() {
                p.table_key = None;
            }
            if p.is_code_row() {
                p.code_key = None;
            }
        }

        self.pgh_views.splice(blk_start..=blk_end, replacement);
        let line_no = blk_start.min(self.pgh_views.len().saturating_sub(1));
        let new_cursor = self.cursor_check(&Cursor {
            line_no,
            segment: 0,
            culumn: 0,
        });
        self.set_cursor2(new_cursor);
        self.set_cursor1_reset();

        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        undo_cmd.set_cursor(old_cursor);
        undo_cmd.push_replace_all(before_snapshot);
        redo_cmd.set_cursor(self.cursor2());
        redo_cmd.push_replace_all(self.pgh_views.clone());
        self.push_do(undo_cmd, redo_cmd);
        self.on_content_change();
    }

    fn replace_line_range_with_markdown(
        &mut self,
        start_line: usize,
        end_line: usize,
        markdown: &str,
    ) {
        if markdown.is_empty() || start_line > end_line || end_line >= self.pgh_views.len() {
            return;
        }
        let before_snapshot = self.pgh_views.clone();
        let old_cursor = self.cursor2();
        let parsed_ctx = Ctx::new().with_cfg(self.cfg()).with_text(markdown, true);
        let mut replacement = parsed_ctx.pgh_views.clone();
        if replacement.is_empty() {
            return;
        }
        for p in &mut replacement {
            if p.is_table_row() {
                p.table_key = None;
            }
            if p.is_code_row() {
                p.code_key = None;
            }
        }
        self.pgh_views.splice(start_line..=end_line, replacement);
        let line_no = start_line.min(self.pgh_views.len().saturating_sub(1));
        let new_cursor = self.cursor_check(&Cursor {
            line_no,
            segment: 0,
            culumn: 0,
        });
        self.set_cursor2(new_cursor);
        self.set_cursor1_reset();

        let mut undo_cmd = DoCmd::new();
        let mut redo_cmd = DoCmd::new();
        undo_cmd.set_cursor(old_cursor);
        undo_cmd.push_replace_all(before_snapshot);
        redo_cmd.set_cursor(self.cursor2());
        redo_cmd.push_replace_all(self.pgh_views.clone());
        self.push_do(undo_cmd, redo_cmd);
        self.on_content_change();
    }


    /// 在 `TableRow` 连续块内将剪贴板中的整张 GFM 表按单元格合并到当前锚点。
    /// 返回 `(光标, 合并后块最后一行的物理行号)`，供 undo 中 `push_delete` 使用。
    pub(super) fn table_row_block_merge_paste(
        &mut self,
        anchor_line_no: usize,
        anchor_segment: usize,
        table: &ParsedTable,
    ) -> Option<(Cursor, usize)> {
        let p = self.get_line(anchor_line_no)?;
        if !p.is_table_row() {
            return None;
        }
        let (blk_start, mut blk_end) = self.table_row_block_range(anchor_line_no)?;
        let min_row = p
            .table_key
            .and_then(|k| self.table_row_no(anchor_line_no, k))
            .unwrap_or(0);
        let min_col = anchor_segment.min(self.table_info_of_line(anchor_line_no)?.col_count.saturating_sub(1));
        let mut col_count = self.table_info_of_line(anchor_line_no)?.col_count;
        let mut n_rows = blk_end.saturating_sub(blk_start) + 1;
        let change_rows = table.row_count.max(1);
        let need_rows = min_row + change_rows;
        let need_cols = min_col + table.col_count;

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

        for r in 0..change_rows {
            for c in 0..table.col_count {
                let org_seg = r * table.col_count + c;
                let org_txt = table.cells.get(org_seg).cloned().unwrap_or_default();
                let dst_ln = blk_start + min_row + r;
                let dst_col = min_col + c;
                if let Some(dst_p) = self.get_line_mut(dst_ln) {
                    dst_p.update_segment_text(dst_col, org_txt);
                }
            }
        }
        self.refresh_table_row_block_metadata(blk_start);

        let (_, blk_end_final) = self.table_row_block_range(blk_start)?;
        Some((
            Cursor {
                line_no: blk_start + min_row,
                segment: min_col,
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

    /// 大纲折叠隐藏行：清空布局 rect，避免残留命中区域；行高在 [`super::Ctx::record_line_scroll_height_after_layout`] 中置 0。
    pub(crate) fn clear_line_layout_rect(&mut self, line_no: usize) {
        if let Some(pv) = self.pgh_views.get_mut(line_no) {
            pv.rect = None;
        }
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

        // 全选整个文档时走快速路径：直接替换为一行空文本，避免逐行 update/delete 的巨大开销
        // 代码行末尾保留逻辑继续走原流程，保持既有行为一致。
        if self.is_document_fully_selected() && cursor2_line_type != Some(PghType::CodeRow) {
            let full_snapshot = self.pgh_views.clone();
            let mut empty_line = PghView::new_text();
            empty_line.push_text(String::new(), None);

            self.pgh_views.clear();
            self.pgh_views.push(empty_line.clone());
            self.set_cursors_to_min();
            self.set_cursor2(self.cursor_check(&self.cursor2()));
            self.set_cursor1_reset();

            undo_cmd.push_replace_all(full_snapshot);
            redo_cmd.push_replace_all(vec![empty_line]);
            return (undo_cmd, redo_cmd);
        }

        let c1 = self.cursor1();
        let c2 = self.cursor2();
        let tr_rect = self.table_row_block_column_rect(&c1, &c2);
        let mut full_row_delete_lines: Vec<usize> = Vec::new();

        if let Some((lo, hi, cl, ch)) = tr_rect {
            if let Some((blk_s, blk_e)) = self.table_row_block_range(lo) {
                let nc = self
                    .table_info_of_line(lo)
                    .map(|t| t.col_count)
                    .unwrap_or(0);
                if nc > 0 {
                    let is_full_rows = cl == 0 && ch + 1 == nc;
                    let is_full_cols = lo == blk_s && hi == blk_e;
                    if is_full_rows {
                        for ln in lo..=hi {
                            full_row_delete_lines.push(ln);
                        }
                    }
                    // 整列选择：直接执行结构删列，不走后续“文本删空后折叠”的通用路径。
                    if is_full_cols && !is_full_rows {
                        let mut cols: Vec<usize> = (cl..=ch).collect();
                        cols.sort_unstable();
                        cols.dedup();
                        cols.reverse();
                        for &col in &cols {
                            self.table_row_block_delete_col(blk_s, col, &mut undo_cmd, &mut redo_cmd);
                        }
                        let c2 = self.cursor_check(&self.cursor2());
                        self.set_cursor2(c2);
                        self.set_cursor1_reset();
                        redo_cmd.set_cursor(self.cursor2());
                        return (undo_cmd, redo_cmd);
                    }
                }
            }
        }

        // 跨段（含普通文本）选区时，`tr_rect` 为空；若某个 TableRow 被整行覆盖，也应允许删行，
        // 否则会退化为仅清空单元格文本，留下 `||` 空表格行。
        if tr_rect.is_none() {
            let min = std::cmp::min(c1, c2);
            let max = std::cmp::max(c1, c2);
            for line_no in self.get_selected_line_nos() {
                let Some(p) = self.get_line(line_no) else {
                    continue;
                };
                if !p.is_table_row() {
                    continue;
                }
                let line_start = p.start_cursor_of_line(line_no);
                let line_end = self.cursor_check(&p.end_cursor_of_line(line_no));
                if min <= line_start && max >= line_end {
                    full_row_delete_lines.push(line_no);
                }
            }
        }

        full_row_delete_lines.sort_unstable();
        full_row_delete_lines.dedup();

        if c1.line_no == c2.line_no {
            if let Some(p) = self.get_line(c1.line_no) {
                if p.is_table_row() {
                    let nc = self
                        .table_info_of_line(c1.line_no)
                        .map(|t| t.col_count)
                        .unwrap_or(0);
                    if nc > 0 {
                        let col_lo = c1.segment.min(c2.segment).min(nc.saturating_sub(1));
                        let col_hi = c1.segment.max(c2.segment).min(nc.saturating_sub(1));
                        if col_lo == 0 && col_hi + 1 == nc {
                            full_row_delete_lines.push(c1.line_no);
                        }
                    }
                }
            }
        }

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

        for (_i, (line_no, after_delete)) in line_set.iter().enumerate() {
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
            let is_table_row_line = self.get_line(*line_no).is_some_and(|p| p.is_table_row());
            let allow_delete_table_row = full_row_delete_lines.iter().any(|ln| ln == line_no);
            // if the line is code and the line is empty, do not delete it
            if new_s.len() == 0
                && !(line_no == &self.cursor2().line_no && cursor2_line_type == Some(PghType::CodeRow))
                && (!is_table_row_line || allow_delete_table_row)
            {
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
            let (first_line_no, _first_s, first_segments) = remain_lines.last().unwrap();
            let (_, last_s, _) = remain_lines.first().unwrap();
            let last_line_no = first_line_no + 1;
            if let Some(last) = self.pgh_views.get(last_line_no) {
                if let Some(first) = self.pgh_views.get(*first_line_no) {
                    let merge_two_code_rows = first.is_code_row()
                        && last.is_code_row()
                        && last_line_no == *first_line_no + 1
                        && match (
                            self.code_row_block_range(*first_line_no),
                            self.code_row_block_range(last_line_no),
                        ) {
                            (Some(a), Some(b)) => a == b,
                            _ => false,
                        };
                    if !last.is_table_like()
                        && !first.is_table_like()
                        && ((!first.is_code_row() && !last.is_code_row()) || merge_two_code_rows)
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
                        if merge_two_code_rows {
                            self.refresh_code_row_block_after_physical_line_deleted(last_line_no);
                        }
                    }
                }
            }
        }
        self.collapse_empty_text_between_table_rows(&mut undo_cmd, &mut redo_cmd);
        let c2 = self.cursor_check(&self.cursor2());
        self.set_cursor2(c2);
        self.set_cursor1_reset();
        log::debug!("cursor after delete: {:?}", self.cursor2());
        return (undo_cmd, redo_cmd);
    }

    /// 在块内每一行删除同一列（`col` 为 `0..col_count`），至少保留一列。
    pub(super) fn table_row_block_delete_col(
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
            .table_info_of_line(s)
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
    pub(super) fn collapse_empty_text_between_table_rows(
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

    pub(super) fn check_to_table_pghview(&self, s: &str) -> Option<ParsedTable> {
        if !s.starts_with("|") {
            return None;
        }
        let rows = self.check_to_table_row_pghviews(s)?;
        let first = rows.first()?;
        let row_count = rows.len();
        let col_count = first.pgh.len();
        if row_count == 0 || col_count == 0 {
            return None;
        }
        let mut cells = Vec::with_capacity(row_count * col_count);
        for row in &rows {
            for col in 0..col_count {
                let txt = row.get_segment_text(col);
                cells.push(txt);
            }
        }
        Some(ParsedTable {
            col_count,
            row_count,
            cells,
        })
    }

    /// 管道块解析为多个 `PghType::TableRow`（GFM 单表根节点）
    pub fn check_to_table_row_pghviews(&self, s: &str) -> Option<Vec<PghView>> {
        if !s.starts_with("|") {
            return None;
        }
        let markdown = MarkDownImpl::new(s, true, None, false, self.cfg());
        markdown.markdown_to_table_rows_if_single_table()
    }

    pub(super) fn change_to_table_by_anchor_line(&mut self, check_line: usize) -> Option<(usize, usize)> {
        if check_line >= self.pgh_views.len() {
            return None;
        }
        let check_text = self.get_line_text(check_line);
        if !check_text.starts_with("|") || self.is_table_line(check_line) {
            return None;
        }

        //collect lines begin with |
        let mut top = vec![];
        for line in (0..=check_line).rev() {
            let txt = self.get_line_text(line);
            if txt.starts_with("|") && !self.is_table_line(line) {
                top.push((line, txt));
            } else {
                break;
            }
        }
        let mut bottom = vec![];
        for line in (check_line + 1)..self.pgh_views.len() {
            let txt = self.get_line_text(line);
            if txt.starts_with("|") && !self.is_table_line(line) {
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
            return Some((line, line + rows.len().saturating_sub(1)));
        }
        None
    }

    pub(super) fn change_to_table_in_line_range(&mut self, line_start: usize, line_end: usize) {
        if self.pgh_views.is_empty() || line_start > line_end {
            return;
        }
        let mut line = line_start.min(self.pgh_views.len() - 1);
        let mut end = line_end.min(self.pgh_views.len() - 1);
        while line <= end && line < self.pgh_views.len() {
            if let Some((_, converted_end)) = self.change_to_table_by_anchor_line(line) {
                line = converted_end.saturating_add(1);
                if self.pgh_views.is_empty() {
                    break;
                }
                end = end.min(self.pgh_views.len() - 1);
                continue;
            }
            line += 1;
        }
    }

    pub fn on_change_to_table(&mut self) {
        let cursor = self.cursor2();
        let cur_text = self.get_line_text(cursor.line_no);
        let check_line = if cur_text.starts_with("|") && !self.is_table_line(cursor.line_no) {
            Some(cursor.line_no)
        } else if cur_text.is_empty() && cursor.line_no > 0 {
            let prev = cursor.line_no - 1;
            if self.get_line_text(prev).starts_with("|") && !self.is_table_line(prev) {
                Some(prev)
            } else {
                None
            }
        } else {
            None
        };
        if let Some(line) = check_line {
            self.change_to_table_by_anchor_line(line);
        }
    }

    pub(super) fn content_change_state(&mut self) {
        self.state.change_current_tick += 1;

        //clean same cache
        self.flash_same_cache_with_seleted();

        //flag need reset ime area
        self.set_ime_area_changed(true);
    }

    pub(super) fn alloc_table_key(&mut self) -> TableKey {
        self.index_cache_mgr.table_cache_mut().alloc_table_key()
    }

    pub fn table_info_by_key(&self, table_key: TableKey) -> Option<&TableInfo> {
        self.index_cache_mgr.table_cache().table_info_by_key(table_key)
    }

    pub fn table_info_by_key_mut(&mut self, table_key: TableKey) -> Option<&mut TableInfo> {
        self.index_cache_mgr.table_cache_mut().table_info_by_key_mut(table_key)
    }

    pub fn table_key_of_line(&self, line_no: usize) -> Option<TableKey> {
        self.get_line(line_no).and_then(|p| p.table_key)
    }

    pub fn table_info_of_line(&self, line_no: usize) -> Option<&TableInfo> {
        let p = self.get_line(line_no)?;
        p.table_key
            .and_then(|k| self.index_cache_mgr.table_cache().table_info_by_key(k))
    }

    pub fn table_row_no(&self, line_no: usize, table_key: TableKey) -> Option<usize> {
        let ti = self.table_info_by_key(table_key)?;
        let row_no = line_no.saturating_sub(ti.head_line_no);
        Some(row_no)
    }

    /// 将 `cfg.table_frame_style` 写入已解析的表格段落（解析时会把该值快照到 `TableInfo`）。
    pub fn sync_table_views_frame_style(&mut self) {
        let style = self.cfg.table_frame_style.clone();
        for ti in self.index_cache_mgr.table_cache_mut().table_infos_mut() {
            ti.frame_style = style.clone();
        }
    }
}
