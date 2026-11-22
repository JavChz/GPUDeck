# Third-Party Notices & Attributions

**GPUDeck** source code is licensed under the **Mozilla Public License Version 2.0 (MPL-2.0)** — see [LICENSE](LICENSE).  
All third-party libraries, font software, and referenced external code/interfaces listed below are compatible with MPL-2.0 and remain under their respective original licenses and copyrights.

---

## 1. Direct Rust Crate Dependencies (`Cargo.toml`)

| Crate | Version | License (SPDX) | Repository |
| :--- | :--- | :--- | :--- |
| `eframe` | `0.31.1` | `MIT OR Apache-2.0` | <https://github.com/emilk/egui/tree/master/crates/eframe> |
| `egui` | `0.31.1` | `MIT OR Apache-2.0` | <https://github.com/emilk/egui> |
| `nvml-wrapper` | `0.13.0` | `MIT OR Apache-2.0` | <https://github.com/Cldfire/nvml-wrapper> |
| `serde` | `1.0.229` | `MIT OR Apache-2.0` | <https://github.com/serde-rs/serde> |
| `ureq` | `2.12.1` | `MIT OR Apache-2.0` | <https://github.com/algesten/ureq> |
| `sha2` | `0.10.9` | `MIT OR Apache-2.0` | <https://github.com/RustCrypto/hashes> |
| `libloading` *(Windows)* | `0.8.9` | `ISC` | <https://github.com/nagisa/rust_libloading> |

---

## 2. Notable Transitive Crate Dependencies (`Cargo.lock`)

| Crate | Version | License (SPDX) | Repository / Notes |
| :--- | :--- | :--- | :--- |
| `epaint_default_fonts` | `0.31.1` | `(MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0` | Bundles `egui` fallback fonts (`Hack`, `NotoEmoji`, `Ubuntu-Light`) |
| `tiny-skia` & `tiny-skia-path` | `0.11.4` | `BSD-3-Clause` | <https://github.com/RazrFalcon/tiny-skia> |
| `arrayref` | `0.3.9` | `BSD-2-Clause` | <https://github.com/droundy/arrayref> |
| `hexf-parse` | `0.2.1` | `CC0-1.0` | <https://github.com/lifthrasiir/hexf> |
| `r-efi` | `5.3.0` / `6.0.0` | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | Used under permissive `MIT` / `Apache-2.0` license option (<https://github.com/r-efi/r-efi>) |
| `ring` | `0.17.14` | `Apache-2.0 AND ISC` | <https://github.com/briansmith/ring> |
| `rustls` | `0.23.46` | `Apache-2.0 OR ISC OR MIT` | <https://github.com/rustls/rustls> |
| `webpki-roots` | `0.26.11` / `1.0.12` | `CDLA-Permissive-2.0` | <https://github.com/rustls/webpki-roots> |

All remaining transitive Rust dependencies resolved in `Cargo.lock` are licensed under permissive licenses (`MIT`, `Apache-2.0`, `Zlib`, `BSL-1.0`, `Unicode-3.0`, `Unlicense`, or `0BSD`).

---

## 3. Bundled Font Software (`assets/fonts/`)

The font files in `assets/fonts/` are licensed under the **SIL Open Font License, Version 1.1** — see [assets/fonts/OFL.txt](assets/fonts/OFL.txt):

- **Inter** (`Inter-Regular.ttf`, `Inter-Medium.ttf`, `Inter-SemiBold.ttf`, `Inter-Bold.ttf`)  
  Copyright 2020 The Inter Project Authors (<https://github.com/rsms/inter>)
- **JetBrains Mono** (`JetBrainsMono-Regular.ttf`, `JetBrainsMono-Bold.ttf`)  
  Copyright 2020 The JetBrains Mono Project Authors (<https://github.com/JetBrains/JetBrainsMono>)

---

## 4. External Code & Reverse-Engineered Interface References

### LibreHardwareMonitor (`MPL-2.0`)
- **Repository:** <https://github.com/LibreHardwareMonitor/LibreHardwareMonitor>
- **License:** Mozilla Public License Version 2.0 (`MPL-2.0`)
- **Referenced files:** `LibreHardwareMonitorLib/Interop/NvApi.cs` and `LibreHardwareMonitorLib/Hardware/Gpu/NvidiaGpu.cs`
- **Used in:** [src/telemetry/nvapi_win.rs](src/telemetry/nvapi_win.rs)
- **Specific items referenced:**
  - Reverse-engineered Windows NVAPI `nvapi_QueryInterface` function IDs: `0x0150E828` (`NvAPI_Initialize`), `0xD22BDD7E` (`NvAPI_Unload`), `0xE5AC921F` (`NvAPI_EnumPhysicalGPUs`), `0x1BE0B8E5` (`NvAPI_GPU_GetBusId`), `0x2A0A350F` (`NvAPI_GPU_GetBusSlotId`), `0x65FE3AAD` (`NvAPI_GPU_GetThermalSensors`), and `0x5F608315` (`NvAPI_GPU_GetTachReading`).
  - `NvThermalSensors` v2 `#[repr(C)]` struct layout (`version = sizeof | (2 << 16)`, `mask`, `reserved[8]`, `temperatures[32]`) and Q8 fixed-point `/ 256.0` Celsius scaling.
  - `1 << bit` thermal sensor bitmask probing loop (`discover_thermal_mask`).
  - Per-generation NVIDIA Hotspot and Memory Junction sensor array index mapping (RTX 50xx/Blackwell, RTX 40xx/Ada, and older GTX 10xx / GTX 16xx / RTX 20xx / RTX 30xx architectures).

---

## 5. Runtime System Driver Interfaces (Not Redistributed)

- **NVIDIA Management Library (NVML)** (`nvml.dll` on Windows, `libnvidia-ml.so.1` on Linux) and **NVIDIA NVAPI** (`nvapi64.dll` on Windows) are dynamically loaded at runtime from the user's installed NVIDIA graphics driver; no proprietary NVIDIA binaries or SDK files are bundled or redistributed with GPUDeck.
- **Linux DRM / `hwmon` sysfs** (`/sys/class/drm/card*/device`) is read in read-only mode from the running Linux kernel.
