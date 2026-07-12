mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn delete_removes_selection() {
    let mut ctx = md_ctx("abcdef");
    set_selection_at_line_chars(&mut ctx, 0, 1, 4);
    assert_action_with_undo_redo(&mut ctx, &Action::delete(), "aef");
}

#[test]
fn delete_read_only_noop() {
    let mut ctx = md_ctx("abc").read_only(true);
    set_selection_at_line_chars(&mut ctx, 0, 0, 2);
    assert_action_with_undo_redo(&mut ctx, &Action::delete(), "abc");
}

#[test]
fn delete_cross_line_markdown_paragraphs() {
    let mut ctx = md_ctx("aa\n\nbb\n\ncc");
    assert!(ctx.line_num() >= 3);
    let la = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("aa"))
        .unwrap();
    let lc = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("cc"))
        .unwrap();
    let end_cc = ctx.get_line_text(lc).chars().count();
    set_selection_lines_chars(&mut ctx, (la, 0), (lc, end_cc));
    assert_action_with_undo_redo(&mut ctx, &Action::delete(), "");
}

#[test]
fn delete_without_selection_removes_following_char() {
    let mut ctx = md_ctx("abcd");
    set_caret_at_line_char(&mut ctx, 0, 1);
    assert_action_with_undo_redo(&mut ctx, &Action::delete(), "acd");
}

#[test]
fn delete_without_selection_at_line_end() {
    let mut ctx = md_ctx("ab");
    let end = ctx.get_line_text(0).chars().count();
    set_caret_at_line_char(&mut ctx, 0, end);
    assert_action_with_undo_redo(&mut ctx, &Action::delete(), "");
}

#[test]
fn delete_select_all_large_doc_undo_redo() {
    let mut text = String::new();
    for i in 0..3000 {
        if i > 0 {
            text.push('\n');
        }
        text.push_str(&format!("line-{i}"));
    }
    let mut ctx = md_ctx(&text);
    ctx.set_cursors_select_all();
    assert_action_with_undo_redo(&mut ctx, &Action::delete(), "");
}

const TABLE_2COL_2DATA: &str = "| a | b |\n| --- | --- |\n| x | y |\n| p | q |";

#[test]
fn delete_table_row_partial_block_selection_keeps_structure() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let lx = find_line_containing(&ctx, "x");
    let lp = find_line_containing(&ctx, "p");
    let c1 = ctx.cursor_check(&cursor(lx, 0, 0));
    let c2 = ctx.cursor_check(&cursor(lp, 0, 1));
    set_selection(&mut ctx, c1, c2);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::delete(),
        "|a|b|\n|--|--|\n||y|\n||q|",
    );
}

#[test]
fn delete_table_row_full_row_selection_deletes_row() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let line_no = find_line_containing(&ctx, "x");
    let n = ctx.get_line_text(line_no).chars().count();
    set_selection_at_line_chars(&mut ctx, line_no, 0, n);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::delete(),
        "|a|b|\n|--|--|\n|p|q|",
    );
}

#[test]
fn delete_table_row_full_col_selection_deletes_col() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let lp = find_line_containing(&ctx, "p");
    let c1 = ctx.cursor_check(&cursor(0, 1, 0));
    let c2 = ctx.cursor_check(&cursor(lp, 1, 1));
    set_selection(&mut ctx, c1, c2);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::delete(),
        "|a|\n|--|\n|x|\n|p|",
    );
}

#[test]
fn delete_cross_table_rows_line_chars_deletes_full_rows() {
    let mut ctx = md_ctx(TABLE_2COL_2DATA);
    let lx = find_line_containing(&ctx, "x");
    let lp = find_line_containing(&ctx, "p");
    let p_end = ctx.get_line_text(lp).chars().count();
    set_selection_lines_chars(&mut ctx, (lx, 0), (lp, p_end));
    assert_action_with_undo_redo(&mut ctx, &Action::delete(), "|a|b|\n");
}
