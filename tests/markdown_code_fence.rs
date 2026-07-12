//! Fenced code 段落专项（与 `action_code_block` 的块级动作互补；`action_enter` 已有单行 body 的 Enter）。
//!
//! `#[test]` 索引：
//! - `fence_multiline_second_line_start_backspace_merges_with_previous`（body 第二行行首 backspace 与上一行合并）
//! - `fence_multiline_first_line_end_delete_merges_with_next`（body 第一行行尾 delete 与下一行合并）
//! - `fence_body_mid_line_backspace_no_selection`
//! - `fence_body_second_line_inner_backspace`
//! - `fence_body_cross_two_lines_delete`
//! - `fence_body_cross_two_lines_insert_text`
//! - `fence_body_partial_selection_code_block_noop`（与 `action_code_block::code_block_noop_when_selection_touches_code_row` 同路径，多行 body 视角）
//! - `fence_body_mid_line_ctrl_enter_inserts_blank_below_block`（块内 Ctrl+Enter 在围栏下方插入空行）
//! - `empty_code_block_at_document_start_backspace_removes_block`（文档首行单行空 code block 行首 backspace 删除整块）
//! - `multi_empty_line_code_block_at_document_start_backspace_noop`（多行空行 code block 行首 backspace 不删块）
//!
//! 复制与围栏（`get_text_by_cursor_range`）：
//! - `full_block_lines_yields_fenced_markdown_even_when_columns_not_full_line`
//! - `full_block_with_end_column_max_still_fenced`
//! - `partial_lines_inside_block_has_no_fenced_wrapper_in_plain_copy`
//! - `selection_spanning_intro_and_code_inserts_fenced_block_after_intro`
//! - `selection_including_text_before_and_after_code_keeps_fence`

mod common;

use common::*;
use egscribe::medit::{Action, Ctx, Cursor, PghType};

#[test]
fn empty_code_block_at_document_start_backspace_removes_block() {
    let mut ctx = md_ctx("```\n```");
    assert!(ctx.is_line_type(0, PghType::CodeRow));
    let (blk_s, blk_e) = ctx.code_row_block_range(0).expect("code block");
    assert_eq!(blk_s, blk_e, "fixture should be a single-row empty code block");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::backspace(), "");
}

#[test]
fn multi_empty_line_code_block_at_document_start_backspace_noop() {
    let mut ctx = md_ctx("");
    set_caret_at_line_char(&mut ctx, 0, 0);
    execute_action(&mut ctx, &Action::code_block());
    assert_eq!(ctx.get_all_text(), "```\n\n```");
    let (blk_s, mut blk_e) = ctx.code_row_block_range(0).expect("code block");
    if blk_s == blk_e {
        set_caret_at_line_char(&mut ctx, blk_s, 0);
        execute_action(&mut ctx, &Action::enter(false));
        blk_e = ctx.code_row_block_range(blk_s).expect("code block").1;
    }
    assert!(blk_e > blk_s, "fixture should be a multi-row empty code block");
    set_caret_at_line_char(&mut ctx, blk_s, 0);
    let before = ctx.get_all_text();
    execute_action(&mut ctx, &Action::backspace());
    assert_eq!(ctx.get_all_text(), before);
}

fn first_code_body_line(ctx: &Ctx, needle: &str) -> usize {
    (0..ctx.line_num())
        .find(|&i| ctx.is_line_type(i, PghType::CodeRow) && ctx.get_line_text(i).contains(needle))
        .unwrap_or_else(|| panic!("no CodeRow containing {needle:?}"))
}

#[test]
fn fence_multiline_second_line_start_backspace_merges_with_previous() {
    let mut ctx = md_ctx("```rust\naa\nbb\ncc\n```");
    let ln = first_code_body_line(&ctx, "bb");
    set_caret_at_line_char(&mut ctx, ln, 0);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::backspace(),
        "```rust\naabb\ncc\n```",
    );
}

#[test]
fn fence_multiline_first_line_end_delete_merges_with_next() {
    let mut ctx = md_ctx("```rust\naa\nbb\ncc\n```");
    let ln = first_code_body_line(&ctx, "aa");
    let end = ctx.get_line_text(ln).chars().count();
    set_caret_at_line_char(&mut ctx, ln, end);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::delete(),
        "```rust\naabb\ncc\n```",
    );
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
fn fence_body_mid_line_ctrl_enter_inserts_blank_below_block() {
    let mut ctx = md_ctx("```rust\naa\nbb\ncc\n```");
    let ln = first_code_body_line(&ctx, "bb");
    set_caret_at_line_char(&mut ctx, ln, 1);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::enter(true),
        "```rust\naa\nbb\ncc\n```\n",
    );
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

fn fenced_rust_ctx() -> (Ctx, usize, usize) {
    let md = "\n\n```rust\nfn a() {}\nfn b() {}\n```\n";
    let ctx = Ctx::new().with_text(md, true);
    let first_code = (0..ctx.line_num())
        .find(|&i| ctx.get_line(i).is_some_and(|p| p.is_code_row()))
        .expect("expected at least one CodeRow");
    let (blk_s, blk_e) = ctx
        .code_row_block_range(first_code)
        .expect("code_row_block_range");
    assert!(
        blk_e > blk_s,
        "test doc should yield multi-line CodeRow block"
    );
    (ctx, blk_s, blk_e)
}

#[test]
fn full_block_lines_yields_fenced_markdown_even_when_columns_not_full_line() {
    let (ctx, blk_s, blk_e) = fenced_rust_ctx();
    let end_norm = ctx.get_line(blk_e).unwrap().end_cursor_of_line(blk_e);
    let c_lo: Cursor = (blk_s, 0, 1).into();
    let c_hi: Cursor = (
        blk_e,
        end_norm.segment,
        end_norm.culumn.saturating_sub(1).max(1),
    )
        .into();
    let fenced = ctx.get_text_by_cursor_range(c_lo, c_hi);
    assert!(
        fenced.starts_with("```rust\n"),
        "missing opening fence: {:?}",
        fenced
    );
    assert!(
        fenced.trim_end().ends_with("```"),
        "missing closing fence: {:?}",
        fenced
    );
    assert!(
        fenced.contains("fn a()") && fenced.contains("fn b()"),
        "body should be full block: {:?}",
        fenced
    );
}

#[test]
fn full_block_with_end_column_max_still_fenced() {
    let (ctx, blk_s, blk_e) = fenced_rust_ctx();
    let mut c_end = ctx.get_line(blk_e).unwrap().end_cursor_of_line(blk_e);
    c_end.culumn = usize::MAX;
    let c_start: Cursor = (blk_s, 0, 0).into();
    let fenced = ctx.get_text_by_cursor_range(c_start, c_end);
    assert!(fenced.starts_with("```rust\n"), "{:?}", fenced);
}

#[test]
fn partial_lines_inside_block_has_no_fenced_wrapper_in_plain_copy() {
    let (ctx, blk_s, blk_e) = fenced_rust_ctx();
    assert!(blk_e > blk_s);
    let end_first = ctx.get_line(blk_s).unwrap().end_cursor_of_line(blk_s);
    let c1: Cursor = (blk_s, 0, 0).into();
    let out = ctx.get_text_by_cursor_range(c1, end_first);
    assert!(
        !out.starts_with("```rust"),
        "partial block selection should not get fenced wrapper: {:?}",
        out
    );
    assert!(out.contains("fn a()"), "{:?}", out);
}

#[test]
fn selection_spanning_intro_and_code_inserts_fenced_block_after_intro() {
    let md = "intro line\n\n```rust\nx\n```\n";
    let ctx = Ctx::new().with_text(md, true);
    let first_code = (0..ctx.line_num())
        .find(|&i| ctx.get_line(i).is_some_and(|p| p.is_code_row()))
        .unwrap();
    let (_blk_s, blk_e) = ctx.code_row_block_range(first_code).unwrap();
    let c_start: Cursor = (0usize, 0, 0).into();
    let c_end = ctx.get_line(blk_e).unwrap().end_cursor_of_line(blk_e);
    let out = ctx.get_text_by_cursor_range(c_start, c_end);
    assert!(
        out.contains("intro"),
        "intro paragraph missing: {:?}",
        out
    );
    assert!(
        out.contains("```rust") && out.contains('x') && out.contains("```"),
        "middle code should stay fenced: {:?}",
        out
    );
    assert!(
        !out.starts_with("```rust"),
        "whole selection should not be fence-only: {:?}",
        out
    );
}

#[test]
fn selection_including_text_before_and_after_code_keeps_fence() {
    let md = "before\n\n```rust\na\nb\n```\n\nafter\n";
    let ctx = Ctx::new().with_text(md, true);
    let last = ctx.line_num().saturating_sub(1);
    let c_start: Cursor = (0usize, 0, 0).into();
    let c_end = ctx.get_line(last).unwrap().end_cursor_of_line(last);
    let out = ctx.get_text_by_cursor_range(c_start, c_end);
    assert!(out.contains("before"), "{:?}", out);
    assert!(out.contains("after"), "{:?}", out);
    assert!(
        out.contains("```rust") && out.contains('a') && out.contains('b'),
        "expected fenced rust block: {:?}",
        out
    );
}
