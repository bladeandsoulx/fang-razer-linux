//! Temperature and power sources on Linux: hwmon for the CPU package
//! temperature, RAPL (powercap) for CPU package power, NVML for the GPU.

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

/// One tick of sensor readings.
#[derive(Clone, Copy, Debug, Default)]
pub struct Readings {
    pub cpu_temp_c: Option<f32>,
    pub gpu_temp_c: Option<f32>,
    pub cpu_power_w: Option<f32>,
    pub gpu_power_w: Option<f32>,
}

pub struct Sensors {
    cpu_temp_file: Option<PathBuf>,
    // One NVML session for the daemon's lifetime. Do NOT cycle
    // init/shutdown per sample: rapid nvmlInit/nvmlShutdown at 1 Hz can
    // livelock inside the NVIDIA driver and wedge the whole daemon.
    nvml: Option<nvml_wrapper::Nvml>,
    nvidia_pm_dir: Option<PathBuf>,
    rapl: Option<Rapl>,
}

/// CPU package power via the RAPL energy counter (root-readable). Power is
/// the energy delta between consecutive samples.
struct Rapl {
    energy_file: PathBuf,
    max_energy_uj: u64,
    last: Option<(u64, Instant)>,
}

impl Rapl {
    fn discover() -> Option<Rapl> {
        for entry in fs::read_dir("/sys/class/powercap").ok()?.flatten() {
            let dir = entry.path();
            // Prefer the MSR-backed zone; skip the duplicate -mmio zone.
            if dir
                .file_name()
                .is_some_and(|n| n.to_string_lossy().contains("mmio"))
            {
                continue;
            }
            let name = fs::read_to_string(dir.join("name")).unwrap_or_default();
            if name.trim() == "package-0" {
                let max_energy_uj = fs::read_to_string(dir.join("max_energy_range_uj"))
                    .ok()?
                    .trim()
                    .parse()
                    .ok()?;
                return Some(Rapl {
                    energy_file: dir.join("energy_uj"),
                    max_energy_uj,
                    last: None,
                });
            }
        }
        None
    }

    fn read_watts(&mut self) -> Option<f32> {
        let uj: u64 = fs::read_to_string(&self.energy_file)
            .ok()?
            .trim()
            .parse()
            .ok()?;
        let now = Instant::now();
        let prev = self.last.replace((uj, now));
        let (prev_uj, prev_t) = prev?;
        let dt = now.duration_since(prev_t).as_secs_f64();
        if dt <= 0.0 {
            return None;
        }
        // The counter wraps at max_energy_range_uj.
        let delta = if uj >= prev_uj {
            uj - prev_uj
        } else {
            self.max_energy_uj - prev_uj + uj
        };
        Some((delta as f64 / dt / 1e6) as f32)
    }
}

impl Sensors {
    pub fn discover() -> Sensors {
        let cpu_temp_file = find_cpu_temp();
        match &cpu_temp_file {
            Some(p) => log::info!("cpu temp source: {}", p.display()),
            None => log::warn!("no coretemp/k10temp hwmon found; cpu temp unavailable"),
        }
        let nvml = match nvml_wrapper::Nvml::init() {
            Ok(n) => {
                log::info!("NVML initialized (nvidia gpu telemetry available)");
                Some(n)
            }
            Err(e) => {
                log::info!("NVML unavailable ({e}); gpu temp disabled");
                None
            }
        };
        Sensors {
            cpu_temp_file,
            nvml,
            nvidia_pm_dir: find_nvidia_pm_dir(),
            rapl: Rapl::discover(),
        }
    }

    pub fn read(&mut self) -> Readings {
        let cpu_temp_c = self
            .cpu_temp_file
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| s.trim().parse::<f32>().ok())
            .map(|milli| milli / 1000.0);
        let cpu_power_w = self.rapl.as_mut().and_then(Rapl::read_watts);
        let (gpu_temp_c, gpu_power_w) = self
            .nvml
            .as_ref()
            .and_then(|nvml| {
                // Gate each query on sysfs runtime-PM state (free to read,
                // never wakes the card): querying a GPU once a second would
                // wake it and reset its autosuspend timer, keeping it out of
                // RTD3 forever. With fine-grained power management the idle
                // NVML session itself doesn't pin the card — only queries do.
                if let Some(dir) = &self.nvidia_pm_dir {
                    let read = |f: &str| {
                        fs::read_to_string(dir.join(f))
                            .unwrap_or_default()
                            .trim()
                            .to_string()
                    };
                    if !should_query_gpu(
                        &read("control"),
                        &read("runtime_status"),
                        &read("runtime_usage"),
                    ) {
                        return None;
                    }
                }
                let dev = nvml.device_by_index(0).ok()?;
                let temp = dev
                    .temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                    .ok()
                    .map(|t| t as f32);
                let power = dev.power_usage().ok().map(|mw| mw as f32 / 1000.0);
                Some((temp, power))
            })
            .unwrap_or((None, None));
        Readings {
            cpu_temp_c,
            gpu_temp_c,
            cpu_power_w,
            gpu_power_w,
        }
    }
}

/// Query only when the card is awake for someone else's sake: with runtime
/// PM enabled (`control == "auto"`), require the device active *and* held by
/// at least one other user, so our sampling never becomes the reason the GPU
/// stays powered. Any other `control` value means runtime PM is off and
/// querying costs nothing.
fn should_query_gpu(control: &str, status: &str, usage: &str) -> bool {
    if control != "auto" {
        return true;
    }
    status == "active" && usage.parse::<u64>().map_or(true, |u| u > 0)
}

/// Locate the NVIDIA VGA controller's runtime-PM directory on the PCI bus.
fn find_nvidia_pm_dir() -> Option<PathBuf> {
    for entry in fs::read_dir("/sys/bus/pci/devices").ok()?.flatten() {
        let dir = entry.path();
        let vendor = fs::read_to_string(dir.join("vendor")).unwrap_or_default();
        let class = fs::read_to_string(dir.join("class")).unwrap_or_default();
        if vendor.trim() == "0x10de" && class.trim().starts_with("0x03") {
            return Some(dir.join("power"));
        }
    }
    None
}

/// Locate the CPU package temperature: hwmon device named coretemp (Intel)
/// or k10temp/zenpower (AMD), preferring the package/Tctl label.
fn find_cpu_temp() -> Option<PathBuf> {
    let hwmon = fs::read_dir("/sys/class/hwmon").ok()?;
    for entry in hwmon.flatten() {
        let dir = entry.path();
        let name = fs::read_to_string(dir.join("name")).unwrap_or_default();
        let name = name.trim();
        if !matches!(name, "coretemp" | "k10temp" | "zenpower") {
            continue;
        }
        // Prefer the package sensor over per-core ones.
        for i in 1..=10 {
            let label = fs::read_to_string(dir.join(format!("temp{i}_label"))).unwrap_or_default();
            let label = label.trim();
            if label.starts_with("Package") || label == "Tctl" || label == "Tdie" {
                return Some(dir.join(format!("temp{i}_input")));
            }
        }
        let first = dir.join("temp1_input");
        if first.exists() {
            return Some(first);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::should_query_gpu;

    #[test]
    fn runtime_pm_off_always_queries() {
        assert!(should_query_gpu("on", "active", "0"));
        assert!(should_query_gpu("", "", ""));
    }

    #[test]
    fn suspended_gpu_is_left_alone() {
        assert!(!should_query_gpu("auto", "suspended", "0"));
        assert!(!should_query_gpu("auto", "suspending", "1"));
    }

    #[test]
    fn idle_but_unclaimed_gpu_is_left_alone() {
        // Active with zero users: the card is coasting toward autosuspend;
        // querying now would reset that timer.
        assert!(!should_query_gpu("auto", "active", "0"));
    }

    #[test]
    fn gpu_in_use_by_others_is_queried() {
        assert!(should_query_gpu("auto", "active", "1"));
        assert!(should_query_gpu("auto", "active", "not-a-number"));
    }
}
