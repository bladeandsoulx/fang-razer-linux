//! Shared daemon core: hardware handle + desired state + status assembly.

use crate::ddc::Ddc;
use crate::gpu::GpuSwitch;
use crate::hw::Hw;
use crate::state::AppliedState;
use fang_protocol::api::{Boost, Command, FanMode, Status};
use std::path::PathBuf;

pub struct Core {
    hw: Box<dyn Hw>,
    gpu: Box<dyn GpuSwitch>,
    ddc: Ddc,
    pub state: AppliedState,
    state_path: PathBuf,
}

impl Core {
    pub fn new(
        hw: Box<dyn Hw>,
        gpu: Box<dyn GpuSwitch>,
        ddc: Ddc,
        state: AppliedState,
        state_path: PathBuf,
    ) -> Core {
        Core {
            hw,
            gpu,
            ddc,
            state,
            state_path,
        }
    }

    pub fn status(&self) -> Status {
        let info = self.hw.info();
        Status {
            model: info.name,
            device_present: info.device_present,
            verified: info.verified,
            mock: info.mock,
            perf_mode: self.state.perf_mode,
            cpu_boost: self.state.cpu_boost,
            gpu_boost: self.state.gpu_boost,
            fan: self.state.fan,
            fan_rpm_min: info.fan_rpm_min,
            fan_rpm_max: info.fan_rpm_max,
            has_cpu_boost_oc: info.has_cpu_boost_oc,
            has_creator_mode: info.has_creator_mode,
            has_bho: info.has_bho,
            bho_enabled: self.state.bho_enabled,
            bho_threshold: self.state.bho_threshold,
            has_logo: info.has_logo,
            kbd_brightness: self.state.kbd_brightness,
            kbd_effect: self.state.kbd_effect,
            logo_led: self.state.logo_led,
            color_ddc: self.ddc.available(),
            color_presets: self.ddc.presets(),
            color_current: self.ddc.current(),
            gpu_mode: self.gpu.current(),
            gpu_mode_pending: self.gpu.pending(),
            daemon_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn sample(&mut self) -> crate::hw::Sample {
        self.hw.sample()
    }

    /// Re-push the persisted state to the EC (startup, resume from suspend).
    pub fn reapply(&mut self) {
        if let Err(e) = self.hw.apply(&self.state) {
            log::warn!("could not apply state to hardware: {e}");
        }
    }

    /// Handle a state-changing command. Returns true when state changed
    /// (caller broadcasts a `state_changed` event).
    pub fn handle_set(&mut self, cmd: &Command) -> Result<bool, String> {
        let mut next = self.state;
        match cmd {
            Command::SetPerfMode {
                perf_mode,
                cpu_boost,
                gpu_boost,
            } => {
                if *perf_mode == fang_protocol::api::PerfMode::Creator
                    && !self.hw.info().has_creator_mode
                {
                    // EC mode 2 is undefined on most models; sending an
                    // undefined mode trips EC failsafes (max fans).
                    return Err("creator mode is not supported on this model".into());
                }
                next.perf_mode = *perf_mode;
                if let Some(b) = cpu_boost {
                    next.cpu_boost = *b;
                }
                if let Some(b) = gpu_boost {
                    next.gpu_boost = *b;
                }
                if next.cpu_boost == Boost::Boost && !self.hw.info().has_cpu_boost_oc {
                    next.cpu_boost = Boost::High;
                }
            }
            Command::SetFan { fan } => {
                if let FanMode::Manual { rpm } = fan {
                    let info = self.hw.info();
                    let clamped = (*rpm).clamp(info.fan_rpm_min, info.fan_rpm_max);
                    next.fan = FanMode::Manual { rpm: clamped };
                } else {
                    next.fan = FanMode::Auto;
                }
            }
            Command::SetGpuMode { gpu_mode } => {
                // Persisted by the PRIME tool itself, not our state file.
                self.gpu.set(*gpu_mode)?;
                return Ok(true);
            }
            Command::SetColorPreset { value } => {
                // The monitor remembers its own OSD setting, so this isn't
                // part of the persisted EC state.
                self.ddc.set(*value)?;
                return Ok(true);
            }
            Command::SetBho { enabled, threshold } => {
                if !self.hw.info().has_bho {
                    return Err("battery health optimizer not supported on this model".into());
                }
                next.bho_enabled = *enabled;
                next.bho_threshold = (*threshold).clamp(50, 80);
            }
            Command::SetLighting {
                brightness,
                kbd_effect,
                logo_led,
            } => {
                if let Some(b) = brightness {
                    next.kbd_brightness = (*b).min(100);
                }
                if let Some(e) = kbd_effect {
                    next.kbd_effect = *e;
                }
                if let Some(l) = logo_led {
                    if !self.hw.info().has_logo {
                        return Err("no logo LED on this model".into());
                    }
                    next.logo_led = *l;
                }
            }
            _ => return Ok(false),
        }
        self.hw.apply(&next)?;
        self.state = next;
        self.state.save(&self.state_path);
        Ok(true)
    }
}
