use docx_rs::{BreakType, Paragraph, Pic, Run, RunFonts, Table, TableCell, TableRow};
use docx_rs::{read_docx, Docx};
use plugin_sdk::{run_plugin, PluginApi, PluginHandler, PluginMap};
use pulldown_cmark::{
    CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd,
};
use quick_xml::events::Event as XmlEvent;
use quick_xml::Reader;
use serde_json::Value;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Cursor, Read};
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use zip::ZipArchive;

const PLANTUML_JAR_FILE: &str = "plantuml-1.2025.2.jar";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const DESC_FILE: &str = "desc.json";

fn plugin_dir() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
    exe.parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn local_name(name: &[u8]) -> &[u8] {
    if let Some(i) = name.iter().rposition(|b| *b == b':') {
        &name[i + 1..]
    } else {
        name
    }
}

fn attr_val(e: &quick_xml::events::BytesStart<'_>, key_suffix: &[u8]) -> Option<String> {
    for a in e.attributes().flatten() {
        let k = a.key.as_ref();
        let k = local_name(k);
        if k == key_suffix {
            return Some(String::from_utf8_lossy(&a.value).into_owned());
        }
    }
    None
}

#[derive(Debug, Default, Clone)]
struct ParsedStyles {
    /// normalized style display name -> styleId
    paragraph: HashMap<String, String>,
    /// (normalized name, styleId) for table styles
    table_styles: Vec<(String, String)>,
}

fn normalize_style_key(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn read_styles_xml_from_docx(docx_bytes: &[u8]) -> Result<String, String> {
    let cursor = Cursor::new(docx_bytes);
    let mut archive =
        ZipArchive::new(cursor).map_err(|e| format!("Open docx zip failed: {}", e))?;
    let mut f = archive
        .by_name("word/styles.xml")
        .map_err(|_| "word/styles.xml not found in template".to_string())?;
    let mut s = String::new();
    f.read_to_string(&mut s)
        .map_err(|e| format!("Read styles.xml failed: {}", e))?;
    Ok(s)
}

fn parse_styles_xml(xml: &str) -> Result<ParsedStyles, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut out = ParsedStyles::default();
    let mut in_style = false;
    let mut style_type = String::new();
    let mut style_id = String::new();
    let mut style_name: Option<String> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(XmlEvent::Start(e)) | Ok(XmlEvent::Empty(e)) => {
                let raw_name = e.name().as_ref().to_vec();
                let ln = local_name(&raw_name);
                if ln == b"style" {
                    in_style = true;
                    style_type = attr_val(&e, b"type").unwrap_or_default();
                    style_id = attr_val(&e, b"styleId").unwrap_or_default();
                    style_name = None;
                } else if in_style && ln == b"name" {
                    if let Some(v) = attr_val(&e, b"val") {
                        style_name = Some(v);
                    }
                }
            }
            Ok(XmlEvent::End(e)) => {
                let raw_name = e.name().as_ref().to_vec();
                let ln = local_name(&raw_name);
                if ln == b"style" {
                    if let (Some(name), id) = (style_name.take(), style_id.clone()) {
                        if !id.is_empty() {
                            let key = normalize_style_key(&name);
                            if style_type == "paragraph" {
                                out.paragraph.insert(key, id.clone());
                            } else if style_type == "table" {
                                out.table_styles.push((key, id));
                            }
                        }
                    }
                    in_style = false;
                }
            }
            Ok(XmlEvent::Eof) => break,
            Err(e) => return Err(format!("Parse styles.xml: {}", e)),
            _ => {}
        }
        buf.clear();
    }
    Ok(out)
}

#[derive(Clone)]
struct ResolvedStyles {
    body: String,
    headings: [String; 6],
    table: Option<String>,
}

impl ResolvedStyles {
    fn heading(&self, level: u8) -> &str {
        let i = (level.saturating_sub(1)).min(5) as usize;
        &self.headings[i]
    }
}

fn heading_aliases(level: u8) -> Vec<String> {
    let mut v = Vec::new();
    let n = level as u32;
    v.push(normalize_style_key(&format!("Heading {}", n)));
    v.push(normalize_style_key(&format!("heading {}", n)));
    v.push(normalize_style_key(&format!("标题 {}", n)));
    v.push(normalize_style_key(&format!("标题{}", n)));
    v
}

fn body_aliases() -> Vec<String> {
    [
        "Normal",
        "正文",
        "Body Text",
        "body text",
        "Plain Text",
    ]
    .into_iter()
    .map(normalize_style_key)
    .collect()
}

fn pick_table_style(parsed: &ParsedStyles, prefer: Option<&str>) -> Option<String> {
    if let Some(p) = prefer {
        let key = normalize_style_key(p);
        for (n, id) in &parsed.table_styles {
            if n == &key {
                return Some(id.clone());
            }
        }
        if !p.is_empty() {
            return Some(p.to_string());
        }
    }
    let hints = ["grid", "table", "网格", "light", "medium", "dark"];
    for hint in hints {
        for (n, id) in &parsed.table_styles {
            if n.contains(hint) {
                return Some(id.clone());
            }
        }
    }
    parsed
        .table_styles
        .first()
        .map(|(_, id)| id.clone())
}

fn resolve_styles(
    parsed: &ParsedStyles,
    config: &PluginMap,
    template_bytes: &[u8],
) -> Result<ResolvedStyles, String> {
    fn cfg_style(config: &PluginMap, key: &str) -> Option<String> {
        config
            .get("style_map")
            .and_then(|v| v.as_object())
            .and_then(|m| m.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    let mut headings = [
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ];
    for lvl in 1u8..=6 {
        let key = format!("h{}", lvl);
        if let Some(id) = cfg_style(config, &key) {
            headings[(lvl - 1) as usize] = id;
            continue;
        }
        let mut found = None;
        for a in heading_aliases(lvl) {
            if let Some(id) = parsed.paragraph.get(&a) {
                found = Some(id.clone());
                break;
            }
        }
        headings[(lvl - 1) as usize] = found.ok_or_else(|| {
            format!(
                "Could not resolve style for heading level {} (set style_map.h{} in desc.json)",
                lvl, lvl
            )
        })?;
    }

    let body = cfg_style(config, "body").or_else(|| {
        body_aliases()
            .into_iter()
            .find_map(|a| parsed.paragraph.get(&a).cloned())
    });

    let body = body.ok_or_else(|| {
        "Could not resolve body paragraph style (set style_map.body in desc.json)".to_string()
    })?;

    let table_override = config
        .get("table_style_id")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let table = pick_table_style(parsed, table_override.as_deref());

    let _ = template_bytes; // reserved for future doc defaults scan

    Ok(ResolvedStyles {
        body,
        headings,
        table,
    })
}

fn output_docx_path(new_file_path: &str) -> Result<PathBuf, String> {
    let p = Path::new(new_file_path);
    if new_file_path.trim().is_empty() {
        return Err("Invalid new_file_path: empty".to_string());
    }
    let ext_is_docx = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("docx"))
        .unwrap_or(false);
    Ok(if ext_is_docx {
        p.to_path_buf()
    } else {
        p.with_extension("docx")
    })
}

fn collect_until_block_end(
    events: &mut std::iter::Peekable<std::vec::IntoIter<Event<'_>>>,
    end: TagEnd,
) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    while let Some(ev) = events.next() {
        match ev {
            Event::Start(_) => depth += 1,
            Event::End(e) => {
                if depth == 0 && e == end {
                    break;
                }
                if depth > 0 {
                    depth -= 1;
                } else {
                    break;
                }
            }
            Event::Text(t) => out.push_str(&t),
            Event::Code(c) => out.push_str(&c),
            Event::SoftBreak | Event::HardBreak => out.push('\n'),
            _ => {}
        }
    }
    out
}

/// Tight list items do not wrap content in [`Tag::Paragraph`]; text lives directly under [`Tag::Item`].
/// Stops before `End(Item)` / `Start(List)` / `Start(Paragraph)` at depth 0 (those events are not consumed).
fn collect_tight_list_item_leading_content(
    events: &mut std::iter::Peekable<std::vec::IntoIter<Event<'_>>>,
) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    loop {
        match events.peek() {
            Some(Event::Start(Tag::List(_))) if depth == 0 => break,
            Some(Event::Start(Tag::Paragraph)) if depth == 0 => break,
            Some(Event::End(TagEnd::Item)) if depth == 0 => break,
            None => break,
            _ => {}
        }
        let Some(ev) = events.next() else {
            break;
        };
        match ev {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            Event::Text(t) => out.push_str(&t),
            Event::Code(c) => out.push_str(&c),
            Event::SoftBreak | Event::HardBreak => out.push('\n'),
            Event::TaskListMarker(checked) => {
                out.push_str(if checked { "[x] " } else { "[ ] " });
            }
            Event::InlineMath(s) | Event::DisplayMath(s) => out.push_str(&s),
            Event::Html(s) | Event::InlineHtml(s) => out.push_str(&s),
            Event::FootnoteReference(r) => {
                out.push_str("[^");
                out.push_str(&r);
                out.push(']');
            }
            Event::Rule => out.push_str("---"),
        }
    }
    out
}

/// `docx_rs::Run::add_text` strips `\n`; use explicit line breaks inside one paragraph.
fn run_with_line_breaks(text: &str) -> Run {
    let mut run = Run::new();
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            run = run.add_break(BreakType::TextWrapping);
        }
        run = run.add_text(line);
    }
    run
}

fn para_with_style(text: impl Into<String>, style: &str) -> Paragraph {
    let s = text.into();
    Paragraph::new()
        .add_run(run_with_line_breaks(&s))
        .style(style)
}

fn para_code_block(text: String, body_style: &str) -> Paragraph {
    let run = run_with_line_breaks(&text)
        .fonts(RunFonts::new().ascii("Consolas").hi_ansi("Consolas").east_asia("Consolas"));
    Paragraph::new().add_run(run).style(body_style)
}

/// 扩展围栏行：` ```plantuml file://.../x.png` → 本地 PNG 路径（与编辑器导出一致）
fn plantuml_png_path_from_fence_info(fence_info: &str) -> Option<PathBuf> {
    let t = fence_info.trim();
    if !t.to_lowercase().starts_with("plantuml") {
        return None;
    }
    let idx = t.find("file://")?;
    let url = t[idx..].split_whitespace().next()?;
    let path = url.strip_prefix("file://")?;
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path))
}

fn fence_is_plantuml(fence_info: &str) -> bool {
    fence_info
        .trim()
        .to_lowercase()
        .starts_with("plantuml")
}

fn resolve_plantuml_jar_path(config: &PluginMap) -> Option<PathBuf> {
    if let Some(v) = config.get("plantuml_jar").and_then(|v| v.as_str()) {
        let path = PathBuf::from(v);
        if path.exists() {
            return Some(path);
        }
    }
    if let Ok(cur_dir) = std::env::current_dir() {
        let path = cur_dir.join(PLANTUML_JAR_FILE);
        if path.exists() {
            return Some(path);
        }
    }
    let base = plugin_dir();
    let path = base.join(PLANTUML_JAR_FILE);
    if path.exists() {
        return Some(path);
    }
    // egscribe\plugins\docx_md\ → 上两级 → egscribe\plantuml-1.2025.2.jar
    if let Some(two_up) = base.parent().and_then(|p| p.parent()) {
        let path = two_up.join(PLANTUML_JAR_FILE);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

fn ensure_plantuml_wrapped(code: &str) -> String {
    let trimmed = code.trim();
    if trimmed.starts_with("@startuml") && trimmed.ends_with("@enduml") {
        code.to_string()
    } else {
        format!("@startuml\n{}\n@enduml\n", code)
    }
}

fn plantuml_digest(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn cleanup_temp_puml(path: &Path) {
    let _ = fs::remove_file(path);
}

fn cleanup_png_best_effort(path: &Path) {
    let _ = fs::remove_file(path);
}

/// 使用 `java -jar plantuml...jar` 将正文渲染为 PNG 字节（与主程序预览逻辑一致）。
fn render_plantuml_png_bytes(code_text: &str, jar_path: &Path) -> Option<Vec<u8>> {
    let base_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("."));
    let cache_dir = base_dir.join("cache").join("plantuml_docx");
    if fs::create_dir_all(&cache_dir).is_err() {
        return None;
    }
    let stem = format!("export_{:016x}", plantuml_digest(code_text));
    let puml_path = cache_dir.join(format!("{}.puml", stem));
    let wrapped = ensure_plantuml_wrapped(code_text);
    if fs::write(&puml_path, wrapped).is_err() {
        return None;
    }
    let mut cmd = Command::new("java");
    cmd.arg("-jar")
        .arg(jar_path)
        .arg("-tpng")
        .arg("-charset")
        .arg("UTF-8")
        .arg("-o")
        .arg(&cache_dir)
        .arg(&puml_path);
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let ok = match cmd.status() {
        Ok(s) => s.success(),
        Err(_) => {
            cleanup_temp_puml(&puml_path);
            return None;
        }
    };
    if !ok {
        cleanup_temp_puml(&puml_path);
        return None;
    }
    let png_path = cache_dir.join(format!("{}.png", stem));
    let bytes = fs::read(&png_path).ok();
    cleanup_temp_puml(&puml_path);
    if bytes.is_some() {
        cleanup_png_best_effort(&png_path);
    }
    bytes
}

fn para_list_line(prefix: &str, text: &str, style: &str, indent_level: usize) -> Paragraph {
    let line = format!("{}{}", prefix, text.trim_end());
    let base = 360i32.saturating_mul(indent_level as i32);
    let p = para_with_style(line, style);
    if indent_level == 0 {
        p
    } else {
        p.indent(Some(base), None, None, None)
    }
}

fn heading_level(lvl: HeadingLevel) -> u8 {
    match lvl {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

struct ListState {
    ordered: bool,
    next_n: u64,
}

fn render_list<'a>(
    mut doc: Docx,
    events: &mut std::iter::Peekable<std::vec::IntoIter<Event<'a>>>,
    styles: &ResolvedStyles,
    indent_level: usize,
    list_stack: &mut Vec<ListState>,
) -> Docx {
    loop {
        match events.peek() {
            None => break,
            Some(Event::End(TagEnd::List(_))) => {
                events.next();
                list_stack.pop();
                break;
            }
            Some(Event::Start(Tag::Item)) => {
                events.next();
                let prefix = {
                    let ctx = list_stack.last_mut().expect("list item without list");
                    if ctx.ordered {
                        let n = ctx.next_n;
                        ctx.next_n = ctx.next_n.saturating_add(1);
                        format!("{}. ", n)
                    } else {
                        "- ".to_string()
                    }
                };
                doc = consume_list_item_content(
                    doc,
                    events,
                    styles,
                    indent_level,
                    &prefix,
                    list_stack,
                );
            }
            Some(_) => {
                events.next();
            }
        }
    }
    doc
}

fn consume_list_item_content<'a>(
    mut doc: Docx,
    events: &mut std::iter::Peekable<std::vec::IntoIter<Event<'a>>>,
    styles: &ResolvedStyles,
    indent_level: usize,
    prefix: &str,
    list_stack: &mut Vec<ListState>,
) -> Docx {
    loop {
        match events.peek() {
            None => break,
            Some(Event::End(TagEnd::Item)) => {
                events.next();
                break;
            }
            Some(Event::Start(Tag::Paragraph)) => {
                events.next();
                let text = collect_until_block_end(events, TagEnd::Paragraph);
                doc = doc.add_paragraph(para_list_line(
                    prefix,
                    &text,
                    &styles.body,
                    indent_level,
                ));
            }
            Some(Event::Start(Tag::List(start))) => {
                let start = *start;
                events.next();
                list_stack.push(ListState {
                    ordered: start.is_some(),
                    next_n: start.unwrap_or(1),
                });
                doc = render_list(doc, events, styles, indent_level + 1, list_stack);
            }
            Some(_) => {
                let text = collect_tight_list_item_leading_content(events);
                if !text.trim().is_empty() {
                    doc = doc.add_paragraph(para_list_line(
                        prefix,
                        &text,
                        &styles.body,
                        indent_level,
                    ));
                }
            }
        }
    }
    doc
}

fn collect_cell_text<'a>(
    events: &mut std::iter::Peekable<std::vec::IntoIter<Event<'a>>>,
) -> String {
    let mut parts = Vec::new();
    loop {
        match events.peek() {
            None => break,
            Some(Event::End(TagEnd::TableCell)) => {
                events.next();
                break;
            }
            Some(Event::Start(Tag::Paragraph)) => {
                events.next();
                parts.push(collect_until_block_end(events, TagEnd::Paragraph));
            }
            Some(_) => {
                events.next();
            }
        }
    }
    parts.join("\n").trim().to_string()
}

fn is_md_table_separator_row(row: &[String]) -> bool {
    !row.is_empty()
        && row.iter().all(|c| {
            let s = c.trim();
            !s.is_empty() && s.chars().all(|ch| ch == '-' || ch == ':' || ch == ' ')
        })
}

fn build_table(rows: &[Vec<String>], cell_style: &str, table_style: Option<&str>) -> Table {
    let mut trows = Vec::new();
    for r in rows {
        let cells: Vec<TableCell> = r
            .iter()
            .map(|c| {
                TableCell::new().add_paragraph(
                    Paragraph::new()
                        .add_run(run_with_line_breaks(c.as_str()))
                        .style(cell_style),
                )
            })
            .collect();
        trows.push(TableRow::new(cells));
    }
    let mut t = Table::new(trows);
    if let Some(ts) = table_style {
        t = t.style(ts);
    }
    let col_count = rows.first().map(|r| r.len()).unwrap_or(0);
    if col_count > 0 {
        let w = 9000usize / col_count.max(1);
        t = t.set_grid(vec![w; col_count]);
    }
    t
}

fn markdown_to_docx(
    md: &str,
    styles: &ResolvedStyles,
    mut doc: Docx,
    config: &PluginMap,
) -> Result<Docx, String> {
    let export_plantuml_code = config
        .get("plantuml_export_code")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let export_tables = config
        .get("export_tables")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    let event_vec: Vec<Event<'_>> = Parser::new_ext(md, opts).collect();
    let mut events = event_vec.into_iter().peekable();
    let mut list_stack: Vec<ListState> = Vec::new();

    while let Some(ev) = events.next() {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                let lvl = heading_level(level);
                let text = collect_until_block_end(&mut events, TagEnd::Heading(level));
                let st = styles.heading(lvl);
                doc = doc.add_paragraph(para_with_style(text.trim(), st));
            }
            Event::Start(Tag::Paragraph) => {
                let text = collect_until_block_end(&mut events, TagEnd::Paragraph);
                let t = text.trim();
                if !t.is_empty() {
                    doc = doc.add_paragraph(para_with_style(t, &styles.body));
                }
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let fence_info = match &kind {
                    CodeBlockKind::Fenced(info) => info.to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                let mut body = String::new();
                loop {
                    match events.next() {
                        Some(Event::Text(t)) => body.push_str(&t),
                        Some(Event::End(TagEnd::CodeBlock)) => break,
                        Some(Event::Start(_)) => {}
                        Some(Event::End(_)) => {}
                        None => break,
                        _ => {}
                    }
                }
                if let CodeBlockKind::Indented = kind {
                    body.push('\n');
                }
                let body = body.trim_end_matches('\n').to_string();

                let plantuml_png_bytes = match &kind {
                    CodeBlockKind::Fenced(_) => plantuml_png_path_from_fence_info(&fence_info)
                        .filter(|p| p.is_file())
                        .and_then(|p| fs::read(p).ok())
                        .or_else(|| {
                            if fence_is_plantuml(&fence_info) && !body.is_empty() {
                                resolve_plantuml_jar_path(config)
                                    .and_then(|jar| render_plantuml_png_bytes(&body, &jar))
                            } else {
                                None
                            }
                        }),
                    CodeBlockKind::Indented => None,
                };
                let omit_plantuml_code =
                    !export_plantuml_code && plantuml_png_bytes.is_some();

                if !body.is_empty() && !omit_plantuml_code {
                    doc = doc.add_paragraph(para_code_block(body, &styles.body));
                }
                if let Some(bytes) = plantuml_png_bytes {
                    let pic = Pic::new(&bytes);
                    doc = doc.add_paragraph(
                        Paragraph::new()
                            .add_run(Run::new().add_image(pic))
                            .style(&styles.body),
                    );
                }
            }
            Event::Start(Tag::List(start)) => {
                list_stack.push(ListState {
                    ordered: start.is_some(),
                    next_n: start.unwrap_or(1),
                });
                doc = render_list(doc, &mut events, styles, 0, &mut list_stack);
            }
            Event::Start(Tag::Table(_)) => {
                if export_tables {
                    let mut rows: Vec<Vec<String>> = Vec::new();
                    loop {
                        match events.next() {
                            None => break,
                            Some(Event::End(TagEnd::Table)) => break,
                            Some(Event::Start(Tag::TableRow)) => {
                                let mut row: Vec<String> = Vec::new();
                                loop {
                                    match events.next() {
                                        None => break,
                                        Some(Event::End(TagEnd::TableRow)) => break,
                                        Some(Event::Start(Tag::TableCell)) => {
                                            row.push(collect_cell_text(&mut events));
                                        }
                                        Some(_) => {}
                                    }
                                }
                                if !row.is_empty() {
                                    rows.push(row);
                                }
                            }
                            Some(_) => {}
                        }
                    }
                    let rows: Vec<Vec<String>> = rows
                        .into_iter()
                        .filter(|r| !is_md_table_separator_row(r.as_slice()))
                        .collect();
                    if !rows.is_empty() {
                        let tbl = build_table(&rows, &styles.body, styles.table.as_deref());
                        doc = doc.add_table(tbl);
                    }
                } else {
                    loop {
                        match events.next() {
                            None => break,
                            Some(Event::End(TagEnd::Table)) => break,
                            Some(_) => {}
                        }
                    }
                }
            }
            Event::Rule => {
                doc = doc.add_paragraph(
                    Paragraph::new()
                        .add_run(Run::new().add_text(""))
                        .style(&styles.body),
                );
            }
            Event::Start(Tag::BlockQuote(kind)) => {
                let end = TagEnd::BlockQuote(kind);
                let inner = collect_until_block_end(&mut events, end);
                let quoted = inner.trim();
                if !quoted.is_empty() {
                    doc = doc.add_paragraph(para_with_style(
                        format!("> {}", quoted.replace('\n', "\n> ")),
                        &styles.body,
                    ));
                }
            }
            _ => {}
        }
    }
    Ok(doc)
}

fn merge_config(init: &PluginMap) -> PluginMap {
    let mut m = init.clone();
    let desc_path = plugin_dir().join(DESC_FILE);
    if let Ok(txt) = std::fs::read_to_string(&desc_path) {
        if let Ok(v) = serde_json::from_str::<Value>(&txt) {
            if let Some(cfg) = v.get("config").and_then(|c| c.as_object()) {
                for (k, val) in cfg {
                    m.insert(k.clone(), val.clone());
                }
            }
        }
    }
    m
}

fn export_outline(
    markdown: &str,
    new_file_path: &str,
    init_config: &PluginMap,
) -> Result<PathBuf, String> {
    if markdown.trim().is_empty() {
        return Err("current_outline_content is empty".to_string());
    }
    let config = merge_config(init_config);
    let template_name = config
        .get("template_file")
        .and_then(|v| v.as_str())
        .unwrap_or("templ.docx")
        .trim()
        .to_string();
    if template_name.is_empty() {
        return Err("template_file is empty".to_string());
    }
    let template_path = plugin_dir().join(&template_name);
    let template_bytes = std::fs::read(&template_path).map_err(|e| {
        format!(
            "Read template {:?} failed: {} (place templ.docx next to the plugin exe)",
            template_path, e
        )
    })?;
    let styles_xml = read_styles_xml_from_docx(&template_bytes)?;
    let parsed = parse_styles_xml(&styles_xml)?;
    let resolved = resolve_styles(&parsed, &config, &template_bytes)?;

    let base = read_docx(&template_bytes).map_err(|e| format!("read_docx: {}", e))?;
    let doc = markdown_to_docx(markdown, &resolved, base.clone(), &config)?;
    let out = output_docx_path(new_file_path)?;
    let f = std::fs::File::create(&out).map_err(|e| format!("Create output failed: {}", e))?;
    doc.build()
        .pack(f)
        .map_err(|e| format!("pack docx failed: {}", e))?;
    Ok(out)
}

struct Docx2MdPlugin {
    config: PluginMap,
}

impl Default for Docx2MdPlugin {
    fn default() -> Self {
        Self {
            config: PluginMap::new(),
        }
    }
}

impl PluginHandler for Docx2MdPlugin {
    fn on_init(&mut self, api: &mut PluginApi, id: String, config: PluginMap) -> io::Result<()> {
        self.config = config;
        api.send_ready(
            "Markdown outline to DOCX",
            "0.1.0",
            vec!["export_outline_to_docx".to_string()],
        )?;
        api.send_ok(
            id,
            Some(serde_json::json!({
                "message": "docx_md plugin initialized",
            })),
        )
    }

    fn on_execute(
        &mut self,
        api: &mut PluginApi,
        id: String,
        command: String,
        params: PluginMap,
    ) -> io::Result<()> {
        match command.as_str() {
            "export_outline_to_docx" => {
                let markdown = params
                    .get("current_outline_content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let new_file_path = params
                    .get("new_file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if new_file_path.is_empty() {
                    return api.send_err(id, "Missing param: new_file_path");
                }
                match export_outline(&markdown, &new_file_path, &self.config) {
                    Ok(output) => {
                        let output_str = output.to_string_lossy().to_string();
                        api.send_ok(
                            id,
                            Some(serde_json::json!({
                                "source": new_file_path,
                                "output": output_str
                            })),
                        )
                    }
                    Err(e) => api.send_err(id, e),
                }
            }
            _ => api.send_err(id, format!("Unknown command: {}", command)),
        }
    }
}

fn main() {
    let mut plugin = Docx2MdPlugin::default();
    run_plugin(&mut plugin);
}
