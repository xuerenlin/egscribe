mod common;

use common::*;
use egscribe::medit::Action;

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
