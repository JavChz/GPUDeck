use std::collections::VecDeque;
use std::f32::consts::PI;

use egui::epaint::{Mesh, Vertex, WHITE_UV};
use egui::{
    Align2, Color32, CornerRadius, Frame, Margin, Painter, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, Ui, Vec2,
};

use super::theme::{
    display_temp, format_temp, lerp_color, temp_unit, with_alpha, BackdropStyle, BarStyle,
    GaugeStyle, HeaderStyle, ThemeSpec,
};
use crate::telemetry::guidelines::{evaluate_temp_level, HealthBadge, MetricLevel, TempRange};

pub fn paint_ambient_backdrop(painter: &Painter, rect: Rect, theme: &ThemeSpec) {
    match theme.backdrop_style {
        BackdropStyle::NeoCyber => {
            let left_glow = Pos2::new(rect.left() + rect.width() * 0.14, rect.top() + 90.0);
            let right_glow = Pos2::new(rect.right() - rect.width() * 0.14, rect.top() + 90.0);
            let bottom_glow = Pos2::new(rect.center().x, rect.bottom() - 40.0);

            for (center, color, base_r) in [
                (left_glow, theme.accent_primary, 260.0),
                (right_glow, theme.accent_tertiary, 240.0),
                (bottom_glow, theme.accent_secondary, 300.0),
            ] {
                for step in 0..4 {
                    let r = base_r * (1.0 - step as f32 * 0.2);
                    let alpha = 4 + step as u8 * 2;
                    painter.circle_filled(center, r, with_alpha(color, alpha));
                }
            }

            let trace_color = with_alpha(theme.accent_primary, 18);
            let r_trace = with_alpha(theme.accent_tertiary, 16);
            if rect.width() > 900.0 {
                let y_mid = rect.center().y;
                painter.line_segment(
                    [
                        Pos2::new(rect.left(), y_mid - 140.0),
                        Pos2::new(rect.left() + 18.0, y_mid - 140.0),
                    ],
                    Stroke::new(1.5_f32, trace_color),
                );
                painter.line_segment(
                    [
                        Pos2::new(rect.left() + 18.0, y_mid - 140.0),
                        Pos2::new(rect.left() + 18.0, y_mid - 60.0),
                    ],
                    Stroke::new(1.5_f32, trace_color),
                );
                painter.line_segment(
                    [
                        Pos2::new(rect.right(), y_mid - 110.0),
                        Pos2::new(rect.right() - 18.0, y_mid - 110.0),
                    ],
                    Stroke::new(1.5_f32, r_trace),
                );
                painter.line_segment(
                    [
                        Pos2::new(rect.right() - 18.0, y_mid - 110.0),
                        Pos2::new(rect.right() - 18.0, y_mid - 30.0),
                    ],
                    Stroke::new(1.5_f32, r_trace),
                );
            }
        }

        BackdropStyle::AeroSky => {
            paint_vertical_gradient(painter, rect, theme.bg_app, theme.bg_app_bottom);

            // Soft modern Vista Aurora light veils
            paint_aurora_ribbon(
                painter,
                rect,
                0.78,
                0.18,
                0.10,
                0.4,
                190.0,
                Color32::from_rgb(56, 224, 168),
                Color32::from_rgb(56, 212, 255),
                80,
            );
            paint_aurora_ribbon(
                painter,
                rect,
                0.22,
                0.58,
                0.08,
                2.1,
                160.0,
                Color32::from_rgb(190, 244, 255),
                Color32::from_rgb(64, 196, 255),
                62,
            );

            // Delicate outer-edge glass bokeh bubbles
            let bubbles = [
                (0.05, 0.22, 34.0),
                (0.11, 0.86, 26.0),
                (0.04, 0.62, 18.0),
                (0.90, 0.19, 38.0),
                (0.95, 0.78, 28.0),
                (0.94, 0.46, 20.0),
            ];
            for (fx, fy, r) in bubbles {
                let c = Pos2::new(
                    rect.left() + rect.width() * fx,
                    rect.top() + rect.height() * fy,
                );
                painter.circle_filled(c, r, with_alpha(Color32::WHITE, 20));
                painter.circle_stroke(c, r, Stroke::new(1.0_f32, with_alpha(Color32::WHITE, 70)));
                painter.circle_filled(
                    Pos2::new(c.x - r * 0.32, c.y - r * 0.34),
                    r * 0.20,
                    with_alpha(Color32::WHITE, 70),
                );
            }
        }

        BackdropStyle::KawaiiPastel => {
            paint_vertical_gradient(painter, rect, theme.bg_app, theme.bg_app_bottom);
            let accents = [
                (0.06, 0.18, 12.0, theme.accent_primary, true),
                (0.14, 0.84, 16.0, theme.accent_secondary, false),
                (0.24, 0.12, 9.0, theme.accent_tertiary, true),
                (0.76, 0.15, 11.0, theme.accent_primary, true),
                (0.89, 0.24, 18.0, theme.accent_secondary, false),
                (0.93, 0.82, 13.0, theme.accent_tertiary, true),
                (0.50, 0.92, 10.0, theme.accent_primary, true),
            ];
            for (fx, fy, size, col, is_star) in accents {
                let c = Pos2::new(
                    rect.left() + rect.width() * fx,
                    rect.top() + rect.height() * fy,
                );
                if is_star {
                    paint_sparkle(painter, c, size, with_alpha(col, 95));
                } else {
                    painter.circle_filled(c, size, with_alpha(col, 42));
                    painter.circle_filled(
                        Pos2::new(c.x - size * 0.28, c.y - size * 0.28),
                        size * 0.28,
                        with_alpha(Color32::WHITE, 130),
                    );
                }
            }
        }

        BackdropStyle::JapandiPaper => {
            painter.line_segment(
                [
                    Pos2::new(rect.left() + 24.0, rect.bottom() - 8.0),
                    Pos2::new(rect.right() - 24.0, rect.bottom() - 8.0),
                ],
                Stroke::new(1.0_f32, with_alpha(theme.border_card, 140)),
            );
        }

        BackdropStyle::ModernAmbient => {
            let center = Pos2::new(rect.center().x, rect.top() + 45.0);
            for step in 0..4 {
                let r = 320.0 - step as f32 * 55.0;
                painter.circle_filled(
                    center,
                    r,
                    with_alpha(theme.accent_primary, 4 + step as u8 * 2),
                );
            }
        }

        BackdropStyle::TerminalCrt => {
            let line_col = with_alpha(theme.accent_primary, 9);
            let mut y = rect.top();
            while y < rect.bottom() {
                painter.line_segment(
                    [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                    Stroke::new(1.0_f32, line_col),
                );
                y += 3.0;
            }
        }

        BackdropStyle::CleanFlat => {}
    }
}

fn paint_vertical_gradient(painter: &Painter, rect: Rect, top_col: Color32, bot_col: Color32) {
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: rect.left_top(),
        uv: WHITE_UV,
        color: top_col,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_top(),
        uv: WHITE_UV,
        color: top_col,
    });
    mesh.vertices.push(Vertex {
        pos: rect.left_bottom(),
        uv: WHITE_UV,
        color: bot_col,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_bottom(),
        uv: WHITE_UV,
        color: bot_col,
    });
    mesh.indices.extend_from_slice(&[0, 1, 2, 1, 3, 2]);
    painter.add(Shape::mesh(mesh));
}

fn paint_sparkle(painter: &Painter, center: Pos2, r: f32, color: Color32) {
    let inner = r * 0.26;
    let pts = vec![
        Pos2::new(center.x, center.y - r),
        Pos2::new(center.x + inner, center.y - inner),
        Pos2::new(center.x + r, center.y),
        Pos2::new(center.x + inner, center.y + inner),
        Pos2::new(center.x, center.y + r),
        Pos2::new(center.x - inner, center.y + inner),
        Pos2::new(center.x - r, center.y),
        Pos2::new(center.x - inner, center.y - inner),
    ];
    painter.add(Shape::convex_polygon(
        vec![pts[0], pts[2], pts[4], pts[6]],
        color,
        Stroke::NONE,
    ));
    painter.add(Shape::convex_polygon(
        vec![pts[1], pts[3], pts[5], pts[7]],
        color,
        Stroke::NONE,
    ));
}

pub struct HeaderActionResult {
    pub reset_all_records: bool,
    pub toggle_theme_menu: bool,
    pub theme_btn_rect: Rect,
    pub toggle_unit: bool,
    pub toggle_settings: bool,
    pub toggle_info: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn draw_header_bar(
    ui: &mut Ui,
    theme: &ThemeSpec,
    gpu_count: usize,
    driver_version: Option<&str>,
    use_fahrenheit: bool,
    theme_menu_open: bool,
    settings_open: bool,
    info_open: bool,
    update_available: bool,
) -> HeaderActionResult {
    let avail_w = ui.available_width();
    let header_h = 52.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(avail_w, header_h), Sense::hover());
    let painter = ui.painter();

    let mut result = HeaderActionResult {
        reset_all_records: false,
        toggle_theme_menu: false,
        theme_btn_rect: Rect::NOTHING,
        toggle_unit: false,
        toggle_settings: false,
        toggle_info: false,
    };

    let header_cr = CornerRadius::same(theme.card_radius.min(12));
    if theme.gloss_sheen {
        painter.rect_filled(
            rect.translate(Vec2::new(0.0, 2.0)),
            header_cr,
            Color32::from_rgba_unmultiplied(4, 24, 52, 34),
        );
        let glass = build_aero_glass_shapes(
            rect,
            header_cr,
            Color32::from_rgba_unmultiplied(255, 255, 255, 238),
            Color32::from_rgba_unmultiplied(245, 251, 255, 232),
            Color32::from_rgba_unmultiplied(240, 249, 255, 228),
            Color32::from_rgba_unmultiplied(238, 248, 255, 226),
            theme.border_card,
            false,
        );
        for shape in glass {
            painter.add(shape);
        }
    } else {
        painter.rect_filled(rect, header_cr, theme.bg_header);
        painter.rect_stroke(
            rect,
            header_cr,
            Stroke::new(1.0_f32, theme.border_card),
            StrokeKind::Inside,
        );
    }

    // Center title treatment per HeaderStyle
    match theme.header_style {
        HeaderStyle::CyberTrapezoid => {
            let trap_w = (avail_w * 0.22).clamp(176.0, 240.0);
            let top_inset = 18.0;
            let trap_rect = Rect::from_center_size(
                Pos2::new(rect.center().x, rect.center().y - 2.0),
                Vec2::new(trap_w, 36.0),
            );
            let pts = vec![
                Pos2::new(trap_rect.left(), trap_rect.top()),
                Pos2::new(trap_rect.right(), trap_rect.top()),
                Pos2::new(trap_rect.right() - top_inset, trap_rect.bottom()),
                Pos2::new(trap_rect.left() + top_inset, trap_rect.bottom()),
            ];
            painter.add(Shape::convex_polygon(
                pts,
                Color32::from_rgb(13, 21, 33),
                Stroke::new(1.0_f32, with_alpha(theme.accent_primary, 85)),
            ));

            let bl = Pos2::new(trap_rect.left() + top_inset, trap_rect.bottom());
            let bm = Pos2::new(trap_rect.center().x, trap_rect.bottom());
            let br = Pos2::new(trap_rect.right() - top_inset, trap_rect.bottom());
            paint_gradient_line(
                painter,
                bl,
                bm,
                3.0,
                theme.accent_primary,
                theme.accent_secondary,
            );
            paint_gradient_line(
                painter,
                bm,
                br,
                3.0,
                theme.accent_secondary,
                theme.accent_tertiary,
            );
            paint_gradient_line(
                painter,
                bl,
                bm,
                7.0,
                with_alpha(theme.accent_primary, 38),
                with_alpha(theme.accent_secondary, 38),
            );
            paint_gradient_line(
                painter,
                bm,
                br,
                7.0,
                with_alpha(theme.accent_secondary, 38),
                with_alpha(theme.accent_tertiary, 38),
            );
            paint_brand_title(painter, trap_rect.center(), theme);
        }

        HeaderStyle::AeroGlassBar => {
            let pill = Rect::from_center_size(rect.center(), Vec2::new(168.0, 33.0));
            painter.rect_filled(
                pill.translate(Vec2::new(0.0, 1.5)),
                CornerRadius::same(16),
                Color32::from_rgba_unmultiplied(6, 30, 58, 42),
            );
            paint_aero_gel_pill(
                painter,
                pill,
                16,
                Color32::from_rgb(12, 123, 181),
                Color32::from_rgb(10, 98, 173),
                Color32::from_rgb(9, 79, 140),
            );
            paint_aero_brand_title(painter, pill.center(), theme);
        }

        HeaderStyle::KawaiiRibbon => {
            let pill = Rect::from_center_size(rect.center(), Vec2::new(172.0, 34.0));
            painter.rect_filled(
                pill,
                CornerRadius::same(17),
                with_alpha(theme.accent_primary, 22),
            );
            painter.rect_stroke(
                pill,
                CornerRadius::same(17),
                Stroke::new(1.2_f32, with_alpha(theme.accent_primary, 130)),
                StrokeKind::Inside,
            );
            paint_sparkle(
                painter,
                Pos2::new(pill.left() + 14.0, pill.center().y),
                6.0,
                theme.accent_primary,
            );
            paint_sparkle(
                painter,
                Pos2::new(pill.right() - 14.0, pill.center().y),
                6.0,
                theme.accent_secondary,
            );
            paint_brand_title(painter, pill.center(), theme);
        }

        HeaderStyle::TerminalBar => {
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "[ GPUDECK :: TELEMETRY ]",
                theme.font_mono_bold(14.5),
                theme.header_text_primary,
            );
        }

        HeaderStyle::ExecutiveBar => {
            paint_gradient_line(
                painter,
                Pos2::new(rect.left() + 2.0, rect.bottom() - 1.5),
                Pos2::new(rect.right() - 2.0, rect.bottom() - 1.5),
                2.5,
                theme.accent_tertiary,
                theme.accent_secondary,
            );
            paint_brand_title(painter, rect.center(), theme);
        }

        HeaderStyle::CleanModern => {
            paint_gradient_line(
                painter,
                Pos2::new(rect.center().x - 68.0, rect.bottom() - 1.5),
                Pos2::new(rect.center().x + 68.0, rect.bottom() - 1.5),
                2.0,
                theme.accent_primary,
                theme.accent_secondary,
            );
            paint_brand_title(painter, rect.center(), theme);
        }
    }

    // Left status summary
    let cy = rect.center().y;
    let gpu_label = if gpu_count == 1 {
        "1 GPU".to_string()
    } else {
        format!("{gpu_count} GPUs")
    };
    let left_status = if avail_w > 780.0 {
        match driver_version {
            Some(drv) => Some(format!("{gpu_label}  ·  Driver {drv}")),
            None => Some(gpu_label),
        }
    } else if avail_w > 560.0 {
        Some(gpu_label)
    } else {
        None
    };
    if let Some(status) = left_status {
        painter.text(
            Pos2::new(rect.left() + 16.0, cy),
            Align2::LEFT_CENTER,
            status,
            theme.font_ui(12.5),
            theme.header_text_secondary,
        );
    }

    // Right interactive controls
    let btn_h = 30.0;
    let mut cursor_x = rect.right() - 12.0;

    // Info (i) button
    let info_rect = Rect::from_center_size(Pos2::new(cursor_x - 15.0, cy), Vec2::new(30.0, btn_h));
    cursor_x -= 37.0;
    let info_resp = paint_icon_button(ui, theme, info_rect, info_open, |p, c, col| {
        p.circle_stroke(c, 9.5, Stroke::new(1.5_f32, col));
        p.circle_filled(Pos2::new(c.x, c.y - 4.2), 1.3, col);
        p.line_segment(
            [Pos2::new(c.x, c.y - 1.2), Pos2::new(c.x, c.y + 4.8)],
            Stroke::new(1.8_f32, col),
        );
    });
    if update_available {
        let dot_c = Pos2::new(info_rect.right() - 4.5, info_rect.top() + 4.5);
        ui.painter().circle_filled(dot_c, 3.5, theme.accent_primary);
        ui.painter()
            .circle_stroke(dot_c, 3.5, Stroke::new(1.0_f32, theme.bg_header));
    }
    let info_resp = info_resp.on_hover_text(if update_available {
        "About, Updates & Guidelines (Update available!)"
    } else {
        "About, Updates & Guidelines"
    });
    if info_resp.clicked() {
        result.toggle_info = true;
    }

    // Settings gear button
    let gear_rect = Rect::from_center_size(Pos2::new(cursor_x - 15.0, cy), Vec2::new(30.0, btn_h));
    cursor_x -= 37.0;
    let gear_resp = paint_icon_button(ui, theme, gear_rect, settings_open, paint_gear_icon)
        .on_hover_text("Settings & Theme Gallery");
    if gear_resp.clicked() {
        result.toggle_settings = true;
    }

    // Unit toggle (°C / °F)
    let unit_rect = Rect::from_center_size(Pos2::new(cursor_x - 18.0, cy), Vec2::new(36.0, btn_h));
    cursor_x -= 43.0;
    let unit_resp = paint_text_button(ui, theme, unit_rect, temp_unit(use_fahrenheit), false)
        .on_hover_text("Switch between °C and °F");
    if unit_resp.clicked() {
        result.toggle_unit = true;
    }

    // Theme dropdown selector pill
    if avail_w > 540.0 {
        let name = theme.id.display_name();
        let name_galley = ui.painter().layout_no_wrap(
            name.to_string(),
            theme.font_ui(12.0),
            theme.header_text_primary,
        );
        let btn_w = (name_galley.size().x + 38.0).clamp(92.0, 146.0);
        let theme_rect = Rect::from_center_size(
            Pos2::new(cursor_x - btn_w * 0.5, cy),
            Vec2::new(btn_w, btn_h),
        );
        cursor_x -= btn_w + 7.0;
        result.theme_btn_rect = theme_rect;
        let theme_resp = paint_theme_pill_button(ui, theme, theme_rect, theme_menu_open)
            .on_hover_text("Switch UI Theme");
        if theme_resp.clicked() {
            result.toggle_theme_menu = true;
        }
    }

    // Global Reset Max Records button
    if avail_w >= 960.0 {
        let reset_w = 88.0;
        let reset_rect = Rect::from_center_size(
            Pos2::new(cursor_x - reset_w * 0.5, cy),
            Vec2::new(reset_w, btn_h),
        );
        let reset_resp =
            paint_icon_label_button(ui, theme, reset_rect, "Reset Max", |p, c, col| {
                paint_reset_icon(p, c, 5.2, col);
            })
            .on_hover_text("Reset recorded max temperatures for all GPUs");
        if reset_resp.clicked() {
            result.reset_all_records = true;
        }
    } else if avail_w >= 700.0 {
        let reset_rect =
            Rect::from_center_size(Pos2::new(cursor_x - 15.0, cy), Vec2::new(30.0, btn_h));
        let reset_resp = paint_icon_button(ui, theme, reset_rect, false, |p, c, col| {
            paint_reset_icon(p, c, 6.0, col);
        })
        .on_hover_text("Reset recorded max temperatures for all GPUs");
        if reset_resp.clicked() {
            result.reset_all_records = true;
        }
    }

    result
}

fn paint_brand_title(painter: &Painter, center: Pos2, theme: &ThemeSpec) {
    let brand_galley = painter.layout_no_wrap(
        "GPUDECK".to_string(),
        theme.font_title(17.5),
        theme.header_text_primary,
    );
    let pos = Pos2::new(
        center.x - brand_galley.size().x * 0.5,
        center.y - brand_galley.size().y * 0.5,
    );
    painter.galley(pos, brand_galley, theme.header_text_primary);
}

fn paint_aero_brand_title(painter: &Painter, center: Pos2, theme: &ThemeSpec) {
    let brand_galley = painter.layout_no_wrap(
        "GPUDECK".to_string(),
        theme.font_title(17.5),
        Color32::WHITE,
    );
    let shadow_col = Color32::from_rgba_unmultiplied(4, 24, 52, 130);
    let pos = Pos2::new(
        center.x - brand_galley.size().x * 0.5,
        center.y - brand_galley.size().y * 0.5,
    );
    painter.galley(
        Pos2::new(pos.x, pos.y + 1.0),
        painter.layout_no_wrap("GPUDECK".to_string(), theme.font_title(17.5), shadow_col),
        shadow_col,
    );
    painter.galley(pos, brand_galley, Color32::WHITE);
}

fn aero_btn_palette(active_state: bool, theme: &ThemeSpec) -> (Color32, Color32, Color32) {
    if active_state {
        (
            Color32::from_rgb(11, 122, 209),
            Color32::from_rgb(8, 98, 173),
            Color32::from_rgb(7, 80, 143),
        )
    } else {
        (
            Color32::from_rgb(255, 255, 255),
            Color32::from_rgb(226, 242, 253),
            theme.header_btn_border,
        )
    }
}

fn paint_theme_pill_button(ui: &mut Ui, theme: &ThemeSpec, rect: Rect, open: bool) -> Response {
    let resp = ui.allocate_rect(rect, Sense::click());
    let hovered = resp.hovered();
    let painter = ui.painter();
    let rad = theme.badge_radius.min(10);
    let cr = CornerRadius::same(rad);

    let (text_col, chev_col) = if theme.gloss_sheen {
        let active_state = open || hovered;
        let (top_c, bot_c, border_c) = aero_btn_palette(active_state, theme);
        paint_aero_gel_pill(painter, rect, rad, top_c, bot_c, border_c);
        if active_state {
            (Color32::WHITE, Color32::from_rgb(228, 244, 255))
        } else {
            (theme.header_text_primary, theme.header_text_secondary)
        }
    } else {
        let bg = if open {
            with_alpha(theme.accent_primary, 52)
        } else if hovered {
            lerp_color(theme.header_btn_bg, theme.accent_primary, 0.14)
        } else {
            theme.header_btn_bg
        };
        let border = if open || hovered {
            theme.accent_primary
        } else {
            theme.header_btn_border
        };

        painter.rect_filled(rect, cr, bg);
        painter.rect_stroke(rect, cr, Stroke::new(1.0_f32, border), StrokeKind::Inside);
        (theme.header_text_primary, theme.header_text_secondary)
    };

    // Accent swatch dot
    let dot_pos = Pos2::new(rect.left() + 12.0, rect.center().y);
    if theme.gloss_sheen {
        paint_aero_orb(painter, dot_pos, 4.6, theme.accent_primary);
    } else {
        painter.circle_filled(dot_pos, 4.5, theme.accent_primary);
        painter.circle_stroke(
            dot_pos,
            4.5,
            Stroke::new(0.8_f32, with_alpha(theme.header_text_primary, 130)),
        );
    }

    // Theme name
    painter.text(
        Pos2::new(rect.left() + 22.0, rect.center().y),
        Align2::LEFT_CENTER,
        theme.id.display_name(),
        theme.font_ui(12.0),
        text_col,
    );

    // Chevron down triangle
    let cx = rect.right() - 11.0;
    let cy = rect.center().y + if open { -1.0 } else { 1.0 };
    let chev = if open {
        vec![
            Pos2::new(cx - 3.5, cy + 2.0),
            Pos2::new(cx + 3.5, cy + 2.0),
            Pos2::new(cx, cy - 2.5),
        ]
    } else {
        vec![
            Pos2::new(cx - 3.5, cy - 2.0),
            Pos2::new(cx + 3.5, cy - 2.0),
            Pos2::new(cx, cy + 2.5),
        ]
    };
    painter.add(Shape::convex_polygon(chev, chev_col, Stroke::NONE));

    resp
}

fn paint_icon_button<F>(
    ui: &mut Ui,
    theme: &ThemeSpec,
    rect: Rect,
    active: bool,
    draw_icon: F,
) -> Response
where
    F: FnOnce(&Painter, Pos2, Color32),
{
    let resp = ui.allocate_rect(rect, Sense::click());
    let hovered = resp.hovered();
    let painter = ui.painter();
    let rad = theme.badge_radius.min(10);
    let cr = CornerRadius::same(rad);

    let col = if theme.gloss_sheen {
        let active_state = active || hovered;
        let (top_c, bot_c, border_c) = aero_btn_palette(active_state, theme);
        paint_aero_gel_pill(painter, rect, rad, top_c, bot_c, border_c);
        if active_state {
            Color32::WHITE
        } else {
            theme.header_text_secondary
        }
    } else {
        let bg = if active {
            with_alpha(theme.accent_primary, 52)
        } else if hovered {
            lerp_color(theme.header_btn_bg, theme.accent_primary, 0.14)
        } else {
            theme.header_btn_bg
        };
        let border = if active || hovered {
            theme.accent_primary
        } else {
            theme.header_btn_border
        };
        painter.rect_filled(rect, cr, bg);
        painter.rect_stroke(rect, cr, Stroke::new(1.0_f32, border), StrokeKind::Inside);
        if active || hovered {
            theme.accent_primary
        } else {
            theme.header_text_secondary
        }
    };
    draw_icon(painter, rect.center(), col);
    resp
}

fn paint_text_button(
    ui: &mut Ui,
    theme: &ThemeSpec,
    rect: Rect,
    text: &str,
    active: bool,
) -> Response {
    let resp = ui.allocate_rect(rect, Sense::click());
    let hovered = resp.hovered();
    let painter = ui.painter();
    let rad = theme.badge_radius.min(10);
    let cr = CornerRadius::same(rad);

    let text_col = if theme.gloss_sheen {
        let active_state = active || hovered;
        let (top_c, bot_c, border_c) = aero_btn_palette(active_state, theme);
        paint_aero_gel_pill(painter, rect, rad, top_c, bot_c, border_c);
        if active_state {
            Color32::WHITE
        } else {
            theme.accent_primary
        }
    } else {
        let bg = if active {
            with_alpha(theme.accent_primary, 52)
        } else if hovered {
            lerp_color(theme.header_btn_bg, theme.accent_primary, 0.14)
        } else {
            theme.header_btn_bg
        };
        let border = if active || hovered {
            theme.accent_primary
        } else {
            theme.header_btn_border
        };
        painter.rect_filled(rect, cr, bg);
        painter.rect_stroke(rect, cr, Stroke::new(1.0_f32, border), StrokeKind::Inside);
        if active || hovered {
            theme.header_text_primary
        } else {
            theme.accent_primary
        }
    };

    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        theme.font_value(12.0),
        text_col,
    );
    resp
}

fn paint_icon_label_button<F>(
    ui: &mut Ui,
    theme: &ThemeSpec,
    rect: Rect,
    text: &str,
    draw_icon: F,
) -> Response
where
    F: FnOnce(&Painter, Pos2, Color32),
{
    let resp = ui.allocate_rect(rect, Sense::click());
    let hovered = resp.hovered();
    let painter = ui.painter();
    let rad = theme.badge_radius.min(10);
    let cr = CornerRadius::same(rad);

    let (icon_col, text_col) = if theme.gloss_sheen {
        let (top_c, bot_c, border_c) = aero_btn_palette(hovered, theme);
        paint_aero_gel_pill(painter, rect, rad, top_c, bot_c, border_c);
        if hovered {
            (Color32::WHITE, Color32::WHITE)
        } else {
            (theme.header_text_secondary, theme.header_text_primary)
        }
    } else {
        let bg = if hovered {
            lerp_color(theme.header_btn_bg, theme.accent_primary, 0.14)
        } else {
            theme.header_btn_bg
        };
        let border = if hovered {
            theme.accent_primary
        } else {
            theme.header_btn_border
        };
        painter.rect_filled(rect, cr, bg);
        painter.rect_stroke(rect, cr, Stroke::new(1.0_f32, border), StrokeKind::Inside);
        let ic = if hovered {
            theme.accent_primary
        } else {
            theme.header_text_secondary
        };
        (ic, theme.header_text_primary)
    };
    draw_icon(
        painter,
        Pos2::new(rect.left() + 13.0, rect.center().y),
        icon_col,
    );
    painter.text(
        Pos2::new(rect.left() + 23.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        theme.font_ui(11.5),
        text_col,
    );
    resp
}

pub fn paint_reset_icon(painter: &Painter, center: Pos2, radius: f32, color: Color32) {
    let steps = 24usize;
    let start = -0.75_f32;
    let sweep = 4.65_f32;
    let mut pts = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let a = start + sweep * (i as f32 / steps as f32);
        pts.push(Pos2::new(
            center.x + a.cos() * radius,
            center.y + a.sin() * radius,
        ));
    }
    painter.add(Shape::line(pts, Stroke::new(1.5_f32, color)));

    // Arrowhead at the start of the arc pointing counter-clockwise
    let tip = Pos2::new(
        center.x + start.cos() * radius,
        center.y + start.sin() * radius,
    );
    let tangent = Vec2::new(start.sin(), -start.cos());
    let radial = Vec2::new(start.cos(), start.sin());
    let head = vec![
        tip + tangent * 2.8,
        tip - tangent * 1.4 + radial * 2.6,
        tip - tangent * 1.4 - radial * 2.6,
    ];
    painter.add(Shape::convex_polygon(head, color, Stroke::NONE));
}

fn paint_gear_icon(painter: &Painter, center: Pos2, color: Color32) {
    painter.circle_stroke(center, 5.2, Stroke::new(1.7_f32, color));
    painter.circle_filled(center, 1.8, color);
    for i in 0..8 {
        let a = i as f32 * PI / 4.0;
        let p0 = Pos2::new(center.x + a.cos() * 6.2, center.y + a.sin() * 6.2);
        let p1 = Pos2::new(center.x + a.cos() * 8.6, center.y + a.sin() * 8.6);
        painter.line_segment([p0, p1], Stroke::new(2.0_f32, color));
    }
}

pub fn draw_status_badge(
    ui: &mut Ui,
    theme: &ThemeSpec,
    badge: HealthBadge,
    subtitle: &str,
    time_sec: f64,
) {
    let (border_col, fill_col, text_col) = theme.badge_colors(badge);
    let pulse = if matches!(badge, HealthBadge::Critical | HealthBadge::Throttling) {
        0.72 + 0.28 * ((time_sec * 4.5).sin() as f32 * 0.5 + 0.5)
    } else {
        1.0
    };

    let sub_galley = ui.painter().layout_no_wrap(
        subtitle.to_string(),
        theme.font_body(10.5),
        with_alpha(text_col, 230),
    );
    let title_galley =
        ui.painter()
            .layout_no_wrap(badge.label().to_string(), theme.font_title(12.5), text_col);

    let w = (sub_galley.size().x.max(title_galley.size().x) + 22.0).clamp(116.0, 238.0);
    let h = 40.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, h), Sense::hover());
    let painter = ui.painter();
    let cr = CornerRadius::same(theme.badge_radius);

    if theme.glow_intensity > 0.05 {
        let glow_alpha = (22.0 * theme.glow_intensity * pulse) as u8;
        painter.rect_filled(
            rect.expand(2.5),
            CornerRadius::same(theme.badge_radius.saturating_add(2)),
            with_alpha(border_col, glow_alpha),
        );
    }

    if theme.gloss_sheen {
        painter.rect_filled(
            rect.translate(Vec2::new(0.0, 1.5)),
            cr,
            Color32::from_rgba_unmultiplied(4, 24, 52, 34),
        );
        let (top_c, bot_c, bdr_c) = match badge {
            HealthBadge::Healthy => (
                Color32::from_rgb(15, 148, 91),
                Color32::from_rgb(10, 116, 70),
                Color32::from_rgb(8, 92, 56),
            ),
            HealthBadge::Warm => (
                Color32::from_rgb(201, 107, 8),
                Color32::from_rgb(166, 82, 4),
                Color32::from_rgb(132, 64, 4),
            ),
            HealthBadge::Critical | HealthBadge::Throttling => (
                Color32::from_rgb(209, 46, 52),
                Color32::from_rgb(168, 30, 36),
                Color32::from_rgb(136, 22, 28),
            ),
        };
        paint_aero_gel_pill(painter, rect, theme.badge_radius, top_c, bot_c, bdr_c);
        let shadow_col = Color32::from_rgba_unmultiplied(0, 0, 0, 100);
        painter.text(
            Pos2::new(
                rect.center().x,
                rect.top() + 5.0 + title_galley.size().y * 0.5 + 1.0,
            ),
            Align2::CENTER_CENTER,
            badge.label(),
            theme.font_title(12.5),
            shadow_col,
        );
        painter.text(
            Pos2::new(
                rect.center().x,
                rect.top() + 21.0 + sub_galley.size().y * 0.5 + 1.0,
            ),
            Align2::CENTER_CENTER,
            subtitle,
            theme.font_body(10.5),
            shadow_col,
        );
    } else {
        painter.rect_filled(rect, cr, fill_col);
        painter.rect_stroke(
            rect,
            cr,
            Stroke::new(1.2_f32, with_alpha(border_col, (215.0 * pulse) as u8)),
            StrokeKind::Inside,
        );
    }

    painter.galley(
        Pos2::new(
            rect.center().x - title_galley.size().x * 0.5,
            rect.top() + 5.0,
        ),
        title_galley,
        text_col,
    );
    painter.galley(
        Pos2::new(
            rect.center().x - sub_galley.size().x * 0.5,
            rect.top() + 21.0,
        ),
        sub_galley,
        text_col,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn draw_arc_gauge(
    ui: &mut Ui,
    theme: &ThemeSpec,
    size: f32,
    label: &str,
    raw_value_c: Option<f32>,
    animated_value_c: Option<f32>,
    use_fahrenheit: bool,
    range: &TempRange,
    unavailable_reason: Option<&str>,
    peak_c: Option<f32>,
) {
    let total_h = size + 35.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(size, total_h), Sense::hover());
    let painter = ui.painter();

    let center = Pos2::new(rect.center().x, rect.top() + size * 0.5);
    let radius = size * 0.39;
    let thickness = match theme.gauge_style {
        GaugeStyle::CuteBubble => (size * 0.105).clamp(8.5, 14.0),
        _ => (size * 0.088).clamp(7.0, 12.5),
    };
    let start_angle = 135.0_f32.to_radians();
    let total_sweep = 270.0_f32.to_radians();

    // Inner ticks
    for i in 0..=18 {
        let t = i as f32 / 18.0;
        let a = start_angle + total_sweep * t;
        let r0 = radius - thickness * 0.92;
        let r1 = radius - thickness * 1.24;
        painter.line_segment(
            [
                Pos2::new(center.x + a.cos() * r0, center.y + a.sin() * r0),
                Pos2::new(center.x + a.cos() * r1, center.y + a.sin() * r1),
            ],
            Stroke::new(1.0_f32, with_alpha(theme.text_muted, 65)),
        );
    }

    if theme.gauge_style == GaugeStyle::TerminalBlock {
        // Discrete phosphor arc segments
        let segs = 22usize;
        let step_angle = total_sweep / segs as f32;
        let gap_angle = 0.035_f32;
        let active_segs = match (raw_value_c, animated_value_c) {
            (Some(_), Some(anim_c)) => {
                let min_scale = 20.0_f32;
                let max_scale = (range.max_spec + 6.0).max(95.0);
                let frac = ((anim_c - min_scale) / (max_scale - min_scale)).clamp(0.05, 1.0);
                ((frac * segs as f32).round() as usize).clamp(1, segs)
            }
            _ => 0,
        };
        let (col_s, col_e) = raw_value_c
            .map(|c| theme.temp_colors(evaluate_temp_level(c, range)))
            .unwrap_or((theme.text_muted, theme.text_muted));

        for i in 0..segs {
            let a0 = start_angle + i as f32 * step_angle + gap_angle * 0.5;
            let sw = (step_angle - gap_angle).max(0.02);
            let col = if i < active_segs {
                lerp_color(col_s, col_e, i as f32 / segs.max(1) as f32)
            } else {
                theme.bg_track
            };
            paint_arc_strip(painter, center, radius, thickness, a0, sw, 6, col, col, true);
        }

        if let Some(actual_c) = raw_value_c {
            let min_scale = 20.0_f32;
            let max_scale = (range.max_spec + 6.0).max(95.0);
            paint_peak_watermark_tick(
                painter,
                theme,
                center,
                radius,
                thickness,
                start_angle,
                total_sweep,
                min_scale,
                max_scale,
                peak_c,
                range,
            );
            let val_str = format_temp(actual_c, use_fahrenheit);
            let y_shift = if peak_c.is_some() { -5.0 } else { 0.0 };
            painter.text(
                Pos2::new(center.x, center.y + y_shift),
                Align2::CENTER_CENTER,
                val_str,
                theme.font_mono_bold((size * 0.21).clamp(17.0, 27.0)),
                theme.text_primary,
            );
            paint_peak_caption(
                painter,
                theme,
                center,
                radius,
                size,
                peak_c,
                range,
                use_fahrenheit,
            );
        } else {
            painter.text(
                Pos2::new(center.x, center.y - 4.0),
                Align2::CENTER_CENTER,
                "N/A",
                theme.font_mono_bold((size * 0.18).clamp(15.0, 22.0)),
                theme.text_muted,
            );
            painter.text(
                Pos2::new(center.x, center.y + 14.0),
                Align2::CENTER_CENTER,
                "No sensor",
                theme.font_ui(9.5),
                theme.text_muted,
            );
        }
    } else {
        if theme.gauge_style == GaugeStyle::AeroGloss {
            // Sleek modern Liquid Glass dial halo behind track
            let bezel_r = radius + thickness * 0.62;
            painter.circle_filled(
                Pos2::new(center.x, center.y + 1.5),
                bezel_r,
                Color32::from_rgba_unmultiplied(6, 36, 68, 18),
            );
            painter.circle_filled(
                center,
                bezel_r,
                Color32::from_rgba_unmultiplied(255, 255, 255, 195),
            );
            painter.circle_stroke(
                center,
                bezel_r,
                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(128, 182, 222, 110)),
            );
        } else if theme.gauge_style == GaugeStyle::CuteBubble {
            painter.circle_filled(center, radius * 0.72, with_alpha(theme.bg_card, 190));
        }

        // Track arc
        paint_arc_strip(
            painter,
            center,
            radius,
            thickness,
            start_angle,
            total_sweep,
            96,
            theme.bg_track,
            theme.bg_track,
            false,
        );
        painter.circle_filled(
            Pos2::new(
                center.x + start_angle.cos() * radius,
                center.y + start_angle.sin() * radius,
            ),
            thickness * 0.5,
            theme.bg_track,
        );
        let end_track = start_angle + total_sweep;
        painter.circle_filled(
            Pos2::new(
                center.x + end_track.cos() * radius,
                center.y + end_track.sin() * radius,
            ),
            thickness * 0.5,
            theme.bg_track,
        );

        if let (Some(actual_c), Some(anim_c)) = (raw_value_c, animated_value_c) {
            let level = evaluate_temp_level(actual_c, range);
            let (col_start, col_end) = theme.temp_colors(level);
            let min_scale = 20.0_f32;
            let max_scale = (range.max_spec + 6.0).max(95.0);
            let frac = ((anim_c - min_scale) / (max_scale - min_scale)).clamp(0.05, 1.0);
            let active_sweep = total_sweep * frac;

            if theme.glow_intensity > 0.05 {
                for (mul, base_alpha) in [(2.3, 14.0), (1.75, 28.0), (1.3, 52.0)] {
                    let a = (base_alpha * theme.glow_intensity).round() as u8;
                    paint_arc_strip(
                        painter,
                        center,
                        radius,
                        thickness * mul,
                        start_angle,
                        active_sweep,
                        72,
                        with_alpha(col_start, a),
                        with_alpha(col_end, a),
                        true,
                    );
                }
            }

            paint_arc_strip(
                painter,
                center,
                radius,
                thickness,
                start_angle,
                active_sweep,
                96,
                col_start,
                col_end,
                false,
            );

            if theme.gauge_style == GaugeStyle::AeroGloss {
                // Restrained modern specular gleam along outer arc
                paint_arc_strip(
                    painter,
                    center,
                    radius + thickness * 0.14,
                    thickness * 0.22,
                    start_angle,
                    active_sweep,
                    72,
                    with_alpha(Color32::WHITE, 80),
                    with_alpha(Color32::WHITE, 60),
                    true,
                );
            }

            painter.circle_filled(
                Pos2::new(
                    center.x + start_angle.cos() * radius,
                    center.y + start_angle.sin() * radius,
                ),
                thickness * 0.5,
                col_start,
            );
            let tip_angle = start_angle + active_sweep;
            let tip_pos = Pos2::new(
                center.x + tip_angle.cos() * radius,
                center.y + tip_angle.sin() * radius,
            );
            painter.circle_filled(tip_pos, thickness * 0.54, col_end);
            if theme.glow_intensity > 0.2 {
                painter.circle_filled(tip_pos, thickness * 0.25, with_alpha(Color32::WHITE, 210));
            }

            let val_str = format_temp(actual_c, use_fahrenheit);
            let val_font_size = (size * 0.215).clamp(17.0, 27.5);
            let y_shift = if peak_c.is_some() { -5.0 } else { 0.0 };
            if theme.is_dark {
                painter.text(
                    Pos2::new(center.x, center.y + y_shift + 1.0),
                    Align2::CENTER_CENTER,
                    &val_str,
                    theme.font_mono_bold(val_font_size),
                    Color32::from_rgba_unmultiplied(0, 0, 0, 120),
                );
            } else if theme.gauge_style == GaugeStyle::AeroGloss {
                painter.text(
                    Pos2::new(center.x, center.y + y_shift + 1.0),
                    Align2::CENTER_CENTER,
                    &val_str,
                    theme.font_mono_bold(val_font_size),
                    with_alpha(Color32::WHITE, 210),
                );
            }
            painter.text(
                Pos2::new(center.x, center.y + y_shift),
                Align2::CENTER_CENTER,
                val_str,
                theme.font_mono_bold(val_font_size),
                theme.text_primary,
            );

            paint_peak_watermark_tick(
                painter,
                theme,
                center,
                radius,
                thickness,
                start_angle,
                total_sweep,
                min_scale,
                max_scale,
                peak_c,
                range,
            );
            paint_peak_caption(
                painter,
                theme,
                center,
                radius,
                size,
                peak_c,
                range,
                use_fahrenheit,
            );
        } else {
            painter.text(
                Pos2::new(center.x, center.y - 4.0),
                Align2::CENTER_CENTER,
                "N/A",
                theme.font_mono_bold((size * 0.19).clamp(16.0, 24.0)),
                theme.text_muted,
            );
            painter.text(
                Pos2::new(center.x, center.y + 14.0),
                Align2::CENTER_CENTER,
                "No sensor",
                theme.font_ui(9.5),
                with_alpha(theme.text_muted, 200),
            );
        }
    }

    painter.text(
        Pos2::new(rect.center().x, rect.bottom() - 22.0),
        Align2::CENTER_CENTER,
        label,
        theme.font_title_semi(11.5),
        theme.text_secondary,
    );

    let tag_y = rect.bottom() - 7.0;
    let sensor_present = raw_value_c.is_some();
    let dot_col = if sensor_present {
        theme.status_healthy
    } else {
        with_alpha(theme.text_muted, 110)
    };
    let prefix_col = if sensor_present {
        theme.text_muted
    } else {
        with_alpha(theme.text_muted, 125)
    };
    let val_col = if sensor_present {
        theme.text_secondary
    } else {
        with_alpha(theme.text_muted, 140)
    };

    let prefix_galley =
        painter.layout_no_wrap("Safe ".to_string(), theme.font_body(10.0), prefix_col);
    let val_galley = painter.layout_no_wrap(
        format!("<{}", format_temp(range.healthy_max, use_fahrenheit)),
        theme.font_mono_bold(10.0),
        val_col,
    );
    let prefix_w = prefix_galley.size().x;
    let prefix_h = prefix_galley.size().y;
    let val_w = val_galley.size().x;
    let val_h = val_galley.size().y;

    let dot_r = 2.5_f32;
    let dot_gap = 4.5_f32;
    let total_w = dot_r * 2.0 + dot_gap + prefix_w + val_w;
    let start_x = rect.center().x - total_w * 0.5;

    if sensor_present && theme.glow_intensity > 0.2 {
        painter.circle_filled(
            Pos2::new(start_x + dot_r, tag_y),
            dot_r + 2.0,
            with_alpha(
                theme.status_healthy,
                (36.0 * theme.glow_intensity).round() as u8,
            ),
        );
    }
    painter.circle_filled(Pos2::new(start_x + dot_r, tag_y), dot_r, dot_col);

    let prefix_x = start_x + dot_r * 2.0 + dot_gap;
    painter.galley(
        Pos2::new(prefix_x, tag_y - prefix_h * 0.5),
        prefix_galley,
        prefix_col,
    );
    painter.galley(
        Pos2::new(prefix_x + prefix_w, tag_y - val_h * 0.5),
        val_galley,
        val_col,
    );

    resp.on_hover_ui(|ui| {
        draw_tooltip_body(ui, 224.0, |ui| {
            let badge = if let Some(c) = raw_value_c {
                match evaluate_temp_level(c, range) {
                    MetricLevel::Healthy => ("Healthy", theme.status_healthy),
                    MetricLevel::Warm => ("Warm", theme.status_warm),
                    MetricLevel::Critical => ("Critical", theme.status_critical),
                }
            } else {
                ("No Sensor", theme.text_muted)
            };
            draw_tooltip_header(ui, theme, &label.to_ascii_uppercase(), Some(badge));

            if let Some(c) = raw_value_c {
                let cur_col = match evaluate_temp_level(c, range) {
                    MetricLevel::Healthy => theme.text_primary,
                    MetricLevel::Warm => theme.status_warm,
                    MetricLevel::Critical => theme.status_critical,
                };
                draw_tooltip_kv(
                    ui,
                    theme,
                    None,
                    "Current",
                    &format!(
                        "{:.1}{}",
                        display_temp(c, use_fahrenheit),
                        temp_unit(use_fahrenheit)
                    ),
                    cur_col,
                    true,
                );
                if let Some(pk) = peak_c {
                    let pk_col = match evaluate_temp_level(pk, range) {
                        MetricLevel::Healthy => theme.text_primary,
                        MetricLevel::Warm => theme.status_warm,
                        MetricLevel::Critical => theme.status_critical,
                    };
                    draw_tooltip_kv(
                        ui,
                        theme,
                        None,
                        "Recorded Max",
                        &format!(
                            "{:.1}{}",
                            display_temp(pk, use_fahrenheit),
                            temp_unit(use_fahrenheit)
                        ),
                        pk_col,
                        true,
                    );
                }
            } else {
                ui.label(
                    egui::RichText::new(unavailable_reason.unwrap_or("No hardware sensor exposed"))
                        .font(theme.font_body(11.0))
                        .color(theme.text_muted),
                );
            }

            draw_tooltip_section(ui, theme, "THRESHOLDS");
            draw_tooltip_kv(
                ui,
                theme,
                Some(theme.status_healthy),
                "Healthy",
                &format!("< {}", format_temp(range.healthy_max, use_fahrenheit)),
                theme.text_primary,
                false,
            );
            draw_tooltip_kv(
                ui,
                theme,
                Some(theme.status_warm),
                "Warm",
                &format!(
                    "{:.0}–{}",
                    display_temp(range.healthy_max, use_fahrenheit),
                    format_temp(range.warn_max, use_fahrenheit)
                ),
                theme.status_warm,
                false,
            );
            draw_tooltip_kv(
                ui,
                theme,
                Some(theme.status_critical),
                "Max Spec",
                &format_temp(range.max_spec, use_fahrenheit),
                theme.status_critical,
                false,
            );
        });
    });
}

#[allow(clippy::too_many_arguments)]
fn paint_peak_watermark_tick(
    painter: &Painter,
    theme: &ThemeSpec,
    center: Pos2,
    radius: f32,
    thickness: f32,
    start_angle: f32,
    total_sweep: f32,
    min_scale: f32,
    max_scale: f32,
    peak_c: Option<f32>,
    range: &TempRange,
) {
    let Some(pk) = peak_c else {
        return;
    };
    if pk < min_scale {
        return;
    }
    let pk_frac = ((pk - min_scale) / (max_scale - min_scale)).clamp(0.05, 1.0);
    let pk_angle = start_angle + total_sweep * pk_frac;
    let r_in = radius - thickness * 0.68;
    let r_out = radius + thickness * 0.72;
    let pk_col = match evaluate_temp_level(pk, range) {
        crate::telemetry::guidelines::MetricLevel::Healthy => theme.text_primary,
        crate::telemetry::guidelines::MetricLevel::Warm => theme.status_warm,
        crate::telemetry::guidelines::MetricLevel::Critical => theme.status_critical,
    };
    let p_in = Pos2::new(
        center.x + pk_angle.cos() * r_in,
        center.y + pk_angle.sin() * r_in,
    );
    let p_out = Pos2::new(
        center.x + pk_angle.cos() * r_out,
        center.y + pk_angle.sin() * r_out,
    );
    if theme.gauge_style == GaugeStyle::AeroGloss {
        painter.line_segment(
            [p_in, p_out],
            Stroke::new(3.2_f32, with_alpha(theme.text_primary, 210)),
        );
        painter.line_segment(
            [p_in, p_out],
            Stroke::new(1.4_f32, with_alpha(Color32::WHITE, 235)),
        );
    } else {
        painter.line_segment([p_in, p_out], Stroke::new(2.0_f32, with_alpha(pk_col, 230)));
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_peak_caption(
    painter: &Painter,
    theme: &ThemeSpec,
    center: Pos2,
    radius: f32,
    size: f32,
    peak_c: Option<f32>,
    range: &TempRange,
    use_fahrenheit: bool,
) {
    let Some(pk) = peak_c else {
        return;
    };
    let pk_col = match evaluate_temp_level(pk, range) {
        crate::telemetry::guidelines::MetricLevel::Healthy => theme.text_secondary,
        crate::telemetry::guidelines::MetricLevel::Warm => theme.status_warm,
        crate::telemetry::guidelines::MetricLevel::Critical => theme.status_critical,
    };
    let font_sz = (size * 0.082).clamp(9.0, 10.5);
    painter.text(
        Pos2::new(center.x, center.y + radius * 0.45),
        Align2::CENTER_CENTER,
        format!("Max {}", format_temp(pk, use_fahrenheit)),
        theme.font_mono(font_sz),
        pk_col,
    );
}

fn feather_width(painter: &Painter) -> f32 {
    let px = 1.0 / painter.ctx().pixels_per_point().max(1.0);
    px * 1.15
}

fn push_ribbon_indices(mesh: &mut Mesh, slice_count: usize) {
    // Each slice has 4 vertices: [outer_low, inner_low, inner_high, outer_high].
    for i in 0..slice_count.saturating_sub(1) {
        let a = (i * 4) as u32;
        let b = a + 4;
        for band in 0..3u32 {
            let v0 = a + band;
            let v1 = v0 + 1;
            let v2 = b + band;
            let v3 = v2 + 1;
            mesh.indices.extend_from_slice(&[v0, v1, v2, v1, v3, v2]);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_arc_strip(
    painter: &Painter,
    center: Pos2,
    radius: f32,
    thickness: f32,
    start_angle: f32,
    sweep_angle: f32,
    segments: usize,
    color_start: Color32,
    color_end: Color32,
    feather_ends: bool,
) {
    if sweep_angle.abs() < 0.001 || segments == 0 {
        return;
    }
    let aa = feather_width(painter);
    let half_t = thickness * 0.5;
    let r_in = (radius - half_t).max(0.5);
    let r_out = radius + half_t;
    let inset = (aa * 0.5).min((r_out - r_in) * 0.45);
    let r0 = (r_in - aa * 0.5).max(0.1);
    let r1 = r_in + inset;
    let r2 = r_out - inset;
    let r3 = r_out + aa * 0.5;

    let mut mesh = Mesh::default();
    let push_slice = |mesh: &mut Mesh, angle: f32, edge_col: Color32, core_col: Color32| {
        let cos = angle.cos();
        let sin = angle.sin();
        for (r, c) in [(r0, edge_col), (r1, core_col), (r2, core_col), (r3, edge_col)] {
            mesh.vertices.push(Vertex {
                pos: Pos2::new(center.x + cos * r, center.y + sin * r),
                uv: WHITE_UV,
                color: c,
            });
        }
    };

    let mut slice_count = segments + 1;
    let (core_start, core_sweep) = if feather_ends {
        let sgn = sweep_angle.signum();
        let d_ang = (aa / radius.max(1.0)).min(sweep_angle.abs() * 0.25);
        push_slice(
            &mut mesh,
            start_angle - sgn * d_ang * 0.5,
            Color32::TRANSPARENT,
            Color32::TRANSPARENT,
        );
        slice_count += 2;
        (start_angle + sgn * d_ang * 0.5, sweep_angle - sgn * d_ang)
    } else {
        (start_angle, sweep_angle)
    };

    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let a = core_start + core_sweep * t;
        let col = lerp_color(color_start, color_end, t);
        push_slice(&mut mesh, a, Color32::TRANSPARENT, col);
    }

    if feather_ends {
        let sgn = sweep_angle.signum();
        let d_ang = (aa / radius.max(1.0)).min(sweep_angle.abs() * 0.25);
        push_slice(
            &mut mesh,
            start_angle + sweep_angle + sgn * d_ang * 0.5,
            Color32::TRANSPARENT,
            Color32::TRANSPARENT,
        );
    }

    push_ribbon_indices(&mut mesh, slice_count);
    painter.add(Shape::mesh(mesh));
}

fn paint_gradient_line(
    painter: &Painter,
    p0: Pos2,
    p1: Pos2,
    thickness: f32,
    c0: Color32,
    c1: Color32,
) {
    let delta = p1 - p0;
    let len = delta.length();
    if len < 0.5 {
        return;
    }
    let aa = feather_width(painter);
    let normal = Vec2::new(-delta.y / len, delta.x / len);
    let half_t = thickness * 0.5;
    let inset = (aa * 0.5).min(half_t * 0.45);
    let offsets = [
        (-(half_t + aa * 0.5), true),
        (-(half_t - inset), false),
        (half_t - inset, false),
        (half_t + aa * 0.5, true),
    ];

    let mut mesh = Mesh::default();
    for (pt, col) in [(p0, c0), (p1, c1)] {
        for (off, is_edge) in offsets {
            mesh.vertices.push(Vertex {
                pos: pt + normal * off,
                uv: WHITE_UV,
                color: if is_edge { Color32::TRANSPARENT } else { col },
            });
        }
    }
    push_ribbon_indices(&mut mesh, 2);
    painter.add(Shape::mesh(mesh));
}

fn paint_feathered_hgradient_rect(
    painter: &Painter,
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    left_col: Color32,
    right_col: Color32,
) {
    let h = bottom - top;
    if right <= left || h <= 0.1 {
        return;
    }
    let aa = feather_width(painter);
    let inset = (aa * 0.5).min(h * 0.45);
    let ys = [
        (top - aa * 0.5, true),
        (top + inset, false),
        (bottom - inset, false),
        (bottom + aa * 0.5, true),
    ];

    let mut mesh = Mesh::default();
    for (x, col) in [(left, left_col), (right, right_col)] {
        for (y, is_edge) in ys {
            mesh.vertices.push(Vertex {
                pos: Pos2::new(x, y),
                uv: WHITE_UV,
                color: if is_edge { Color32::TRANSPARENT } else { col },
            });
        }
    }
    push_ribbon_indices(&mut mesh, 2);
    painter.add(Shape::mesh(mesh));
}

#[allow(clippy::too_many_arguments)]
pub fn draw_glow_bar(
    ui: &mut Ui,
    theme: &ThemeSpec,
    width: f32,
    height: f32,
    fraction: f32,
    start_color: Color32,
    end_color: Color32,
    marker_fraction: Option<f32>,
) {
    let h = if theme.bar_style == BarStyle::AeroGlossy {
        height.max(9.5)
    } else {
        height
    };
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, h), Sense::hover());
    let painter = ui.painter();
    let frac = fraction.clamp(0.0, 1.0);

    if theme.bar_style == BarStyle::TerminalBlocks {
        let count = 20usize;
        let gap = 2.0;
        let seg_w = ((rect.width() - gap * (count as f32 - 1.0)) / count as f32).max(2.0);
        let filled = (frac * count as f32).round() as usize;
        for i in 0..count {
            let x = rect.left() + i as f32 * (seg_w + gap);
            let seg = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(seg_w, h));
            let col = if i < filled {
                lerp_color(start_color, end_color, i as f32 / count.max(1) as f32)
            } else {
                theme.bg_track
            };
            painter.rect_filled(seg, CornerRadius::ZERO, col);
        }
    } else if theme.bar_style == BarStyle::AeroGlossy {
        let rad = (h * 0.5).round() as u8;
        let cr = CornerRadius::same(rad);
        // Airy modern frosted trough
        painter.rect_filled(rect, cr, Color32::from_rgb(206, 227, 243));
        painter.rect_stroke(
            rect,
            cr,
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(124, 174, 212, 110)),
            StrokeKind::Inside,
        );

        if frac > 0.01 {
            let fill_w = (rect.width() * frac).max(h);
            let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w.min(rect.width()), h));
            let mid_col = lerp_color(start_color, end_color, frac);
            let cap_r = h * 0.5;
            let inner_left = fill_rect.left() + cap_r;
            let inner_right = (fill_rect.right() - cap_r).max(inner_left);
            painter.circle_filled(Pos2::new(inner_left, rect.center().y), cap_r, start_color);
            painter.circle_filled(Pos2::new(inner_right, rect.center().y), cap_r, mid_col);
            if inner_right > inner_left {
                paint_feathered_hgradient_rect(
                    painter,
                    inner_left,
                    inner_right,
                    fill_rect.top(),
                    fill_rect.bottom(),
                    start_color,
                    mid_col,
                );
            }
            // Delicate 2px white glass gleam along upper rim
            let gleam_rect = Rect::from_min_max(
                Pos2::new(fill_rect.left() + 2.0, fill_rect.top() + 0.8),
                Pos2::new(
                    fill_rect.right() - 2.0,
                    (fill_rect.top() + 2.8).min(fill_rect.center().y),
                ),
            );
            if gleam_rect.width() > 2.0 {
                painter.rect_filled(
                    gleam_rect,
                    CornerRadius::same(rad.saturating_sub(1)),
                    with_alpha(Color32::WHITE, 85),
                );
            }
        }
    } else {
        let cr = CornerRadius::same(if theme.badge_radius == 0 {
            0
        } else {
            (h * 0.5).round() as u8
        });

        painter.rect_filled(rect, cr, theme.bg_track);

        if frac > 0.01 {
            let fill_w = (rect.width() * frac).max(h);
            let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w.min(rect.width()), h));
            let mid_col = lerp_color(start_color, end_color, frac);

            if theme.glow_intensity > 0.05 {
                let glow_a = (44.0 * theme.glow_intensity).round() as u8;
                painter.rect_filled(
                    fill_rect.expand2(Vec2::new(1.0, 2.5)),
                    cr,
                    with_alpha(mid_col, glow_a),
                );
            }

            let cap_r = h * 0.5;
            let inner_left = fill_rect.left() + cap_r;
            let inner_right = (fill_rect.right() - cap_r).max(inner_left);
            painter.circle_filled(Pos2::new(inner_left, rect.center().y), cap_r, start_color);
            painter.circle_filled(Pos2::new(inner_right, rect.center().y), cap_r, mid_col);
            if inner_right > inner_left {
                paint_feathered_hgradient_rect(
                    painter,
                    inner_left,
                    inner_right,
                    fill_rect.top(),
                    fill_rect.bottom(),
                    start_color,
                    mid_col,
                );
            }
        }
    }

    if let Some(m_frac) = marker_fraction {
        let mx = rect.left() + rect.width() * m_frac.clamp(0.02, 0.98);
        painter.line_segment(
            [
                Pos2::new(mx, rect.top() - 1.5),
                Pos2::new(mx, rect.bottom() + 1.5),
            ],
            Stroke::new(1.5_f32, with_alpha(theme.text_primary, 200)),
        );
    }
}

pub fn draw_segmented_throttle_bar(
    ui: &mut Ui,
    theme: &ThemeSpec,
    width: f32,
    height: f32,
    active: bool,
    time_sec: f64,
) {
    if !active {
        draw_glow_bar(
            ui,
            theme,
            width,
            height,
            1.0,
            theme.status_healthy,
            lerp_color(theme.status_healthy, theme.accent_secondary, 0.25),
            None,
        );
        return;
    }

    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height + 4.0), Sense::hover());
    let painter = ui.painter();

    let count = 18usize;
    let gap = 3.0;
    let seg_w = ((rect.width() - gap * (count as f32 - 1.0)) / count as f32).max(3.0);
    let seg_cr = CornerRadius::same(if theme.badge_radius == 0 { 0 } else { 2 });

    for i in 0..count {
        let x = rect.left() + i as f32 * (seg_w + gap);
        let seg_rect =
            Rect::from_min_size(Pos2::new(x, rect.top() + 2.0), Vec2::new(seg_w, height));
        let wave = ((time_sec * 5.0 - i as f64 * 0.25).sin() * 0.5 + 0.5) as f32;
        let col = lerp_color(theme.status_critical, theme.status_warm, wave * 0.45);
        if theme.glow_intensity > 0.05 {
            let glow_alpha = ((25.0 + 40.0 * wave) * theme.glow_intensity) as u8;
            painter.rect_filled(seg_rect.expand(1.5), seg_cr, with_alpha(col, glow_alpha));
        }
        painter.rect_filled(
            seg_rect,
            seg_cr,
            with_alpha(col, (165.0 + 90.0 * wave) as u8),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_sparkline_chart(
    ui: &mut Ui,
    theme: &ThemeSpec,
    size: Vec2,
    title: &str,
    readout: &str,
    points: &VecDeque<f32>,
    min_y: f32,
    max_y: f32,
    unit: &str,
    accent_color: Color32,
    secondary_points: Option<&VecDeque<f32>>,
    secondary_color: Option<Color32>,
) {
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    let cr = CornerRadius::same(theme.inner_radius);

    if theme.gloss_sheen {
        for shape in build_aero_recessed_well_shapes(
            rect,
            theme.inner_radius,
            theme.bg_inner,
            theme.border_subtle,
        ) {
            painter.add(shape);
        }
    } else {
        painter.rect_filled(rect, cr, theme.bg_inner);
        painter.rect_stroke(
            rect,
            cr,
            Stroke::new(1.0_f32, theme.border_subtle),
            StrokeKind::Inside,
        );
    }

    painter.text(
        Pos2::new(rect.left() + 8.0, rect.top() + 10.0),
        Align2::LEFT_CENTER,
        title,
        theme.font_title_semi(10.5),
        theme.text_secondary,
    );
    painter.text(
        Pos2::new(rect.right() - 8.0, rect.top() + 10.0),
        Align2::RIGHT_CENTER,
        readout,
        theme.font_mono_bold(10.0),
        accent_color,
    );

    let plot = Rect::from_min_max(
        Pos2::new(rect.left() + 28.0, rect.top() + 22.0),
        Pos2::new(rect.right() - 8.0, rect.bottom() - 7.0),
    );
    if plot.height() < 12.0 || plot.width() < 20.0 {
        return;
    }

    let mut hi = max_y.max(min_y + 1.0);
    for &v in points.iter() {
        if v > hi {
            hi = v * 1.1;
        }
    }
    if let Some(sec) = secondary_points {
        for &v in sec.iter() {
            if v > hi {
                hi = v * 1.1;
            }
        }
    }

    for (t, val) in [(0.0, hi), (0.5, (min_y + hi) * 0.5), (1.0, min_y)] {
        let y = plot.top() + plot.height() * t;
        painter.line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(0.8_f32, theme.chart_grid),
        );
        let label = if hi >= 100.0 || val == 0.0 {
            format!("{:.0}", val)
        } else {
            format!("{:.1}", val)
        };
        painter.text(
            Pos2::new(plot.left() - 4.0, y),
            Align2::RIGHT_CENTER,
            label,
            theme.font_mono(8.5),
            theme.text_muted,
        );
    }

    if let (Some(sec_pts), Some(sec_col)) = (secondary_points, secondary_color) {
        if sec_pts.len() >= 2 {
            paint_series(
                painter,
                plot,
                sec_pts,
                min_y,
                hi,
                sec_col,
                false,
                theme.is_dark,
            );
        }
    }

    if points.len() >= 2 {
        paint_series(
            painter,
            plot,
            points,
            min_y,
            hi,
            accent_color,
            true,
            theme.is_dark,
        );
    } else if let Some(&single) = points.back() {
        let y = map_y(single, min_y, hi, plot);
        painter.line_segment(
            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
            Stroke::new(1.5_f32, accent_color),
        );
    } else {
        painter.text(
            plot.center(),
            Align2::CENTER_CENTER,
            "—",
            theme.font_body(11.0),
            theme.text_muted,
        );
    }

    if !points.is_empty() {
        resp.on_hover_ui(|ui| {
            draw_tooltip_body(ui, 204.0, |ui| {
                let cur = points.back().copied().unwrap_or(0.0);
                let min_v = points.iter().copied().fold(f32::INFINITY, f32::min);
                let max_v = points.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                draw_tooltip_header(ui, theme, &title.to_ascii_uppercase(), None);
                draw_tooltip_kv(
                    ui,
                    theme,
                    None,
                    "Current",
                    &format!("{cur:.1}{unit}"),
                    accent_color,
                    true,
                );
                draw_tooltip_kv(
                    ui,
                    theme,
                    None,
                    "Window Min",
                    &format!("{min_v:.1}{unit}"),
                    theme.text_primary,
                    false,
                );
                draw_tooltip_kv(
                    ui,
                    theme,
                    None,
                    "Window Max",
                    &format!("{max_v:.1}{unit}"),
                    theme.text_primary,
                    false,
                );
            });
        });
    }
}

pub fn draw_tooltip_body<R>(
    ui: &mut Ui,
    width: f32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    ui.set_max_width(width + 6.0);
    Frame::new()
        .inner_margin(Margin::symmetric(3, 2))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_max_width(width);
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
            add_contents(ui)
        })
        .inner
}

pub fn draw_tooltip_header(
    ui: &mut Ui,
    theme: &ThemeSpec,
    title: &str,
    badge: Option<(&str, Color32)>,
) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(title)
                .font(theme.font_title(12.0))
                .color(theme.text_primary),
        );
        if let Some((badge_text, badge_col)) = badge {
            ui.add_space(10.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let galley = ui.painter().layout_no_wrap(
                    badge_text.to_string(),
                    theme.font_value(10.0),
                    badge_col,
                );
                let pill_w = galley.size().x + 12.0;
                let (rect, _) = ui.allocate_exact_size(Vec2::new(pill_w, 18.0), Sense::hover());
                let cr = CornerRadius::same(theme.badge_radius.min(5).max(3));
                let p = ui.painter();
                p.rect_filled(rect, cr, with_alpha(badge_col, 26));
                p.rect_stroke(
                    rect,
                    cr,
                    Stroke::new(0.9_f32, with_alpha(badge_col, 140)),
                    StrokeKind::Inside,
                );
                p.galley(
                    Pos2::new(
                        rect.center().x - galley.size().x * 0.5,
                        rect.center().y - galley.size().y * 0.5,
                    ),
                    galley,
                    badge_col,
                );
            });
        }
    });
    ui.add_space(2.0);
    let (div_r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(div_r.left(), div_r.center().y),
            Pos2::new(div_r.right(), div_r.center().y),
        ],
        Stroke::new(1.0_f32, with_alpha(theme.border_subtle, 160)),
    );
    ui.add_space(3.0);
}

pub fn draw_tooltip_section(ui: &mut Ui, theme: &ThemeSpec, label: &str) {
    ui.add_space(3.0);
    let (div_r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(div_r.left(), div_r.center().y),
            Pos2::new(div_r.right(), div_r.center().y),
        ],
        Stroke::new(1.0_f32, with_alpha(theme.border_subtle, 145)),
    );
    ui.add_space(3.0);
    ui.label(
        egui::RichText::new(label)
            .font(theme.font_title_semi(9.5))
            .color(theme.text_muted),
    );
    ui.add_space(1.0);
}

pub fn draw_tooltip_kv(
    ui: &mut Ui,
    theme: &ThemeSpec,
    dot: Option<Color32>,
    key: &str,
    value: &str,
    value_col: Color32,
    bold_value: bool,
) {
    ui.horizontal(|ui| {
        if let Some(dot_col) = dot {
            let (dot_r, _) = ui.allocate_exact_size(Vec2::new(6.0, 14.0), Sense::hover());
            ui.painter().circle_filled(dot_r.center(), 2.8, dot_col);
        }
        ui.label(
            egui::RichText::new(key)
                .font(theme.font_ui(12.0))
                .color(theme.text_secondary),
        );
        ui.add_space(12.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let font = if bold_value {
                theme.font_mono_bold(12.5)
            } else {
                theme.font_mono(11.5)
            };
            ui.label(egui::RichText::new(value).font(font).color(value_col));
        });
    });
}

fn map_y(v: f32, min_y: f32, max_y: f32, plot: Rect) -> f32 {
    let frac = ((v - min_y) / (max_y - min_y).max(0.001)).clamp(0.0, 1.0);
    plot.bottom() - frac * plot.height()
}

#[allow(clippy::too_many_arguments)]
fn paint_series(
    painter: &Painter,
    plot: Rect,
    points: &VecDeque<f32>,
    min_y: f32,
    max_y: f32,
    color: Color32,
    fill_area: bool,
    is_dark: bool,
) {
    let n = points.len();
    let mut screen_pts = Vec::with_capacity(n);
    for (i, &val) in points.iter().enumerate() {
        let x = if n == 1 {
            plot.right()
        } else {
            plot.left() + (i as f32 / (n - 1) as f32) * plot.width()
        };
        let y = map_y(val, min_y, max_y, plot);
        screen_pts.push(Pos2::new(x, y));
    }

    if fill_area && screen_pts.len() >= 2 {
        let mut mesh = Mesh::default();
        let top_col = with_alpha(color, if is_dark { 62 } else { 44 });
        let bot_col = with_alpha(color, 4);
        for &pt in &screen_pts {
            mesh.vertices.push(Vertex {
                pos: pt,
                uv: WHITE_UV,
                color: top_col,
            });
            mesh.vertices.push(Vertex {
                pos: Pos2::new(pt.x, plot.bottom()),
                uv: WHITE_UV,
                color: bot_col,
            });
        }
        for i in 0..(screen_pts.len() - 1) {
            let idx = (i * 2) as u32;
            mesh.indices
                .extend_from_slice(&[idx, idx + 1, idx + 2, idx + 1, idx + 3, idx + 2]);
        }
        painter.add(Shape::mesh(mesh));
    }

    painter.add(Shape::line(screen_pts, Stroke::new(1.6_f32, color)));
}

pub fn paint_lightning_icon(painter: &Painter, center: Pos2, color: Color32) {
    let pts = vec![
        Pos2::new(center.x + 1.5, center.y - 6.0),
        Pos2::new(center.x - 4.0, center.y + 0.8),
        Pos2::new(center.x - 0.3, center.y + 0.8),
        Pos2::new(center.x - 2.0, center.y + 6.0),
        Pos2::new(center.x + 4.0, center.y - 0.8),
        Pos2::new(center.x + 0.4, center.y - 0.8),
    ];
    painter.add(Shape::convex_polygon(
        vec![pts[0], pts[1], pts[2]],
        color,
        Stroke::NONE,
    ));
    painter.add(Shape::convex_polygon(
        vec![pts[2], pts[4], pts[3]],
        color,
        Stroke::NONE,
    ));
    painter.add(Shape::line(
        vec![pts[0], pts[1], pts[2], pts[3], pts[4], pts[5], pts[0]],
        Stroke::new(0.8_f32, color),
    ));
}

#[allow(clippy::too_many_arguments)]
fn paint_aurora_ribbon(
    painter: &Painter,
    rect: Rect,
    y_start_frac: f32,
    y_end_frac: f32,
    amp_frac: f32,
    phase: f32,
    half_band: f32,
    col_start: Color32,
    col_end: Color32,
    peak_alpha: u8,
) {
    let steps = 36usize;
    let mut mesh = Mesh::default();
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = rect.left() + t * rect.width();
        let base_y = rect.top() + (y_start_frac + (y_end_frac - y_start_frac) * t) * rect.height();
        let wave = (t * PI * 1.6 + phase).sin() * (rect.height() * amp_frac);
        let cy = base_y + wave;
        let edge_fade = (t * PI).sin().powf(0.45);
        let a = (peak_alpha as f32 * edge_fade).round() as u8;
        let col = lerp_color(col_start, col_end, t);
        let transparent = with_alpha(col, 0);
        let center_col = with_alpha(col, a);

        mesh.vertices.push(Vertex {
            pos: Pos2::new(x, cy - half_band),
            uv: WHITE_UV,
            color: transparent,
        });
        mesh.vertices.push(Vertex {
            pos: Pos2::new(x, cy),
            uv: WHITE_UV,
            color: center_col,
        });
        mesh.vertices.push(Vertex {
            pos: Pos2::new(x, cy + half_band),
            uv: WHITE_UV,
            color: transparent,
        });
    }

    for i in 0..steps {
        let b = (i * 3) as u32;
        // Top half strip
        mesh.indices
            .extend_from_slice(&[b, b + 1, b + 3, b + 1, b + 4, b + 3]);
        // Bottom half strip
        mesh.indices
            .extend_from_slice(&[b + 1, b + 2, b + 4, b + 2, b + 5, b + 4]);
    }
    painter.add(Shape::mesh(mesh));
}

#[allow(clippy::too_many_arguments)]
pub fn build_aero_glass_shapes(
    rect: Rect,
    cr: CornerRadius,
    top_hi: Color32,
    _top_lo: Color32,
    _bot_hi: Color32,
    bot_lo: Color32,
    outer_border: Color32,
    _diagonal_glare: bool,
) -> Vec<Shape> {
    let mut shapes = Vec::new();
    shapes.push(Shape::rect_filled(rect, cr, bot_lo));

    // Gentle soft upper frost sheen (no harsh 2007 mid-line step)
    let top_h = (rect.height() * 0.34).max(4.0);
    let top_cr = CornerRadius {
        nw: cr.nw.saturating_sub(1),
        ne: cr.ne.saturating_sub(1),
        sw: 0,
        se: 0,
    };
    let top_rect = Rect::from_min_max(
        Pos2::new(rect.left() + 1.0, rect.top() + 1.0),
        Pos2::new(
            rect.right() - 1.0,
            (rect.top() + top_h).min(rect.bottom() - 1.0),
        ),
    );
    shapes.push(Shape::rect_filled(
        top_rect,
        top_cr,
        with_alpha(top_hi, (top_hi.a() as f32 * 0.45) as u8),
    ));

    if outer_border.a() > 0 {
        let inner_rim_cr = CornerRadius {
            nw: cr.nw.saturating_sub(1),
            ne: cr.ne.saturating_sub(1),
            sw: cr.sw.saturating_sub(1),
            se: cr.se.saturating_sub(1),
        };
        shapes.push(Shape::rect_stroke(
            rect.shrink(1.0),
            inner_rim_cr,
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 215)),
            StrokeKind::Inside,
        ));
        shapes.push(Shape::rect_stroke(
            rect,
            cr,
            Stroke::new(1.0_f32, outer_border),
            StrokeKind::Inside,
        ));
    }

    shapes
}

pub fn paint_aero_gel_pill(
    painter: &Painter,
    rect: Rect,
    radius: u8,
    top_col: Color32,
    bot_col: Color32,
    border_col: Color32,
) {
    let cr = CornerRadius::same(radius);
    painter.rect_filled(rect, cr, bot_col);

    // Soft upper tint preserving rich color saturation for crisp text contrast
    let top_rect = Rect::from_min_max(
        Pos2::new(rect.left() + 1.0, rect.top() + 1.0),
        Pos2::new(rect.right() - 1.0, rect.top() + rect.height() * 0.46),
    );
    let top_cr = CornerRadius {
        nw: radius.saturating_sub(1),
        ne: radius.saturating_sub(1),
        sw: (radius / 3).min(4),
        se: (radius / 3).min(4),
    };
    let soft_upper = lerp_color(top_col, bot_col, 0.25);
    painter.rect_filled(top_rect, top_cr, with_alpha(soft_upper, 140));

    // Delicate thin top-edge specular gleam (2-3px rim only, never washing out text)
    if rect.height() >= 10.0 {
        let rim_h = (rect.height() * 0.14).clamp(2.0, 3.0);
        let rim_rect = Rect::from_min_max(
            Pos2::new(rect.left() + 2.0, rect.top() + 1.2),
            Pos2::new(rect.right() - 2.0, rect.top() + 1.2 + rim_h),
        );
        painter.rect_filled(
            rim_rect,
            CornerRadius {
                nw: radius.saturating_sub(2),
                ne: radius.saturating_sub(2),
                sw: 1,
                se: 1,
            },
            Color32::from_rgba_unmultiplied(255, 255, 255, 95),
        );
    }

    painter.rect_stroke(
        rect,
        cr,
        Stroke::new(1.0_f32, with_alpha(border_col, 150)),
        StrokeKind::Inside,
    );
}

pub fn paint_aero_orb(painter: &Painter, center: Pos2, radius: f32, base_col: Color32) {
    let dark_rim = lerp_color(base_col, Color32::BLACK, 0.38);
    let caustic = lerp_color(base_col, Color32::WHITE, 0.45);
    painter.circle_filled(center, radius, base_col);
    painter.circle_filled(
        Pos2::new(center.x, center.y + radius * 0.28),
        radius * 0.62,
        with_alpha(caustic, 95),
    );
    painter.circle_filled(
        Pos2::new(center.x - radius * 0.22, center.y - radius * 0.30),
        radius * 0.44,
        with_alpha(Color32::WHITE, 195),
    );
    painter.circle_stroke(center, radius, Stroke::new(0.9_f32, dark_rim));
}

pub fn build_aero_recessed_well_shapes(
    rect: Rect,
    radius: u8,
    fill: Color32,
    border: Color32,
) -> Vec<Shape> {
    let cr = CornerRadius::same(radius);
    let mut shapes = Vec::new();
    shapes.push(Shape::rect_filled(rect, cr, fill));

    let pad = (radius as f32 * 0.45).max(4.0);
    // Subtle top inner recess shadow
    shapes.push(Shape::line_segment(
        [
            Pos2::new(rect.left() + pad, rect.top() + 1.2),
            Pos2::new(rect.right() - pad, rect.top() + 1.2),
        ],
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(68, 122, 168, 85)),
    ));
    // Crisp white lower glass bevel edge
    shapes.push(Shape::line_segment(
        [
            Pos2::new(rect.left() + pad, rect.bottom() - 1.0),
            Pos2::new(rect.right() - pad, rect.bottom() - 1.0),
        ],
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 205)),
    ));
    shapes.push(Shape::rect_stroke(
        rect,
        cr,
        Stroke::new(1.0_f32, border),
        StrokeKind::Inside,
    ));
    shapes
}
