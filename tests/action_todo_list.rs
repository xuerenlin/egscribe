mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn todo_list_inserts_checkbox_markdown() {
    let mut ctx = md_ctx("todo");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::todo_list(), "- [ ] todo");
}

#[test]
fn todo_list_on_partial_selection() {
    let mut ctx = md_ctx("task body");
    set_selection_at_line_chars(&mut ctx, 0, 0, 4);
    assert_action_with_undo_redo(&mut ctx, &Action::todo_list(), "- [ ] task body");
}
