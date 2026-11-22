# Changelog

All notable changes to **GPUDeck** are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Windows Setup Installer (`GPUDeck-Setup-vX.Y.Z-x86_64.exe`):** Inno Setup 6 modern Next → Next → Finish wizard with license prompt, Start Menu shortcut, optional Desktop shortcut, automatic upgrade process closing, and launch-on-finish option (`installer/gpudeck.iss`).
- **Linux Debian Package (`gpudeck_X.Y.Z_amd64.deb`):** Automated `.deb` packaging with desktop launcher and documentation installed to `/usr/bin/gpudeck`.
- **In-App GitHub Auto-Updater:** Checks GitHub Releases automatically on launch (pure-Rust `ureq` + `sha2`), downloads the Windows installer in the background with live progress, verifies its SHA-256 hash against `SHA256SUMS.txt`, and launches the upgrade wizard (`src/updater.rs`).
- **About & Updates Section (`(i)` modal):** Displays application version (`vX.Y.Z`), author (`Javier Garcia`), MPL-2.0 license, GitHub repository link, live update status/actions (`Check Updates`, `Download & Install`, `Release Notes`), and an update indicator dot on the header `(i)` button.

### Changed
- **2-Font Typography System:** Standardized strictly on **Inter** (Bold/SemiBold uppercase titles and Regular/Medium UI/body text) paired with **JetBrains Mono** selectively for tabular numeric readouts, gauge thresholds, chart axes, and tooltip metrics.
- **UI & Header Refinements:** Removed the `LIVE` header chip in favor of a clean GPU/driver summary, and standardized ALL-CAPS styling across all card, dial, chart, modal, and tooltip titles.
- **Frameless Modals & Compact Hover Tooltips:** Redesigned **Settings** (miniature live theme previews, custom segmented controls, backdrop scrim dismissal) and **About & Guidelines** modals, and tightened hover tooltip sizing, dividers, and right-aligned value columns.

## [0.1.0] - 2026-11-21

### Added
- **Cross-Platform Multi-GPU Live Grid:** Simultaneous monitoring of all detected GPUs without switching tabs using NVIDIA NVML, Windows `nvapi64.dll` extended thermal sensors (Hotspot & VRAM Memory Junction), and Linux DRM/`hwmon` sysfs (`amdgpu`, `i915`, `xe`).
- **Thermal Guidelines & Power/Throttling Diagnostics:** Per-GPU safe temperature ranges (`Core`, `Hotspot`, `Memory` by VRAM class), `ΔT` hotspot delta indicator, `Spec` tooltip, power ceiling vs. default TDP detection (`100%`, `SW Limit`, `Unlocked`), and active NVML/sysfs throttle reasons.
- **Persistent Max Temperature Records:** Tracks peak Core, Hotspot, Memory temperatures and past throttling events across sessions (`records.cfg`) with high-watermark dial ticks and per-GPU / global **Reset** buttons.
- **9 Built-In Themes:** `Neo`, `Frutiger Aero`, `Kawaii`, `Japandi`, `Minimalist`, `Modern`, `Win-Like`, `Terminal`, and `Corporate`, persisted in `settings.cfg`.
- **2-Row Concept-Grouped Telemetry Sparklines & Anti-Aliased Gauges:** Live history for **Thermals & Cooling** (`CORE / HOTSPOT`, `MEMORY TEMP`, `FANS`) and **Workload & Power** (`GPU USAGE`, `VRAM USAGE`, `POWER DRAW`), rendered with sub-pixel feathered gauge arcs.
- **GitHub Actions CI & Versioned Releases:** Cross-platform CI checks and tagged `v*.*.*` release builds, MPL-2.0 license, and third-party attributions (`THIRD_PARTY_NOTICES.md`).

[Unreleased]: https://github.com/JavChz/GPUDeck/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/JavChz/GPUDeck/releases/tag/v0.1.0
