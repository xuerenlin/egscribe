use std::any::Any;
use std::collections::{HashMap, HashSet};

use super::index::IndexCache;
use super::Ctx;
use crate::medit::{TableInfo, TableKey};

/// 表格元数据缓存：`TableKey` → [`TableInfo`]，及重建扫描时的 key 占用集。
#[derive(Clone, Debug)]
pub(crate) struct TableCache {
    store: HashMap<TableKey, TableInfo>,
    next_table_key: TableKey,
    used_table_keys: HashSet<TableKey>,
}

impl Default for TableCache {
    fn default() -> Self {
        Self::new()
    }
}

impl TableCache {
    pub(crate) fn new() -> Self {
        Self {
            store: HashMap::new(),
            next_table_key: 1,
            used_table_keys: HashSet::new(),
        }
    }

    pub(crate) fn alloc_table_key(&mut self) -> TableKey {
        let key = self.next_table_key;
        self.next_table_key = self.next_table_key.saturating_add(1);
        key
    }

    pub(crate) fn table_info_by_key(&self, table_key: TableKey) -> Option<&TableInfo> {
        self.store.get(&table_key)
    }

    pub(crate) fn table_info_by_key_mut(&mut self, table_key: TableKey) -> Option<&mut TableInfo> {
        self.store.get_mut(&table_key)
    }

    pub(crate) fn table_info_cloned_by_key(&self, table_key: TableKey) -> Option<TableInfo> {
        self.store.get(&table_key).cloned()
    }

    pub(crate) fn upsert_table_info(&mut self, table_key: TableKey, table_info: TableInfo) {
        self.store.insert(table_key, table_info);
    }

    pub(crate) fn retain_used_only(&mut self) {
        self.store.retain(|k, _| self.used_table_keys.contains(k));
    }

    pub(crate) fn table_infos_mut(
        &mut self,
    ) -> std::collections::hash_map::ValuesMut<'_, TableKey, TableInfo> {
        self.store.values_mut()
    }

    fn mark_table_key_used(&mut self, table_key: TableKey) {
        self.used_table_keys.insert(table_key);
    }
}

impl IndexCache for TableCache {
    fn rebuild_index_init(&mut self, _gen: u64) {
        self.used_table_keys.clear();
    }

    fn rebuild_index_step(&mut self, ctx: &mut Ctx, line_no: usize) -> usize {
        if line_no >= ctx.line_num() {
            return ctx.line_num();
        }
        if ctx
            .get_line(line_no)
            .map(|p| p.is_table_row())
            .unwrap_or(false)
        {
            if let Some((s, e2)) = ctx.table_row_block_range(line_no) {
                let n = e2.saturating_sub(s) + 1;
                let mut table_key = ctx.get_line(s).and_then(|p| p.table_key).unwrap_or(0);
                if table_key == 0 {
                    table_key = self.alloc_table_key();
                }
                let mut base = self.table_info_cloned_by_key(table_key).unwrap_or_default();
                base.row_index = 0;
                base.row_count = n;
                base.col_count = ctx.get_line(s).map(|p| p.pgh.len()).unwrap_or(base.col_count);
                base.head_line_no = s;
                base.frame_style = ctx.cfg().table_frame_style.clone();
                base.ensure_head_col_checked_len();
                self.upsert_table_info(table_key, base.clone());
                for ln in s..=e2 {
                    if let Some(p) = ctx.get_line_mut(ln) {
                        if p.is_table_row() {
                            p.table_key = Some(table_key);
                        }
                    }
                }
                if let Some(key) = ctx.get_line(line_no).and_then(|p| p.table_key) {
                    self.mark_table_key_used(key);
                }
                return e2.saturating_add(1);
            }
        }
        if let Some(p) = ctx.get_line_mut(line_no) {
            p.table_key = None;
        }
        line_no.saturating_add(1)
    }

    fn rebuild_index_end(&mut self, _ctx: &mut Ctx, _gen: u64) {
        self.retain_used_only();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
