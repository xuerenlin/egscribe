use crate::medit::{Cursor, PghView};
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Clone, Debug)]
pub struct DoLine {
    pub line: usize,
    pub pgh_view: Option<PghView>,
}

#[derive(Clone, Debug)]
pub enum DoItem {
    Insert(DoLine),
    Delete(DoLine),
    Update(DoLine),
}

#[derive(Clone, Debug)]
pub struct DoCmd {
    pub cursor: Cursor,
    pub items: Vec<DoItem>,
}

/// 包含 undo/redo 命令对和动作名称的结构体
#[derive(Clone, Debug)]
pub struct DoCommand {
    pub undo: DoCmd,
    pub redo: DoCmd,
    pub action_name: Option<String>,
}

#[derive(Clone)]
pub struct DoMngr {
    pub active: bool,
    pub index: usize,
    pub do_list: Vec<DoCommand>,
}

impl DoCmd {
    pub fn new() -> Self {
        DoCmd {
            cursor: 0.into(),
            items: vec![]
        }
    }

    pub fn push_insert(&mut self, line: usize, pgh_view: Option<PghView>) {
        let item = DoItem::Insert(DoLine{line, pgh_view});
        self.items.push(item);
    }

    pub fn push_delete(&mut self, line: usize) {
        let item = DoItem::Delete(DoLine{line, pgh_view:None});
        self.items.push(item);
    }

    pub fn push_update(&mut self, line: usize, pgh_view: Option<PghView>) {
        let item = DoItem::Update(DoLine{line, pgh_view});
        self.items.push(item);
    }

    pub fn set_cursor(&mut self, cursor: Cursor) {
        self.cursor = cursor;
    }

}

impl DoMngr {
    pub fn new() -> DoMngr {
        Self {
            active: true,
            index: 0,
            do_list: vec![]
        }
    }

    pub fn enable(&mut self) {
        self.active = true;
    }

    pub fn disable(&mut self) {
        self.active = false;
    }

    /// 获取最后一个命令的 action_name
    pub fn last_action_name(&self) -> Option<String> {
        if self.index == 0 {
            return None;
        }
        if let Some(last_cmd) = self.do_list.get(self.index - 1) {
            last_cmd.action_name.clone()
        } else {
            None
        }
    }

    /// 添加一个 undo/redo 命令对
    /// 如果 active 为 false，则不执行任何操作
    pub fn push(&mut self, undo: DoCmd, redo: DoCmd, action_name: Option<String>) -> bool {
        if !self.active {
            return false;
        }

        let index = self.index;
        self.do_list.insert(index, DoCommand {
            undo,
            redo,
            action_name,
        });
        let new_index = index + 1;
        self.index = new_index;
        self.do_list.truncate(new_index);
        true
    }

    /// 合并指定范围内的命令
    /// start_idx: 起始索引（包含）
    /// end_idx: 结束索引（不包含）
    /// action_name: 合并后的 action_name（如果为 None，则使用 None）
    fn merge_range(&mut self, start_idx: usize, end_idx: usize, action_name: Option<String>) {
        if !self.active {
            return;
        }

        if start_idx >= end_idx || end_idx > self.index {
            return;
        }

        let merge_count = end_idx - start_idx;
        if merge_count == 0 {
            return;
        }
        
        // 如果只有一个命令，但提供了 action_name，则更新该命令的 action_name
        if merge_count == 1 {
            if let Some(action_name) = action_name {
                if let Some(cmd) = self.do_list.get_mut(start_idx) {
                    cmd.action_name = Some(action_name);
                }
            }
            // 即使 action_name 是 None，也不需要合并，直接返回
            return;
        }

        // 合并 undo 和 redo 命令
        let mut merged_undo = DoCmd::new();
        let mut merged_redo = DoCmd::new();

        // 收集所有要合并的命令
        let mut commands_to_merge = Vec::new();
        for i in start_idx..end_idx {
            if let Some(cmd) = self.do_list.get(i) {
                commands_to_merge.push((cmd.undo.clone(), cmd.redo.clone()));
            }
        }

        // 合并 undo 命令
        if let Some((first_undo, _)) = commands_to_merge.first() {
            merged_undo.cursor = first_undo.cursor;
            for (undo, _) in &commands_to_merge {
                merged_undo.items.extend(undo.items.iter().cloned());
            }
        }

        // 合并 redo 命令（正向合并，最后一个命令的 cursor 作为最终）
        if let Some((_, last_redo)) = commands_to_merge.last() {
            merged_redo.cursor = last_redo.cursor;
            for (_, redo) in &commands_to_merge {
                merged_redo.items.extend(redo.items.iter().cloned());
            }
        }

        // 合并 action_name
        // 如果没有提供 action_name，直接使用 None
        let merged_action_name = action_name;

        // 删除旧的命令，插入合并后的命令
        self.do_list.drain(start_idx..end_idx);
        self.do_list.insert(start_idx, DoCommand {
            undo: merged_undo,
            redo: merged_redo,
            action_name: merged_action_name,
        });
        let new_index = start_idx + 1;
        self.index = new_index;
        self.do_list.truncate(new_index);
    }
}

/// Guard 用于自动合并作用域内的 redo 和 undo 命令
/// 当 guard 离开作用域时，会自动合并从创建时到当前的所有命令
pub struct MergeRedoAndUndoGuard {
    pub start_index: usize,
    pub do_mngr: Rc<RefCell<DoMngr>>,
    pub action_name: Option<String>,
}

impl Drop for MergeRedoAndUndoGuard {
    fn drop(&mut self) {
        let mut do_mngr = self.do_mngr.borrow_mut();
        let current_index = do_mngr.index;
        if current_index <= self.start_index {
            return;
        }

        // 合并后设置 action_name
        do_mngr.merge_range(self.start_index, current_index, self.action_name.clone());
    }
}


