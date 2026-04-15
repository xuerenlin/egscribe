mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn unordered_list_inserts_bullet() {
    let mut ctx = md_ctx("item");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::unordered_list(), "- item");
}

#[test]
fn unordered_list_multiline_selection() {
    let mut ctx = md_ctx("a\n\nb");
    let la = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains('a')).unwrap();
    let lb = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains('b')).unwrap();
    set_selection_lines_chars(&mut ctx, (la, 0), (lb, 1));
    assert_action_with_undo_redo(&mut ctx, &Action::unordered_list(), "- a\n- \n- b");
}
