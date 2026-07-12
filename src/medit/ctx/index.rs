//! 分帧 `rebuild_index` 任务中各子缓存的统一生命周期（init / step / end）。

use std::any::Any;

use super::cache_code::CodeCache;
use super::cache_filter::FindFilterCache;
use super::cache_linepos::LinePosCache;
use super::cache_outline::TocCache;
use super::cache_table::TableCache;
use super::Ctx;
use eframe::egui::Rect;

/// 由 [`IndexCacheMgr`] 驱动的索引缓存刷新接口。
pub(crate) trait IndexCache {
    fn rebuild_index_init(&mut self, gen: u64);
    /// 从 `from` 起处理至少一行（表格：可能一次跳过整块 `TableRow`），返回下一扫描行号。
    fn rebuild_index_step(&mut self, ctx: &mut Ctx, from: usize) -> usize;
    fn rebuild_index_end(&mut self, ctx: &mut Ctx, gen: u64);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// `caches` 向量中的槽位：Table 先于 Toc（影响 `rebuild_index_end` 顺序）。
const TABLE_CACHE_INDEX: usize = 0;
const CODE_CACHE_INDEX: usize = 1;
const TOC_CACHE_INDEX: usize = 2;
const FIND_FILTER_CACHE_INDEX: usize = 3;
const SCROLL_LAYOUT_CACHE_INDEX: usize = 4;

/// 单个子缓存类型；每个 rebuild 任务只处理一种 cache。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IndexCacheKind {
    Table,
    Code,
    Toc,
    FindFilter,
    ScrollLayout,
}

impl IndexCacheKind {
    fn cache_index(self) -> usize {
        match self {
            IndexCacheKind::Table => TABLE_CACHE_INDEX,
            IndexCacheKind::Code => CODE_CACHE_INDEX,
            IndexCacheKind::Toc => TOC_CACHE_INDEX,
            IndexCacheKind::FindFilter => FIND_FILTER_CACHE_INDEX,
            IndexCacheKind::ScrollLayout => SCROLL_LAYOUT_CACHE_INDEX,
        }
    }

    /// 是否需要在全量扫描前先完成「可视区优先」阶段（Toc 为全文档结构，直接全量扫）。
    fn needs_visible_first_pass(self) -> bool {
        !matches!(self, IndexCacheKind::Toc)
    }

    fn default_frame_budget(self) -> usize {
        match self {
            IndexCacheKind::FindFilter => 2000,
            _ => 200,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RebuildPhase {
    VisibleFirst,
    FullFromHead,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RebuildReason {
    /// 布局后发现行高变化，需要刷新可视区及后续索引。
    LayoutHeightChanged,
    /// 首次布局时触发的初始化索引重建。
    FirstLayout,
    /// 内容结构（增删行等）发生变化后触发。
    ContentChanged,
    /// expanded ctx 内容更新后触发。
    ExpandedCtxUpdated,
    /// FindFilter 启用/变更/关闭后触发。
    FindFilterChanged,
}

impl RebuildReason {
    fn as_str(self) -> &'static str {
        match self {
            RebuildReason::LayoutHeightChanged => "layout_height_changed",
            RebuildReason::FirstLayout => "first_layout",
            RebuildReason::ContentChanged => "content_changed",
            RebuildReason::ExpandedCtxUpdated => "expanded_ctx_updated",
            RebuildReason::FindFilterChanged => "find_filter_changed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RebuildMode {
    /// 由后续 UI 帧 [`IndexCacheMgr::tick`] 分帧完成扫描。
    Deferred,
    /// 在本次 `request_rebuild` 内用单帧极大预算跑完可视区与全量扫描；须传入 `ctx: Some(&mut Ctx)`（通常用 `std::mem::take` 将管理器从 `Ctx` 上临时移出后调用）。
    Immediate,
}

#[derive(Clone, Debug)]
struct RebuildTask {
    /// 当前重建任务代数（每次入队递增）。
    gen: u64,
    /// 触发本次重建的原因。
    reason: RebuildReason,
    /// 本次任务处理的子缓存类型。
    kind: IndexCacheKind,
    /// 当前所处阶段：可视区优先或从头全量收尾。
    phase: RebuildPhase,
    /// 本帧可视区扫描的起始行（含）。
    visible_start: usize,
    /// 本帧可视区扫描的结束行（不含）。
    visible_end: usize,
    /// 单帧允许消耗的扫描预算（按“扫描行数”近似计量）。
    frame_budget: usize,
    /// 可视区阶段完成时记录的 tick（用于计算首屏完成耗时）。
    visible_done_tick: Option<u64>,
    /// 任务累计扫描行数（用于日志与性能观测）。
    total_scanned_lines: usize,
}

impl RebuildTask {
    fn new(
        gen: u64,
        reason: RebuildReason,
        kind: IndexCacheKind,
        visible_start: usize,
        visible_end: usize,
        frame_budget: usize,
    ) -> Self {
        Self {
            gen,
            reason,
            kind,
            phase: RebuildPhase::VisibleFirst,
            visible_start,
            visible_end,
            frame_budget,
            visible_done_tick: None,
            total_scanned_lines: 0,
        }
    }
}

struct ManagedIndexCache {
    /// 具体索引缓存实现（如 Table/Toc/LinePos）。
    cache: Box<dyn IndexCache>,
    /// 全量收尾阶段的扫描游标（下一次 full pass 的起始行）。
    full_cursor: usize,
    /// 可视区优先阶段的扫描游标（下一次 visible pass 的起始行）。
    visible_cursor: usize,
}

impl ManagedIndexCache {
    fn new(cache: Box<dyn IndexCache>) -> Self {
        Self {
            cache,
            full_cursor: 0,
            visible_cursor: 0,
        }
    }
}

pub struct IndexCacheMgr {
    caches: Vec<ManagedIndexCache>,
    task_gen: u64,
    /// 正在扫描中的任务（多种 cache 可并行推进，共享每帧预算）。
    running_tasks: Vec<RebuildTask>,
    pending_tasks: Vec<RebuildTask>,
    tick_seq: u64,
}

impl Default for IndexCacheMgr {
    fn default() -> Self {
        Self {
            caches: vec![
                ManagedIndexCache::new(Box::new(TableCache::new())),
                ManagedIndexCache::new(Box::new(CodeCache::new())),
                ManagedIndexCache::new(Box::new(TocCache::new())),
                ManagedIndexCache::new(Box::new(FindFilterCache::new())),
                ManagedIndexCache::new(Box::new(LinePosCache::new())),
            ],
            task_gen: 0,
            running_tasks: Vec::new(),
            pending_tasks: Vec::new(),
            tick_seq: 0,
        }
    }
}

impl IndexCacheMgr {
    pub(crate) fn table_cache(&self) -> &TableCache {
        self.caches[TABLE_CACHE_INDEX]
            .cache
            .as_any()
            .downcast_ref::<TableCache>()
            .expect("TABLE_CACHE_INDEX must be TableCache")
    }

    pub(crate) fn table_cache_mut(&mut self) -> &mut TableCache {
        self.caches[TABLE_CACHE_INDEX]
            .cache
            .as_any_mut()
            .downcast_mut::<TableCache>()
            .expect("TABLE_CACHE_INDEX must be TableCache")
    }

    pub(crate) fn toc_cache(&self) -> &TocCache {
        self.caches[TOC_CACHE_INDEX]
            .cache
            .as_any()
            .downcast_ref::<TocCache>()
            .expect("TOC_CACHE_INDEX must be TocCache")
    }

    pub(crate) fn code_cache(&self) -> &CodeCache {
        self.caches[CODE_CACHE_INDEX]
            .cache
            .as_any()
            .downcast_ref::<CodeCache>()
            .expect("CODE_CACHE_INDEX must be CodeCache")
    }

    pub(crate) fn code_cache_mut(&mut self) -> &mut CodeCache {
        self.caches[CODE_CACHE_INDEX]
            .cache
            .as_any_mut()
            .downcast_mut::<CodeCache>()
            .expect("CODE_CACHE_INDEX must be CodeCache")
    }

    pub(crate) fn find_filter_cache(&self) -> &FindFilterCache {
        self.caches[FIND_FILTER_CACHE_INDEX]
            .cache
            .as_any()
            .downcast_ref::<FindFilterCache>()
            .expect("FIND_FILTER_CACHE_INDEX must be FindFilterCache")
    }

    pub(crate) fn find_filter_cache_mut(&mut self) -> &mut FindFilterCache {
        self.caches[FIND_FILTER_CACHE_INDEX]
            .cache
            .as_any_mut()
            .downcast_mut::<FindFilterCache>()
            .expect("FIND_FILTER_CACHE_INDEX must be FindFilterCache")
    }

    pub(crate) fn scroll_layout_cache(&self) -> &LinePosCache {
        self.caches[SCROLL_LAYOUT_CACHE_INDEX]
            .cache
            .as_any()
            .downcast_ref::<LinePosCache>()
            .expect("SCROLL_LAYOUT_CACHE_INDEX must be LinePosCache")
    }

    pub(crate) fn scroll_layout_cache_mut(&mut self) -> &mut LinePosCache {
        self.caches[SCROLL_LAYOUT_CACHE_INDEX]
            .cache
            .as_any_mut()
            .downcast_mut::<LinePosCache>()
            .expect("SCROLL_LAYOUT_CACHE_INDEX must be LinePosCache")
    }

    pub(crate) fn scroll_layout_patch_end(&self) -> usize {
        self.scroll_layout_cache().layout_patch_end
    }

    pub(crate) fn set_scroll_layout_patch_end(&mut self, end: usize) {
        self.scroll_layout_cache_mut().layout_patch_end = end;
    }

    pub(crate) fn scroll_cum_at(&self, i: usize) -> f32 {
        self.scroll_layout_cache().cum_at(i)
    }

    pub(crate) fn scroll_lines_visible_for_viewport(
        &self,
        viewport: &Rect,
        margin: f32,
        n: usize,
    ) -> (usize, usize) {
        self.scroll_layout_cache()
            .lines_visible_for_viewport(viewport, margin, n)
    }

    /// 入队指定 cache 种类列表的重建任务。
    pub(crate) fn request_rebuild_kinds(
        &mut self,
        ctx: Option<&mut Ctx>,
        visible_start: usize,
        visible_end: usize,
        total_lines: usize,
        reason: RebuildReason,
        mode: RebuildMode,
        kinds: &[IndexCacheKind],
    ) {
        for &kind in kinds {
            self.enqueue_rebuild_task(
                visible_start,
                visible_end,
                total_lines,
                reason,
                kind,
                kind.default_frame_budget(),
            );
        }
        self.run_immediate_if_needed(ctx, visible_start, visible_end, mode);
    }

    pub(crate) fn find_filter_is_active(&self) -> bool {
        self.find_filter_cache().is_active()
    }

    pub(crate) fn find_filter_visible_line_count(&self) -> usize {
        self.find_filter_cache().visible_line_count()
    }

    fn run_immediate_if_needed(
        &mut self,
        ctx: Option<&mut Ctx>,
        visible_start: usize,
        visible_end: usize,
        mode: RebuildMode,
    ) {
        if matches!(mode, RebuildMode::Immediate) {
            if let Some(ctx) = ctx {
                self.run_all_pending_to_completion(ctx, visible_start, visible_end);
            } else {
                log::warn!(
                    "rebuild_index Immediate without ctx; leaving tasks for later tick"
                );
            }
        }
    }

    fn enqueue_rebuild_task(
        &mut self,
        visible_start: usize,
        visible_end: usize,
        total_lines: usize,
        reason: RebuildReason,
        kind: IndexCacheKind,
        frame_budget: usize,
    ) {
        let bounded_start = visible_start.min(total_lines);
        let bounded_end = visible_end.max(bounded_start.saturating_add(1)).min(total_lines);
        if kind == IndexCacheKind::FindFilter && self.cancel_running_kind(kind) {
            log::debug!(
                "rebuild_index supersede running FindFilter reason={}",
                reason.as_str()
            );
        }
        self.task_gen = self.task_gen.saturating_add(1);
        let task = RebuildTask::new(
            self.task_gen,
            reason,
            kind,
            bounded_start,
            bounded_end,
            frame_budget,
        );
        self.pending_tasks.retain(|t| t.kind != kind);
        self.pending_tasks.push(task);
        log::debug!(
            "rebuild_index enqueue gen={} kind={:?} reason={} visible=[{}, {}) pending={}",
            self.task_gen,
            kind,
            reason.as_str(),
            bounded_start,
            bounded_end,
            self.pending_tasks.len()
        );
        self.try_start_runnable_tasks();
    }

    fn is_kind_running(&self, kind: IndexCacheKind) -> bool {
        self.running_tasks.iter().any(|t| t.kind == kind)
    }

    /// 取消正在执行的指定 cache 任务（不调用 `rebuild_index_end`）。
    fn cancel_running_kind(&mut self, kind: IndexCacheKind) -> bool {
        let before = self.running_tasks.len();
        self.running_tasks.retain(|t| t.kind != kind);
        if self.running_tasks.len() < before {
            let idx = kind.cache_index();
            self.caches[idx].full_cursor = 0;
            self.caches[idx].visible_cursor = 0;
            true
        } else {
            false
        }
    }

    fn try_start_runnable_tasks(&mut self) {
        let mut i = 0;
        while i < self.pending_tasks.len() {
            if self.is_kind_running(self.pending_tasks[i].kind) {
                i += 1;
                continue;
            }
            let task = self.pending_tasks.remove(i);
            let idx = task.kind.cache_index();
            self.caches[idx].full_cursor = 0;
            self.caches[idx].visible_cursor = task.visible_start;
            self.caches[idx].cache.rebuild_index_init(task.gen);
            log::debug!(
                "rebuild_index start gen={} kind={:?} reason={} visible=[{}, {})",
                task.gen,
                task.kind,
                task.reason.as_str(),
                task.visible_start,
                task.visible_end
            );
            self.running_tasks.push(task);
        }
    }

    fn run_all_pending_to_completion(
        &mut self,
        ctx: &mut Ctx,
        visible_start: usize,
        visible_end: usize,
    ) {
        self.tick_seq = self.tick_seq.saturating_add(1);
        let total_lines = ctx.line_num();
        if total_lines == 0 {
            self.running_tasks.clear();
            self.pending_tasks.clear();
            return;
        }
        self.try_start_runnable_tasks();
        while self.has_pending_work() {
            self.advance_rebuild(ctx, visible_start, visible_end, total_lines, usize::MAX);
        }
    }

    pub fn has_pending_work(&self) -> bool {
        !self.running_tasks.is_empty() || !self.pending_tasks.is_empty()
    }

    pub fn has_active_find_filter_task(&self) -> bool {
        let is_find_filter = |t: &RebuildTask| {
            t.kind == IndexCacheKind::FindFilter && t.reason == RebuildReason::FindFilterChanged
        };
        self.running_tasks.iter().any(is_find_filter)
            || self.pending_tasks.iter().any(is_find_filter)
    }

    /// FindFilter 重建进度 `[0.0, 1.0]`；无活跃任务时返回 `None`。
    pub fn find_filter_task_progress(&self, total_lines: usize) -> Option<f32> {
        let is_find_filter = |t: &RebuildTask| {
            t.kind == IndexCacheKind::FindFilter && t.reason == RebuildReason::FindFilterChanged
        };
        let task = self
            .running_tasks
            .iter()
            .find(|t| is_find_filter(t))
            .or_else(|| self.pending_tasks.iter().find(|t| is_find_filter(t)))?;
        if total_lines == 0 {
            return Some(1.0);
        }
        if !self.running_tasks.iter().any(is_find_filter) {
            return Some(0.0);
        }
        let cache = &self.caches[IndexCacheKind::FindFilter.cache_index()];
        let visible_span = task
            .visible_end
            .saturating_sub(task.visible_start)
            .max(1);
        let total_work = visible_span.saturating_add(total_lines).max(1);
        let done = match task.phase {
            RebuildPhase::VisibleFirst => cache
                .visible_cursor
                .saturating_sub(task.visible_start)
                .min(visible_span),
            RebuildPhase::FullFromHead => visible_span.saturating_add(cache.full_cursor.min(total_lines)),
        };
        Some((done as f32 / total_work as f32).clamp(0.0, 1.0))
    }

    pub fn active_task_gen(&self) -> Option<u64> {
        self.running_tasks.first().map(|t| t.gen)
    }

    pub fn latest_gen(&self) -> u64 {
        self.task_gen
    }

    fn managed_cache_mut(&mut self, kind: IndexCacheKind) -> &mut ManagedIndexCache {
        &mut self.caches[kind.cache_index()]
    }

    fn align_visible_cursor(&mut self, kind: IndexCacheKind, vis_lo: usize) {
        if !kind.needs_visible_first_pass() {
            return;
        }
        let e = self.managed_cache_mut(kind);
        if e.visible_cursor < vis_lo {
            e.visible_cursor = vis_lo;
        }
    }

    fn finalize_rebuild_task(&mut self, ctx: &mut Ctx, task: RebuildTask) {
        let kind = task.kind;
        let gen = task.gen;
        log::info!(
            "rebuild_index finish gen={} kind={:?} reason={} scanned_total={}",
            gen,
            kind,
            task.reason.as_str(),
            task.total_scanned_lines
        );
        self.managed_cache_mut(kind).cache.rebuild_index_end(ctx, gen);
        self.running_tasks.retain(|t| t.gen != gen);
        self.try_start_runnable_tasks();
    }

    pub fn tick(&mut self, ctx: &mut Ctx, visible_start: usize, visible_end: usize) {
        self.tick_seq = self.tick_seq.saturating_add(1);
        let total_lines = ctx.line_num();
        if total_lines == 0 {
            self.running_tasks.clear();
            self.pending_tasks.clear();
            return;
        }
        self.try_start_runnable_tasks();
        let budget = self
            .running_tasks
            .iter()
            .map(|t| t.frame_budget)
            .max()
            .unwrap_or(200);
        self.advance_rebuild(ctx, visible_start, visible_end, total_lines, budget);
    }

    fn advance_rebuild(
        &mut self,
        ctx: &mut Ctx,
        visible_start: usize,
        visible_end: usize,
        total_lines: usize,
        mut budget: usize,
    ) {
        if self.running_tasks.is_empty() {
            return;
        }
        let phase_at_frame_begin: Vec<_> = self
            .running_tasks
            .iter()
            .map(|t| (t.kind, t.phase))
            .collect();
        let mut frame_scanned_lines = 0usize;
        while budget > 0 && !self.running_tasks.is_empty() {
            for task in self.running_tasks.iter_mut() {
                task.visible_start = visible_start.min(total_lines);
                task.visible_end = visible_end
                    .max(task.visible_start.saturating_add(1))
                    .min(total_lines);
            }

            let mut round_scanned = 0usize;
            let mut completed_gens = Vec::new();
            let task_count = self.running_tasks.len();
            for i in 0..task_count {
                let kind = self.running_tasks[i].kind;
                let vis_lo = self.running_tasks[i].visible_start;
                let vis_hi = self.running_tasks[i].visible_end;
                let phase = self.running_tasks[i].phase;
                self.align_visible_cursor(kind, vis_lo);

                match phase {
                    RebuildPhase::VisibleFirst => {
                        let visible_done = !kind.needs_visible_first_pass()
                            || self.caches[kind.cache_index()].visible_cursor >= vis_hi;
                        if visible_done {
                            self.running_tasks[i].visible_done_tick.get_or_insert(self.tick_seq);
                            self.running_tasks[i].phase = RebuildPhase::FullFromHead;
                            self.caches[kind.cache_index()].full_cursor = 0;
                            round_scanned = round_scanned.saturating_add(1);
                        } else {
                            let from = self.caches[kind.cache_index()].visible_cursor;
                            let next = self.caches[kind.cache_index()]
                                .cache
                                .rebuild_index_step(ctx, from);
                            let scanned = next.saturating_sub(from).max(1);
                            self.caches[kind.cache_index()].visible_cursor = next;
                            round_scanned = round_scanned.saturating_add(scanned);
                            self.running_tasks[i].total_scanned_lines = self.running_tasks[i]
                                .total_scanned_lines
                                .saturating_add(scanned);
                        }
                    }
                    RebuildPhase::FullFromHead => {
                        let idx = kind.cache_index();
                        if self.caches[idx].full_cursor >= total_lines {
                            completed_gens.push(self.running_tasks[i].gen);
                            continue;
                        }
                        let from = self.caches[idx].full_cursor;
                        let next = self.caches[idx].cache.rebuild_index_step(ctx, from);
                        let scanned = next.saturating_sub(from).max(1);
                        self.caches[idx].full_cursor = next;
                        round_scanned = round_scanned.saturating_add(scanned);
                        self.running_tasks[i].total_scanned_lines = self.running_tasks[i]
                            .total_scanned_lines
                            .saturating_add(scanned);
                    }
                }
            }

            for gen in completed_gens {
                if let Some(pos) = self.running_tasks.iter().position(|t| t.gen == gen) {
                    let task = self.running_tasks.remove(pos);
                    self.finalize_rebuild_task(ctx, task);
                }
            }

            if round_scanned == 0 {
                break;
            }
            budget = budget.saturating_sub(round_scanned.max(1));
            frame_scanned_lines = frame_scanned_lines.saturating_add(round_scanned);
        }

        if !self.running_tasks.is_empty() {
            log::debug!(
                "rebuild_index frame tick={} running={} pending={} scanned_lines={} phases_begin={:?}",
                self.tick_seq,
                self.running_tasks.len(),
                self.pending_tasks.len(),
                frame_scanned_lines,
                phase_at_frame_begin
            );
        }
    }
}
