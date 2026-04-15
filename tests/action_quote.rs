mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn quote_prefix_applied() {
    let mut ctx = md_ctx("text");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::quote(), "> text");
}

#[test]
fn quote_with_selection_mid_line() {
    let mut ctx = md_ctx("hello world");
    set_selection_at_line_chars(&mut ctx, 0, 0, 5);
    assert_action_with_undo_redo(&mut ctx, &Action::quote(), "> hello world");
}
