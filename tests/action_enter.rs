mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn enter_inserts_newline_flow() {
    let mut ctx = md_ctx("hi");
    set_caret_at_line_char(&mut ctx, 0, 2);
    assert_action_with_undo_redo(&mut ctx, &Action::enter(false), "hi\n");
}

#[test]
fn enter_ctrl_inserts_line_below() {
    let mut ctx = md_ctx("one");
    set_caret_at_line_char(&mut ctx, 0, 1);
    assert_action_with_undo_redo(&mut ctx, &Action::enter(true), "one\n");
}

#[test]
fn enter_ctrl_at_end_of_single_line() {
    let mut ctx = md_ctx("z");
    let end = ctx.get_line_text(0).chars().count();
    set_caret_at_line_char(&mut ctx, 0, end);
    execute_action(&mut ctx, &Action::enter(true));
    assert_doc(&ctx, "z\n");
}

#[test]
fn enter_normal_inside_fenced_code_body() {
    let mut ctx = md_ctx("```\nbody\n```");
    let ln = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("body"))
        .expect("code body line");
    let col = ctx.get_line_text(ln).chars().count().min(2);
    set_caret_at_line_char(&mut ctx, ln, col);
    assert_action_with_undo_redo(&mut ctx, &Action::enter(false), "```\nbo\ndy\n```");
}
