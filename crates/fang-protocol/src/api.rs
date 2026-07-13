//! JSON-lines API between `fangd` and clients.
//!
//! Client sends one JSON object per line:
//! `{"id":1,"api_version":1,"cmd":"get_status"}`.
//! Daemon answers `{"id": 1, "ok": true, "data": {...}}` and, after a
//! `subscribe`, pushes `{"event": "telemetry", "data": {...}}` lines.

use serde::{Deserialize, Serialize};

/// Socket API compatibility version. App and daemon may have different patch
/// versions, but state-changing commands are allowed only when this matches.
pub const API_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerfMode {
    Balanced,
    Gaming,
    Silent,
    Custom,
}

impl PerfMode {
    pub fn to_ec(self) -> u8 {
        match self {
            PerfMode::Balanced => 0,
            PerfMode::Gaming => 1,
            // The EC has no silent mode: razer-laptop-control defines only
            // 0, 1, 2 and 4, and sending the undefined 3 trips an EC failsafe
            // with fans at max. Silent rides on Custom; the hardware backend
            // pins both boosts to Low.
            PerfMode::Silent => 4,
            PerfMode::Custom => 4,
        }
    }
}

/// CPU/GPU power boost levels, meaningful in [`PerfMode::Custom`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Boost {
    Low,
    Medium,
    High,
    /// Overclock; CPU only, models with the "boost" feature.
    Boost,
}

impl Boost {
    pub fn to_ec(self) -> u8 {
        match self {
            Boost::Low => 0,
            Boost::Medium => 1,
            Boost::High => 2,
            Boost::Boost => 3,
        }
    }
}

/// One point on a software-controlled fan curve. The daemon interpolates RPM
/// linearly between points using the hotter of the CPU and GPU sensors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FanCurvePoint {
    pub temp_c: u8,
    pub rpm: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum FanMode {
    Auto,
    Manual { rpm: u16 },
    Curve { points: Vec<FanCurvePoint> },
}

/// Why the mandatory software fan guard is forcing maximum RPM.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThermalOverrideReason {
    Temperature,
    SensorUnavailable,
}

/// Logo LED behaviour (models with the "logo" feature).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogoMode {
    Off,
    Static,
    Breathing,
}

/// Keyboard backlight hardware effect. Only effects the reference
/// implementation exercises on real ECs are exposed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "effect", rename_all = "snake_case")]
pub enum KbdEffect {
    Off,
    Static { r: u8, g: u8, b: u8 },
    Spectrum,
    Wave,
}

/// One selectable color-temperature preset on an external DDC/CI monitor
/// (VCP feature 0x14). Laptop eDP panels don't support DDC/CI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColorPreset {
    /// The VCP 0x14 value written over DDC/CI.
    pub value: u8,
    pub name: String,
}

/// Which GPU drives the system (the Linux take on Synapse's "GPU mode" /
/// Advanced Optimus). Switching takes effect at the next logout/reboot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GpuMode {
    /// iGPU only; dGPU powered off (battery life).
    Integrated,
    /// NVIDIA PRIME render offload (on-demand dGPU).
    Hybrid,
    /// dGPU drives everything (lowest latency, external-display friendly).
    Dedicated,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    pub id: u64,
    /// Optional on the wire for read-only backward compatibility. Mutating
    /// commands require an exact match with API_VERSION.
    #[serde(default)]
    pub api_version: u16,
    #[serde(flatten)]
    pub cmd: Command,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Command {
    GetStatus,
    SetPerfMode {
        perf_mode: PerfMode,
        #[serde(default)]
        cpu_boost: Option<Boost>,
        #[serde(default)]
        gpu_boost: Option<Boost>,
    },
    SetFan {
        #[serde(flatten)]
        fan: FanMode,
    },
    /// Switch the active GPU (applies at next logout/reboot).
    SetGpuMode {
        gpu_mode: GpuMode,
    },
    /// Battery Health Optimizer: cap charging at `threshold` percent
    /// (50..=80, clamped) to extend battery lifespan.
    SetBho {
        enabled: bool,
        threshold: u8,
    },
    /// Lighting; omitted fields keep their current value.
    SetLighting {
        /// Keyboard backlight brightness percent (0..=100).
        #[serde(default)]
        brightness: Option<u8>,
        #[serde(default)]
        kbd_effect: Option<KbdEffect>,
        #[serde(default)]
        logo_led: Option<LogoMode>,
    },
    /// Set the external monitor's DDC/CI color-temperature preset (VCP 0x14).
    SetColorPreset {
        value: u8,
    },
    /// Set the external monitor's DDC/CI brightness (VCP 0x10) as a 0..=100
    /// percent of the monitor's own luminance range.
    SetMonitorBrightness {
        value: u8,
    },
    /// Toggle automatic perf-profile switching on AC ↔ battery transitions,
    /// and set the profile + fan applied for each power source.
    SetAutoPower {
        enabled: bool,
        ac_profile: PerfMode,
        battery_profile: PerfMode,
        /// Fan policy applied on AC.
        ac_fan: FanMode,
        /// Fan policy applied on battery.
        battery_fan: FanMode,
    },
    /// Start receiving `telemetry` / `state_changed` events on this connection.
    Subscribe,
    Ping,
}

impl Command {
    pub fn is_mutating(&self) -> bool {
        !matches!(
            self,
            Command::GetStatus | Command::Subscribe | Command::Ping
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Response {
    pub id: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn ok(id: u64, data: impl Serialize) -> Response {
        Response {
            id,
            ok: true,
            data: Some(serde_json::to_value(data).expect("serializable")),
            error: None,
        }
    }

    pub fn err(id: u64, msg: impl Into<String>) -> Response {
        Response {
            id,
            ok: false,
            data: None,
            error: Some(msg.into()),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum Event {
    Telemetry(Telemetry),
    StateChanged(Status),
}

/// Full daemon/device state, returned by `get_status` and on `state_changed`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Status {
    /// Socket API version used for app/daemon compatibility checks.
    #[serde(default)]
    pub api_version: u16,
    pub model: String,
    /// False when no Razer laptop was found (daemon still runs, monitor-only).
    pub device_present: bool,
    /// False when the device's PID is not in the model table; controls still
    /// work but limits are conservative defaults.
    pub verified: bool,
    pub mock: bool,
    pub perf_mode: PerfMode,
    pub cpu_boost: Boost,
    pub gpu_boost: Boost,
    pub fan: FanMode,
    /// Last saved custom curve, retained even while Auto/Manual is active.
    #[serde(default)]
    pub fan_curve: Vec<FanCurvePoint>,
    pub fan_rpm_min: u16,
    pub fan_rpm_max: u16,
    pub has_cpu_boost_oc: bool,
    /// Model supports the Battery Health Optimizer charge limiter.
    pub has_bho: bool,
    pub bho_enabled: bool,
    /// Charge cap percent (50..=80), meaningful when `bho_enabled`.
    pub bho_threshold: u8,
    /// Model has a lid logo LED.
    pub has_logo: bool,
    /// Keyboard backlight brightness percent (0..=100).
    pub kbd_brightness: u8,
    pub kbd_effect: KbdEffect,
    pub logo_led: LogoMode,
    /// An external monitor exposes DDC/CI color-temperature presets.
    pub color_ddc: bool,
    pub color_presets: Vec<ColorPreset>,
    /// Active preset's VCP 0x14 value, if read.
    pub color_current: Option<u8>,
    /// External monitor's brightness (VCP 0x10) as a 0..=100 percent. None
    /// when no monitor, or the monitor doesn't expose luminance over DDC/CI.
    pub monitor_brightness: Option<u8>,
    /// Automatically switch perf profile when AC power is connected/removed.
    pub auto_power: bool,
    /// Profile applied when on AC (meaningful when `auto_power`).
    pub ac_profile: PerfMode,
    /// Profile applied when on battery (meaningful when `auto_power`).
    pub battery_profile: PerfMode,
    /// Fan policy applied on AC alongside `ac_profile`.
    pub ac_fan: FanMode,
    /// Fan policy applied on battery alongside `battery_profile`.
    pub battery_fan: FanMode,
    /// None when no supported GPU-switching tool is available on the host.
    pub gpu_mode: Option<GpuMode>,
    /// True when the GPU mode was changed this boot and needs a
    /// logout/reboot to take effect.
    pub gpu_mode_pending: bool,
    pub daemon_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Telemetry {
    pub cpu_temp_c: Option<f32>,
    pub gpu_temp_c: Option<f32>,
    /// CPU package power draw in watts (RAPL), when readable.
    #[serde(default)]
    pub cpu_power_w: Option<f32>,
    /// GPU power draw in watts (NVML), when the GPU is awake.
    #[serde(default)]
    pub gpu_power_w: Option<f32>,
    /// True on AC, false on battery, None when no AC adapter is exposed
    /// (desktop, or unreadable).
    #[serde(default)]
    pub on_ac: Option<bool>,
    /// EC fan-speed setpoint per fan — Razer laptops expose no live
    /// tachometer, so this is the target, not a measurement. Empty when
    /// unreadable.
    pub fan_rpm: Vec<u32>,
    /// Software-selected target while Manual or Curve control is active.
    #[serde(default)]
    pub fan_target_rpm: Option<u16>,
    /// The mandatory thermal guard has overridden Manual/Curve control and is
    /// forcing the model's maximum fan target.
    #[serde(default)]
    pub thermal_override_active: bool,
    /// The mandatory CPU temperature input has produced a fresh reading.
    #[serde(default)]
    pub thermal_sensor_ok: bool,
    /// Present while `thermal_override_active` explains why max RPM is forced.
    #[serde(default)]
    pub thermal_override_reason: Option<ThermalOverrideReason>,
    pub ts_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_wire_format() {
        let r: Request =
            serde_json::from_str(r#"{"id":3,"cmd":"set_fan","mode":"manual","rpm":4400}"#).unwrap();
        assert_eq!(r.id, 3);
        assert_eq!(r.api_version, 0);
        match r.cmd {
            Command::SetFan {
                fan: FanMode::Manual { rpm },
            } => assert_eq!(rpm, 4400),
            other => panic!("bad parse: {other:?}"),
        }

        let r: Request = serde_json::from_str(
            r#"{"id":8,"cmd":"set_fan","mode":"curve","points":[{"temp_c":45,"rpm":2200},{"temp_c":85,"rpm":5000}]}"#,
        )
        .unwrap();
        match r.cmd {
            Command::SetFan {
                fan: FanMode::Curve { points },
            } => {
                assert_eq!(points.len(), 2);
                assert_eq!(points[1].temp_c, 85);
                assert_eq!(points[1].rpm, 5000);
            }
            other => panic!("bad curve parse: {other:?}"),
        }

        let r: Request = serde_json::from_str(
            r#"{"id":4,"cmd":"set_perf_mode","perf_mode":"custom","cpu_boost":"boost"}"#,
        )
        .unwrap();
        match r.cmd {
            Command::SetPerfMode {
                perf_mode,
                cpu_boost,
                gpu_boost,
            } => {
                assert_eq!(perf_mode, PerfMode::Custom);
                assert_eq!(cpu_boost, Some(Boost::Boost));
                assert_eq!(gpu_boost, None);
            }
            other => panic!("bad parse: {other:?}"),
        }
    }

    #[test]
    fn gpu_mode_wire_format() {
        let r: Request =
            serde_json::from_str(r#"{"id":9,"cmd":"set_gpu_mode","gpu_mode":"dedicated"}"#)
                .unwrap();
        match r.cmd {
            Command::SetGpuMode { gpu_mode } => assert_eq!(gpu_mode, GpuMode::Dedicated),
            other => panic!("bad parse: {other:?}"),
        }
    }

    #[test]
    fn event_wire_format() {
        let e = Event::Telemetry(Telemetry {
            cpu_temp_c: Some(61.5),
            gpu_temp_c: None,
            cpu_power_w: Some(28.4),
            gpu_power_w: None,
            on_ac: Some(true),
            fan_rpm: vec![2300, 2280],
            fan_target_rpm: Some(2300),
            thermal_override_active: false,
            thermal_sensor_ok: true,
            thermal_override_reason: None,
            ts_ms: 12,
        });
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.starts_with(r#"{"event":"telemetry","data":{"#), "{s}");
    }

    #[test]
    fn only_exposed_perf_modes_reach_the_ec() {
        // EC mode 2 is intentionally not exposed, while 3 is undefined.
        for m in [
            PerfMode::Balanced,
            PerfMode::Gaming,
            PerfMode::Silent,
            PerfMode::Custom,
        ] {
            assert!(
                [0, 1, 4].contains(&m.to_ec()),
                "unexpected EC mode for {m:?}"
            );
        }
        // Silent rides on Custom; the backend pins its boosts to Low.
        assert_eq!(PerfMode::Silent.to_ec(), PerfMode::Custom.to_ec());
        assert!(serde_json::from_str::<Request>(
            r#"{"id":10,"cmd":"set_perf_mode","perf_mode":"creator"}"#
        )
        .is_err());
    }

    #[test]
    fn command_mutability_is_explicit() {
        assert!(!Command::GetStatus.is_mutating());
        assert!(!Command::Subscribe.is_mutating());
        assert!(!Command::Ping.is_mutating());
        assert!(Command::SetFan { fan: FanMode::Auto }.is_mutating());
    }
}
