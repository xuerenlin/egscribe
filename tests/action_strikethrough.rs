mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn strikethrough_wraps_selection() {
    let mut ctx = md_ctx("word");
    set_selection_at_line_chars(&mut ctx, 0, 0, 4);
    assert_action_with_undo_redo(&mut ctx, &Action::strikethrough(), "~~word~~");
}

#[test]
fn strikethrough_cross_line() {
    let mut ctx = md_ctx("m\n\nn");
    let a = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains('m')).unwrap();
    let b = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains('n')).unwrap();
    set_selection_lines_chars(&mut ctx, (a, 0), (b, 1));
    assert_action_with_undo_redo(&mut ctx, &Action::strikethrough(), "~~m~~\n\n~~n~~");
}
