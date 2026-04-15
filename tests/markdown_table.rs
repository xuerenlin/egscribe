//! GFM 表格段落专项：`#[test]` 索引
//! - `table_full_row_segment0_backspace_clears_row_like_empty_insert`
//! - `table_full_row_selected_via_line_chars_backspace`（末行数据整行逻辑字符全选 + `backspace`）
//! - `table_middle_data_row_full_line_chars_backspace`（中间数据行 `|x|y|` 整行逻辑字符全选 + `backspace`）
//! - `table_full_row_segment0_delete_matches_backspace`
//! - `table_full_row_segment0_cut_doc`
//! - `table_enter_after_first_cell_appends_empty_row`（表内 Enter 追加空数据行）
//! - `table_cross_cell_segment0_insert_text`（跨单元格 segment0 选区 + `insert_text`）
//! - `table_column_block_first_col_backspace`（跨数据行的列矩形选区 + `backspace`）
//! - `table_column_block_three_cols_middle_and_last_delete_drops_empty_cols`（三列表跨整块删中间+末列后自动去掉空列）
//! - `table_column_block_second_col_insert_text`（列矩形选区 + `insert_text`）
//! - `table_insert_text_merge_subtable_at_cell`（`insert_text` 粘贴整张 GFM 表，与剪贴板 `paste` 同合并路径）
//! - `table_insert_text_merge_subtable_wider_than_main`（子表列数大于主表，`table_row_block_insert_col` 扩展）
//! - `table_insert_text_merge_subtable_taller_than_main`（子表数据行数超出块内剩余行，插逻辑行扩展）

mod common;

use common::*;
use egscribe::medit::Action;

/// 最小 GFM 表：表头 + 分隔行 + 两行数据（`| x | y |`、`| p | q |`）。
const TABLE_2COL_2DATA: &str = "| a | b |\n| --- | --- |\n| x | y |\n| p | q |";

/// 三列表：表头 + 分隔行 + 两行数据。
const TABLE_3COL_2DATA: &str = "| a | b | c |\n| --- | --- | --- |\n| x | y | z |\n| p | q | r |";

#[test]
fn table_full_row_segment0_backspace_clears_row_like_empty_insert() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (line_no, last_col) = find_table_data_row_line_last_col(&ctx, 'x');
    set_selection_line_segment0(&mut ctx, line_no, 0, last_col);
    assert_action_with_undo_redo(&mut ctx, &Action::backspace(), "|a|b|\n|--|--|\n||y|\n|p|q|");
}

/// 末行数据：整行用 `set_selection_at_line_chars` 全选（与 segment0 整行选区路径不同），`backspace` 仍走选区删除；删空末行数据后留下空行（`get_all_text` 为行末换行 + 空行）。
#[test]
fn table_full_row_selected_via_line_chars_backspace() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (line_no, _) = find_table_data_row_line_last_col(&ctx, 'p');
    let t = ctx.get_line_text(line_no);
    let n = t.chars().count();
    set_selection_at_line_chars(&mut ctx, line_no, 0, n);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::backspace(),
        "|a|b|\n|--|--|\n|x|y|\n",
    );
}

/// 中间数据行（`| x | y |` 对应行）：整行逻辑字符全选 + `backspace`；中间空 `Text` 行会被合并去掉，表仍为一块（`get_all_text` 无额外空段）。
#[test]
fn table_middle_data_row_full_line_chars_backspace() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (line_no, _) = find_table_data_row_line_last_col(&ctx, 'x');
    let t = ctx.get_line_text(line_no);
    let n = t.chars().count();
    set_selection_at_line_chars(&mut ctx, line_no, 0, n);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::backspace(),
        "|a|b|\n|--|--|\n|p|q|",
    );
}

#[test]
fn table_full_row_segment0_delete_matches_backspace() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (line_no, last_col) = find_table_data_row_line_last_col(&ctx, 'x');
    set_selection_line_segment0(&mut ctx, line_no, 0, last_col);
    assert_action_with_undo_redo(&mut ctx, &Action::delete(), "|a|b|\n|--|--|\n||y|\n|p|q|");
}

#[test]
fn table_full_row_segment0_cut_doc() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (line_no, last_col) = find_table_data_row_line_last_col(&ctx, 'x');
    set_selection_line_segment0(&mut ctx, line_no, 0, last_col);
    assert_action_with_undo_redo(&mut ctx, &Action::cut(), "|a|b|\n|--|--|\n||y|\n|p|q|");
}

#[test]
fn table_enter_after_first_cell_appends_empty_row() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (line_no, col) = (0..ctx.line_num())
        .find_map(|i| {
            let t = ctx.get_line_text(i);
            if !t.contains('x') {
                return None;
            }
            if t.chars().all(|c| c == '|' || c == '-' || c.is_whitespace()) {
                return None;
            }
            let char_idx = t.find('x').map(|b| t[..b].chars().count()).unwrap_or(0);
            Some((i, char_idx))
        })
        .expect("data row");
    set_caret_line_segment0(&mut ctx, line_no, col + 1);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::enter(false),
        "|a|b|\n|--|--|\n|x|y|\n|||\n|p|q|",
    );
}

#[test]
fn table_cross_cell_segment0_insert_text() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (line_no, _) = find_table_data_row_line_last_col(&ctx, 'x');
    let t = ctx.get_line_text(line_no);
    let x_pos = t.find('x').unwrap();
    let y_pos = t.find('y').unwrap();
    let c0 = t[..x_pos].chars().count();
    let c1 = t[..y_pos].chars().count() + 1;
    set_selection_line_segment0(&mut ctx, line_no, c0, c1);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::insert_text("Z".into()),
        "|a|b|\n|--|--|\n|xZ|y|\n|p|q|",
    );
}

/// `Ctx::table_row_block_column_rect`：跨两行 `TableRow`、同一列（segment）的矩形选区；有选区时 `backspace` 即 `delete`。
#[test]
fn table_column_block_first_col_backspace() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (lx, _) = find_table_data_row_line_last_col(&ctx, 'x');
    let (lp, _) = find_table_data_row_line_last_col(&ctx, 'p');
    let c1 = ctx.cursor_check(&cursor(lx, 0, 0));
    let c2 = ctx.cursor_check(&cursor(lp, 0, 1));
    set_selection(&mut ctx, c1, c2);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::backspace(),
        "|a|b|\n|--|--|\n||y|\n||q|",
    );
}

/// 跨整块 `TableRow`（含表头与分隔行）的列矩形：删掉中间列与最后一列的正文后，块内全为空白的多余列从模型中移除，不保留 `||` 空列。
#[test]
fn table_column_block_three_cols_middle_and_last_delete_drops_empty_cols() {
    let mut ctx = md_ctx(TABLE_3COL_2DATA);
    // 列矩形右下角若仅落在末格 `culumn == 0`，`table_row_column_block_cell_span` 会把末格删成空区间；末格需指到格尾。
    let c1 = ctx.cursor_check(&cursor(0, 1, 0));
    let (lp, _) = find_table_data_row_line_last_col(&ctx, 'p');
    let c2 = ctx.cursor_check(
        &ctx.get_line(lp)
            .expect("second data row")
            .end_cursor_of_line(lp),
    );
    set_selection(&mut ctx, c1, c2);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::delete(),
        "|a|\n|--|\n|x|\n|p|",
    );
}

#[test]
fn table_column_block_second_col_insert_text() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (lx, _) = find_table_data_row_line_last_col(&ctx, 'x');
    let (lp, _) = find_table_data_row_line_last_col(&ctx, 'p');
    let c1 = ctx.cursor_check(&cursor(lx, 1, 0));
    let c2 = ctx.cursor_check(&cursor(lp, 1, 1));
    set_selection(&mut ctx, c1, c2);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::insert_text("Z".into()),
        "|a|b|\n|--|--|\n|x|Z|\n|p||",
    );
}

/// `Ctx::insert` 在 `TableRow` 上识别整张表 Markdown 时走 `table_row_block_merge_paste`（与 `Action::paste` 读剪贴板后 `insert` 同路径）。
#[test]
fn table_insert_text_merge_subtable_at_cell() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (ln, _) = find_table_data_row_line_last_col(&ctx, 'x');
    set_caret_line_segment0(&mut ctx, ln, 1);
    let sub = "|u|v|\n|---|---|\n|m|n|";
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::insert_text(sub.to_string()),
        "|a|b|\n|--|--|\n|u|v|\n|m|n|",
    );
}

/// 子表列数多于主表：从首格锚点合并，块内 `table_row_block_insert_col` 扩列；子表头/分隔/首数据行写入后原第二行数据被覆盖（以 `get_all_text()` 为准）。
#[test]
fn table_insert_text_merge_subtable_wider_than_main() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (ln, _) = find_table_data_row_line_last_col(&ctx, 'x');
    set_caret_line_segment0(&mut ctx, ln, 0);
    let sub = "|1|2|3|\n|---|---|---|\n|A|B|C|";
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::insert_text(sub.to_string()),
        "|a|b||\n|--|--|--|\n|1|2|3|\n|A|B|C|",
    );
}

/// 子表数据行数超出锚点下剩余行：`table_row_block_insert_logical_row` 扩行后整块仍连续；原 `|p|q|` 行被子表末行覆盖。
#[test]
fn table_insert_text_merge_subtable_taller_than_main() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let (ln, _) = find_table_data_row_line_last_col(&ctx, 'x');
    set_caret_line_segment0(&mut ctx, ln, 0);
    let sub = "|d|e|\n|---|---|\n|f|g|\n|h|i|\n|j|k|";
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::insert_text(sub.to_string()),
        "|a|b|\n|--|--|\n|d|e|\n|f|g|\n|h|i|\n|j|k|",
    );
}
