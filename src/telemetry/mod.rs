pub mod guidelines;
#[cfg(target_os = "linux")]
pub mod linux_sysfs;
#[cfg(target_os = "windows")]
pub mod nvapi_win;
pub mod nvidia;

use std::collections::{HashMap, VecDeque};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Condvar, Mutex, RwLock,
};
use std::thread;
use std::time::{Duration, Instant};

use guidelines::{
    evaluate_overall_health, guidelines_for_gpu, GpuGuidelines, HealthBadge, HealthEvalInput,
};

pub const HISTORY_CAPACITY: usize = 120;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuVendor {
    Nvidia,
    Amd,
    Intel,
    Other,
}

#[allow(dead_code)]
impl GpuVendor {
    pub fn label(self) -> &'static str {
        match self {
            GpuVendor::Nvidia => "NVIDIA",
            GpuVendor::Amd => "AMD",
            GpuVendor::Intel => "Intel",
            GpuVendor::Other => "GPU",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ThermalTelemetry {
    pub core: Option<f32>,
    pub hotspot: Option<f32>,
    pub hotspot_note: Option<String>,
    pub memory: Option<f32>,
    pub memory_note: Option<String>,
    pub delta_hotspot_core: Option<f32>,
    pub hw_target_temp: Option<f32>,
    pub hw_max_operating_temp: Option<f32>,
    pub hw_slowdown_temp: Option<f32>,
    pub hw_shutdown_temp: Option<f32>,
    pub hw_memory_max_temp: Option<f32>,
    pub peak_core: Option<f32>,
    pub peak_hotspot: Option<f32>,
    pub peak_memory: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PowerLimitMode {
    Full100Percent,
    SoftwareLimited {
        pct: f32,
        active_w: f32,
        default_w: f32,
    },
    OverclockedLimit {
        pct: f32,
        active_w: f32,
        default_w: f32,
    },
    Unknown,
}

#[derive(Debug, Clone)]
pub struct PowerTelemetry {
    pub draw_w: Option<f32>,
    pub active_limit_w: Option<f32>,
    pub enforced_limit_w: Option<f32>,
    pub default_limit_w: Option<f32>,
    pub min_limit_w: Option<f32>,
    pub max_limit_w: Option<f32>,
    pub limit_pct_of_default: Option<f32>,
    pub mode: PowerLimitMode,
}

impl Default for PowerTelemetry {
    fn default() -> Self {
        Self {
            draw_w: None,
            active_limit_w: None,
            enforced_limit_w: None,
            default_limit_w: None,
            min_limit_w: None,
            max_limit_w: None,
            limit_pct_of_default: None,
            mode: PowerLimitMode::Unknown,
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct ThrottleTelemetry {
    pub thermal_throttling: bool,
    pub hw_thermal: bool,
    pub sw_thermal: bool,
    pub sw_power_cap: bool,
    pub hw_power_brake: bool,
    pub hw_slowdown: bool,
    pub app_clocks_cap: bool,
    pub display_clock_cap: bool,
    pub gpu_idle: bool,
    pub software_cap_summary: String,
    pub active_reasons: Vec<String>,
    pub ever_thermal_throttled: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct FanTelemetry {
    pub speed_pct: Option<u32>,
    pub rpm: Option<u32>,
    pub fan_count: u32,
    pub per_fan_pct: Vec<u32>,
    pub per_fan_rpm: Vec<Option<u32>>,
}

#[derive(Debug, Clone, Default)]
pub struct VramTelemetry {
    pub used_mb: u64,
    pub total_mb: u64,
}

impl VramTelemetry {
    pub fn used_gb(&self) -> f32 {
        self.used_mb as f32 / 1024.0
    }

    pub fn total_gb(&self) -> f32 {
        self.total_mb as f32 / 1024.0
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClockTelemetry {
    pub graphics_mhz: Option<u32>,
    pub max_graphics_mhz: Option<u32>,
    pub memory_mhz: Option<u32>,
    pub gpu_util_pct: Option<u32>,
    pub mem_util_pct: Option<u32>,
    pub pstate: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GpuHistory {
    pub core_temp: VecDeque<f32>,
    pub hotspot_temp: VecDeque<f32>,
    pub memory_temp: VecDeque<f32>,
    pub fan_pct: VecDeque<f32>,
    pub fan_rpm: VecDeque<f32>,
    pub vram_used_gb: VecDeque<f32>,
    pub gpu_util_pct: VecDeque<f32>,
    pub power_w: VecDeque<f32>,
}

impl Default for GpuHistory {
    fn default() -> Self {
        Self {
            core_temp: VecDeque::with_capacity(HISTORY_CAPACITY),
            hotspot_temp: VecDeque::with_capacity(HISTORY_CAPACITY),
            memory_temp: VecDeque::with_capacity(HISTORY_CAPACITY),
            fan_pct: VecDeque::with_capacity(HISTORY_CAPACITY),
            fan_rpm: VecDeque::with_capacity(HISTORY_CAPACITY),
            vram_used_gb: VecDeque::with_capacity(HISTORY_CAPACITY),
            gpu_util_pct: VecDeque::with_capacity(HISTORY_CAPACITY),
            power_w: VecDeque::with_capacity(HISTORY_CAPACITY),
        }
    }
}

impl GpuHistory {
    pub fn push_bounded(buf: &mut VecDeque<f32>, val: f32) {
        if buf.len() >= HISTORY_CAPACITY {
            buf.pop_front();
        }
        buf.push_back(val);
    }

    pub fn push_sample(&mut self, sample: &RawGpuSample) {
        if let Some(c) = sample.thermals.core {
            Self::push_bounded(&mut self.core_temp, c);
        }
        if let Some(h) = sample.thermals.hotspot {
            Self::push_bounded(&mut self.hotspot_temp, h);
        }
        if let Some(m) = sample.thermals.memory {
            Self::push_bounded(&mut self.memory_temp, m);
        }
        if let Some(pct) = sample.fans.speed_pct {
            Self::push_bounded(&mut self.fan_pct, pct as f32);
        }
        if let Some(rpm) = sample.fans.rpm {
            Self::push_bounded(&mut self.fan_rpm, rpm as f32);
        }
        Self::push_bounded(&mut self.vram_used_gb, sample.vram.used_gb());
        if let Some(u) = sample.clocks.gpu_util_pct {
            Self::push_bounded(&mut self.gpu_util_pct, u as f32);
        }
        if let Some(w) = sample.power.draw_w {
            Self::push_bounded(&mut self.power_w, w);
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawGpuSample {
    pub key: String,
    pub index: u32,
    pub name: String,
    pub vendor: GpuVendor,
    pub pci_bus_id: String,
    pub driver_version: Option<String>,
    pub architecture: Option<String>,
    pub thermals: ThermalTelemetry,
    pub power: PowerTelemetry,
    pub throttle: ThrottleTelemetry,
    pub fans: FanTelemetry,
    pub vram: VramTelemetry,
    pub clocks: ClockTelemetry,
}

#[derive(Debug, Clone)]
pub struct GpuSnapshot {
    pub key: String,
    pub index: u32,
    pub name: String,
    pub vendor: GpuVendor,
    pub pci_bus_id: String,
    pub driver_version: Option<String>,
    pub architecture: Option<String>,
    pub thermals: ThermalTelemetry,
    pub power: PowerTelemetry,
    pub throttle: ThrottleTelemetry,
    pub fans: FanTelemetry,
    pub vram: VramTelemetry,
    pub clocks: ClockTelemetry,
    pub guidelines: GpuGuidelines,
    pub health_badge: HealthBadge,
    pub health_subtitle: String,
    pub history: GpuHistory,
}

pub trait GpuBackend: Send {
    fn name(&self) -> &'static str;
    fn poll(&mut self) -> Vec<RawGpuSample>;
    fn diagnostics(&self) -> Option<String> {
        None
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct TelemetryState {
    pub gpus: Vec<GpuSnapshot>,
    pub driver_version: Option<String>,
    pub poll_interval_ms: u64,
    pub backend_notes: Vec<String>,
    pub last_updated: Option<Instant>,
}

#[derive(Debug, Default)]
struct PersistentGpuStats {
    history: GpuHistory,
    peak_core: Option<f32>,
    peak_hotspot: Option<f32>,
    peak_memory: Option<f32>,
    ever_thermal_throttled: bool,
}

impl PersistentGpuStats {
    fn clear_peaks(&mut self) {
        self.peak_core = None;
        self.peak_hotspot = None;
        self.peak_memory = None;
        self.ever_thermal_throttled = false;
    }
}

pub fn normalize_pci_id(id: &str) -> String {
    id.trim().to_ascii_lowercase()
}

pub fn classify_power_mode(
    active_limit_w: Option<f32>,
    enforced_limit_w: Option<f32>,
    default_limit_w: Option<f32>,
) -> (Option<f32>, PowerLimitMode) {
    let effective = enforced_limit_w.or(active_limit_w);
    match (effective, default_limit_w) {
        (Some(active), Some(default)) if default > 1.0 => {
            let pct = (active / default) * 100.0;
            let mode = if pct < 99.0 {
                PowerLimitMode::SoftwareLimited {
                    pct,
                    active_w: active,
                    default_w: default,
                }
            } else if pct > 101.0 {
                PowerLimitMode::OverclockedLimit {
                    pct,
                    active_w: active,
                    default_w: default,
                }
            } else {
                PowerLimitMode::Full100Percent
            };
            (Some(pct), mode)
        }
        _ => (None, PowerLimitMode::Unknown),
    }
}

enum ResetTarget {
    All,
    Single(String),
}

pub struct TelemetryManager {
    state: Arc<RwLock<TelemetryState>>,
    poll_interval_ms: Arc<AtomicU64>,
    reset_requests: Arc<Mutex<Vec<ResetTarget>>>,
    stop_flag: Arc<AtomicBool>,
    wakeup: Arc<(Mutex<u64>, Condvar)>,
    worker: Option<thread::JoinHandle<()>>,
}

impl TelemetryManager {
    pub fn start(initial_interval_ms: u64, repaint_ctx: Option<egui::Context>) -> Self {
        let interval = initial_interval_ms.clamp(250, 10_000);
        let state = Arc::new(RwLock::new(TelemetryState {
            poll_interval_ms: interval,
            ..Default::default()
        }));
        let poll_interval_ms = Arc::new(AtomicU64::new(interval));
        let reset_requests = Arc::new(Mutex::new(Vec::new()));
        let stop_flag = Arc::new(AtomicBool::new(false));
        let wakeup = Arc::new((Mutex::new(0u64), Condvar::new()));

        let s_clone = Arc::clone(&state);
        let int_clone = Arc::clone(&poll_interval_ms);
        let reset_clone = Arc::clone(&reset_requests);
        let stop_clone = Arc::clone(&stop_flag);
        let wake_clone = Arc::clone(&wakeup);

        let worker = thread::Builder::new()
            .name("gpudeck-telemetry".to_string())
            .spawn(move || {
                let mut live_backends = create_live_backends();
                let mut persistent: HashMap<String, PersistentGpuStats> = crate::config::load_records()
                    .into_iter()
                    .map(|(k, rec)| {
                        (
                            k,
                            PersistentGpuStats {
                                history: GpuHistory::default(),
                                peak_core: rec.peak_core,
                                peak_hotspot: rec.peak_hotspot,
                                peak_memory: rec.peak_memory,
                                ever_thermal_throttled: rec.ever_thermal_throttled,
                            },
                        )
                    })
                    .collect();
                let mut last_seq = 0u64;

                while !stop_clone.load(Ordering::Relaxed) {
                    let mut records_dirty = false;
                    let drained: Vec<ResetTarget> = reset_clone
                        .lock()
                        .map(|mut q| std::mem::take(&mut *q))
                        .unwrap_or_default();

                    for target in drained {
                        records_dirty = true;
                        match target {
                            ResetTarget::All => {
                                for stats in persistent.values_mut() {
                                    stats.clear_peaks();
                                }
                            }
                            ResetTarget::Single(key) => {
                                if let Some(stats) = persistent.get_mut(&key) {
                                    stats.clear_peaks();
                                }
                            }
                        }
                    }

                    let (samples, notes) = collect_live_samples(&mut live_backends);

                    let (snapshots, peak_bumped) = build_snapshots(samples, &mut persistent);
                    if peak_bumped {
                        records_dirty = true;
                    }
                    if records_dirty {
                        let to_save = persistent
                            .iter()
                            .map(|(k, st)| {
                                (
                                    k.clone(),
                                    crate::config::GpuTempRecord {
                                        peak_core: st.peak_core,
                                        peak_hotspot: st.peak_hotspot,
                                        peak_memory: st.peak_memory,
                                        ever_thermal_throttled: st.ever_thermal_throttled,
                                    },
                                )
                            })
                            .collect();
                        crate::config::save_records(&to_save);
                    }

                    let driver_version = snapshots.iter().find_map(|g| g.driver_version.clone());
                    let interval_now = int_clone.load(Ordering::Relaxed);

                    if let Ok(mut guard) = s_clone.write() {
                        guard.gpus = snapshots;
                        guard.driver_version = driver_version;
                        guard.poll_interval_ms = interval_now;
                        guard.backend_notes = notes;
                        guard.last_updated = Some(Instant::now());
                    }

                    if let Some(ctx) = &repaint_ctx {
                        ctx.request_repaint();
                    }

                    let wait_dur = Duration::from_millis(interval_now);
                    let (lock, cvar) = &*wake_clone;
                    if let Ok(guard) = lock.lock() {
                        let _ = cvar.wait_timeout_while(guard, wait_dur, |seq| {
                            *seq == last_seq && !stop_clone.load(Ordering::Relaxed)
                        });
                        last_seq = *lock.lock().unwrap_or_else(|e| e.into_inner());
                    }
                }
            })
            .ok();

        Self {
            state,
            poll_interval_ms,
            reset_requests,
            stop_flag,
            wakeup,
            worker,
        }
    }

    pub fn snapshot(&self) -> TelemetryState {
        self.state
            .read()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    pub fn set_poll_interval_ms(&self, ms: u64) {
        let clamped = ms.clamp(250, 10_000);
        self.poll_interval_ms.store(clamped, Ordering::Relaxed);
        self.notify_worker();
    }

    pub fn reset_peaks(&self) {
        if let Ok(mut q) = self.reset_requests.lock() {
            q.push(ResetTarget::All);
        }
        self.notify_worker();
    }

    pub fn reset_gpu_peaks(&self, gpu_key: &str) {
        if let Ok(mut q) = self.reset_requests.lock() {
            q.push(ResetTarget::Single(gpu_key.to_string()));
        }
        self.notify_worker();
    }

    pub fn force_refresh(&self) {
        self.notify_worker();
    }

    fn notify_worker(&self) {
        let (lock, cvar) = &*self.wakeup;
        if let Ok(mut seq) = lock.lock() {
            *seq = seq.wrapping_add(1);
            cvar.notify_all();
        }
    }

    pub fn probe_once() -> (Vec<GpuSnapshot>, Vec<String>) {
        let mut backends = create_live_backends();
        let (samples, notes) = collect_live_samples(&mut backends);
        let mut persistent = HashMap::new();
        let (snaps, _) = build_snapshots(samples, &mut persistent);
        (snaps, notes)
    }
}

impl Drop for TelemetryManager {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        self.notify_worker();
        if let Some(handle) = self.worker.take() {
            let _ = handle.join();
        }
    }
}

fn create_live_backends() -> Vec<Box<dyn GpuBackend>> {
    let mut backends: Vec<Box<dyn GpuBackend>> = Vec::new();
    backends.push(Box::new(nvidia::NvidiaBackend::new()));
    #[cfg(target_os = "linux")]
    {
        backends.push(Box::new(linux_sysfs::LinuxSysfsBackend::new()));
    }
    backends
}

fn collect_live_samples(backends: &mut [Box<dyn GpuBackend>]) -> (Vec<RawGpuSample>, Vec<String>) {
    let mut all = Vec::new();
    let mut notes = Vec::new();
    let mut seen_pci: Vec<String> = Vec::new();

    for backend in backends.iter_mut() {
        if let Some(diag) = backend.diagnostics() {
            notes.push(format!("{}: {}", backend.name(), diag));
        }
        for mut sample in backend.poll() {
            let norm_pci = normalize_pci_id(&sample.pci_bus_id);
            if !norm_pci.is_empty() && seen_pci.iter().any(|p| p == &norm_pci) {
                continue;
            }
            if !norm_pci.is_empty() {
                seen_pci.push(norm_pci);
            }
            sample.index = all.len() as u32;
            all.push(sample);
        }
    }
    (all, notes)
}

fn update_max(slot: &mut Option<f32>, val: Option<f32>) -> bool {
    if let Some(v) = val {
        let dirty = match *slot {
            Some(prev) => v >= prev + 0.1,
            None => true,
        };
        *slot = Some(match *slot {
            Some(prev) => prev.max(v),
            None => v,
        });
        dirty
    } else {
        false
    }
}

fn build_snapshots(
    samples: Vec<RawGpuSample>,
    persistent: &mut HashMap<String, PersistentGpuStats>,
) -> (Vec<GpuSnapshot>, bool) {
    let mut any_dirty = false;
    let snaps = samples
        .into_iter()
        .map(|mut sample| {
            let stats = persistent.entry(sample.key.clone()).or_default();
            stats.history.push_sample(&sample);

            any_dirty |= update_max(&mut stats.peak_core, sample.thermals.core);
            any_dirty |= update_max(&mut stats.peak_hotspot, sample.thermals.hotspot);
            any_dirty |= update_max(&mut stats.peak_memory, sample.thermals.memory);
            if sample.throttle.thermal_throttling && !stats.ever_thermal_throttled {
                stats.ever_thermal_throttled = true;
                any_dirty = true;
            }

            sample.thermals.peak_core = stats.peak_core;
            sample.thermals.peak_hotspot = stats.peak_hotspot;
            sample.thermals.peak_memory = stats.peak_memory;
            sample.throttle.ever_thermal_throttled = stats.ever_thermal_throttled;

            let guidelines = guidelines_for_gpu(
                &sample.name,
                sample.thermals.hw_target_temp,
                sample.thermals.hw_max_operating_temp,
                sample.thermals.hw_slowdown_temp,
                sample.thermals.hw_memory_max_temp,
            );

            let under_load = sample
                .clocks
                .gpu_util_pct
                .map(|u| u >= 25)
                .unwrap_or_else(|| sample.thermals.core.map(|c| c >= 55.0).unwrap_or(false));

            let (health_badge, health_subtitle) = evaluate_overall_health(HealthEvalInput {
                core: sample.thermals.core,
                hotspot: sample.thermals.hotspot,
                memory: sample.thermals.memory,
                delta_hotspot_core: sample.thermals.delta_hotspot_core,
                under_load,
                thermal_throttling: sample.throttle.thermal_throttling,
                guidelines: &guidelines,
            });

            GpuSnapshot {
                key: sample.key,
                index: sample.index,
                name: sample.name,
                vendor: sample.vendor,
                pci_bus_id: sample.pci_bus_id,
                driver_version: sample.driver_version,
                architecture: sample.architecture,
                thermals: sample.thermals,
                power: sample.power,
                throttle: sample.throttle,
                fans: sample.fans,
                vram: sample.vram,
                clocks: sample.clocks,
                guidelines,
                health_badge,
                health_subtitle,
                history: stats.history.clone(),
            }
        })
        .collect();
    (snaps, any_dirty)
}
