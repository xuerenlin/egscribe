use eframe::egui::epaint::text::{LayoutJob, TextFormat};
use eframe::egui::{
    Align, Color32, CursorIcon, FontFamily, FontId, Frame, Image, Layout, NumExt, Pos2, Rect, Response, RichText, Sense,
    Stroke, StrokeKind, Ui,
    Widget, vec2,
};
use super::pgh_items::{PghBreak, PghIndent};
use crate::medit::{Ctx, PghText, TextSpacing};
use egscribe_sitter::{LightSlice, highlight_lines, support_lang};
use crate::uicom::CONTROL_HIGHLIGHT;
use super::{LayoutResponse, PghView, SegmentType};

/// impl code
impl PghView {
    pub(crate) fn layout_code_plantuml_image(
        ui: &mut Ui,
        ctx: &mut Ctx,
        line_no: usize,
    ) -> Option<(Response, Rect)> {
        let Some(code_info) = ctx.code_info_of_line(line_no) else {
            return None;
        };
        if code_info.code_row_index + 1 != code_info.code_total_rows {
            return None;
        }
        let Some(image_url) = ctx.code_plantuml_image_of_line(line_no) else {
            return None;
        };

        // 在代码与图片之间插入 PghBreak 同款分隔线（仅绘制，不写入 view）。
        let sep_row_height = (ctx.font_heigh() * 0.45).at_least(6.0);
        let sep_cursor_rect = ui.cursor();
        let sep_right = ctx.edit_right().min(ui.max_rect().right());
        let sep_width = (sep_right - sep_cursor_rect.left()).max(0.0);
        let (sep_rect, mut response) = ui.allocate_exact_size(vec2(sep_width, sep_row_height), ctx.sense());
        PghBreak::paint_groove_line(ui, sep_rect);
        let mut bounds = sep_rect;

        let mut preview_rect = ui.cursor();
        preview_rect.set_right(ctx.edit_right());
        preview_rect.set_height(2.0);
        let r_preview_top = ui.allocate_rect(preview_rect, ctx.sense());
        bounds = bounds.union(r_preview_top.rect);
        response |= r_preview_top;
        ui.horizontal(|ui| {
            // 这里不能复用 PghIndent::layout_paragraph（会写回 line_no 的 view 信息，污染最后一行 CodeRow 命中区域）
            ui.add_space(ctx.cfg().indent_size);
            let corner_radius = 6.0;
            let max_w = (ctx.edit_right() - ui.cursor().min.x).at_least(10.0);
            let image_response = ui
                .allocate_ui_with_layout(vec2(max_w, 0.0), Layout::top_down(Align::Center), |ui| {
                    ui.add(
                        Image::new(&image_url)
                            .fit_to_original_size(1.0)
                            .max_width(max_w)
                            .corner_radius(corner_radius),
                    )
                })
                .inner
                .on_hover_cursor(CursorIcon::Default);

            // 参考普通图片段落交互：允许左/右键点击。
            let id = ui.id().with(format!("plantuml_code_image_{}", line_no));
            let click_response = ui.interact(image_response.rect, id, Sense::click_and_drag());
            if click_response.clicked() || click_response.secondary_clicked() {
                if let Some((blk_s, blk_e)) = ctx.code_row_block_range(line_no) {
                    let start = (blk_s, 0, 0).into();
                    if let Some(end) = ctx.get_line(blk_e).map(|p| p.end_cursor_of_line(blk_e)) {
                        ctx.set_cursor1(start);
                        ctx.set_cursor2(end);
                    }
                }
            }

            // 代码块整段被选中时，为 PlantUML 预览图绘制选中描边。
            let mut is_selected = false;
            if ctx.is_selected() {
                if let Some((blk_s, blk_e)) = ctx.code_row_block_range(line_no) {
                    let cursor1 = ctx.cursor1();
                    let cursor2 = ctx.cursor2();
                    let sel_min = std::cmp::min(cursor1, cursor2);
                    let sel_max = std::cmp::max(cursor1, cursor2);
                    let block_start = (blk_s, 0, 0).into();
                    if let Some(block_end) = ctx.get_line(blk_e).map(|p| p.end_cursor_of_line(blk_e)) {
                        is_selected = sel_min <= block_start && block_end <= sel_max;
                    }
                }
            }
            if is_selected {
                let stroke = Stroke::new(2.0, CONTROL_HIGHLIGHT);
                ui.painter()
                    .rect_stroke(image_response.rect, corner_radius, stroke, StrokeKind::Outside);
            }

            bounds = bounds.union(image_response.rect);

            response |= image_response;
            response |= click_response;
        });
        let mut preview_rect_bottom = ui.cursor();
        preview_rect_bottom.set_right(ctx.edit_right());
        preview_rect_bottom.set_height(2.0);
        let r_preview_bottom = ui.allocate_rect(preview_rect_bottom, ctx.sense());
        bounds = bounds.union(r_preview_bottom.rect);
        response |= r_preview_bottom;
        Some((response, bounds))
    }

    pub fn code_format(slice: &LightSlice, _ui: &Ui, ctx: &Ctx) -> TextFormat {
        let color = if ctx.cfg().dark_mode{
            slice.dark_color
        } else {
            slice.light_color
        };
        let _brightness = ctx.cfg().text_color_brightness;
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

    fn is_plantuml_lang(lang: &str) -> bool {
        lang.eq_ignore_ascii_case("plantuml")
    }

    fn switch_code_lang_and_refresh_index(ctx: &mut Ctx, lang_line_no: usize, next_lang: &str) {
        let prev_lang = ctx
            .get_line(lang_line_no)
            .and_then(|p| p.code_lang.clone())
            .unwrap_or_default();
        if prev_lang.eq_ignore_ascii_case(next_lang) {
            return;
        }

        if let Some(update) = ctx.get_line_mut(lang_line_no) {
            update.code_lang = Some(next_lang.to_string());
            ctx.line_change_tick(lang_line_no);
        }
        ctx.on_content_change();

        // PlantUML 开关会影响代码块图片索引，切换时强制请求一次重建。
        if Self::is_plantuml_lang(&prev_lang) || Self::is_plantuml_lang(next_lang) {
            ctx.request_rebuild_index_if_needed(true);
        }
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
                ui.scope(|ui| {
                    // 仅去掉语言按钮底色，不影响其他控件样式。
                    ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                    ui.visuals_mut().widgets.hovered.weak_bg_fill = Color32::TRANSPARENT;
                    ui.visuals_mut().widgets.active.weak_bg_fill = Color32::TRANSPARENT;
                    ui.menu_button(cur_lang, |ui| {
                        let mut langs = support_lang();
                        if !langs.iter().any(|x| x.eq_ignore_ascii_case("plantuml")) {
                            langs.push("plantuml");
                        }
                        for lang in langs {
                            if ui.button(lang).clicked() {
                                Self::switch_code_lang_and_refresh_index(self.ctx, lang_line_no, lang);
                                ui.close();
                            }
                        }
                    })
                    .response
                })
                .inner
            })
            .inner;
        r
    }
}

impl PghView {
    fn paint_code_block_border(ui: &Ui, ctx: &Ctx, line_no: usize, rect: eframe::egui::Rect) {
        let Some(ci) = ctx.code_info_of_line(line_no) else {
            return;
        };
        let border_color = ui.visuals().weak_text_color().gamma_multiply(0.52);
        let stroke = Stroke::new(1.0, border_color);
        let painter = ui.painter();
        let left_top = Pos2::new(rect.left(), rect.top());
        let left_bottom = Pos2::new(rect.left(), rect.bottom());
        let right_top = Pos2::new(rect.right(), rect.top());
        let right_bottom = Pos2::new(rect.right(), rect.bottom());

        // 每行都画左右竖线，首行补顶部横线，末行补底部横线。
        //painter.line_segment([left_top, left_bottom], stroke);
        //painter.line_segment([right_top, right_bottom], stroke);
        if ci.code_row_index == 0 {
            painter.line_segment([left_top, right_top], stroke);
        }
        if ci.code_row_index + 1 == ci.code_total_rows {
            painter.line_segment([left_bottom, right_bottom], stroke);
        }
    }

    /// 代码块行左侧：按块内总行数对齐宽度的行号（右对齐）。
    fn layout_code_row_line_no_gutter(ui: &mut Ui, ctx: &Ctx, line_no: usize) -> Response {
        let Some(ci) = ctx.code_info_of_line(line_no) else {
            return ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1;
        };
        let total = ci.code_total_rows.max(1);
        let idx = ci.code_row_index;
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
        // 编辑导致块结构变化后由 `Ctx::refresh_code_row_block_metadata` 再同步。此处按 `CodeCache`
        // 再约束一层，避免中间行误带旧间距。
        let (spacing_top, spacing_bottom, num_segments) = {
            let Some(p) = ctx.get_line(line_no) else {
                return LayoutResponse::from_response(
                    ui.allocate_exact_size(vec2(0.0, 0.0), ctx.sense()).1,
                );
            };
            let (top, bottom) = match ctx.code_info_of_line(line_no) {
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
                    response |= PghIndent::layout_paragraph(ui, ctx, line_no, segment, ctx.cfg().indent_size);
                    response |= Self::layout_code_row_line_no_gutter(ui, ctx, line_no);
                    response |= PghIndent::layout_paragraph(ui, ctx, line_no, segment, 8.0);
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
                            let show_lang_menu = ctx
                                .code_info_of_line(line_no)
                                .map(|ci| ci.code_row_index == 0)
                                .unwrap_or(true);
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
            if let Some((image_response, plantuml_bounds)) =
                Self::layout_code_plantuml_image(ui, ctx, line_no)
            {
                response |= image_response;
                ctx.merge_line_pgh_rect_from_segments(line_no, plantuml_bounds);
            }

            response.on_hover_cursor(CursorIcon::Text)
        });
        Self::paint_code_block_border(ui, ctx, line_no, frame_response.response.rect);

        response = frame_response.inner;
        let mut bottom_rect = ui.cursor();
        bottom_rect.set_right(ctx.edit_right());
        bottom_rect.set_height(spacing_bottom);
        response |= ui.allocate_rect(bottom_rect, ctx.sense());

        if let Some(lang_menu_response) = lang_menu_response {
            lang_menu_response.on_hover_cursor(CursorIcon::Default);
        }

        LayoutResponse::new(response.clone(), response, false)
    }
}
