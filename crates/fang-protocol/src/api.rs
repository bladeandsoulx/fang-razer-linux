//! JSON-lines API between `fangd` and clients.
//!
//! Client sends one JSON object per line: `{"id": 1, "cmd": "get_status"}`.
//! Daemon answers `{"id": 1, "ok": true, "data": {...}}` and, after a
//! `subscribe`, pushes `{"event": "telemetry", "data": {...}}` lines.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PerfMode {
    Balanced,
    Gaming,
    Creator,
    Silent,
    Custom,
}

impl PerfMode {
    pub fn to_ec(self) -> u8 {
        match self {
            PerfMode::Balanced => 0,
            PerfMode::Gaming => 1,
            PerfMode::Creator => 2,
            // The EC has no silent mode: razer-laptop-control defines only
            // 0..=2 and 4, and sending the undefined 3 puts at least the
            // pid 02b8 EC into a failsafe with fans at max. Silent rides on
            // Custom; the hardware backend pins both boosts to Low.
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum FanMode {
    Auto,
    Manual { rpm: u16 },
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
    /// Start receiving `telemetry` / `state_changed` events on this connection.
    Subscribe,
    Ping,
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
    pub fan_rpm_min: u16,
    pub fan_rpm_max: u16,
    pub has_cpu_boost_oc: bool,
    /// Model's EC defines power mode 2 (Creator); the UI hides the mode and
    /// the daemon rejects it otherwise.
    pub has_creator_mode: bool,
    /// Model supports the Battery Health Optimizer charge limiter.
    pub has_bho: bool,
    pub bho_enabled: bool,
    /// Charge cap percent (50..=80), meaningful when `bho_enabled`.
    pub bho_threshold: u8,
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
    /// Measured RPM per fan (empty when unreadable).
    pub fan_rpm: Vec<u32>,
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
        match r.cmd {
            Command::SetFan {
                fan: FanMode::Manual { rpm },
            } => assert_eq!(rpm, 4400),
            other => panic!("bad parse: {other:?}"),
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
            fan_rpm: vec![2300, 2280],
            ts_ms: 12,
        });
        let s = serde_json::to_string(&e).unwrap();
        assert!(s.starts_with(r#"{"event":"telemetry","data":{"#), "{s}");
    }

    #[test]
    fn no_perf_mode_emits_the_undefined_ec_value() {
        // 3 is not a valid EC power mode (razer-laptop-control defines only
        // 0..=2 and 4); ECs answer it with a max-fan failsafe.
        for m in [
            PerfMode::Balanced,
            PerfMode::Gaming,
            PerfMode::Creator,
            PerfMode::Silent,
            PerfMode::Custom,
        ] {
            assert_ne!(m.to_ec(), 3, "{m:?} must not reach the EC as 3");
        }
        // Silent rides on Custom; the backend pins its boosts to Low.
        assert_eq!(PerfMode::Silent.to_ec(), PerfMode::Custom.to_ec());
    }
}
