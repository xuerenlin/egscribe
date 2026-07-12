//! Markdown 标题 / 目录：从 [`Ctx`] 提取标题层级与多级编号；编辑区标题行折叠 / 展开。

use std::collections::{HashMap, HashSet};

use super::Ctx;
use super::index::IndexCache;

/// 与 [`Ctx::get_line_text`] 配合：解析 ATX 标题行级别（1..=6），与分帧重建目录逻辑一致。
pub(crate) fn parse_heading_level_for_toc(line: &str) -> u8 {
    let s = line.trim_start();
    let mut n = 0u8;
    for ch in s.chars() {
        if ch == '#' {
            n += 1;
            if n >= 6 {
                return 6;
            }
        } else {
            break;
        }
    }
    n.clamp(1, 6)
}

/// 挂在 [`Ctx`] 上的目录缓存（按文档、按编辑上下文保存，避免随侧栏 UI 重建而丢失）。
#[derive(Clone, Debug)]
pub struct TocCache {
    pub entries: Vec<TocEntry>,
    /// 按行号快速查找（与 `entries` 同步更新）
    pub by_line: HashMap<usize, TocEntry>,
    /// 当前重建任务已经扫描到的标题行号，用于结束时做删除清理。
    rebuild_seen_lines: HashSet<usize>,
    /// 当前重建任务的最新标题快照（按行号）。
    rebuild_by_line: HashMap<usize, TocEntry>,
    /// 分帧 `rebuild_index` 时维护的多级编号计数（由 [`IndexCache::rebuild_index_init`] 清零）。
    rebuild_section_counts: [u32; 6],
}

impl TocCache {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            by_line: HashMap::new(),
            rebuild_seen_lines: HashSet::new(),
            rebuild_by_line: HashMap::new(),
            rebuild_section_counts: [0; 6],
        }
    }

    pub fn entry_for_line(&self, line_no: usize) -> Option<&TocEntry> {
        self.by_line.get(&line_no)
    }

    pub(crate) fn next_section_number_for_rebuild(&mut self, level: u8) -> Option<String> {
        let l = level as usize;
        if !(1..=6).contains(&l) {
            return None;
        }
        self.rebuild_section_counts[l - 1] += 1;
        for c in self.rebuild_section_counts.iter_mut().skip(l) {
            *c = 0;
        }
        Some(
            self.rebuild_section_counts[..l]
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join("."),
        )
    }

    pub(crate) fn push_rebuild_entry(&mut self, entry: TocEntry) {
        self.rebuild_seen_lines.insert(entry.line_no);
        self.rebuild_by_line.insert(entry.line_no, entry.clone());
        self.upsert_live_entry(entry);
    }

    fn upsert_live_entry(&mut self, entry: TocEntry) {
        self.by_line.insert(entry.line_no, entry.clone());
        if let Some(existing) = self.entries.iter_mut().find(|e| e.line_no == entry.line_no) {
            *existing = entry;
        } else {
            self.entries.push(entry);
        }
    }

    fn remove_live_entry_by_line(&mut self, line_no: usize) {
        self.by_line.remove(&line_no);
        self.entries.retain(|e| e.line_no != line_no);
    }

    fn cleanup_deleted_entries_after_rebuild(&mut self) {
        self.entries
            .retain(|e| self.rebuild_seen_lines.contains(&e.line_no));
        self.by_line
            .retain(|line_no, _| self.rebuild_seen_lines.contains(line_no));
        // step 中是按批次 upsert，这里统一按文档顺序收敛一次，避免插入顺序抖动。
        self.entries.sort_by_key(|e| e.line_no);
        // by_line 与 entries 最终对齐（不清空，按增量修补缺失项）。
        for e in &self.entries {
            if !self.by_line.contains_key(&e.line_no) {
                self.by_line.insert(e.line_no, e.clone());
            }
        }
    }

}

impl IndexCache for TocCache {
    fn rebuild_index_init(&mut self, _gen: u64) {
        self.rebuild_seen_lines.clear();
        self.rebuild_by_line.clear();
        self.rebuild_section_counts = [0; 6];
    }

    fn rebuild_index_step(&mut self, ctx: &mut Ctx, from: usize) -> usize {
        if !ctx.cfg().is_markdown {
            return from.saturating_add(1);
        }
        let n = ctx.line_num();
        if from >= n {
            return n;
        }
        let batch = ctx.patch_num().max(1);
        let end = from.saturating_add(batch).min(n);
        for line_no in from..end {
            if ctx.is_heading_line(line_no) {
                let full = ctx.get_line_text(line_no);
                let level = parse_heading_level_for_toc(&full);
                let title = Ctx::remove_heading_prefix(&full);
                if let Some(section_number) = self.next_section_number_for_rebuild(level) {
                    let entry = TocEntry {
                        line_no,
                        level,
                        title,
                        section_number,
                    };
                    self.push_rebuild_entry(entry);
                }
                let collapsed = ctx
                    .pgh_views
                    .get(line_no)
                    .and_then(|p| p.outline_fold_collapsed())
                    .unwrap_or(false)
                    || ctx.outline_section_content_appears_folded(line_no);
                if let Some(pv) = ctx.pgh_views.get_mut(line_no) {
                    pv.heading_level = Some(level);
                    pv.ensure_outline_fold_segment(collapsed);
                }
            } else {
                // 增量扫描到非标题行时，立即移除该行旧目录项，避免残留到任务结束。
                self.remove_live_entry_by_line(line_no);
            }
        }
        end
    }

    fn rebuild_index_end(&mut self, ctx: &mut Ctx, _gen: u64) {
        self.cleanup_deleted_entries_after_rebuild();
        ctx.line_flash_all();
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// 单条目录项：对应编辑器中一行 ATX 标题。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TocEntry {
    pub line_no: usize,
    /// 1..=6
    pub level: u8,
    /// 标题正文（已去掉行首 `#` 与空格）
    pub title: String,
    /// 多级编号，如 `1`、`2.1`（Markdown 标题行在 [`crate::medit::pgh::PghView::layout_sigle_line`] 内与正文一起绘制）
    pub section_number: String,
}

/// 目录树节点（按标题层级 `level` 嵌套）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TocNode {
    pub entry: TocEntry,
    /// 主光标行是否落在此标题所覆盖的节内（从本标题行到下一同级/更高级标题行之前）。
    pub cursor_in_section: bool,
    pub children: Vec<TocNode>,
}

/// 判断 `entries[j]` 对应节是否包含行号 `cursor_line`（与 `toc_entries` 文档顺序一致）。
fn toc_entry_cursor_in_section(entries: &[TocEntry], j: usize, cursor_line: usize) -> bool {
    if cursor_line < entries[j].line_no {
        return false;
    }
    for k in (j + 1)..entries.len() {
        if entries[k].level <= entries[j].level {
            return cursor_line < entries[k].line_no;
        }
    }
    true
}

fn parse_toc_node(entries: &[TocEntry], i: &mut usize, cursor_line: usize) -> TocNode {
    let j = *i;
    let e = entries[j].clone();
    *i += 1;
    let cursor_in_section = toc_entry_cursor_in_section(entries, j, cursor_line);
    let mut children = Vec::new();
    while *i < entries.len() && entries[*i].level > e.level {
        children.push(parse_toc_node(entries, i, cursor_line));
    }
    TocNode {
        entry: e,
        cursor_in_section,
        children,
    }
}

/// 将扁平标题序列转为森林（与文档顺序、层级一致），并根据 `cursor_line` 标出光标所在节。
pub fn toc_entries_to_forest(entries: &[TocEntry], cursor_line: usize) -> Vec<TocNode> {
    let mut i = 0;
    let mut roots = Vec::new();
    while i < entries.len() {
        roots.push(parse_toc_node(entries, &mut i, cursor_line));
    }
    roots
}

impl Ctx {
    /// 大纲折叠或搜索过滤隐藏行：清理布局 rect、同步行高为 0 并刷新 LinePos。
    ///
    /// 返回 `true` 表示该行应跳过 [`crate::medit::layout::Layout::draw_all_pgh`] 中的 horizontal 绘制。
    pub(crate) fn prepare_render_hidden_line_for_draw(&mut self, line_no: usize) -> bool {
        if !self
            .pgh_views
            .get(line_no)
            .map(|p| p.is_render_hidden())
            .unwrap_or(false)
        {
            return false;
        }
        self.clear_line_layout_rect(line_no);
        self.record_line_scroll_height_after_layout(line_no);
        true
    }

    /// 标题行之下、下一同级或更高级标题之前的正文行范围（不含标题行自身）。
    pub fn outline_section_content_range(&self, heading_line: usize) -> Option<std::ops::Range<usize>> {
        if !self.cfg().is_markdown || !self.is_heading_line(heading_line) {
            return None;
        }
        let root_level = self
            .pgh_views
            .get(heading_line)
            .and_then(|p| p.heading_level)
            .unwrap_or_else(|| parse_heading_level_for_toc(&self.get_line_text(heading_line)));
        let start = heading_line.saturating_add(1);
        let n = self.line_num();
        if start >= n {
            return Some(start..start);
        }
        let mut end = n;
        for line_no in start..n {
            if self.is_heading_line(line_no) {
                let lev = parse_heading_level_for_toc(&self.get_line_text(line_no));
                if lev <= root_level {
                    end = line_no;
                    break;
                }
            }
        }
        Some(start..end)
    }

    pub fn outline_heading_has_foldable_content(&self, heading_line: usize) -> bool {
        self.outline_section_content_range(heading_line)
            .map(|r| !r.is_empty())
            .unwrap_or(false)
    }

    /// 标题下内容行是否仍被本标题级别折叠（用于重解析后恢复折叠 UI）。
    pub(crate) fn outline_section_content_appears_folded(&self, heading_line: usize) -> bool {
        let Some(root_level) = self
            .pgh_views
            .get(heading_line)
            .and_then(|p| p.heading_level)
        else {
            return false;
        };
        let idx = root_level.saturating_sub(1) as usize;
        if idx >= 6 {
            return false;
        }
        let Some(range) = self.outline_section_content_range(heading_line) else {
            return false;
        };
        range.clone().any(|line_no| {
            self.pgh_views
                .get(line_no)
                .map(|p| p.hidden_by_level[idx])
                .unwrap_or(false)
        })
    }

    pub fn toggle_outline_fold(&mut self, heading_line: usize) {
        let folded = self
            .pgh_views
            .get(heading_line)
            .and_then(|p| p.outline_fold_collapsed())
            .unwrap_or(false);
        self.set_outline_folded(heading_line, !folded);
    }

    pub fn set_outline_folded(&mut self, heading_line: usize, folded: bool) {
        if !self.is_heading_line(heading_line) {
            return;
        }
        let Some(content_range) = self.outline_section_content_range(heading_line) else {
            return;
        };
        if content_range.is_empty() && folded {
            return;
        }
        let root_level = self
            .pgh_views
            .get(heading_line)
            .and_then(|p| p.heading_level)
            .unwrap_or_else(|| parse_heading_level_for_toc(&self.get_line_text(heading_line)));

        if let Some(p) = self.pgh_views.get_mut(heading_line) {
            p.set_outline_fold_collapsed_on_segment(folded);
        }

        for line_no in content_range.clone() {
            if let Some(p) = self.pgh_views.get_mut(line_no) {
                p.set_hidden_at_level(root_level, folded);
            }
        }
        self.line_flash_tick(heading_line);

        self.migrate_cursor_after_outline_fold(heading_line, folded, &content_range);

        let n = self.line_num();
        let rebuild_start = heading_line;
        let rebuild_end = content_range
            .end
            .max(heading_line.saturating_add(1))
            .min(n);
        self.request_rebuild_index_for_layout(rebuild_start, rebuild_end);
    }

    fn migrate_cursor_after_outline_fold(
        &mut self,
        heading_line: usize,
        folded: bool,
        content_range: &std::ops::Range<usize>,
    ) {
        if !folded {
            return;
        }
        let c2_line = self.state.cursor2.line_no;
        let c1_line = self.state.cursor1.line_no;
        let in_hidden = |ln: usize| content_range.contains(&ln);
        if in_hidden(c2_line) {
            let mut c = self.state.cursor2.clone();
            c.line_no = heading_line;
            c.segment = 0;
            c.culumn = 0;
            if let Some(p) = self.pgh_views.get(heading_line) {
                c.segment = p.last_text_segment();
                c.culumn = p.max_culumn(&c);
            }
            self.state.cursor2 = self.cursor_check(&c);
        }
        if in_hidden(c1_line) {
            self.state.cursor1 = self.state.cursor2.clone();
        }
    }

    /// 自 `from` 起向 `dir`（1=向下，-1=向上）找下一可见行。
    pub(crate) fn next_visible_line(&self, from: usize, dir: i8) -> usize {
        let n = self.line_num();
        if n == 0 {
            return 0;
        }
        let from = from.min(n.saturating_sub(1));
        let is_visible = |line: usize| {
            self.pgh_views
                .get(line)
                .map(|p| !p.is_render_hidden())
                .unwrap_or(true)
        };
        if dir > 0 {
            for line in (from + 1)..n {
                if is_visible(line) {
                    return line;
                }
            }
            for line in (0..=from).rev() {
                if is_visible(line) {
                    return line;
                }
            }
        } else {
            for line in (0..from).rev() {
                if is_visible(line) {
                    return line;
                }
            }
            for line in (from + 1)..n {
                if is_visible(line) {
                    return line;
                }
            }
        }
        from
    }
}
