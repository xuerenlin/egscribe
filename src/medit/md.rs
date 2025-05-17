use core::f32;
use std::ops::Range;

use crate::medit::{icon, ImageInfo, PghView, TableInfo, TEXT_TOP_SPACE, TEXT_BOTTOM_SPACE};
use eframe::egui::epaint::text::{LayoutJob, TextFormat};
use eframe::egui::{Color32, FontFamily, Stroke};
use markdown;
use markdown::mdast::Node;
use markdown::unist::Position;
use regex::Regex;

use super::ctx::EditCfg;

#[derive(Clone)]
pub enum LinkInfo {
    File(String),   //file
    Link(String),   //url
    Image(ImageInfo)
}

#[derive(Clone)]
pub struct LinkEnd {
    end_pos: usize,
    link_info: LinkInfo,
}

impl LinkEnd {
    pub fn new_file(end_pos: usize, file: String) -> Self {
        LinkEnd { end_pos, link_info: LinkInfo::File(file) }
    }
    pub fn new_link(end_pos: usize, url: String) -> Self {
        LinkEnd { end_pos, link_info: LinkInfo::Link(url) }
    }
    pub fn new_image(end_pos: usize, alt: String, url: String) -> Self {
        LinkEnd { end_pos, link_info: LinkInfo::Image(ImageInfo{alt, url, img:None}) }
    }
}

pub struct MarkDownImpl<'a> {
    text: String,
    enable_markdown: bool,
    curosr_char_index: Option<usize>,
    seleting: bool,
    cfg: &'a EditCfg,
}

impl<'a> MarkDownImpl<'a> {
    pub fn new(
        s: &str,
        enable_markdown: bool,
        curosr_char_index: Option<usize>,
        seleting: bool,
        cfg: &'a EditCfg,
    ) -> Self {
        let mut md = MarkDownImpl {
            text: s.to_string(),
            enable_markdown,
            curosr_char_index,
            seleting,
            cfg,
        };

        md.text = md.text.replace("\r\n", "\n");
        md
    }

    pub fn new_simple(s: &str, cfg: &'a EditCfg) -> Self {
        Self::new(s, true, None, false, cfg)
    }

    fn format_default(&self) -> TextFormat {
        let mut format = TextFormat::default();
        format.font_id.size = self.cfg.font_size;
        format.font_id.family = FontFamily::Proportional;
        format.color = self.cfg.text_color();
        format
    }

    fn format_code(&self) -> TextFormat {
        let mut format = TextFormat::default();
        format.font_id.size = self.cfg.font_size;
        format.font_id.family = FontFamily::Monospace;
        format.color = self.cfg.text_color();
        format
    }

    fn format_hide(&self, left: &Range<usize>, right: &Range<usize>) -> TextFormat {
        let mut format = TextFormat::default();
        format.font_id.size = 0.1;
        if let Some(char_index) = self.curosr_char_index {
            if self.seleting {
                format.font_id.size = self.cfg.font_size;
            } else {
                let byte_index: usize = self
                    .text
                    .chars()
                    .take(char_index)
                    .map(|c| c.len_utf8())
                    .sum();

                if byte_index >= left.start && byte_index <= left.end && left.end > left.start {
                    format.font_id.size = self.cfg.font_size;
                }
                if byte_index >= right.start && byte_index <= right.end && right.end > right.start {
                    format.font_id.size = self.cfg.font_size;
                }
            }
        }
        format
    }

    //deep: (between `1` and `6`, both including)
    fn format_head(&self, deep: u8) -> TextFormat {
        let mut format = self.format_default();
        if deep < 1 || deep > 6 {
            return format;
        }

        self.format_strong(&mut format);
        let max_font_size = self.cfg.font_size * 1.2;
        let delta_font_size = (max_font_size - self.cfg.font_size) / 6.0;
        let head_font_size = self.cfg.font_size + (7 - deep) as f32 * delta_font_size;
        format.font_id.size = head_font_size;
        format
    }

    fn format_strong(&self, format: &mut TextFormat) {
        format.font_id.family = FontFamily::Name("msyhb".into());
    }

    fn format_italics(&self, format: &mut TextFormat) {
        format.italics = true;
    }

    fn format_underline(&self, format: &mut TextFormat) {
        format.underline = Stroke::new(1.0, self.cfg.text_color());
    }

    fn format_delete(&self, format: &mut TextFormat) {
        format.color = self.cfg.weak_color();
        format.strikethrough = Stroke::new(1.0, self.cfg.weak_color());
    }

    fn format_inlinecode(&self, format: &mut TextFormat) {
        format.background = self.cfg.code_bg_color();
    }

    fn format_link(&self, format: &mut TextFormat) {
        format.underline = Stroke::new(1.0, self.cfg.link_color());
        format.color = self.cfg.link_color();
    }

    fn text_between_pos(&self, p1: Option<&Position>, p2: Option<&Position>) -> String {
        if let Some(pos1) = p1 {
            if let Some(pos2) = p2 {
                if pos2.start.offset > pos1.start.offset {
                    let ctrl = &self.text[pos1.start.offset..pos2.start.offset];
                    return ctrl.to_string();
                }
            }
        }
        "".to_string()
    }

    fn node_text(&self, node: &Node) -> &str {
        if let Some(p) = node.position() {
            &self.text[p.start.offset..p.end.offset]
        } else {
            ""
        }
    }

    fn node_children_count(&self, node: &Node) -> usize {
        if let Some(children) = node.children() {
            children.len()
        } else {
            0
        }
    }

    fn text_double_links(&self, node: &Node) -> Vec<(usize, usize)> {
        if let Node::Text(text) = node {
            let re = Regex::new(r"\[\[(.*?)\]\]").unwrap();
            let info: Vec<_> = re.captures_iter(&text.value)
                .map(|cap|{
                    let start = cap.get(0).unwrap().start();
                    let end = cap.get(0).unwrap().end();
                    (start, end)
                }).collect();
            info
        } else {
            vec![]
        }
    }

    fn text_check_double_link(&self, node: &Node, job: &mut LayoutJob, link_ends: &mut Vec<LinkEnd>, format: &mut TextFormat) {
        if let Node::Text(text) = node {
            let info = self.text_double_links(node);
            if info.is_empty() {
                job.append(&text.value, 0.0, format.clone());
            } else {
                let mut pre = 0 as usize;
                for x in info {
                    if x.0 > pre {
                        let value = &text.value[pre..x.0];
                        job.append(&value, 0.0, format.clone());
                    }

                    let pos = node.position().unwrap();
                    let range_left = pos.start.offset + x.0 .. pos.start.offset + x.0 + 2;
                    let range_right = pos.start.offset + x.1 - 2 .. pos.start.offset + x.1;
                    job.append("[[", 0.0, self.format_hide(&range_left, &range_right));

                    let value = &text.value[x.0+2..x.1-2];
                    let mut link_format = format.clone();
                    self.format_link(&mut link_format);
                    job.append(value, 0.0, link_format);

                    job.append("]]", 0.0, self.format_hide(&range_left, &range_right));
                    link_ends.push(LinkEnd::new_file(job.sections.len(), value.to_string()));
                    pre = x.1;
                }
                let value = &text.value[pre..];
                if value.len() > 0 {
                    job.append(&value, 0.0, format.clone());
                }
            }
        }
    }

    fn paragraph_format(
        &self,
        node: &Node,
        parent_pos: Option<&Position>,
        first: bool,
        last: bool,
        job: &mut LayoutJob,
        link_ends: &mut Vec<LinkEnd>,
        format: &mut TextFormat,
    ) {
        let mut link_url = None;
        //first child, add left ctrl
        if let Some(pos) = node.position() {
            if let Some(parent_pos) = parent_pos {
                if first {
                    let range_left = parent_pos.start.offset..pos.start.offset;
                    let range_right = pos.end.offset..parent_pos.end.offset;
                    let ctrl_left = &self.text[range_left.clone()];
                    if !ctrl_left.is_empty() {
                        job.append(ctrl_left, 0.0, self.format_hide(&range_left, &range_right));
                    }
                }
            }
        }

        //add value and childrens
        if self.node_children_count(node) == 0 {
            match node {
                Node::Text(text) => {
                    //job.append(&text.value, 0.0, format.clone());
                    self.text_check_double_link(node, job, link_ends, format);
                }
                Node::Image(image) => {
                    let mut new_format = format.clone();
                    self.format_link(&mut new_format);
                    let text = self.node_text(node);
                    job.append(text, 0.0, new_format);
                    link_ends.push(LinkEnd::new_image(job.sections.len(),  image.alt.clone(), image.url.clone()));
                }
                /* todo
                Node::InlineCode(code) => {
                    let mut new_format = format.clone();
                    self.format_inlinecode(&mut new_format);
                    let value = format!("`{}`", code.value);
                    job.append(&value, 0.0, new_format);
                }
                */
                _ => {
                    if let Some(pos) = node.position() {
                        let range = pos.start.offset..pos.end.offset;
                        let ctrl = &self.text[range.clone()];
                        job.append(ctrl, 0.0, self.format_hide(&range, &range));
                    }
                }
            }
        } else {
            let mut new_format = format.clone();
            match node {
                Node::Link(link) => {
                    self.format_link(&mut new_format);
                    link_url = Some(link.url.clone());
                }
                Node::Strong(_) => {
                    self.format_strong(&mut new_format);
                }
                Node::Delete(_) => {
                    self.format_delete(&mut new_format);
                }
                Node::Emphasis(_) => {
                    let node_str = self.node_text(&node);
                    if node_str.starts_with('_') {
                        self.format_underline(&mut new_format);
                    } else {
                        self.format_italics(&mut new_format);
                    }
                }
                _ => {}
            }

            if let Some(items) = node.children() {
                for (i, item) in items.iter().enumerate() {
                    let is_first = i == 0;
                    let is_last = i + 1 == items.len();
                    self.paragraph_format(
                        item,
                        node.position(),
                        is_first,
                        is_last,
                        job,
                        link_ends,
                        &mut new_format,
                    );
                }
            }
        }

        //last child, add right ctrl
        if let Some(pos) = node.position() {
            if let Some(parent_pos) = parent_pos {
                if last {
                    let range_left = parent_pos.start.offset..pos.start.offset;
                    let range_right = pos.end.offset..parent_pos.end.offset;
                    let ctrl_right = &self.text[range_right.clone()];
                    if !ctrl_right.is_empty() {
                        job.append(ctrl_right, 0.0, self.format_hide(&range_left, &range_right));
                    }
                }
            }
        }

        if let Some(link_url) = link_url {
            link_ends.push(LinkEnd::new_link(job.sections.len(),  link_url));
        }
    }

    fn paragraph_push_to_pghview(&self, node: &Node, format: TextFormat, pghview: &mut PghView) {
        let mut job: LayoutJob = LayoutJob::default();
        let mut link_ends = vec![];
        let mut format = format;
        if let Some(pos) = node.position() {
            self.paragraph_format(node, None, false, false, &mut job, &mut link_ends, &mut format);
            let total_s = &self.text[pos.start.offset..pos.end.offset];

            let mut sub_job: LayoutJob = LayoutJob::default();
            let mut seg_str = String::new();
            for (i, x) in job.sections.iter().enumerate() {
                let sub_str = total_s[x.byte_range.clone()].to_string();
                sub_job.append(&sub_str, 0.0, x.format.clone());
                seg_str += &sub_str;

                if let Some(link) = link_ends.iter().find(|end| end.end_pos == i+1) {
                    pghview.push_text(seg_str, Some(sub_job));

                    //push link pgh_segment
                    pghview.push_icon(icon::IconName::icon_external_link(link.link_info.clone()));
                    sub_job = LayoutJob::default();
                    seg_str = String::new();

                    //push image pgh_segment
                    if let LinkInfo::Image(image_info) = &link.link_info {
                        pghview.push_image(image_info.to_owned());
                    }

                    //the last segment, push a empty text segment after the icon-button
                    if i+1 == job.sections.len() {
                        pghview.push_text("".to_string(), None);
                    }
                }
            }
            if seg_str.len() > 0 {
                pghview.push_text(seg_str, Some(sub_job));
            }
        }
    }

    fn paragraph_to_pghview(&self, node: &Node, format: TextFormat) -> PghView {
        let mut pghview = PghView::new_text();
        pghview.push_indent();
        self.paragraph_push_to_pghview(node, format, &mut pghview);
        pghview.spacing_top = self.cfg.font_size / 5.0;
        pghview.spacing_bottom = self.cfg.font_size / 5.0;
        pghview
    }

    fn heading_to_pghview(&self, node: &Node) -> PghView {
        let mut depth = 0;
        if let Node::Heading(h) = node {
            depth = h.depth
        }
        let mut pghview = PghView::new_heading();
        self.paragraph_push_to_pghview(node, self.format_head(depth), &mut pghview);
        pghview.spacing_top = self.cfg.font_size;
        pghview.spacing_bottom = self.cfg.font_size / 1.5;
        pghview
    }

    fn list_to_pghview(&self, node: &Node) -> PghView {
        let mut pghview = PghView::new_list_item();
        if let Some(items) = node.children() {
            if let Some(list_node) = items.first() {
                let mut format = self.format_default();
                if let Node::ListItem(it) = list_node {
                    pghview.push_indent();
                    if let Some(checked) = it.checked {
                        pghview.push_checkbox();
                        if checked {
                            self.format_delete(&mut format);
                        }
                    } else {
                        pghview.push_point();
                    }
                }

                self.paragraph_push_to_pghview(list_node, format, &mut pghview);
            }
        }
        pghview.spacing_bottom = self.cfg.font_size / 5.0;
        pghview
    }

    fn blockquote_to_pghview(&self, node: &Node) -> PghView {
        let mut pghview = PghView::new_block_line();
        if let Some(items) = node.children() {
            if let Some(list_node) = items.first() {
                pghview.push_quote_indent();

                //println!("blockquote={:?}", node);
                self.paragraph_push_to_pghview(node, self.format_default(), &mut pghview);
            } else {
                let s = self.node_text(node);
                pghview.push_text(s.to_string(), None);
            }
        }
        pghview
    }

    fn thematicbreak_to_pghview(&self, node: &Node) -> PghView {
        let mut pghview = PghView::new_break_line();
        let s = self.node_text(node);
        //pghview.push_text(s.to_string(), None);
        self.paragraph_push_to_pghview(node, self.format_default(), &mut pghview);
        pghview.push_break();
        pghview
    }

    fn table_to_pghview(&self, node: &Node) -> PghView {
        let mut table_info = TableInfo::default();
        let mut data: Vec<Vec<String>> = vec![];
        if let Some(table) = node.children() {
            for row in table {
                if let Some(cols) = row.children() {
                    table_info.row_count += 1;
                    let mut row_data = vec![];
                    let mut col_count = 0;
                    for col in cols {
                        if let Some(para) = col.children() {
                            if let Some(text) = para.first() {
                                col_count += 1;
                                row_data.push(self.node_text(text).to_string());
                            }
                        }
                    }
                    if col_count > table_info.col_count {
                        table_info.col_count = col_count;
                    }
                    data.push(row_data);
                }
            }
        }

        //println!("table_info = {:?}, data={:?}", table_info, data);
        let mut pghview = PghView::new_table();
        for r in 0..table_info.row_count {
            for c in 0..table_info.col_count {
                if let Some(row) = data.get(r) {
                    if let Some(cell) = row.get(c) {
                        //println!("cell: {}", cell);
                        pghview.push_text(cell.to_string(), None);
                    } else {
                        //println!("cell: {}", "");
                        pghview.push_text("".to_string(), None);
                    }
                }
            }
        }
        pghview.table_info = Some(table_info);
        pghview
    }

    fn code_to_pghview(&self, node: &Node) -> PghView {
        let mut pghview = PghView::new_code();
        if let Node::Code(code) = node {
            pghview.code_lang = code.lang.clone();
            println!("code lang:{:?}", pghview.code_lang);
            for line in code.value.split('\n') {
                println!("code_line=[{}]", line);
                let text = line.to_string();
                pghview.push_text(text, None);
            }
        }
        pghview.spacing_top = 2.0;
        pghview.spacing_bottom = self.cfg.font_size / 3.0;
        pghview
    }

    fn node_to_pghview(&self, node: &Node) -> PghView {
        match node {
            Node::Paragraph(_) => self.paragraph_to_pghview(node, self.format_default()),
            Node::Heading(_) => self.heading_to_pghview(node),
            Node::List(_) => self.list_to_pghview(node),
            Node::Blockquote(_) => self.blockquote_to_pghview(node),
            Node::ThematicBreak(_) => self.thematicbreak_to_pghview(node),
            Node::Table(_) => self.table_to_pghview(node),
            Node::Code(_) => self.code_to_pghview(node),
            _ => {
                let mut pghview = PghView::new_text();
                if let Some(pos) = node.position() {
                    let s = &self.text[pos.start.offset..pos.end.offset];
                    pghview.push_text(s.to_string(), None);
                } else {
                    pghview.push_text("invalid postion".to_string(), None);
                }
                pghview
            }
        }
    }

    pub fn markdown_to_pghview(&self) -> PghView {
        if self.enable_markdown {
            if let Ok(ast) = markdown::to_mdast(&self.text, &markdown::ParseOptions::gfm()) {
                if let Some(items) = ast.children() {
                    if let Some(item) = items.first() {
                        //todo, only get first now
                        return self.node_to_pghview(item);
                    } else {
                        //empty line
                        let mut pghview = PghView::new_text();
                        pghview.push_indent();
                        pghview.push_text(self.text.clone(), None);
                        return pghview;
                    }
                }
            }
        } else {
            let mut pgh_view = PghView::new_text();
            pgh_view.push_text(self.text.clone(), None);
            return pgh_view;
        }

        PghView::new_text()
    }

    fn push_text(&self, node: &Node, pghvews: &mut Vec<PghView>) {
        let mut pghview = PghView::new_text();
        if let Some(pos) = node.position() {
            let s = &self.text[pos.start.offset..pos.end.offset];
            let mut line = s.to_string();
            line.retain(|c| c != '\n'); //删除同一个段落中的换行符号
            pghview.push_text(line, None);
            pghvews.push(pghview);
        }
    }

    fn push_table(&self, node: &Node, pghvews: &mut Vec<PghView>) {
        let pgh_view = self.table_to_pghview(node);
        pghvews.push(pgh_view);
    }

    fn push_code(&self, node: &Node, pghvews: &mut Vec<PghView>) {
        let pgh_view = self.code_to_pghview(node);
        pghvews.push(pgh_view);
    }

    fn node_to_pgh_text(&self, node: &Node, pghvews: &mut Vec<PghView>) {
        match node {
            Node::Paragraph(p) => {
                //println!("{:?}", node);
                self.push_text(node, pghvews);
            }
            Node::List(list) => {
                for item in &list.children {
                    //println!("{:?}", item);
                    self.push_text(item, pghvews);
                }
            }
            Node::Blockquote(block) => {
                if let Some(first) = block.children.first() {
                    let s = self.node_text(first);
                    for value in s.split('\n') {
                        let quote_s = if value.starts_with(">") {
                            format!("{}", value)
                        } else {
                            format!(">{}", value)
                        };
                        let mut pghview = PghView::new_block_line();
                        pghview.push_text(quote_s, None);
                        pghvews.push(pghview);
                    }
                } else {
                    let mut pghview = PghView::new_block_line();
                    pghview.push_text(">".to_string(), None);
                    pghvews.push(pghview);
                }
            }
            Node::Table(_) => {
                //println!("{:?}", node);
                self.push_table(node, pghvews);
            }
            Node::Code(_) => {
                self.push_code(node, pghvews);
            }
            _ => {
                //println!("{:?}", node);
                self.push_text(node, pghvews);
            }
        }
    }

    pub fn markdown_to_pgh_texts(&self) -> Vec<PghView> {
        let mut pghvews = vec![];
        if self.enable_markdown {
            //println!("{}");
            if let Ok(ast) = markdown::to_mdast(&self.text, &markdown::ParseOptions::gfm()) {
                if let Some(items) = ast.children() {
                    for item in items {
                        self.node_to_pgh_text(item, &mut pghvews);
                    }
                }
            }
        } else {
            for (no, line) in self.text.split('\n').enumerate() {
                let sline = line.to_string();
                let mut pgh_view = PghView::new_text();
                pgh_view.push_text(sline, None);
                pgh_view.spacing_top = TEXT_TOP_SPACE;
                pgh_view.spacing_bottom = TEXT_BOTTOM_SPACE;
                pghvews.push(pgh_view);
            }
        }

        //empty content, insert one empty line
        if pghvews.is_empty() {
            let mut pgh_view = PghView::new_text();
            pgh_view.push_text("".to_string(), None);
            pghvews.push(pgh_view);
        }

        pghvews
    }

    fn get_node_links(&self, node: &Node, links: &mut Vec<String>) {
        match node {
            Node::Text(p) => {
                for (start, end) in self.text_double_links(node) {
                    links.push(p.value[start+2..end-2].to_string());
                }
            }
            Node::Link(link) => {
                //todo, only support double links
                //links.push(link.url.clone());
            }
            _ => {
                if let Some(items) = node.children() {
                    for item in items {
                        self.get_node_links(item, links);
                    }
                }
            }
        }
    }

    pub fn markdown_get_links(&self) -> Vec<String> {
        let mut links:Vec<String> = vec![];
        if let Ok(ast) = markdown::to_mdast(&self.text, &markdown::ParseOptions::gfm()) {
            self.get_node_links(&ast, &mut links);
        }
        links
    }
}

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

pub fn echo_ast(md: &str, ast: &Node) {
    if let Some(pos) = ast.position() {
        let s = &md[pos.start.offset..pos.end.offset];
        println!("--------------------------\n{}\n{}", ast_type(ast), s);
    }
    if let Some(c) = ast.children() {
        for x in c {
            echo_ast(md, x);
        }
    }
}

#[test]
pub fn test_md() {
    let md = r#"
![image](file://test1.png)
"#;

    let ast = markdown::to_mdast(md, &markdown::ParseOptions::gfm()).unwrap();

    echo_ast(md, &ast);

    println!("{:?}", ast);
}
