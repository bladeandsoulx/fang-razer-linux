//! Shared daemon core: hardware handle + desired state + status assembly.

use crate::gpu::GpuSwitch;
use crate::hw::Hw;
use crate::state::AppliedState;
use fang_protocol::api::{Boost, Command, FanMode, Status};
use std::path::PathBuf;

pub struct Core {
    hw: Box<dyn Hw>,
    gpu: Box<dyn GpuSwitch>,
    pub state: AppliedState,
    state_path: PathBuf,
}

impl Core {
    pub fn new(
        hw: Box<dyn Hw>,
        gpu: Box<dyn GpuSwitch>,
        state: AppliedState,
        state_path: PathBuf,
    ) -> Core {
        Core {
            hw,
            gpu,
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
            _ => return Ok(false),
        }
        self.hw.apply(&next)?;
        self.state = next;
        self.state.save(&self.state_path);
        Ok(true)
    }
}
