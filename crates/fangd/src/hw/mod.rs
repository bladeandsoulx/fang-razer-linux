//! Hardware backends: the real Razer EC on Linux, a simulator everywhere.

use crate::state::AppliedState;

pub mod mock;
#[cfg(target_os = "linux")]
pub mod razer;
#[cfg(target_os = "linux")]
pub mod sensors;

#[derive(Clone, Debug)]
pub struct ModelInfo {
    pub name: String,
    pub device_present: bool,
    pub verified: bool,
    pub mock: bool,
    pub fan_rpm_min: u16,
    pub fan_rpm_max: u16,
    pub has_cpu_boost_oc: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Sample {
    pub cpu_temp_c: Option<f32>,
    pub gpu_temp_c: Option<f32>,
    pub fan_rpm: Vec<u32>,
}

pub trait Hw: Send {
    fn info(&self) -> ModelInfo;
    /// Push the desired state to the EC. Errors are surfaced to the client.
    fn apply(&mut self, state: &AppliedState) -> Result<(), String>;
    fn sample(&mut self) -> Sample;
}

/// Pick the backend: real hardware on Linux unless `mock` is requested;
/// always the simulator elsewhere (development on Windows/macOS).
pub fn open(mock: bool) -> Box<dyn Hw> {
    #[cfg(target_os = "linux")]
    {
        if !mock {
            match razer::RazerHw::open() {
                Ok(hw) => return Box::new(hw),
                Err(e) => {
                    log::warn!("no Razer laptop device: {e}; running monitor-only");
                    return Box::new(razer::MonitorOnly::new());
                }
            }
        }
    }
    let _ = mock;
    Box::new(mock::MockHw::new())
}
