mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn ordered_list_inserts_number() {
    let mut ctx = md_ctx("step");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::ordered_list(), "1. step");
}

#[test]
fn ordered_list_second_line_continuation() {
    let mut ctx = md_ctx("1. first\n\nsecond");
    let ln = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("second"))
        .unwrap();
    set_caret_at_line_char(&mut ctx, ln, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::ordered_list(), "1. first\n\n1. second");
}

#[test]
fn ordered_list_adjacent_line_continuation_starts_from_next_number() {
    let mut ctx = md_ctx("1. first\nsecond");
    let ln = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("second"))
        .unwrap();
    set_caret_at_line_char(&mut ctx, ln, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::ordered_list(), "1. first\n2. second");
}
