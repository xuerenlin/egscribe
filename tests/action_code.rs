mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn inline_code_wraps_selection() {
    let mut ctx = md_ctx("code");
    set_selection_at_line_chars(&mut ctx, 0, 0, 4);
    assert_action_with_undo_redo(&mut ctx, &Action::code(), "`code`");
}

#[test]
fn code_toggle_off_when_wrapped_in_backticks() {
    let mut ctx = md_ctx("`x`");
    set_selection_at_line_chars(&mut ctx, 0, 0, 3);
    assert_action_with_undo_redo(&mut ctx, &Action::code(), "x");
}
