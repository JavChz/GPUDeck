use std::fs;
use std::path::{Path, PathBuf};

use super::{
    classify_power_mode, ClockTelemetry, FanTelemetry, GpuBackend, GpuVendor, PowerTelemetry,
    RawGpuSample, ThermalTelemetry, ThrottleTelemetry, VramTelemetry,
};

pub struct LinuxSysfsBackend {
    last_error: Option<String>,
}

impl LinuxSysfsBackend {
    pub fn new() -> Self {
        Self { last_error: None }
    }
}

impl GpuBackend for LinuxSysfsBackend {
    fn name(&self) -> &'static str {
        "Linux DRM/hwmon"
    }

    fn diagnostics(&self) -> Option<String> {
        self.last_error.clone()
    }

    fn poll(&mut self) -> Vec<RawGpuSample> {
        let mut results = Vec::new();
        let drm_dir = Path::new("/sys/class/drm");
        let Ok(entries) = fs::read_dir(drm_dir) else {
            return results;
        };

        let mut card_paths: Vec<PathBuf> = entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.starts_with("card") && !name.contains('-') {
                    Some(e.path())
                } else {
                    None
                }
            })
            .collect();
        card_paths.sort();

        for card_path in card_paths {
            let dev_dir = card_path.join("device");
            if !dev_dir.exists() {
                continue;
            }
            let vendor_hex = read_trimmed(&dev_dir.join("vendor")).unwrap_or_default();
            let vendor = match vendor_hex.as_str() {
                "0x1002" => GpuVendor::Amd,
                "0x8086" => GpuVendor::Intel,
                "0x10de" => GpuVendor::Nvidia,
                _ => GpuVendor::Other,
            };

            // Only pick up non-NVIDIA or fallback NVIDIA if NVML didn't claim the PCI ID.
            let pci_bus_id = fs::read_link(&dev_dir)
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_default();

            let hwmon_dir = find_hwmon_dir(&dev_dir);
            let Some(hwmon) = hwmon_dir else {
                continue;
            };

            let mut core_temp = None;
            let mut hotspot_temp = None;
            let mut memory_temp = None;
            let mut hw_crit = None;
            let mut hw_mem_crit = None;

            for i in 1..=8 {
                let input_path = hwmon.join(format!("temp{i}_input"));
                let Some(val_c) = read_u64(&input_path).map(|v| v as f32 / 1000.0) else {
                    continue;
                };
                let label = read_trimmed(&hwmon.join(format!("temp{i}_label")))
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                let crit = read_u64(&hwmon.join(format!("temp{i}_crit")))
                    .map(|v| v as f32 / 1000.0);

                if label.contains("junction") || label.contains("hotspot") {
                    hotspot_temp = Some(val_c);
                } else if label.contains("mem") {
                    memory_temp = Some(val_c);
                    hw_mem_crit = crit;
                } else if label.contains("edge") || core_temp.is_none() {
                    core_temp = Some(val_c);
                    hw_crit = crit;
                }
            }

            let draw_w = read_u64(&hwmon.join("power1_average"))
                .or_else(|| read_u64(&hwmon.join("power1_input")))
                .map(|uw| uw as f32 / 1_000_000.0);
            let cap_w = read_u64(&hwmon.join("power1_cap")).map(|uw| uw as f32 / 1_000_000.0);
            let default_cap_w =
                read_u64(&hwmon.join("power1_cap_default")).map(|uw| uw as f32 / 1_000_000.0);
            let max_cap_w =
                read_u64(&hwmon.join("power1_cap_max")).map(|uw| uw as f32 / 1_000_000.0);
            let min_cap_w =
                read_u64(&hwmon.join("power1_cap_min")).map(|uw| uw as f32 / 1_000_000.0);

            let (limit_pct, mode) = classify_power_mode(cap_w, cap_w, default_cap_w);
            let sw_power_cap = matches!(mode, super::PowerLimitMode::SoftwareLimited { .. });

            let fan_rpm = read_u64(&hwmon.join("fan1_input")).map(|v| v as u32);
            let fan_pct = read_u64(&hwmon.join("pwm1")).and_then(|pwm| {
                let max = read_u64(&hwmon.join("pwm1_max")).unwrap_or(255).max(1);
                Some(((pwm as f32 / max as f32) * 100.0).round() as u32)
            });

            let vram_used_mb =
                read_u64(&dev_dir.join("mem_info_vram_used")).unwrap_or(0) / (1024 * 1024);
            let vram_total_mb =
                read_u64(&dev_dir.join("mem_info_vram_total")).unwrap_or(0) / (1024 * 1024);
            let gpu_util_pct = read_u64(&dev_dir.join("gpu_busy_percent")).map(|v| v as u32);

            let device_id = read_trimmed(&dev_dir.join("device")).unwrap_or_default();
            let hwmon_name = read_trimmed(&hwmon.join("name")).unwrap_or_default();
            let name = match vendor {
                GpuVendor::Amd => {
                    if device_id.is_empty() {
                        "AMD Radeon GPU".to_string()
                    } else {
                        format!("AMD Radeon GPU ({device_id})")
                    }
                }
                GpuVendor::Intel => format!("Intel GPU ({device_id})"),
                GpuVendor::Nvidia => format!("NVIDIA GPU ({device_id})"),
                GpuVendor::Other => format!("GPU ({hwmon_name})"),
            };

            let delta_hotspot_core = match (core_temp, hotspot_temp) {
                (Some(c), Some(h)) => Some((h - c).max(0.0)),
                _ => None,
            };

            let thermal_throttling = match (core_temp, hw_crit) {
                (Some(c), Some(crit)) => c >= crit,
                _ => false,
            };

            let software_cap_summary = match &mode {
                super::PowerLimitMode::SoftwareLimited { pct, .. } => {
                    format!("Power Cap ({pct:.0}%)")
                }
                _ => "None".to_string(),
            };

            results.push(RawGpuSample {
                key: format!("sysfs-{}", pci_bus_id),
                index: results.len() as u32,
                name,
                vendor,
                pci_bus_id,
                driver_version: None,
                architecture: None,
                thermals: ThermalTelemetry {
                    core: core_temp,
                    hotspot: hotspot_temp,
                    hotspot_note: if hotspot_temp.is_none() {
                        Some("Not exposed by sysfs".to_string())
                    } else {
                        None
                    },
                    memory: memory_temp,
                    memory_note: if memory_temp.is_none() {
                        Some("Not exposed by sysfs".to_string())
                    } else {
                        None
                    },
                    delta_hotspot_core,
                    hw_target_temp: None,
                    hw_max_operating_temp: hw_crit,
                    hw_slowdown_temp: hw_crit,
                    hw_shutdown_temp: None,
                    hw_memory_max_temp: hw_mem_crit,
                    ..Default::default()
                },
                power: PowerTelemetry {
                    draw_w,
                    active_limit_w: cap_w,
                    enforced_limit_w: cap_w,
                    default_limit_w: default_cap_w,
                    min_limit_w: min_cap_w,
                    max_limit_w: max_cap_w,
                    limit_pct_of_default: limit_pct,
                    mode,
                },
                throttle: ThrottleTelemetry {
                    thermal_throttling,
                    sw_power_cap,
                    software_cap_summary,
                    ..Default::default()
                },
                fans: FanTelemetry {
                    speed_pct: fan_pct,
                    rpm: fan_rpm,
                    fan_count: u32::from(fan_pct.is_some() || fan_rpm.is_some()),
                    per_fan_pct: fan_pct.into_iter().collect(),
                    per_fan_rpm: fan_rpm.map(Some).into_iter().collect(),
                },
                vram: VramTelemetry {
                    used_mb: vram_used_mb,
                    total_mb: vram_total_mb,
                },
                clocks: ClockTelemetry {
                    gpu_util_pct,
                    ..Default::default()
                },
            });
        }

        results
    }
}

fn find_hwmon_dir(dev_dir: &Path) -> Option<PathBuf> {
    let hwmon_root = dev_dir.join("hwmon");
    let entries = fs::read_dir(hwmon_root).ok()?;
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            return Some(entry.path());
        }
    }
    None
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn read_u64(path: &Path) -> Option<u64> {
    read_trimmed(path)?.parse::<u64>().ok()
}
