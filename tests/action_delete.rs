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
