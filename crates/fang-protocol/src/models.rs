//! Supported laptop table, keyed by USB product id.
//!
//! Entries verified against razer-laptop-control's laptops.json. Razer ships
//! new PIDs every model year; unknown Blades get [`FALLBACK`] limits and are
//! reported with `verified: false` so the UI can show a heads-up. Add new
//! models here after confirming fan limits.

pub struct LaptopModel {
    pub pid: u16,
    pub name: &'static str,
    pub fan_rpm_min: u16,
    pub fan_rpm_max: u16,
    /// Supports CPU overclock boost level (feature "boost").
    pub has_cpu_boost_oc: bool,
}

pub const MODELS: &[LaptopModel] = &[
    LaptopModel {
        pid: 0x02A0,
        name: "Razer Blade 18 (2023)",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
    },
    // Limits and features from Razer-Control's laptops.json (GPL-2.0),
    // exercised on this hardware: modes, boosts, manual fan, telemetry.
    LaptopModel {
        pid: 0x02B8,
        name: "Razer Blade 18 (2024)",
        fan_rpm_min: 2200,
        fan_rpm_max: 5000,
        has_cpu_boost_oc: true,
    },
];

/// Conservative limits for Razer laptops not (yet) in [`MODELS`].
pub const FALLBACK: LaptopModel = LaptopModel {
    pid: 0x0000,
    name: "Unknown Razer laptop",
    fan_rpm_min: 2200,
    fan_rpm_max: 5000,
    has_cpu_boost_oc: false,
};

pub fn by_pid(pid: u16) -> Option<&'static LaptopModel> {
    MODELS.iter().find(|m| m.pid == pid)
}
