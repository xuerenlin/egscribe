use crate::medit::{Cursor, PghView};

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

#[derive(Clone)]
pub struct DoMngr {
    pub index: usize,
    pub do_list: Vec<(DoCmd,DoCmd)>,    //(undo,redo)
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
            index: 0,
            do_list: vec![]
        }
    }
}

