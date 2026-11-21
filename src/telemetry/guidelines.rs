use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricLevel {
    Healthy,
    Warm,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthBadge {
    Healthy,
    Warm,
    Critical,
    Throttling,
}

impl HealthBadge {
    pub fn label(self) -> &'static str {
        match self {
            HealthBadge::Healthy => "HEALTHY",
            HealthBadge::Warm => "WARM",
            HealthBadge::Critical => "CRITICAL",
            HealthBadge::Throttling => "THROTTLING",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TempRange {
    pub healthy_max: f32,
    pub warn_max: f32,
    pub max_spec: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuGuidelines {
    pub core: TempRange,
    pub hotspot: TempRange,
    pub memory: TempRange,
    pub hotspot_delta_warn: f32,
    pub vram_class: String,
}

pub fn detect_vram_class(name: &str) -> &'static str {
    let upper = name.to_ascii_uppercase();
    if upper.contains("RTX 50") || upper.contains("BLACKWELL") {
        return "GDDR7";
    }
    let gddr6x_tokens = [
        "3070 TI",
        "3080",
        "3090",
        "4070",
        "4080",
        "4090",
    ];
    if gddr6x_tokens.iter().any(|tok| upper.contains(tok)) {
        return "GDDR6X";
    }
    if upper.contains("1080 TI") || upper.contains("1080") || upper.contains("TITAN X") {
        return "GDDR5X";
    }
    if upper.contains("GTX 10") {
        return "GDDR5";
    }
    if upper.contains("RTX 20")
        || upper.contains("GTX 16")
        || upper.contains("RTX 30")
        || upper.contains("RTX 40")
        || upper.contains("RX 6")
        || upper.contains("RX 7")
        || upper.contains("RX 9")
        || upper.contains("ARC ")
    {
        return "GDDR6";
    }
    if upper.contains("VEGA") || upper.contains("RADEON VII") || upper.contains("HBM") {
        return "HBM2";
    }
    "GDDR6"
}

pub fn guidelines_for_gpu(
    name: &str,
    hw_target: Option<f32>,
    hw_max_op: Option<f32>,
    hw_slowdown: Option<f32>,
    hw_mem_max: Option<f32>,
) -> GpuGuidelines {
    let upper = name.to_ascii_uppercase();
    let vram_class = detect_vram_class(name).to_string();

    let default_core_max = if upper.contains("GTX 10") { 94.0 } else { 90.0 };
    let core_max_spec = hw_max_op.or(hw_slowdown).unwrap_or(default_core_max).max(70.0);
    let core_healthy = (hw_target.unwrap_or(core_max_spec - 10.0) - 3.0).clamp(75.0, 82.0);
    let core_warn = hw_target
        .unwrap_or(core_max_spec - 6.0)
        .clamp(core_healthy + 4.0, (core_max_spec - 2.0).max(core_healthy + 4.0));

    let (mut mem_healthy, mut mem_warn, mut mem_max_spec): (f32, f32, f32) =
        match vram_class.as_str() {
        "GDDR6X" => (95.0, 105.0, 110.0),
        "GDDR7" => (90.0, 100.0, 105.0),
        _ => (85.0, 95.0, 100.0),
    };

    if let Some(hw_mem) = hw_mem_max {
        if hw_mem > 50.0 {
            mem_max_spec = hw_mem;
            mem_warn = mem_warn.min((mem_max_spec - 3.0).max(60.0));
            mem_healthy = mem_healthy.min((mem_warn - 5.0).max(55.0));
        }
    }

    GpuGuidelines {
        core: TempRange {
            healthy_max: core_healthy,
            warn_max: core_warn,
            max_spec: core_max_spec,
        },
        hotspot: TempRange {
            healthy_max: 95.0,
            warn_max: 105.0,
            max_spec: 110.0,
        },
        memory: TempRange {
            healthy_max: mem_healthy,
            warn_max: mem_warn,
            max_spec: mem_max_spec,
        },
        hotspot_delta_warn: 18.0,
        vram_class,
    }
}

pub fn evaluate_temp_level(temp: f32, range: &TempRange) -> MetricLevel {
    if temp < range.healthy_max {
        MetricLevel::Healthy
    } else if temp < range.warn_max {
        MetricLevel::Warm
    } else {
        MetricLevel::Critical
    }
}

pub struct HealthEvalInput<'a> {
    pub core: Option<f32>,
    pub hotspot: Option<f32>,
    pub memory: Option<f32>,
    pub delta_hotspot_core: Option<f32>,
    pub under_load: bool,
    pub thermal_throttling: bool,
    pub guidelines: &'a GpuGuidelines,
}

pub fn evaluate_overall_health(input: HealthEvalInput<'_>) -> (HealthBadge, String) {
    let max_spec = input.guidelines.core.max_spec.round() as i32;

    if input.thermal_throttling {
        return (
            HealthBadge::Throttling,
            "Thermal throttle active!".to_string(),
        );
    }

    let core_lvl = input
        .core
        .map(|t| evaluate_temp_level(t, &input.guidelines.core))
        .unwrap_or(MetricLevel::Healthy);
    let hot_lvl = input
        .hotspot
        .map(|t| evaluate_temp_level(t, &input.guidelines.hotspot))
        .unwrap_or(MetricLevel::Healthy);
    let mem_lvl = input
        .memory
        .map(|t| evaluate_temp_level(t, &input.guidelines.memory))
        .unwrap_or(MetricLevel::Healthy);

    if core_lvl == MetricLevel::Critical
        || hot_lvl == MetricLevel::Critical
        || mem_lvl == MetricLevel::Critical
    {
        return (
            HealthBadge::Critical,
            format!("Overheating risk, max spec: {}°C", max_spec),
        );
    }

    let high_delta = input.under_load
        && input
            .delta_hotspot_core
            .map(|d| d > input.guidelines.hotspot_delta_warn)
            .unwrap_or(false);

    if core_lvl == MetricLevel::Warm
        || hot_lvl == MetricLevel::Warm
        || mem_lvl == MetricLevel::Warm
        || high_delta
    {
        let reason = if high_delta
            && core_lvl == MetricLevel::Healthy
            && hot_lvl == MetricLevel::Healthy
            && mem_lvl == MetricLevel::Healthy
        {
            format!("High hotspot delta, max spec: {}°C", max_spec)
        } else {
            format!("Check airflow, max spec: {}°C", max_spec)
        };
        return (HealthBadge::Warm, reason);
    }

    (
        HealthBadge::Healthy,
        format!("Within safe range, max spec: {}°C", max_spec),
    )
}
