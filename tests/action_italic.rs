mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn italic_wraps_selection() {
    let mut ctx = md_ctx("word");
    set_selection_at_line_chars(&mut ctx, 0, 0, 4);
    assert_action_with_undo_redo(&mut ctx, &Action::italic(), "*word*");
}

#[test]
fn italic_cross_line_selection() {
    let mut ctx = md_ctx("x\n\ny");
    let a = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains('x')).unwrap();
    let b = (0..ctx.line_num()).find(|&i| ctx.get_line_text(i).contains('y')).unwrap();
    set_selection_lines_chars(&mut ctx, (a, 0), (b, 1));
    // 源码中 `x` 与 `y` 之间有空行，斜体后仍保留空行
    assert_action_with_undo_redo(&mut ctx, &Action::italic(), "*x*\n\n*y*");
}
