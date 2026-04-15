use eframe::egui::Rect;

use crate::medit::PghView;

#[inline]
pub(crate) fn scroll_line_height_estimate(pgh_views: &[PghView], line: usize, font_h: f32) -> f32 {
    pgh_views
        .get(line)
        .map(|p| {
            if p.last_scroll_height > 0.0 {
                p.last_scroll_height
            } else {
                font_h
            }
        })
        .unwrap_or(font_h)
}

/// 1-based Fenwick 树：支持单点增量和前缀和查询。
struct FenwickTree {
    n: usize,
    bit: Vec<f32>,
}

impl FenwickTree {
    fn new(n: usize) -> Self {
        Self {
            n,
            bit: vec![0.0; n + 1],
        }
    }

    fn reset(&mut self, n: usize) {
        self.n = n;
        self.bit.clear();
        self.bit.resize(n + 1, 0.0);
    }

    fn add(&mut self, idx: usize, delta: f32) {
        let mut i = idx + 1;
        while i <= self.n {
            self.bit[i] += delta;
            i += i & (!i + 1);
        }
    }

    fn build_from(values: &[f32]) -> Self {
        let n = values.len();
        let mut fw = Self {
            n,
            bit: vec![0.0; n + 1],
        };
        // O(n) 建树：先写入本位，再把本位贡献推到父位
        for i in 1..=n {
            fw.bit[i] += values[i - 1];
            let j = i + (i & (!i + 1));
            if j <= n {
                fw.bit[j] += fw.bit[i];
            }
        }
        fw
    }

    /// 前缀和 `sum(values[0..len])`
    fn prefix_sum(&self, len: usize) -> f32 {
        let mut i = len.min(self.n);
        let mut sum = 0.0f32;
        while i > 0 {
            sum += self.bit[i];
            i &= i - 1;
        }
        sum
    }

    fn total_sum(&self) -> f32 {
        self.prefix_sum(self.n)
    }

    /// 最小 `k`（0..=n）使得 `prefix_sum(k) >= target`。
    /// 若 `target <= 0` 返回 0；若 `target > total_sum` 返回 n+1。
    fn lower_bound_prefix_ge(&self, target: f32) -> usize {
        if target <= 0.0 {
            return 0;
        }
        if target > self.total_sum() {
            return self.n + 1;
        }
        let mut idx = 0usize;
        let mut acc = 0.0f32;
        let mut bit = 1usize;
        while (bit << 1) <= self.n {
            bit <<= 1;
        }
        while bit > 0 {
            let next = idx + bit;
            if next <= self.n && acc + self.bit[next] < target {
                idx = next;
                acc += self.bit[next];
            }
            bit >>= 1;
        }
        idx + 1
    }

    /// 最小 `k`（1..=n）使得 `prefix_sum(k) > target`。
    /// 若 `target < 0` 返回 1；若 `target >= total_sum` 返回 n+1。
    fn lower_bound_prefix_gt(&self, target: f32) -> usize {
        if target < 0.0 {
            return 1;
        }
        if target >= self.total_sum() {
            return self.n + 1;
        }
        let mut idx = 0usize;
        let mut acc = 0.0f32;
        let mut bit = 1usize;
        while (bit << 1) <= self.n {
            bit <<= 1;
        }
        while bit > 0 {
            let next = idx + bit;
            if next <= self.n && acc + self.bit[next] <= target {
                idx = next;
                acc += self.bit[next];
            }
            bit >>= 1;
        }
        idx + 1
    }
}

/// 与 `ScrollArea::show_viewport` 配合：本帧 layout 行区间上界 + 高度树（Fenwick）
pub(crate) struct EditScrollLayout {
    /// 本帧在 `show_viewport` 中实际 layout 的结束行（开区间上界），由 layout 写入
    pub(crate) layout_patch_end: usize,
    /// 每行当前用于滚动估计的高度（与 Fenwick 同步）
    line_heights: Vec<f32>,
    fenwick: FenwickTree,
    /// `Some(k)` 表示需从第 k 行起重算后缀；`None` 表示与当前行数/字高/行高缓存一致
    cum_dirty_from: Option<usize>,
    /// 行数不变时的局部脏行集合（点更新）
    dirty_lines: Vec<usize>,
    last_line_count: usize,
    last_font_heigh: f32,
}

impl EditScrollLayout {
    pub(crate) fn new() -> Self {
        Self {
            layout_patch_end: 0,
            line_heights: Vec::new(),
            fenwick: FenwickTree::new(0),
            cum_dirty_from: Some(0),
            dirty_lines: Vec::new(),
            last_line_count: 0,
            last_font_heigh: 0.0,
        }
    }

    pub(crate) fn invalidate_cum_full(&mut self) {
        self.cum_dirty_from = Some(0);
        self.dirty_lines.clear();
    }

    /// 纯滚动且未改行高时 `cum_dirty_from == None`，直接返回 **O(1)**
    pub(crate) fn ensure_cumulative_offsets(
        &mut self,
        line_count: usize,
        font_heigh: f32,
        pgh_views: &[PghView],
    ) {
        let n = line_count;
        if n == 0 {
            self.line_heights.clear();
            self.fenwick.reset(0);
            self.cum_dirty_from = None;
            self.dirty_lines.clear();
            self.last_line_count = 0;
            return;
        }

        if self.line_heights.len() != n {
            self.line_heights.resize(n, font_heigh);
            self.cum_dirty_from = Some(0);
        }

        if self.last_line_count != n {
            self.cum_dirty_from = Some(0);
        } else if (self.last_font_heigh - font_heigh).abs() > 1e-3 {
            self.cum_dirty_from = Some(0);
        }
        self.last_line_count = n;
        self.last_font_heigh = font_heigh;

        if self.cum_dirty_from.is_none() && self.dirty_lines.is_empty() {
            return;
        }

        if self.cum_dirty_from.is_some() {
            for i in 0..n {
                self.line_heights[i] = scroll_line_height_estimate(pgh_views, i, font_heigh);
            }
            self.fenwick = FenwickTree::build_from(&self.line_heights);
            self.dirty_lines.clear();
        } else {
            for i in self.dirty_lines.drain(..) {
                if i >= n {
                    continue;
                }
                let h = scroll_line_height_estimate(pgh_views, i, font_heigh);
                let old = self.line_heights[i];
                if (h - old).abs() > 1e-3 {
                    self.line_heights[i] = h;
                    self.fenwick.add(i, h - old);
                }
            }
        }
        self.cum_dirty_from = None;
    }

    #[inline]
    pub(crate) fn cum_at(&self, i: usize) -> f32 {
        self.fenwick.prefix_sum(i.min(self.fenwick.n))
    }

    pub(crate) fn note_line_height_changed(&mut self, line: usize, line_count: usize) {
        if line_count == 0 {
            return;
        }
        let line = line.min(line_count.saturating_sub(1));
        if self.cum_dirty_from.is_some() {
            return;
        }
        if !self.dirty_lines.contains(&line) {
            self.dirty_lines.push(line);
        }
    }

    pub(crate) fn lines_visible_for_viewport(
        &self,
        viewport: &Rect,
        margin: f32,
        n: usize,
    ) -> (usize, usize) {
        if n == 0 {
            return (0, 0);
        }
        let v0 = viewport.min.y - margin;
        let v1 = viewport.max.y + margin;
        if v1 <= 0.0 {
            return (0, 1.min(n));
        }
        let total = self.fenwick.total_sum();
        if v0 >= total {
            let s = n.saturating_sub(1);
            return (s, n);
        }
        let start_k = self.fenwick.lower_bound_prefix_gt(v0.max(0.0));
        let mut start = start_k.saturating_sub(1).min(n.saturating_sub(1));
        if v0 <= 0.0 {
            start = 0;
        }
        let end_k = self.fenwick.lower_bound_prefix_ge(v1.max(0.0));
        let mut end = end_k.min(n);
        if end < start + 1 {
            end = (start + 1).min(n);
        }
        (start, end)
    }
}
