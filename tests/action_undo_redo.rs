//! 专测「多步 undo 链」等；单步 action 的 undo/redo 见 [`common::assert_action_with_undo_redo`]，
//! 已并入各 `action_*.rs` 用例。

mod common;

use common::*;
use egscribe::medit::Action;

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
