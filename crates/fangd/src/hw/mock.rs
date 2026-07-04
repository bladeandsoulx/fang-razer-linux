//! Simulated Blade 18 for development and demos (`fangd --mock`).
//!
//! Temperatures drift toward a load level implied by the performance mode,
//! and fans ease toward their target RPM, so the UI shows plausible motion.

use super::{Hw, ModelInfo, Sample};
use crate::state::AppliedState;
use fang_protocol::api::{FanMode, PerfMode};
use std::time::Instant;

pub struct MockHw {
    state: AppliedState,
    started: Instant,
    rpm: [f32; 2],
    cpu_temp: f32,
    gpu_temp: f32,
}

impl MockHw {
    pub fn new() -> MockHw {
        MockHw {
            state: AppliedState::default(),
            started: Instant::now(),
            rpm: [2300.0, 2280.0],
            cpu_temp: 52.0,
            gpu_temp: 46.0,
        }
    }

    fn target_rpm(&self) -> f32 {
        match self.state.fan {
            FanMode::Manual { rpm } => rpm as f32,
            FanMode::Auto => match self.state.perf_mode {
                PerfMode::Silent => 2200.0,
                PerfMode::Balanced => 2600.0,
                PerfMode::Creator => 3300.0,
                PerfMode::Gaming => 3800.0,
                PerfMode::Custom => 3400.0,
            },
        }
    }

    fn target_temps(&self) -> (f32, f32) {
        match self.state.perf_mode {
            PerfMode::Silent => (54.0, 48.0),
            PerfMode::Balanced => (58.0, 52.0),
            PerfMode::Creator => (68.0, 63.0),
            PerfMode::Gaming => (74.0, 70.0),
            PerfMode::Custom => (70.0, 66.0),
        }
    }
}

impl Hw for MockHw {
    fn info(&self) -> ModelInfo {
        ModelInfo {
            name: "Razer Blade 18 (simulated)".into(),
            device_present: true,
            verified: true,
            mock: true,
            fan_rpm_min: 2200,
            fan_rpm_max: 5000,
            has_cpu_boost_oc: true,
            has_bho: true,
        }
    }

    fn apply(&mut self, state: &AppliedState) -> Result<(), String> {
        self.state = *state;
        Ok(())
    }

    fn sample(&mut self) -> Sample {
        let t = self.started.elapsed().as_secs_f32();
        let wiggle = (t * 0.7).sin() * 1.2 + (t * 0.13).sin() * 2.0;
        let (ct, gt) = self.target_temps();
        self.cpu_temp += (ct + wiggle - self.cpu_temp) * 0.08;
        self.gpu_temp += (gt + wiggle * 0.8 - self.gpu_temp) * 0.06;
        let target = self.target_rpm();
        for (i, r) in self.rpm.iter_mut().enumerate() {
            let jitter = ((t * 1.9 + i as f32).sin()) * 25.0;
            *r += (target + jitter - *r) * 0.15;
        }
        Sample {
            cpu_temp_c: Some(self.cpu_temp),
            gpu_temp_c: Some(self.gpu_temp),
            fan_rpm: self.rpm.iter().map(|r| *r as u32).collect(),
        }
    }
}
