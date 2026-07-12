use std::any::Any;

use super::index::IndexCache;
use super::Ctx;
use crate::medit::FindReplaceCtx;

pub(crate) struct FindFilterCache {
    enabled: bool,
    param: FindReplaceCtx,
    /// 过滤完成后仍显示的行数（不含 `hidden_by_find_filter` 的行）。
    visible_line_count: usize,
}

impl FindFilterCache {
    pub(crate) fn new() -> Self {
        Self {
            enabled: false,
            param: FindReplaceCtx::new(),
            visible_line_count: 0,
        }
    }

    pub(crate) fn visible_line_count(&self) -> usize {
        self.visible_line_count
    }

    pub(crate) fn set_filter(&mut self, param: FindReplaceCtx, enabled: bool) {
        self.param = param;
        self.enabled = enabled;
    }

    pub(crate) fn is_active(&self) -> bool {
        self.enabled && !self.param.find.is_empty()
    }

    fn should_filter(&self) -> bool {
        self.is_active()
    }
}

impl IndexCache for FindFilterCache {
    fn rebuild_index_init(&mut self, _gen: u64) {
        self.visible_line_count = 0;
    }

    fn rebuild_index_step(&mut self, ctx: &mut Ctx, from: usize) -> usize {
        let n = ctx.line_num();
        if from >= n {
            return n;
        }
        // 全文阶段从 0 重新累计，避免可视区优先阶段重复计数。
        if from == 0 {
            self.visible_line_count = 0;
        }
        let filter_active = self.should_filter();
        let batch = ctx.patch_num().max(1);
        let end = from.saturating_add(batch).min(n);
        for line_no in from..end {
            let text = ctx
                .pgh_views
                .get(line_no)
                .map(|p| p.get_search_text());
            if let Some(p) = ctx.pgh_views.get_mut(line_no) {
                let hidden = filter_active
                    && text
                        .as_ref()
                        .map_or(true, |t| !Ctx::line_matches_find(t, &self.param));
                p.set_find_filter_hidden(hidden);
                if !hidden {
                    self.visible_line_count = self.visible_line_count.saturating_add(1);
                }
            }
        }
        end
    }

    fn rebuild_index_end(&mut self, ctx: &mut Ctx, _gen: u64) {
        ctx.line_flash_all();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
