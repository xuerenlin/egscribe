use std::any::Any;

use eframe::egui::Rect;

use super::index::IndexCache;
use super::Ctx;
use crate::medit::PghView;

#[inline]
pub(crate) fn line_scroll_height(pgh_views: &[PghView], line: usize, font_h: f32) -> f32 {
    pgh_views
        .get(line)
        .map(|p| {
            if p.is_render_hidden() {
                0.0
            } else if p.last_scroll_height > 0.0 {
                p.last_scroll_height
            } else {
                font_h
            }
        })
        .unwrap_or(font_h)
}

#[inline]
fn line_height_estimate(pgh_views: &[PghView], line: usize, font_h: f32) -> f32 {
    line_scroll_height(pgh_views, line, font_h)
}

pub(crate) struct LinePosCache {
    /// 当前已完成布局增量更新的末尾行索引（不含）。
    pub(crate) layout_patch_end: usize,
    /// 每一行对应的渲染高度缓存。
    line_heights: Vec<f32>,
    /// 行高前缀和，满足 `cumulative_offsets[k] == sum(line_heights[0..k])`。
    cumulative_offsets: Vec<f32>,
    /// 上次缓存构建时的总行数，用于判断是否需要重建。
    last_line_count: usize,
    /// 上次缓存构建时使用的字体高度基准值。
    last_font_heigh: f32,
}

impl LinePosCache {
    pub(crate) fn new() -> Self {
        Self {
            layout_patch_end: 0,
            line_heights: Vec::new(),
            cumulative_offsets: vec![0.0],
            last_line_count: 0,
            last_font_heigh: 0.0,
        }
    }

    #[inline]
    pub(crate) fn cum_at(&self, i: usize) -> f32 {
        self.cumulative_offsets
            .get(i.min(self.line_heights.len()))
            .copied()
            .unwrap_or(0.0)
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
        let total = self.cum_at(n);
        if v0 >= total {
            let s = n.saturating_sub(1);
            return (s, n);
        }

        let low = v0.max(0.0);
        let high = v1.max(0.0);
        let start = self
            .cumulative_offsets
            .partition_point(|&sum| sum <= low)
            .saturating_sub(1)
            .min(n.saturating_sub(1));
        let mut end = self
            .cumulative_offsets
            .partition_point(|&sum| sum < high)
            .min(n);
        if end < start + 1 {
            end = (start + 1).min(n);
        }
        (start, end)
    }
}

impl IndexCache for LinePosCache {
    fn rebuild_index_init(&mut self, _gen: u64) {}

    fn rebuild_index_step(&mut self, ctx: &mut Ctx, from: usize) -> usize {
        let n = ctx.line_num();
        if from >= n {
            return n;
        }
        let font_heigh = ctx.font_heigh();
        let font_delta = (self.last_font_heigh - font_heigh).abs();
        if font_delta > 1e-3 {
            log::info!(
                "linepos rebuild_index_step incremental font update: from={} n={} font_delta={:.4}",
                from,
                n,
                font_delta
            );
            // 字体高度变化也走分帧增量修正，不触发全量重建。
            self.last_font_heigh = font_heigh;
        }

        // 行数变化时不做一次性全量重建，只做结构扩缩，后续在分帧扫描中逐行修正。
        if self.line_heights.len() != n || self.cumulative_offsets.len() != n + 1 || self.last_line_count != n {
            let old_n = self.line_heights.len();
            if n > old_n {
                self.line_heights.resize(n, font_heigh);
                self.cumulative_offsets.resize(n + 1, 0.0);
                let mut acc = self.cumulative_offsets.get(old_n).copied().unwrap_or(0.0);
                for i in old_n..n {
                    let h = line_height_estimate(&ctx.pgh_views, i, font_heigh);
                    self.line_heights[i] = h;
                    acc += h;
                    self.cumulative_offsets[i + 1] = acc;
                }
            } else {
                self.line_heights.truncate(n);
                self.cumulative_offsets.truncate(n + 1);
            }
            self.last_line_count = n;
            self.last_font_heigh = font_heigh;
        }

        // 每步至少处理一屏，避免“每次只更新 1 行”导致刷新过慢。
        let batch = ctx.patch_num().max(1);
        let end = from.saturating_add(batch).min(n);
        if from == 0 {
            self.cumulative_offsets[0] = 0.0;
        }
        for i in from..end {
            let h = line_height_estimate(&ctx.pgh_views, i, font_heigh);
            self.line_heights[i] = h;
            // 分帧时按顺序修正当前块前缀和，避免整段 O(n) 回写。
            self.cumulative_offsets[i + 1] = self.cumulative_offsets[i] + h;
        }
        end
    }

    fn rebuild_index_end(&mut self, _ctx: &mut Ctx, _gen: u64) {}

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
