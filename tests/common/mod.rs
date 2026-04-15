//! medit 集成测试共用夹具：`Ctx` + `egui::Ui` 上执行 `Action::execute`。
//!
//! **Undo/Redo**：单步操作用 [`assert_action_with_undo_redo`]（默认只比对 `get_all_text`）；
//! 需要同时锁定光标语义时用 [`assert_action_with_undo_redo_and_cursors`]。

#![allow(dead_code)]

use egscribe::medit::{Action, Cursor, Ctx};

/// Markdown 模式下的编辑器上下文。
pub fn md_ctx(text: &str) -> Ctx {
    Ctx::new().with_text(text, true)
}

/// 纯文本模式下的编辑器上下文。
pub fn plain_ctx(text: &str) -> Ctx {
    Ctx::new().with_text(text, false)
}

/// 低层构造：`(line_no, segment, culumn)`。新用例请优先用 [`cursor_at_line_char`]，以免 segment 结构变化导致用例大面积失效。
pub fn cursor(line_no: usize, segment: usize, culumn: usize) -> Cursor {
    Cursor::from((line_no, segment, culumn))
}

/// 按「行号 + 该行 [`Ctx::get_line_text`] 的逻辑字符下标」得到光标（与 `PghView::text_char_index_to_cursor` 一致，自动跨过前导 Icon 等 segment）。
///
/// `char_index` 为 Unicode 标量字符计数；可等于该行字符数以表示行尾。越界行号会 panic。
pub fn cursor_at_line_char(ctx: &Ctx, line_no: usize, char_index: usize) -> Cursor {
    let line = ctx.get_line(line_no).unwrap_or_else(|| {
        panic!(
            "cursor_at_line_char: line {line_no} out of range (line_num={})",
            ctx.line_num()
        )
    });
    let max_chars = line.get_text().chars().count();
    let idx = char_index.min(max_chars);
    let raw = line.text_char_index_to_cursor(idx, line_no);
    ctx.cursor_check(&raw)
}

/// 将主光标置于「指定行、该行逻辑字符下标」处。
pub fn set_caret_at_line_char(ctx: &mut Ctx, line_no: usize, char_index: usize) {
    let c = cursor_at_line_char(ctx, line_no, char_index);
    ctx.set_cursor2(c);
    ctx.set_cursor1_reset();
}

/// 将主光标置于 `segment == 0`、`culumn == col`（经 [`Ctx::cursor_check`]）。
///
/// 与 [`set_caret_at_line_char`] 不同：后者按 `get_line_text()` 的**逻辑字符**映射到各 segment，适合普通段落；
/// 某些 `TableRow` 等行在内部把管道符拆进多段，`text_char_index_to_cursor` 与「在整行字符串上的列号」不一致，
/// 此时若你掌握的是「与旧 UI 一致的 segment0 列号」，可用本函数（列号通常与 ASCII 下 `get_line_text()` 的 byte 列一致）。
pub fn set_caret_line_segment0(ctx: &mut Ctx, line_no: usize, col: usize) {
    let c = ctx.cursor_check(&cursor(line_no, 0, col));
    ctx.set_cursor2(c);
    ctx.set_cursor1_reset();
}

/// 折叠选区：锚点在 `c2`，与 UI 中常见「仅移动插入点」一致。
pub fn set_caret(ctx: &mut Ctx, c: Cursor) {
    ctx.set_cursor2(c);
    ctx.set_cursor1_reset();
}

/// 同一行内按逻辑字符下标设置选区（`char_start` / `char_end` 可任意顺序）。
pub fn set_selection_at_line_chars(ctx: &mut Ctx, line_no: usize, char_start: usize, char_end: usize) {
    let lo = char_start.min(char_end);
    let hi = char_start.max(char_end);
    let c1 = cursor_at_line_char(ctx, line_no, lo);
    let c2 = cursor_at_line_char(ctx, line_no, hi);
    ctx.set_cursor1(c1);
    ctx.set_cursor2(c2);
}

/// 跨行选区：锚点 `(line, char_index)` 使用与 [`cursor_at_line_char`] 相同的逻辑字符下标；两端可任意顺序。
pub fn set_selection_lines_chars(
    ctx: &mut Ctx,
    anchor_a: (usize, usize),
    anchor_b: (usize, usize),
) {
    let c_a = cursor_at_line_char(ctx, anchor_a.0, anchor_a.1);
    let c_b = cursor_at_line_char(ctx, anchor_b.0, anchor_b.1);
    if c_a <= c_b {
        ctx.set_cursor1(c_a);
        ctx.set_cursor2(c_b);
    } else {
        ctx.set_cursor1(c_b);
        ctx.set_cursor2(c_a);
    }
}

/// 同一行内用 `segment == 0` 与列号设选区（与 [`set_caret_line_segment0`] 语义一致，适合 `TableRow`）。
pub fn set_selection_line_segment0(
    ctx: &mut Ctx,
    line_no: usize,
    col_start: usize,
    col_end: usize,
) {
    let lo = col_start.min(col_end);
    let hi = col_start.max(col_end);
    let c1 = ctx.cursor_check(&cursor(line_no, 0, lo));
    let c2 = ctx.cursor_check(&cursor(line_no, 0, hi));
    if c1 <= c2 {
        ctx.set_cursor1(c1);
        ctx.set_cursor2(c2);
    } else {
        ctx.set_cursor1(c2);
        ctx.set_cursor2(c1);
    }
}

/// 跨行选区：`segment0` + 列号（表格等）。
pub fn set_selection_lines_segment0(
    ctx: &mut Ctx,
    anchor_a: (usize, usize),
    anchor_b: (usize, usize),
) {
    let c_a = ctx.cursor_check(&cursor(anchor_a.0, 0, anchor_a.1));
    let c_b = ctx.cursor_check(&cursor(anchor_b.0, 0, anchor_b.1));
    if c_a <= c_b {
        ctx.set_cursor1(c_a);
        ctx.set_cursor2(c_b);
    } else {
        ctx.set_cursor1(c_b);
        ctx.set_cursor2(c_a);
    }
}

/// 设置选区（`cursor1` 与 `cursor2` 可任意顺序）。
pub fn set_selection(ctx: &mut Ctx, c1: Cursor, c2: Cursor) {
    ctx.set_cursor1(c1);
    ctx.set_cursor2(c2);
}

/// 在解析后的 GFM 表格中查找**数据行**（排除仅由 `|`、`-` 与空白构成的分隔行），返回 `(行号, 该行 [`Ctx::get_line_text`] 的字符数)`。
pub fn find_table_data_row_line_last_col(ctx: &Ctx, needle: char) -> (usize, usize) {
    (0..ctx.line_num())
        .find_map(|i| {
            let t = ctx.get_line_text(i);
            if !t.contains(needle) {
                return None;
            }
            if t.chars().all(|c: char| c == '|' || c == '-' || c.is_whitespace()) {
                return None;
            }
            Some((i, t.chars().count()))
        })
        .expect("table data row")
}

/// 返回 [`Ctx::get_line_text`] 含 `needle` 的首个行号。
pub fn find_line_containing(ctx: &Ctx, needle: &str) -> usize {
    (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains(needle))
        .unwrap_or_else(|| panic!("no line containing {needle:?}"))
}

/// 在 egui 一帧内执行闭包（提供真实 `Ui`，供 `cut` 等需要 `ui.ctx()` 的 action）。
///
/// 若 egui 触发多 pass，闭包仅在首次进入面板时执行一次，避免重复 `execute`。
pub fn run_with_ui(ctx: &mut Ctx, f: impl FnOnce(&mut Ctx, &mut egui::Ui)) {
    let egui_ctx = egui::Context::default();
    let mut f = Some(f);
    let mut ran = false;
    let _ = egui_ctx.run(egui::RawInput::default(), |c| {
        egui::CentralPanel::default().show(c, |ui| {
            if !ran {
                if let Some(ff) = f.take() {
                    ff(ctx, ui);
                }
                ran = true;
            }
        });
    });
}

/// 对 `medit_ctx` 执行 `action.execute`（带 egui）。
pub fn execute_action(ctx: &mut Ctx, action: &Action) {
    let action = action.clone();
    run_with_ui(ctx, move |ctx, ui| {
        action.execute(ctx, ui);
    });
}

/// 执行 `action`，断言操作后文档为 `expected_after`，再 **undo → redo** 仅校验 **`get_all_text()`**
///（避免部分命令在 undo 后未完全还原 `cursor1`/`cursor2` 的实现差异导致用例不稳定）。
///
/// 用于各 action 集成测试，避免手写 undo/redo 样板。
pub fn assert_action_with_undo_redo(ctx: &mut Ctx, action: &Action, expected_after: &str) {
    let before_doc = ctx.get_all_text();
    execute_action(ctx, action);
    assert_eq!(
        ctx.get_all_text(),
        expected_after,
        "操作后文档与期望不一致"
    );
    execute_action(ctx, &Action::undo());
    assert_eq!(
        ctx.get_all_text(),
        before_doc,
        "undo 后文档应恢复为操作前"
    );
    execute_action(ctx, &Action::redo());
    assert_eq!(
        ctx.get_all_text(),
        expected_after,
        "redo 后文档应回到操作后"
    );
}

/// 与 [`assert_action_with_undo_redo`] 相同，并校验 undo/redo 前后 **cursor1、cursor2** 与操作前/操作后一致（更严格，适合光标语义已对齐的命令）。
pub fn assert_action_with_undo_redo_and_cursors(
    ctx: &mut Ctx,
    action: &Action,
    expected_after: &str,
) {
    let before_doc = ctx.get_all_text();
    let before_c1 = ctx.cursor1();
    let before_c2 = ctx.cursor2();
    execute_action(ctx, action);
    assert_eq!(
        ctx.get_all_text(),
        expected_after,
        "操作后文档与期望不一致"
    );
    let after_c1 = ctx.cursor1();
    let after_c2 = ctx.cursor2();
    execute_action(ctx, &Action::undo());
    assert_eq!(
        ctx.get_all_text(),
        before_doc,
        "undo 后文档应恢复为操作前"
    );
    assert_eq!(ctx.cursor1(), before_c1, "undo 后 cursor1 应恢复");
    assert_eq!(ctx.cursor2(), before_c2, "undo 后 cursor2 应恢复");
    execute_action(ctx, &Action::redo());
    assert_eq!(
        ctx.get_all_text(),
        expected_after,
        "redo 后文档应回到操作后"
    );
    assert_eq!(ctx.cursor1(), after_c1, "redo 后 cursor1 应与操作后一致");
    assert_eq!(ctx.cursor2(), after_c2, "redo 后 cursor2 应与操作后一致");
}

/// 兼容旧名：与 [`assert_action_with_undo_redo`] 相同（仅文档）。
pub fn assert_action_with_undo_redo_doc_only(
    ctx: &mut Ctx,
    action: &Action,
    expected_after: &str,
) {
    assert_action_with_undo_redo(ctx, action, expected_after);
}

/// 断言全文与 `get_all_text()` 序列化结果一致。
pub fn assert_doc(ctx: &Ctx, expected: &str) {
    assert_eq!(ctx.get_all_text(), expected, "get_all_text mismatch");
}

/// 断言主光标 `cursor2`。
pub fn assert_caret(ctx: &Ctx, line_no: usize, segment: usize, culumn: usize) {
    let c = ctx.cursor2();
    assert_eq!(
        (c.line_no, c.segment, c.culumn),
        (line_no, segment, culumn),
        "cursor2 mismatch"
    );
}

/// 断言主光标落在「指定行、该行逻辑字符下标」处（与 [`cursor_at_line_char`] 语义一致）。
pub fn assert_caret_at_line_char(ctx: &Ctx, line_no: usize, char_index: usize) {
    let line = ctx.get_line(line_no).expect("assert_caret_at_line_char: line in range");
    let got = ctx.cursor2();
    let idx = line.cursor_to_text_char_index(&got);
    assert_eq!(
        idx, char_index,
        "cursor2 expected at logical char {char_index} on line {line_no}, got index {idx} (cursor={got:?})"
    );
}
