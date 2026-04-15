//! Markdown 标题 / 目录：从 [`Ctx`] 提取标题层级与多级编号。

use std::collections::HashMap;

use crate::medit::Ctx;

/// 目录缓存刷新间隔（秒）：[`Ctx::toc_ensure_updated`] 由 [`crate::medit::layout::Edit`] 按此周期调用。
pub const TOC_SCAN_INTERVAL_SECS: f64 = 3.0;

/// 挂在 [`Ctx`] 上的目录缓存（按文档、按编辑上下文保存，避免随侧栏 UI 重建而丢失）。
#[derive(Clone, Debug)]
pub struct TocCache {
    pub entries: Vec<TocEntry>,
    /// 按行号快速查找（与 `entries` 同步更新）
    pub by_line: HashMap<usize, TocEntry>,
    pub last_scan_secs: f64,
    /// 上次刷新目录缓存时的 `Ctx` 内容变更代数。
    pub last_build_content_tick: u64,
}

impl TocCache {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            by_line: HashMap::new(),
            last_scan_secs: -1000.0,
            last_build_content_tick: u64::MAX,
        }
    }

    pub fn replace_entries(&mut self, entries: Vec<TocEntry>) {
        self.by_line.clear();
        self.by_line.reserve(entries.len());
        for e in &entries {
            self.by_line.insert(e.line_no, e.clone());
        }
        self.entries = entries;
    }

    pub fn clear_all(&mut self) {
        self.entries.clear();
        self.by_line.clear();
        self.last_scan_secs = -1000.0;
        self.last_build_content_tick = u64::MAX;
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

fn parse_heading_level(line: &str) -> u8 {
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
    if n == 0 {
        return 1;
    }
    n
}

fn assign_section_numbers(raw: Vec<(usize, u8, String)>) -> Vec<TocEntry> {
    let mut counts = [0u32; 6];
    let mut out = Vec::with_capacity(raw.len());
    for (line_no, level, title) in raw {
        let l = level as usize;
        if l == 0 || l > 6 {
            continue;
        }
        counts[l - 1] += 1;
        for c in counts.iter_mut().skip(l) {
            *c = 0;
        }
        let section_number = counts[..l]
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(".");
        out.push(TocEntry {
            line_no,
            level,
            title,
            section_number,
        });
    }
    out
}

/// 从当前缓冲的段落视图提取全部标题并计算编号。
pub fn collect_headings_from_ctx(ctx: &Ctx) -> Vec<TocEntry> {
    let mut raw = Vec::new();
    for line_no in 0..ctx.line_num() {
        if !ctx.is_heading_line(line_no) {
            continue;
        }
        let full = ctx.get_line_text(line_no);
        let level = parse_heading_level(&full).clamp(1, 6);
        let title = Ctx::remove_heading_prefix(&full);
        raw.push((line_no, level, title));
    }
    assign_section_numbers(raw)
}

/// 与 UI 缓存同步的命名空间；实现见 [`collect_headings_from_ctx`]。
#[derive(Debug, Clone, Copy, Default)]
pub struct MarkdownOutline;

impl MarkdownOutline {
    pub fn collect_from_ctx(ctx: &Ctx) -> Vec<TocEntry> {
        collect_headings_from_ctx(ctx)
    }
}

/// 目录树节点（按标题层级 `level` 嵌套）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TocNode {
    pub entry: TocEntry,
    pub children: Vec<TocNode>,
}

fn parse_toc_node(entries: &[TocEntry], i: &mut usize) -> TocNode {
    let e = entries[*i].clone();
    *i += 1;
    let mut children = Vec::new();
    while *i < entries.len() && entries[*i].level > e.level {
        children.push(parse_toc_node(entries, i));
    }
    TocNode { entry: e, children }
}

/// 将扁平标题序列转为森林（与文档顺序、层级一致）。
pub fn toc_entries_to_forest(entries: &[TocEntry]) -> Vec<TocNode> {
    let mut i = 0;
    let mut roots = Vec::new();
    while i < entries.len() {
        roots.push(parse_toc_node(entries, &mut i));
    }
    roots
}
