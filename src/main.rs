#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod telemetry;
mod ui;

use std::sync::Arc;

use config::AppConfig;
use telemetry::{PowerLimitMode, TelemetryManager};
use ui::theme::ThemeId;
use ui::GpuDeckApp;

fn main() -> eframe::Result<()> {
    let mut probe = false;
    let mut config = AppConfig::load();

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--probe" => probe = true,
            "--theme" => {
                let Some(val) = args.next() else {
                    eprintln!(
                        "Error: --theme requires a theme name. Valid themes: {}",
                        valid_theme_list()
                    );
                    std::process::exit(2);
                };
                match ThemeId::from_str_loose(&val) {
                    Some(t) => config.theme = t,
                    None => {
                        eprintln!(
                            "Error: unknown theme '{val}'. Valid themes: {}",
                            valid_theme_list()
                        );
                        std::process::exit(2);
                    }
                }
            }
            "--interval" => {
                if let Some(val) = args.next() {
                    if let Ok(parsed) = val.parse::<u64>() {
                        config.poll_interval_ms = parsed.clamp(250, 10_000);
                    }
                }
            }
            "--help" | "-h" => {
                println!(
                    "GPUDeck - Multi-GPU Thermal, Power Limit & Throttling Dashboard\n\
                     Usage: gpudeck [OPTIONS]\n\n\
                     Options:\n\
                       --probe            Print detected GPU telemetry once to stdout and exit\n\
                       --theme <name>     Start with a specific theme ({})\n\
                       --interval <ms>    Polling interval in milliseconds (default: 1000)\n\
                       -V, --version      Show application version\n\
                       -h, --help         Show this help message",
                    valid_theme_list()
                );
                return Ok(());
            }
            "-V" | "--version" => {
                println!("gpudeck {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => {}
        }
    }

    if probe {
        run_probe();
        return Ok(());
    }

    let icon = Arc::new(build_app_icon());
    let viewport = egui::ViewportBuilder::default()
        .with_title("GPUDeck")
        .with_inner_size([1280.0, 720.0])
        .with_min_inner_size([640.0, 540.0])
        .with_icon(icon);

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "GPUDeck",
        options,
        Box::new(move |cc| Ok(Box::new(GpuDeckApp::new(cc, config)))),
    )
}

fn valid_theme_list() -> String {
    ThemeId::ALL
        .iter()
        .map(|t| t.slug())
        .collect::<Vec<_>>()
        .join(", ")
}

fn run_probe() {
    let (gpus, notes) = TelemetryManager::probe_once();
    println!("=== GPUDeck Hardware Probe ===");
    for note in &notes {
        println!("[diag] {note}");
    }
    println!("Detected GPUs: {}", gpus.len());
    for g in &gpus {
        println!("\nGPU {}: {} ({:?})", g.index, g.name, g.vendor);
        println!(
            "  PCI: {}  |  Arch: {}  |  VRAM Class: {}  |  Driver: {}",
            g.pci_bus_id,
            g.architecture.as_deref().unwrap_or("Unknown"),
            g.guidelines.vram_class,
            g.driver_version.as_deref().unwrap_or("Unknown")
        );
        println!(
            "  Health: {} ({})",
            g.health_badge.label(),
            g.health_subtitle
        );
        println!(
            "  Temps: Core={}  |  Hotspot={}  |  Memory={}",
            fmt_opt_c(g.thermals.core),
            fmt_opt_c_note(g.thermals.hotspot, g.thermals.hotspot_note.as_deref()),
            fmt_opt_c_note(g.thermals.memory, g.thermals.memory_note.as_deref()),
        );
        println!(
            "  Guidelines (Healthy): Core <{:.0}C | Hotspot <{:.0}C | Memory <{:.0}C (Core max spec {:.0}C)",
            g.guidelines.core.healthy_max,
            g.guidelines.hotspot.healthy_max,
            g.guidelines.memory.healthy_max,
            g.guidelines.core.max_spec
        );
        let power_mode_str = match g.power.mode {
            PowerLimitMode::Full100Percent => "100% (No Software Limit)".to_string(),
            PowerLimitMode::SoftwareLimited {
                pct,
                active_w,
                default_w,
            } => format!("{pct:.0}% (Software Limit: {active_w:.0}W / {default_w:.0}W)"),
            PowerLimitMode::OverclockedLimit {
                pct,
                active_w,
                default_w,
            } => format!("{pct:.0}% (Unlocked: {active_w:.0}W / {default_w:.0}W)"),
            PowerLimitMode::Unknown => "Unknown".to_string(),
        };
        println!(
            "  Power: Draw={} / Limit={} (Default={})  |  Power Mode: {}",
            fmt_opt_w(g.power.draw_w),
            fmt_opt_w(g.power.enforced_limit_w.or(g.power.active_limit_w)),
            fmt_opt_w(g.power.default_limit_w),
            power_mode_str
        );
        println!(
            "  Throttling: Thermal={}  |  Software Cap: {}",
            if g.throttle.thermal_throttling {
                "YES"
            } else {
                "NO"
            },
            g.throttle.software_cap_summary
        );
        println!(
            "  Fans: {}% / {} RPM  |  VRAM: {:.2} / {:.2} GB  |  Clocks: {} MHz ({})",
            g.fans
                .speed_pct
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            g.fans
                .rpm
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            g.vram.used_gb(),
            g.vram.total_gb(),
            g.clocks
                .graphics_mhz
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            g.clocks.pstate.as_deref().unwrap_or("N/A"),
        );
    }
}

fn fmt_opt_c(v: Option<f32>) -> String {
    v.map(|c| format!("{c:.1}C"))
        .unwrap_or_else(|| "N/A".to_string())
}

fn fmt_opt_c_note(v: Option<f32>, note: Option<&str>) -> String {
    match (v, note) {
        (Some(c), _) => format!("{c:.1}C"),
        (None, Some(n)) => format!("N/A ({n})"),
        (None, None) => "N/A".to_string(),
    }
}

fn fmt_opt_w(v: Option<f32>) -> String {
    v.map(|w| format!("{w:.1}W"))
        .unwrap_or_else(|| "N/A".to_string())
}

fn build_app_icon() -> egui::IconData {
    let size = 64usize;
    let mut rgba = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let cx = size as f32 * 0.5;
            let cy = size as f32 * 0.5;
            let dx = fx - cx;
            let dy = fy - cy;
            let dist = (dx * dx + dy * dy).sqrt();

            let rx = (dx.abs() - 22.0).max(0.0);
            let ry = (dy.abs() - 22.0).max(0.0);
            let box_dist = (rx * rx + ry * ry).sqrt() - 8.0;
            if box_dist > 0.0 {
                continue;
            }

            let mut r = 10.0f32;
            let mut g = 16.0f32;
            let mut b = 26.0f32;

            if box_dist > -2.6 {
                let t = fx / size as f32;
                r = 34.0 + (236.0 - 34.0) * t;
                g = 211.0 + (72.0 - 211.0) * t;
                b = 238.0 + (153.0 - 238.0) * t;
            } else if (dist - 16.5).abs() < 3.4 {
                let angle = dy.atan2(dx).to_degrees();
                if !(45.0..=135.0).contains(&angle) {
                    let t = ((angle + 180.0) / 360.0).clamp(0.0, 1.0);
                    r = 34.0 + (56.0 - 34.0) * t;
                    g = 211.0 + (189.0 - 211.0) * t;
                    b = 238.0 + (248.0 - 238.0) * t;
                }
            } else if dist < 5.0 {
                r = 94.0;
                g = 242.0;
                b = 255.0;
            }

            let idx = (y * size + x) * 4;
            rgba[idx] = r.round() as u8;
            rgba[idx + 1] = g.round() as u8;
            rgba[idx + 2] = b.round() as u8;
            rgba[idx + 3] = 255;
        }
    }

    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::config::{parse_config, parse_records, serialize_records, AppConfig, GpuTempRecord};
    use super::telemetry::guidelines::{
        evaluate_overall_health, evaluate_temp_level, guidelines_for_gpu, HealthBadge,
        HealthEvalInput, MetricLevel,
    };
    use super::telemetry::{classify_power_mode, GpuHistory, PowerLimitMode, HISTORY_CAPACITY};
    use super::ui::theme::{ThemeId, ThemeSpec};

    #[test]
    fn test_guidelines_vram_classes() {
        let gtx1060 =
            guidelines_for_gpu("NVIDIA GeForce GTX 1060 6GB", None, Some(94.0), None, None);
        assert_eq!(gtx1060.vram_class, "GDDR5");
        assert_eq!(gtx1060.core.max_spec, 94.0);
        assert!(gtx1060.core.healthy_max <= 82.0);
        assert_eq!(gtx1060.memory.healthy_max, 85.0);

        let rtx3060 = guidelines_for_gpu(
            "NVIDIA GeForce RTX 3060",
            Some(83.0),
            Some(93.0),
            None,
            None,
        );
        assert_eq!(rtx3060.vram_class, "GDDR6");
        assert_eq!(rtx3060.core.healthy_max, 80.0);
        assert_eq!(rtx3060.memory.healthy_max, 85.0);

        let rtx3090 = guidelines_for_gpu(
            "NVIDIA GeForce RTX 3090",
            Some(83.0),
            Some(93.0),
            Some(95.0),
            None,
        );
        assert_eq!(rtx3090.vram_class, "GDDR6X");
        assert_eq!(rtx3090.memory.healthy_max, 95.0);
        assert_eq!(rtx3090.memory.max_spec, 110.0);

        let rtx4070ti =
            guidelines_for_gpu("NVIDIA GeForce RTX 4070 Ti", None, Some(90.0), None, None);
        assert_eq!(rtx4070ti.vram_class, "GDDR6X");
        assert_eq!(rtx4070ti.memory.healthy_max, 95.0);

        let rtx5090 = guidelines_for_gpu("NVIDIA GeForce RTX 5090", None, Some(90.0), None, None);
        assert_eq!(rtx5090.vram_class, "GDDR7");
        assert_eq!(rtx5090.memory.healthy_max, 90.0);
        assert_eq!(rtx5090.memory.max_spec, 105.0);

        let custom_mem =
            guidelines_for_gpu("NVIDIA GeForce RTX 3090", None, None, None, Some(96.0));
        assert_eq!(custom_mem.memory.max_spec, 96.0);
        assert!(custom_mem.memory.warn_max <= 93.0);
    }

    #[test]
    fn test_power_limit_classification() {
        let (pct_full, mode_full) = classify_power_mode(Some(350.0), Some(350.0), Some(350.0));
        assert_eq!(pct_full, Some(100.0));
        assert_eq!(mode_full, PowerLimitMode::Full100Percent);

        let (pct_sw, mode_sw) = classify_power_mode(Some(280.0), Some(280.0), Some(350.0));
        assert_eq!(pct_sw, Some(80.0));
        assert!(matches!(
            mode_sw,
            PowerLimitMode::SoftwareLimited { pct, active_w, default_w }
                if (pct - 80.0).abs() < 0.01 && active_w == 280.0 && default_w == 350.0
        ));

        let (pct_oc, mode_oc) = classify_power_mode(Some(385.0), Some(385.0), Some(350.0));
        assert!((pct_oc.unwrap() - 110.0).abs() < 0.01);
        assert!(matches!(mode_oc, PowerLimitMode::OverclockedLimit { .. }));
    }

    #[test]
    fn test_health_precedence_and_levels() {
        let g = guidelines_for_gpu(
            "NVIDIA GeForce RTX 4070 Ti",
            Some(84.0),
            Some(90.0),
            None,
            None,
        );
        assert_eq!(evaluate_temp_level(68.0, &g.core), MetricLevel::Healthy);
        assert_eq!(evaluate_temp_level(82.0, &g.core), MetricLevel::Warm);
        assert_eq!(evaluate_temp_level(88.0, &g.core), MetricLevel::Critical);

        let (b_ok, _) = evaluate_overall_health(HealthEvalInput {
            core: Some(68.0),
            hotspot: Some(81.0),
            memory: Some(74.0),
            delta_hotspot_core: Some(13.0),
            under_load: true,
            thermal_throttling: false,
            guidelines: &g,
        });
        assert_eq!(b_ok, HealthBadge::Healthy);

        let (b_warm_delta, _) = evaluate_overall_health(HealthEvalInput {
            core: Some(70.0),
            hotspot: Some(92.0),
            memory: Some(76.0),
            delta_hotspot_core: Some(22.0),
            under_load: true,
            thermal_throttling: false,
            guidelines: &g,
        });
        assert_eq!(b_warm_delta, HealthBadge::Warm);

        let (b_crit, _) = evaluate_overall_health(HealthEvalInput {
            core: Some(89.0),
            hotspot: Some(106.0),
            memory: Some(98.0),
            delta_hotspot_core: Some(17.0),
            under_load: true,
            thermal_throttling: false,
            guidelines: &g,
        });
        assert_eq!(b_crit, HealthBadge::Critical);

        let (b_thr, _) = evaluate_overall_health(HealthEvalInput {
            core: Some(89.0),
            hotspot: Some(106.0),
            memory: Some(98.0),
            delta_hotspot_core: Some(17.0),
            under_load: true,
            thermal_throttling: true,
            guidelines: &g,
        });
        assert_eq!(b_thr, HealthBadge::Throttling);
    }

    #[test]
    fn test_history_buffer_bounded() {
        let mut buf = std::collections::VecDeque::new();
        for i in 0..(HISTORY_CAPACITY + 25) {
            GpuHistory::push_bounded(&mut buf, i as f32);
        }
        assert_eq!(buf.len(), HISTORY_CAPACITY);
        assert_eq!(buf.front().copied(), Some(25.0));
    }

    #[test]
    fn test_all_nine_themes_and_aliases() {
        assert_eq!(ThemeId::ALL.len(), 9);
        for id in ThemeId::ALL {
            assert_eq!(ThemeId::from_str_loose(id.slug()), Some(id));
            assert_eq!(ThemeId::from_str_loose(id.display_name()), Some(id));
            let spec = ThemeSpec::for_id(id);
            assert_eq!(spec.id, id);
        }
        assert_eq!(ThemeId::from_str_loose("japendi"), Some(ThemeId::Japandi));
        assert_eq!(ThemeId::from_str_loose("aero"), Some(ThemeId::FrutigerAero));
        assert_eq!(ThemeId::from_str_loose("win"), Some(ThemeId::WinLike));
        assert_eq!(ThemeId::from_str_loose("tui"), Some(ThemeId::Terminal));
        assert_eq!(
            ThemeId::from_str_loose("enterprise"),
            Some(ThemeId::Corporate)
        );
        assert!(ThemeSpec::for_id(ThemeId::Terminal).force_monospace);
        assert!(!ThemeSpec::for_id(ThemeId::FrutigerAero).is_dark);
        assert!(ThemeSpec::for_id(ThemeId::WinLike).is_dark);
    }

    #[test]
    fn test_config_roundtrip() {
        for id in ThemeId::ALL {
            let cfg = AppConfig {
                theme: id,
                use_fahrenheit: true,
                poll_interval_ms: 500,
            };
            let parsed = parse_config(&cfg.serialize_config());
            assert_eq!(parsed, cfg);
        }
    }

    #[test]
    fn test_records_roundtrip() {
        let mut records = HashMap::new();
        records.insert(
            "nv-00000000:0a:00.0".to_string(),
            GpuTempRecord {
                peak_core: Some(73.5),
                peak_hotspot: Some(85.3),
                peak_memory: Some(92.0),
                ever_thermal_throttled: false,
            },
        );
        records.insert(
            "nv-00000000:0b:00.0".to_string(),
            GpuTempRecord {
                peak_core: Some(68.2),
                peak_hotspot: Some(81.0),
                peak_memory: None,
                ever_thermal_throttled: true,
            },
        );
        let serialized = serialize_records(&records);
        let parsed = parse_records(&serialized);
        assert_eq!(parsed, records);
    }
}
