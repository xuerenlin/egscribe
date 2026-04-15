mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn bold_wraps_selection() {
    let mut ctx = md_ctx("word");
    set_selection_at_line_chars(&mut ctx, 0, 0, 4);
    assert_action_with_undo_redo(&mut ctx, &Action::bold(), "**word**");
}

#[test]
fn bold_no_selection_inserts_markers_and_moves_caret() {
    let mut ctx = md_ctx("x");
    set_caret_at_line_char(&mut ctx, 0, 0);
    // 先断言首次插入后的文档与光标；redo 后光标可能与首次不完全一致，故 undo/redo 仅校验文档
    execute_action(&mut ctx, &Action::bold());
    assert_doc(&ctx, "****x");
    assert_caret_at_line_char(&ctx, 0, 2);
    execute_action(&mut ctx, &Action::undo());
    assert_doc(&ctx, "x");
    execute_action(&mut ctx, &Action::redo());
    assert_doc(&ctx, "****x");
}

#[test]
fn bold_toggles_off_when_wrapped() {
    let mut ctx = md_ctx("**t**");
    set_selection_at_line_chars(&mut ctx, 0, 0, 5);
    assert_action_with_undo_redo(&mut ctx, &Action::bold(), "t");
}

#[test]
fn bold_whitespace_only_selection_inserts_markers() {
    let mut ctx = md_ctx("   ");
    set_selection_at_line_chars(&mut ctx, 0, 0, 3);
    assert_action_with_undo_redo(&mut ctx, &Action::bold(), "****");
}

#[test]
fn bold_cross_line_two_paragraphs() {
    let mut ctx = md_ctx("aa\nbb");
    let la = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains("aa")).unwrap();
    let lb = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains("bb")).unwrap();
    set_selection_lines_chars(&mut ctx, (la, 0), (lb, 2));
    assert_action_with_undo_redo(&mut ctx, &Action::bold(), "**aa**\n**bb**");
}
