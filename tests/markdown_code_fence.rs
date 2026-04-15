//! Fenced code 段落专项（与 `action_code_block` 的块级动作互补；`action_enter` 已有单行 body 的 Enter）。
//!
//! `#[test]` 索引：
//! - `fence_multiline_second_line_start_backspace_no_op`（多行 CodeRow 行首 backspace 当前不合并行）
//! - `fence_multiline_first_line_end_delete_no_op`（行尾 delete 不跨行合并）
//! - `fence_body_mid_line_backspace_no_selection`
//! - `fence_body_second_line_inner_backspace`
//! - `fence_body_cross_two_lines_delete`
//! - `fence_body_cross_two_lines_insert_text`
//! - `fence_body_partial_selection_code_block_noop`（与 `action_code_block::code_block_noop_when_selection_touches_code_row` 同路径，多行 body 视角）

mod common;

use common::*;
use egscribe::medit::{Action, Ctx, PghType};

fn first_code_body_line(ctx: &Ctx, needle: &str) -> usize {
    (0..ctx.line_num())
        .find(|&i| ctx.is_line_type(i, PghType::CodeRow) && ctx.get_line_text(i).contains(needle))
        .unwrap_or_else(|| panic!("no CodeRow containing {needle:?}"))
}

#[test]
fn fence_multiline_second_line_start_backspace_no_op() {
    let mut ctx = md_ctx("```rust\naa\nbb\ncc\n```");
    let before = ctx.get_all_text();
    let ln = first_code_body_line(&ctx, "bb");
    set_caret_at_line_char(&mut ctx, ln, 0);
    execute_action(&mut ctx, &Action::backspace());
    assert_eq!(ctx.get_all_text(), before, "多行 fenced body 行首 backspace 当前不改变导出串");
}

#[test]
fn fence_multiline_first_line_end_delete_no_op() {
    let mut ctx = md_ctx("```rust\naa\nbb\ncc\n```");
    let before = ctx.get_all_text();
    let ln = first_code_body_line(&ctx, "aa");
    let end = ctx.get_line_text(ln).chars().count();
    set_caret_at_line_char(&mut ctx, ln, end);
    execute_action(&mut ctx, &Action::delete());
    assert_eq!(ctx.get_all_text(), before, "多行 fenced body 行尾 delete 当前不吞掉下一行行首");
}

#[test]
fn fence_body_mid_line_backspace_no_selection() {
    let mut ctx = md_ctx("```rust\naa\nbb\n```");
    let ln = first_code_body_line(&ctx, "aa");
    set_caret_at_line_char(&mut ctx, ln, 1);
    assert_action_with_undo_redo(&mut ctx, &Action::backspace(), "```rust\na\nbb\n```");
}

#[test]
fn fence_body_second_line_inner_backspace() {
    let mut ctx = md_ctx("```rust\naa\nbb\ncc\n```");
    let ln = first_code_body_line(&ctx, "bb");
    set_caret_at_line_char(&mut ctx, ln, 1);
    assert_action_with_undo_redo(&mut ctx, &Action::backspace(), "```rust\naa\nb\ncc\n```");
}

#[test]
fn fence_body_cross_two_lines_delete() {
    let mut ctx = md_ctx("```rust\naa\nbb\ncc\n```");
    let la = first_code_body_line(&ctx, "aa");
    let lb = first_code_body_line(&ctx, "bb");
    let end_b = ctx.get_line_text(lb).chars().count();
    set_selection_lines_chars(&mut ctx, (la, 1), (lb, end_b));
    assert_action_with_undo_redo(&mut ctx, &Action::delete(), "```rust\na\ncc\n```");
}

#[test]
fn fence_body_cross_two_lines_insert_text() {
    let mut ctx = md_ctx("```rust\naa\nbb\ncc\n```");
    let la = first_code_body_line(&ctx, "aa");
    let lb = first_code_body_line(&ctx, "bb");
    let end_b = ctx.get_line_text(lb).chars().count();
    set_selection_lines_chars(&mut ctx, (la, 1), (lb, end_b));
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("Z".into()), "```rust\naZ\ncc\n```");
}

#[test]
fn fence_body_partial_selection_code_block_noop() {
    let mut ctx = md_ctx("```rust\naa\nbb\ncc\n```");
    let la = first_code_body_line(&ctx, "aa");
    let lb = first_code_body_line(&ctx, "bb");
    set_selection_lines_chars(&mut ctx, (la, 0), (lb, 1));
    let before = ctx.get_all_text();
    execute_action(&mut ctx, &Action::code_block());
    assert_eq!(ctx.get_all_text(), before);
}
