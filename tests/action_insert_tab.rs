mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn insert_tab_without_selection_inserts_tab_char() {
    let mut ctx = md_ctx("ab");
    set_caret_at_line_char(&mut ctx, 0, 1);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_tab(), "a\tb");
}

#[test]
fn insert_tab_multiline_non_table_prepends_tab_each_line() {
    let mut ctx = md_ctx("aa\n\nbb\n\ncc");
    let la = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains("aa")).unwrap();
    let lc = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains("cc")).unwrap();
    let end_cc = ctx.get_line_text(lc).chars().count();
    set_selection_lines_chars(&mut ctx, (la, 0), (lc, end_cc));
    assert_action_with_undo_redo(&mut ctx, &Action::insert_tab(), "\taa\n\t\n\tbb\n\t\n\tcc");
}

#[test]
fn insert_tab_inside_table_row_inserts_single_tab() {
    let mut ctx = md_ctx("| a | b |\n| --- | --- |\n| x | y |");
    let row = (0..ctx.line_num())
        .find(|&i| {
            let t = ctx.get_line_text(i);
            t.contains('x') && !t.chars().all(|c| c == '|' || c == '-' || c.is_whitespace())
        })
        .expect("data row");
    set_selection_line_segment0(&mut ctx, row, 1, 3);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_tab(), "|a|b|\n|--|--|\n|x\t|y|");
}

#[test]
fn insert_tab_inside_code_fence_single_tab() {
    let mut ctx = md_ctx("```\nab\n```");
    let ln = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("ab"))
        .unwrap();
    set_caret_at_line_char(&mut ctx, ln, 1);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_tab(), "```\na\tb\n```");
}
