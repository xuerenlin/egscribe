//! 简单 action 用例聚合。
//! 复杂场景（如回车、粘贴、删除等）仍保留在各自文件。

mod common;

use common::*;
use egscribe::medit::{Action, PghType};
use std::collections::HashMap;

fn heading_action(level: u8) -> Action {
    let cmd = format!("heading_{level}");
    Action::new(cmd, HashMap::new())
}

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
    let la = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains('a'))
        .unwrap();
    let lb = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains('b'))
        .unwrap();
    set_selection_lines_chars(&mut ctx, (la, 0), (lb, 1));
    assert_action_with_undo_redo(&mut ctx, &Action::link(), "[a](url)\n\n[b](url)");
}

#[test]
fn italic_wraps_selection() {
    let mut ctx = md_ctx("word");
    set_selection_at_line_chars(&mut ctx, 0, 0, 4);
    assert_action_with_undo_redo(&mut ctx, &Action::italic(), "*word*");
}

#[test]
fn italic_cross_line_selection() {
    let mut ctx = md_ctx("x\n\ny");
    let a = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains('x'))
        .unwrap();
    let b = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains('y'))
        .unwrap();
    set_selection_lines_chars(&mut ctx, (a, 0), (b, 1));
    assert_action_with_undo_redo(&mut ctx, &Action::italic(), "*x*\n\n*y*");
}

#[test]
fn bold_wraps_selection() {
    let mut ctx = md_ctx("word");
    set_selection_at_line_chars(&mut ctx, 0, 0, 4);
    assert_action_with_undo_redo(&mut ctx, &Action::bold(), "**word**");
}

#[test]
fn bold_no_selection_inserts_markers_and_moves_caret() {
    let mut ctx = md_ctx("x");
    set_caret_at_line_char(&mut ctx, 0, 0);
    // 首次执行校验文档和光标；redo 时只校验文档，避免光标差异导致不稳定。
    execute_action(&mut ctx, &Action::bold());
    assert_doc(&ctx, "****x");
    assert_caret_at_line_char(&ctx, 0, 2);
    execute_action(&mut ctx, &Action::undo());
    assert_doc(&ctx, "x");
    execute_action(&mut ctx, &Action::redo());
    assert_doc(&ctx, "****x");
}

#[test]
fn bold_toggles_off_when_wrapped() {
    let mut ctx = md_ctx("**t**");
    set_selection_at_line_chars(&mut ctx, 0, 0, 5);
    assert_action_with_undo_redo(&mut ctx, &Action::bold(), "t");
}

#[test]
fn bold_whitespace_only_selection_inserts_markers() {
    let mut ctx = md_ctx("   ");
    set_selection_at_line_chars(&mut ctx, 0, 0, 3);
    assert_action_with_undo_redo(&mut ctx, &Action::bold(), "****");
}

#[test]
fn bold_cross_line_two_paragraphs() {
    let mut ctx = md_ctx("aa\nbb");
    let la = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("aa"))
        .unwrap();
    let lb = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("bb"))
        .unwrap();
    set_selection_lines_chars(&mut ctx, (la, 0), (lb, 2));
    assert_action_with_undo_redo(&mut ctx, &Action::bold(), "**aa**\n**bb**");
}

#[test]
fn code_block_wraps_current_line_when_no_selection() {
    let mut ctx = md_ctx("line");
    set_caret_at_line_char(&mut ctx, 0, 2);
    assert_action_with_undo_redo(&mut ctx, &Action::code_block(), "```\nline\n```");
}

#[test]
fn code_block_noop_when_selection_touches_code_row() {
    let mut ctx = md_ctx("```\nbody\n```");
    let code_line = (0..ctx.line_num())
        .find(|&i| ctx.is_line_type(i, PghType::CodeRow))
        .expect("fenced code should contain a CodeRow line");
    let before = ctx.get_all_text();
    set_selection_at_line_chars(&mut ctx, code_line, 0, 1);
    execute_action(&mut ctx, &Action::code_block());
    assert_eq!(ctx.get_all_text(), before);
}

#[test]
fn code_block_multiline_paragraph_selection() {
    let mut ctx = md_ctx("one\n\ntwo\n\nthree");
    let a = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("one"))
        .unwrap();
    let c = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("three"))
        .unwrap();
    let end = ctx.get_line_text(c).chars().count();
    set_selection_lines_chars(&mut ctx, (a, 0), (c, end));
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::code_block(),
        "```\none\n\ntwo\n\nthree\n```",
    );
}

#[test]
fn code_block_empty_line_no_selection() {
    let mut ctx = md_ctx("");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::code_block(), "```\n\n```");
}

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

#[test]
fn heading_1_through_6_inserts_hashes() {
    for level in 1u8..=6 {
        let mut ctx = md_ctx("Title");
        set_caret_at_line_char(&mut ctx, 0, 0);
        let expected = format!("{} Title", "#".repeat(level as usize));
        assert_action_with_undo_redo(&mut ctx, &heading_action(level), &expected);
    }
}

#[test]
fn heading_multiline_selection_both_prefixed() {
    let mut ctx = md_ctx("line1\n\nline2");
    let a = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("line1"))
        .unwrap();
    let b = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("line2"))
        .unwrap();
    let eb = ctx.get_line_text(b).chars().count();
    set_selection_lines_chars(&mut ctx, (a, 0), (b, eb));
    assert_action_with_undo_redo(&mut ctx, &heading_action(2), "## line1\n## \n## line2");
}

#[test]
fn heading_change_level_on_already_heading_line() {
    let mut ctx = md_ctx("## Sub");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &heading_action(1), "# Sub");
}

#[test]
fn horizontal_rule_inserts_rule_line() {
    let mut ctx = md_ctx("hello");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::horizontal_rule(), "hello\n\n---");
}

#[test]
fn horizontal_rule_mid_paragraph() {
    let mut ctx = md_ctx("before after");
    set_caret_at_line_char(&mut ctx, 0, 6);
    assert_action_with_undo_redo(&mut ctx, &Action::horizontal_rule(), "before\n\n---\nafter");
}

#[test]
fn strikethrough_wraps_selection() {
    let mut ctx = md_ctx("word");
    set_selection_at_line_chars(&mut ctx, 0, 0, 4);
    assert_action_with_undo_redo(&mut ctx, &Action::strikethrough(), "~~word~~");
}

#[test]
fn strikethrough_cross_line() {
    let mut ctx = md_ctx("m\n\nn");
    let a = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains('m'))
        .unwrap();
    let b = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains('n'))
        .unwrap();
    set_selection_lines_chars(&mut ctx, (a, 0), (b, 1));
    assert_action_with_undo_redo(&mut ctx, &Action::strikethrough(), "~~m~~\n\n~~n~~");
}

#[test]
fn table_inserts_table_skeleton() {
    let mut ctx = md_ctx("");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::table(),
        "|ColA|ColB|ColC|\n|--|--|--|\n||||",
    );
}

#[test]
fn table_on_nonempty_line_appends_below() {
    let mut ctx = md_ctx("intro");
    set_caret_at_line_char(&mut ctx, 0, 2);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::table(),
        "intro\n|ColA|ColB|ColC|\n|--|--|--|\n||||",
    );
}

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

#[test]
fn unordered_list_inserts_bullet() {
    let mut ctx = md_ctx("item");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::unordered_list(), "- item");
}

#[test]
fn unordered_list_multiline_selection() {
    let mut ctx = md_ctx("a\n\nb");
    let la = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains('a'))
        .unwrap();
    let lb = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains('b'))
        .unwrap();
    set_selection_lines_chars(&mut ctx, (la, 0), (lb, 1));
    assert_action_with_undo_redo(&mut ctx, &Action::unordered_list(), "- a\n- \n- b");
}

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

#[test]
fn undo_redo_roundtrip_insert_text() {
    let mut ctx = md_ctx("a");
    set_caret_at_line_char(&mut ctx, 0, 1);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("b".into()), "ab");
}

#[test]
fn undo_redo_chain_multiple_inserts_then_redo() {
    let mut ctx = md_ctx("x");
    set_caret_at_line_char(&mut ctx, 0, 1);
    for ch in ["a", "b", "c"] {
        execute_action(&mut ctx, &Action::insert_text(ch.into()));
    }
    assert_doc(&ctx, "xabc");
    execute_action(&mut ctx, &Action::undo());
    execute_action(&mut ctx, &Action::undo());
    execute_action(&mut ctx, &Action::undo());
    assert_doc(&ctx, "x");
    execute_action(&mut ctx, &Action::redo());
    execute_action(&mut ctx, &Action::redo());
    execute_action(&mut ctx, &Action::redo());
    assert_doc(&ctx, "xabc");
}
