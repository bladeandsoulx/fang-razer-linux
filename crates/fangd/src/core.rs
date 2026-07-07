//! Shared daemon core: hardware handle + desired state + status assembly.

use crate::ddc::Ddc;
use crate::gpu::GpuSwitch;
use crate::hw::Hw;
use crate::state::AppliedState;
use fang_protocol::api::{Boost, Command, FanMode, PerfMode, Status};
use std::path::PathBuf;

pub struct Core {
    hw: Box<dyn Hw>,
    gpu: Box<dyn GpuSwitch>,
    ddc: Ddc,
    pub state: AppliedState,
    state_path: PathBuf,
    /// Last sampled power source, so automation acts only on transitions.
    last_on_ac: Option<bool>,
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
            last_on_ac: None,
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
            monitor_brightness: self.ddc.brightness(),
            auto_power: self.state.auto_power,
            ac_profile: self.state.ac_profile,
            battery_profile: self.state.battery_profile,
            ac_fan: self.state.ac_fan,
            battery_fan: self.state.battery_fan,
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

    /// Clamp a manual fan speed into the model's limits; Auto passes through.
    fn clamp_fan(&self, fan: FanMode) -> FanMode {
        match fan {
            FanMode::Manual { rpm } => {
                let info = self.hw.info();
                FanMode::Manual {
                    rpm: rpm.clamp(info.fan_rpm_min, info.fan_rpm_max),
                }
            }
            FanMode::Auto => FanMode::Auto,
        }
    }

    /// Fed the current power source each telemetry tick. When automation is on
    /// and the source just changed (including the first known reading), applies
    /// that source's profile + fan and returns the new status to broadcast.
    pub fn power_tick(&mut self, on_ac: Option<bool>) -> Option<Status> {
        let changed = on_ac != self.last_on_ac;
        self.last_on_ac = on_ac;
        if !self.state.auto_power || !changed {
            return None;
        }
        let on_ac = on_ac?;
        let (mode, fan) = if on_ac {
            (self.state.ac_profile, self.state.ac_fan)
        } else {
            (self.state.battery_profile, self.state.battery_fan)
        };
        // Never send an undefined EC mode.
        if mode == PerfMode::Creator && !self.hw.info().has_creator_mode {
            return None;
        }
        let mut next = self.state;
        next.perf_mode = mode;
        if next.cpu_boost == Boost::Boost && !self.hw.info().has_cpu_boost_oc {
            next.cpu_boost = Boost::High;
        }
        next.fan = self.clamp_fan(fan);
        // Nothing to do if this source's profile is already in effect.
        if next.perf_mode == self.state.perf_mode && next.fan == self.state.fan {
            return None;
        }
        if let Err(e) = self.hw.apply(&next) {
            log::warn!("power automation: applying {mode:?} failed: {e}");
            return None;
        }
        self.state = next;
        self.state.save(&self.state_path);
        log::info!(
            "power automation: now on {} — {mode:?}, fan {:?}",
            if on_ac { "AC" } else { "battery" },
            next.fan
        );
        Some(self.status())
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
            Command::SetMonitorBrightness { value } => {
                // Also lives in the monitor's own NVRAM, not our state file.
                self.ddc.set_brightness(*value)?;
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
            Command::SetAutoPower {
                enabled,
                ac_profile,
                battery_profile,
                ac_fan,
                battery_fan,
            } => {
                next.auto_power = *enabled;
                next.ac_profile = *ac_profile;
                next.battery_profile = *battery_profile;
                next.ac_fan = self.clamp_fan(*ac_fan);
                next.battery_fan = self.clamp_fan(*battery_fan);
                // Enabling enforces the policy right away for the current
                // source; otherwise it takes effect on the next transition.
                if *enabled {
                    if let Some(on_ac) = self.last_on_ac {
                        let (mode, fan) = if on_ac {
                            (*ac_profile, next.ac_fan)
                        } else {
                            (*battery_profile, next.battery_fan)
                        };
                        if mode != PerfMode::Creator || self.hw.info().has_creator_mode {
                            next.perf_mode = mode;
                            next.fan = fan;
                        }
                    }
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
