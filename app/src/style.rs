//! Minecraft-ish widgets: bevels, stone buttons, pixel labels and sliders.

use eframe::egui::{
    self, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Pos2, Rect, Response,
    Sense, Stroke, Ui, Vec2,
};

pub const PANEL: Color32 = Color32::from_rgb(30, 33, 40);
pub const PANEL_DARK: Color32 = Color32::from_rgb(19, 21, 26);
pub const PANEL_DEEP: Color32 = Color32::from_rgb(13, 15, 19);
pub const SLOT: Color32 = Color32::from_rgb(11, 12, 15);
pub const OUTLINE: Color32 = Color32::from_rgb(6, 7, 9);
pub const BEVEL_LIGHT: Color32 = Color32::from_rgb(94, 100, 112);
pub const BEVEL_DARK: Color32 = Color32::from_rgb(8, 9, 12);
pub const GOLD: Color32 = Color32::from_rgb(255, 211, 92);
pub const GOLD_DARK: Color32 = Color32::from_rgb(152, 105, 30);
pub const GREEN: Color32 = Color32::from_rgb(112, 220, 120);
pub const RED: Color32 = Color32::from_rgb(235, 92, 82);
pub const AMBER: Color32 = Color32::from_rgb(245, 178, 64);
pub const TEXT: Color32 = Color32::from_rgb(240, 240, 232);
pub const MUTED: Color32 = Color32::from_rgb(148, 153, 162);
pub const MUTED_DARK: Color32 = Color32::from_rgb(96, 101, 110);
pub const BUTTON: Color32 = Color32::from_rgb(76, 80, 90);
pub const BUTTON_HOVER: Color32 = Color32::from_rgb(102, 108, 120);
pub const BUTTON_PRESSED: Color32 = Color32::from_rgb(55, 59, 67);
pub const BUTTON_DISABLED: Color32 = Color32::from_rgb(48, 51, 58);
pub const CYAN: Color32 = Color32::from_rgb(110, 230, 230);
pub const MAGENTA: Color32 = Color32::from_rgb(235, 120, 220);
pub const ORANGE: Color32 = Color32::from_rgb(250, 150, 70);
pub const PURPLE: Color32 = Color32::from_rgb(172, 140, 255);

pub const PIXEL_FONT: &[u8] = include_bytes!("../assets/PressStart2P-Regular.ttf");

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "press-start-2p".to_owned(),
        FontData::from_static(PIXEL_FONT),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        if let Some(list) = fonts.families.get_mut(&family) {
            list.insert(0, "press-start-2p".to_owned());
        }
    }
    ctx.set_fonts(fonts);
}

pub fn apply_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = Color32::TRANSPARENT;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = PANEL_DEEP;
    visuals.faint_bg_color = PANEL_DARK;
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.inactive.bg_fill = BUTTON;
    visuals.widgets.hovered.bg_fill = BUTTON_HOVER;
    visuals.widgets.active.bg_fill = BUTTON_PRESSED;
    visuals.selection.bg_fill = GOLD_DARK;
    visuals.selection.stroke = Stroke::new(1.0_f32, GOLD);
    ctx.set_visuals(visuals);
    ctx.style_mut(|style| {
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(8.0, 4.0);
        style.animation_time = 0.08;
    });
}

pub fn paint_bevel(painter: &egui::Painter, rect: Rect, thickness: f32) {
    let t = thickness.max(1.0);
    painter.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(rect.width(), t)),
        0.0,
        BEVEL_LIGHT,
    );
    painter.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(t, rect.height())),
        0.0,
        BEVEL_LIGHT,
    );
    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(rect.left(), rect.bottom() - t),
            Vec2::new(rect.width(), t),
        ),
        0.0,
        BEVEL_DARK,
    );
    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(rect.right() - t, rect.top()),
            Vec2::new(t, rect.height()),
        ),
        0.0,
        BEVEL_DARK,
    );
}

pub fn pixel_text(
    painter: &egui::Painter,
    pos: Pos2,
    align: Align2,
    text: &str,
    size: f32,
    color: Color32,
) {
    let font = FontId::proportional(size);
    painter.text(
        pos + Vec2::new(1.5, 1.5),
        align,
        text,
        font.clone(),
        Color32::from_black_alpha(210),
    );
    painter.text(pos, align, text, font, color);
}

pub fn pixel_label(ui: &mut Ui, text: &str, size: f32, color: Color32) -> Response {
    let font = FontId::proportional(size);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), color);
    let galley_size = galley.size();
    let (rect, response) =
        ui.allocate_exact_size(galley_size + Vec2::new(2.0, 2.0), Sense::hover());
    let painter = ui.painter();
    let shadow = painter.layout_no_wrap(text.to_owned(), font, Color32::from_black_alpha(200));
    painter.galley(rect.min + Vec2::new(1.5, 1.5), shadow, Color32::PLACEHOLDER);
    painter.galley(rect.min, galley, Color32::PLACEHOLDER);
    response
}

pub fn mc_button(ui: &mut Ui, label: &str, size: Vec2, enabled: bool) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();

    let fill = if !enabled {
        BUTTON_DISABLED
    } else if pressed {
        BUTTON_PRESSED
    } else if hovered {
        BUTTON_HOVER
    } else {
        BUTTON
    };

    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, fill);
    painter.rect_stroke(rect, 0.0, Stroke::new(2.0_f32, OUTLINE));

    if pressed {
        // inverted bevel
        painter.rect_filled(
            Rect::from_min_size(
                rect.min + Vec2::splat(2.0),
                Vec2::new(rect.width() - 4.0, 2.0),
            ),
            0.0,
            BEVEL_DARK,
        );
        painter.rect_filled(
            Rect::from_min_size(
                rect.min + Vec2::splat(2.0),
                Vec2::new(2.0, rect.height() - 4.0),
            ),
            0.0,
            BEVEL_DARK,
        );
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.left() + 2.0, rect.bottom() - 4.0),
                Vec2::new(rect.width() - 4.0, 2.0),
            ),
            0.0,
            BEVEL_LIGHT,
        );
        painter.rect_filled(
            Rect::from_min_size(
                Pos2::new(rect.right() - 4.0, rect.top() + 2.0),
                Vec2::new(2.0, rect.height() - 4.0),
            ),
            0.0,
            BEVEL_LIGHT,
        );
    } else {
        paint_bevel(painter, rect.shrink(2.0), 2.0);
    }

    if hovered {
        painter.rect_stroke(
            rect.shrink(1.0),
            0.0,
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 48)),
        );
    }

    let text_color = if enabled { TEXT } else { MUTED_DARK };
    let text_pos = rect.center()
        + if pressed {
            Vec2::new(1.0, 1.0)
        } else {
            Vec2::ZERO
        };
    pixel_text(
        painter,
        text_pos,
        Align2::CENTER_CENTER,
        label,
        if label.len() > 16 { 6.0 } else { 7.0 },
        text_color,
    );

    response
}

pub fn toggle_button(
    ui: &mut Ui,
    label: &str,
    size: Vec2,
    enabled: bool,
    active: bool,
) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(size, sense);
    let hovered = enabled && response.hovered();
    let pressed = enabled && response.is_pointer_button_down_on();

    let fill = if !enabled {
        BUTTON_DISABLED
    } else if active {
        if pressed {
            Color32::from_rgb(110, 76, 24)
        } else if hovered {
            Color32::from_rgb(176, 124, 36)
        } else {
            Color32::from_rgb(148, 102, 28)
        }
    } else if pressed {
        BUTTON_PRESSED
    } else if hovered {
        BUTTON_HOVER
    } else {
        BUTTON
    };

    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, fill);
    painter.rect_stroke(rect, 0.0, Stroke::new(2.0_f32, OUTLINE));
    if !pressed {
        paint_bevel(painter, rect.shrink(2.0), 2.0);
    }
    if active {
        painter.rect_stroke(rect.shrink(3.0), 0.0, Stroke::new(1.0_f32, GOLD));
    }
    pixel_text(
        painter,
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        6.0,
        if enabled { TEXT } else { MUTED_DARK },
    );
    response
}

pub fn status_pill(ui: &mut Ui, text: &str, color: Color32) {
    let font = FontId::proportional(6.0);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), font.clone(), color);
    let size = galley.size() + Vec2::new(18.0, 12.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, Color32::from_rgb(17, 19, 23));
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, color.gamma_multiply(0.6)));
    painter.rect_filled(
        Rect::from_min_size(rect.min + Vec2::splat(5.0), Vec2::splat(4.0)),
        0.0,
        color,
    );
    painter.text(
        rect.center() + Vec2::new(5.0, 0.0),
        Align2::CENTER_CENTER,
        text,
        font,
        color,
    );
}

pub fn slider(ui: &mut Ui, value: &mut f32, width: f32) -> (Response, bool) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 26.0), Sense::click_and_drag());
    let mut changed = false;
    if response.clicked() || response.dragged() {
        if let Some(pos) = ui.ctx().pointer_interact_pos() {
            let new = ((pos.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
            if (new - *value).abs() > 0.001 {
                *value = new;
                changed = true;
            }
        }
    }

    let painter = ui.painter();
    let track = Rect::from_center_size(rect.center(), Vec2::new(rect.width(), 12.0));
    painter.rect_filled(track, 0.0, PANEL_DEEP);
    painter.rect_stroke(track, 0.0, Stroke::new(1.0_f32, OUTLINE));
    let fill = Rect::from_min_max(
        track.min + Vec2::splat(2.0),
        Pos2::new(
            track.left() + 2.0 + (track.width() - 4.0) * value.clamp(0.0, 1.0),
            track.bottom() - 2.0,
        ),
    );
    painter.rect_filled(fill, 0.0, GOLD_DARK);
    painter.rect_filled(
        Rect::from_min_size(fill.min, Vec2::new(fill.width(), 2.0)),
        0.0,
        GOLD,
    );

    let knob_x = track.left() + 2.0 + (track.width() - 4.0) * value.clamp(0.0, 1.0);
    let knob = Rect::from_center_size(Pos2::new(knob_x, rect.center().y), Vec2::new(10.0, 22.0));
    painter.rect_filled(knob, 0.0, BUTTON_HOVER);
    painter.rect_stroke(knob, 0.0, Stroke::new(1.0_f32, OUTLINE));
    paint_bevel(painter, knob.shrink(1.0), 1.0);

    (response, changed)
}

pub fn progress_bar(ui: &mut Ui, ratio: f32, width: f32) -> (Response, Option<f32>) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 18.0), Sense::click_and_drag());
    let mut seek = None;
    if response.clicked() || response.dragged() {
        if let Some(pos) = ui.ctx().pointer_interact_pos() {
            seek = Some(((pos.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0));
        }
    }

    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, PANEL_DEEP);
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, OUTLINE));
    let inner = rect.shrink(3.0);
    let fill_w = inner.width() * ratio.clamp(0.0, 1.0);
    let fill = Rect::from_min_size(inner.min, Vec2::new(fill_w, inner.height()));
    painter.rect_filled(fill, 0.0, Color32::from_rgb(86, 150, 70));
    painter.rect_filled(
        Rect::from_min_size(fill.min, Vec2::new(fill.width(), 2.0)),
        0.0,
        Color32::from_rgb(150, 225, 120),
    );
    // chunky segment lines, like the game's hunger/xp bars
    let mut x = inner.left();
    while x < inner.right() {
        painter.rect_filled(
            Rect::from_min_size(Pos2::new(x, inner.top()), Vec2::new(1.0, inner.height())),
            0.0,
            Color32::from_black_alpha(38),
        );
        x += 12.0;
    }
    painter.rect_stroke(
        rect.shrink(2.0),
        0.0,
        Stroke::new(1.0_f32, Color32::from_rgb(55, 60, 48)),
    );
    (response, seek)
}

pub fn draw_note_icon(painter: &egui::Painter, center: Pos2, size: f32, color: Color32) {
    let s = size / 16.0;
    let x = center.x - 5.0 * s;
    let y = center.y - 8.0 * s;
    painter.rect_filled(
        Rect::from_min_size(Pos2::new(x, y + 8.0 * s), Vec2::new(7.0 * s, 5.0 * s)),
        0.0,
        color,
    );
    painter.rect_filled(
        Rect::from_min_size(Pos2::new(x + 5.0 * s, y), Vec2::new(2.0 * s, 11.0 * s)),
        0.0,
        color,
    );
    painter.rect_filled(
        Rect::from_min_size(Pos2::new(x + 7.0 * s, y), Vec2::new(6.0 * s, 2.0 * s)),
        0.0,
        color,
    );
    painter.rect_filled(
        Rect::from_min_size(
            Pos2::new(x + 7.0 * s, y + 3.0 * s),
            Vec2::new(4.0 * s, 2.0 * s),
        ),
        0.0,
        color,
    );
}

pub fn draw_music_note(painter: &egui::Painter, center: Pos2, size: f32, color: Color32, kind: u8) {
    match kind % 4 {
        0 => {
            // eighth note
            let s = size / 12.0;
            let head = Rect::from_center_size(
                center + Vec2::new(-2.0 * s, 3.0 * s),
                Vec2::new(6.0 * s, 4.5 * s),
            );
            painter.rect_filled(head, 0.0, color);
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(center.x + 1.0 * s, center.y - 5.0 * s),
                    Vec2::new(2.0 * s, 9.0 * s),
                ),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(center.x + 2.0 * s, center.y - 5.0 * s),
                    Vec2::new(4.0 * s, 2.0 * s),
                ),
                0.0,
                color,
            );
        }
        1 => {
            // beamed pair
            let s = size / 14.0;
            painter.rect_filled(
                Rect::from_center_size(
                    center + Vec2::new(-3.0 * s, 4.0 * s),
                    Vec2::new(5.0 * s, 4.0 * s),
                ),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_center_size(
                    center + Vec2::new(5.0 * s, 2.0 * s),
                    Vec2::new(5.0 * s, 4.0 * s),
                ),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(center.x - 1.0 * s, center.y - 6.0 * s),
                    Vec2::new(2.0 * s, 10.0 * s),
                ),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(center.x + 7.0 * s, center.y - 8.0 * s),
                    Vec2::new(2.0 * s, 10.0 * s),
                ),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(center.x - 1.0 * s, center.y - 7.0 * s),
                    Vec2::new(10.0 * s, 3.0 * s),
                ),
                0.0,
                color,
            );
        }
        2 => {
            // sparkle
            let s = size / 12.0;
            painter.rect_filled(
                Rect::from_center_size(center, Vec2::new(2.0 * s, 10.0 * s)),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_center_size(center, Vec2::new(10.0 * s, 2.0 * s)),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_center_size(center, Vec2::new(4.0 * s, 4.0 * s)),
                0.0,
                color,
            );
        }
        _ => {
            // quarter note
            let s = size / 12.0;
            painter.rect_filled(
                Rect::from_center_size(
                    center + Vec2::new(-2.0 * s, 3.0 * s),
                    Vec2::new(7.0 * s, 5.0 * s),
                ),
                0.0,
                color,
            );
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(center.x + 2.0 * s, center.y - 6.0 * s),
                    Vec2::new(2.0 * s, 10.0 * s),
                ),
                0.0,
                color,
            );
        }
    }
}
