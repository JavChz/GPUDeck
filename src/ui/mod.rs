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
    apply_theme_visuals, setup_fonts, with_alpha, ThemeId, ThemeSpec,
};
use widgets::{draw_header_bar, paint_ambient_backdrop};

use crate::config::AppConfig;
use crate::telemetry::TelemetryManager;

pub struct AeroGpuApp {
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

impl AeroGpuApp {
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
                                .font(self.theme.font_bold(10.5))
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
                                    self.theme.font_bold(12.5)
                                } else {
                                    self.theme.font_semibold(12.5)
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
        let mut open = self.show_settings;
        let mut pending_theme: Option<ThemeId> = None;

        Window::new("Settings & Themes")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(430.0, 390.0))
            .show(ctx, |ui| {
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("Visual Theme")
                        .font(self.theme.font_bold(14.0))
                        .color(self.theme.accent_primary),
                );
                ui.add_space(6.0);

                let avail_w = ui.available_width();
                let gap = 8.0;
                let cell_w = ((avail_w - gap * 2.0) / 3.0).floor();

                for row in ThemeId::ALL.chunks(3) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for &id in row {
                            let selected = id == self.config.theme;
                            let swatches = id.swatch_colors();
                            let (r, resp) =
                                ui.allocate_exact_size(Vec2::new(cell_w, 46.0), Sense::click());
                            let p = ui.painter();
                            let cr = CornerRadius::same(self.theme.inner_radius.min(10));

                            let bg = if selected {
                                with_alpha(self.theme.accent_primary, 36)
                            } else if resp.hovered() {
                                with_alpha(self.theme.accent_primary, 18)
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

                            for (i, col) in swatches.iter().enumerate() {
                                let c = Pos2::new(r.left() + 14.0 + i as f32 * 11.0, r.top() + 14.0);
                                p.circle_filled(c, 4.5, *col);
                                p.circle_stroke(
                                    c,
                                    4.5,
                                    Stroke::new(
                                        0.8_f32,
                                        with_alpha(self.theme.text_primary, 110),
                                    ),
                                );
                            }

                            p.text(
                                Pos2::new(r.left() + 10.0, r.bottom() - 12.0),
                                Align2::LEFT_CENTER,
                                id.display_name(),
                                if selected {
                                    self.theme.font_bold(11.5)
                                } else {
                                    self.theme.font_semibold(11.5)
                                },
                                self.theme.text_primary,
                            );

                            if resp.clicked() {
                                pending_theme = Some(id);
                            }
                        }
                    });
                    ui.add_space(6.0);
                }

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    egui::RichText::new("Telemetry & Display")
                        .font(self.theme.font_bold(14.0))
                        .color(self.theme.accent_primary),
                );
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Temperature Unit:")
                            .font(self.theme.font_semibold(12.5))
                            .color(self.theme.text_secondary),
                    );
                    if ui
                        .selectable_label(!self.config.use_fahrenheit, "Celsius (°C)")
                        .clicked()
                    {
                        self.config.use_fahrenheit = false;
                        self.config.save();
                    }
                    if ui
                        .selectable_label(self.config.use_fahrenheit, "Fahrenheit (°F)")
                        .clicked()
                    {
                        self.config.use_fahrenheit = true;
                        self.config.save();
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Polling Rate:")
                            .font(self.theme.font_semibold(12.5))
                            .color(self.theme.text_secondary),
                    );
                    for (label, ms) in [("500ms", 500u64), ("1000ms", 1000), ("2000ms", 2000)] {
                        if ui
                            .selectable_label(self.config.poll_interval_ms == ms, label)
                            .clicked()
                        {
                            self.config.poll_interval_ms = ms;
                            self.telemetry.set_poll_interval_ms(ms);
                            self.config.save();
                        }
                    }
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Reset All Max Records").clicked() {
                        self.telemetry.reset_peaks();
                    }
                    if ui.button("Poll Now").clicked() {
                        self.telemetry.force_refresh();
                    }
                });

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(
                        "AERO-GPU is read-only and never modifies GPU clocks, fan curves, or power limits.",
                    )
                    .font(self.theme.font_regular(11.0))
                    .color(self.theme.text_muted),
                );
            });

        self.show_settings = open;
        if let Some(id) = pending_theme {
            self.set_theme(ctx, id);
        }
    }

    fn render_info_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_info;
        Window::new("Thermal & Power Guidelines")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(560.0, 435.0))
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Safe Operating Guidelines by Sensor & Memory Type")
                        .font(self.theme.font_bold(14.0))
                        .color(self.theme.accent_primary),
                );
                ui.add_space(6.0);

                egui::Grid::new("guidelines_grid")
                    .striped(true)
                    .spacing([18.0, 7.0])
                    .show(ui, |ui| {
                        for h in ["Sensor / VRAM Class", "Healthy", "Warm", "Critical / Max"] {
                            ui.label(
                                egui::RichText::new(h)
                                    .font(self.theme.font_bold(12.0))
                                    .color(self.theme.text_primary),
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
                                    .font(self.theme.font_semibold(11.5))
                                    .color(self.theme.text_primary),
                            );
                            ui.label(
                                egui::RichText::new(ok)
                                    .font(self.theme.font_mono(11.5))
                                    .color(self.theme.status_healthy),
                            );
                            ui.label(
                                egui::RichText::new(warn)
                                    .font(self.theme.font_mono(11.5))
                                    .color(self.theme.status_warm),
                            );
                            ui.label(
                                egui::RichText::new(crit)
                                    .font(self.theme.font_mono(11.5))
                                    .color(self.theme.status_critical),
                            );
                            ui.end_row();
                        }
                    });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(
                    egui::RichText::new("Power & Throttling Indicators")
                        .font(self.theme.font_bold(13.5))
                        .color(self.theme.accent_secondary),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(
                        "• Power 100% (No Software Limit): Enforced power ceiling equals 100% of factory default TDP.",
                    )
                    .font(self.theme.font_regular(11.5))
                    .color(self.theme.text_secondary),
                );
                ui.label(
                    egui::RichText::new(
                        "• Software Limit (<99%): Driver/app lowered the power slider below factory TDP.",
                    )
                    .font(self.theme.font_regular(11.5))
                    .color(self.theme.text_secondary),
                );
                ui.label(
                    egui::RichText::new(
                        "• Thermal Throttling: Active HW or SW thermal slowdown reported directly by NVML/sysfs.",
                    )
                    .font(self.theme.font_regular(11.5))
                    .color(self.theme.text_secondary),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(
                        "Note: On Linux, NVIDIA's proprietary driver exposes Core and (on supported models) Memory temps, while Hotspot requires Windows NVAPI. Older GTX 10-series cards (e.g. GTX 1060) have no VRAM thermal diode.",
                    )
                    .font(self.theme.font_regular(10.5))
                    .color(self.theme.text_muted),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(
                        "Disclaimer: Provided \"AS IS\" for informational monitoring only. Guidelines are general estimates; the author assumes no responsibility for overheating, burned/damaged GPUs, or inaccurate driver readings. Licensed under MPL-2.0.",
                    )
                    .font(self.theme.font_regular(10.5))
                    .color(self.theme.text_muted),
                );
            });
        self.show_info = open;
    }
}

impl eframe::App for AeroGpuApp {
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
                }
                if actions.toggle_info {
                    self.show_info = !self.show_info;
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
                                            egui::RichText::new("No Supported GPUs Detected")
                                                .font(self.theme.font_bold(18.0))
                                                .color(self.theme.text_primary),
                                        );
                                        ui.add_space(6.0);
                                        ui.label(
                                            egui::RichText::new(
                                                "Ensure NVIDIA drivers (NVML) or Linux DRM/hwmon sensors are available.",
                                            )
                                            .font(self.theme.font_regular(12.5))
                                            .color(self.theme.text_secondary),
                                        );
                                        for note in &snap.backend_notes {
                                            ui.add_space(4.0);
                                            ui.label(
                                                egui::RichText::new(note)
                                                    .font(self.theme.font_mono(11.0))
                                                    .color(self.theme.text_muted),
                                            );
                                        }
                                        ui.add_space(14.0);
                                        if ui.button("Refresh Hardware").clicked() {
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
