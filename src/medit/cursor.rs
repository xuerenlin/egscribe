
use crate::medit::{SegmentType, PghView};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct Cursor {
    pub line_no: usize,
    pub segment: usize,
    pub culumn: usize,
}

impl PartialOrd for Cursor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self == other {
            Some(std::cmp::Ordering::Equal)
        } else if self.line_no < other.line_no {
            Some(std::cmp::Ordering::Less)
        } else if self.line_no > other.line_no {
            Some(std::cmp::Ordering::Greater)
        } else if self.segment < other.segment {
            Some(std::cmp::Ordering::Less)
        } else if self.segment > other.segment {
            Some(std::cmp::Ordering::Greater)
        } else {
            self.culumn.partial_cmp(&other.culumn)
        }
    }
}

impl Ord for Cursor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self == other {
            std::cmp::Ordering::Equal
        } else if self.line_no < other.line_no {
            std::cmp::Ordering::Less
        } else if self.line_no > other.line_no {
            std::cmp::Ordering::Greater
        } else if self.segment < other.segment {
            std::cmp::Ordering::Less
        } else if self.segment > other.segment {
            std::cmp::Ordering::Greater
        } else {
            self.culumn.cmp(&other.culumn)
        }
    }
}

impl From<usize> for Cursor {
    fn from(line_no: usize) -> Self {
        Cursor {
            line_no,
            segment: 0,
            culumn: 0,
        }
    }
}

impl From<(usize, usize, usize)> for Cursor {
    fn from(x: (usize, usize, usize)) -> Self {
        Cursor {
            line_no: x.0,
            segment: x.1,
            culumn: x.2,
        }
    }
}

impl Cursor {
    pub fn line_no(&self) -> usize {
        self.line_no
    }

    pub fn cursor_move_prev(&self, pgh_view: &PghView) -> Cursor {
        let mut cursor = self.clone();

        //prev node is not text segment, skip over
        while cursor.segment > 0 && cursor.culumn == 0 && pgh_view.get_segment_type(cursor.segment - 1) != SegmentType::Text {
            cursor.segment -= 1;
            cursor.culumn = 0;
        }

        if cursor.culumn > 0 {
            cursor.culumn -= 1;
        } else if cursor.segment > 0 {
            cursor.segment -= 1;
            cursor.culumn = pgh_view.max_culumn(&cursor);
            if cursor.culumn > 0 && !pgh_view.is_code() {
                cursor.culumn -= 1;
            }
        } else if cursor.line_no > 0 {
            cursor.line_no -= 1;
            cursor.segment = usize::MAX;
            cursor.culumn = usize::MAX;
        }
        cursor
    }

    pub fn cursor_move_next(&self, pgh_view: &PghView) -> Cursor {
        let mut cursor = self.clone();
        cursor.culumn += 1;

        if (cursor.culumn == pgh_view.max_culumn(&cursor)
            && cursor.segment < pgh_view.max_segment()
            && pgh_view.get_segment_type(cursor.segment) != SegmentType::Text)
            || cursor.culumn > pgh_view.max_culumn(&cursor)
        {
            cursor.segment += 1;
            cursor.culumn = 0;
        }

        if cursor.segment > pgh_view.max_segment() {
            cursor.line_no += 1;
            cursor.segment = 0;
            cursor.culumn = 0;
        }

        cursor
    }

    pub fn cursor_move_up(&self) -> Cursor {
        let mut cursor = self.clone();
        if cursor.line_no > 0 {
            cursor.line_no -= 1;
        }
        cursor
    }

    pub fn cursor_move_down(&self) -> Cursor {
        let mut cursor = self.clone();
        cursor.line_no += 1;
        cursor
    }

    pub fn cursor_move_home(&self) -> Cursor {
        let mut cursor = self.clone();
        cursor.culumn = 0;
        cursor
    }

    pub fn cursor_move_end(&self) -> Cursor {
        let mut cursor = self.clone();
        cursor.culumn = usize::MAX;
        cursor
    }

    pub fn cursor_move_enter(&self) -> Cursor {
        let mut cursor = self.cursor_move_down();
        cursor.segment = 0;
        cursor.culumn = 0;
        cursor
    }
}
