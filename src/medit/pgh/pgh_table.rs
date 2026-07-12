use core::f32;
use serde::{Serialize, Deserialize};
use eframe::egui::epaint::text::{LayoutJob, TextFormat};
use eframe::egui::{
    Align, FontFamily, Grid, Layout, NumExt, Pos2, Rect, Response, Stroke, StrokeKind, Ui, Vec2,
    vec2, CursorIcon, Sense,
};
use super::pgh_items::PghIndent;
use crate::medit::{Cursor, Ctx, DoCmd, PghText, TextSpacing};
use crate::uicom::{CONTROL_HIGHLIGHT, IconName, icon_button_builder};
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
    pub col_count: usize,
    pub row_count: usize,
    pub row_index: usize,
    pub head_line_no: usize,
    pub spacing_x: f32,
    pub spacing_y: f32,
    pub spacing_indent: f32,
    pub col_min_width: f32,
    pub frame_style: TableFrameStyle,
    /// 表头（逻辑第 0 行）各数据列 checkbox 勾选状态
    pub head_col_checked: Vec<bool>,
    /// 表头行号列 checkbox 勾选状态（仅在显示行号列时可见）
    pub head_index_checked: bool,
}

impl Default for TableInfo {
    fn default() -> Self {
        TableInfo {
            col_count: 0,
            row_index: 0,
            row_count: 0,
            head_line_no: 0,
            spacing_x: 12.0,
            spacing_y: 12.0,
            spacing_indent: 16.0,
            col_min_width: 64.0,
            frame_style: TableFrameStyle::Full,
            head_col_checked: vec![],
            head_index_checked: false,
        }
    }
}

impl TableInfo {
    /// 行号列宽度等 UI 用的逻辑行数（整块表）
    pub fn logical_row_count_for_ui(&self) -> usize {
        if self.row_count > 0 {
            self.row_count
        } else {
            1
        }
    }

    pub fn ensure_head_col_checked_len(&mut self) {
        if self.head_col_checked.len() < self.col_count {
            self.head_col_checked.resize(self.col_count, false);
        } else if self.head_col_checked.len() > self.col_count {
            self.head_col_checked.truncate(self.col_count);
        }
    }

}

impl PghView {
    const TABLE_HEAD_CHECKBOX_GAP_X: f32 = 8.0;

    pub fn new_table_row() -> Self {
        PghView::new(PghType::TableRow)
    }

    pub fn is_table_row(&self) -> bool {
        self.pgh_type == PghType::TableRow
    }

    pub fn is_table_like(&self) -> bool {
        self.is_table_row()
    }

    fn table_col_count_local(&self) -> usize {
        self.pgh.len().max(1)
    }
}

/// impl tables
impl PghView {
    pub fn table_segment_to_cell(&self, segment: usize) -> Option<TableCell> {
        if self.is_table_row() {
            let col_count = self.table_col_count_local();
            let col = segment.min(col_count.saturating_sub(1));
            Some(TableCell {
                row: 0,
                col,
                segment: col,
            })
        } else {
            None
        }
    }

    pub fn table_cell_to_segment(&self, cell: &TableCell) -> usize {
        if self.is_table_row() {
            cell.col
        } else {
            0
        }
    }

    //return left-top,right-bottom
    pub fn table_range_to_cells(&self, s1: usize, s2: usize) -> Option<(TableCell, TableCell)> {
        if self.is_table_row() {
            let col_count = self.table_col_count_local();
            let col_a = s1.min(col_count.saturating_sub(1));
            let col_b = s2.min(col_count.saturating_sub(1));
            let col_min = col_a.min(col_b);
            let col_max = col_a.max(col_b);
            Some((
                TableCell {
                    row: 0,
                    col: col_min,
                    segment: col_min,
                },
                TableCell {
                    row: 0,
                    col: col_max,
                    segment: col_max,
                },
            ))
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
        if !self.is_table_row() {
            return true;
        }
        if row != 0 {
            return false;
        }
        for col in 0..self.table_col_count_local() {
            if let Some(pgh_segment) = self.pgh.get(col) {
                if pgh_segment.item.text().len() > 0 {
                    return false;
                }
            }
        }
        true
    }

    pub fn table_is_empty_col(&self, col: usize) -> bool {
        if self.is_table_row() {
            if let Some(pgh_segment) = self.pgh.get(col) {
                return pgh_segment.item.text().is_empty();
            }
            return true;
        }
        true
    }

    pub fn table_delete_row(&mut self, row: usize) {
        if !self.is_table_row() || row > 0 {
            return;
        }
        self.pgh.clear();
    }

    pub fn table_delete_col(&mut self, col: usize) {
        if !self.is_table_row() {
            return;
        }
        if col < self.pgh.len() {
            self.pgh.remove(col);
        }
    }

    ///return: segments inserted
    pub fn table_insert_row(&mut self, row: usize) -> usize {
        if !self.is_table_row() || row != 0 {
            return 0;
        }
        let col_count = self.table_col_count_local();
        for _ in 0..col_count {
            self.push_text(String::new(), None);
        }
        col_count
    }

    ///return: segments inserted
    pub fn table_insert_col(&mut self, col: usize) -> usize {
        if !self.is_table_row() {
            return 0;
        }
        self.insert_text(col.min(self.pgh.len()), "".to_string(), None);
        1
    }

    /// 在 `PghType::TableRow` 的当前行于列 `col` 前插入一空列
    pub fn table_row_insert_col(&mut self, col: usize) {
        if !self.is_table_row() {
            return;
        }
        self.insert_text(col, "".to_string(), None);
    }

    /// 删除 `TableRow` 当前行的一列（`0..col_count`），至少保留一列。
    pub fn table_row_delete_col(&mut self, col: usize) {
        if !self.is_table_row() {
            return;
        }
        let col_count = self.table_col_count_local();
        if col_count <= 1 || col >= col_count || col >= self.pgh.len() {
            return;
        }
        self.pgh.remove(col);
    }

    //return new segment after change
    pub fn table_merge(&mut self, segment: usize, change: &PghView) -> usize {
        let new_seg = segment.min(self.pgh.len());
        for i in 0..change.pgh.len() {
            let txt = change.get_segment_text(i);
            if new_seg + i < self.pgh.len() {
                self.update_segment_text(new_seg + i, txt);
            } else {
                self.insert_text(new_seg + i, txt, None);
            }
        }
        new_seg
    }

    pub fn table_head_job(_ui: &Ui, ctx: &Ctx, text: &str) -> LayoutJob {
        let mut job: LayoutJob = LayoutJob::default();
        let mut format = TextFormat::default();
        format.font_id.size = ctx.font_size();
        format.font_id.family = FontFamily::Name("msyhb".into());
        format.color = ctx.cfg().text_color();
        job.append(text, 0.0, format);
        job
    }

    pub fn table_cell_job(_ui: &Ui, ctx: &Ctx, text: &str) -> LayoutJob {
        let mut job: LayoutJob = LayoutJob::default();
        let mut format = TextFormat::default();
        format.font_id.size = ctx.font_size();
        format.font_id.family = ctx.cfg().font_family();
        format.color = ctx.cfg().text_color();
        job.append(text, 0.0, format);
        job
    }

    fn table_head_checkbox_size(ctx: &Ctx) -> Vec2 {
        let row_height = ctx.font_heigh();
        vec2(row_height * 0.7, row_height * 0.7)
    }

    pub(crate) fn table_head_checkbox_total_width(ctx: &Ctx) -> f32 {
        Self::table_head_checkbox_size(ctx).x + Self::TABLE_HEAD_CHECKBOX_GAP_X
    }

    fn table_layout_head_checkbox(
        ui: &mut Ui,
        ctx: &Ctx,
        checked: bool,
        interactive: bool,
    ) -> (bool, Response) {
        let box_size = Self::table_head_checkbox_size(ctx);
        let sense = if interactive { Sense::click() } else { Sense::hover() };
        let (rect, mut response) = ui.allocate_exact_size(box_size, sense);
        let mut new_checked = checked;
        if interactive && response.clicked() {
            new_checked = !new_checked;
            response.mark_changed();
        }

        let stroke_color = CONTROL_HIGHLIGHT;
        let rounding = 1.0;
        let rect_inner = rect.shrink(1.0);
        ui.painter().rect_stroke(
            rect_inner,
            rounding,
            Stroke::new(1.0, stroke_color),
            StrokeKind::Outside,
        );

        if new_checked {
            let w = rect_inner.width();
            let h = rect_inner.height();
            let p1 = Pos2::new(rect_inner.left() + w * 0.2, rect_inner.center().y);
            let p2 = Pos2::new(rect_inner.left() + w * 0.45, rect_inner.bottom() - h * 0.2);
            let p3 = Pos2::new(rect_inner.right() - w * 0.2, rect_inner.top() + h * 0.2);
            ui.painter()
                .line_segment([p1, p2], Stroke::new(2.8, stroke_color));
            ui.painter()
                .line_segment([p2, p3], Stroke::new(2.8, stroke_color));
        }

        response |= ui
            .allocate_exact_size(
                vec2(Self::TABLE_HEAD_CHECKBOX_GAP_X, response.rect.height()),
                Sense::hover(),
            )
            .1;
        (new_checked, response)
    }

    pub fn table_guess_text_width(ui: &Ui, ctx: &Ctx, row: usize, text: String) -> f32 {
        let min_width = 8.0;
        let job = if row == 0 {
            Self::table_head_job(ui, ctx, &text)
        } else {
            Self::table_cell_job(ui, ctx, &text)
        };
        let mut width = ui.fonts_mut(|f| f.layout_job(job)).rect.width().at_least(min_width);
        if row == 0 && ctx.cfg().show_table_head_checkbox {
            width += Self::table_head_checkbox_total_width(ctx);
        }
        width
    }

    pub fn table_guess_width(&self, ui: &Ui, ctx: &Ctx) -> Vec<f32> {
        if !self.is_table_row() {
            return vec![];
        }
        let table_info = TableInfo::default();
        let mut width_info = vec![];
        for c in 0..self.table_col_count_local() {
            let text = self.get_segment_text(c);
            width_info.push(Self::table_guess_text_width(ui, ctx, 0, text).at_least(table_info.col_min_width));
        }
        width_info
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
                                table_info.row_index.saturating_add(r) == 0;
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
                // 逻辑首行补顶边、首列补左边：每格 `Grid` 只有一行（`r==0`），
                // 须用 `table_row_index==0` 判断首行
                //（与 `layout_table_row_line` 的 `is_first_row` 一致）。
                let stroke = Stroke::new(0.5, ui.visuals().weak_text_color());
                let painter = ui.painter();

                for r in frame_start..=frame_end {
                    if let Some(row) = cell_rects.get(r) {
                        let is_logical_first_row = table_info.row_index.saturating_add(r) == 0;
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

    fn table_draw_checked_background(
        ui: &mut Ui,
        ctx: &Ctx,
        table_info: &TableInfo,
        current_row_checked: bool,
        all_cell_rects: &[Vec<Rect>],
        data_cell_rects: &[Vec<Rect>],
        frame_start: usize,
        frame_end: usize,
    ) {
        let row_count = all_cell_rects.len();
        if row_count == 0 {
            return;
        }
        let last = row_count.saturating_sub(1);
        let frame_start = frame_start.min(last);
        let frame_end = frame_end.min(last);
        if frame_start > frame_end {
            return;
        }

        // 采用较浅底色，避免覆盖文本可读性。
        let bg = ctx.cfg().select_color().linear_multiply(0.14);
        let painter = ui.painter();
        let show_indices = ctx.cfg().show_table_row_no;
        let expand_cell = |cell: &Rect| {
            cell.expand2(Vec2 {
                x: table_info.spacing_x / 2.0,
                y: table_info.spacing_y / 2.0,
            })
        };

        for r in frame_start..=frame_end {
            let logical_row = table_info.row_index.saturating_add(r);
            let row_checked = current_row_checked && logical_row == table_info.row_index;
            if row_checked {
                if let Some(row) = all_cell_rects.get(r) {
                    for cell in row {
                        painter.rect_filled(expand_cell(cell), 0.0, bg);
                    }
                }
            }

            if let Some(row) = data_cell_rects.get(r) {
                for (c, cell) in row.iter().enumerate() {
                    let col_checked = table_info.head_col_checked.get(c).copied().unwrap_or(false);
                    if col_checked {
                        painter.rect_filled(expand_cell(cell), 0.0, bg);
                    }
                }
            }

            if show_indices && logical_row == 0 && table_info.head_index_checked {
                if let Some(row) = all_cell_rects.get(r) {
                    if let Some(index_cell) = row.first() {
                        painter.rect_filled(expand_cell(index_cell), 0.0, bg);
                    }
                }
            }
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
            let ti = ctx.table_info_of_line(cursor.line_no).cloned().unwrap_or_default();
            let c = segment.min(ti.col_count.saturating_sub(1));
            let r = ctx
                .table_key_of_line(cursor.line_no)
                .and_then(|k| ctx.table_row_no(cursor.line_no, k))
                .unwrap_or(0);
            (r, c, 0usize)
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
            }
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
        let mut index_col_width = Self::table_index_col_width(
            table_info.logical_row_count_for_ui(),
            table_info.col_min_width,
        );
        if ctx.cfg().show_table_head_checkbox {
            index_col_width += Self::table_head_checkbox_total_width(ctx);
        }
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
        let show_row_checkbox = ctx.cfg().show_table_head_checkbox;
        let text_width = if show_row_checkbox {
            (index_col_width - Self::table_head_checkbox_total_width(ctx)).at_least(8.0)
        } else {
            index_col_width
        };
        let mut cell_size = ui.available_size_before_wrap();
        cell_size.y = cell_size.y.max(target_h);
        let row_rsp = ui.allocate_ui_with_layout(
            cell_size,
            Layout::left_to_right(Align::Min),
            |ui| {
            if show_row_checkbox {
                let checked = if row == 0 {
                    table_info.head_index_checked
                } else {
                    ctx.get_line(line_no)
                        .map(|p| p.row_index_checked)
                        .unwrap_or(false)
                };
                let (new_checked, _cb_rsp) =
                    Self::table_layout_head_checkbox(ui, ctx, checked, true);
                if row == 0 {
                    if new_checked != table_info.head_index_checked {
                        ctx.table_row_block_set_head_index_checked(line_no, new_checked);
                    }
                } else if new_checked != checked {
                    ctx.table_row_block_set_row_index_checked(line_no, row, new_checked);
                }
            }

            let mut item_rect = ui.cursor();
            item_rect.set_width(text_width);
            item_rect.set_height(target_h);
            let spacing = TextSpacing::text_spacing_in_rect(item_rect, text_width)
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
            *response |= rsp;
        },
        );
        row_cell_rects.push(row_rsp.response.rect);
    }

    fn layout_table_grid_row_finalize(
        show_indices: bool,
        mut row_cell_rects: Vec<Rect>,
    ) -> (Vec<Rect>, Vec<Rect>, f32) {
        let row_h = Self::normalize_table_row_cell_rects(&mut row_cell_rects);
        let data_cells = Self::table_data_column_rects(show_indices, &row_cell_rects);
        (row_cell_rects, data_cells, row_h)
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
        let r_idx = table_info.row_index;
        let rh0 = row_heights.get(0).copied().unwrap_or(default_row_h);
        let target_h = rh0.max(default_row_h);
        let mut row_cell_rects = vec![];
        if show_indices {
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
            let show_head_checkbox = r_idx == 0 && ctx.cfg().show_table_head_checkbox;
            let text_width = if show_head_checkbox {
                (warp_width - Self::table_head_checkbox_total_width(ctx)).at_least(8.0)
            } else {
                warp_width
            };
            let mut cell_size = ui.available_size_before_wrap();
            cell_size.y = cell_size.y.max(target_h);
            let cell_rsp = ui.allocate_ui_with_layout(
                cell_size,
                Layout::left_to_right(Align::Min),
                |ui| {
                if show_head_checkbox {
                    let checked = table_info.head_col_checked.get(c).copied().unwrap_or(false);
                    let (new_checked, _cb_rsp) =
                        Self::table_layout_head_checkbox(ui, ctx, checked, true);
                    if new_checked != checked {
                        ctx.table_row_block_set_head_col_checked(line_no, c, new_checked);
                    }
                }
                if let Some(pgh_segment) = ctx.get_line(line_no).and_then(|p| p.pgh.get(cell_i)) {
                    let text = pgh_segment.item.text();
                    let job = if r_idx == 0 {
                        Self::table_head_job(ui, ctx, &text)
                    } else {
                        Self::table_cell_job(ui, ctx, &text)
                    };
                    let mut item_rect = ui.cursor();
                    item_rect.set_width(text_width);
                    let spacing = TextSpacing::text_spacing_in_rect(item_rect, text_width)
                        .with_spacing_top_bottom(table_info.spacing_y / 2.0, table_info.spacing_y / 2.0)
                        .with_need_expand(true)
                        .with_once_allocate(true);
                    let rsp = PghText::layout_paragraph(
                        ui,
                        ctx,
                        line_no,
                        cell_i,
                        spacing,
                        text,
                        &Some(job),
                    );
                    *response |= rsp;
                } else {
                    let mut placeholder_rect = ui.cursor();
                    placeholder_rect.set_width(text_width);
                    let rh0 = row_heights.get(0).copied().unwrap_or(default_row_h);
                    placeholder_rect.set_height(rh0.max(default_row_h));
                    let _ = ui.allocate_rect(placeholder_rect, Sense::hover());
                }
            },
            );
            row_cell_rects.push(cell_rsp.response.rect);
        }

        Self::layout_table_grid_row_finalize(show_indices, row_cell_rects)
    }

    pub fn layout_table_row(ui: &mut Ui, ctx: &mut Ctx, line_no: usize) -> Response {
        let mut response = ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1;

        let width_info = ctx.table_guess_width_for_table_row_block(line_no, ui);
        let max_col_width = ctx.edit_width();
        let Some(mut table_info) = ctx.table_info_of_line(line_no).cloned() else {
            return response;
        };
        if let Some(table_key) = ctx.table_key_of_line(line_no) {
            if let Some(row_no) = ctx.table_row_no(line_no, table_key) {
                table_info.row_index = row_no;
            }
        }
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
                line_no, table_info.row_index
            );

            // TableRow 每行一个 Grid（仅一行）
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

                    Self::table_draw_checked_background(
                        ui,
                        ctx,
                        &table_info,
                        ctx.get_line(line_no).map(|p| p.row_index_checked).unwrap_or(false),
                        &all_cell_rects,
                        &data_cell_rects,
                        0,
                        0,
                    );
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
        let Some(mut table_info) = ctx.table_info_of_line(line_no).cloned() else {
            let response = response.on_hover_cursor(CursorIcon::Text);
            return LayoutResponse::new(response.clone(), response, handled);
        };
        if let Some(table_key) = ctx.table_key_of_line(line_no) {
            if let Some(row_no) = ctx.table_row_no(line_no, table_key) {
                table_info.row_index = row_no;
            }
        }
        {
            let size = icon_button_builder(ui)
                .icon(IconName::icon_chevron_down)
                .font_size(12.0)
                .size();
            let total_rows = table_info.row_count.max(1);
            let is_first_row = table_info.row_index == 0;
            let is_last_row = table_info.row_index + 1 >= total_rows;
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
        LayoutResponse::new(response.clone(), response, handled)
    }

}
