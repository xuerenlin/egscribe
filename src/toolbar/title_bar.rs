//! 无边框主窗口顶部的自定义标题栏（拖拽、双击最大化、最小化/最大化/关闭），
//! 以及四周原生缩放抓取区（`ViewportCommand::BeginResize`）。

use eframe::egui::{
    self,
    viewport::ResizeDirection,
    Align, Align2, Area, CornerRadius, CursorIcon, Id, Label, LayerId, Layout, Order,
    PointerButton, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions,
    Ui, UiBuilder, Vec2, ViewportCommand, Window,
};
use crate::i18n::tr;
use crate::store::Store;
use crate::uicom::{galley_builder, IconName};

use super::ToolBar;

const TITLE_BAR_HEIGHT: f32 = 38.0;
/// 右侧系统按钮区域宽度（最小化、最大化、关闭；含间距与内边距）。
const BUTTON_STRIP_W: f32 = 156.0;
/// 标题栏中间拖拽区域最小宽度。
const MIN_DRAG_REGION_W: f32 = 96.0;
/// 单个窗口控制按钮最小宽度（图标左右留白）。
const CAPTION_BTN_MIN_WIDTH: f32 = 36.0;
/// 关闭按钮悬停时图标颜色。
const CAPTION_CLOSE_HOVER_FG: egui::Color32 = egui::Color32::from_rgb(232, 17, 35);
/// 最小化 / 最大化 / 还原按钮悬停时图标颜色（与关闭区分）。
const CAPTION_CHROME_HOVER_FG: egui::Color32 = egui::Color32::from_rgb(0, 120, 212);
/// 非最大化时窗口轮廓圆角（逻辑像素），TODO: 暂时禁用圆角，圆角需要裁剪掉四角区域待实现
const RESTORED_WINDOW_CORNER_RADIUS: CornerRadius = CornerRadius::same(0);
/// 非最大化时窗口细边框线宽。
const RESTORED_WINDOW_BORDER_WIDTH: f32 = 1.0;
/// 标题栏左侧应用图标边长（逻辑像素）。
const TITLE_ICON_SIZE: f32 = 22.0;
/// 图标与左边缘、工具栏的间距。
const TITLE_ICON_MARGIN_L: f32 = 8.0;
const TITLE_ICON_GAP_TOOLBAR: f32 = 6.0;

fn about_visible_id() -> Id {
    Id::new("egscribe_about_visible")
}

fn set_about_visible(ctx: &egui::Context, visible: bool) {
    ctx.data_mut(|d| d.insert_temp(about_visible_id(), visible));
}

fn about_visible(ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<bool>(about_visible_id()).unwrap_or(false))
}

fn about_body_text() -> String {
    tr("about.body")
        .replace("{version}", env!("CARGO_PKG_VERSION"))
        .replace("{build_time}", env!("EGSCRIBE_BUILD_TIME"))
}

fn show_about_labels(ui: &mut Ui) {
    let visuals = ui.visuals();
    let text_color = visuals.text_color();
    let meta_color = visuals.weak_text_color();
    let accent_color = visuals.strong_text_color();

    ui.set_min_width(380.0);

    let body = about_body_text();
    let parts: Vec<&str> = body.split("\n\n").collect();

    if let Some(version) = parts.first() {
        ui.add(
            Label::new(
                RichText::new(*version)
                    .strong()
                    .size(15.0)
                    .color(accent_color),
            )
            .selectable(true),
        );
    }

    if let Some(desc) = parts.get(1) {
        ui.add_space(10.0);
        ui.add(
            Label::new(RichText::new(*desc).size(14.0).color(text_color)).selectable(true),
        );
    }

    if let Some(meta) = parts.get(2) {
        ui.add_space(10.0);
        for line in meta.lines() {
            if line.is_empty() {
                continue;
            }
            ui.add(
                Label::new(RichText::new(line).size(13.0).color(meta_color)).selectable(true),
            );
        }
    }
}

/// About 对话框：不透明窗口 + 只读 Label（可选中复制）；底部「确认」关闭。
pub fn show_about_dialog(ctx: &egui::Context) {
    if !about_visible(ctx) {
        return;
    }

    let mut open = true;
    Window::new(tr("about.title"))
        .open(&mut open)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .movable(true)
        .default_width(420.0)
        .order(Order::Foreground)
        .show(ctx, |ui| {
            show_about_labels(ui);
        });

    if !open {
        set_about_visible(ctx, false);
    }
}

/// `desktop/egscribe.png`，与 `main.rs` 里窗口图标同源。
static APP_ICON_PNG: &[u8] = include_bytes!("../../desktop/egscribe.png");

/// 标题栏背景：在主题 `panel_fill` 上做轻微明暗偏移，与侧栏/顶栏主色区分层次。
pub fn title_bar_fill(visuals: &egui::Visuals) -> egui::Color32 {
    let base = visuals.panel_fill;
    let d = if visuals.dark_mode {
        24i16
    } else {
        -20i16
    };
    egui::Color32::from_rgb(
        (base.r() as i16 + d).clamp(0, 255) as u8,
        (base.g() as i16 + d).clamp(0, 255) as u8,
        (base.b() as i16 + d).clamp(0, 255) as u8,
    )
}

fn png_to_color_image(bytes: &[u8]) -> Option<egui::ColorImage> {
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let w = img.width() as f32;
    let h = img.height() as f32;
    let size = [img.width() as usize, img.height() as usize];
    let pixels = img
        .into_raw()
        .chunks_exact(4)
        .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
        .collect();
    Some(egui::ColorImage {
        size,
        pixels,
        source_size: Vec2::new(w, h),
    })
}

fn title_bar_app_icon_texture(ctx: &egui::Context) -> TextureHandle {
    let id = Id::new("egscribe_main_title_bar_app_icon_tex");
    if let Some(tex) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return tex;
    }
    let color_image = png_to_color_image(APP_ICON_PNG).unwrap_or_else(|| {
        egui::ColorImage::new([1, 1], vec![egui::Color32::TRANSPARENT])
    });
    let tex = ctx.load_texture(
        "egscribe_main_title_icon",
        color_image,
        TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    tex
}

pub fn show_main_title_bar(ui: &mut Ui, title: &str, store: &mut Store) {
    let width = ui.available_width();
    let (title_bar_rect, _) =
        ui.allocate_exact_size(Vec2::new(width, TITLE_BAR_HEIGHT), Sense::hover());

    let _display_title = if title.is_empty() {
        tr("app.name")
    } else {
        title.to_string()
    };

    // 左侧图标+工具栏最多占用的宽度（保留右侧至少 MIN_DRAG_REGION_W 给拖拽条）。
    let left_budget = (width - BUTTON_STRIP_W - MIN_DRAG_REGION_W).max(0.0);
    let strip_rect = Rect::from_min_max(
        title_bar_rect.min,
        egui::pos2(
            (title_bar_rect.min.x + left_budget).min(title_bar_rect.max.x),
            title_bar_rect.max.y,
        ),
    );
    // 此前把整个 strip_rect 当作「左栏」，但 ToolBar 往往填不满；右缘空白仍在 strip 内，
    // 而 drag_rect 却从 strip 右缘才开始，导致 ToolBar 右侧空白无法拖拽/双击。此处用内容实际 min_rect 右缘作为拖拽区起点。
    let content_right_edge = ui
        .scope_builder(UiBuilder::new().max_rect(strip_rect), |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add_space(TITLE_ICON_MARGIN_L);
                let icon_tex = title_bar_app_icon_texture(ui.ctx());
                let icon_size = Vec2::splat(TITLE_ICON_SIZE);
                let (icon_rect, icon_resp) =
                    ui.allocate_exact_size(icon_size, Sense::click());
                ui.put(
                    icon_rect,
                    egui::Image::new(&icon_tex).fit_to_exact_size(icon_size),
                );
                if icon_resp.clicked() {
                    set_about_visible(ui.ctx(), true);
                }
                ui.add_space(TITLE_ICON_GAP_TOOLBAR);
                ui.add(ToolBar::new(store));
            });
            ui.min_rect().max.x
        })
        .inner;

    let drag_left = content_right_edge.clamp(
        title_bar_rect.min.x,
        strip_rect.max.x,
    );

    let drag_rect = Rect::from_min_max(
        egui::pos2(drag_left, title_bar_rect.min.y),
        egui::pos2(title_bar_rect.max.x - BUTTON_STRIP_W, title_bar_rect.max.y),
    );

    let title_resp = ui.interact(
        drag_rect,
        Id::new("egscribe_main_title_drag"),
        Sense::click_and_drag(),
    );

    // `Response::double_clicked()` 需要 Flags::CLICKED；此处补充「主键双击且指针在空白区」判定，行为与系统标题栏一致。
    let pointer_pos = ui
        .ctx()
        .pointer_interact_pos()
        .or_else(|| ui.ctx().pointer_latest_pos());
    let double_click_in_blank = ui.ctx().input(|i| {
        i.pointer
            .button_double_clicked(PointerButton::Primary)
    }) && pointer_pos.is_some_and(|p| drag_rect.contains(p));
    let toggle_maximize = title_resp.double_clicked() || double_click_in_blank;

    let painter = ui.painter();
    /* 
    painter.text(
        drag_rect.left_center() + Vec2::new(10.0, 0.0),
        Align2::LEFT_CENTER,
        display_title,
        egui::FontId::proportional(14.0),
        ui.visuals().text_color(),
    );
    */
    painter.line_segment(
        [title_bar_rect.left_bottom(), title_bar_rect.right_bottom()],
        ui.visuals().widgets.noninteractive.bg_stroke,
    );

    if toggle_maximize {
        let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::Maximized(!is_maximized));
    } else if title_resp.drag_started_by(PointerButton::Primary) {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
    }

    let btn_rect = Rect::from_min_max(
        egui::pos2(title_bar_rect.max.x - BUTTON_STRIP_W, title_bar_rect.min.y),
        title_bar_rect.max,
    );
    ui.scope_builder(UiBuilder::new().max_rect(btn_rect), |ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.visuals_mut().button_frame = false;

            ui.add_space(10.0);

            if caption_icon_button(
                ui,
                IconName::icon_window_close,
                tr("main_title_bar.close.tooltip"),
                Some(CAPTION_CLOSE_HOVER_FG),
            )
            .clicked()
            {
                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
            }

            let is_maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            if is_maximized {
                if caption_icon_button(
                    ui,
                    IconName::icon_window_restore,
                    tr("main_title_bar.restore.tooltip"),
                    Some(CAPTION_CHROME_HOVER_FG),
                )
                .clicked()
                {
                    ui.ctx()
                        .send_viewport_cmd(ViewportCommand::Maximized(false));
                }
            } else if caption_icon_button(
                ui,
                IconName::icon_window_maximize,
                tr("main_title_bar.maximize.tooltip"),
                Some(CAPTION_CHROME_HOVER_FG),
            )
            .clicked()
            {
                ui.ctx()
                    .send_viewport_cmd(ViewportCommand::Maximized(true));
            }

            if caption_icon_button(
                ui,
                IconName::icon_window_minimize,
                tr("main_title_bar.minimize.tooltip"),
                Some(CAPTION_CHROME_HOVER_FG),
            )
            .clicked()
            {
                ui.ctx().send_viewport_cmd(ViewportCommand::Minimized(true));
            }

            ui.add_space(8.0);
        });
    });
}

/// `hover_fg`：悬停时图标使用该色；`None` 则始终用主题正文色。
fn caption_icon_button(
    ui: &mut Ui,
    icon: IconName,
    _tooltip: String,
    hover_fg: Option<egui::Color32>,
) -> egui::Response {
    let size = Vec2::new(
        CAPTION_BTN_MIN_WIDTH,
        (TITLE_BAR_HEIGHT - 6.0).max(24.0),
    );
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let hovered = response.hovered();
    let fg = match (hover_fg, hovered) {
        (Some(c), true) => c,
        _ => ui.visuals().text_color(),
    };
    let galley = galley_builder(ui)
        .icon(icon)
        .bg(egui::Color32::TRANSPARENT)
        .fg(fg)
        .build();
    let galley_pos = rect.center() - 0.5 * galley.rect.size();
    ui.painter().galley(galley_pos, galley, fg);
    //response.on_hover_text(tooltip)
    response
}

/// 每帧主 UI 布局之后调用：绘制非最大化时的圆角细边框，并布置四边/四角原生缩放条。 
pub fn paint_window_border(ctx: &egui::Context) {
    paint_restored_window_border(ctx);
    show_native_resize_grips(ctx);
}

/// 在未最大化、非全屏时绘制圆角细边框（仅 egui 轮廓；Windows 上真实窗缘圆角由 `win_window_corners` + DWM 处理）。
fn paint_restored_window_border(ctx: &egui::Context) {
    if ctx.input(|i| {
        let v = i.viewport();
        v.maximized == Some(true)
            || v.fullscreen == Some(true)
            || v.minimized == Some(true)
    }) {
        return;
    }

    let rect = ctx.content_rect();
    let stroke = Stroke::new(
        RESTORED_WINDOW_BORDER_WIDTH,
        ctx.style().visuals.window_stroke().color,
    );
    let painter = ctx.layer_painter(LayerId::new(
        Order::Foreground,
        Id::new("egscribe_restored_window_border"),
    ));
    painter.rect_stroke(rect, RESTORED_WINDOW_CORNER_RADIUS, stroke, StrokeKind::Inside);
}

/// 在窗口四边与四角放置不可见的拖拽缩放条。
fn show_native_resize_grips(ctx: &egui::Context) {
    if ctx.input(|i| {
        let v = i.viewport();
        v.maximized == Some(true) || v.fullscreen == Some(true)
    }) {
        return;
    }

    let screen = ctx.content_rect();
    let g = ctx
        .style()
        .interaction
        .resize_grab_radius_side
        .max(8.0);

    let min = screen.min;
    let max = screen.max;

    let zones: [(&str, Rect, ResizeDirection, CursorIcon); 8] = [
        (
            "n",
            Rect::from_min_max(
                Pos2::new(min.x + g, min.y),
                Pos2::new(max.x - g, min.y + g),
            ),
            ResizeDirection::North,
            CursorIcon::ResizeNorth,
        ),
        (
            "s",
            Rect::from_min_max(
                Pos2::new(min.x + g, max.y - g),
                Pos2::new(max.x - g, max.y),
            ),
            ResizeDirection::South,
            CursorIcon::ResizeSouth,
        ),
        (
            "w",
            Rect::from_min_max(
                Pos2::new(min.x, min.y + g),
                Pos2::new(min.x + g, max.y - g),
            ),
            ResizeDirection::West,
            CursorIcon::ResizeWest,
        ),
        (
            "e",
            Rect::from_min_max(
                Pos2::new(max.x - g, min.y + g),
                Pos2::new(max.x, max.y - g),
            ),
            ResizeDirection::East,
            CursorIcon::ResizeEast,
        ),
        (
            "nw",
            Rect::from_min_max(min, Pos2::new(min.x + g, min.y + g)),
            ResizeDirection::NorthWest,
            CursorIcon::ResizeNorthWest,
        ),
        (
            "ne",
            Rect::from_min_max(
                Pos2::new(max.x - g, min.y),
                Pos2::new(max.x, min.y + g),
            ),
            ResizeDirection::NorthEast,
            CursorIcon::ResizeNorthEast,
        ),
        (
            "sw",
            Rect::from_min_max(
                Pos2::new(min.x, max.y - g),
                Pos2::new(min.x + g, max.y),
            ),
            ResizeDirection::SouthWest,
            CursorIcon::ResizeSouthWest,
        ),
        (
            "se",
            Rect::from_min_max(Pos2::new(max.x - g, max.y - g), max),
            ResizeDirection::SouthEast,
            CursorIcon::ResizeSouthEast,
        ),
    ];

    for (salt, rect, dir, cursor) in zones {
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            continue;
        }
        Area::new(Id::new("egscribe_native_resize").with(salt))
            .order(Order::Foreground)
            .fixed_pos(rect.min)
            .movable(false)
            .interactable(true)
            .show(ctx, |ui| {
                let (_, resp) = ui.allocate_exact_size(rect.size(), Sense::click_and_drag());
                if resp.hovered() {
                    ctx.set_cursor_icon(cursor);
                }
                if resp.drag_started_by(PointerButton::Primary) {
                    ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
                }
            });
    }
}
