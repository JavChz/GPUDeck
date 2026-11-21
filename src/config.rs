use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::ui::theme::ThemeId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppConfig {
    pub theme: ThemeId,
    pub use_fahrenheit: bool,
    pub poll_interval_ms: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemeId::Neo,
            use_fahrenheit: false,
            poll_interval_ms: 1000,
        }
    }
}

fn config_dir_named(app_name: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join(app_name))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|p| p.join(app_name))
    }
}

fn app_config_dir() -> Option<PathBuf> {
    config_dir_named("gpudeck")
}

fn legacy_config_dir() -> Option<PathBuf> {
    config_dir_named("aero-gpu")
}

fn read_with_legacy_fallback(primary: Option<PathBuf>, file_name: &str) -> Option<String> {
    if let Some(path) = primary {
        if let Ok(contents) = fs::read_to_string(path) {
            return Some(contents);
        }
    }
    let legacy_path = legacy_config_dir().map(|d| d.join(file_name))?;
    fs::read_to_string(legacy_path).ok()
}

pub fn config_path() -> Option<PathBuf> {
    app_config_dir().map(|d| d.join("settings.cfg"))
}

pub fn records_path() -> Option<PathBuf> {
    app_config_dir().map(|d| d.join("records.cfg"))
}

impl AppConfig {
    pub fn load() -> Self {
        let Some(contents) = read_with_legacy_fallback(config_path(), "settings.cfg") else {
            return Self::default();
        };
        parse_config(&contents)
    }

    pub fn save(&self) {
        let Some(path) = config_path() else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(path, self.serialize_config());
    }

    pub fn serialize_config(&self) -> String {
        format!(
            "theme={}\nunit={}\ninterval_ms={}\n",
            self.theme.slug(),
            if self.use_fahrenheit { "f" } else { "c" },
            self.poll_interval_ms.clamp(250, 10_000),
        )
    }
}

pub fn parse_config(text: &str) -> AppConfig {
    let mut cfg = AppConfig::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim();
        match key {
            "theme" => {
                if let Some(t) = ThemeId::from_str_loose(val) {
                    cfg.theme = t;
                }
            }
            "unit" => {
                cfg.use_fahrenheit = val.eq_ignore_ascii_case("f");
            }
            "interval_ms" => {
                if let Ok(ms) = val.parse::<u64>() {
                    cfg.poll_interval_ms = ms.clamp(250, 10_000);
                }
            }
            _ => {}
        }
    }
    cfg
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpuTempRecord {
    pub peak_core: Option<f32>,
    pub peak_hotspot: Option<f32>,
    pub peak_memory: Option<f32>,
    pub ever_thermal_throttled: bool,
}

pub fn load_records() -> HashMap<String, GpuTempRecord> {
    let Some(contents) = read_with_legacy_fallback(records_path(), "records.cfg") else {
        return HashMap::new();
    };
    parse_records(&contents)
}

pub fn save_records(records: &HashMap<String, GpuTempRecord>) {
    let Some(path) = records_path() else {
        return;
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, serialize_records(records));
}

pub fn serialize_records(records: &HashMap<String, GpuTempRecord>) -> String {
    let mut keys: Vec<&String> = records.keys().collect();
    keys.sort();
    let mut out = String::new();
    for key in keys {
        let rec = &records[key];
        let fmt_opt = |v: Option<f32>| match v {
            Some(c) => format!("{c:.1}"),
            None => "none".to_string(),
        };
        out.push_str(&format!(
            "{key}|core={}|hotspot={}|memory={}|throttled={}\n",
            fmt_opt(rec.peak_core),
            fmt_opt(rec.peak_hotspot),
            fmt_opt(rec.peak_memory),
            if rec.ever_thermal_throttled { "1" } else { "0" },
        ));
    }
    out
}

pub fn parse_records(text: &str) -> HashMap<String, GpuTempRecord> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split('|');
        let Some(key) = parts.next().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let mut rec = GpuTempRecord::default();
        for field in parts {
            let Some((k, v)) = field.split_once('=') else {
                continue;
            };
            let k = k.trim();
            let v = v.trim();
            let parse_opt_temp = |s: &str| -> Option<f32> {
                if s.eq_ignore_ascii_case("none") {
                    None
                } else {
                    s.parse::<f32>().ok().filter(|t| (0.0..=170.0).contains(t))
                }
            };
            match k {
                "core" => rec.peak_core = parse_opt_temp(v),
                "hotspot" => rec.peak_hotspot = parse_opt_temp(v),
                "memory" => rec.peak_memory = parse_opt_temp(v),
                "throttled" => rec.ever_thermal_throttled = v == "1" || v.eq_ignore_ascii_case("true"),
                _ => {}
            }
        }
        map.insert(key.to_string(), rec);
    }
    map
}
