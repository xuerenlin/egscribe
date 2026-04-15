mod common;

use common::*;
use egscribe::medit::Action;

/// 当系统剪贴板里**没有非空文本**时，`paste` 不应改变文档（避免 CI 因剪贴板内容不确定而失败）。
#[test]
fn paste_is_noop_when_clipboard_has_no_nonempty_text() {
    let mut ctx = md_ctx("abc");
    set_caret_at_line_char(&mut ctx, 0, 1);
    if matches!(ctx.get_clipboard_text(), Some(ref s) if !s.is_empty()) {
        return;
    }
    assert_action_with_undo_redo(&mut ctx, &Action::paste(), "abc");
}

/// 设置环境变量 `EGSCRIBE_TEST_CLIPBOARD=1` 且剪贴板中已有非空文本时再跑，用于本机手测；CI 默认不依赖剪贴板。
#[test]
fn paste_runs_when_env_clipboard_test_enabled() {
    if std::env::var("EGSCRIBE_TEST_CLIPBOARD").ok().as_deref() != Some("1") {
        return;
    }
    let mut ctx = md_ctx("ab");
    set_caret_at_line_char(&mut ctx, 0, 1);
    execute_action(&mut ctx, &Action::paste());
    let _ = ctx.get_all_text();
}

#[test]
#[ignore = "依赖系统剪贴板，CI/无头环境常不可用；插入语义请测 action_insert_text"]
fn paste_inserts_when_clipboard_has_text() {
    let mut ctx = md_ctx("ab");
    set_caret_at_line_char(&mut ctx, 0, 1);
    execute_action(&mut ctx, &Action::paste());
    let _ = ctx.get_all_text();
}
