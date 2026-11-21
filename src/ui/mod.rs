pub mod gpu_card;
pub mod theme;
pub mod widgets;

use std::collections::HashMap;
use std::time::Duration;

use egui::{
    Align2, Area, Color32, CornerRadius, Frame, Id, Key, Margin, Order, Pos2, Rect, Sense, Shadow,
    Stroke, StrokeKind, Vec2, Window,
};

use gpu_card::{draw_gpu_card, AnimatedGpuValues};
use theme::{
    apply_theme_visuals, lerp_color, setup_fonts, with_alpha, ThemeId, ThemeSpec,
};
use widgets::{draw_header_bar, paint_ambient_backdrop};

use crate::config::AppConfig;
use crate::telemetry::TelemetryManager;

pub struct GpuDeckApp {
    telemetry: TelemetryManager,
    config: AppConfig,
    theme: ThemeSpec,
    show_theme_menu: bool,
    theme_btn_rect: Option<Rect>,
    show_settings: bool,
    show_info: bool,
    animated: HashMap<String, AnimatedGpuValues>,
    last_time: f64,
}

impl GpuDeckApp {
    pub fn new(cc: &eframe::CreationContext<'_>, config: AppConfig) -> Self {
        setup_fonts(&cc.egui_ctx);
        let theme = ThemeSpec::for_id(config.theme);
        apply_theme_visuals(&cc.egui_ctx, &theme);

        let telemetry =
            TelemetryManager::start(config.poll_interval_ms, Some(cc.egui_ctx.clone()));
        Self {
            telemetry,
            config,
            theme,
            show_theme_menu: false,
            theme_btn_rect: None,
            show_settings: false,
            show_info: false,
            animated: HashMap::new(),
            last_time: 0.0,
        }
    }

    fn set_theme(&mut self, ctx: &egui::Context, id: ThemeId) {
        self.config.theme = id;
        self.theme = ThemeSpec::for_id(id);
        apply_theme_visuals(ctx, &self.theme);
        self.config.save();
    }

    fn render_theme_popover(&mut self, ctx: &egui::Context) {
        let Some(anchor) = self.theme_btn_rect else {
            self.show_theme_menu = false;
            return;
        };
        if anchor == Rect::NOTHING {
            self.show_theme_menu = false;
            return;
        }

        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.show_theme_menu = false;
            return;
        }

        let popup_w = 216.0;
        let screen = ctx.screen_rect();
        let pos_x = (anchor.right() - popup_w).clamp(screen.left() + 8.0, screen.right() - popup_w - 8.0);
        let pos_y = (anchor.bottom() + 6.0).min(screen.bottom() - 320.0);

        let area_resp = Area::new(Id::new("theme_selector_popup"))
            .order(Order::Foreground)
            .fixed_pos(Pos2::new(pos_x, pos_y))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(self.theme.bg_card)
                    .stroke(Stroke::new(1.2_f32, self.theme.border_card))
                    .corner_radius(CornerRadius::same(self.theme.card_radius.min(12)))
                    .shadow(Shadow {
                        offset: [0, 6],
                        blur: 18,
                        spread: 0,
                        color: Color32::from_rgba_unmultiplied(0, 0, 0, 110),
                    })
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_width(popup_w - 16.0);
                        ui.label(
                            egui::RichText::new("SELECT THEME")
                                .font(self.theme.font_title_semi(10.5))
                                .color(self.theme.text_muted),
                        );
                        ui.add_space(4.0);

                        let mut chosen: Option<ThemeId> = None;
                        for id in ThemeId::ALL {
                            let selected = id == self.config.theme;
                            let swatches = id.swatch_colors();
                            let row_w = ui.available_width();
                            let (r, resp) =
                                ui.allocate_exact_size(Vec2::new(row_w, 28.0), Sense::click());
                            let p = ui.painter();
                            let cr = CornerRadius::same(self.theme.badge_radius.min(8));

                            if selected {
                                p.rect_filled(r, cr, with_alpha(self.theme.accent_primary, 36));
                                p.rect_stroke(
                                    r,
                                    cr,
                                    Stroke::new(1.0_f32, self.theme.accent_primary),
                                    StrokeKind::Inside,
                                );
                            } else if resp.hovered() {
                                p.rect_filled(r, cr, with_alpha(self.theme.accent_primary, 18));
                            }

                            // 3-dot palette preview
                            for (i, col) in swatches.iter().enumerate() {
                                let cx = r.left() + 12.0 + i as f32 * 9.5;
                                let c = Pos2::new(cx, r.center().y);
                                p.circle_filled(c, 4.0, *col);
                                p.circle_stroke(
                                    c,
                                    4.0,
                                    Stroke::new(
                                        0.7_f32,
                                        with_alpha(self.theme.text_primary, 110),
                                    ),
                                );
                            }

                            p.text(
                                Pos2::new(r.left() + 44.0, r.center().y),
                                Align2::LEFT_CENTER,
                                id.display_name(),
                                if selected {
                                    self.theme.font_value(12.0)
                                } else {
                                    self.theme.font_ui(12.0)
                                },
                                self.theme.text_primary,
                            );

                            if selected {
                                p.circle_filled(
                                    Pos2::new(r.right() - 12.0, r.center().y),
                                    3.5,
                                    self.theme.accent_primary,
                                );
                            }

                            if resp.clicked() {
                                chosen = Some(id);
                            }
                        }

                        if let Some(id) = chosen {
                            self.set_theme(ctx, id);
                            self.show_theme_menu = false;
                        }
                    });
            });

        // Close if user clicked anywhere outside the popover and outside the toggle button
        let clicked_outside = ctx.input(|i| {
            if i.pointer.any_pressed() {
                if let Some(pos) = i.pointer.interact_pos() {
                    return !area_resp.response.rect.contains(pos) && !anchor.contains(pos);
                }
            }
            false
        });
        if clicked_outside {
            self.show_theme_menu = false;
        }
    }

    fn render_settings_window(&mut self, ctx: &egui::Context) {
        if draw_modal_scrim(ctx, &self.theme, Id::new("settings_modal_scrim")) {
            self.show_settings = false;
            return;
        }

        let mut close_clicked = false;
        let mut pending_theme: Option<ThemeId> = None;

        Window::new("settings_modal")
            .title_bar(false)
            .order(Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .frame(modal_frame(&self.theme))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .auto_sized()
            .show(ctx, |ui| {
                ui.set_width(464.0);
                if draw_modal_header(
                    ui,
                    &self.theme,
                    "SETTINGS",
                    "Appearance, temperature units, and sensor polling",
                ) {
                    close_clicked = true;
                }

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("VISUAL THEME")
                            .font(self.theme.font_title_semi(10.5))
                            .color(self.theme.text_muted),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(self.config.theme.display_name())
                                .font(self.theme.font_value(11.0))
                                .color(self.theme.accent_primary),
                        );
                    });
                });
                ui.add_space(6.0);

                let avail_w = ui.available_width();
                let gap = 9.0;
                let cell_w = ((avail_w - gap * 2.0) / 3.0).floor();

                for row in ThemeId::ALL.chunks(3) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for &id in row {
                            let selected = id == self.config.theme;
                            let preview = ThemeSpec::for_id(id);
                            let (r, resp) =
                                ui.allocate_exact_size(Vec2::new(cell_w, 60.0), Sense::click());
                            let p = ui.painter();
                            let cr = CornerRadius::same(self.theme.inner_radius.min(10));

                            let bg = if selected {
                                with_alpha(
                                    self.theme.accent_primary,
                                    if self.theme.is_dark { 22 } else { 26 },
                                )
                            } else if resp.hovered() {
                                with_alpha(self.theme.accent_primary, 10)
                            } else {
                                self.theme.bg_inner
                            };
                            let stroke_col = if selected {
                                self.theme.accent_primary
                            } else if resp.hovered() {
                                self.theme.border_card
                            } else {
                                self.theme.border_subtle
                            };

                            p.rect_filled(r, cr, bg);
                            p.rect_stroke(
                                r,
                                cr,
                                Stroke::new(if selected { 1.5_f32 } else { 1.0_f32 }, stroke_col),
                                StrokeKind::Inside,
                            );

                            // Miniature UI theme preview strip
                            let prev_rect = Rect::from_min_max(
                                Pos2::new(r.left() + 6.0, r.top() + 6.0),
                                Pos2::new(r.right() - 6.0, r.top() + 33.0),
                            );
                            let prev_cr = CornerRadius::same(preview.inner_radius.min(6));
                            p.rect_filled(prev_rect, prev_cr, preview.bg_app);
                            p.rect_stroke(
                                prev_rect,
                                prev_cr,
                                Stroke::new(0.8_f32, preview.border_card),
                                StrokeKind::Inside,
                            );
                            let mini_card = prev_rect.shrink(3.5);
                            let mini_cr = CornerRadius::same(preview.badge_radius.min(4));
                            p.rect_filled(mini_card, mini_cr, preview.bg_card);
                            let dot_pos = Pos2::new(mini_card.left() + 6.5, mini_card.center().y);
                            p.circle_filled(dot_pos, 2.6, preview.status_healthy);
                            let bar_bg = Rect::from_min_max(
                                Pos2::new(mini_card.left() + 13.0, mini_card.center().y - 2.0),
                                Pos2::new(mini_card.right() - 5.0, mini_card.center().y + 2.0),
                            );
                            p.rect_filled(bar_bg, CornerRadius::same(2), preview.bg_track);
                            let bar_fill = Rect::from_min_max(
                                bar_bg.min,
                                Pos2::new(
                                    bar_bg.left() + bar_bg.width() * 0.64,
                                    bar_bg.bottom(),
                                ),
                            );
                            p.rect_filled(bar_fill, CornerRadius::same(2), preview.accent_primary);

                            p.text(
                                Pos2::new(r.left() + 8.0, r.bottom() - 12.0),
                                Align2::LEFT_CENTER,
                                id.display_name(),
                                if selected {
                                    self.theme.font_value(11.5)
                                } else {
                                    self.theme.font_ui(11.5)
                                },
                                self.theme.text_primary,
                            );

                            if selected {
                                let chk = Pos2::new(r.right() - 13.0, r.bottom() - 12.0);
                                p.circle_filled(chk, 6.2, self.theme.accent_primary);
                                let check_col = if self.theme.is_dark {
                                    self.theme.bg_app
                                } else {
                                    Color32::WHITE
                                };
                                p.line_segment(
                                    [
                                        Pos2::new(chk.x - 2.6, chk.y + 0.1),
                                        Pos2::new(chk.x - 0.6, chk.y + 2.1),
                                    ],
                                    Stroke::new(1.5_f32, check_col),
                                );
                                p.line_segment(
                                    [
                                        Pos2::new(chk.x - 0.6, chk.y + 2.1),
                                        Pos2::new(chk.x + 2.8, chk.y - 1.9),
                                    ],
                                    Stroke::new(1.5_f32, check_col),
                                );
                            }

                            if resp.clicked() {
                                pending_theme = Some(id);
                            }
                        }
                    });
                    ui.add_space(6.0);
                }

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("TELEMETRY & DISPLAY")
                        .font(self.theme.font_title_semi(10.5))
                        .color(self.theme.text_muted),
                );
                ui.add_space(6.0);

                Frame::new()
                    .fill(self.theme.bg_inner)
                    .stroke(Stroke::new(1.0_f32, self.theme.border_subtle))
                    .corner_radius(CornerRadius::same(self.theme.inner_radius.min(10)))
                    .inner_margin(Margin::symmetric(14, 12))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("Temperature Unit")
                                        .font(self.theme.font_value(12.5))
                                        .color(self.theme.text_primary),
                                );
                                ui.label(
                                    egui::RichText::new("Display scale across dials and charts")
                                        .font(self.theme.font_body(10.5))
                                        .color(self.theme.text_muted),
                                );
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let active_u = usize::from(self.config.use_fahrenheit);
                                if let Some(sel) = draw_segmented_control(
                                    ui,
                                    &self.theme,
                                    &["°C Celsius", "°F Fahrenheit"],
                                    active_u,
                                    178.0,
                                ) {
                                    self.config.use_fahrenheit = sel == 1;
                                    self.config.save();
                                }
                            });
                        });

                        ui.add_space(8.0);
                        let (div_r, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 1.0),
                            Sense::hover(),
                        );
                        ui.painter().line_segment(
                            [
                                Pos2::new(div_r.left(), div_r.center().y),
                                Pos2::new(div_r.right(), div_r.center().y),
                            ],
                            Stroke::new(
                                1.0_f32,
                                with_alpha(self.theme.border_subtle, 170),
                            ),
                        );
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    egui::RichText::new("Polling Interval")
                                        .font(self.theme.font_value(12.5))
                                        .color(self.theme.text_primary),
                                );
                                ui.label(
                                    egui::RichText::new("How often GPU sensors refresh")
                                        .font(self.theme.font_body(10.5))
                                        .color(self.theme.text_muted),
                                );
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let rates = [500u64, 1000, 2000];
                                let active_r = rates
                                    .iter()
                                    .position(|&ms| ms == self.config.poll_interval_ms)
                                    .unwrap_or(1);
                                if let Some(sel) = draw_segmented_control(
                                    ui,
                                    &self.theme,
                                    &["500 ms", "1 s", "2 s"],
                                    active_r,
                                    178.0,
                                ) {
                                    let ms = rates[sel];
                                    self.config.poll_interval_ms = ms;
                                    self.telemetry.set_poll_interval_ms(ms);
                                    self.config.save();
                                }
                            });
                        });
                    });

                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Read-only · Never alters clocks or fan curves.")
                            .font(self.theme.font_body(10.5))
                            .color(self.theme.text_muted),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if draw_modal_action_button(
                            ui,
                            &self.theme,
                            "Refresh Now",
                            true,
                            false,
                        )
                        .clicked()
                        {
                            self.telemetry.force_refresh();
                        }
                        if draw_modal_action_button(
                            ui,
                            &self.theme,
                            "Reset Peaks",
                            false,
                            true,
                        )
                        .clicked()
                        {
                            self.telemetry.reset_peaks();
                        }
                    });
                });
            });

        if close_clicked {
            self.show_settings = false;
        }
        if let Some(id) = pending_theme {
            self.set_theme(ctx, id);
        }
    }

    fn render_info_window(&mut self, ctx: &egui::Context) {
        if draw_modal_scrim(ctx, &self.theme, Id::new("info_modal_scrim")) {
            self.show_info = false;
            return;
        }

        let mut close_clicked = false;
        Window::new("guidelines_modal")
            .title_bar(false)
            .order(Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .frame(modal_frame(&self.theme))
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .auto_sized()
            .show(ctx, |ui| {
                ui.set_width(536.0);
                if draw_modal_header(
                    ui,
                    &self.theme,
                    "THERMAL & POWER GUIDELINES",
                    "Reference limits and power status definitions",
                ) {
                    close_clicked = true;
                }

                ui.label(
                    egui::RichText::new("SAFE OPERATING GUIDELINES")
                        .font(self.theme.font_title_semi(11.0))
                        .color(self.theme.accent_primary),
                );
                ui.add_space(6.0);

                Frame::new()
                    .fill(self.theme.bg_inner)
                    .stroke(Stroke::new(1.0_f32, self.theme.border_subtle))
                    .corner_radius(CornerRadius::same(self.theme.inner_radius.min(10)))
                    .inner_margin(Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        egui::Grid::new("guidelines_grid")
                            .striped(true)
                            .spacing([18.0, 7.0])
                            .show(ui, |ui| {
                                for h in ["SENSOR / VRAM", "HEALTHY", "WARM", "CRITICAL / MAX"] {
                                    ui.label(
                                        egui::RichText::new(h)
                                            .font(self.theme.font_title_semi(10.5))
                                            .color(self.theme.text_muted),
                                    );
                                }
                                ui.end_row();

                                let rows = [
                                    ("GPU Core Temp", "< 80°C", "80 – 84°C", "≥ 85°C (Spec ~90–94°C)"),
                                    ("GPU Hotspot (Junction)", "< 95°C", "95 – 105°C", "≥ 105°C (Max 110°C)"),
                                    ("Hotspot - Core ΔT", "≤ 15°C", "15 – 18°C", "> 18°C (Paste check)"),
                                    ("GDDR5 / GDDR6 VRAM", "< 85°C", "85 – 95°C", "≥ 95°C (Max 100°C)"),
                                    ("GDDR6X (3080/3090/4070Ti+)", "< 95°C", "95 – 105°C", "≥ 105°C (Max 110°C)"),
                                    ("GDDR7 (RTX 50 Series)", "< 90°C", "90 – 100°C", "≥ 100°C (Max 105°C)"),
                                ];
                                for (name, ok, warn, crit) in rows {
                                    ui.label(
                                        egui::RichText::new(name)
                                            .font(self.theme.font_ui(11.5))
                                            .color(self.theme.text_primary),
                                    );
                                    ui.label(
                                        egui::RichText::new(ok)
                                            .font(self.theme.font_value(11.5))
                                            .color(self.theme.status_healthy),
                                    );
                                    ui.label(
                                        egui::RichText::new(warn)
                                            .font(self.theme.font_value(11.5))
                                            .color(self.theme.status_warm),
                                    );
                                    ui.label(
                                        egui::RichText::new(crit)
                                            .font(self.theme.font_value(11.5))
                                            .color(self.theme.status_critical),
                                    );
                                    ui.end_row();
                                }
                            });
                    });

                ui.add_space(12.0);
                ui.label(
                    egui::RichText::new("POWER & THROTTLING")
                        .font(self.theme.font_title_semi(11.0))
                        .color(self.theme.accent_secondary),
                );
                ui.add_space(4.0);
                for line in [
                    "• Power 100% (No Software Limit): Enforced ceiling equals 100% of factory default TDP.",
                    "• Software Limit (<99%): Driver/app lowered the power slider below factory TDP.",
                    "• Thermal Throttling: Active HW or SW thermal slowdown reported directly by NVML/sysfs.",
                ] {
                    ui.label(
                        egui::RichText::new(line)
                            .font(self.theme.font_body(11.5))
                            .color(self.theme.text_secondary),
                    );
                }

                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(
                        "Note: On Linux, NVIDIA's proprietary driver exposes Core and (on supported models) Memory temps, while Hotspot requires Windows NVAPI. Older GTX 10-series cards (e.g. GTX 1060) have no VRAM thermal diode.",
                    )
                    .font(self.theme.font_body(10.5))
                    .color(self.theme.text_muted),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(
                        "Disclaimer: Provided \"AS IS\" for informational monitoring only. Guidelines are general estimates; the author assumes no responsibility for overheating, burned/damaged GPUs, or inaccurate driver readings. Licensed under MPL-2.0.",
                    )
                    .font(self.theme.font_body(10.5))
                    .color(self.theme.text_muted),
                );
            });

        if close_clicked {
            self.show_info = false;
        }
    }
}

impl eframe::App for GpuDeckApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = ctx.input(|i| i.time);
        let dt = if self.last_time == 0.0 {
            0.016
        } else {
            (now - self.last_time).clamp(0.001, 0.1) as f32
        };
        self.last_time = now;

        let snap = self.telemetry.snapshot();
        let mut any_throttling = false;
        for gpu in &snap.gpus {
            if gpu.throttle.thermal_throttling {
                any_throttling = true;
            }
            self.animated
                .entry(gpu.key.clone())
                .or_default()
                .step_towards(gpu, dt);
        }

        let panel_frame = Frame::new()
            .fill(self.theme.bg_app)
            .inner_margin(Margin::same(12));

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                paint_ambient_backdrop(ui.painter(), ui.max_rect(), &self.theme);

                let actions = draw_header_bar(
                    ui,
                    &self.theme,
                    snap.gpus.len(),
                    snap.driver_version.as_deref(),
                    self.config.use_fahrenheit,
                    self.show_theme_menu,
                    self.show_settings,
                    self.show_info,
                );
                if actions.theme_btn_rect != Rect::NOTHING {
                    self.theme_btn_rect = Some(actions.theme_btn_rect);
                }

                if actions.reset_all_records {
                    self.telemetry.reset_peaks();
                }
                if actions.toggle_theme_menu {
                    self.show_theme_menu = !self.show_theme_menu;
                }
                if actions.toggle_unit {
                    self.config.use_fahrenheit = !self.config.use_fahrenheit;
                    self.config.save();
                }
                if actions.toggle_settings {
                    self.show_settings = !self.show_settings;
                    if self.show_settings {
                        self.show_info = false;
                        self.show_theme_menu = false;
                    }
                }
                if actions.toggle_info {
                    self.show_info = !self.show_info;
                    if self.show_info {
                        self.show_settings = false;
                        self.show_theme_menu = false;
                    }
                }

                ui.add_space(10.0);

                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let avail_w = ui.available_width();

                        if snap.gpus.is_empty() {
                            ui.add_space(36.0);
                            ui.vertical_centered(|ui| {
                                Frame::new()
                                    .fill(self.theme.bg_card)
                                    .stroke(Stroke::new(1.0_f32, self.theme.border_card))
                                    .corner_radius(CornerRadius::same(self.theme.card_radius))
                                    .inner_margin(Margin::same(24))
                                    .show(ui, |ui| {
                                        ui.set_max_width(520.0);
                                        ui.label(
                                            egui::RichText::new("NO COMPATIBLE GPU DETECTED")
                                                .font(self.theme.font_title(18.0))
                                                .color(self.theme.text_primary),
                                        );
                                        ui.add_space(6.0);
                                        ui.label(
                                            egui::RichText::new(
                                                "Ensure NVIDIA drivers (NVML) or Linux DRM/hwmon sensors are available.",
                                            )
                                            .font(self.theme.font_body(12.5))
                                            .color(self.theme.text_secondary),
                                        );
                                        for note in &snap.backend_notes {
                                            ui.add_space(4.0);
                                            ui.label(
                                                egui::RichText::new(note)
                                                    .font(self.theme.font_body(11.0))
                                                    .color(self.theme.text_muted),
                                            );
                                        }
                                        ui.add_space(14.0);
                                        if draw_modal_action_button(
                                            ui,
                                            &self.theme,
                                            "Refresh Hardware",
                                            true,
                                            false,
                                        )
                                        .clicked()
                                        {
                                            self.telemetry.force_refresh();
                                        }
                                    });
                            });
                            return;
                        }

                        let col_gap = 12.0;
                        let cols = match snap.gpus.len() {
                            1 => 1,
                            2 => {
                                if avail_w >= 960.0 {
                                    2
                                } else {
                                    1
                                }
                            }
                            _ => {
                                if avail_w >= 1560.0 {
                                    3
                                } else if avail_w >= 960.0 {
                                    2
                                } else {
                                    1
                                }
                            }
                        };

                        if snap.gpus.len() == 1 {
                            let card_w = avail_w.min(760.0);
                            let left_pad = ((avail_w - card_w) * 0.5).max(0.0);
                            ui.horizontal_top(|ui| {
                                if left_pad > 0.0 {
                                    ui.add_space(left_pad);
                                }
                                let gpu = &snap.gpus[0];
                                let anim = self.animated.get(&gpu.key).cloned().unwrap_or_default();
                                if draw_gpu_card(
                                    ui,
                                    &self.theme,
                                    gpu,
                                    &anim,
                                    self.config.use_fahrenheit,
                                    now,
                                    card_w,
                                ) {
                                    self.telemetry.reset_gpu_peaks(&gpu.key);
                                }
                            });
                        } else {
                            let card_w = ((avail_w - col_gap * (cols as f32 - 1.0)) / cols as f32)
                                .max(320.0);
                            for chunk in snap.gpus.chunks(cols) {
                                ui.horizontal_top(|ui| {
                                    ui.spacing_mut().item_spacing.x = col_gap;
                                    for gpu in chunk {
                                        let anim =
                                            self.animated.get(&gpu.key).cloned().unwrap_or_default();
                                        if draw_gpu_card(
                                            ui,
                                            &self.theme,
                                            gpu,
                                            &anim,
                                            self.config.use_fahrenheit,
                                            now,
                                            card_w,
                                        ) {
                                            self.telemetry.reset_gpu_peaks(&gpu.key);
                                        }
                                    }
                                });
                                ui.add_space(10.0);
                            }
                        }
                    });
            });

        if self.show_theme_menu {
            self.render_theme_popover(ctx);
        }
        if self.show_settings {
            self.render_settings_window(ctx);
        }
        if self.show_info {
            self.render_info_window(ctx);
        }

        let repaint_ms = if any_throttling { 40 } else { 100 };
        ctx.request_repaint_after(Duration::from_millis(repaint_ms));
    }
}

fn draw_modal_scrim(ctx: &egui::Context, theme: &ThemeSpec, id: Id) -> bool {
    let screen = ctx.screen_rect();
    let scrim_col = Color32::from_rgba_unmultiplied(0, 0, 0, if theme.is_dark { 145 } else { 75 });
    let mut dismiss = ctx.input(|i| i.key_pressed(Key::Escape));
    Area::new(id)
        .order(Order::Middle)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
            ui.painter().rect_filled(rect, CornerRadius::ZERO, scrim_col);
            if resp.clicked() {
                dismiss = true;
            }
        });
    dismiss
}

fn modal_frame(theme: &ThemeSpec) -> Frame {
    Frame::new()
        .fill(theme.bg_card)
        .stroke(Stroke::new(1.0_f32, theme.border_card))
        .corner_radius(CornerRadius::same(theme.card_radius.max(10)))
        .inner_margin(Margin::same(20))
        .shadow(Shadow {
            offset: [0, 10],
            blur: 28,
            spread: 0,
            color: Color32::from_rgba_unmultiplied(0, 0, 0, if theme.is_dark { 165 } else { 65 }),
        })
}

fn draw_modal_header(ui: &mut egui::Ui, theme: &ThemeSpec, title: &str, subtitle: &str) -> bool {
    let mut closed = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(title)
                    .font(theme.font_title(15.0))
                    .color(theme.text_primary),
            );
            ui.label(
                egui::RichText::new(subtitle)
                    .font(theme.font_body(11.5))
                    .color(theme.text_muted),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
            let (btn_r, resp) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
            let cr = CornerRadius::same(theme.badge_radius.min(8));
            let bg = if resp.hovered() {
                with_alpha(theme.accent_primary, if theme.is_dark { 32 } else { 24 })
            } else {
                theme.bg_inner
            };
            let bdr = if resp.hovered() {
                theme.accent_primary
            } else {
                theme.border_subtle
            };
            let p = ui.painter();
            p.rect_filled(btn_r, cr, bg);
            p.rect_stroke(btn_r, cr, Stroke::new(1.0_f32, bdr), StrokeKind::Inside);
            let x_col = if resp.hovered() {
                theme.text_primary
            } else {
                theme.text_secondary
            };
            let c = btn_r.center();
            p.line_segment(
                [Pos2::new(c.x - 4.0, c.y - 4.0), Pos2::new(c.x + 4.0, c.y + 4.0)],
                Stroke::new(1.5_f32, x_col),
            );
            p.line_segment(
                [Pos2::new(c.x - 4.0, c.y + 4.0), Pos2::new(c.x + 4.0, c.y - 4.0)],
                Stroke::new(1.5_f32, x_col),
            );
            if resp.clicked() {
                closed = true;
            }
        });
    });

    ui.add_space(8.0);
    let (div_r, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(div_r.left(), div_r.center().y),
            Pos2::new(div_r.right(), div_r.center().y),
        ],
        Stroke::new(
            1.0_f32,
            with_alpha(theme.border_subtle, if theme.is_dark { 175 } else { 150 }),
        ),
    );
    ui.add_space(12.0);
    closed
}

fn draw_segmented_control(
    ui: &mut egui::Ui,
    theme: &ThemeSpec,
    options: &[&str],
    active_idx: usize,
    total_w: f32,
) -> Option<usize> {
    if options.is_empty() {
        return None;
    }
    let h = 28.0_f32;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(total_w, h), Sense::hover());
    let outer_rad = if theme.badge_radius == 0 {
        0
    } else {
        theme.badge_radius.min(8).max(4)
    };
    let outer_cr = CornerRadius::same(outer_rad);
    ui.painter().rect_filled(rect, outer_cr, theme.bg_track);
    ui.painter().rect_stroke(
        rect,
        outer_cr,
        Stroke::new(1.0_f32, theme.border_subtle),
        StrokeKind::Inside,
    );

    let inner = rect.shrink(2.5);
    let seg_w = inner.width() / options.len() as f32;
    let seg_cr = CornerRadius::same(outer_rad.saturating_sub(2));
    let mut clicked = None;

    for (idx, label) in options.iter().enumerate() {
        let seg_rect = Rect::from_min_size(
            Pos2::new(inner.left() + idx as f32 * seg_w, inner.top()),
            Vec2::new(seg_w, inner.height()),
        );
        let resp = ui.interact(
            seg_rect,
            ui.id().with("seg_btn").with(idx),
            Sense::click(),
        );
        let is_active = idx == active_idx;
        let p = ui.painter();
        if is_active {
            let fill = lerp_color(
                theme.bg_card,
                theme.accent_primary,
                if theme.is_dark { 0.14 } else { 0.10 },
            );
            p.rect_filled(seg_rect, seg_cr, fill);
            p.rect_stroke(
                seg_rect,
                seg_cr,
                Stroke::new(1.0_f32, with_alpha(theme.accent_primary, 180)),
                StrokeKind::Inside,
            );
        } else if resp.hovered() {
            p.rect_filled(seg_rect, seg_cr, with_alpha(theme.text_primary, 14));
        }

        let text_col = if is_active || resp.hovered() {
            theme.text_primary
        } else {
            theme.text_secondary
        };
        let font = if is_active {
            theme.font_value(11.0)
        } else {
            theme.font_ui(11.0)
        };
        p.text(
            seg_rect.center(),
            Align2::CENTER_CENTER,
            *label,
            font,
            text_col,
        );
        if resp.clicked() {
            clicked = Some(idx);
        }
    }

    clicked
}

fn draw_modal_action_button(
    ui: &mut egui::Ui,
    theme: &ThemeSpec,
    label: &str,
    accent: bool,
    danger_hover: bool,
) -> egui::Response {
    let galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), theme.font_value(11.5), theme.text_primary);
    let w = (galley.size().x + 24.0).max(86.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 28.0), Sense::click());
    let cr = CornerRadius::same(theme.badge_radius.min(8));
    let hovered = resp.hovered();

    let (fill, border, text_col) = if accent {
        (
            if hovered {
                with_alpha(theme.accent_primary, if theme.is_dark { 42 } else { 30 })
            } else {
                with_alpha(theme.accent_primary, if theme.is_dark { 22 } else { 16 })
            },
            theme.accent_primary,
            theme.accent_primary,
        )
    } else if danger_hover && hovered {
        (
            with_alpha(theme.status_warm, if theme.is_dark { 32 } else { 22 }),
            theme.status_warm,
            theme.status_warm,
        )
    } else {
        (
            if hovered {
                with_alpha(theme.text_primary, 14)
            } else {
                theme.bg_inner
            },
            theme.border_card,
            if hovered {
                theme.text_primary
            } else {
                theme.text_secondary
            },
        )
    };

    let p = ui.painter();
    p.rect_filled(rect, cr, fill);
    p.rect_stroke(rect, cr, Stroke::new(1.0_f32, border), StrokeKind::Inside);
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        theme.font_value(11.5),
        text_col,
    );
    resp
}
