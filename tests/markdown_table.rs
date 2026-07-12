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
//! - `table_split_by_selected_cols_keeps_parent_and_content`
//! - `table_split_by_selected_cols_ignores_unselected_cols_before_split`
//! - `table_split_by_selected_cols_no_content_cols_keeps_original`
//! - `table_split_by_selected_cols_inherits_parent_heading_level`
//! - `table_merge_under_current_heading_flattens_descendant_tables`
//! - `table_selection_with_surrounding_text_copy_keeps_separator`

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

#[test]
fn table_split_by_selected_cols_keeps_parent_and_content() {
    let src = "|ColA|ColB|ColC|ColD|ColE|\n|--|--|--|--|--|\n|H1|ig|H1Group1|foo|as|\n||ig||bar|4|\n||ig||test|5|\n||ig|H1Group2|xxx|zzz|\n||ig||yyy|iii|\n|H2|test|H2Group1|i|1|\n||ig|H2Group2|j|2|\n||ig||k|3|\n||ig||l|4|";
    let mut ctx = md_ctx(src);
    ctx.table_row_block_set_head_col_checked(0, 0, true);
    ctx.table_row_block_set_head_col_checked(0, 2, true);
    set_caret(&mut ctx, cursor(0, 2, 0));
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::table_split_by_selected_cols(),
        "## H1\n### H1Group1\n|ColD|ColE|\n|--|--|\n|foo|as|\n|bar|4|\n|test|5|\n\n### H1Group2\n|ColD|ColE|\n|--|--|\n|xxx|zzz|\n|yyy|iii|\n\n## H2\n### H2Group1\n|ColD|ColE|\n|--|--|\n|i|1|\n\n### H2Group2\n|ColD|ColE|\n|--|--|\n|j|2|\n|k|3|\n|l|4|",
    );
}

#[test]
fn table_split_by_selected_cols_ignores_unselected_cols_before_split() {
    let src = "|A|B|C|D|\n|--|--|--|--|\n|P1|skip|G1|x|\n||skip||y|\n|P2|skip|G2|z|";
    let mut ctx = md_ctx(src);
    ctx.table_row_block_set_head_col_checked(0, 2, true);
    set_caret(&mut ctx, cursor(0, 2, 0));
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::table_split_by_selected_cols(),
        "## G1\n|D|\n|--|\n|x|\n|y|\n\n## G2\n|D|\n|--|\n|z|",
    );
}

#[test]
fn table_split_by_selected_cols_no_content_cols_keeps_original() {
    let src = "|A|B|C|\n|--|--|--|\n|P|x|g1|\n||y|g2|";
    let mut ctx = md_ctx(src);
    ctx.table_row_block_set_head_col_checked(0, 2, true);
    set_caret(&mut ctx, cursor(0, 2, 0));
    assert_action_with_undo_redo(&mut ctx, &Action::table_split_by_selected_cols(), "|A|B|C|\n|--|--|--|\n|P|x|g1|\n||y|g2|");
}

#[test]
fn table_split_by_selected_cols_inherits_parent_heading_level() {
    let src = "### Parent\n|A|B|C|D|\n|--|--|--|--|\n|H1|ig|G1|x|\n||ig||y|";
    let mut ctx = md_ctx(src);
    ctx.table_row_block_set_head_col_checked(1, 0, true);
    ctx.table_row_block_set_head_col_checked(1, 2, true);
    set_caret(&mut ctx, cursor(1, 2, 0));
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::table_split_by_selected_cols(),
        "### Parent\n#### H1\n##### G1\n|D|\n|--|\n|x|\n|y|",
    );
}

#[test]
fn table_merge_under_current_heading_flattens_descendant_tables() {
    let src = "## Sheet1\n### V6.1.0版本IF1接口核心字段适配\n#### 生产日期字段适配\n|五级功能点描述|E||用户ID、用户姓名、查询时间|\n|--|--|--|--|\n|基于V6.1.0版本IF1接口中新增的生产日期字段，增加对该字段上报消息的解析、数据清洗及数据入库。|E|接口消息表|生产日期、消息ID、时间戳|\n|基于V6.1.0版本IF1接口中新增的生产日期字段，增加对该字段上报消息的解析、数据清洗及数据入库。|R|清洗规则配置表|规则ID、字段名称、格式要求|\n段落中其他的文本，表格合并时应该忽略。\n#### 生产日期字段前端查询功能\n|五级功能点描述|E||用户ID、用户姓名、查询时间|\n|--|--|--|--|\n|支持用户在前端界面手动输入查询条件（如生产日期范围、具体生产日期等）|E|查询条件表|开始日期、结束日期、查询模式|\n|支持用户在前端界面手动输入查询条件（如生产日期范围、具体生产日期等）|R|机顶盒设备表|设备编号、生产日期、设备状态、型号编码|";
    let mut ctx = md_ctx(src);
    set_caret(&mut ctx, cursor(0, 0, 0));
    assert_eq!(
        ctx.current_outline_merged_table().as_deref(),
        Some("|X级目录|XX级目录|五级功能点描述|E||用户ID、用户姓名、查询时间|\n|--|--|--|--|--|--|\n|V6.1.0版本IF1接口核心字段适配|生产日期字段适配|基于V6.1.0版本IF1接口中新增的生产日期字段，增加对该字段上报消息的解析、数据清洗及数据入库。|E|接口消息表|生产日期、消息ID、时间戳|\n|V6.1.0版本IF1接口核心字段适配|生产日期字段适配|基于V6.1.0版本IF1接口中新增的生产日期字段，增加对该字段上报消息的解析、数据清洗及数据入库。|R|清洗规则配置表|规则ID、字段名称、格式要求|\n|V6.1.0版本IF1接口核心字段适配|生产日期字段前端查询功能|支持用户在前端界面手动输入查询条件（如生产日期范围、具体生产日期等）|E|查询条件表|开始日期、结束日期、查询模式|\n|V6.1.0版本IF1接口核心字段适配|生产日期字段前端查询功能|支持用户在前端界面手动输入查询条件（如生产日期范围、具体生产日期等）|R|机顶盒设备表|设备编号、生产日期、设备状态、型号编码|")
    );
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::table_merge_under_current_heading(),
        "## Sheet1\n|X级目录|XX级目录|五级功能点描述|E||用户ID、用户姓名、查询时间|\n|--|--|--|--|--|--|\n|V6.1.0版本IF1接口核心字段适配|生产日期字段适配|基于V6.1.0版本IF1接口中新增的生产日期字段，增加对该字段上报消息的解析、数据清洗及数据入库。|E|接口消息表|生产日期、消息ID、时间戳|\n|V6.1.0版本IF1接口核心字段适配|生产日期字段适配|基于V6.1.0版本IF1接口中新增的生产日期字段，增加对该字段上报消息的解析、数据清洗及数据入库。|R|清洗规则配置表|规则ID、字段名称、格式要求|\n|V6.1.0版本IF1接口核心字段适配|生产日期字段前端查询功能|支持用户在前端界面手动输入查询条件（如生产日期范围、具体生产日期等）|E|查询条件表|开始日期、结束日期、查询模式|\n|V6.1.0版本IF1接口核心字段适配|生产日期字段前端查询功能|支持用户在前端界面手动输入查询条件（如生产日期范围、具体生产日期等）|R|机顶盒设备表|设备编号、生产日期、设备状态、型号编码|",
    );
}

#[test]
fn table_selection_with_surrounding_text_copy_keeps_separator() {
    let src = "before\n\n| a | b |\n| --- | --- |\n| x | y |\n\nafter";
    let mut ctx = md_ctx(src);
    let line_after = find_line_containing(&ctx, "after");
    assert!(!ctx.is_table_line(line_after), "after 行应保持普通文本");
    let end_char = ctx.get_line_text(line_after).chars().count();
    set_selection_lines_chars(&mut ctx, (0, 0), (line_after, end_char));
    assert_eq!(
        ctx.get_selected_text(),
        "before\n\n|a|b|\n|--|--|\n|x|y|\n\nafter",
        "跨域表格与前后文本复制时应保留表头分隔行",
    );
}

#[test]
fn table_ops_after_inserting_text_before_table_keep_table_behavior() {
    let src = "before\n| a | b |\n| --- | --- |\n| x | y |\n| p | q |";
    let mut ctx = md_ctx(src);
    let end = ctx.get_line_text(0).chars().count();
    set_caret_at_line_char(&mut ctx, 0, end);
    execute_action(&mut ctx, &Action::enter(true));
    let (line_no, _) = find_table_data_row_line_last_col(&ctx, 'x');
    set_caret_line_segment0(&mut ctx, line_no, 1);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::enter(false),
        "before\n\n|a|b|\n|--|--|\n|x|y|\n|||\n|p|q|",
    );
}

#[test]
fn copy_document_with_two_tables_keeps_both_separators() {
    let src = "| a | b |\n| --- | --- |\n| x | y |\n\nmid\n\n| c | d |\n| --- | --- |\n| p | q |";
    let mut ctx = md_ctx(src);
    let last_line = ctx.line_num().saturating_sub(1);
    let end_char = ctx.get_line_text(last_line).chars().count();
    set_selection_lines_chars(&mut ctx, (0, 0), (last_line, end_char));
    assert_eq!(
        ctx.get_selected_text(),
        "|a|b|\n|--|--|\n|x|y|\n\nmid\n\n|c|d|\n|--|--|\n|p|q|",
        "跨多表全文复制时应保留每个表的分隔行",
    );
}

#[test]
fn text_only_edit_does_not_schedule_rebuild_index_task() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let gen_before = ctx.rebuild_index_latest_gen();
    let (line_no, _) = find_table_data_row_line_last_col(&ctx, 'x');
    set_caret_line_segment0(&mut ctx, line_no, 2);
    execute_action(&mut ctx, &Action::insert_text("Z".to_string()));
    let gen_after = ctx.rebuild_index_latest_gen();
    assert!(
        !ctx.has_rebuild_index_task(),
        "纯文本编辑不应触发 rebuild_index 任务"
    );
    assert_eq!(
        gen_after, gen_before,
        "纯文本编辑不应创建新的重建任务代次"
    );
}

#[test]
fn line_basic_change_restarts_rebuild_index_task() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let gen_before = ctx.rebuild_index_latest_gen();
    let (line_no, _) = find_table_data_row_line_last_col(&ctx, 'x');
    set_caret_line_segment0(&mut ctx, line_no, 1);
    execute_action(&mut ctx, &Action::enter(false));
    let gen1 = ctx.rebuild_index_latest_gen();
    assert!(gen1 > gen_before, "首次结构变更应推进任务代次");

    // 在任务完成前再次触发行结构变化，验证任务会被替换（gen 递增）
    set_caret_line_segment0(&mut ctx, line_no + 1, 0);
    execute_action(&mut ctx, &Action::enter(false));
    let gen2 = ctx.rebuild_index_latest_gen();
    assert!(gen2 > gen1, "重建中再次触发行结构变化时应合并 pending 并推进代次");
}
