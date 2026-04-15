mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn horizontal_rule_inserts_rule_line() {
    // 行首插水平线：正文保留，下一行为 `---`；整篇 `get_all_text` 为逐行拼接，末尾不一定有换行
    let mut ctx = md_ctx("hello");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::horizontal_rule(), "hello\n\n---");
}

#[test]
fn horizontal_rule_mid_paragraph() {
    // 段中插入：`---` 单独成行；重解析前可能为 `Text`，整篇导出时 `select` 与 `BreakLine` 一样带前导 `\n` 以满足 GFM
    let mut ctx = md_ctx("before after");
    set_caret_at_line_char(&mut ctx, 0, 6);
    assert_action_with_undo_redo(&mut ctx, &Action::horizontal_rule(), "before\n\n---\nafter");
}
