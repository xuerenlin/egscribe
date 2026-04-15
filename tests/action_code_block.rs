mod common;

use common::*;
use egscribe::medit::{Action, PghType};

#[test]
fn code_block_wraps_current_line_when_no_selection() {
    let mut ctx = md_ctx("line");
    set_caret_at_line_char(&mut ctx, 0, 2);
    assert_action_with_undo_redo(&mut ctx, &Action::code_block(), "```\nline\n```");
}

#[test]
fn code_block_noop_when_selection_touches_code_row() {
    let mut ctx = md_ctx("```\nbody\n```");
    let code_line = (0..ctx.line_num())
        .find(|&i| ctx.is_line_type(i, PghType::CodeRow))
        .expect("fenced code should contain a CodeRow line");
    let before = ctx.get_all_text();
    set_selection_at_line_chars(&mut ctx, code_line, 0, 1);
    execute_action(&mut ctx, &Action::code_block());
    assert_eq!(ctx.get_all_text(), before);
}

#[test]
fn code_block_multiline_paragraph_selection() {
    let mut ctx = md_ctx("one\n\ntwo\n\nthree");
    let a = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains("one")).unwrap();
    let c = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains("three")).unwrap();
    let end = ctx.get_line_text(c).chars().count();
    set_selection_lines_chars(&mut ctx, (a, 0), (c, end));
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::code_block(),
        "```\none\n\ntwo\n\nthree\n```",
    );
}

#[test]
fn code_block_empty_line_no_selection() {
    let mut ctx = md_ctx("");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::code_block(), "```\n\n```");
}
