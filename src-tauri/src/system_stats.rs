use serde::Serialize;
use std::sync::Mutex;
use std::time::Instant;
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskStat {
    pub name: String,
    pub used: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSnapshot {
    pub cpu_percent: f32,
    pub cpu_ghz: f32,
    pub cpu_cores: u32,
    pub cpu_name: String,
    pub memory_used: u64,
    pub memory_total: u64,
    pub gpu_name: String,
    pub gpu_percent: Option<f32>,
    pub gpu_memory_total: u64,
    pub net_down_bps: u64,
    pub net_up_bps: u64,
    pub disks: Vec<DiskStat>,
}

struct Sampler {
    system: System,
    disks: Disks,
    networks: Networks,
    last_net: Option<Instant>,
    #[cfg(windows)]
    cpu_times: Option<(u64, u64, u64)>,
    gpu_name: String,
    gpu_memory_total: u64,
    #[cfg(windows)]
    gpu: Option<GpuQuery>,
}

struct SampleState {
    sampler: Sampler,
}

fn sample_state() -> &'static Mutex<SampleState> {
    static STATE: std::sync::OnceLock<Mutex<SampleState>> = std::sync::OnceLock::new();
    STATE.get_or_init(|| Mutex::new(SampleState { sampler: Sampler::new() }))
}

impl Sampler {
    fn new() -> Self {
        let (gpu_name, gpu_memory_total) = gpu_identity();
        Self {
            system: System::new(),
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            last_net: None,
            #[cfg(windows)]
            cpu_times: None,
            gpu_name,
            gpu_memory_total,
            #[cfg(windows)]
            gpu: GpuQuery::open(),
        }
    }

    fn tick(&mut self) -> SystemSnapshot {
        self.system.refresh_cpu_usage();
        self.system.refresh_cpu_frequency();
        self.system.refresh_memory();
        let _ = self.disks.refresh(false);

        let now = Instant::now();
        let elapsed = self
            .last_net
            .map(|then| now.saturating_duration_since(then).as_secs_f64())
            .unwrap_or(0.0);
        self.networks.refresh(false);
        self.last_net = Some(now);

        let cpus = self.system.cpus();
        let cpu_cores = cpus.len() as u32;
        let cpu_percent = overall_cpu_percent(
            &self.system,
            #[cfg(windows)]
            &mut self.cpu_times,
        );
        let freq_mhz = {
            let samples: Vec<u64> = cpus.iter().map(|cpu| cpu.frequency()).filter(|hz| *hz > 0).collect();
            if samples.is_empty() {
                0.0
            } else {
                samples.iter().sum::<u64>() as f32 / samples.len() as f32
            }
        };
        let cpu_name = cpus
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| "CPU".to_string());

        let (net_down_bps, net_up_bps) = network_rates(&self.networks, elapsed);
        let gpu_percent = gpu_usage(
            #[cfg(windows)]
            &mut self.gpu,
        );

        SystemSnapshot {
            cpu_percent,
            cpu_ghz: freq_mhz / 1000.0,
            cpu_cores,
            cpu_name,
            memory_used: self.system.used_memory(),
            memory_total: self.system.total_memory(),
            gpu_name: self.gpu_name.clone(),
            gpu_percent,
            gpu_memory_total: self.gpu_memory_total,
            net_down_bps,
            net_up_bps,
            disks: disk_stats(&self.disks),
        }
    }
}

fn overall_cpu_percent(
    system: &System,
    #[cfg(windows)] cpu_times: &mut Option<(u64, u64, u64)>,
) -> f32 {
    #[cfg(windows)]
    if let Some(percent) = windows_cpu_percent(cpu_times) {
        return percent;
    }
    let cpus = system.cpus();
    if cpus.is_empty() {
        return 0.0;
    }
    let total: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();
    (total / cpus.len() as f32).clamp(0.0, 100.0)
}

/// Whole-processor usage, matching Task Manager. `GetSystemTimes` idle time
/// is included in kernel time, so one busy core does not read as 100%.
#[cfg(windows)]
fn windows_cpu_percent(previous: &mut Option<(u64, u64, u64)>) -> Option<f32> {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::GetSystemTimes;

    fn ticks(time: FILETIME) -> u64 {
        ((time.dwHighDateTime as u64) << 32) | time.dwLowDateTime as u64
    }

    unsafe {
        let mut idle = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)).ok()?;
        let idle = ticks(idle);
        let kernel = ticks(kernel);
        let user = ticks(user);
        let percent = if let Some((prev_idle, prev_kernel, prev_user)) = *previous {
            let idle_delta = idle.saturating_sub(prev_idle);
            let total = kernel.saturating_sub(prev_kernel) + user.saturating_sub(prev_user);
            if total == 0 {
                0.0
            } else {
                let busy = total.saturating_sub(idle_delta);
                (busy as f64 / total as f64 * 100.0) as f32
            }
        } else {
            0.0
        };
        *previous = Some((idle, kernel, user));
        Some(percent.clamp(0.0, 100.0))
    }
}

fn network_rates(networks: &Networks, elapsed: f64) -> (u64, u64) {
    if elapsed < 0.05 {
        return (0, 0);
    }
    let mut down = 0u64;
    let mut up = 0u64;
    for (name, data) in networks.list() {
        let lower = name.to_ascii_lowercase();
        if lower.contains("loopback") || lower == "lo" || lower.contains("isatap") || lower.contains("teredo") {
            continue;
        }
        down = down.saturating_add(data.received());
        up = up.saturating_add(data.transmitted());
    }
    (
        (down as f64 / elapsed) as u64,
        (up as f64 / elapsed) as u64,
    )
}

fn disk_stats(disks: &Disks) -> Vec<DiskStat> {
    #[cfg(windows)]
    {
        let _ = disks;
        return windows_logical_drives();
    }
    #[cfg(not(windows))]
    {
        let mut stats: Vec<DiskStat> = disks
            .list()
            .iter()
            .filter(|disk| disk.total_space() > 0)
            .map(|disk| {
                let total = disk.total_space();
                let available = disk.available_space().min(total);
                DiskStat {
                    name: disk_label(disk),
                    used: total - available,
                    total,
                }
            })
            .filter(|disk| !disk.name.is_empty())
            .collect();
        stats.sort_by(|a, b| a.name.cmp(&b.name));
        stats
    }
}

/// Local drive letters, including virtual disks Windows treats as local.
/// Network locations (mapped shares) are skipped.
#[cfg(windows)]
fn windows_logical_drives() -> Vec<DiskStat> {
    use windows::Win32::Storage::FileSystem::{
        GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives,
    };
    use windows::core::PCWSTR;

    const DRIVE_REMOVABLE: u32 = 2;
    const DRIVE_FIXED: u32 = 3;
    const DRIVE_RAMDISK: u32 = 6;

    let mask = unsafe { GetLogicalDrives() };
    let mut stats = Vec::new();
    for index in 0..26u32 {
        if mask & (1 << index) == 0 {
            continue;
        }
        let letter = (b'A' + index as u8) as u16;
        let path = [letter, u16::from(b':'), u16::from(b'\\'), 0];
        let root = PCWSTR(path.as_ptr());
        let drive_type = unsafe { GetDriveTypeW(root) };
        if drive_type != DRIVE_FIXED && drive_type != DRIVE_REMOVABLE && drive_type != DRIVE_RAMDISK
        {
            continue;
        }
        let mut available = 0u64;
        let mut total = 0u64;
        if unsafe { GetDiskFreeSpaceExW(root, Some(&mut available), Some(&mut total), None) }.is_err()
            || total == 0
        {
            continue;
        }
        let available = available.min(total);
        stats.push(DiskStat {
            name: format!("{}:", char::from(letter as u8)),
            used: total - available,
            total,
        });
    }
    stats
}

#[cfg(not(windows))]
fn disk_label(disk: &sysinfo::Disk) -> String {
    let mount = disk.mount_point().display().to_string();
    let trimmed = mount.trim_end_matches(['\\', '/']).to_string();
    if trimmed.len() == 2 && trimmed.as_bytes().get(1) == Some(&b':') {
        return trimmed;
    }
    let name = disk.name().to_string_lossy().trim().to_string();
    if !name.is_empty() {
        return name;
    }
    trimmed
}

#[cfg(windows)]
struct GpuQuery {
    query: isize,
    counter: isize,
    primed: bool,
}

#[cfg(windows)]
impl GpuQuery {
    fn open() -> Option<Self> {
        use windows::core::w;
        use windows::Win32::System::Performance::{PdhAddEnglishCounterW, PdhOpenQueryW};
        unsafe {
            let mut query = 0isize;
            if PdhOpenQueryW(None, 0, &mut query) != 0 {
                return None;
            }
            let mut counter = 0isize;
            let status = PdhAddEnglishCounterW(
                query,
                w!("\\GPU Engine(*)\\Utilization Percentage"),
                0,
                &mut counter,
            );
            if status != 0 {
                return None;
            }
            Some(Self {
                query,
                counter,
                primed: false,
            })
        }
    }

    fn usage(&mut self) -> Option<f32> {
        use windows::Win32::System::Performance::{
            PdhCollectQueryData, PdhGetFormattedCounterArrayW, PDH_FMT, PDH_FMT_COUNTERVALUE_ITEM_W,
        };
        const PDH_FMT_DOUBLE: u32 = 0x0000_0200;
        const PDH_MORE_DATA: u32 = 0x8000_07D2;
        unsafe {
            if PdhCollectQueryData(self.query) != 0 {
                return None;
            }
            if !self.primed {
                self.primed = true;
                return None;
            }
            let format = PDH_FMT(PDH_FMT_DOUBLE);
            let mut size = 0u32;
            let mut count = 0u32;
            let first = PdhGetFormattedCounterArrayW(self.counter, format, &mut size, &mut count, None);
            if first != PDH_MORE_DATA && first != 0 {
                return None;
            }
            if size == 0 || count == 0 {
                return Some(0.0);
            }
            let mut buffer = vec![0u8; size as usize];
            let status = PdhGetFormattedCounterArrayW(
                self.counter,
                format,
                &mut size,
                &mut count,
                Some(buffer.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W),
            );
            if status != 0 {
                return None;
            }
            let items = std::slice::from_raw_parts(
                buffer.as_ptr() as *const PDH_FMT_COUNTERVALUE_ITEM_W,
                count as usize,
            );
            let mut total = 0.0f64;
            for item in items {
                if item.szName.0.is_null() {
                    continue;
                }
                let name = item.szName.to_string().unwrap_or_default();
                if !name.to_ascii_lowercase().contains("engtype_3d") {
                    continue;
                }
                total += item.FmtValue.Anonymous.doubleValue;
            }
            Some(total.min(100.0) as f32)
        }
    }
}

#[cfg(windows)]
fn gpu_identity() -> (String, u64) {
    use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory};
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory>() else {
            return ("GPU".to_string(), 0);
        };
        for index in 0..8u32 {
            let Ok(adapter) = factory.EnumAdapters(index) else {
                break;
            };
            let Ok(desc) = adapter.GetDesc() else {
                continue;
            };
            let name = String::from_utf16_lossy(&desc.Description);
            let name = name.trim_end_matches('\0').trim();
            if name.is_empty() || name.to_ascii_lowercase().contains("microsoft basic") {
                continue;
            }
            let memory = if desc.DedicatedVideoMemory > 0 {
                desc.DedicatedVideoMemory as u64
            } else {
                desc.SharedSystemMemory as u64
            };
            return (name.to_string(), memory);
        }
    }
    ("GPU".to_string(), 0)
}

#[cfg(not(windows))]
fn gpu_identity() -> (String, u64) {
    ("GPU".to_string(), 0)
}

#[cfg(windows)]
fn gpu_usage(gpu: &mut Option<GpuQuery>) -> Option<f32> {
    gpu.as_mut().and_then(|query| query.usage())
}

#[cfg(not(windows))]
fn gpu_usage() -> Option<f32> {
    None
}

#[tauri::command]
pub fn get_system_stats() -> SystemSnapshot {
    let mut state = sample_state().lock().expect("system stats");
    state.sampler.tick()
}
