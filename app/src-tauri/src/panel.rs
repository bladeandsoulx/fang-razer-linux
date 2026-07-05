//! Internal laptop-panel backlight brightness.
//!
//! A user-session capability (like refresh rate), so it lives in the app.
//! Current level is read from sysfs; changes go through logind's
//! `SetBrightness`, which the active session may call without root and which
//! works on Wayland (GNOME's own Power `Brightness` property reports -1 on
//! hybrid-GPU / external-primary setups, so we don't rely on it).

use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default)]
pub struct PanelInfo {
    pub supported: bool,
    /// Backlight level as a percentage (0..=100).
    pub brightness: u8,
    /// Filled when unsupported.
    pub hint: String,
}

#[cfg(not(target_os = "linux"))]
mod backend {
    use super::PanelInfo;
    use std::sync::Mutex;

    static CURRENT: Mutex<u8> = Mutex::new(80);

    pub fn get() -> PanelInfo {
        PanelInfo {
            supported: true,
            brightness: *CURRENT.lock().unwrap(),
            hint: String::new(),
        }
    }

    pub fn set(percent: u8) -> Result<PanelInfo, String> {
        *CURRENT.lock().unwrap() = percent.clamp(5, 100);
        Ok(get())
    }
}

#[cfg(target_os = "linux")]
mod backend {
    use super::PanelInfo;
    use std::fs;

    /// The internal panel's backlight sysfs device and its max raw level.
    /// Prefer the GPU/ACPI panel controllers over a dGPU's own backlight.
    fn find_backlight() -> Option<(String, u32)> {
        let mut candidates: Vec<(String, u32)> = Vec::new();
        for entry in fs::read_dir("/sys/class/backlight").ok()?.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if let Some(max) = fs::read_to_string(entry.path().join("max_brightness"))
                .ok()
                .and_then(|s| s.trim().parse::<u32>().ok())
                .filter(|m| *m > 0)
            {
                candidates.push((name, max));
            }
        }
        const PREFERRED: [&str; 4] = ["intel_backlight", "amdgpu_bl1", "amdgpu_bl0", "acpi_video0"];
        for p in PREFERRED {
            if let Some(c) = candidates.iter().find(|(n, _)| n == p) {
                return Some(c.clone());
            }
        }
        candidates
            .iter()
            .find(|(n, _)| !n.starts_with("nvidia"))
            .or_else(|| candidates.first())
            .cloned()
    }

    fn raw_level(name: &str) -> Option<u32> {
        fs::read_to_string(format!("/sys/class/backlight/{name}/brightness"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
    }

    pub fn get() -> PanelInfo {
        let Some((name, max)) = find_backlight() else {
            return PanelInfo {
                supported: false,
                brightness: 0,
                hint: "No internal panel backlight found.".into(),
            };
        };
        let cur = raw_level(&name).unwrap_or(0);
        PanelInfo {
            supported: true,
            brightness: ((cur.min(max) * 100 + max / 2) / max) as u8,
            hint: String::new(),
        }
    }

    pub fn set(percent: u8) -> Result<PanelInfo, String> {
        let (name, max) = find_backlight().ok_or("no internal panel backlight")?;
        // Never let the app drive the panel fully dark.
        let pct = percent.clamp(5, 100) as u32;
        let raw = (pct * max / 100).max(1);
        let conn = zbus::blocking::Connection::system().map_err(|e| e.to_string())?;
        conn.call_method(
            Some("org.freedesktop.login1"),
            "/org/freedesktop/login1/session/auto",
            Some("org.freedesktop.login1.Session"),
            "SetBrightness",
            &("backlight", name.as_str(), raw),
        )
        .map_err(|e| format!("SetBrightness: {e}"))?;
        Ok(get())
    }
}

pub fn get() -> PanelInfo {
    backend::get()
}

pub fn set(percent: u8) -> Result<PanelInfo, String> {
    backend::set(percent)
}
