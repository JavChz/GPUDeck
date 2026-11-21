use nvml_wrapper::bitmasks::device::ThrottleReasons;
use nvml_wrapper::enum_wrappers::device::{Clock, PerformanceState, TemperatureSensor, TemperatureThreshold};
use nvml_wrapper::enums::device::SampleValue;
use nvml_wrapper::structs::device::FieldId;
use nvml_wrapper::Nvml;

use super::guidelines::detect_vram_class;
#[cfg(target_os = "windows")]
use super::nvapi_win::NvApiSession;
use super::{
    classify_power_mode, ClockTelemetry, FanTelemetry, GpuBackend, GpuVendor, PowerLimitMode,
    PowerTelemetry, RawGpuSample, ThermalTelemetry, ThrottleTelemetry, VramTelemetry,
};

const NVML_FI_DEV_MEMORY_TEMP: u32 = 82;

pub struct NvidiaBackend {
    nvml: Option<Nvml>,
    #[cfg(target_os = "windows")]
    nvapi: Option<NvApiSession>,
    init_error: Option<String>,
}

impl NvidiaBackend {
    pub fn new() -> Self {
        let (nvml, init_error) = match Nvml::init() {
            Ok(n) => (Some(n), None),
            Err(e) => (None, Some(format!("NVML unavailable: {e}"))),
        };

        #[cfg(target_os = "windows")]
        let nvapi = NvApiSession::new().ok();

        Self {
            nvml,
            #[cfg(target_os = "windows")]
            nvapi,
            init_error,
        }
    }
}

impl GpuBackend for NvidiaBackend {
    fn name(&self) -> &'static str {
        "NVIDIA NVML"
    }

    fn diagnostics(&self) -> Option<String> {
        self.init_error.clone()
    }

    fn poll(&mut self) -> Vec<RawGpuSample> {
        let Some(ref nvml) = self.nvml else {
            return Vec::new();
        };

        let Ok(count) = nvml.device_count() else {
            return Vec::new();
        };
        let driver_version = nvml.sys_driver_version().ok();
        let mut samples = Vec::with_capacity(count as usize);

        for idx in 0..count {
            let Ok(device) = nvml.device_by_index(idx) else {
                continue;
            };

            let name = device
                .name()
                .unwrap_or_else(|_| format!("NVIDIA GPU {idx}"));
            let pci = device.pci_info().ok();
            let pci_bus_id = pci
                .as_ref()
                .map(|p| p.bus_id.clone())
                .unwrap_or_else(|| format!("nvml-idx-{idx}"));
            let bus = pci.as_ref().map(|p| p.bus).unwrap_or(0);
            let device_slot = pci.as_ref().map(|p| p.device).unwrap_or(0);
            let architecture = device.architecture().ok().map(|a| format!("{a:?}"));

            let core = device
                .temperature(TemperatureSensor::Gpu)
                .ok()
                .map(|t| t as f32);

            let hw_target_temp = device
                .temperature_threshold(TemperatureThreshold::AcousticCurr)
                .ok()
                .map(|t| t as f32);
            let hw_max_operating_temp = device
                .temperature_threshold(TemperatureThreshold::GpuMax)
                .ok()
                .map(|t| t as f32);
            let hw_slowdown_temp = device
                .temperature_threshold(TemperatureThreshold::Slowdown)
                .ok()
                .map(|t| t as f32);
            let hw_shutdown_temp = device
                .temperature_threshold(TemperatureThreshold::Shutdown)
                .ok()
                .map(|t| t as f32);
            let hw_memory_max_temp = device
                .temperature_threshold(TemperatureThreshold::MemoryMax)
                .ok()
                .map(|t| t as f32);

            #[allow(unused_mut)]
            let mut hotspot: Option<f32> = None;
            let mut memory: Option<f32> = None;
            #[allow(unused_mut)]
            let mut nvapi_tach_rpm: Option<u32> = None;

            #[cfg(target_os = "windows")]
            if let Some(ref nvapi) = self.nvapi {
                let ext = nvapi.read_extended_sensors(bus, device_slot, &name, idx as usize);
                hotspot = ext.hotspot_c;
                memory = ext.memory_c;
                nvapi_tach_rpm = ext.tach_rpm;
            }

            if memory.is_none() {
                memory = query_nvml_memory_temp(&device);
            }

            let delta_hotspot_core = match (core, hotspot) {
                (Some(c), Some(h)) => Some((h - c).max(0.0)),
                _ => None,
            };

            let hotspot_note = if hotspot.is_none() {
                let upper = name.to_ascii_uppercase();
                if upper.contains("RTX 50") || upper.contains("BLACKWELL") {
                    Some("Removed on RTX 50 series".to_string())
                } else {
                    #[cfg(target_os = "linux")]
                    {
                        Some("Not exposed by NVIDIA Linux driver".to_string())
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        Some("Sensor unavailable".to_string())
                    }
                }
            } else {
                None
            };

            let memory_note = if memory.is_none() {
                let vram_class = detect_vram_class(&name);
                if vram_class == "GDDR6X" || vram_class == "GDDR7" {
                    #[cfg(target_os = "linux")]
                    {
                        Some("Not exposed on this Linux driver".to_string())
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        Some("Sensor unavailable".to_string())
                    }
                } else {
                    Some("No VRAM thermal diode on this model".to_string())
                }
            } else {
                None
            };

            let draw_w = device.power_usage().ok().map(|mw| mw as f32 / 1000.0);
            let active_limit_w = device
                .power_management_limit()
                .ok()
                .map(|mw| mw as f32 / 1000.0);
            let enforced_limit_w = device
                .enforced_power_limit()
                .ok()
                .map(|mw| mw as f32 / 1000.0);
            let default_limit_w = device
                .power_management_limit_default()
                .ok()
                .map(|mw| mw as f32 / 1000.0);
            let (min_limit_w, max_limit_w) = match device.power_management_limit_constraints().ok() {
                Some(c) => (
                    Some(c.min_limit as f32 / 1000.0),
                    Some(c.max_limit as f32 / 1000.0),
                ),
                None => (None, None),
            };
            let (limit_pct_of_default, mode) =
                classify_power_mode(active_limit_w, enforced_limit_w, default_limit_w);

            let reasons = device
                .current_throttle_reasons()
                .unwrap_or(ThrottleReasons::empty());
            let hw_thermal = reasons.contains(ThrottleReasons::HW_THERMAL_SLOWDOWN);
            let sw_thermal = reasons.contains(ThrottleReasons::SW_THERMAL_SLOWDOWN);
            let sw_power_cap = reasons.contains(ThrottleReasons::SW_POWER_CAP);
            let hw_power_brake = reasons.contains(ThrottleReasons::HW_POWER_BRAKE_SLOWDOWN);
            let hw_slowdown = reasons.contains(ThrottleReasons::HW_SLOWDOWN);
            let app_clocks_cap = reasons.contains(ThrottleReasons::APPLICATIONS_CLOCKS_SETTING);
            let display_clock_cap = reasons.contains(ThrottleReasons::DISPLAY_CLOCK_SETTING);
            let gpu_idle = reasons.contains(ThrottleReasons::GPU_IDLE);

            let thermal_from_hw_slowdown = hw_slowdown
                && match (core, hw_slowdown_temp) {
                    (Some(c), Some(s)) => c >= (s - 1.0),
                    _ => false,
                };
            let thermal_throttling = hw_thermal || sw_thermal || thermal_from_hw_slowdown;

            let mut active_reasons = Vec::new();
            if hw_thermal {
                active_reasons.push("HW Thermal Slowdown".to_string());
            }
            if sw_thermal {
                active_reasons.push("SW Thermal Slowdown".to_string());
            }
            if sw_power_cap {
                active_reasons.push("SW Power Cap".to_string());
            }
            if hw_power_brake {
                active_reasons.push("HW Power Brake".to_string());
            }
            if app_clocks_cap {
                active_reasons.push("App Clocks Cap".to_string());
            }
            if display_clock_cap {
                active_reasons.push("Display Clock Cap".to_string());
            }
            if reasons.contains(ThrottleReasons::SYNC_BOOST) {
                active_reasons.push("Sync Boost".to_string());
            }

            let mut sw_caps = Vec::new();
            if let PowerLimitMode::SoftwareLimited { pct, .. } = mode {
                sw_caps.push(format!("Power Limit ({pct:.0}%)"));
            }
            if sw_power_cap {
                sw_caps.push("SW Power Cap Active".to_string());
            }
            if app_clocks_cap {
                sw_caps.push("App Clocks Cap".to_string());
            }
            if sw_thermal {
                sw_caps.push("SW Thermal Limit".to_string());
            }
            let software_cap_summary = if sw_caps.is_empty() {
                "None".to_string()
            } else {
                sw_caps.join(", ")
            };

            let fan_count = device.num_fans().unwrap_or(0);
            let mut per_fan_pct = Vec::new();
            let mut per_fan_rpm = Vec::new();
            if fan_count > 0 {
                for f_idx in 0..fan_count {
                    if let Ok(pct) = device.fan_speed(f_idx) {
                        per_fan_pct.push(pct);
                    }
                    per_fan_rpm.push(device.fan_speed_rpm(f_idx).ok());
                }
            } else if let Ok(pct) = device.fan_speed(0) {
                per_fan_pct.push(pct);
                per_fan_rpm.push(device.fan_speed_rpm(0).ok());
            }

            let speed_pct = if per_fan_pct.is_empty() {
                None
            } else {
                let sum: u32 = per_fan_pct.iter().sum();
                Some(sum / per_fan_pct.len() as u32)
            };
            let rpm = per_fan_rpm
                .iter()
                .flatten()
                .copied()
                .max()
                .or(nvapi_tach_rpm);

            let vram = match device.memory_info() {
                Ok(mem) => VramTelemetry {
                    used_mb: mem.used / (1024 * 1024),
                    total_mb: mem.total / (1024 * 1024),
                },
                Err(_) => VramTelemetry::default(),
            };

            let graphics_mhz = device.clock_info(Clock::Graphics).ok();
            let max_graphics_mhz = device.max_clock_info(Clock::Graphics).ok();
            let memory_mhz = device.clock_info(Clock::Memory).ok();
            let util = device.utilization_rates().ok();
            let pstate = device.performance_state().ok().map(format_pstate);

            samples.push(RawGpuSample {
                key: format!("nv-{pci_bus_id}"),
                index: idx,
                name,
                vendor: GpuVendor::Nvidia,
                pci_bus_id,
                driver_version: driver_version.clone(),
                architecture,
                thermals: ThermalTelemetry {
                    core,
                    hotspot,
                    hotspot_note,
                    memory,
                    memory_note,
                    delta_hotspot_core,
                    hw_target_temp,
                    hw_max_operating_temp,
                    hw_slowdown_temp,
                    hw_shutdown_temp,
                    hw_memory_max_temp,
                    ..Default::default()
                },
                power: PowerTelemetry {
                    draw_w,
                    active_limit_w,
                    enforced_limit_w,
                    default_limit_w,
                    min_limit_w,
                    max_limit_w,
                    limit_pct_of_default,
                    mode,
                },
                throttle: ThrottleTelemetry {
                    thermal_throttling,
                    hw_thermal,
                    sw_thermal,
                    sw_power_cap,
                    hw_power_brake,
                    hw_slowdown,
                    app_clocks_cap,
                    display_clock_cap,
                    gpu_idle,
                    software_cap_summary,
                    active_reasons,
                    ..Default::default()
                },
                fans: FanTelemetry {
                    speed_pct,
                    rpm,
                    fan_count: fan_count.max(per_fan_pct.len() as u32),
                    per_fan_pct,
                    per_fan_rpm,
                },
                vram,
                clocks: ClockTelemetry {
                    graphics_mhz,
                    max_graphics_mhz,
                    memory_mhz,
                    gpu_util_pct: util.as_ref().map(|u| u.gpu),
                    mem_util_pct: util.as_ref().map(|u| u.memory),
                    pstate,
                },
            });
        }

        samples
    }
}

fn query_nvml_memory_temp(device: &nvml_wrapper::Device<'_>) -> Option<f32> {
    let samples = device
        .field_values_for(&[FieldId(NVML_FI_DEV_MEMORY_TEMP)])
        .ok()?;
    for sample in samples {
        let Ok(sample) = sample else { continue };
        if let Ok(val) = sample.value {
            let temp = match val {
                SampleValue::U32(v) => v as f32,
                SampleValue::U64(v) => v as f32,
                SampleValue::I64(v) => v as f32,
                SampleValue::F64(v) => v as f32,
            };
            if (5.0..=155.0).contains(&temp) {
                return Some(temp);
            }
        }
    }
    None
}

fn format_pstate(p: PerformanceState) -> String {
    match p {
        PerformanceState::Zero => "P0".to_string(),
        PerformanceState::One => "P1".to_string(),
        PerformanceState::Two => "P2".to_string(),
        PerformanceState::Three => "P3".to_string(),
        PerformanceState::Four => "P4".to_string(),
        PerformanceState::Five => "P5".to_string(),
        PerformanceState::Six => "P6".to_string(),
        PerformanceState::Seven => "P7".to_string(),
        PerformanceState::Eight => "P8".to_string(),
        PerformanceState::Nine => "P9".to_string(),
        PerformanceState::Ten => "P10".to_string(),
        PerformanceState::Eleven => "P11".to_string(),
        PerformanceState::Twelve => "P12".to_string(),
        PerformanceState::Thirteen => "P13".to_string(),
        PerformanceState::Fourteen => "P14".to_string(),
        PerformanceState::Fifteen => "P15".to_string(),
        PerformanceState::Unknown => "Unknown".to_string(),
    }
}
