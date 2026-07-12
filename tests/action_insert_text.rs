mod common;

use common::*;
use egscribe::medit::Action;

#[test]
fn insert_text_non_markdown_plain() {
    let mut ctx = plain_ctx("ab");
    set_caret_at_line_char(&mut ctx, 0, 1);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("Z".into()), "aZb");
}

#[test]
fn insert_text_plain_paragraph_end() {
    let mut ctx = md_ctx("hello");
    set_caret_at_line_char(&mut ctx, 0, 5);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text(" world".into()), "hello world");
}

#[test]
fn insert_text_plain_paragraph_start() {
    let mut ctx = md_ctx("hello");
    set_caret_at_line_char(&mut ctx, 0, 0);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("Hi ".into()), "Hi hello");
}

#[test]
fn insert_text_replaces_selection() {
    let mut ctx = md_ctx("abcdef");
    set_selection_at_line_chars(&mut ctx, 0, 2, 4); // "cd"
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("ZZ".into()), "abZZef");
}

#[test]
fn insert_text_multiline_inserts_newlines() {
    let mut ctx = md_ctx("line1");
    set_caret_at_line_char(&mut ctx, 0, 5);
    // 插入 `\n` 后两段在 `get_all_text` 中按行拼接为单换行（不再在块之间额外补空行）
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("\nline2".into()), "line1\nline2");
}

#[test]
fn insert_text_read_only_noop() {
    let mut ctx = md_ctx("abc").read_only(true);
    set_caret_at_line_char(&mut ctx, 0, 1);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("X".into()), "abc");
}

#[test]
fn insert_text_inside_markdown_table_row() {
    let mut ctx = md_ctx("| a | b |\n| --- | --- |\n| x | y |");
    let (line_no, col) = (0..ctx.line_num())
        .find_map(|i| {
            let t = ctx.get_line_text(i);
            if !t.contains('x') {
                return None;
            }
            // 跳过分隔行（如 |---|---|），避免误匹配
            if t.chars().all(|c| c == '|' || c == '-' || c.is_whitespace()) {
                return None;
            }
            t.find('x').map(|byte_idx| {
                let char_idx = t[..byte_idx].chars().count();
                (i, char_idx)
            })
        })
        .expect("parsed table should contain cell text x");
    // TableRow：segment0 列号与整行 `get_line_text()` 字符索引对齐方式与 `text_char_index_to_cursor` 不同
    set_caret_line_segment0(&mut ctx, line_no, col);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("!".into()), "|a|b|\n|--|--|\n|x!|y|");
}

#[test]
fn insert_text_inside_fenced_code_body() {
    let mut ctx = md_ctx("```rust\nlet x = 1;\n```");
    let body_line = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("let x"))
        .expect("fenced code should contain body line");
    let line = ctx.get_line_text(body_line);
    let char_idx = line.find('x').expect("body should contain x");
    set_caret_at_line_char(&mut ctx, body_line, char_idx);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::insert_text(" // c".into()),
        "```rust\nlet  // cx = 1;\n```",
    );
}

#[test]
fn insert_text_cross_line_replaces_markdown_three_paragraphs() {
    // 双换行拆成多段，保证 `line_num` 上为三行逻辑段落
    let mut ctx = md_ctx("aa\n\nbb\n\ncc");
    assert!(
        ctx.line_num() >= 3,
        "expected >=3 pgh lines, got {}",
        ctx.line_num()
    );
    let la = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("aa"))
        .expect("line aa");
    let lc = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("cc"))
        .expect("line cc");
    set_selection_lines_chars(&mut ctx, (la, 1), (lc, 1));
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("Z".into()), "aZc");
}

#[test]
fn insert_text_cross_line_selection_order_invariant() {
    let mut ctx = md_ctx("11\n\n22\n\n33");
    assert!(ctx.line_num() >= 3, "lines={}", ctx.line_num());
    let l0 = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("11"))
        .unwrap();
    let l2 = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("33"))
        .unwrap();
    set_selection_lines_chars(&mut ctx, (l0, 1), (l2, 1));
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("X".into()), "1X3");
    let forward = ctx.get_all_text();

    let mut ctx2 = md_ctx("11\n\n22\n\n33");
    let l0 = (0..ctx2.line_num())
        .find(|&i| ctx2.get_line_text(i).contains("11"))
        .unwrap();
    let l2 = (0..ctx2.line_num())
        .find(|&i| ctx2.get_line_text(i).contains("33"))
        .unwrap();
    set_selection_lines_chars(&mut ctx2, (l2, 1), (l0, 1));
    execute_action(&mut ctx2, &Action::insert_text("X".into()));
    assert_eq!(ctx2.get_all_text(), forward, "anchor order should not matter");
}

#[test]
fn insert_text_at_line_end_char_index() {
    let mut ctx = md_ctx("hello");
    let end = ctx.get_line_text(0).chars().count();
    set_caret_at_line_char(&mut ctx, 0, end);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("!".into()), "hello!");
}

#[test]
fn insert_text_table_row_full_line_segment0_replace_with_empty() {
    let mut ctx = md_ctx("| a | b |\n| --- | --- |\n| x | y |");
    let (line_no, last_col) = (0..ctx.line_num())
        .find_map(|i| {
            let t = ctx.get_line_text(i);
            if !t.contains('x') {
                return None;
            }
            if t.chars().all(|c| c == '|' || c == '-' || c.is_whitespace()) {
                return None;
            }
            let n = t.chars().count();
            Some((i, n))
        })
        .expect("data row with x");
    set_selection_line_segment0(&mut ctx, line_no, 0, last_col);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("".into()), "|a|b|\n|--|--|\n||y|");
}

#[test]
fn insert_text_mixed_heading_todo_table_preserves_blocks() {
    let raw = "## Title\n\nmiddle line\n\n- [ ] todo\n\n| h |\n| - |\n| c |\n";
    let mut ctx = md_ctx(raw);
    let mid = (0..ctx.line_num())
        .find(|&i| ctx.get_line_text(i).contains("middle"))
        .expect("middle line");
    let t = ctx.get_line_text(mid);
    let pos = t.find("line").unwrap_or(0);
    set_caret_at_line_char(&mut ctx, mid, pos);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::insert_text("X".into()),
        "## Title\n\nmiddle Xline\n\n- [ ] todo\n\n|h|\n|--|\n|c|\n",
    );
}

#[test]
fn insert_text_table_with_surrounding_text_auto_converts_table_rows() {
    let mut ctx = md_ctx("");
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::insert_text("before\n| a | b |\n| --- | --- |\n| x | y |\nafter".into()),
        "before\n|a|b|\n|--|--|\n|x|y|\nafter",
    );
}

#[test]
fn insert_text_single_line_pipe_row_triggers_table_check() {
    let mut ctx = md_ctx("| a | b |\n|  |  |\n| x | y |");
    let line_no = find_line_containing(&ctx, "|  |  |");
    let n = ctx.get_line_text(line_no).chars().count();
    set_selection_at_line_chars(&mut ctx, line_no, 0, n);
    assert_action_with_undo_redo(
        &mut ctx,
        &Action::insert_text("| --- | --- |".into()),
        "|a|b|\n|--|--|\n|x|y|",
    );
}

#[test]
fn insert_text_unicode_scalar_boundary() {
    let mut ctx = md_ctx("café");
    set_caret_at_line_char(&mut ctx, 0, 3);
    assert_action_with_undo_redo(&mut ctx, &Action::insert_text("X".into()), "cafXé");
}

#[test]
fn insert_text_missing_text_param_no_panic_doc_unchanged() {
    let mut ctx = md_ctx("unchanged");
    let bad = Action::new("insert_text".into(), std::collections::HashMap::new());
    assert_action_with_undo_redo(&mut ctx, &bad, "unchanged");
}
