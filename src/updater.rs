use std::cmp::Ordering;
#[cfg(target_os = "windows")]
use std::fs::{self, File};
#[cfg(target_os = "windows")]
use std::io::{Read, Write};
use std::path::Path;
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering as AtomicOrdering},
    Arc, Mutex,
};
use std::thread;
use std::time::Duration;

use serde::Deserialize;
#[cfg(target_os = "windows")]
use sha2::{Digest, Sha256};

pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_AUTHOR: &str = env!("CARGO_PKG_AUTHORS");
pub const REPO_URL: &str = "https://github.com/JavChz/GPUDeck";
const RELEASES_API_URL: &str = "https://api.github.com/repos/JavChz/GPUDeck/releases/latest";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Option<String>,
}

impl SemVer {
    pub fn parse(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        let no_prefix = trimmed
            .strip_prefix('v')
            .or_else(|| trimmed.strip_prefix('V'))
            .unwrap_or(trimmed);
        let no_build = no_prefix.split('+').next()?.trim();
        let (core, pre) = match no_build.split_once('-') {
            Some((c, p)) => {
                if p.is_empty() {
                    return None;
                }
                (c, Some(p.to_string()))
            }
            None => (no_build, None),
        };
        let mut parts = core.split('.');
        let major = parts.next()?.parse::<u64>().ok()?;
        let minor = parts.next()?.parse::<u64>().ok()?;
        let patch = parts.next()?.parse::<u64>().ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
            pre,
        })
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => cmp_prerelease(a, b),
            })
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn cmp_prerelease(a: &str, b: &str) -> Ordering {
    let mut pa = a.split('.');
    let mut pb = b.split('.');
    loop {
        match (pa.next(), pb.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(ia), Some(ib)) => {
                let ord = match (ia.parse::<u64>(), ib.parse::<u64>()) {
                    (Ok(na), Ok(nb)) => na.cmp(&nb),
                    (Ok(_), Err(_)) => Ordering::Less,
                    (Err(_), Ok(_)) => Ordering::Greater,
                    (Err(_), Err(_)) => ia.cmp(ib),
                };
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub tag: String,
    pub version: String,
    pub html_url: String,
    pub installer_name: Option<String>,
    pub installer_url: Option<String>,
    pub checksums_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate {
        latest_tag: String,
    },
    Available(ReleaseInfo),
    Downloading {
        tag: String,
        downloaded: u64,
        total: Option<u64>,
    },
    ReadyToRestart {
        tag: String,
    },
    Error(String),
}

#[derive(Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<GitHubAsset>,
}

#[derive(Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

struct BusyReset(Arc<AtomicBool>);

impl Drop for BusyReset {
    fn drop(&mut self) {
        self.0.store(false, AtomicOrdering::Release);
    }
}

pub struct UpdateManager {
    status: Arc<Mutex<UpdateStatus>>,
    busy: Arc<AtomicBool>,
    repaint_ctx: Option<egui::Context>,
}

impl UpdateManager {
    pub fn start(repaint_ctx: Option<egui::Context>) -> Self {
        let mgr = Self {
            status: Arc::new(Mutex::new(UpdateStatus::Idle)),
            busy: Arc::new(AtomicBool::new(false)),
            repaint_ctx,
        };
        mgr.check_now();
        mgr
    }

    pub fn status(&self) -> UpdateStatus {
        self.status
            .lock()
            .map(|g| g.clone())
            .unwrap_or(UpdateStatus::Idle)
    }

    pub fn has_update(&self) -> bool {
        matches!(self.status(), UpdateStatus::Available(_))
    }

    pub fn check_now(&self) {
        if self
            .busy
            .compare_exchange(false, true, AtomicOrdering::AcqRel, AtomicOrdering::Acquire)
            .is_err()
        {
            return;
        }

        set_status(&self.status, &self.repaint_ctx, UpdateStatus::Checking);
        let status_arc = Arc::clone(&self.status);
        let busy_arc = Arc::clone(&self.busy);
        let ctx = self.repaint_ctx.clone();

        let _ = thread::Builder::new()
            .name("gpudeck-updater".to_string())
            .spawn(move || {
                let _reset = BusyReset(busy_arc);
                let next = fetch_latest_release();
                set_status(&status_arc, &ctx, next);
            });
    }

    pub fn download_and_install(&self, info: ReleaseInfo) {
        #[cfg(not(target_os = "windows"))]
        {
            open_url(&info.html_url);
        }

        #[cfg(target_os = "windows")]
        {
            let (Some(url), Some(raw_name)) =
                (info.installer_url.clone(), info.installer_name.clone())
            else {
                open_url(&info.html_url);
                return;
            };
            if !url.starts_with("https://") {
                open_url(&info.html_url);
                return;
            }
            if self
                .busy
                .compare_exchange(false, true, AtomicOrdering::AcqRel, AtomicOrdering::Acquire)
                .is_err()
            {
                return;
            }

            let tag = info.tag.clone();
            set_status(
                &self.status,
                &self.repaint_ctx,
                UpdateStatus::Downloading {
                    tag: tag.clone(),
                    downloaded: 0,
                    total: None,
                },
            );

            let status_arc = Arc::clone(&self.status);
            let busy_arc = Arc::clone(&self.busy);
            let ctx = self.repaint_ctx.clone();

            let _ = thread::Builder::new()
                .name("gpudeck-installer".to_string())
                .spawn(move || {
                    let _reset = BusyReset(busy_arc);
                    let next = run_windows_download_and_launch(
                        &tag,
                        &raw_name,
                        &url,
                        info.checksums_url.as_deref(),
                        &status_arc,
                        &ctx,
                    );
                    set_status(&status_arc, &ctx, next);
                });
        }
    }
}

fn set_status(
    status: &Arc<Mutex<UpdateStatus>>,
    ctx: &Option<egui::Context>,
    next: UpdateStatus,
) {
    if let Ok(mut guard) = status.lock() {
        *guard = next;
    }
    if let Some(c) = ctx {
        c.request_repaint();
    }
}

fn build_agent(connect_secs: u64, read_secs: u64) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(connect_secs))
        .timeout_read(Duration::from_secs(read_secs))
        .build()
}

fn user_agent() -> String {
    format!("GPUDeck/{CURRENT_VERSION}")
}

fn fetch_latest_release() -> UpdateStatus {
    let agent = build_agent(10, 12);
    let resp = match agent
        .get(RELEASES_API_URL)
        .set("User-Agent", &user_agent())
        .set("Accept", "application/vnd.github+json")
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => {
            return UpdateStatus::Error("No releases published yet".to_string());
        }
        Err(ureq::Error::Status(403 | 429, _)) => {
            return UpdateStatus::Error("GitHub rate limit reached; try again later".to_string());
        }
        Err(ureq::Error::Status(code, _)) => {
            return UpdateStatus::Error(format!("Update check failed (HTTP {code})"));
        }
        Err(_) => {
            return UpdateStatus::Error("Could not reach GitHub releases".to_string());
        }
    };

    let release: GitHubRelease = match resp.into_json() {
        Ok(r) => r,
        Err(_) => {
            return UpdateStatus::Error("Invalid release metadata from GitHub".to_string());
        }
    };

    let current = SemVer::parse(CURRENT_VERSION);
    let latest = SemVer::parse(&release.tag_name);
    let is_newer = match (current, latest) {
        (Some(cur), Some(lat)) => lat > cur,
        _ => false,
    };

    if !is_newer {
        return UpdateStatus::UpToDate {
            latest_tag: release.tag_name,
        };
    }

    let (installer_name, installer_url) = pick_platform_asset(&release.assets);
    let checksums_url = release
        .assets
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case("SHA256SUMS.txt"))
        .map(|a| a.browser_download_url.clone());

    let version = release
        .tag_name
        .strip_prefix('v')
        .or_else(|| release.tag_name.strip_prefix('V'))
        .unwrap_or(&release.tag_name)
        .to_string();

    UpdateStatus::Available(ReleaseInfo {
        tag: release.tag_name,
        version,
        html_url: release.html_url,
        installer_name,
        installer_url,
        checksums_url,
    })
}

fn pick_platform_asset(assets: &[GitHubAsset]) -> (Option<String>, Option<String>) {
    #[cfg(target_os = "windows")]
    let found = assets
        .iter()
        .find(|a| {
            let lower = a.name.to_ascii_lowercase();
            lower.ends_with(".exe") && lower.contains("setup")
        })
        .or_else(|| {
            assets
                .iter()
                .find(|a| a.name.to_ascii_lowercase().ends_with(".exe"))
        });

    #[cfg(not(target_os = "windows"))]
    let found = assets
        .iter()
        .find(|a| a.name.to_ascii_lowercase().ends_with(".deb"))
        .or_else(|| {
            assets
                .iter()
                .find(|a| a.name.to_ascii_lowercase().ends_with(".tar.gz"))
        });

    match found {
        Some(a) => (Some(a.name.clone()), Some(a.browser_download_url.clone())),
        None => (None, None),
    }
}

#[cfg(target_os = "windows")]
fn run_windows_download_and_launch(
    tag: &str,
    raw_name: &str,
    url: &str,
    checksums_url: Option<&str>,
    status_arc: &Arc<Mutex<UpdateStatus>>,
    ctx: &Option<egui::Context>,
) -> UpdateStatus {
    let Some(safe_name) = Path::new(raw_name).file_name().and_then(|s| s.to_str()) else {
        return UpdateStatus::Error("Invalid installer filename".to_string());
    };
    let update_dir = std::env::temp_dir().join("gpudeck-update");
    if fs::create_dir_all(&update_dir).is_err() {
        return UpdateStatus::Error("Could not create temp update folder".to_string());
    }
    let dest_path = update_dir.join(safe_name);

    let agent = build_agent(10, 30);
    let resp = match agent.get(url).set("User-Agent", &user_agent()).call() {
        Ok(r) => r,
        Err(_) => return UpdateStatus::Error("Failed to download installer".to_string()),
    };

    let total = resp
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok());

    let mut file = match File::create(&dest_path) {
        Ok(f) => f,
        Err(_) => return UpdateStatus::Error("Could not write installer to temp".to_string()),
    };

    let mut reader = resp.into_reader();
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 32 * 1024];
    let mut downloaded = 0u64;
    let mut last_ui_bytes = 0u64;

    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => {
                drop(file);
                let _ = fs::remove_file(&dest_path);
                return UpdateStatus::Error("Download interrupted".to_string());
            }
        };
        if file.write_all(&buf[..n]).is_err() {
            drop(file);
            let _ = fs::remove_file(&dest_path);
            return UpdateStatus::Error("Failed while writing installer".to_string());
        }
        hasher.update(&buf[..n]);
        downloaded += n as u64;
        if downloaded - last_ui_bytes >= 128 * 1024 {
            last_ui_bytes = downloaded;
            set_status(
                status_arc,
                ctx,
                UpdateStatus::Downloading {
                    tag: tag.to_string(),
                    downloaded,
                    total,
                },
            );
        }
    }

    if file.flush().is_err() {
        drop(file);
        let _ = fs::remove_file(&dest_path);
        return UpdateStatus::Error("Failed to flush installer file".to_string());
    }
    drop(file);

    let actual_hex: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    if let Some(cs_url) = checksums_url {
        if !cs_url.starts_with("https://") {
            let _ = fs::remove_file(&dest_path);
            return UpdateStatus::Error("Invalid checksum URL".to_string());
        }
        let Ok(cs_resp) = agent.get(cs_url).set("User-Agent", &user_agent()).call() else {
            let _ = fs::remove_file(&dest_path);
            return UpdateStatus::Error("Failed to fetch SHA256SUMS.txt".to_string());
        };
        let Ok(cs_text) = cs_resp.into_string() else {
            let _ = fs::remove_file(&dest_path);
            return UpdateStatus::Error("Unreadable SHA256SUMS.txt".to_string());
        };
        let Some(expected) = parse_sha256_for_file(&cs_text, safe_name) else {
            let _ = fs::remove_file(&dest_path);
            return UpdateStatus::Error("Installer missing from SHA256SUMS.txt".to_string());
        };
        if !actual_hex.eq_ignore_ascii_case(&expected) {
            let _ = fs::remove_file(&dest_path);
            return UpdateStatus::Error("Checksum verification failed".to_string());
        }
    }

    match Command::new(&dest_path).spawn() {
        Ok(_) => UpdateStatus::ReadyToRestart {
            tag: tag.to_string(),
        },
        Err(_) => UpdateStatus::Error("Failed to launch installer".to_string()),
    }
}

pub fn parse_sha256_for_file(text: &str, filename: &str) -> Option<String> {
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let hash = parts.next()?.trim();
        let rest = parts.next()?.trim().trim_start_matches('*').trim();
        let entry_name = Path::new(rest)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(rest);
        if entry_name == filename
            && hash.len() == 64
            && hash.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Some(hash.to_ascii_lowercase());
        }
    }
    None
}

pub fn open_url(url: &str) {
    if !url.starts_with("https://") {
        return;
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = Command::new("cmd")
            .args(["/C", "start", "", url])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}
