mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn cut_removes_selected_text() {
    let mut ctx = md_ctx("abcdef");
    set_selection_at_line_chars(&mut ctx, 0, 2, 4);
    assert_action_with_undo_redo(&mut ctx, &Action::cut(), "abef");
}

#[test]
fn cut_read_only_noop() {
    let mut ctx = md_ctx("abc").read_only(true);
    set_selection_at_line_chars(&mut ctx, 0, 0, 2);
    assert_action_with_undo_redo(&mut ctx, &Action::cut(), "abc");
}

#[test]
fn cut_cross_line_markdown() {
    let mut ctx = md_ctx("p1\n\np2\n\np3");
    let a = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("p1"))
        .unwrap();
    let c = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("p3"))
        .unwrap();
    let end_p3 = ctx.get_line_text(c).chars().count();
    set_selection_lines_chars(&mut ctx, (a, 0), (c, end_p3));
    execute_action(&mut ctx, &Action::cut());
    assert_doc(&ctx, "");
}

#[test]
fn cut_collapsed_selection_no_change() {
    let mut ctx = md_ctx("xyz");
    set_caret_at_line_char(&mut ctx, 0, 1);
    let before = ctx.get_all_text();
    assert_action_with_undo_redo(&mut ctx, &Action::cut(), &before);
}
