mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn backspace_deletes_one_char_before_caret() {
    let mut ctx = md_ctx("abc");
    set_caret_at_line_char(&mut ctx, 0, 3);
    assert_action_with_undo_redo(&mut ctx, &Action::backspace(), "ab");
}

#[test]
fn backspace_with_selection_deletes_range() {
    let mut ctx = md_ctx("abcdef");
    set_selection_at_line_chars(&mut ctx, 0, 2, 4);
    assert_action_with_undo_redo(&mut ctx, &Action::backspace(), "abef");
}

#[test]
fn backspace_read_only_noop() {
    let mut ctx = md_ctx("ab").read_only(true);
    set_caret_at_line_char(&mut ctx, 0, 2);
    execute_action(&mut ctx, &Action::backspace());
    assert_doc(&ctx, "ab");
}

#[test]
fn backspace_without_selection_at_line_start() {
    let mut ctx = md_ctx("ab");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::backspace(), "ab");
}

#[test]
fn backspace_list_item_line_after_enter_then_prefix() {
    // 列表行：光标在行首 item 前；退格可能走合并/前缀逻辑（与实现绑定，只断言不 panic）
    let mut ctx = md_ctx("- hello");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::backspace(), "- hello");
}

#[test]
fn backspace_cross_line_selection() {
    let mut ctx = md_ctx("xx\n\nyy\n\nzz");
    let lx = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains("xx")).unwrap();
    let lz = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains("zz")).unwrap();
    let end_zz = ctx.get_line_text(lz).chars().count();
    set_selection_lines_chars(&mut ctx, (lx, 0), (lz, end_zz));
    assert_action_with_undo_redo(&mut ctx, &Action::backspace(), "");
}
