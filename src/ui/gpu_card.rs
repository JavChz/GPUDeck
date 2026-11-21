use std::collections::VecDeque;

use egui::{
    Align2, Color32, CornerRadius, Frame, Margin, Pos2, Rect, Sense, Shadow, Shape, Stroke,
    StrokeKind, Ui, Vec2,
};

use super::theme::{display_temp, format_temp, temp_unit, with_alpha, ThemeSpec};
use super::widgets::{
    build_aero_recessed_well_shapes, draw_arc_gauge, draw_glow_bar, draw_segmented_throttle_bar,
    draw_sparkline_chart, draw_status_badge, paint_aero_gel_pill, paint_aero_orb,
    paint_lightning_icon, paint_reset_icon,
};
use crate::telemetry::guidelines::{evaluate_temp_level, HealthBadge, MetricLevel};
use crate::telemetry::{GpuSnapshot, PowerLimitMode};

#[derive(Debug, Clone, Default)]
pub struct AnimatedGpuValues {
    pub core: Option<f32>,
    pub hotspot: Option<f32>,
    pub memory: Option<f32>,
    pub power_frac: f32,
    pub limit_frac: f32,
}

impl AnimatedGpuValues {
    pub fn step_towards(&mut self, gpu: &GpuSnapshot, dt: f32) {
        let alpha = (dt * 7.5).clamp(0.05, 1.0);
        self.core = lerp_opt(self.core, gpu.thermals.core, alpha);
        self.hotspot = lerp_opt(self.hotspot, gpu.thermals.hotspot, alpha);
        self.memory = lerp_opt(self.memory, gpu.thermals.memory, alpha);

        let target_power = match (
            gpu.power.draw_w,
            gpu.power
                .enforced_limit_w
                .or(gpu.power.active_limit_w)
                .or(gpu.power.default_limit_w),
        ) {
            (Some(d), Some(l)) if l > 0.1 => (d / l).clamp(0.0, 1.0),
            _ => 0.0,
        };
        self.power_frac += (target_power - self.power_frac) * alpha;

        let target_limit = match gpu.power.limit_pct_of_default {
            Some(pct) => (pct / 100.0).clamp(0.0, 1.0),
            None => 0.0,
        };
        self.limit_frac += (target_limit - self.limit_frac) * alpha;
    }
}

fn lerp_opt(cur: Option<f32>, target: Option<f32>, alpha: f32) -> Option<f32> {
    match (cur, target) {
        (Some(c), Some(t)) => Some(c + (t - c) * alpha),
        (None, Some(t)) => Some(t),
        (_, None) => None,
    }
}

pub fn draw_gpu_card(
    ui: &mut Ui,
    theme: &ThemeSpec,
    gpu: &GpuSnapshot,
    anim: &AnimatedGpuValues,
    use_fahrenheit: bool,
    time_sec: f64,
    card_width: f32,
) -> bool {
    let mut reset_clicked = false;
    let border_col = match gpu.health_badge {
        HealthBadge::Healthy => {
            if theme.glow_intensity > 0.2 {
                with_alpha(theme.accent_primary, 75)
            } else {
                theme.border_card
            }
        }
        HealthBadge::Warm => with_alpha(theme.status_warm, 145),
        HealthBadge::Critical | HealthBadge::Throttling => with_alpha(theme.status_critical, 175),
    };
    let title_col = if theme.gloss_sheen {
        match gpu.health_badge {
            HealthBadge::Healthy => Color32::from_rgb(8, 30, 52),
            HealthBadge::Warm => Color32::from_rgb(180, 83, 9),
            HealthBadge::Critical | HealthBadge::Throttling => Color32::from_rgb(185, 28, 28),
        }
    } else {
        match gpu.health_badge {
            HealthBadge::Healthy => {
                if theme.is_dark {
                    theme.accent_secondary
                } else {
                    theme.accent_primary
                }
            }
            HealthBadge::Warm => theme.status_warm,
            HealthBadge::Critical | HealthBadge::Throttling => theme.status_critical,
        }
    };

    let card_cr = CornerRadius::same(theme.card_radius);
    let inner_cr = CornerRadius::same(theme.inner_radius);
    let shadow = if theme.gloss_sheen {
        Shadow {
            offset: [0, 5],
            blur: 18,
            spread: 0,
            color: Color32::from_rgba_unmultiplied(4, 24, 52, 42),
        }
    } else if theme.card_shadow {
        Shadow {
            offset: [0, 4],
            blur: 14,
            spread: 0,
            color: Color32::from_rgba_unmultiplied(15, 23, 42, if theme.is_dark { 60 } else { 22 }),
        }
    } else {
        Shadow::NONE
    };

    ui.allocate_ui_with_layout(
        Vec2::new(card_width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            let mut chrome_idx = None;
            let mut header_bottom = 0.0_f32;
            let frame_resp = Frame::new()
                .fill(theme.bg_card)
                .stroke(Stroke::new(1.2_f32, border_col))
                .corner_radius(card_cr)
                .shadow(shadow)
                .inner_margin(Margin::same(14))
                .show(ui, |ui| {
                    if theme.gloss_sheen {
                        chrome_idx = Some(ui.painter().add(Shape::Noop));
                    }
                    let inner_w = ui.available_width();

                    // 1. Header Row: GPU Title + Health Badge
                    let header_resp = ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_max_width((inner_w - 200.0).max(160.0));
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 7.0;
                                if theme.gloss_sheen {
                                    let orb_col = match gpu.health_badge {
                                        HealthBadge::Healthy => theme.accent_primary,
                                        HealthBadge::Warm => theme.status_warm,
                                        HealthBadge::Critical | HealthBadge::Throttling => {
                                            theme.status_critical
                                        }
                                    };
                                    let (orb_r, _) = ui
                                        .allocate_exact_size(Vec2::new(12.0, 18.0), Sense::hover());
                                    paint_aero_orb(ui.painter(), orb_r.center(), 5.0, orb_col);
                                }
                                ui.label(
                                    egui::RichText::new(format!("GPU {}: {}", gpu.index, gpu.name))
                                        .font(theme.font_title(15.5))
                                        .color(title_col),
                                );
                            });
                            let mut meta_parts = Vec::new();
                            if let Some(ref arch) = gpu.architecture {
                                meta_parts.push(arch.clone());
                            }
                            meta_parts.push(gpu.guidelines.vram_class.clone());
                            if !gpu.pci_bus_id.is_empty() {
                                meta_parts.push(format!("PCI {}", gpu.pci_bus_id));
                            }
                            ui.label(
                                egui::RichText::new(meta_parts.join("  •  "))
                                    .font(theme.font_body(10.5))
                                    .color(theme.text_muted),
                            );
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            draw_status_badge(
                                ui,
                                theme,
                                gpu.health_badge,
                                &gpu.health_subtitle,
                                time_sec,
                            );
                        });
                    });
                    header_bottom = header_resp.response.rect.bottom();

                    ui.add_space(12.0);

                    // 2. Tri-Gauge Thermal Cluster + Guidelines Strip
                    let therm_bg_idx = if theme.gloss_sheen {
                        Some(ui.painter().add(Shape::Noop))
                    } else {
                        None
                    };
                    let therm_frame = Frame::new()
                        .fill(if theme.gloss_sheen {
                            Color32::TRANSPARENT
                        } else {
                            theme.bg_inner
                        })
                        .stroke(if theme.gloss_sheen {
                            Stroke::NONE
                        } else {
                            Stroke::new(1.0_f32, theme.border_subtle)
                        })
                        .corner_radius(inner_cr)
                        .inner_margin(Margin::symmetric(12, 10))
                        .show(ui, |ui| {
                            let panel_w = ui.available_width();
                            let gauge_size = ((panel_w - 36.0) / 3.0).clamp(96.0, 142.0);
                            let gap = ((panel_w - gauge_size * 3.0) / 2.0).max(6.0);

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = gap;
                                draw_arc_gauge(
                                    ui,
                                    theme,
                                    gauge_size,
                                    "CORE TEMP",
                                    gpu.thermals.core,
                                    anim.core,
                                    use_fahrenheit,
                                    &gpu.guidelines.core,
                                    Some("Core temperature sensor unavailable"),
                                    gpu.thermals.peak_core,
                                );
                                draw_arc_gauge(
                                    ui,
                                    theme,
                                    gauge_size,
                                    "HOTSPOT",
                                    gpu.thermals.hotspot,
                                    anim.hotspot,
                                    use_fahrenheit,
                                    &gpu.guidelines.hotspot,
                                    gpu.thermals.hotspot_note.as_deref(),
                                    gpu.thermals.peak_hotspot,
                                );
                                draw_arc_gauge(
                                    ui,
                                    theme,
                                    gauge_size,
                                    "MEMORY",
                                    gpu.thermals.memory,
                                    anim.memory,
                                    use_fahrenheit,
                                    &gpu.guidelines.memory,
                                    gpu.thermals.memory_note.as_deref(),
                                    gpu.thermals.peak_memory,
                                );
                            });

                            ui.add_space(4.0);
                            let (div_rect, _) =
                                ui.allocate_exact_size(Vec2::new(panel_w, 1.0), Sense::hover());
                            ui.painter().line_segment(
                                [
                                    Pos2::new(div_rect.left(), div_rect.center().y),
                                    Pos2::new(div_rect.right(), div_rect.center().y),
                                ],
                                Stroke::new(
                                    1.0_f32,
                                    with_alpha(
                                        theme.border_subtle,
                                        if theme.is_dark { 170 } else { 150 },
                                    ),
                                ),
                            );
                            ui.add_space(5.0);

                            let pill_h = 20.0_f32;
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;

                                if let Some(delta_c) = gpu.thermals.delta_hotspot_core {
                                    let delta_disp = if use_fahrenheit {
                                        delta_c * 9.0 / 5.0
                                    } else {
                                        delta_c
                                    };
                                    let d_col = if delta_c > gpu.guidelines.hotspot_delta_warn {
                                        theme.status_warm
                                    } else {
                                        theme.accent_primary
                                    };
                                    let delta_txt = format!(
                                        "ΔT +{:.0}{}",
                                        delta_disp,
                                        temp_unit(use_fahrenheit)
                                    );
                                    let d_galley = ui.painter().layout_no_wrap(
                                        delta_txt.clone(),
                                        theme.font_value(10.0),
                                        d_col,
                                    );
                                    let d_w = (d_galley.size().x + 14.0).max(66.0);
                                    let (d_rect, d_resp) =
                                        ui.allocate_exact_size(Vec2::new(d_w, pill_h), Sense::hover());
                                    let d_cr = CornerRadius::same(theme.badge_radius.min(6));
                                    ui.painter().rect_filled(
                                        d_rect,
                                        d_cr,
                                        with_alpha(d_col, if theme.is_dark { 24 } else { 28 }),
                                    );
                                    ui.painter().rect_stroke(
                                        d_rect,
                                        d_cr,
                                        Stroke::new(0.9_f32, with_alpha(d_col, 130)),
                                        StrokeKind::Inside,
                                    );
                                    ui.painter().text(
                                        d_rect.center(),
                                        Align2::CENTER_CENTER,
                                        delta_txt,
                                        theme.font_value(10.0),
                                        d_col,
                                    );
                                    d_resp.on_hover_text(
                                        "Hotspot - Core delta. Under heavy load, <= 15°C is ideal; > 18°C can indicate uneven thermal paste coverage.",
                                    );
                                }

                                let spec_txt = format!(
                                    "Spec {}",
                                    format_temp(gpu.guidelines.core.max_spec, use_fahrenheit)
                                );
                                let s_galley = ui.painter().layout_no_wrap(
                                    spec_txt.clone(),
                                    theme.font_ui(10.0),
                                    theme.text_secondary,
                                );
                                let s_w = (s_galley.size().x + 14.0).max(68.0);
                                let (s_rect, s_resp) =
                                    ui.allocate_exact_size(Vec2::new(s_w, pill_h), Sense::hover());
                                let s_cr = CornerRadius::same(theme.badge_radius.min(6));
                                let (s_fill, s_border, s_text_col) = if theme.gloss_sheen {
                                    (
                                        if s_resp.hovered() {
                                            Color32::from_rgba_unmultiplied(255, 255, 255, 245)
                                        } else {
                                            Color32::from_rgba_unmultiplied(245, 251, 255, 225)
                                        },
                                        if s_resp.hovered() {
                                            theme.header_btn_border
                                        } else {
                                            theme.border_subtle
                                        },
                                        if s_resp.hovered() {
                                            theme.text_primary
                                        } else {
                                            theme.text_secondary
                                        },
                                    )
                                } else {
                                    (
                                        with_alpha(
                                            theme.bg_card,
                                            if s_resp.hovered() { 245 } else { 215 },
                                        ),
                                        if s_resp.hovered() {
                                            theme.border_card
                                        } else {
                                            theme.border_subtle
                                        },
                                        if s_resp.hovered() {
                                            theme.text_primary
                                        } else {
                                            theme.text_secondary
                                        },
                                    )
                                };
                                ui.painter().rect_filled(s_rect, s_cr, s_fill);
                                ui.painter().rect_stroke(
                                    s_rect,
                                    s_cr,
                                    Stroke::new(0.9_f32, s_border),
                                    StrokeKind::Inside,
                                );
                                ui.painter().text(
                                    s_rect.center(),
                                    Align2::CENTER_CENTER,
                                    spec_txt,
                                    theme.font_ui(10.0),
                                    s_text_col,
                                );
                                s_resp.on_hover_ui(|ui| {
                                    ui.set_min_width(225.0);
                                    ui.label(
                                        egui::RichText::new("THERMAL THRESHOLDS")
                                            .font(theme.font_title(12.5))
                                            .color(theme.accent_primary),
                                    );
                                    ui.add_space(4.0);
                                    ui.label(format!(
                                        "Core: Safe <{}  •  Max Spec {}",
                                        format_temp(gpu.guidelines.core.healthy_max, use_fahrenheit),
                                        format_temp(gpu.guidelines.core.max_spec, use_fahrenheit),
                                    ));
                                    ui.label(format!(
                                        "Hotspot: Safe <{}  •  Max Spec {}",
                                        format_temp(
                                            gpu.guidelines.hotspot.healthy_max,
                                            use_fahrenheit
                                        ),
                                        format_temp(gpu.guidelines.hotspot.max_spec, use_fahrenheit),
                                    ));
                                    ui.label(format!(
                                        "Memory ({}): Safe <{}  •  Max Spec {}",
                                        gpu.guidelines.vram_class,
                                        format_temp(
                                            gpu.guidelines.memory.healthy_max,
                                            use_fahrenheit
                                        ),
                                        format_temp(gpu.guidelines.memory.max_spec, use_fahrenheit),
                                    ));

                                    let mut hw_lines = Vec::new();
                                    if let Some(t) = gpu.thermals.hw_target_temp {
                                        hw_lines.push(format!(
                                            "Target: {}",
                                            format_temp(t, use_fahrenheit)
                                        ));
                                    }
                                    if let Some(t) = gpu.thermals.hw_max_operating_temp {
                                        hw_lines.push(format!(
                                            "Max Operating: {}",
                                            format_temp(t, use_fahrenheit)
                                        ));
                                    }
                                    if let Some(t) = gpu.thermals.hw_slowdown_temp {
                                        hw_lines.push(format!(
                                            "Slowdown: {}",
                                            format_temp(t, use_fahrenheit)
                                        ));
                                    }
                                    if let Some(t) = gpu.thermals.hw_shutdown_temp {
                                        hw_lines.push(format!(
                                            "Shutdown: {}",
                                            format_temp(t, use_fahrenheit)
                                        ));
                                    }
                                    if let Some(t) = gpu.thermals.hw_memory_max_temp {
                                        hw_lines.push(format!(
                                            "Memory Max: {}",
                                            format_temp(t, use_fahrenheit)
                                        ));
                                    }
                                    if !hw_lines.is_empty() {
                                        ui.add_space(4.0);
                                        ui.separator();
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Driver Limits: {}",
                                                hw_lines.join("  •  ")
                                            ))
                                            .font(theme.font_body(10.5))
                                            .color(theme.text_secondary),
                                        );
                                        ui.label(
                                            egui::RichText::new("Source: Live GPU driver limits")
                                                .font(theme.font_body(10.0))
                                                .color(theme.text_muted),
                                        );
                                    } else {
                                        ui.add_space(4.0);
                                        ui.label(
                                            egui::RichText::new(
                                                "Source: Architecture reference defaults",
                                            )
                                            .font(theme.font_body(10.0))
                                            .color(theme.text_muted),
                                        );
                                    }
                                });

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let (r_rect, r_resp) = ui.allocate_exact_size(
                                            Vec2::new(62.0, pill_h),
                                            Sense::click(),
                                        );
                                        let r_rad = theme.badge_radius.min(6);
                                        let r_cr = CornerRadius::same(r_rad);
                                        let r_col = if theme.gloss_sheen {
                                            let (top_c, bot_c, bdr_c) = if r_resp.hovered() {
                                                (
                                                    Color32::from_rgb(11, 122, 209),
                                                    Color32::from_rgb(8, 98, 173),
                                                    Color32::from_rgb(7, 80, 143),
                                                )
                                            } else {
                                                (
                                                    Color32::from_rgb(255, 255, 255),
                                                    Color32::from_rgb(228, 243, 253),
                                                    theme.header_btn_border,
                                                )
                                            };
                                            paint_aero_gel_pill(
                                                ui.painter(),
                                                r_rect,
                                                r_rad,
                                                top_c,
                                                bot_c,
                                                bdr_c,
                                            );
                                            if r_resp.hovered() {
                                                Color32::WHITE
                                            } else {
                                                Color32::from_rgb(30, 68, 102)
                                            }
                                        } else {
                                            let r_bg = if r_resp.hovered() {
                                                with_alpha(
                                                    theme.accent_primary,
                                                    if theme.is_dark { 36 } else { 32 },
                                                )
                                            } else {
                                                theme.bg_card
                                            };
                                            let r_border = if r_resp.hovered() {
                                                theme.accent_primary
                                            } else {
                                                theme.border_card
                                            };
                                            ui.painter().rect_filled(r_rect, r_cr, r_bg);
                                            ui.painter().rect_stroke(
                                                r_rect,
                                                r_cr,
                                                Stroke::new(0.9_f32, r_border),
                                                StrokeKind::Inside,
                                            );
                                            if r_resp.hovered() {
                                                theme.accent_primary
                                            } else {
                                                theme.text_secondary
                                            }
                                        };
                                        paint_reset_icon(
                                            ui.painter(),
                                            Pos2::new(r_rect.left() + 12.0, r_rect.center().y),
                                            4.0,
                                            r_col,
                                        );
                                        ui.painter().text(
                                            Pos2::new(r_rect.left() + 20.0, r_rect.center().y),
                                            Align2::LEFT_CENTER,
                                            "Reset",
                                            theme.font_ui(10.0),
                                            if theme.gloss_sheen {
                                                r_col
                                            } else if r_resp.hovered() {
                                                theme.text_primary
                                            } else {
                                                theme.text_secondary
                                            },
                                        );
                                        let r_resp = r_resp.on_hover_text(format!(
                                            "Reset recorded max temperatures for GPU {}",
                                            gpu.index
                                        ));
                                        if r_resp.clicked() {
                                            reset_clicked = true;
                                        }
                                    },
                                );
                            });
                        });
                    if let Some(idx) = therm_bg_idx {
                        ui.painter().set(
                            idx,
                            Shape::Vec(build_aero_recessed_well_shapes(
                                therm_frame.response.rect,
                                theme.inner_radius,
                                theme.bg_inner,
                                theme.border_subtle,
                            )),
                        );
                    }

                    ui.add_space(10.0);

                    // 3. Power & Thermal Throttling Panel
                    let power_bg_idx = if theme.gloss_sheen {
                        Some(ui.painter().add(Shape::Noop))
                    } else {
                        None
                    };
                    let power_frame = Frame::new()
                        .fill(if theme.gloss_sheen {
                            Color32::TRANSPARENT
                        } else {
                            theme.bg_inner
                        })
                        .stroke(if theme.gloss_sheen {
                            Stroke::NONE
                        } else {
                            Stroke::new(1.0_f32, theme.border_subtle)
                        })
                        .corner_radius(inner_cr)
                        .inner_margin(Margin::symmetric(12, 11))
                        .show(ui, |ui| {
                            let panel_w = ui.available_width();
                            let col_gap = 18.0;
                            let col_w = ((panel_w - col_gap) * 0.5).max(160.0);

                            ui.horizontal_top(|ui| {
                                ui.spacing_mut().item_spacing.x = col_gap;

                                // Left Column: Live Power Draw / TDP + Clocks/Util
                                ui.vertical(|ui| {
                                    ui.set_width(col_w);

                                    let active_lim = gpu
                                        .power
                                        .enforced_limit_w
                                        .or(gpu.power.active_limit_w)
                                        .or(gpu.power.default_limit_w);

                                    let power_text = match (gpu.power.draw_w, active_lim) {
                                        (Some(d), Some(l)) => {
                                            format!("Power: {:.0}W / {:.0}W TDP", d, l)
                                        }
                                        (Some(d), None) => format!("Power: {:.0}W", d),
                                        _ => "Power: Unavailable".to_string(),
                                    };
                                    let p_col = if anim.power_frac > 0.94 {
                                        theme.status_warm
                                    } else {
                                        theme.accent_primary
                                    };

                                    draw_icon_text_row(ui, theme, p_col, &power_text, |p, c, col| {
                                        paint_lightning_icon(p, c, col);
                                    });
                                    ui.add_space(4.0);
                                    draw_glow_bar(
                                        ui,
                                        theme,
                                        col_w,
                                        8.0,
                                        anim.power_frac,
                                        theme.gauge_healthy.0,
                                        if anim.power_frac > 0.94 {
                                            theme.status_warm
                                        } else {
                                            theme.gauge_healthy.1
                                        },
                                        None,
                                    );

                                    ui.add_space(9.0);

                                    let default_str = match (
                                        gpu.power.default_limit_w,
                                        gpu.power.min_limit_w,
                                        gpu.power.max_limit_w,
                                    ) {
                                        (Some(def), Some(min), Some(max)) => format!(
                                            "Default TDP: {:.0}W  (Range {:.0}–{:.0}W)",
                                            def, min, max
                                        ),
                                        (Some(def), _, _) => format!("Default TDP: {:.0}W", def),
                                        _ => "Default TDP: N/A".to_string(),
                                    };
                                    ui.label(
                                        egui::RichText::new(default_str)
                                            .font(theme.font_body(11.5))
                                            .color(theme.text_secondary),
                                    );

                                    ui.add_space(3.0);

                                    let mut clk_parts = Vec::new();
                                    if let Some(clk) = gpu.clocks.graphics_mhz {
                                        clk_parts.push(format!("Core {clk} MHz"));
                                    }
                                    if let Some(u) = gpu.clocks.gpu_util_pct {
                                        clk_parts.push(format!("Load {u}%"));
                                    }
                                    if let Some(ref ps) = gpu.clocks.pstate {
                                        clk_parts.push(ps.clone());
                                    }
                                    let clk_line = if clk_parts.is_empty() {
                                        "Clocks: N/A".to_string()
                                    } else {
                                        clk_parts.join("  •  ")
                                    };
                                    ui.label(
                                        egui::RichText::new(clk_line)
                                            .font(theme.font_body(10.5))
                                            .color(theme.text_muted),
                                    )
                                    .on_hover_ui(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} TELEMETRY DETAILS",
                                                gpu.vendor.label().to_ascii_uppercase()
                                            ))
                                            .font(theme.font_title(12.0))
                                            .color(theme.accent_primary),
                                        );
                                        if let Some(m) = gpu.clocks.max_graphics_mhz {
                                            ui.label(format!("Max Boost Clock: {m} MHz"));
                                        }
                                        if let Some(m) = gpu.clocks.memory_mhz {
                                            ui.label(format!("Memory Clock: {m} MHz"));
                                        }
                                        if let Some(mu) = gpu.clocks.mem_util_pct {
                                            ui.label(format!("Memory Controller Load: {mu}%"));
                                        }
                                        if let Some(sd) = gpu.thermals.hw_shutdown_temp {
                                            ui.label(format!(
                                                "HW Shutdown Threshold: {}",
                                                format_temp(sd, use_fahrenheit)
                                            ));
                                        }
                                        if !gpu.fans.per_fan_pct.is_empty() {
                                            let fan_strs: Vec<String> = gpu
                                                .fans
                                                .per_fan_pct
                                                .iter()
                                                .enumerate()
                                                .map(|(i, pct)| {
                                                    match gpu
                                                        .fans
                                                        .per_fan_rpm
                                                        .get(i)
                                                        .copied()
                                                        .flatten()
                                                    {
                                                        Some(r) => {
                                                            format!("Fan {}: {pct}% ({r} RPM)", i + 1)
                                                        }
                                                        None => format!("Fan {}: {pct}%", i + 1),
                                                    }
                                                })
                                                .collect();
                                            ui.label(fan_strs.join(" | "));
                                        }
                                    });
                                });

                                // Right Column: Power 100% / Software Limit + Thermal Throttling + Software Cap
                                ui.vertical(|ui| {
                                    ui.set_width(col_w);

                                    let (lim_label, lim_col_s, lim_col_e, marker) =
                                        match gpu.power.mode {
                                            PowerLimitMode::Full100Percent => (
                                                "Power: 100% (No Software Limit)".to_string(),
                                                theme.gauge_healthy.0,
                                                theme.gauge_healthy.1,
                                                None,
                                            ),
                                            PowerLimitMode::SoftwareLimited {
                                                pct,
                                                active_w,
                                                default_w,
                                            } => (
                                                format!(
                                                    "Power: {pct:.0}% (SW Limit: {active_w:.0}/{default_w:.0}W)"
                                                ),
                                                theme.gauge_warm.0,
                                                theme.gauge_warm.1,
                                                None,
                                            ),
                                            PowerLimitMode::OverclockedLimit {
                                                pct,
                                                active_w,
                                                default_w,
                                            } => (
                                                format!(
                                                    "Power: {pct:.0}% (Unlocked: {active_w:.0}/{default_w:.0}W)"
                                                ),
                                                theme.accent_primary,
                                                theme.accent_tertiary,
                                                Some((default_w / active_w).clamp(0.1, 0.95)),
                                            ),
                                            PowerLimitMode::Unknown => (
                                                "Power Limit: Unavailable".to_string(),
                                                theme.text_muted,
                                                theme.text_muted,
                                                None,
                                            ),
                                        };

                                    draw_icon_text_row(
                                        ui,
                                        theme,
                                        lim_col_s,
                                        &lim_label,
                                        |p, c, col| {
                                            paint_lightning_icon(p, c, col);
                                        },
                                    );
                                    ui.add_space(4.0);
                                    draw_glow_bar(
                                        ui,
                                        theme,
                                        col_w,
                                        8.0,
                                        if matches!(gpu.power.mode, PowerLimitMode::Unknown) {
                                            0.0
                                        } else {
                                            anim.limit_frac.max(0.05)
                                        },
                                        lim_col_s,
                                        lim_col_e,
                                        marker,
                                    );

                                    ui.add_space(8.0);

                                    let thr_active = gpu.throttle.thermal_throttling;
                                    let thr_col = if thr_active {
                                        theme.status_critical
                                    } else {
                                        theme.status_healthy
                                    };
                                    let thr_text = if thr_active {
                                        let kind = match (
                                            gpu.throttle.hw_thermal,
                                            gpu.throttle.sw_thermal,
                                        ) {
                                            (true, true) => "HW + SW",
                                            (true, false) => "HW",
                                            (false, true) => "SW",
                                            (false, false) => "Active",
                                        };
                                        format!("Thermal Throttling: Active ({kind})")
                                    } else if gpu.throttle.ever_thermal_throttled {
                                        "Thermal Throttling: No (Peak tripped earlier)".to_string()
                                    } else {
                                        "Thermal Throttling: No".to_string()
                                    };

                                    draw_icon_text_row(ui, theme, thr_col, &thr_text, |p, c, col| {
                                        p.circle_filled(c, 4.5, col);
                                    });
                                    ui.add_space(3.0);
                                    draw_segmented_throttle_bar(
                                        ui,
                                        theme,
                                        col_w,
                                        8.0,
                                        thr_active,
                                        time_sec,
                                    );

                                    ui.add_space(6.0);

                                    let has_sw_cap = gpu.throttle.software_cap_summary != "None";
                                    let cap_col = if has_sw_cap {
                                        theme.status_warm
                                    } else {
                                        theme.text_muted
                                    };
                                    let cap_label =
                                        format!("Software Cap: {}", gpu.throttle.software_cap_summary);
                                    let cap_row = draw_icon_text_row(
                                        ui,
                                        theme,
                                        cap_col,
                                        &cap_label,
                                        |p, c, col| {
                                            if has_sw_cap {
                                                p.circle_filled(c, 4.0, col);
                                            } else {
                                                p.circle_stroke(
                                                    c,
                                                    4.0,
                                                    Stroke::new(1.2_f32, col),
                                                );
                                            }
                                        },
                                    );
                                    if !gpu.throttle.active_reasons.is_empty() {
                                        cap_row.on_hover_text(format!(
                                            "Active NVML throttle flags:\n• {}",
                                            gpu.throttle.active_reasons.join("\n• ")
                                        ));
                                    }
                                });
                            });
                        });
                    if let Some(idx) = power_bg_idx {
                        ui.painter().set(
                            idx,
                            Shape::Vec(build_aero_recessed_well_shapes(
                                power_frame.response.rect,
                                theme.inner_radius,
                                theme.bg_inner,
                                theme.border_subtle,
                            )),
                        );
                    }

                    ui.add_space(10.0);

                    // 4. Bottom 2-Row Concept-Grouped Telemetry Charts
                    let chart_gap = 10.0;
                    let chart_w = ((inner_w - chart_gap * 2.0) / 3.0).max(95.0);
                    let chart_h = 74.0;
                    let chart_size = Vec2::new(chart_w, chart_h);

                    // Row 1: Thermals & Cooling
                    draw_chart_group_label(ui, theme, inner_w, "THERMALS & COOLING");
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = chart_gap;

                        // Chart 1: Core / Hotspot Temp History
                        let core_disp_pts = map_display_temps(&gpu.history.core_temp, use_fahrenheit);
                        let hot_disp_pts =
                            map_display_temps(&gpu.history.hotspot_temp, use_fahrenheit);
                        let temp_readout = gpu
                            .thermals
                            .core
                            .map(|c| format_temp(c, use_fahrenheit))
                            .unwrap_or_else(|| "N/A".to_string());
                        let t_col = if gpu.throttle.thermal_throttling {
                            theme.status_critical
                        } else {
                            gpu.thermals
                                .core
                                .map(|c| {
                                    level_accent_color(
                                        evaluate_temp_level(c, &gpu.guidelines.core),
                                        theme,
                                    )
                                })
                                .unwrap_or(theme.text_muted)
                        };
                        let (t_min, t_max) = if use_fahrenheit {
                            (68.0, 220.0)
                        } else {
                            (20.0, 105.0)
                        };
                        draw_sparkline_chart(
                            ui,
                            theme,
                            chart_size,
                            "CORE / HOTSPOT",
                            &temp_readout,
                            &core_disp_pts,
                            t_min,
                            t_max,
                            temp_unit(use_fahrenheit),
                            t_col,
                            (!hot_disp_pts.is_empty()).then_some(&hot_disp_pts),
                            Some(theme.chart_temp_secondary),
                        );

                        // Chart 2: Memory Temp
                        let mem_disp_pts =
                            map_display_temps(&gpu.history.memory_temp, use_fahrenheit);
                        let mem_readout = gpu
                            .thermals
                            .memory
                            .map(|m| format_temp(m, use_fahrenheit))
                            .unwrap_or_else(|| "N/A".to_string());
                        let m_col = gpu
                            .thermals
                            .memory
                            .map(|m| {
                                level_accent_color(
                                    evaluate_temp_level(m, &gpu.guidelines.memory),
                                    theme,
                                )
                            })
                            .unwrap_or(theme.text_muted);
                        let (m_min, m_max) = if use_fahrenheit {
                            (68.0, 230.0)
                        } else {
                            (20.0, 110.0)
                        };
                        draw_sparkline_chart(
                            ui,
                            theme,
                            chart_size,
                            "MEMORY TEMP",
                            &mem_readout,
                            &mem_disp_pts,
                            m_min,
                            m_max,
                            temp_unit(use_fahrenheit),
                            m_col,
                            None,
                            None,
                        );

                        // Chart 3: Fans (% & RPM)
                        let fan_readout = match (gpu.fans.speed_pct, gpu.fans.rpm) {
                            (Some(0), Some(0) | None) => "0% (Zero-RPM)".to_string(),
                            (Some(pct), Some(rpm)) => format!("{pct}% / {rpm} RPM"),
                            (Some(pct), None) => format!("{pct}%"),
                            (None, Some(rpm)) => format!("{rpm} RPM"),
                            (None, None) => "N/A".to_string(),
                        };
                        draw_sparkline_chart(
                            ui,
                            theme,
                            chart_size,
                            "FANS",
                            &fan_readout,
                            &gpu.history.fan_pct,
                            0.0,
                            100.0,
                            "%",
                            if matches!(
                                gpu.health_badge,
                                HealthBadge::Warm | HealthBadge::Critical | HealthBadge::Throttling
                            ) {
                                theme.status_warm
                            } else {
                                theme.accent_secondary
                            },
                            None,
                            None,
                        );
                    });

                    ui.add_space(7.0);

                    // Row 2: Workload & Power
                    draw_chart_group_label(ui, theme, inner_w, "WORKLOAD & POWER");
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = chart_gap;

                        // Chart 4: GPU Usage (%)
                        let util_readout = gpu
                            .clocks
                            .gpu_util_pct
                            .map(|u| format!("{u}%"))
                            .unwrap_or_else(|| "N/A".to_string());
                        let util_col = if gpu.clocks.gpu_util_pct.is_some() {
                            theme.accent_primary
                        } else {
                            theme.text_muted
                        };
                        draw_sparkline_chart(
                            ui,
                            theme,
                            chart_size,
                            "GPU USAGE",
                            &util_readout,
                            &gpu.history.gpu_util_pct,
                            0.0,
                            100.0,
                            "%",
                            util_col,
                            None,
                            None,
                        );

                        // Chart 5: VRAM Usage (GB)
                        let vram_total_gb = gpu.vram.total_gb().max(1.0);
                        let vram_readout = if gpu.vram.total_mb > 0 {
                            format!("{:.1}/{:.0} GB", gpu.vram.used_gb(), vram_total_gb)
                        } else {
                            "N/A".to_string()
                        };
                        let vram_col = if gpu.vram.total_mb > 0 {
                            theme.accent_secondary
                        } else {
                            theme.text_muted
                        };
                        draw_sparkline_chart(
                            ui,
                            theme,
                            chart_size,
                            "VRAM USAGE",
                            &vram_readout,
                            &gpu.history.vram_used_gb,
                            0.0,
                            vram_total_gb,
                            " GB",
                            vram_col,
                            None,
                            None,
                        );

                        // Chart 6: Power Draw (W)
                        let power_limit_w = gpu
                            .power
                            .enforced_limit_w
                            .or(gpu.power.active_limit_w)
                            .or(gpu.power.default_limit_w);
                        let power_max_w = power_limit_w.unwrap_or(250.0).max(50.0);
                        let power_readout = match (gpu.power.draw_w, power_limit_w) {
                            (Some(d), Some(l)) => format!("{d:.0}/{l:.0}W"),
                            (Some(d), None) => format!("{d:.0}W"),
                            (None, _) => "N/A".to_string(),
                        };
                        let power_col = match gpu.power.draw_w {
                            None => theme.text_muted,
                            Some(d) => {
                                let ratio = power_limit_w
                                    .filter(|&l| l > 0.1)
                                    .map(|l| d / l)
                                    .unwrap_or(0.0);
                                if ratio >= 0.92 {
                                    theme.status_warm
                                } else {
                                    theme.accent_primary
                                }
                            }
                        };
                        draw_sparkline_chart(
                            ui,
                            theme,
                            chart_size,
                            "POWER DRAW",
                            &power_readout,
                            &gpu.history.power_w,
                            0.0,
                            power_max_w,
                            "W",
                            power_col,
                            None,
                            None,
                        );
                    });
                });

            if let Some(idx) = chrome_idx {
                let r = frame_resp.response.rect;
                let title_end_y = (header_bottom + 7.0).min(r.top() + 72.0);
                let title_rect = Rect::from_min_max(
                    Pos2::new(r.left() + 1.0, r.top() + 1.0),
                    Pos2::new(r.right() - 1.0, title_end_y),
                );
                let top_r = theme.card_radius.saturating_sub(1);
                let title_cr = CornerRadius {
                    nw: top_r,
                    ne: top_r,
                    sw: 0,
                    se: 0,
                };
                let shapes = vec![
                    Shape::rect_filled(
                        title_rect,
                        title_cr,
                        Color32::from_rgba_unmultiplied(224, 243, 255, 115),
                    ),
                    Shape::line_segment(
                        [
                            Pos2::new(r.left() + 2.0, title_end_y),
                            Pos2::new(r.right() - 2.0, title_end_y),
                        ],
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(136, 186, 222, 95)),
                    ),
                    Shape::rect_stroke(
                        r.shrink(1.0),
                        CornerRadius::same(top_r),
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 215)),
                        StrokeKind::Inside,
                    ),
                ];
                ui.painter().set(idx, Shape::Vec(shapes));
            }
        },
    );
    reset_clicked
}

fn draw_icon_text_row<F>(
    ui: &mut Ui,
    theme: &ThemeSpec,
    accent: Color32,
    text: &str,
    paint_icon: F,
) -> egui::Response
where
    F: FnOnce(&egui::Painter, Pos2, Color32),
{
    let avail = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(avail, 18.0), Sense::hover());
    let painter = ui.painter();
    let icon_center = Pos2::new(rect.left() + 7.0, rect.center().y);
    paint_icon(painter, icon_center, accent);
    painter.text(
        Pos2::new(rect.left() + 18.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        theme.font_value(12.0),
        if accent == theme.text_muted {
            theme.text_secondary
        } else {
            theme.text_primary
        },
    );
    resp
}

fn draw_chart_group_label(ui: &mut Ui, theme: &ThemeSpec, width: f32, label: &str) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 13.0), Sense::hover());
    let painter = ui.painter();
    let galley = painter.layout_no_wrap(
        label.to_string(),
        theme.font_title_semi(10.0),
        theme.text_muted,
    );
    let label_w = galley.size().x;
    let label_h = galley.size().y;
    painter.galley(
        Pos2::new(rect.left() + 2.0, rect.center().y - label_h * 0.5),
        galley,
        theme.text_muted,
    );
    let line_x0 = (rect.left() + 2.0 + label_w + 8.0).min(rect.right() - 2.0);
    if rect.right() - 2.0 > line_x0 {
        painter.line_segment(
            [
                Pos2::new(line_x0, rect.center().y),
                Pos2::new(rect.right() - 2.0, rect.center().y),
            ],
            Stroke::new(
                1.0_f32,
                with_alpha(
                    theme.border_subtle,
                    if theme.is_dark { 150 } else { 130 },
                ),
            ),
        );
    }
}

fn map_display_temps(pts: &VecDeque<f32>, use_fahrenheit: bool) -> VecDeque<f32> {
    pts.iter().map(|&c| display_temp(c, use_fahrenheit)).collect()
}

fn level_accent_color(level: MetricLevel, theme: &ThemeSpec) -> Color32 {
    match level {
        MetricLevel::Healthy => theme.accent_primary,
        MetricLevel::Warm => theme.status_warm,
        MetricLevel::Critical => theme.status_critical,
    }
}
