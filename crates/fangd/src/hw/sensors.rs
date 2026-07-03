//! Temperature sources on Linux: hwmon for the CPU package, NVML for the GPU.

use std::fs;
use std::path::PathBuf;

pub struct Sensors {
    cpu_temp_file: Option<PathBuf>,
    nvml: Option<nvml_wrapper::Nvml>,
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
        }
    }

    pub fn temps(&self) -> (Option<f32>, Option<f32>) {
        let cpu = self
            .cpu_temp_file
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| s.trim().parse::<f32>().ok())
            .map(|milli| milli / 1000.0);
        let gpu = self.nvml.as_ref().and_then(|nvml| {
            let dev = nvml.device_by_index(0).ok()?;
            dev.temperature(nvml_wrapper::enum_wrappers::device::TemperatureSensor::Gpu)
                .ok()
                .map(|t| t as f32)
        });
        (cpu, gpu)
    }
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
