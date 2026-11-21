// SPDX-License-Identifier: MPL-2.0
//
// Windows NVAPI extended thermal sensor & tachometer reader.
//
// Interface IDs, `NvThermalSensors` v2 struct layout, `1 << bit` sensor mask
// discovery, Q8 fixed-point (`/ 256.0`) scaling, and per-generation Hotspot /
// Memory Junction array indices are derived from LibreHardwareMonitor
// (`LibreHardwareMonitorLib/Interop/NvApi.cs` and
// `LibreHardwareMonitorLib/Hardware/Gpu/NvidiaGpu.cs`), licensed under MPL-2.0:
// https://github.com/LibreHardwareMonitor/LibreHardwareMonitor

use std::ffi::c_void;
use std::mem::size_of;

use libloading::Library;

const NVAPI_OK: i32 = 0;
const MAX_PHYSICAL_GPUS: usize = 64;
const THERMAL_SENSOR_RESERVED_COUNT: usize = 8;
const THERMAL_SENSOR_TEMPERATURE_COUNT: usize = 32;

const ID_INITIALIZE: u32 = 0x0150_E828;
const ID_UNLOAD: u32 = 0xD22B_DD7E;
const ID_ENUM_PHYSICAL_GPUS: u32 = 0xE5AC_921F;
const ID_GPU_GET_BUS_ID: u32 = 0x1BE0_B8E5;
const ID_GPU_GET_BUS_SLOT_ID: u32 = 0x2A0A_350F;
const ID_GPU_GET_THERMAL_SENSORS: u32 = 0x65FE_3AAD;
const ID_GPU_GET_TACH_READING: u32 = 0x5F60_8315;

type QueryInterfaceFn = unsafe extern "C" fn(u32) -> *const c_void;
type InitializeFn = unsafe extern "C" fn() -> i32;
type UnloadFn = unsafe extern "C" fn() -> i32;
type EnumPhysicalGpusFn = unsafe extern "C" fn(*mut *mut c_void, *mut i32) -> i32;
type GetBusIdFn = unsafe extern "C" fn(*mut c_void, *mut u32) -> i32;
type GetBusSlotIdFn = unsafe extern "C" fn(*mut c_void, *mut u32) -> i32;
type GetThermalSensorsFn = unsafe extern "C" fn(*mut c_void, *mut NvThermalSensors) -> i32;
type GetTachReadingFn = unsafe extern "C" fn(*mut c_void, *mut i32) -> i32;

#[repr(C)]
struct NvThermalSensors {
    version: u32,
    mask: u32,
    reserved: [i32; THERMAL_SENSOR_RESERVED_COUNT],
    temperatures: [i32; THERMAL_SENSOR_TEMPERATURE_COUNT],
}

impl Default for NvThermalSensors {
    fn default() -> Self {
        Self {
            version: (size_of::<Self>() as u32) | (2 << 16),
            mask: 0,
            reserved: [0; THERMAL_SENSOR_RESERVED_COUNT],
            temperatures: [0; THERMAL_SENSOR_TEMPERATURE_COUNT],
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NvApiExtendedReading {
    pub hotspot_c: Option<f32>,
    pub memory_c: Option<f32>,
    pub tach_rpm: Option<u32>,
}

struct NvApiGpuHandle {
    handle: *mut c_void,
    bus_id: Option<u32>,
    slot_id: Option<u32>,
    thermal_mask: u32,
}

unsafe impl Send for NvApiGpuHandle {}
unsafe impl Send for NvApiSession {}

pub struct NvApiSession {
    _lib: Library,
    unload_fn: Option<UnloadFn>,
    get_thermal_sensors_fn: Option<GetThermalSensorsFn>,
    get_tach_reading_fn: Option<GetTachReadingFn>,
    gpus: Vec<NvApiGpuHandle>,
}

unsafe fn resolve_nvapi<T: Copy>(query: QueryInterfaceFn, id: u32) -> Option<T> {
    let ptr = query(id);
    if ptr.is_null() {
        None
    } else {
        Some(std::mem::transmute_copy::<*const c_void, T>(&ptr))
    }
}

impl NvApiSession {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let lib = Library::new("nvapi64.dll")
                .map_err(|e| format!("Failed to load nvapi64.dll: {e}"))?;

            let query: QueryInterfaceFn = *lib
                .get::<QueryInterfaceFn>(b"nvapi_QueryInterface\0")
                .map_err(|e| format!("Missing nvapi_QueryInterface: {e}"))?;

            let init_fn: InitializeFn = resolve_nvapi(query, ID_INITIALIZE)
                .ok_or_else(|| "Failed to resolve NvAPI_Initialize".to_string())?;
            let status = init_fn();
            if status != NVAPI_OK {
                return Err(format!("NvAPI_Initialize failed with status {status}"));
            }

            let unload_fn: Option<UnloadFn> = resolve_nvapi(query, ID_UNLOAD);
            let enum_gpus_fn: EnumPhysicalGpusFn = resolve_nvapi(query, ID_ENUM_PHYSICAL_GPUS)
                .ok_or_else(|| "Failed to resolve NvAPI_EnumPhysicalGPUs".to_string())?;
            let get_bus_id_fn: Option<GetBusIdFn> = resolve_nvapi(query, ID_GPU_GET_BUS_ID);
            let get_slot_id_fn: Option<GetBusSlotIdFn> =
                resolve_nvapi(query, ID_GPU_GET_BUS_SLOT_ID);
            let get_thermal_sensors_fn: Option<GetThermalSensorsFn> =
                resolve_nvapi(query, ID_GPU_GET_THERMAL_SENSORS);
            let get_tach_reading_fn: Option<GetTachReadingFn> =
                resolve_nvapi(query, ID_GPU_GET_TACH_READING);

            let mut raw_handles = [std::ptr::null_mut::<c_void>(); MAX_PHYSICAL_GPUS];
            let mut count = 0i32;
            let enum_status = enum_gpus_fn(raw_handles.as_mut_ptr(), &mut count);
            if enum_status != NVAPI_OK || count <= 0 {
                if let Some(u) = unload_fn {
                    u();
                }
                return Err(format!(
                    "NvAPI_EnumPhysicalGPUs failed (status={enum_status}, count={count})"
                ));
            }

            let mut gpus = Vec::with_capacity(count as usize);
            for &handle in raw_handles.iter().take((count as usize).min(MAX_PHYSICAL_GPUS)) {
                if handle.is_null() {
                    continue;
                }
                let bus_id = get_bus_id_fn.and_then(|f| {
                    let mut v = 0u32;
                    (f(handle, &mut v) == NVAPI_OK).then_some(v)
                });
                let slot_id = get_slot_id_fn.and_then(|f| {
                    let mut v = 0u32;
                    (f(handle, &mut v) == NVAPI_OK).then_some(v)
                });
                let thermal_mask = match get_thermal_sensors_fn {
                    Some(f) => discover_thermal_mask(handle, f),
                    None => 0,
                };
                gpus.push(NvApiGpuHandle {
                    handle,
                    bus_id,
                    slot_id,
                    thermal_mask,
                });
            }

            Ok(Self {
                _lib: lib,
                unload_fn,
                get_thermal_sensors_fn,
                get_tach_reading_fn,
                gpus,
            })
        }
    }

    pub fn read_extended_sensors(
        &self,
        bus: u32,
        device: u32,
        gpu_name: &str,
        fallback_index: usize,
    ) -> NvApiExtendedReading {
        let gpu = self
            .gpus
            .iter()
            .find(|g| g.bus_id == Some(bus) && g.slot_id == Some(device))
            .or_else(|| self.gpus.iter().find(|g| g.bus_id == Some(bus)))
            .or_else(|| self.gpus.get(fallback_index));

        let Some(gpu) = gpu else {
            return NvApiExtendedReading::default();
        };

        let mut reading = NvApiExtendedReading::default();

        if let (Some(f), true) = (self.get_thermal_sensors_fn, gpu.thermal_mask > 0) {
            let mut sensors = NvThermalSensors {
                mask: gpu.thermal_mask,
                ..Default::default()
            };
            let status = unsafe { f(gpu.handle, &mut sensors) };
            if status == NVAPI_OK {
                let upper = gpu_name.to_ascii_uppercase();
                let (hot_idx, mem_idx) =
                    if upper.contains("RTX 50") || upper.contains("BLACKWELL") {
                        (None, Some(2usize))
                    } else if upper.contains("RTX 40") || upper.contains("ADA") {
                        (Some(1usize), Some(7usize))
                    } else {
                        (Some(1usize), Some(9usize))
                    };

                reading.hotspot_c = hot_idx.and_then(|i| parse_raw_temp(sensors.temperatures[i]));
                reading.memory_c = mem_idx.and_then(|i| parse_raw_temp(sensors.temperatures[i]));
            }
        }

        if let Some(f) = self.get_tach_reading_fn {
            let mut rpm = 0i32;
            let status = unsafe { f(gpu.handle, &mut rpm) };
            if status == NVAPI_OK && rpm >= 0 {
                reading.tach_rpm = Some(rpm as u32);
            }
        }

        reading
    }
}

impl Drop for NvApiSession {
    fn drop(&mut self) {
        if let Some(unload) = self.unload_fn {
            unsafe {
                unload();
            }
        }
    }
}

unsafe fn discover_thermal_mask(handle: *mut c_void, f: GetThermalSensorsFn) -> u32 {
    let mut valid_mask = 0u32;
    for bit in 0..32u32 {
        let probe = 1u32 << bit;
        let mut sensors = NvThermalSensors {
            mask: probe,
            ..Default::default()
        };
        if f(handle, &mut sensors) == NVAPI_OK {
            valid_mask |= probe;
        } else {
            break;
        }
    }
    valid_mask
}

fn parse_raw_temp(raw: i32) -> Option<f32> {
    if raw <= 0 {
        return None;
    }
    let temp_c = raw as f32 / 256.0;
    if (5.0..=155.0).contains(&temp_c) {
        Some(temp_c)
    } else {
        None
    }
}
