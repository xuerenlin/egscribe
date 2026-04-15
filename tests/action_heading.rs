mod common;

use common::*;
use egscribe::medit::Action;
use std::collections::HashMap;

fn heading_action(level: u8) -> Action {
    let cmd = format!("heading_{level}");
    Action::new(cmd, HashMap::new())
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
