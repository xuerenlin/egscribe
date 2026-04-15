use eframe::egui::epaint::text::{LayoutJob, TextFormat};
use eframe::egui::{
    Align, Color32, CursorIcon, FontFamily, FontId, Frame, Layout, Response, RichText, Ui,
    Widget, vec2,
};
use super::pgh_items::PghIndent;
use crate::medit::{Ctx, PghText, TextSpacing};
use crate::sitter::{LightSlice, highlight_lines, support_lang};
use super::{LayoutResponse, PghView, SegmentType};

/// impl code
impl PghView {
    pub fn code_format(slice: &LightSlice, ui: &Ui, ctx: &Ctx) -> TextFormat {
        let color = if ctx.cfg().dark_mode{
            slice.dark_color
        } else {
            slice.light_color
        };
        let brightness = ctx.cfg().text_color_brightness;
        let color = color.linear_multiply(ctx.cfg().text_color_brightness);

        let mut format = TextFormat::default();
        format.font_id.size = ctx.font_size();
        format.font_id.family = FontFamily::Monospace;
        format.color = color;
        format
    }

    fn code_highlight_job(ui: &Ui, ctx: &mut Ctx, line_no: usize, is_line_changed: bool) {
        if !is_line_changed {
            return;
        }
        let Some((blk_s, blk_e)) = ctx.code_row_block_range(line_no) else {
            return;
        };
        let Some(code_lang) = ctx.get_line(blk_s).and_then(|p| p.code_lang.clone()) else {
            return;
        };

        // 与 `Ctx::highlight_range_text` 一致：只对「当前视口上扩」的连续行做 tree-sitter，避免超大 CodeRow 块每次改一行就全块高亮。
        const VIEW_MARGIN: usize = 20;
        let vis_top = ctx.top_line().saturating_sub(VIEW_MARGIN);
        let vis_end = ctx.patch_end().saturating_sub(1);
        let mut clip_s = blk_s.max(vis_top);
        let mut clip_e = blk_e.min(vis_end);
        if clip_s > clip_e {
            clip_s = line_no.saturating_sub(VIEW_MARGIN).max(blk_s);
            clip_e = (line_no.saturating_add(VIEW_MARGIN)).min(blk_e);
        }

        let mut partial_src = String::new();
        for ln in clip_s..=clip_e {
            let Some(p) = ctx.get_line(ln) else {
                continue;
            };
            if ln > clip_s {
                partial_src.push('\n');
            }
            partial_src.push_str(&p.text_to_vec().join("\n"));
        }

        let n_clip = clip_e.saturating_sub(clip_s) + 1;
        if let Ok(hl_lines) = highlight_lines(code_lang, partial_src.as_bytes()) {
            for i in 0..n_clip {
                let physical_ln = clip_s + i;
                let job = hl_lines.get(i).and_then(|line_slices| {
                    if line_slices.is_empty() {
                        return None;
                    }
                    let mut job: LayoutJob = LayoutJob::default();
                    for slice in line_slices {
                        job.append(
                            &String::from_utf8_lossy(slice.slice),
                            0.0,
                            Self::code_format(slice, ui, ctx),
                        );
                    }
                    Some(job)
                });
                ctx.update_pgh_segment_job(physical_ln, 0, job);
            }
        }
    }

    fn code_lang_menu(ctx: &mut Ctx, ui: &mut Ui, line_no: usize) {
        ui.add(CodeLangMenu::new(ctx, line_no));
    }
}

/// Code language menu widget
pub struct CodeLangMenu<'a> {
    ctx: &'a mut Ctx,
    line_no: usize,
}

impl<'a> CodeLangMenu<'a> {
    pub fn new(ctx: &'a mut Ctx, line_no: usize) -> Self {
        Self { ctx, line_no }
    }
}

impl<'a> Widget for CodeLangMenu<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let lang_line_no = self
            .ctx
            .code_row_block_range(self.line_no)
            .map(|(s, _)| s)
            .unwrap_or(self.line_no);
        let cur_lang = self
            .ctx
            .get_line(lang_line_no)
            .and_then(|p| p.code_lang.clone())
            .unwrap_or_else(|| "Lang".to_string());
        let r = ui
            .with_layout(Layout::right_to_left(Align::TOP), |ui| {
                ui.menu_button(cur_lang, |ui| {
                    for lang in support_lang() {
                        if ui.button(lang).clicked() {
                            if let Some(update) = self.ctx.get_line_mut(lang_line_no) {
                                update.code_lang = Some(lang.to_string());
                                self.ctx.line_change_tick(lang_line_no);
                            }
                            self.ctx.on_content_change();
                            ui.close();
                        }
                    }
                })
                .response
            })
            .inner;
        r
    }
}

impl PghView {
    /// 代码块行左侧：按块内总行数对齐宽度的行号（右对齐）。
    fn layout_code_row_line_no_gutter(ui: &mut Ui, ctx: &Ctx, line_no: usize) -> Response {
        let Some(p) = ctx.get_line(line_no) else {
            return ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1;
        };
        let total = p
            .code_info
            .as_ref()
            .map(|ci| ci.code_total_rows)
            .unwrap_or(1)
            .max(1);
        let idx = p
            .code_info
            .as_ref()
            .map(|ci| ci.code_row_index)
            .unwrap_or(0);
        let display_line_no = idx + 1;
        let gutter_font = FontId::new(ctx.font_size(), FontFamily::Monospace);
        let gutter_w = {
            let job = LayoutJob::simple_singleline(
                "8".repeat(total.to_string().len()),
                gutter_font.clone(),
                Color32::PLACEHOLDER,
            );
            ui.fonts_mut(|f| f.layout_job(job).size().x) + 8.0
        };
        ui.allocate_ui_with_layout(
            vec2(gutter_w, ctx.font_heigh()),
            Layout::right_to_left(Align::Center),
            |ui| {
                ui.label(
                    RichText::new(format!("{display_line_no}"))
                        .font(gutter_font)
                        .color(ui.style().visuals.weak_text_color()),
                );
            },
        )
        .response
    }

    pub fn layout_code_line(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
        is_line_changed: bool,
    ) -> LayoutResponse {
        // 段前/段后空隙：解析时由 md 只在代码块首行写 spacing_top、末行写 spacing_bottom；
        // 编辑导致块结构变化后由 `Ctx::refresh_code_row_block_metadata` 再同步。此处按 `code_info`
        // 再约束一层，避免中间行误带旧间距。
        let (spacing_top, spacing_bottom, num_segments) = {
            let Some(p) = ctx.get_line(line_no) else {
                return LayoutResponse::from_response(
                    ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1,
                );
            };
            let (top, bottom) = match &p.code_info {
                Some(ci) => {
                    let is_first = ci.code_row_index == 0;
                    let is_last = ci.code_row_index + 1 == ci.code_total_rows;
                    (
                        if is_first { p.spacing_top } else { 0.0 },
                        if is_last { p.spacing_bottom } else { 0.0 },
                    )
                }
                None => (p.spacing_top, p.spacing_bottom),
            };
            (top, bottom, p.pgh.len())
        };

        let mut lang_menu_response = None;
        // 块首/块末的段前段后间隙画在 Frame 外，避免被 faint 底色铺满
        let mut top_rect = ui.cursor();
        top_rect.set_right(ctx.edit_right());
        top_rect.set_height(spacing_top);
        let mut response = ui.allocate_rect(top_rect, ctx.sense());

        let frame = Frame::default()
            //.stroke(Stroke::new(1.0, ui.style().visuals.faint_bg_color))
            .fill(ui.style().visuals.faint_bg_color)
            .corner_radius(1.0)
            .inner_margin(1.0);

        let frame_response = frame.show(ui, |ui| {
            //highlight
            Self::code_highlight_job(ui, ctx, line_no, is_line_changed);

            //layout
            for segment in 0..num_segments {
                let seg_type = ctx
                    .get_line(line_no)
                    .and_then(|p| p.pgh.get(segment))
                    .map(|s| s.seg_type.clone());
                let Some(seg_type) = seg_type else {
                    continue;
                };
                ui.horizontal(|ui| {
                    response |= Self::layout_code_row_line_no_gutter(ui, ctx, line_no);
                    response |= PghIndent::layout_paragraph(ui, ctx, line_no, segment, ctx.cfg().indent_size);
                    let need_expand = true;
                    let keep_space = 0.0;
                    match seg_type {
                        SegmentType::Text => {
                            let (text, job) = {
                                let p = ctx.get_line(line_no).unwrap();
                                let seg = &p.pgh[segment];
                                (seg.item.text(), seg.item.layout_job())
                            };
                            let mut item_rect = ui.cursor();
                            item_rect.set_right(ctx.edit_right());
                            let warp_width = Self::get_text_warp_width_base_cursor(ui, ctx, keep_space);
                            let spacing = TextSpacing::text_spacing_in_rect(item_rect, warp_width)
                                .with_spacing_top_bottom(spacing_top, spacing_bottom)
                                .with_need_expand(need_expand)
                                .with_once_allocate(false)
                                .with_first_row_indentation(ui);
                            let r = PghText::layout_paragraph(
                                ui,
                                ctx,
                                line_no,
                                segment,
                                spacing,
                                text,
                                &job,
                            );
                            let show_lang_menu = ctx.get_line(line_no).is_some_and(|p| {
                                p.code_info
                                    .as_ref()
                                    .map(|ci| ci.code_row_index == 0)
                                    .unwrap_or(true)
                            });
                            if segment == 0 && show_lang_menu {
                                let lang_menu_r = ui.put(r.rect, CodeLangMenu::new(ctx, line_no));
                                lang_menu_response = Some(lang_menu_r);
                            }
                            response |= r;
                        }
                        _ => {}
                    }
                });
            }

            response.on_hover_cursor(CursorIcon::Text)
        });

        response = frame_response.inner;
        let mut bottom_rect = ui.cursor();
        bottom_rect.set_right(ctx.edit_right());
        bottom_rect.set_height(spacing_bottom);
        response |= ui.allocate_rect(bottom_rect, ctx.sense());

        if let Some(lang_menu_response) = lang_menu_response {
            lang_menu_response.on_hover_cursor(CursorIcon::Default);
        }

        LayoutResponse::new(response, false)
    }
}
