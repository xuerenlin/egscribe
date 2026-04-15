use std::collections::HashMap;
use std::ops::Range;

use crate::medit::{CodeInfo, ImageInfo, PghView, TableInfo};
use crate::medit::pgh::PghType;
use crate::uicom::IconName;
use eframe::egui::epaint::text::{LayoutJob, TextFormat};
use eframe::egui::{FontFamily, Stroke};
use markdown;
use markdown::mdast::Node;
use markdown::unist::Position;
use regex::Regex;

use super::cfg::EditCfg;

#[derive(Clone, Debug)]
pub struct UrlInfo {
    pub url: String,
    pub title: Option<String>,
    pub text: String,
    pub pos: (usize, usize),
}

#[derive(Clone)]
pub enum LinkInfo {
    File(String),   //file
    Url(UrlInfo),   //url
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
    pub fn new_url(end_pos: usize, url_info: UrlInfo) -> Self {
        LinkEnd { end_pos, link_info: LinkInfo::Url(url_info) }
    }
    pub fn new_image(end_pos: usize, alt: String, url: String, url_range: Option<(usize, usize)>) -> Self {
        LinkEnd { end_pos, link_info: LinkInfo::Image(ImageInfo{alt, url, img:None, url_range}) }
    }
}

pub struct MarkDownImpl<'a> {
    text: String,
    enable_markdown: bool,
    curosr_char_index: Option<usize>,
    seleting: bool,
    cfg: &'a EditCfg,
    prefix: String,
    indent_level: usize,
}

impl<'a> MarkDownImpl<'a> {
    pub fn new(
        s: &str,
        enable_markdown: bool,
        curosr_char_index: Option<usize>,
        seleting: bool,
        cfg: &'a EditCfg,
    ) -> Self {
        let mut text = s.to_string();
        text = text.replace("\r\n", "\n");
        
        // 提取前缀和缩进层级
        let (prefix, indent_level, cleaned_text) = if enable_markdown {
            Self::extract_prefix_and_indent_impl(&text) 
        } else { 
            (String::new(), 0, text)
        };
        
        // 修正光标位置：如果光标位置在前缀范围内，需要减去前缀的字符长度
        let mut adjusted_cursor = curosr_char_index;
        if let Some(char_index) = curosr_char_index {
            let prefix_char_len = prefix.len();
            if char_index >= prefix_char_len {
                adjusted_cursor = Some(char_index - prefix_char_len);
            } else {
                adjusted_cursor = Some(0);
            }
        }
        
        MarkDownImpl {
            text:cleaned_text,
            enable_markdown,
            curosr_char_index: adjusted_cursor,
            seleting,
            cfg,
            prefix,
            indent_level,
        }
    }

    pub fn new_simple(s: &str, cfg: &'a EditCfg) -> Self {
        Self::new(s, true, None, false, cfg)
    }

    pub fn extract_prefix_and_indent(text: &str) -> (String, usize, String) {
        Self::extract_prefix_and_indent_impl(text)
    }
    
    fn extract_prefix_and_indent_impl(text: &str) -> (String, usize, String) {
        let mut prefix = String::new();
        let mut indent_level = 0;
        let mut pos = 0;
        let chars: Vec<char> = text.chars().collect();
        
        // 先提取开头的 \t 和空格
        let indent_start = pos;
        while pos < chars.len() {
            let ch = chars[pos];
            if ch == '\t' {
                prefix.push(ch);
                indent_level += 1;
                pos += 1;
            } else if ch == ' ' {
                prefix.push(ch);
                let mut space_count = 1;
                pos += 1;
                // 计算连续空格，每4个空格算一级缩进
                while pos < chars.len() && chars[pos] == ' ' {
                    space_count += 1;
                    prefix.push(' ');
                    pos += 1;
                }
                indent_level += space_count / 4;
            } else {
                break;
            }
        }
        
        // 如果没有空格/Tab，直接返回，不提取前缀
        if pos == indent_start {
            return (String::new(), 0, text.to_string());
        }
        
        // 检查后面是否有列表控制字符（-、*、+、数字+.等）
        let has_list_marker = if pos < chars.len() {
            let ch = chars[pos];
            if ch == '-' || ch == '*' || ch == '+' || ch == '>' {
                true
            } else if ch.is_ascii_digit() { //123.
                let num_start = pos;
                let mut check_pos = pos;
                while check_pos < chars.len() && chars[check_pos].is_ascii_digit() {
                    check_pos += 1;
                }
                check_pos < chars.len() && chars[check_pos] == '.'
            } else {
                false
            }
        } else {
            false
        };
        
        if !has_list_marker {
            return (String::new(), 0, text.to_string());
        }
        let cleaned_text: String = chars[pos..].iter().collect();
        (prefix, indent_level, cleaned_text)
    }

    fn apply_prefix_to_pghview(&self, pghview: &mut PghView) {
        if self.prefix.is_empty() {
            return;
        }
        if pghview.pgh_type == PghType::Table
            || pghview.pgh_type == PghType::TableRow
            || pghview.pgh_type == PghType::CodeRow
        {
            return;
        }
        if pghview.pgh_type == PghType::ListItem || pghview.pgh_type == PghType::BlockLine {
            for _ in 0..self.indent_level {
                pghview.insert_list_item_indent(0);
            }
        }

        let mut job: LayoutJob = LayoutJob::default();
        job.append(&self.prefix, 0.0, self.format_hide_prefix());
        pghview.insert_text_before_next_text(0, self.prefix.to_string(), Some(job));
    }

    pub fn format_default(&self) -> TextFormat {
        let mut format = TextFormat::default();
        format.font_id.size = self.cfg.font_size;
        format.font_id.family = self.cfg.font_family();
        format.color = self.cfg.text_color();
        format
    }

    fn format_code(&self) -> TextFormat {
        let mut format = TextFormat::default();
        format.font_id.size = self.cfg.font_size;
        format.font_id.family = FontFamily::Monospace; // Code blocks always use monospace
        format.color = self.cfg.text_color();
        format
    }

    fn format_hide(&self, left: &Range<usize>, right: &Range<usize>) -> TextFormat {
        let mut format = TextFormat::default();
        format.font_id.size = 0.1;
        if self.seleting {
            format.font_id.size = self.cfg.font_size;
        } else {
            if let Some(char_index) = self.curosr_char_index {
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

    fn format_hide_prefix(&self) -> TextFormat {
        let mut format = TextFormat::default();
        format.font_id.size = 0.1;
        if let Some(char_index) = self.curosr_char_index {
            if char_index == 0 {
                format.font_id.size = self.cfg.font_size;
            }
        }
        format
    }

    //deep: (between `1` and `6`, both including)
    fn format_head(&self, deep: u8) -> TextFormat {
        let mut format = self.format_default();
        format.font_id = self.cfg.heading_font_id(deep);
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

    fn text_double_links(&self, text_value: &str) -> Vec<(usize, usize)> {
        let re = Regex::new(r"\[\[(.*?)\]\]").unwrap();
        let info: Vec<_> = re.captures_iter(text_value)
            .map(|cap|{
                let start = cap.get(0).unwrap().start();
                let end = cap.get(0).unwrap().end();
                (start, end)
            }).collect();
        info
    }

    fn text_check_double_link(&self, node: &Node, job: &mut LayoutJob, link_ends: &mut Vec<LinkEnd>, format: &mut TextFormat) {
        if let Node::Text(text) = node {
            let text_value = self.node_text(node);
            let info = self.text_double_links(text_value);
            if info.is_empty() {
                job.append(&text_value, 0.0, format.clone());
            } else {
                let mut pre = 0 as usize;
                for x in info {
                    if x.0 > pre {
                        let value = &text_value[pre..x.0];
                        job.append(&value, 0.0, format.clone());
                    }

                    let value = &text_value[x.0 + 2..x.1 - 2];
                    let mut link_format = format.clone();
                    self.format_link(&mut link_format);
                    if let Some(pos) = node.position() {
                        let range_left = pos.start.offset + x.0..pos.start.offset + x.0 + 2;
                        let range_right = pos.start.offset + x.1 - 2..pos.start.offset + x.1;
                        job.append("[[", 0.0, self.format_hide(&range_left, &range_right));
                        job.append(value, 0.0, link_format);
                        job.append("]]", 0.0, self.format_hide(&range_left, &range_right));
                    } else {
                        job.append("[[", 0.0, format.clone());
                        job.append(value, 0.0, link_format);
                        job.append("]]", 0.0, format.clone());
                    }
                    link_ends.push(LinkEnd::new_file(job.sections.len(), value.to_string()));
                    pre = x.1;
                }
                let value = &text_value[pre..];
                if value.len() > 0 {
                    job.append(&value, 0.0, format.clone());
                }
            }
        }
    }

    fn format_inlinecode_node(
        &self,
        node: &Node,
        parent_pos: Option<&Position>,
        job: &mut LayoutJob,
        format: &mut TextFormat,
    ) {
        if let Some(pos) = node.position() {
            let mut new_format = format.clone();
            self.format_inlinecode(&mut new_format);
            let node_text = self.node_text(node);
            if node_text.starts_with('`') && node_text.ends_with('`') {
                let range_left = pos.start.offset..pos.start.offset+1;
                let range_right = pos.end.offset-1..pos.end.offset;
                let code_content = &node_text[1..node_text.len() - 1];
                job.append("`", 0.0, self.format_hide(&range_left, &range_right));
                job.append(code_content, 0.0, new_format);
                job.append("`", 0.0, self.format_hide(&range_left, &range_right));
            } else {
                job.append(node_text, 0.0, new_format);
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
                    
                    let url_range = if let Some(pos) = node.position() {
                        // Convert byte offset to char offset
                        let byte_start = pos.start.offset;
                        let byte_end = pos.end.offset;
                        let char_start = self.text[..byte_start].chars().count();
                        let char_end = self.text[..byte_end].chars().count();
                        Some((char_start, char_end))
                    } else {
                        None
                    };
                    link_ends.push(LinkEnd::new_image(job.sections.len(),  image.alt.clone(), image.url.clone(), url_range));
                }
                Node::InlineCode(_) => {
                    self.format_inlinecode_node(node, parent_pos, job, format);
                }
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
                    link_url = Some(link);
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
                if last && pos.end.offset < parent_pos.end.offset{
                    let range_left = parent_pos.start.offset..pos.start.offset;
                    let range_right = pos.end.offset..parent_pos.end.offset;
                    let ctrl_right = &self.text[range_right.clone()];
                    if ctrl_right.trim_end().is_empty() { //all space, do not hide it 
                        job.append(ctrl_right, 0.0, format.clone());
                    } else {
                        job.append(ctrl_right, 0.0, self.format_hide(&range_left, &range_right));
                    }
                }
            }
        }

        if let Some(link_url) = link_url {
            if let Some(pos) = &link_url.position {
                let url_info = UrlInfo {
                    url: link_url.url.to_owned(),
                    title: link_url.title.to_owned(),
                    text: self.text[pos.start.offset..pos.end.offset].to_string(),
                    pos: (pos.start.offset, pos.end.offset),
                };
                link_ends.push(LinkEnd::new_url(job.sections.len(),  url_info));
            }
        }
    }

    pub fn paragraph_push_to_pghview(&self, node: &Node, format: TextFormat, pghview: &mut PghView) {
        let mut job: LayoutJob = LayoutJob::default();
        let mut link_ends = vec![];
        let mut format = format;
        if let Some(pos) = node.position() {
            self.paragraph_format(node, None, false, false, &mut job, &mut link_ends, &mut format);
            //let total_s = &self.text[pos.start.offset..pos.end.offset];
            let total_s = &job.text;

            let mut sub_job: LayoutJob = LayoutJob::default();
            let mut seg_str = String::new();
            for (i, x) in job.sections.iter().enumerate() {
                let sub_str = total_s[x.byte_range.clone()].to_string();
                sub_job.append(&sub_str, 0.0, x.format.clone());
                seg_str += &sub_str;

                if let Some(link) = link_ends.iter().find(|end| end.end_pos == i+1) {
                    pghview.push_text(seg_str, Some(sub_job));
                    
                    //push link pgh_segment
                    pghview.push_icon_with_link(IconName::icon_external_link, link.link_info.clone());

                    //push image pgh_segment
                    if let LinkInfo::Image(image_info) = &link.link_info {
                        pghview.push_image(image_info.to_owned());
                    }

                    if i+1 == job.sections.len() {
                        pghview.push_text("".to_string(), None);
                    }

                    sub_job = LayoutJob::default();
                    seg_str = String::new();
                }
            }
            if seg_str.len() > 0 {
                pghview.push_text(seg_str, Some(sub_job));
            }
        }
    }

    fn paragraph_text_space(&self, pghview: &mut PghView){
        pghview.spacing_top = self.cfg.spacing.paragraph.top;
        pghview.spacing_bottom = self.cfg.spacing.paragraph.bottom;
    }

    fn paragraph_to_pghview(&self, node: &Node, format: TextFormat) -> PghView {
        let mut pghview = PghView::new_text();
        pghview.push_indent();
        self.paragraph_push_to_pghview(node, format, &mut pghview);
        self.paragraph_text_space(&mut pghview);
        pghview
    }

    fn heading_to_pghview(&self, node: &Node) -> PghView {
        let mut depth = 0;
        if let Node::Heading(h) = node {
            depth = h.depth
        }
        let mut pghview = PghView::new_heading();
        self.paragraph_push_to_pghview(node, self.format_head(depth), &mut pghview);
        pghview.spacing_top = self.cfg.spacing.heading.top;
        pghview.spacing_bottom = self.cfg.spacing.heading.bottom;
        pghview
    }

    fn list_to_pghview(&self, node: &Node) -> PghView {
        let mut pghview = PghView::new_list_item();
        if let Node::List(list) = node {
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
                            let listordered = list.ordered;
                            pghview.push_point();
                        }
                    }
    
                    self.paragraph_push_to_pghview(list_node, format, &mut pghview);
                }
            }
        }
        pghview.spacing_top = self.cfg.spacing.list.top;
        pghview.spacing_bottom = self.cfg.spacing.list.bottom;
        pghview
    }

    fn blockquote_to_pghview(&self, node: &Node) -> PghView {
        let mut pghview = PghView::new_block_line();
        if let Some(items) = node.children() {
            if let Some(list_node) = items.first() {
                pghview.push_quote_indent();

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
        table_info.frame_style = self.cfg.table_frame_style.clone();
        let mut data: Vec<Vec<LayoutJob>> = vec![];
        if let Some(table) = node.children() {
            for row in table {
                if let Some(cols) = row.children() {
                    table_info.row_count += 1;
                    let mut row_data = vec![];
                    let mut col_count = 0;
                    for col in cols {
                        let mut job: LayoutJob = LayoutJob::default();
                        let mut link_ends = vec![];
                        let mut format = self.format_default();
                        if let Some(children) = col.children() {
                            for (i, child) in children.iter().enumerate() {
                                let is_first = i == 0;
                                let is_last = i + 1 == children.len();
                                self.paragraph_format(child, None, is_first, is_last, &mut job, &mut link_ends, &mut format);
                            }
                        }
                        col_count += 1;
                        row_data.push(job);
                    }
                    if col_count > table_info.col_count {
                        table_info.col_count = col_count;
                    }
                    data.push(row_data);
                }
            }
        }

        let mut pghview = PghView::new_table();
        for r in 0..table_info.row_count {
            for c in 0..table_info.col_count {
                if let Some(row) = data.get(r) {
                    if let Some(cell_job) = row.get(c) {
                        pghview.push_text(cell_job.text.clone(), Some(cell_job.clone()));
                    } else {
                        pghview.push_text("".to_string(), None);
                    }
                }
            }
        }
        table_info.table_total_rows = table_info.row_count;
        table_info.table_row_index = 0;
        pghview.table_info = Some(table_info);
        pghview
    }

    /// 将 AST 表格展开为每行一个 `PghType::TableRow`（用于 `markdown_to_pgh_texts` / 管道块合并）
    pub(crate) fn table_to_table_row_pghviews(&self, node: &Node) -> Vec<PghView> {
        let mut table_info = TableInfo::default();
        table_info.frame_style = self.cfg.table_frame_style.clone();
        let mut data: Vec<Vec<LayoutJob>> = vec![];
        if let Some(table) = node.children() {
            for row in table {
                if let Some(cols) = row.children() {
                    table_info.row_count += 1;
                    let mut row_data = vec![];
                    let mut col_count = 0;
                    for col in cols {
                        let mut job: LayoutJob = LayoutJob::default();
                        let mut link_ends = vec![];
                        let mut format = self.format_default();
                        if let Some(children) = col.children() {
                            for (i, child) in children.iter().enumerate() {
                                let is_first = i == 0;
                                let is_last = i + 1 == children.len();
                                self.paragraph_format(
                                    child,
                                    None,
                                    is_first,
                                    is_last,
                                    &mut job,
                                    &mut link_ends,
                                    &mut format,
                                );
                            }
                        }
                        col_count += 1;
                        row_data.push(job);
                    }
                    if col_count > table_info.col_count {
                        table_info.col_count = col_count;
                    }
                    data.push(row_data);
                }
            }
        }

        let total = table_info.row_count;
        let mut out = Vec::with_capacity(total);
        for r in 0..total {
            let mut pghview = PghView::new_table_row();
            let mut row_ti = table_info.clone();
            row_ti.row_count = 1;
            row_ti.table_row_index = r;
            row_ti.table_total_rows = total;
            for c in 0..row_ti.col_count {
                if let Some(row) = data.get(r) {
                    if let Some(cell_job) = row.get(c) {
                        pghview.push_text(cell_job.text.clone(), Some(cell_job.clone()));
                    } else {
                        pghview.push_text("".to_string(), None);
                    }
                }
            }
            pghview.table_info = Some(row_ti);
            out.push(pghview);
        }
        out
    }

    /// 将 `Code` 节点展开为多行 `PghType::CodeRow`（与 `table_to_table_row_pghviews` 对称）
    pub(crate) fn code_to_code_row_pghviews(&self, node: &Node) -> Vec<PghView> {
        // Tab 或空格缩进的代码行，当成文本处理
        if !self.node_text(node).starts_with("```") {
            let mut pghview = PghView::new_text();
            pghview.push_indent();
            pghview.push_text(self.node_text(node).to_string(), None);
            self.paragraph_text_space(&mut pghview);
            return vec![pghview];
        }
        let Node::Code(code) = node else {
            return vec![];
        };
        let lines: Vec<&str> = code.value.split('\n').collect();
        let total = lines.len().max(1);
        let mut out = Vec::with_capacity(total);
        let top = self.cfg.spacing.code.top;
        let bottom = self.cfg.spacing.code.bottom;
        for (r, line) in lines.iter().enumerate() {
            let mut pghview = PghView::new_code_row();
            pghview.push_text((*line).to_string(), None);
            pghview.code_info = Some(CodeInfo {
                code_row_index: r,
                code_total_rows: total,
            });
            if r == 0 {
                pghview.code_lang = code.lang.clone();
                pghview.spacing_top = top;
            } else {
                pghview.spacing_top = 0.0;
            }
            if r + 1 == total {
                pghview.spacing_bottom = bottom;
            } else {
                pghview.spacing_bottom = 0.0;
            }
            out.push(pghview);
        }
        if out.is_empty() {
            let mut pghview = PghView::new_code_row();
            pghview.push_text(String::new(), None);
            pghview.code_info = Some(CodeInfo {
                code_row_index: 0,
                code_total_rows: 1,
            });
            pghview.code_lang = code.lang.clone();
            pghview.spacing_top = top;
            pghview.spacing_bottom = bottom;
            out.push(pghview);
        }
        log::debug!("code lang:{:?}, rows={}", code.lang, out.len());
        out
    }

    /// 单行 `markdown_to_pghview` 用：多行 fenced 时折叠为单行 `CodeRow`（保留全文），整篇解析见 `markdown_to_pgh_texts`
    fn code_to_pghview(&self, node: &Node) -> PghView {
        let rows = self.code_to_code_row_pghviews(node);
        if rows.is_empty() {
            return PghView::new_text();
        }
        if rows.len() == 1 {
            return rows.into_iter().next().unwrap();
        }
        let mut merged = rows[0].clone();
        let full_body: String = rows.iter().map(PghView::get_text).collect::<Vec<_>>().join("\n");
        merged.pgh.clear();
        merged.push_text(full_body, None);
        if let Some(ci) = merged.code_info.as_mut() {
            ci.code_row_index = 0;
            ci.code_total_rows = 1;
        }
        merged.spacing_bottom = rows
            .last()
            .map(|r| r.spacing_bottom)
            .unwrap_or(merged.spacing_bottom);
        merged
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

    // 解析 markdown 文本并处理前缀
    fn parse_markdown_with_prefix(&self) -> PghView {
        let mut pghview: PghView = PghView::new_text();
        if let Ok(ast) = markdown::to_mdast(&self.text, &markdown::ParseOptions::gfm()) {
            if let Some(items) = ast.children() {
                if let Some(item) = items.first() {
                    //todo, only get first now
                    pghview = self.node_to_pghview(item)
                } else {
                    pghview.push_indent();
                    pghview.push_text(self.text.clone(), None);
                    self.paragraph_text_space(&mut pghview);
                }
            }
        }

        self.apply_prefix_to_pghview(&mut pghview);
        pghview
    }

    pub fn markdown_to_pghview(&self) -> PghView {
        if self.enable_markdown {
            self.parse_markdown_with_prefix()
        } else {
            let mut pgh_view = PghView::new_text();
            pgh_view.push_text(self.text.clone(), None);
            pgh_view
        }
    }

    /// 若整段文本解析为单个 `Code` 根节点，返回多行 `PghType::CodeRow`（供 `Ctx` 围栏块合并）
    pub(crate) fn markdown_to_code_rows_if_single_code(&self) -> Option<Vec<PghView>> {
        if !self.enable_markdown {
            return None;
        }
        if let Ok(ast) = markdown::to_mdast(&self.text, &markdown::ParseOptions::gfm()) {
            if let Some(items) = ast.children() {
                if items.len() == 1 {
                    if let Node::Code(_) = &items[0] {
                        return Some(self.code_to_code_row_pghviews(&items[0]));
                    }
                }
            }
        }
        None
    }

    /// 若整段文本解析为单个 GFM `Table` 根节点，返回多行 `PghType::TableRow`（供 `Ctx` 管道块合并）
    pub(crate) fn markdown_to_table_rows_if_single_table(&self) -> Option<Vec<PghView>> {
        if !self.enable_markdown {
            return None;
        }
        if let Ok(ast) = markdown::to_mdast(&self.text, &markdown::ParseOptions::gfm()) {
            if let Some(items) = ast.children() {
                if items.len() == 1 {
                    if let Node::Table(_) = &items[0] {
                        return Some(self.table_to_table_row_pghviews(&items[0]));
                    }
                }
            }
        }
        None
    }

    fn push_text(&self, node: &Node, pghviews: &mut Vec<PghView>) {
        let mut pghview = PghView::new_text();
        if let Some(pos) = node.position() {
            let s = &self.text[pos.start.offset..pos.end.offset];
            let mut line = s.to_string();

            //delete \n in one paragraph
            line.retain(|c| c != '\n'); 

            pghview.push_text(line, None);
            pghviews.push(pghview);
        }
    }

    fn push_text_from_line_start(&self, node: &Node, pghviews: &mut Vec<PghView>) {
        let mut pghview = PghView::new_text();
        if let Some(pos) = node.position() {
            // 找到 pos.start.offset 所在行的第一个字符位置
            let line_start = if pos.start.offset > 0 {
                // 向前查找最近的 \n，行首是 \n 之后的位置
                self.text[..pos.start.offset]
                    .rfind('\n')
                    .map(|idx| idx + 1)
                    .unwrap_or(0)
            } else {
                0
            };

            // 从行首到 pos.end.offset 提取文本
            let s = &self.text[line_start..pos.end.offset];
            let line = s.to_string();

            pghview.push_text(line, None);
            pghviews.push(pghview);
        }
    }

    fn push_table(&self, node: &Node, pghviews: &mut Vec<PghView>) {
        //TODO: 删除Table模式下的table_to_pghview，只使用table_to_table_row_pghviews
        //let pghview = self.table_to_pghview(node);
        //pghviews.push(pghview);

        for row in self.table_to_table_row_pghviews(node) {
            pghviews.push(row);
        }
    }

    fn push_code(&self, node: &Node, pghviews: &mut Vec<PghView>) {
        for row in self.code_to_code_row_pghviews(node) {
            pghviews.push(row);
        }
    }

    fn list_to_pgh_text_recursive(&self, node: &Node, pghviews: &mut Vec<PghView>, indent_level: usize) {
        if let Some(items) = node.children() {
            for list_node in items {
                if let Node::ListItem(_) = list_node {
                    // 处理列表项的内容（段落等，但不包括嵌套列表）
                    if let Some(children) = list_node.children() {
                        if let Some(child) = children.first() {
                            match child {
                                Node::List(_) => {}
                                _ => {
                                    self.push_text_from_line_start(&child, pghviews);
                                }
                            }
                        }
                    } else {
                        // 没有子节点，添加空文本
                        self.push_text(list_node, pghviews);
                    }

                    // 递归处理嵌套列表
                    if let Some(children) = list_node.children() {
                        for child in children {
                            if let Node::List(_) = child {
                                self.list_to_pgh_text_recursive(child, pghviews, indent_level + 1);
                            }
                        }
                    }
                }
            }
        }
    }

    fn node_to_pgh_text(&self, node: &Node, pghviews: &mut Vec<PghView>) {
        match node {
            Node::Paragraph(_) => {
                let pghview = self.paragraph_to_pghview(node, self.format_default());
                pghviews.push(pghview);
            }
            Node::List(list) => {
                self.list_to_pgh_text_recursive(node, pghviews, 0);
            }
            Node::Blockquote(block) => {
                if let Some(first) = block.children.first() {
                    let s = self.node_text(first);
                    let re = Regex::new(r"^\s*>").unwrap();
                    for value in s.split('\n') {
                        let quote_s = if re.is_match(value) {
                            format!("{}", value)
                        } else {
                            format!(">{}", value)
                        };
                        let mut pghview = PghView::new_block_line();
                        pghview.push_text(quote_s, None);
                        pghviews.push(pghview);
                    }
                } else {
                    let mut pghview = PghView::new_block_line();
                    pghview.push_text(">".to_string(), None);
                    pghviews.push(pghview);
                }
            }
            Node::Table(_) => {
                //println!("{:?}", node);
                self.push_table(node, pghviews);
            }
            Node::Code(_) => {
                self.push_code(node, pghviews);
            }
            Node::Heading(_) => {
                let pgh_view = self.heading_to_pghview(node);
                pghviews.push(pgh_view);
            }
            _ => {
                self.push_text(node, pghviews);
            }
        }
    }

    fn push_root_gap_empty_lines(&self, count: usize, pghviews: &mut Vec<PghView>) {
        for _ in 0..count {
            let mut pgh_view = PghView::new_text();
            pgh_view.push_text(String::new(), None);
            pgh_view.spacing_top = self.cfg.spacing.text.top;
            pgh_view.spacing_bottom = self.cfg.spacing.text.bottom;
            pghviews.push(pgh_view);
        }
    }

    pub fn markdown_to_pgh_texts(&self) -> Vec<PghView> {
        let mut pghviews = vec![];
        if self.enable_markdown {
            if let Ok(ast) = markdown::to_mdast(&self.text, &markdown::ParseOptions::gfm()) {
                if let Some(items) = ast.children() {
                    let mut prev_end: usize = 0;
                    let mut any_positioned = false;
                    for item in items {
                        if let Some(pos) = item.position() {
                            any_positioned = true;
                            let empty_lines = self.root_gap_empty_lines_before_node(prev_end, item);
                            if empty_lines > 0 {
                                self.push_root_gap_empty_lines(empty_lines, &mut pghviews);
                            }
                            self.node_to_pgh_text(item, &mut pghviews);
                            prev_end = pos.end.offset;
                        } else {
                            self.node_to_pgh_text(item, &mut pghviews);
                        }
                    }
                    if any_positioned && prev_end < self.text.len() {
                        let n = Self::root_tail_empty_line_count(&self.text[prev_end..]);
                        self.push_root_gap_empty_lines(n, &mut pghviews);
                    }
                }
            }

            //带换行的文本拆分成多行
            pghviews = pghviews
                .into_iter()
                .flat_map(PghView::split_text_by_embedded_newlines)
                .collect();

        } else {
            for (no, line) in self.text.split('\n').enumerate() {
                let sline = line.to_string();
                let mut pgh_view = PghView::new_text();
                pgh_view.push_text(sline, None);
                pgh_view.spacing_top = self.cfg.spacing.text.top;
                pgh_view.spacing_bottom = self.cfg.spacing.text.bottom;
                pghviews.push(pgh_view);
            }
        }

        //empty content, insert one empty line
        if pghviews.is_empty() {
            let mut pgh_view = PghView::new_text();
            pgh_view.push_text("".to_string(), None);
            pghviews.push(pgh_view);
        }

        pghviews
    }

    fn get_node_links(&self, node: &Node, links: &mut HashMap<String, ()>) {
        match node {
            Node::Text(_) => {
                let text_value = self.node_text(node);
                for (start, end) in self.text_double_links(text_value) {
                    links.insert(text_value[start + 2..end - 2].to_string(), ());
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

    pub fn markdown_get_links(&self) -> HashMap<String, ()> {
        let mut links: HashMap<String, ()> = HashMap::new();
        if let Ok(ast) = markdown::to_mdast(&self.text, &markdown::ParseOptions::gfm()) {
            self.get_node_links(&ast, &mut links);
        }
        links
    }
}

/// 统计根级节点之间应还原的空行数。
impl<'a> MarkDownImpl<'a> {
    fn root_gap_is_whitespace_only(gap: &str) -> bool {
        !gap.is_empty()
            && gap
                .chars()
                .all(|c| c == '\n' || c == '\r' || c == ' ' || c == '\t')
    }

    fn root_gap_newline_count(gap: &str) -> usize {
        gap.chars().filter(|&c| c == '\n').count()
    }

    /// 根级 gap 后紧跟 [`Node::ThematicBreak`] 时，其中一对换行是 GFM 把 `---` 与上一块分开的语法空隙，
    /// 不应再还原成独立空 `PghView`（否则会多出一行空行）。
    fn adjust_root_gap_for_next_node(blank_lines: usize, next: &Node) -> usize {
        if matches!(next, Node::ThematicBreak(_)) {
            blank_lines.saturating_sub(1)
        } else {
            blank_lines
        }
    }

    /// 根级相邻 AST 节点之间、仅含空白/换行的源码子串 → 应插入的空行数（与 `get_all_text` 按行用 `\n` 拼接一致）。
    /// `N` 个换行符对应 `N - 1` 条视觉空行。
    pub fn root_gap_empty_line_count(gap: &str) -> usize {
        if !Self::root_gap_is_whitespace_only(gap) {
            return 0;
        }
        Self::root_gap_newline_count(gap).saturating_sub(1)
    }

    /// 根级最后一个节点之后到文末的空白段。
    /// 文末 `N` 个换行应保留为 `N` 条尾部空行（例如以 `\n` 结尾会保留最后空行）。
    fn root_tail_empty_line_count(gap: &str) -> usize {
        if !Self::root_gap_is_whitespace_only(gap) {
            return 0;
        }
        Self::root_gap_newline_count(gap)
    }

    pub fn root_gap_empty_line_count_before_node(gap: &str, next: &Node) -> usize {
        let blank_lines = Self::root_gap_empty_line_count(gap);
        Self::adjust_root_gap_for_next_node(blank_lines, next)
    }

    /// 统计两个根级节点之间应还原的空行数。
    /// 某些节点（例如列表）的 `position.end.offset` 可能已经覆盖了一个换行，
    /// 因此会把 `gap_start` 前一个 `\n` 也纳入换行总数。
    pub fn root_gap_empty_line_count_between(
        text: &str,
        gap_start: usize,
        gap_end: usize,
        next: &Node,
    ) -> usize {
        if gap_start >= gap_end || gap_start > text.len() || gap_end > text.len() {
            return 0;
        }

        let gap = &text[gap_start..gap_end];
        if !Self::root_gap_is_whitespace_only(gap) {
            return 0;
        }

        let mut newline_count = Self::root_gap_newline_count(gap);
        if gap_start > 0 && text.as_bytes()[gap_start - 1] == b'\n' {
            newline_count += 1;
        }

        let blank_lines = newline_count.saturating_sub(1);
        Self::adjust_root_gap_for_next_node(blank_lines, next)
    }

    fn root_gap_empty_lines_before_node(&self, prev_end: usize, next: &Node) -> usize {
        let Some(pos) = next.position() else {
            return 0;
        };
        let gap_start = prev_end.min(self.text.len());
        let gap_end = pos.start.offset.min(self.text.len());
        if gap_start >= gap_end {
            return 0;
        }
        Self::root_gap_empty_line_count_between(&self.text, gap_start, gap_end, next)
    }
}
