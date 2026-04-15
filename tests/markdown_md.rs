use eframe::egui::epaint::text::LayoutJob;
use egscribe::medit::cfg::{EditCfg, HeightMode};
use egscribe::medit::{MarkDownImpl, PghView, SegmentType};
use markdown::mdast::Node;

fn ast_type(ast: &Node) -> &str {
    match ast {
        Node::Root(_) => "Root",
        Node::Blockquote(_) => "Blockquote",
        Node::FootnoteDefinition(_) => "FootnoteDefinition",
        Node::MdxJsxFlowElement(_) => "MdxJsxFlowElement",
        Node::List(_) => "List",
        Node::MdxjsEsm(_) => "MdxjsEsm",
        Node::Toml(_) => "Toml",
        Node::Yaml(_) => "Yaml",
        Node::Break(_) => "Break",
        Node::InlineCode(_) => "InlineCode",
        Node::InlineMath(_) => "InlineMath",
        Node::Delete(_) => "Delete",
        Node::Emphasis(_) => "Emphasis",
        Node::MdxTextExpression(_) => "MdxTextExpression",
        Node::FootnoteReference(_) => "FootnoteReference",
        Node::Html(_) => "Html",
        Node::Image(_) => "Image",
        Node::ImageReference(_) => "ImageReference",
        Node::MdxJsxTextElement(_) => "MdxJsxTextElement",
        Node::Link(_) => "Link",
        Node::LinkReference(_) => "LinkReference",
        Node::Strong(_) => "Strong",
        Node::Text(_) => "Text",
        Node::Code(_) => "Code",
        Node::Math(_) => "Math",
        Node::MdxFlowExpression(_) => "MdxFlowExpression",
        Node::Heading(_) => "Heading",
        Node::Table(_) => "Table",
        Node::ThematicBreak(_) => "ThematicBreak",
        Node::TableRow(_) => "TableRow",
        Node::TableCell(_) => "TableCell",
        Node::ListItem(_) => "ListItem",
        Node::Definition(_) => "Definition",
        Node::Paragraph(_) => "Paragraph",
    }
}

fn default_cfg() -> EditCfg {
    EditCfg::new(14.0, true, None, HeightMode::fix_max())
}

#[test]
fn root_gap_empty_line_count_basic() {
    assert_eq!(MarkDownImpl::root_gap_empty_line_count(""), 0);
    assert_eq!(MarkDownImpl::root_gap_empty_line_count("\n"), 0);
    assert_eq!(MarkDownImpl::root_gap_empty_line_count("\n\n"), 1);
    assert_eq!(MarkDownImpl::root_gap_empty_line_count("\n\n\n"), 2);
    assert_eq!(MarkDownImpl::root_gap_empty_line_count(" \t\n  \n\t "), 1);
    assert_eq!(MarkDownImpl::root_gap_empty_line_count("a\n"), 0);
}

#[test]
fn thematic_break_mdast_gap_shape() {
    let md = "hello\n\n---\n";
    let ast = markdown::to_mdast(md, &markdown::ParseOptions::gfm()).unwrap();
    let items: Vec<_> = ast.children().unwrap().iter().collect();
    assert_eq!(items.len(), 2);
    let p = items[0].position().unwrap();
    let t = items[1].position().unwrap();
    assert_eq!(&md[p.start.offset..p.end.offset], "hello");
    assert_eq!(&md[t.start.offset..t.end.offset], "---");
    let gap = &md[p.end.offset..t.start.offset];
    assert_eq!(gap, "\n\n");
    assert_eq!(MarkDownImpl::root_gap_empty_line_count(gap), 1);
    assert_eq!(
        MarkDownImpl::root_gap_empty_line_count_before_node(gap, items[1]),
        0
    );
}

#[test]
fn markdown_to_pgh_texts_paragraph_then_thematic_no_extra_empty_pgh() {
    let cfg = default_cfg();
    let md = MarkDownImpl::new("hello\n\n---\n", true, None, false, &cfg);
    let views = md.markdown_to_pgh_texts();
    let lines: Vec<String> = views.iter().map(|v| v.get_text()).collect();
    assert_eq!(lines, vec!["hello", "---", ""], "{lines:?}");
}

#[test]
fn markdown_to_pgh_texts_root_gap_blank_lines() {
    let cfg = default_cfg();
    let md = MarkDownImpl::new("hello\n\nworld", true, None, false, &cfg);
    let views = md.markdown_to_pgh_texts();
    let lines: Vec<String> = views.iter().map(|v| v.get_text()).collect();
    assert_eq!(lines, vec!["hello", "", "world"], "{lines:?}");
}

#[test]
fn list_then_paragraph_gap_count_preserves_blank_line() {
    let src = "- list\n\ngood\n";
    let ast = markdown::to_mdast(src, &markdown::ParseOptions::gfm()).unwrap();
    let items: Vec<_> = ast.children().unwrap().iter().collect();
    assert_eq!(items.len(), 2);
    let p0 = items[0].position().unwrap();
    let p1 = items[1].position().unwrap();
    let n = MarkDownImpl::root_gap_empty_line_count_between(
        src,
        p0.end.offset,
        p1.start.offset,
        items[1],
    );
    assert_eq!(n, 1, "gap should keep one blank line, got {n}");
}

#[test]
fn markdown_to_pgh_texts_list_then_paragraph_keeps_blank_line() {
    let cfg = default_cfg();
    let md = MarkDownImpl::new("- list\n\ngood\n", true, None, false, &cfg);
    let views = md.markdown_to_pgh_texts();
    let lines: Vec<String> = views.iter().map(|v| v.get_text()).collect();
    assert_eq!(lines, vec!["- list", "", "good", ""], "{lines:?}");
}

#[test]
fn markdown_to_pgh_texts_list_then_table_keeps_blank_line() {
    let cfg = default_cfg();
    let md = MarkDownImpl::new("- [ ] todo\n\n| h |\n| - |\n| c |\n", true, None, false, &cfg);
    let views = md.markdown_to_pgh_texts();
    let lines: Vec<String> = views.iter().map(|v| v.get_text()).collect();
    assert_eq!(lines, vec!["- [ ] todo", "", "|h|", "|c|", ""], "{lines:?}");
}

fn echo_ast(md: &str, ast: &Node) {
    echo_ast_with_depth(md, ast, 0);
}

fn echo_ast_with_depth(md: &str, ast: &Node, depth: usize) {
    let indent = "  ".repeat(depth);
    if let Some(pos) = ast.position() {
        let s = &md[pos.start.offset..pos.end.offset];
        let s_no_newline = s.replace('\n', " ");
        println!("{}{}:[{}]", indent, ast_type(ast), s_no_newline);
    }
    if let Some(c) = ast.children() {
        for x in c {
            echo_ast_with_depth(md, x, depth + 1);
        }
    }
}

#[test]
fn split_text_pghview_embedded_newlines_three_lines() {
    let mut v = PghView::new_text();
    v.push_indent();
    v.push_text("test ".to_string(), None);
    v.push_text("line1\ntest".to_string(), Some(LayoutJob::default()));
    v.push_text(" line2\ntest line3".to_string(), None);
    let lines = v.split_text_by_embedded_newlines();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0].get_text(), "test line1");
    assert_eq!(lines[1].get_text(), "test line2");
    assert_eq!(lines[2].get_text(), "test line3");
    for line in &lines {
        for s in &line.pgh {
            if s.seg_type == SegmentType::Text {
                assert!(
                    s.item.layout_job().is_none(),
                    "跨行拆分后 Text 段不应保留 LayoutJob"
                );
            }
        }
    }
}

#[test]
fn test_md() {
    let md = r#"
# head
desc me

asd
"#;

    let ast = markdown::to_mdast(md, &markdown::ParseOptions::gfm()).unwrap();

    echo_ast(md, &ast);

    println!("{:?}", ast);
}

#[test]
fn test_extract_prefix_and_indent() {
    // 测试用例 1: 4个空格 + 列表控制字符
    let (prefix, indent_level, cleaned_text) = MarkDownImpl::extract_prefix_and_indent("    - test");
    assert_eq!(prefix, "    ");
    assert_eq!(indent_level, 1);
    assert_eq!(cleaned_text, "- test");

    // 测试用例 2: Tab + 列表控制字符
    let (prefix, indent_level, cleaned_text) = MarkDownImpl::extract_prefix_and_indent("\t- test");
    assert_eq!(prefix, "\t");
    assert_eq!(indent_level, 1);
    assert_eq!(cleaned_text, "- test");

    // 测试用例 3: 8个空格 + 列表控制字符（2级缩进）
    let (prefix, indent_level, cleaned_text) = MarkDownImpl::extract_prefix_and_indent("        - test");
    assert_eq!(prefix, "        ");
    assert_eq!(indent_level, 2);
    assert_eq!(cleaned_text, "- test");

    // 测试用例 4: 有序列表（数字+.）
    let (prefix, indent_level, cleaned_text) = MarkDownImpl::extract_prefix_and_indent("    1. test");
    assert_eq!(prefix, "    ");
    assert_eq!(indent_level, 1);
    assert_eq!(cleaned_text, "1. test");

    // 测试用例 5: 有序列表（多位数）
    let (prefix, indent_level, cleaned_text) = MarkDownImpl::extract_prefix_and_indent("  123. test");
    assert_eq!(prefix, "  ");
    assert_eq!(indent_level, 0);
    assert_eq!(cleaned_text, "123. test");

    // 测试用例 6: 没有空格/Tab，只有列表控制字符（不应该提取前缀）
    let (prefix, indent_level, cleaned_text) = MarkDownImpl::extract_prefix_and_indent("- test");
    assert_eq!(prefix, "");
    assert_eq!(indent_level, 0);
    assert_eq!(cleaned_text, "- test");

    // 测试用例 7: 只有空格，没有列表控制字符（不应该提取前缀）
    let (prefix, indent_level, cleaned_text) = MarkDownImpl::extract_prefix_and_indent("    test");
    assert_eq!(prefix, "");
    assert_eq!(indent_level, 0);
    assert_eq!(cleaned_text, "    test");

    // 测试用例 8: 空字符串
    let (prefix, indent_level, cleaned_text) = MarkDownImpl::extract_prefix_and_indent("");
    assert_eq!(prefix, "");
    assert_eq!(indent_level, 0);
    assert_eq!(cleaned_text, "");

    // 测试用例 9: 使用 * 作为列表控制字符
    let (prefix, indent_level, cleaned_text) = MarkDownImpl::extract_prefix_and_indent("  * test");
    assert_eq!(prefix, "  ");
    assert_eq!(indent_level, 0);
    assert_eq!(cleaned_text, "* test");

    // 测试用例 10: 使用 + 作为列表控制字符
    let (prefix, indent_level, cleaned_text) = MarkDownImpl::extract_prefix_and_indent("    + test");
    assert_eq!(prefix, "    ");
    assert_eq!(indent_level, 1);
    assert_eq!(cleaned_text, "+ test");

    // 测试用例 11: 数字后面不是点（不应该识别为有序列表）
    let (prefix, indent_level, cleaned_text) = MarkDownImpl::extract_prefix_and_indent("    123 test");
    assert_eq!(prefix, "");
    assert_eq!(indent_level, 0);
    assert_eq!(cleaned_text, "    123 test");
}

#[test]
fn test_chinese_text_panic() {
    // 测试包含中文字符的文本是否会引起 panic
    let cfg = default_cfg();
    let md = MarkDownImpl::new(r"-\# 一级目录", true, None, false, &cfg);

    let ast = markdown::to_mdast(r"-\# 一级目录", &markdown::ParseOptions::gfm()).unwrap();

    // 查找包含文本的节点并测试 paragraph_push_to_pghview
    fn test_node(md: &MarkDownImpl<'_>, node: &markdown::mdast::Node) -> bool {
        match node {
            markdown::mdast::Node::Text(_) | markdown::mdast::Node::Paragraph(_) => {
                let mut pghview = PghView::new_text();
                md.paragraph_push_to_pghview(node, md.format_default(), &mut pghview);
                true
            }
            _ => {
                if let Some(children) = node.children() {
                    for child in children {
                        if test_node(md, child) {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }

    let found_text = test_node(&md, &ast);
    assert!(found_text, "Should find text node to test");
}
