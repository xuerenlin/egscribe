//! 已有 Markdown 列表段落上的编辑专项（与 `action_*_list` 的「列表动作」互补）。
//!
//! `#[test]` 索引：
//! - `list_second_item_line_start_backspace`
//! - `list_item_mid_enter_duplicates_item_prefix_block`
//! - `list_cross_two_items_delete`
//! - `list_cross_two_items_insert_text`
//! - `list_line_insert_tab_inside_item_text`
//! - `list_ordered_second_line_delete_char`

mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn list_second_item_line_start_backspace() {
    let mut ctx = md_ctx("- aa\n- bb\n\nmiddle\n\n1. ox\n1. oy\n\n- [ ] todo\n");
    let ln = find_line_containing(&ctx, "bb");
    set_caret_at_line_char(&mut ctx, ln, 0);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::backspace(),
        "- aa- bb\n\nmiddle\n\n1. ox\n1. oy\n\n- [ ] todo\n",
    );
}

#[test]
fn list_item_mid_enter_duplicates_item_prefix_block() {
    let mut ctx = md_ctx("- aa\n- bb\n\nmiddle\n");
    let ln = find_line_containing(&ctx, "aa");
    let pos = ctx.get_line_text(ln).find("aa").unwrap() + 1;
    set_caret_at_line_char(&mut ctx, ln, pos);
    assert_action_with_undo_redo(&mut ctx, &Action::enter(false), "- a\n- a\n- bb\n\nmiddle\n");
}

#[test]
fn list_cross_two_items_delete() {
    let mut ctx = md_ctx("- aa\n- bb\n\nmiddle\n");
    let la = find_line_containing(&ctx, "aa");
    let lb = find_line_containing(&ctx, "bb");
    set_selection_lines_chars(&mut ctx, (la, 3), (lb, 3));
    assert_action_with_undo_redo(&mut ctx, &Action::delete(), "- ab\n\nmiddle\n");
}

#[test]
fn list_cross_two_items_insert_text() {
    let mut ctx = md_ctx("- aa\n- bb\n\nmiddle\n");
    let la = find_line_containing(&ctx, "aa");
    let lb = find_line_containing(&ctx, "bb");
    set_selection_lines_chars(&mut ctx, (la, 3), (lb, 3));
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("Z".into()), "- aZb\n\nmiddle\n");
}

#[test]
fn list_line_insert_tab_inside_item_text() {
    let mut ctx = md_ctx("- aa\n- bb\n");
    let ln = find_line_containing(&ctx, "aa");
    let pos = ctx.get_line_text(ln).find("aa").unwrap() + 1;
    set_caret_at_line_char(&mut ctx, ln, pos);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_tab(), "- a\ta\n- bb\n");
}

#[test]
fn list_ordered_second_line_delete_char() {
    let mut ctx = md_ctx("- aa\n\n1. ox\n1. oy\n");
    let ln = find_line_containing(&ctx, "oy");
    set_caret_at_line_char(&mut ctx, ln, 3);
    assert_action_with_undo_redo(&mut ctx, &Action::delete(), "- aa\n\n1. ox\n1. y\n");
}
