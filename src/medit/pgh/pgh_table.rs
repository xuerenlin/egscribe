use core::f32;
use serde::{Serialize, Deserialize};
use eframe::egui::epaint::text::{LayoutJob, TextFormat};
use eframe::egui::{
    FontFamily, Grid, NumExt, Pos2, Rect, Response, Stroke, Ui, Vec2,
    vec2, CursorIcon, Sense,
};
use super::pgh_items::PghIndent;
use crate::medit::{Cursor, Ctx, DoCmd, PghText, TextSpacing};
use crate::uicom::{IconName, icon_button_builder};
use super::{LayoutResponse, PghType, PghView};

#[derive(Clone, Debug)]
pub struct TableCell {
    pub row: usize,
    pub col: usize,
    pub segment: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableFrameStyle {
    Full,      // 全边框
    Horizontal, // 仅横线
    None,       // 无边框
}

#[derive(Clone, Debug)]
pub struct TableInfo {
    pub row_count: usize,
    pub col_count: usize,
    /// 在整张逻辑表中的行下标（`PghType::TableRow` 每行一个 `PghView` 时使用；`Table` 可为 0）
    pub table_row_index: usize,
    /// 逻辑表总行数（`TableRow` 使用；`Table` 可与 `row_count` 一致）
    pub table_total_rows: usize,
    pub spacing_x: f32,
    pub spacing_y: f32,
    pub spacing_indent: f32,
    pub col_min_width: f32,
    pub frame_style: TableFrameStyle,
}

impl Default for TableInfo {
    fn default() -> Self {
        TableInfo {
            row_count: 0,
            col_count: 0,
            table_row_index: 0,
            table_total_rows: 0,
            spacing_x: 12.0,
            spacing_y: 12.0,
            spacing_indent: 16.0,
            col_min_width: 64.0,
            frame_style: TableFrameStyle::Full,
        }
    }
}

impl TableInfo {
    /// 行号列宽度等 UI 用的逻辑行数（整块表）
    pub fn logical_row_count_for_ui(&self) -> usize {
        if self.table_total_rows > 0 {
            self.table_total_rows
        } else {
            self.row_count.max(1)
        }
    }
}

impl PghView {
    pub fn new_table() -> Self {
        PghView::new(PghType::Table)
    }

    pub fn new_table_row() -> Self {
        PghView::new(PghType::TableRow)
    }

    pub fn is_table(&self) -> bool {
        self.pgh_type == PghType::Table
    }

    pub fn is_table_row(&self) -> bool {
        self.pgh_type == PghType::TableRow
    }

    pub fn is_table_like(&self) -> bool {
        self.is_table() || self.is_table_row()
    }
}

/// impl tables
impl PghView {
    pub fn table_segment_to_cell(&self, segment: usize) -> Option<TableCell> {
        if let Some(table_info) = &self.table_info {
            if self.is_table_row() {
                let col = segment.min(table_info.col_count.saturating_sub(1));
                Some(TableCell {
                    row: table_info.table_row_index,
                    col,
                    segment: col,
                })
            } else {
                Some(TableCell {
                    row: segment / table_info.col_count,
                    col: segment % table_info.col_count,
                    segment,
                })
            }
        } else {
            None
        }
    }

    pub fn table_cell_to_segment(&self, cell: &TableCell) -> usize {
        if let Some(table_info) = &self.table_info {
            if self.is_table_row() {
                cell.col
            } else {
                cell.row * table_info.col_count + cell.col
            }
        } else {
            0
        }
    }

    //return left-top,right-bottom
    pub fn table_range_to_cells(&self, s1: usize, s2: usize) -> Option<(TableCell, TableCell)> {
        if let Some(table_info) = &self.table_info {
            if self.is_table_row() {
                let col_a = s1.min(table_info.col_count.saturating_sub(1));
                let col_b = s2.min(table_info.col_count.saturating_sub(1));
                let col_min = col_a.min(col_b);
                let col_max = col_a.max(col_b);
                let r = table_info.table_row_index;
                Some((
                    TableCell {
                        row: r,
                        col: col_min,
                        segment: col_min,
                    },
                    TableCell {
                        row: r,
                        col: col_max,
                        segment: col_max,
                    },
                ))
            } else {
                let c1 = self.table_segment_to_cell(s1).unwrap();
                let c2 = self.table_segment_to_cell(s2).unwrap();
                let row_min = std::cmp::min(c1.row, c2.row);
                let row_max = std::cmp::max(c1.row, c2.row);
                let col_min = std::cmp::min(c1.col, c2.col);
                let col_max = std::cmp::max(c1.col, c2.col);
                Some((
                    TableCell {
                        row: row_min,
                        col: col_min,
                        segment: row_min * table_info.col_count + col_min,
                    },
                    TableCell {
                        row: row_max,
                        col: col_max,
                        segment: row_max * table_info.col_count + col_max,
                    },
                ))
            }
        } else {
            None
        }
    }

    pub fn table_range_rect(&self, s1: usize, s2: usize) -> Option<Rect> {
        if let Some((c1, c2)) = self.table_range_to_cells(s1, s2) {
            if let Some(rect1) = self.get_segment_rect(c1.segment) {
                if let Some(rect2) = self.get_segment_rect(c2.segment) {
                    return Some(Rect::from_two_pos(rect1.left_top(), rect2.right_bottom()));
                }
            }
        }
        None
    }

    pub fn table_is_empty_row(&self, row: usize) -> bool {
        if let Some(table_info) = &self.table_info {
            if self.is_table_row() {
                if row != table_info.table_row_index {
                    return false;
                }
                for col in 0..table_info.col_count {
                    if let Some(pgh_segment) = self.pgh.get(col) {
                        if pgh_segment.item.text().len() > 0 {
                            return false;
                        }
                    }
                }
                return true;
            }
            for col in 0..table_info.col_count {
                let segment = row * table_info.col_count + col;
                if let Some(pgh_segment) = self.pgh.get(segment) {
                    if pgh_segment.item.text().len() > 0 {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn table_is_empty_col(&self, col: usize) -> bool {
        if let Some(table_info) = &self.table_info {
            if self.is_table_row() {
                if let Some(pgh_segment) = self.pgh.get(col) {
                    return pgh_segment.item.text().is_empty();
                }
                return true;
            }
            for row in 0..table_info.row_count {
                let segment = row * table_info.col_count + col;
                if let Some(pgh_segment) = self.pgh.get(segment) {
                    if pgh_segment.item.text().len() > 0 {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn table_delete_row(&mut self, row: usize) {
        if let Some(table_info) = &mut self.table_info {
            for col in 0..table_info.col_count {
                let segment = row * table_info.col_count;
                self.pgh.remove(segment);
            }
            table_info.row_count -= 1;
        }
    }

    pub fn table_delete_col(&mut self, col: usize) {
        if let Some(table_info) = &mut self.table_info {
            for row in (0..table_info.row_count).rev() {
                let segment = row * table_info.col_count + col;
                self.pgh.remove(segment);
            }
            table_info.col_count -= 1;
        }
    }

    pub fn table_delete_empty_in_range(&mut self, s1: usize, s2: usize) {
        let mut empty_row = vec![];
        let mut empty_col = vec![];
        if let Some(table_info) = &self.table_info {
            if self.is_table_row() {
                // 整行/整列删除由 Ctx 在块上处理；此处不修改 segment 结构
                let _ = (s1, s2, table_info);
                return;
            }
            if let Some((min, max)) = self.table_range_to_cells(s1, s2) {
                if min.col == 0 && max.col + 1 == table_info.col_count {
                    for row in min.row..=max.row {
                        if self.table_is_empty_row(row) {
                            empty_row.push(row);
                        }
                    }
                }
                if min.row == 0 && max.row + 1 == table_info.row_count {
                    for col in min.col..=max.col {
                        if self.table_is_empty_col(col) {
                            empty_col.push(col);
                        }
                    }
                }
            }
            for row in empty_row.iter().rev() {
                self.table_delete_row(*row);
            }
            for col in empty_col.iter().rev() {
                self.table_delete_col(*col);
            }
        }
    }

    ///return: segments inserted
    pub fn table_insert_row(&mut self, row: usize) -> usize {
        let mut segment = 0;
        let mut col_count = 0;
        if let Some(table_info) = &mut self.table_info {
            segment = table_info.col_count * row;
            col_count = table_info.col_count;
            table_info.row_count += 1;
        }
        for i in 0..col_count {
            self.insert_text(segment, "".to_string(), None);
        }
        return col_count;
    }

    ///return: segments inserted
    pub fn table_insert_col(&mut self, col: usize) -> usize {
        let mut segments = vec![];
        if let Some(table_info) = &mut self.table_info {
            for row in (0..table_info.row_count).rev() {
                segments.push(table_info.col_count * row + col);
            }
            table_info.col_count += 1;
        }
        for i in &segments {
            self.insert_text(*i, "".to_string(), None);
        }
        return segments.len();
    }

    /// 在 `PghType::TableRow` 的当前行于列 `col` 前插入一空列
    pub fn table_row_insert_col(&mut self, col: usize) {
        if !self.is_table_row() {
            return;
        }
        self.insert_text(col, "".to_string(), None);
        if let Some(ti) = &mut self.table_info {
            ti.col_count += 1;
        }
    }

    /// 删除 `TableRow` 当前行的一列（`0..col_count`），至少保留一列。
    pub fn table_row_delete_col(&mut self, col: usize) {
        if !self.is_table_row() {
            return;
        }
        let Some(ti) = &mut self.table_info else {
            return;
        };
        if ti.col_count <= 1 || col >= ti.col_count || col >= self.pgh.len() {
            return;
        }
        self.pgh.remove(col);
        ti.col_count -= 1;
    }

    //return new segment after change
    pub fn table_merge(&mut self, segment: usize, change: &PghView) -> usize {
        let mut min_cell = TableCell {
            row: 0,
            col: 0,
            segment: 0,
        };
        let mut new_seg = segment;
        if let Some(table_info) = self.table_info.clone() {
            if let Some(change_info) = &change.table_info {
                min_cell = self.table_segment_to_cell(segment).unwrap();
                let max_cell = TableCell {
                    row: min_cell.row + change_info.row_count,
                    col: min_cell.col + change_info.col_count,
                    segment: 0,
                };
                for r in table_info.row_count..max_cell.row {
                    self.table_insert_row(table_info.row_count);
                }
                for c in table_info.col_count..max_cell.col {
                    self.table_insert_col(table_info.col_count);
                }
            }
        }

        if let Some(table_info) = self.table_info.clone() {
            new_seg = min_cell.row * table_info.col_count + min_cell.col;
            if let Some(change_info) = &change.table_info {
                for r in 0..change_info.row_count {
                    for c in 0..change_info.col_count {
                        let org_seg = r * change_info.col_count + c;
                        let org_txt = change.get_segment_text(org_seg);
                        let dst_seg =
                            (min_cell.row + r) * table_info.col_count + (min_cell.col + c);
                        self.update_segment_text(dst_seg, org_txt);
                    }
                }
            }
        }

        new_seg
    }

    pub fn table_head_job(ui: &Ui, ctx: &Ctx, text: &str) -> LayoutJob {
        let mut job: LayoutJob = LayoutJob::default();
        let mut format = TextFormat::default();
        format.font_id.size = ctx.font_size();
        format.font_id.family = FontFamily::Name("msyhb".into());
        format.color = ctx.cfg().text_color();
        job.append(text, 0.0, format);
        job
    }

    pub fn table_cell_job(ui: &Ui, ctx: &Ctx, text: &str) -> LayoutJob {
        let mut job: LayoutJob = LayoutJob::default();
        let mut format = TextFormat::default();
        format.font_id.size = ctx.font_size();
        format.font_id.family = ctx.cfg().font_family();
        format.color = ctx.cfg().text_color();
        job.append(text, 0.0, format);
        job
    }

    pub fn table_guess_text_width(ui: &Ui, ctx: &Ctx, row: usize, text: String) -> f32 {
        let min_width = 8.0;
        let job = if row == 0 {
            Self::table_head_job(ui, ctx, &text)
        } else {
            Self::table_cell_job(ui, ctx, &text)
        };
        ui.fonts_mut(|f| f.layout_job(job)).rect.width().at_least(min_width)
    }

    pub fn table_guess_width(&self, ui: &Ui, ctx: &Ctx) -> Vec<f32> {
        if let Some(table_info) = &self.table_info {
            const TABLE_WIDTH_SAMPLE_ROWS: usize = 15;
            let mut width_info = vec![];
            let mut max_width = ctx.edit_width();
            max_width -= table_info.spacing_indent;
            max_width -= 64.0; //left right buttons space
            if ctx.cfg().show_table_row_no {
                max_width -= Self::table_index_col_width(
                    table_info.logical_row_count_for_ui(),
                    table_info.col_min_width,
                );
                max_width -= table_info.spacing_x;
            }
            let sample_rows = table_info.row_count.min(TABLE_WIDTH_SAMPLE_ROWS);

            for c in 0..table_info.col_count {
                let mut c_width = 0.0;
                for r in 0..sample_rows {
                    let cell_i = r * table_info.col_count + c;
                    if let Some(pgh_segment) = self.pgh.get(cell_i) {
                        let text = pgh_segment.item.text();
                        let w = Self::table_guess_text_width(ui, ctx, r, text);
                        c_width = w.at_least(c_width);
                    }
                }
                if table_info.row_count > TABLE_WIDTH_SAMPLE_ROWS {
                    c_width = c_width.at_least(table_info.col_min_width);
                }
                width_info.push(c_width);
                if c != 0 {
                    max_width -= table_info.spacing_x;
                }
            }

            let total: f32 = width_info.iter().sum();
            let warp_total: f32 = width_info
                .iter()
                .filter(|w| **w > table_info.col_min_width)
                .sum();
            let keep_total: f32 = total - warp_total;
            let max_warp_width = max_width - keep_total;

            if total > max_width && max_width > 0.0 && max_warp_width > 0.0 {
                let new_info: Vec<f32> = width_info
                    .iter()
                    .map(|w| {
                        if *w <= table_info.col_min_width {
                            *w
                        } else {
                            (w / warp_total * max_warp_width).at_least(table_info.col_min_width)
                        }
                    })
                    .collect();
                width_info = new_info;
            }
            width_info
        } else {
            vec![]
        }
    }

    pub(crate) fn table_index_col_width(row_count: usize, col_min_width: f32) -> f32 {
        let digits = row_count.max(1).to_string().len() as f32;
        (digits * 7.0 + 8.0)
            .at_least(col_min_width * 0.5)
            .at_least(20.0)
    }

    /// `frame_start..=frame_end`：只绘制该区间内的行边框（与视口 + overscan 对齐，上下各扩一行避免接缝缺口）。
    fn table_draw_frame(
        ui: &mut Ui,
        ctx: &mut Ctx,
        table_info: &TableInfo,
        cell_rects: &[Vec<Rect>],
        frame_start: usize,
        frame_end: usize,
    ) {
        let row_count = cell_rects.len();
        if row_count == 0 {
            return;
        }
        let last = row_count.saturating_sub(1);
        let frame_start = frame_start.min(last);
        let frame_end = frame_end.min(last);
        if frame_start > frame_end {
            return;
        }

        match table_info.frame_style {
            TableFrameStyle::None => {
                // 无边框，不绘制任何内容
            }
            TableFrameStyle::Horizontal => {
                // 仅横线：绘制每一行的底部横线
                let weak_stroke = Stroke::new(0.5, ui.visuals().weak_text_color());
                let text_stroke = Stroke::new(1.0, ctx.cfg().text_color()); // 第一行使用粗体和字体颜色
                let painter = ui.painter();

                for r in frame_start..=frame_end {
                    if let Some(row) = cell_rects.get(r) {
                        if let (Some(first_cell), Some(_last_cell)) = (row.first(), row.last()) {
                            let left = first_cell.left() - table_info.spacing_x / 2.0;
                            let right = row[row.len() - 1].right() + table_info.spacing_x / 2.0;
                            let y = row[0].bottom() + table_info.spacing_y / 2.0;

                            // 逻辑首行：与 Full 边框、`layout_table_row_line` 的 `is_first_row` 一致
                            let is_logical_first_row =
                                table_info.table_row_index.saturating_add(r) == 0;
                            let stroke = if is_logical_first_row {
                                text_stroke
                            } else {
                                weak_stroke
                            };
                            painter.line_segment([Pos2::new(left, y), Pos2::new(right, y)], stroke);
                        }
                    }
                }
            }
            TableFrameStyle::Full => {
                // 全边框：与 Horizontal 相同用 `line_segment`；每个单元格只在右侧画竖线、在底部画横线。
                // 逻辑首行补顶边、首列补左边：`Table` 时 `table_row_index==0`，行下标即 `r`；
                // `TableRow` 时每格 `Grid` 只有一行（`r==0`），须用 `table_row_index==0` 判断首行
                //（与 `layout_table_row_line` 的 `is_first_row` 一致）。
                let stroke = Stroke::new(0.5, ui.visuals().weak_text_color());
                let painter = ui.painter();

                for r in frame_start..=frame_end {
                    if let Some(row) = cell_rects.get(r) {
                        let is_logical_first_row = table_info.table_row_index.saturating_add(r) == 0;
                        for (c, cell) in row.iter().enumerate() {
                            let rect = cell.expand2(Vec2 {
                                x: table_info.spacing_x / 2.0,
                                y: table_info.spacing_y / 2.0,
                            });
                            painter.line_segment(
                                [
                                    Pos2::new(rect.right(), rect.top()),
                                    Pos2::new(rect.right(), rect.bottom()),
                                ],
                                stroke,
                            );
                            painter.line_segment(
                                [
                                    Pos2::new(rect.left(), rect.bottom()),
                                    Pos2::new(rect.right(), rect.bottom()),
                                ],
                                stroke,
                            );
                            if is_logical_first_row {
                                painter.line_segment(
                                    [
                                        Pos2::new(rect.left(), rect.top()),
                                        Pos2::new(rect.right(), rect.top()),
                                    ],
                                    stroke,
                                );
                            }
                            if c == 0 {
                                painter.line_segment(
                                    [
                                        Pos2::new(rect.left(), rect.top()),
                                        Pos2::new(rect.left(), rect.bottom()),
                                    ],
                                    stroke,
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    fn table_reset_cursor(pgh: &PghView, ctx: &mut Ctx, row: usize, col: usize, cursor: &Cursor) {
        let mut new_cursor = *cursor;
        if let Some(info) = &pgh.table_info {
            new_cursor.segment = if pgh.is_table_row() {
                col
            } else {
                row * info.col_count + col
            };
            ctx.set_cursor2(new_cursor);
            ctx.set_cursor1_reset();
        }
    }

    fn table_draw_buttons(
        ui: &mut Ui,
        ctx: &mut Ctx,
        cursor: &Cursor,
        table_info: &TableInfo,
        cell_rects: &Vec<Vec<Rect>>,
    ) {
        let segment = cursor.segment;
        let is_row = ctx
            .get_line(cursor.line_no)
            .is_some_and(|p| p.is_table_row());
        let (r, c, rect_row_i) = if is_row {
            let ti = ctx
                .get_line(cursor.line_no)
                .and_then(|p| p.table_info.clone())
                .unwrap_or_default();
            let c = segment.min(ti.col_count.saturating_sub(1));
            (ti.table_row_index, c, 0usize)
        } else {
            let r = segment / table_info.col_count;
            let c = segment % table_info.col_count;
            (r, c, r)
        };
        let mut insert_col: Option<usize> = None;
        let mut insert_row: Option<usize> = None;

        // 列插入按钮仅在首行上方（整块表与 TableRow 一致）
        if r == 0 {
            if let Some(row) = cell_rects.get(rect_row_i) {
                if let Some(cell) = row.get(c) {
                    let size = icon_button_builder(ui)
                        .icon(IconName::icon_chevron_down)
                        .font_size(12.0)
                        .size();
                    let mut rect = cell.expand2(Vec2 {
                        x: table_info.spacing_x / 2.0,
                        y: table_info.spacing_y / 2.0,
                    });
                    rect.min.x -= size.x / 2.0;
                    rect.min.y -= size.y;
                    rect.max.x -= size.x / 2.0;

                    let id: String = format!("{}.left", segment);
                    if icon_button_builder(ui)
                        .icon(IconName::icon_chevron_down)
                        .pos(rect.left_top())
                        .id(id)
                        .font_size(12.0)
                        .build_inner()
                    {
                        insert_col = Some(c);
                    }
                    let id: String = format!("{}.right", segment);
                    if icon_button_builder(ui)
                        .icon(IconName::icon_chevron_down)
                        .pos(rect.right_top())
                        .id(id)
                        .font_size(12.0)
                        .build_inner()
                    {
                        insert_col = Some(c + 1);
                    }
                }
            }
        }

        //right buttons
        if let Some(row) = cell_rects.get(rect_row_i) {
            if let Some(cell) = row.get(table_info.col_count - 1) {
                let size = icon_button_builder(ui)
                    .icon(IconName::icon_chevron_left)
                    .font_size(12.0)
                    .size();
                let mut rect = cell.expand2(Vec2 {
                    x: table_info.spacing_x / 2.0,
                    y: table_info.spacing_y / 2.0,
                });
                rect.min.y -= size.y / 2.0 - 2.0;
                rect.max.y -= size.y / 2.0 - 2.0;
                rect.max.x += table_info.spacing_x / 4.0;

                let id: String = format!("{}.top", segment);
                if icon_button_builder(ui)
                    .icon(IconName::icon_chevron_left)
                    .pos(rect.right_top())
                    .id(id)
                    .font_size(12.0)
                    .build_inner() {
                    insert_row = Some(r);
                }
                let id: String = format!("{}.bottom", segment);
                if icon_button_builder(ui)
                    .icon(IconName::icon_chevron_left)
                    .pos(rect.right_bottom())
                    .id(id)
                    .font_size(12.0)
                    .build_inner() {
                    insert_row = Some(r+1);
                }
            }
        }

        //insert row/col
        if insert_col != None || insert_row != None {
            let mut undo_cmd = DoCmd::new();
            let mut redo_cmd = DoCmd::new();
            undo_cmd.set_cursor(ctx.cursor2());
            if is_row {
                if let Some((blk_start, blk_end)) = ctx.table_row_block_range(cursor.line_no) {
                    for ln in blk_start..=blk_end {
                        undo_cmd.push_update(ln, ctx.get_line_clone(ln));
                    }
                } else {
                    undo_cmd.push_update(cursor.line_no, ctx.get_line_clone(cursor.line_no));
                }
                if let Some(row) = insert_row {
                    ctx.table_row_block_insert_logical_row(cursor.line_no, row, c);
                }
                if let Some(col) = insert_col {
                    ctx.table_row_block_insert_col(col);
                }
                if let Some((blk_start, blk_end)) = ctx.table_row_block_range(cursor.line_no) {
                    for ln in blk_start..=blk_end {
                        redo_cmd.push_update(ln, ctx.get_line_clone(ln));
                    }
                } else {
                    redo_cmd.push_update(cursor.line_no, ctx.get_line_clone(cursor.line_no));
                }
                redo_cmd.set_cursor(ctx.cursor2());
                ctx.push_do(undo_cmd, redo_cmd);
            } else {
                undo_cmd.push_update(cursor.line_no, ctx.get_line_clone(cursor.line_no));
                if let Some(pgh) = ctx.get_line_mut(cursor.line_no) {
                    if let Some(row) = insert_row {
                        pgh.table_insert_row(row);
                    }
                    if let Some(col) = insert_col {
                        pgh.table_insert_col(col);
                        if let Some(info) = &pgh.table_info {
                            let mut new_cursor = *cursor;
                            new_cursor.segment = r * info.col_count + c;
                            ctx.set_cursor2(new_cursor);
                            ctx.set_cursor1_reset();
                        }
                    }
                }
                redo_cmd.push_update(cursor.line_no, ctx.get_line_clone(cursor.line_no));
                redo_cmd.set_cursor(ctx.cursor2());
                ctx.push_do(undo_cmd, redo_cmd);
            }
        }
    }

    /// 同步行高缓冲长度，并按裁剪区估计需要实际布局的行区间（含 overscan，且包含 `cursor_row`）。
    fn prepare_table_visible_rows(
        clip_rect: Rect,
        grid_top_y: f32,
        row_heights: &mut Vec<f32>,
        row_count: usize,
        default_row_h: f32,
        cursor_row: usize,
        overscan: usize,
    ) -> (usize, usize) {
        if row_count == 0 {
            return (0, 0);
        }

        if row_heights.len() != row_count {
            row_heights.resize(row_count, default_row_h);
        }

        let mut acc_y = grid_top_y;
        let mut visible_start = 0usize;
        for (i, h) in row_heights.iter().enumerate() {
            let next = acc_y + *h;
            if next >= clip_rect.top() {
                visible_start = i;
                break;
            }
            acc_y = next;
        }

        acc_y = grid_top_y;
        let mut visible_end = row_count.saturating_sub(1);
        for (i, h) in row_heights.iter().enumerate() {
            let next = acc_y + *h;
            if acc_y <= clip_rect.bottom() {
                visible_end = i;
            }
            if acc_y > clip_rect.bottom() {
                break;
            }
            acc_y = next;
        }
        if visible_end < visible_start {
            visible_end = visible_start;
        }

        let mut visible_start = visible_start.saturating_sub(overscan);
        let mut visible_end = (visible_end + overscan).min(row_count.saturating_sub(1));
        visible_start = visible_start.min(cursor_row);
        visible_end = visible_end.max(cursor_row).min(row_count.saturating_sub(1));
        (visible_start, visible_end)
    }

    /// 虚拟滚动：小表全量；大表按是否滚动扩大 overscan，减轻空白行。
    fn table_layout_overscan(row_count: usize, is_scrolling: bool) -> usize {
        if row_count <= 50 {
            row_count
        } else if is_scrolling {
            40
        } else {
            20
        }
    }

    /// 将 Grid 一行内各格的 `min.y` 统一到同一行带（索引列与 PghText 分配 rect 顶边可能不一致）。
    fn normalize_table_row_cell_rects(cells: &mut [Rect]) -> f32 {
        let row_top = cells.iter().map(|r| r.min.y).fold(f32::INFINITY, f32::min);
        let row_bottom = cells.iter().map(|r| r.max.y).fold(f32::NEG_INFINITY, f32::max);
        let h = (row_bottom - row_top).at_least(1.0);
        for c in cells.iter_mut() {
            c.min.y = row_top;
            c.max.y = row_bottom;
        }
        h
    }

    fn table_data_column_rects(show_indices: bool, row_cells: &[Rect]) -> Vec<Rect> {
        if show_indices {
            row_cells[1..].to_vec()
        } else {
            row_cells.to_vec()
        }
    }

    fn table_grid_row_common_metrics(ctx: &Ctx, table_info: &TableInfo) -> (bool, f32, f32, f32) {
        let show_indices = ctx.cfg().show_table_row_no;
        let index_col_width = Self::table_index_col_width(
            table_info.logical_row_count_for_ui(),
            table_info.col_min_width,
        );
        let max_col_width = ctx.edit_width();
        let default_row_h = (ctx.font_size() + table_info.spacing_y + 6.0).at_least(18.0);
        (show_indices, index_col_width, max_col_width, default_row_h)
    }

    /// 行号列用 `PghText::layout_paragraph` 时的 segment：非真实单元格，`update_view` 不写回 `pgh`。
    const TABLE_ROW_INDEX_VIEW_SEGMENT: usize = usize::MAX;

    /// `layout_index == false` 时仅占位（placeholder 行），不排版、不绘制行号。
    fn layout_table_grid_row_push_index_cell(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        table_info: &TableInfo,
        row: usize,
        index_col_width: f32,
        target_h: f32,
        row_cell_rects: &mut Vec<Rect>,
        layout_index: bool,
        response: &mut Response,
    ) {
        if !layout_index {
            let (rect, _) = ui.allocate_exact_size(vec2(index_col_width, target_h), Sense::hover());
            row_cell_rects.push(rect);
            return;
        }

        let index_text = if row == 0 {
            "#".to_string()
        } else {
            row.to_string()
        };
        let job = if row == 0 {
            Self::table_head_job(ui, ctx, &index_text)
        } else {
            Self::table_cell_job(ui, ctx, &index_text)
        };

        let mut item_rect = ui.cursor();
        item_rect.set_width(index_col_width);
        item_rect.set_height(target_h);
        let spacing = TextSpacing::text_spacing_in_rect(item_rect, index_col_width)
            .with_spacing_top_bottom(table_info.spacing_y / 2.0, table_info.spacing_y / 2.0)
            .with_need_expand(true)
            .with_once_allocate(true)
            .with_first_row_indentation(ui);
        let rsp = PghText::layout_paragraph(
            ui,
            ctx,
            line_no,
            Self::TABLE_ROW_INDEX_VIEW_SEGMENT,
            spacing,
            index_text,
            &Some(job),
        );
        row_cell_rects.push(rsp.rect);
        *response |= rsp;
    }

    fn layout_table_grid_row_finalize(
        show_indices: bool,
        mut row_cell_rects: Vec<Rect>,
    ) -> (Vec<Rect>, Vec<Rect>, f32) {
        let row_h = Self::normalize_table_row_cell_rects(&mut row_cell_rects);
        let data_cells = Self::table_data_column_rects(show_indices, &row_cell_rects);
        (row_cell_rects, data_cells, row_h)
    }

    /// 可见区一行：`PghText` 排版数据格（含空单元格占位）。调用方在本行 push 完后执行 `ui.end_row()`。
    /// `width_info` 须由 `table_guess_width` 在整张表上只算一次再传入。
    fn layout_table_grid_row(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        table_info: &TableInfo,
        row: usize,
        width_info: &[f32],
        row_heights: &mut Vec<f32>,
        response: &mut Response,
    ) -> (Vec<Rect>, Vec<Rect>, f32) {
        let (show_indices, index_col_width, max_col_width, default_row_h) =
            Self::table_grid_row_common_metrics(ctx, table_info);

        let mut row_cell_rects = vec![];
        if show_indices {
            let target_h = row_heights[row].max(default_row_h);
            Self::layout_table_grid_row_push_index_cell(
                ui,
                ctx,
                line_no,
                table_info,
                row,
                index_col_width,
                target_h,
                &mut row_cell_rects,
                true,
                response,
            );
        }

        for c in 0..table_info.col_count {
            let warp_width = *width_info.get(c).unwrap_or_else(|| &max_col_width);
            let cell_i = row * table_info.col_count + c;
            if let Some(pgh_segment) = ctx.get_line(line_no).and_then(|p| p.pgh.get(cell_i)) {
                let text = pgh_segment.item.text();
                let job = if row == 0 {
                    Self::table_head_job(ui, ctx, &text)
                } else {
                    Self::table_cell_job(ui, ctx, &text)
                };
                let mut item_rect = ui.cursor();
                item_rect.set_width(warp_width);
                let spacing = TextSpacing::text_spacing_in_rect(item_rect, warp_width)
                    .with_spacing_top_bottom(table_info.spacing_y / 2.0, table_info.spacing_y / 2.0)
                    .with_need_expand(true)
                    .with_once_allocate(true)
                    .with_first_row_indentation(ui);
                let rsp = PghText::layout_paragraph(
                    ui,
                    ctx,
                    line_no,
                    cell_i,
                    spacing,
                    text,
                    &Some(job),
                );
                row_cell_rects.push(rsp.rect);
                *response |= rsp;
            } else {
                let mut placeholder_rect = ui.cursor();
                placeholder_rect.set_width(warp_width);
                placeholder_rect.set_height(row_heights[row].max(default_row_h));
                let rsp = ui.allocate_rect(placeholder_rect, Sense::hover());
                row_cell_rects.push(rsp.rect);
            }
        }

        Self::layout_table_grid_row_finalize(show_indices, row_cell_rects)
    }

    /// 单行 `TableRow`：`segment` 即列下标，`row_heights` 长度须为 1。
    fn layout_table_grid_row_table_row(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        table_info: &TableInfo,
        width_info: &[f32],
        row_heights: &mut Vec<f32>,
        response: &mut Response,
    ) -> (Vec<Rect>, Vec<Rect>, f32) {
        let (show_indices, index_col_width, max_col_width, default_row_h) =
            Self::table_grid_row_common_metrics(ctx, table_info);
        let r_idx = table_info.table_row_index;
        let mut row_cell_rects = vec![];
        if show_indices {
            let rh0 = row_heights.get(0).copied().unwrap_or(default_row_h);
            let target_h = rh0.max(default_row_h);
            Self::layout_table_grid_row_push_index_cell(
                ui,
                ctx,
                line_no,
                table_info,
                r_idx,
                index_col_width,
                target_h,
                &mut row_cell_rects,
                true,
                response,
            );
        }

        for c in 0..table_info.col_count {
            let warp_width = *width_info.get(c).unwrap_or_else(|| &max_col_width);
            let cell_i = c;
            if let Some(pgh_segment) = ctx.get_line(line_no).and_then(|p| p.pgh.get(cell_i)) {
                let text = pgh_segment.item.text();
                let job = if r_idx == 0 {
                    Self::table_head_job(ui, ctx, &text)
                } else {
                    Self::table_cell_job(ui, ctx, &text)
                };
                let mut item_rect = ui.cursor();
                item_rect.set_width(warp_width);
                let spacing = TextSpacing::text_spacing_in_rect(item_rect, warp_width)
                    .with_spacing_top_bottom(table_info.spacing_y / 2.0, table_info.spacing_y / 2.0)
                    .with_need_expand(true)
                    .with_once_allocate(true)
                    .with_first_row_indentation(ui);
                let rsp = PghText::layout_paragraph(
                    ui,
                    ctx,
                    line_no,
                    cell_i,
                    spacing,
                    text,
                    &Some(job),
                );
                row_cell_rects.push(rsp.rect);
                *response |= rsp;
            } else {
                let mut placeholder_rect = ui.cursor();
                placeholder_rect.set_width(warp_width);
                let rh0 = row_heights.get(0).copied().unwrap_or(default_row_h);
                placeholder_rect.set_height(rh0.max(default_row_h));
                let rsp = ui.allocate_rect(placeholder_rect, Sense::hover());
                row_cell_rects.push(rsp.rect);
            }
        }

        Self::layout_table_grid_row_finalize(show_indices, row_cell_rects)
    }

    /// 虚拟化不可见行：仅按缓存行高占位，不跑 `PghText`。
    fn layout_table_grid_row_placeholder(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        table_info: &TableInfo,
        row: usize,
        width_info: &[f32],
        row_heights: &mut Vec<f32>,
        response: &mut Response,
    ) -> (Vec<Rect>, Vec<Rect>, f32) {
        let (show_indices, index_col_width, max_col_width, _default_row_h) =
            Self::table_grid_row_common_metrics(ctx, table_info);

        let mut row_cell_rects = vec![];
        if show_indices {
            let target_h = row_heights[row];
            Self::layout_table_grid_row_push_index_cell(
                ui,
                ctx,
                line_no,
                table_info,
                row,
                index_col_width,
                target_h,
                &mut row_cell_rects,
                false,
                response,
            );
        }

        for col in 0..table_info.col_count {
            let warp_width = *width_info.get(col).unwrap_or_else(|| &max_col_width);
            let mut placeholder_rect = ui.cursor();
            placeholder_rect.set_width(warp_width);
            placeholder_rect.set_height(row_heights[row]);
            let rsp = ui.allocate_rect(placeholder_rect, Sense::hover());
            row_cell_rects.push(rsp.rect);
        }

        Self::layout_table_grid_row_finalize(show_indices, row_cell_rects)
    }

    pub fn layout_table(ui: &mut Ui, ctx: &mut Ctx, line_no: usize) -> Response {
        let mut response = ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1;

        let width_info = {
            let Some(p) = ctx.get_line(line_no) else {
                return response;
            };
            p.table_guess_width(ui, ctx)
        };
        let max_col_width = ctx.edit_width();
        let Some(table_info) = ctx.get_line(line_no).and_then(|p| p.table_info.clone()) else {
            return response;
        };
        if table_info.row_count == 0 {
            return response;
        }

        let cursor = ctx.cursor2();
        let cursor_row = cursor.segment / table_info.col_count;

        let default_row_h = (ctx.font_size() + table_info.spacing_y + 6.0).at_least(18.0);
        let is_scrolling = ui.ctx().input(|i| {
            i.raw_scroll_delta.y.abs() > 0.0 || i.smooth_scroll_delta.y.abs() > 0.0
        });
        let overscan = Self::table_layout_overscan(table_info.row_count, is_scrolling);

        let mut row_heights = {
            let mut v = ctx
                .get_line(line_no)
                .map(|p| p.table_row_heights.borrow().clone())
                .unwrap_or_default();
            if v.len() < table_info.row_count {
                v.resize(table_info.row_count, default_row_h);
            } else {
                v.truncate(table_info.row_count);
            }
            v
        };

        let (visible_start, visible_end) = Self::prepare_table_visible_rows(
            ui.clip_rect(),
            ui.cursor().top(),
            &mut row_heights,
            table_info.row_count,
            default_row_h,
            cursor_row,
            overscan,
        );

        ui.horizontal(|ui| {
            PghIndent::layout_paragraph(ui, ctx, line_no, 0, ctx.cfg().indent_size);
            let table_id = format!("table_id_{}", line_no);

            let _grid = Grid::new(&table_id)
                .striped(table_info.frame_style == TableFrameStyle::None)
                .num_columns(
                    table_info.col_count + if ctx.cfg().show_table_row_no { 1 } else { 0 },
                )
                .min_col_width(0.0)
                .min_row_height(0.0)
                .max_col_width(max_col_width)
                .spacing(Vec2 {
                    x: table_info.spacing_x,
                    y: table_info.spacing_y,
                })
                .show(ui, |ui| {
                    let mut all_cell_rects = vec![];
                    let mut data_cell_rects = vec![];

                    for r in 0..table_info.row_count {
                        let render_row = r == 0
                            || r == cursor_row
                            || (r >= visible_start && r <= visible_end);

                        let (row_cells, data_cells, row_h) = if render_row {
                            Self::layout_table_grid_row(
                                ui,
                                ctx,
                                line_no,
                                &table_info,
                                r,
                                &width_info,
                                &mut row_heights,
                                &mut response,
                            )
                        } else {
                            Self::layout_table_grid_row_placeholder(
                                ui,
                                ctx,
                                line_no,
                                &table_info,
                                r,
                                &width_info,
                                &mut row_heights,
                                &mut response,
                            )
                        };
                        ui.end_row();

                        if render_row {
                            row_heights[r] = row_h.at_least(1.0);
                        }
                        all_cell_rects.push(row_cells);
                        data_cell_rects.push(data_cells);
                    }

                    // 视口行 + overscan，上下各扩一行，减少大表滚动时表格线断层
                    let frame_start = visible_start.saturating_sub(1);
                    let frame_end = (visible_end + 1).min(table_info.row_count.saturating_sub(1));
                    Self::table_draw_frame(
                        ui,
                        ctx,
                        &table_info,
                        &all_cell_rects,
                        frame_start,
                        frame_end,
                    );

                    if cursor.line_no == line_no && !ctx.is_selected() && !ctx.cfg().is_read_only {
                        Self::table_draw_buttons(ui, ctx, &cursor, &table_info, &data_cell_rects);
                    }
                });
        });

        if let Some(p) = ctx.get_line_mut(line_no) {
            *p.table_row_heights.borrow_mut() = row_heights;
        }

        response
    }

    pub fn layout_table_row(ui: &mut Ui, ctx: &mut Ctx, line_no: usize) -> Response {
        let mut response = ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1;

        let width_info = ctx.table_guess_width_for_table_row_block(line_no, ui);
        let max_col_width = ctx.edit_width();
        let Some(table_info) = ctx.get_line(line_no).and_then(|p| p.table_info.clone()) else {
            return response;
        };
        if table_info.col_count == 0 {
            return response;
        }

        let cursor = ctx.cursor2();
        let default_row_h = (ctx.font_size() + table_info.spacing_y + 6.0).at_least(18.0);

        let mut row_heights = {
            let mut v = ctx
                .get_line(line_no)
                .map(|p| p.table_row_heights.borrow().clone())
                .unwrap_or_default();
            if v.is_empty() {
                v.push(default_row_h);
            } else {
                v.truncate(1);
            }
            if v.is_empty() {
                v.push(default_row_h);
            }
            v
        };

        ui.horizontal(|ui| {
            PghIndent::layout_paragraph(ui, ctx, line_no, 0, ctx.cfg().indent_size);
            let table_id = format!(
                "table_row_id_{}_{}",
                line_no, table_info.table_row_index
            );

            // TableRow 每行一个 Grid（仅一行）
            let grid = Grid::new(&table_id)
                .striped(table_info.frame_style == TableFrameStyle::None)
                .num_columns(
                    table_info.col_count + if ctx.cfg().show_table_row_no { 1 } else { 0 },
                )
                .min_col_width(0.0)
                .min_row_height(0.0)
                .max_col_width(max_col_width)
                .spacing(Vec2 {
                    x: table_info.spacing_x,
                    y: table_info.spacing_y,
                })
                .show(ui, |ui| {
                    let mut all_cell_rects = vec![];
                    let mut data_cell_rects = vec![];

                    let (row_cells, data_cells, row_h) = Self::layout_table_grid_row_table_row(
                        ui,
                        ctx,
                        line_no,
                        &table_info,
                        &width_info,
                        &mut row_heights,
                        &mut response,
                    );
                    ui.end_row();
                    row_heights[0] = row_h.at_least(1.0);
                    all_cell_rects.push(row_cells);
                    data_cell_rects.push(data_cells);

                    Self::table_draw_frame(ui, ctx, &table_info, &all_cell_rects, 0, 0);

                    if cursor.line_no == line_no && !ctx.is_selected() && !ctx.cfg().is_read_only {
                        Self::table_draw_buttons(ui, ctx, &cursor, &table_info, &data_cell_rects);
                    }

                    //插入一个空行，这样Grid才会插入table_info.spacing_y的间隙
                    ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense());
                    ui.end_row();
                });
        });

        

        if let Some(p) = ctx.get_line_mut(line_no) {
            *p.table_row_heights.borrow_mut() = row_heights;
        }

        response
    }

    pub fn layout_table_row_line(ui: &mut Ui, ctx: &mut Ctx, line_no: usize) -> LayoutResponse {
        let mut response = ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1;
        let handled = false;
        let Some(table_info) = ctx.get_line(line_no).and_then(|p| p.table_info.clone()) else {
            return LayoutResponse::new(response.on_hover_cursor(CursorIcon::Text), handled);
        };
        {
            let size = icon_button_builder(ui)
                .icon(IconName::icon_chevron_down)
                .font_size(12.0)
                .size();
            let total_rows = table_info.table_total_rows.max(1);
            let is_first_row = table_info.table_row_index == 0;
            let is_last_row = table_info.table_row_index + 1 >= total_rows;
            let top_band = if is_first_row { size.y } else { 0.0 };
            let bottom_band = if is_last_row { size.y } else { 0.0 };

            if top_band > 0.0 {
                ui.allocate_exact_size(vec2(0.0, top_band), ctx.sense()).1;
            }

            ctx.update_spacing(
                line_no,
                table_info.spacing_y / 2.0,
                table_info.spacing_y / 2.0,
            );

            ui.horizontal(|ui| {
                ui.allocate_exact_size(vec2(table_info.spacing_indent, 0.0), ctx.sense());
                let rsp = Self::layout_table_row(ui, ctx, line_no);
                let table_rect = rsp.rect.expand2(Vec2 {
                    x: table_info.spacing_x / 2.0,
                    y: table_info.spacing_y / 2.0,
                });
                let icon_pad_x = if is_first_row { size.x } else { 0.0 };
                let table_rect = table_rect.expand2(Vec2 {
                    x: icon_pad_x,
                    y: 0.0,
                });
                response |= rsp;

                if table_rect.right() < ctx.edit_right() {
                    let mut right_rect = table_rect;
                    right_rect.set_left(table_rect.right());
                    right_rect.set_right(ctx.edit_right());
                    response |= ui.allocate_rect(right_rect, ctx.sense());
                }
            });

            if bottom_band > 0.0 {
                let mut bottom_rect = ui.cursor();
                bottom_rect.set_right(ctx.edit_right());
                bottom_rect.set_height(bottom_band);
                response |= ui.allocate_rect(bottom_rect, ctx.sense());
            }
        }

        let response = response.on_hover_cursor(CursorIcon::Text);
        LayoutResponse::new(response, handled)
    }

    pub fn layout_table_line(ui: &mut Ui, ctx: &mut Ctx, line_no: usize) -> LayoutResponse {
        let mut response = ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1;
        let handled = false;
        let Some(table_info) = ctx.get_line(line_no).and_then(|p| p.table_info.clone()) else {
            return LayoutResponse::new(response.on_hover_cursor(CursorIcon::Text), handled);
        };
        {
            //top space
            let size = icon_button_builder(ui)
                .icon(IconName::icon_chevron_down)
                .font_size(12.0)
                .size();
            ui.allocate_exact_size(vec2(0.0, size.y), ctx.sense()).1;

            ctx.update_spacing(
                line_no,
                table_info.spacing_y / 2.0,
                table_info.spacing_y / 2.0,
            );

            ui.horizontal(|ui| {
                ui.allocate_exact_size(vec2(table_info.spacing_indent, 0.0), ctx.sense());
                let rsp = Self::layout_table(ui, ctx, line_no);
                
                //table rect inclue frame and icon_space_x
                let table_rect = rsp.rect.expand2(Vec2 { x: table_info.spacing_x/2.0, y: table_info.spacing_y/2.0});    //include frame
                let table_rect = table_rect.expand2(Vec2 { x: size.x, y: 0.0 });  //include icon_space_x
                //ui.painter().rect_stroke(table_rect, 0.0, Stroke::new(0.5, Color32::RED), StrokeKind::Outside);
                response |= rsp;

                //add right space
                if table_rect.right() < ctx.edit_right() {
                    let mut right_rect = table_rect; 
                    right_rect.set_left(table_rect.right());
                    right_rect.set_right(ctx.edit_right());
                    response |= ui.allocate_rect(right_rect, ctx.sense());
                }
            });

            //bottom space
            let mut bottom_rect = ui.cursor();
            bottom_rect.set_right(ctx.edit_right());
            bottom_rect.set_height(size.y);
            response |= ui.allocate_rect(bottom_rect, ctx.sense());
            //response |= ui.allocate_exact_size(vec2(0.0, size.y), ctx.sense()).1;
        }

        let response = response.on_hover_cursor(CursorIcon::Text);
        LayoutResponse::new(response, handled)
    }
}
