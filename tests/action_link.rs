mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn link_inserts_placeholder() {
    let mut ctx = md_ctx("t");
    set_caret_at_line_char(&mut ctx, 0, 1);
    assert_action_with_undo_redo(&mut ctx, &Action::link(), "t[text](url)");
}

#[test]
fn link_single_line_selected_existing_link_short_circuits() {
    let mut ctx = md_ctx("[label](http://x)");
    let t = ctx.get_line_text(0);
    let end = t.chars().count();
    set_selection_at_line_chars(&mut ctx, 0, 0, end);
    let before = ctx.get_all_text();
    assert_action_with_undo_redo(&mut ctx, &Action::link(), &before);
}

#[test]
fn link_multiline_placeholder() {
    let mut ctx = md_ctx("a\n\nb");
    let la = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains('a')).unwrap();
    let lb = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains('b')).unwrap();
    set_selection_lines_chars(&mut ctx, (la, 0), (lb, 1));
    assert_action_with_undo_redo(&mut ctx, &Action::link(), "[a](url)\n\n[b](url)");
}
