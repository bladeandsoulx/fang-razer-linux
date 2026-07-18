//! Shared daemon core: hardware handle + desired state + status assembly.

use crate::hw::{Hw, Sample};
use crate::peripherals::PeripheralSnapshot;
use crate::state::AppliedState;
use fang_protocol::api::{
    Boost, Command, FanCurvePoint, FanMode, Status, ThermalOverrideReason, API_VERSION,
};
use std::path::PathBuf;

/// The thermal guard is deliberately not configurable. It overrides both
/// Manual and Curve control at the model's maximum fan target, then uses lower
/// release thresholds to avoid rapidly toggling around the boundary.
const CPU_OVERRIDE_ON_C: f32 = 95.0;
const CPU_OVERRIDE_OFF_C: f32 = 88.0;
const GPU_OVERRIDE_ON_C: f32 = 87.0;
const GPU_OVERRIDE_OFF_C: f32 = 82.0;
/// Tolerate two isolated read failures, then treat the mandatory CPU sensor as
/// stale. At the 1 Hz control rate this forces max fans within three seconds.
const CPU_SENSOR_MISS_LIMIT: u8 = 3;
const MIN_CURVE_POINTS: usize = 2;
const MAX_CURVE_POINTS: usize = 8;

pub struct ControlledSample {
    pub hw: Sample,
    pub fan_target_rpm: Option<u16>,
    pub thermal_override_active: bool,
    pub thermal_sensor_ok: bool,
    pub thermal_override_reason: Option<ThermalOverrideReason>,
}

pub struct Core {
    hw: Box<dyn Hw>,
    pub state: AppliedState,
    state_path: PathBuf,
    /// Last sampled power source, so automation acts only on transitions.
    last_on_ac: Option<bool>,
    /// Last target successfully sent by the software fan policy.
    fan_target_rpm: Option<u16>,
    /// True only after a complete state application succeeds. When recovery
    /// falls back to EC Auto, this prevents target-only writes from silently
    /// resuming software fan control without first re-enabling manual mode.
    hardware_state_applied: bool,
    thermal_override_active: bool,
    thermal_override_reason: Option<ThermalOverrideReason>,
    last_cpu_temp_c: Option<f32>,
    last_gpu_temp_c: Option<f32>,
    cpu_missed_samples: u8,
}

impl Core {
    pub fn new(hw: Box<dyn Hw>, state: AppliedState, state_path: PathBuf) -> Core {
        let mut core = Core {
            hw,
            state,
            state_path,
            last_on_ac: None,
            fan_target_rpm: None,
            hardware_state_applied: false,
            thermal_override_active: false,
            thermal_override_reason: None,
            last_cpu_temp_c: None,
            last_gpu_temp_c: None,
            cpu_missed_samples: CPU_SENSOR_MISS_LIMIT,
        };
        core.sanitize_loaded_state();
        core
    }

    pub fn status(&self, peripherals: &PeripheralSnapshot) -> Status {
        let info = self.hw.info();
        Status {
            api_version: API_VERSION,
            model: info.name,
            device_present: info.device_present,
            verified: info.verified,
            mock: info.mock,
            perf_mode: self.state.perf_mode,
            cpu_boost: self.state.cpu_boost,
            gpu_boost: self.state.gpu_boost,
            fan: self.state.fan.clone(),
            fan_curve: self.state.fan_curve.clone(),
            fan_rpm_min: info.fan_rpm_min,
            fan_rpm_max: info.fan_rpm_max,
            has_cpu_boost_oc: info.has_cpu_boost_oc,
            has_bho: info.has_bho,
            bho_enabled: self.state.bho_enabled,
            bho_threshold: self.state.bho_threshold,
            has_logo: info.has_logo,
            kbd_brightness: self.state.kbd_brightness,
            kbd_effect: self.state.kbd_effect,
            logo_led: self.state.logo_led,
            color_ddc: peripherals.color_ddc,
            color_presets: peripherals.color_presets.clone(),
            color_current: peripherals.color_current,
            monitor_brightness: peripherals.monitor_brightness,
            auto_power: self.state.auto_power,
            ac_profile: self.state.ac_profile,
            battery_profile: self.state.battery_profile,
            ac_fan: self.state.ac_fan.clone(),
            battery_fan: self.state.battery_fan.clone(),
            gpu_mode: peripherals.gpu_mode,
            gpu_mode_pending: peripherals.gpu_mode_pending,
            daemon_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    pub fn sample(&mut self) -> ControlledSample {
        let hw = self.hw.sample();
        if let Some(cpu) = hw.cpu_temp_c {
            self.last_cpu_temp_c = Some(cpu);
            self.cpu_missed_samples = 0;
        } else {
            self.cpu_missed_samples = self.cpu_missed_samples.saturating_add(1);
        }
        self.last_gpu_temp_c = hw.gpu_temp_c;
        self.update_fan_policy();
        ControlledSample {
            hw,
            fan_target_rpm: self.fan_target_rpm,
            thermal_override_active: self.thermal_override_active,
            thermal_sensor_ok: self.thermal_sensor_ok(),
            thermal_override_reason: self.thermal_override_reason,
        }
    }

    fn thermal_sensor_ok(&self) -> bool {
        self.last_cpu_temp_c.is_some() && self.cpu_missed_samples < CPU_SENSOR_MISS_LIMIT
    }

    /// Re-push the persisted state to the EC (startup, resume from suspend).
    pub fn reapply(&mut self) {
        let state = self.state.clone();
        if let Err(e) = self.apply_hardware_state(&state) {
            log::warn!("could not apply state to hardware: {e}");
            return;
        }
        self.reset_runtime_fan_target();
        self.update_fan_policy();
    }

    /// Restore the EC's automatic fan policy without changing the persisted
    /// preference. A later daemon start can reapply that preference safely.
    pub fn restore_auto_fan(&mut self) -> Result<(), String> {
        let result = self.hw.restore_auto_fan(self.state.perf_mode);
        self.mark_hardware_unapplied();
        result
    }

    /// Apply all hardware fields as one logical transaction. EC commands are
    /// inherently sequential, so an error is recovered by reapplying the last
    /// complete state. If that cannot be proven, EC automatic fan control is
    /// the final safe state and target-only writes remain disabled.
    fn apply_hardware_state(&mut self, desired: &AppliedState) -> Result<(), String> {
        let previous = self.state.clone();
        let previous_was_applied = self.hardware_state_applied;
        let apply_error = match self.hw.apply(desired) {
            Ok(()) => {
                self.hardware_state_applied = true;
                return Ok(());
            }
            Err(e) => e,
        };

        let mut rollback_error = None;
        if previous_was_applied {
            match self.hw.apply(&previous) {
                Ok(()) => {
                    self.hardware_state_applied = true;
                    self.reset_runtime_fan_target();
                    return Err(format!("{apply_error}; previous hardware state restored"));
                }
                Err(e) => rollback_error = Some(e),
            }
        }

        let auto_result = self.hw.restore_auto_fan(previous.perf_mode);
        self.mark_hardware_unapplied();
        match (rollback_error, auto_result) {
            (Some(rollback), Ok(())) => Err(format!(
                "{apply_error}; restoring the previous state failed: {rollback}; fell back to EC automatic fan control"
            )),
            (Some(rollback), Err(auto)) => Err(format!(
                "{apply_error}; restoring the previous state failed: {rollback}; EC automatic fan recovery also failed: {auto}"
            )),
            (None, Ok(())) => Err(format!(
                "{apply_error}; fell back to EC automatic fan control"
            )),
            (None, Err(auto)) => Err(format!(
                "{apply_error}; EC automatic fan recovery also failed: {auto}"
            )),
        }
    }

    fn mark_hardware_unapplied(&mut self) {
        self.hardware_state_applied = false;
        self.fan_target_rpm = None;
        self.thermal_override_active = false;
        self.thermal_override_reason = None;
    }

    /// Normalize persisted values before the first EC command. This protects
    /// against older state schemas, hand-edited files, and moving a state file
    /// between laptop models with different limits/features.
    fn sanitize_loaded_state(&mut self) {
        let before = self.state.clone();
        let info = self.hw.info();
        if info.device_present {
            if self.state.cpu_boost == Boost::Boost && !info.has_cpu_boost_oc {
                self.state.cpu_boost = Boost::High;
            }
            if !info.has_bho {
                self.state.bho_enabled = false;
            }
            let active_fan = self.normalized_fan_or_auto(&self.state.fan, "active");
            if let FanMode::Curve { points } = &active_fan {
                self.state.fan_curve = points.clone();
            }
            self.state.fan = active_fan;
            if !self.state.fan_curve.is_empty() {
                let saved = FanMode::Curve {
                    points: self.state.fan_curve.clone(),
                };
                match self.normalize_fan(&saved) {
                    Ok(FanMode::Curve { points }) => self.state.fan_curve = points,
                    Ok(_) => unreachable!(),
                    Err(e) => {
                        log::warn!("invalid saved fan curve ({e}); discarding it");
                        self.state.fan_curve.clear();
                    }
                }
            }
            self.state.ac_fan = self.normalized_fan_or_auto(&self.state.ac_fan, "AC automation");
            self.state.battery_fan =
                self.normalized_fan_or_auto(&self.state.battery_fan, "battery automation");
        }
        self.state.bho_threshold = self.state.bho_threshold.clamp(50, 80);
        self.state.kbd_brightness = self.state.kbd_brightness.min(100);
        if self.state != before {
            log::warn!("normalized persisted state for the detected model");
            self.state.save(&self.state_path);
        }
    }

    fn normalized_fan_or_auto(&self, fan: &FanMode, label: &str) -> FanMode {
        match self.normalize_fan(fan) {
            Ok(fan) => fan,
            Err(e) => {
                log::warn!("invalid {label} fan policy in persisted state ({e}); using Auto");
                FanMode::Auto
            }
        }
    }

    fn normalize_fan(&self, fan: &FanMode) -> Result<FanMode, String> {
        let info = self.hw.info();
        if !info.device_present {
            return match fan {
                FanMode::Auto => Ok(FanMode::Auto),
                _ => Err("fan control is unavailable without a Razer device".into()),
            };
        }
        match fan {
            FanMode::Auto => Ok(FanMode::Auto),
            FanMode::Manual { rpm } => Ok(FanMode::Manual {
                rpm: normalize_rpm(*rpm, info.fan_rpm_min, info.fan_rpm_max),
            }),
            FanMode::Curve { points } => {
                if !(MIN_CURVE_POINTS..=MAX_CURVE_POINTS).contains(&points.len()) {
                    return Err(format!(
                        "fan curve needs {MIN_CURVE_POINTS}..={MAX_CURVE_POINTS} points"
                    ));
                }
                let mut normalized: Vec<FanCurvePoint> = Vec::with_capacity(points.len());
                for point in points {
                    if !(30..=100).contains(&point.temp_c) {
                        return Err("fan-curve temperatures must be between 30 and 100 C".into());
                    }
                    let point = FanCurvePoint {
                        temp_c: point.temp_c,
                        rpm: normalize_rpm(point.rpm, info.fan_rpm_min, info.fan_rpm_max),
                    };
                    if let Some(previous) = normalized.last() {
                        if point.temp_c <= previous.temp_c {
                            return Err("fan-curve temperatures must be strictly increasing".into());
                        }
                        if point.rpm < previous.rpm {
                            return Err(
                                "fan-curve RPM must not decrease as temperature rises".into()
                            );
                        }
                    }
                    normalized.push(point);
                }
                Ok(FanMode::Curve { points: normalized })
            }
        }
    }

    fn reset_runtime_fan_target(&mut self) {
        let max = self.hw.info().fan_rpm_max;
        match self.state.fan {
            FanMode::Auto => {
                self.fan_target_rpm = None;
                self.thermal_override_active = false;
                self.thermal_override_reason = None;
            }
            FanMode::Manual { .. } | FanMode::Curve { .. } => {
                // The hardware backend starts software control at max RPM. It
                // stays there until the mandatory CPU sensor is fresh and cool.
                self.fan_target_rpm = Some(max);
                self.thermal_override_active = true;
                self.thermal_override_reason = Some(ThermalOverrideReason::SensorUnavailable);
            }
        }
    }

    fn update_fan_policy(&mut self) {
        if !self.hardware_state_applied {
            self.fan_target_rpm = None;
            self.thermal_override_active = false;
            self.thermal_override_reason = None;
            return;
        }
        let info = self.hw.info();
        let requested = match &self.state.fan {
            FanMode::Auto => {
                self.fan_target_rpm = None;
                self.thermal_override_active = false;
                self.thermal_override_reason = None;
                return;
            }
            FanMode::Manual { rpm } => *rpm,
            FanMode::Curve { points } => {
                match hottest_temp(self.last_cpu_temp_c, self.last_gpu_temp_c) {
                    Some(temp) => curve_target(points, temp),
                    None => info.fan_rpm_max,
                }
            }
        };

        let sensor_unavailable = !self.thermal_sensor_ok();
        let temperature_override = thermal_override_next(
            self.thermal_override_active,
            self.last_cpu_temp_c,
            self.last_gpu_temp_c,
        );
        let override_active = sensor_unavailable || temperature_override;
        let override_reason = if sensor_unavailable {
            Some(ThermalOverrideReason::SensorUnavailable)
        } else if temperature_override {
            Some(ThermalOverrideReason::Temperature)
        } else {
            None
        };
        let target = if override_active {
            info.fan_rpm_max
        } else {
            normalize_rpm(requested, info.fan_rpm_min, info.fan_rpm_max)
        };

        if override_active != self.thermal_override_active {
            if override_active {
                log::warn!(
                    "thermal fan override active ({:?}; cpu {:?} C, gpu {:?} C); forcing {} RPM",
                    override_reason,
                    self.last_cpu_temp_c,
                    self.last_gpu_temp_c,
                    info.fan_rpm_max
                );
            } else {
                log::info!("thermal fan override released");
            }
        }
        self.thermal_override_active = override_active;
        self.thermal_override_reason = override_reason;

        if self.fan_target_rpm == Some(target) {
            return;
        }
        match self.hw.set_fan_target(target) {
            Ok(()) => self.fan_target_rpm = Some(target),
            Err(e) => {
                log::warn!("could not update software fan target to {target} RPM: {e}");
                // A two-zone target update can itself fail halfway through.
                // Reapply the complete current state, which starts at max RPM;
                // the next sensor tick may safely lower it again.
                let current = self.state.clone();
                if let Err(recovery) = self.apply_hardware_state(&current) {
                    log::warn!("fan-target recovery: {recovery}");
                } else {
                    self.reset_runtime_fan_target();
                }
            }
        }
    }

    /// Fed the current power source each telemetry tick. When automation is on
    /// and the source just changed (including the first known reading), applies
    /// that source's profile + fan and returns the new status to broadcast.
    pub fn power_tick(&mut self, on_ac: Option<bool>) -> bool {
        let changed = on_ac != self.last_on_ac;
        self.last_on_ac = on_ac;
        if !self.state.auto_power || !changed {
            return false;
        }
        let Some(on_ac) = on_ac else {
            return false;
        };
        let (mode, fan) = if on_ac {
            (self.state.ac_profile, self.state.ac_fan.clone())
        } else {
            (self.state.battery_profile, self.state.battery_fan.clone())
        };
        let mut next = self.state.clone();
        next.perf_mode = mode;
        if next.cpu_boost == Boost::Boost && !self.hw.info().has_cpu_boost_oc {
            next.cpu_boost = Boost::High;
        }
        next.fan = fan;
        if let FanMode::Curve { points } = &next.fan {
            next.fan_curve = points.clone();
        }
        if next.perf_mode == self.state.perf_mode && next.fan == self.state.fan {
            return false;
        }
        if let Err(e) = self.apply_hardware_state(&next) {
            log::warn!("power automation: applying {mode:?} failed: {e}");
            return false;
        }
        self.state = next;
        self.reset_runtime_fan_target();
        self.update_fan_policy();
        self.state.save(&self.state_path);
        log::info!(
            "power automation: now on {} — {mode:?}, fan {:?}",
            if on_ac { "AC" } else { "battery" },
            self.state.fan
        );
        true
    }

    /// Handle a state-changing command. Returns true when state changed
    /// (caller broadcasts a `state_changed` event).
    pub fn handle_set(&mut self, cmd: &Command) -> Result<bool, String> {
        let mut next = self.state.clone();
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
                let fan = self.normalize_fan(fan)?;
                if let FanMode::Curve { points } = &fan {
                    next.fan_curve = points.clone();
                }
                next.fan = fan;
            }
            Command::SetGpuMode { .. }
            | Command::SetColorPreset { .. }
            | Command::SetMonitorBrightness { .. } => return Ok(false),
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
                next.ac_fan = self.normalize_fan(ac_fan)?;
                next.battery_fan = self.normalize_fan(battery_fan)?;
                if *enabled {
                    if let Some(on_ac) = self.last_on_ac {
                        let (mode, fan) = if on_ac {
                            (*ac_profile, next.ac_fan.clone())
                        } else {
                            (*battery_profile, next.battery_fan.clone())
                        };
                        next.perf_mode = mode;
                        next.fan = fan;
                        if let FanMode::Curve { points } = &next.fan {
                            next.fan_curve = points.clone();
                        }
                    }
                }
            }
            _ => return Ok(false),
        }
        self.apply_hardware_state(&next)?;
        self.state = next;
        self.reset_runtime_fan_target();
        self.update_fan_policy();
        self.state.save(&self.state_path);
        Ok(true)
    }
}

fn normalize_rpm(rpm: u16, min: u16, max: u16) -> u16 {
    let clamped = rpm.clamp(min, max);
    let rounded = (((clamped as u32 + 50) / 100) * 100) as u16;
    rounded.clamp(min, max)
}

fn hottest_temp(cpu: Option<f32>, gpu: Option<f32>) -> Option<f32> {
    match (cpu, gpu) {
        (Some(cpu), Some(gpu)) => Some(cpu.max(gpu)),
        (Some(cpu), None) => Some(cpu),
        (None, Some(gpu)) => Some(gpu),
        (None, None) => None,
    }
}

fn curve_target(points: &[FanCurvePoint], temp_c: f32) -> u16 {
    let first = points[0];
    if temp_c <= first.temp_c as f32 {
        return first.rpm;
    }
    for pair in points.windows(2) {
        let low = pair[0];
        let high = pair[1];
        if temp_c <= high.temp_c as f32 {
            let span = (high.temp_c - low.temp_c) as f32;
            let fraction = (temp_c - low.temp_c as f32) / span;
            return (low.rpm as f32 + (high.rpm as f32 - low.rpm as f32) * fraction).round() as u16;
        }
    }
    points.last().expect("validated non-empty curve").rpm
}

fn thermal_override_next(active: bool, cpu: Option<f32>, gpu: Option<f32>) -> bool {
    if active {
        // If every sensor vanishes while already protecting the machine, stay
        // at max until a cool reading proves it is safe to release.
        if cpu.is_none() && gpu.is_none() {
            return true;
        }
        cpu.is_some_and(|t| t >= CPU_OVERRIDE_OFF_C) || gpu.is_some_and(|t| t >= GPU_OVERRIDE_OFF_C)
    } else {
        cpu.is_some_and(|t| t >= CPU_OVERRIDE_ON_C) || gpu.is_some_and(|t| t >= GPU_OVERRIDE_ON_C)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hw::ModelInfo;
    use fang_protocol::api::PerfMode;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    const CURVE: [FanCurvePoint; 3] = [
        FanCurvePoint {
            temp_c: 40,
            rpm: 2200,
        },
        FanCurvePoint {
            temp_c: 70,
            rpm: 3400,
        },
        FanCurvePoint {
            temp_c: 90,
            rpm: 5000,
        },
    ];

    struct TestHw {
        sample: Sample,
        restored: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    }

    impl Hw for TestHw {
        fn info(&self) -> ModelInfo {
            ModelInfo {
                name: "test laptop".into(),
                device_present: true,
                verified: true,
                mock: true,
                fan_rpm_min: 2200,
                fan_rpm_max: 5000,
                has_cpu_boost_oc: true,
                has_bho: true,
                has_logo: true,
            }
        }

        fn apply(&mut self, _state: &AppliedState) -> Result<(), String> {
            Ok(())
        }

        fn set_fan_target(&mut self, _rpm: u16) -> Result<(), String> {
            Ok(())
        }

        fn restore_auto_fan(
            &mut self,
            _perf_mode: fang_protocol::api::PerfMode,
        ) -> Result<(), String> {
            if let Some(restored) = &self.restored {
                restored.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            Ok(())
        }

        fn sample(&mut self) -> Sample {
            self.sample.clone()
        }
    }

    struct ScriptedTrace {
        applied: Mutex<Vec<AppliedState>>,
        apply_results: Mutex<VecDeque<Result<(), String>>>,
        restore_results: Mutex<VecDeque<Result<(), String>>>,
        fan_target_results: Mutex<VecDeque<Result<(), String>>>,
        restores: AtomicUsize,
        fan_targets: AtomicUsize,
    }

    struct ScriptedHw {
        trace: Arc<ScriptedTrace>,
        sample: Sample,
    }

    impl Hw for ScriptedHw {
        fn info(&self) -> ModelInfo {
            ModelInfo {
                name: "scripted laptop".into(),
                device_present: true,
                verified: true,
                mock: true,
                fan_rpm_min: 2200,
                fan_rpm_max: 5000,
                has_cpu_boost_oc: true,
                has_bho: true,
                has_logo: true,
            }
        }

        fn apply(&mut self, state: &AppliedState) -> Result<(), String> {
            self.trace.applied.lock().unwrap().push(state.clone());
            self.trace
                .apply_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(()))
        }

        fn set_fan_target(&mut self, _rpm: u16) -> Result<(), String> {
            self.trace.fan_targets.fetch_add(1, Ordering::Relaxed);
            self.trace
                .fan_target_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(()))
        }

        fn restore_auto_fan(&mut self, _perf_mode: PerfMode) -> Result<(), String> {
            self.trace.restores.fetch_add(1, Ordering::Relaxed);
            self.trace
                .restore_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(()))
        }

        fn sample(&mut self) -> Sample {
            self.sample.clone()
        }
    }

    fn scripted_core(
        state: AppliedState,
        sample: Sample,
        apply_results: Vec<Result<(), String>>,
        restore_results: Vec<Result<(), String>>,
    ) -> (Core, Arc<ScriptedTrace>) {
        let trace = Arc::new(ScriptedTrace {
            applied: Mutex::new(Vec::new()),
            apply_results: Mutex::new(apply_results.into()),
            restore_results: Mutex::new(restore_results.into()),
            fan_target_results: Mutex::new(VecDeque::new()),
            restores: AtomicUsize::new(0),
            fan_targets: AtomicUsize::new(0),
        });
        let hw = ScriptedHw {
            trace: Arc::clone(&trace),
            sample,
        };
        let core = Core::new(
            Box::new(hw),
            state,
            std::env::temp_dir().join("fangd-core-transaction-test-state.json"),
        );
        (core, trace)
    }

    fn core_with(fan: FanMode, sample: Sample) -> Core {
        let state = AppliedState {
            fan,
            ..AppliedState::default()
        };
        Core::new(
            Box::new(TestHw {
                sample,
                restored: None,
            }),
            state,
            std::env::temp_dir().join("fangd-core-test-state.json"),
        )
    }

    #[test]
    fn fan_curve_clamps_and_interpolates() {
        assert_eq!(curve_target(&CURVE, 30.0), 2200);
        assert_eq!(curve_target(&CURVE, 55.0), 2800);
        assert_eq!(curve_target(&CURVE, 80.0), 4200);
        assert_eq!(curve_target(&CURVE, 100.0), 5000);
    }

    #[test]
    fn thermal_override_has_hysteresis_and_fails_safe() {
        assert!(!thermal_override_next(false, Some(94.9), Some(86.9)));
        assert!(thermal_override_next(false, Some(95.0), None));
        assert!(thermal_override_next(false, None, Some(87.0)));
        assert!(thermal_override_next(true, Some(88.0), Some(70.0)));
        assert!(!thermal_override_next(true, Some(87.9), Some(81.9)));
        assert!(thermal_override_next(true, None, None));
    }

    #[test]
    fn hot_manual_control_is_always_overridden() {
        let mut core = core_with(
            FanMode::Manual { rpm: 2200 },
            Sample {
                cpu_temp_c: Some(95.0),
                gpu_temp_c: Some(70.0),
                ..Sample::default()
            },
        );
        core.reapply();
        let sample = core.sample();
        assert!(sample.thermal_override_active);
        assert_eq!(sample.fan_target_rpm, Some(5000));
    }

    #[test]
    fn curve_without_temperature_fails_safe_at_maximum() {
        let mut core = core_with(
            FanMode::Curve {
                points: CURVE.to_vec(),
            },
            Sample::default(),
        );
        core.reapply();
        let sample = core.sample();
        assert!(sample.thermal_override_active);
        assert_eq!(sample.fan_target_rpm, Some(5000));
        assert_eq!(
            sample.thermal_override_reason,
            Some(ThermalOverrideReason::SensorUnavailable)
        );
    }

    #[test]
    fn manual_without_cpu_temperature_fails_safe_at_maximum() {
        let mut core = core_with(FanMode::Manual { rpm: 2200 }, Sample::default());
        core.reapply();
        let sample = core.sample();
        assert!(sample.thermal_override_active);
        assert!(!sample.thermal_sensor_ok);
        assert_eq!(sample.fan_target_rpm, Some(5000));
        assert_eq!(
            sample.thermal_override_reason,
            Some(ThermalOverrideReason::SensorUnavailable)
        );
    }

    #[test]
    fn stale_cpu_sensor_forces_max_after_short_grace_period() {
        let mut core = core_with(
            FanMode::Manual { rpm: 2200 },
            Sample {
                cpu_temp_c: Some(70.0),
                ..Sample::default()
            },
        );
        core.reapply();
        let sample = core.sample();
        assert!(sample.thermal_sensor_ok);
        assert!(!sample.thermal_override_active);

        core.cpu_missed_samples = CPU_SENSOR_MISS_LIMIT - 1;
        core.update_fan_policy();
        assert!(!core.thermal_override_active);

        core.cpu_missed_samples = CPU_SENSOR_MISS_LIMIT;
        core.update_fan_policy();
        assert!(core.thermal_override_active);
        assert_eq!(core.fan_target_rpm, Some(5000));
        assert_eq!(
            core.thermal_override_reason,
            Some(ThermalOverrideReason::SensorUnavailable)
        );
    }

    #[test]
    fn shutdown_restores_ec_auto_without_overwriting_saved_preference() {
        let restored = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let state = AppliedState {
            fan: FanMode::Manual { rpm: 2200 },
            ..AppliedState::default()
        };
        let mut core = Core::new(
            Box::new(TestHw {
                sample: Sample::default(),
                restored: Some(std::sync::Arc::clone(&restored)),
            }),
            state,
            std::env::temp_dir().join("fangd-core-shutdown-test-state.json"),
        );
        core.restore_auto_fan().expect("restore Auto");
        assert!(restored.load(std::sync::atomic::Ordering::Relaxed));
        assert!(matches!(core.state.fan, FanMode::Manual { rpm: 2200 }));
        assert_eq!(core.fan_target_rpm, None);
    }

    #[test]
    fn automatic_ec_control_is_not_replaced_by_software_guard() {
        let mut core = core_with(
            FanMode::Auto,
            Sample {
                cpu_temp_c: Some(99.0),
                gpu_temp_c: Some(90.0),
                ..Sample::default()
            },
        );
        core.reapply();
        let sample = core.sample();
        assert!(!sample.thermal_override_active);
        assert_eq!(sample.fan_target_rpm, None);
    }

    #[test]
    fn rpm_targets_are_clamped_and_rounded_to_ec_steps() {
        assert_eq!(normalize_rpm(1000, 2200, 5000), 2200);
        assert_eq!(normalize_rpm(3349, 2200, 5000), 3300);
        assert_eq!(normalize_rpm(3350, 2200, 5000), 3400);
        assert_eq!(normalize_rpm(9000, 2200, 5000), 5000);
    }

    #[test]
    fn failed_state_change_restores_the_previous_complete_state() {
        let initial = AppliedState::default();
        let (mut core, trace) = scripted_core(
            initial.clone(),
            Sample::default(),
            vec![Ok(()), Err("forward write failed".into()), Ok(())],
            vec![],
        );
        core.reapply();

        let error = core
            .handle_set(&Command::SetFan {
                fan: FanMode::Manual { rpm: 3000 },
            })
            .expect_err("the failed update must be reported");

        assert!(
            error.contains("previous hardware state restored"),
            "{error}"
        );
        assert_eq!(core.state, initial);
        assert!(core.hardware_state_applied);
        assert_eq!(trace.restores.load(Ordering::Relaxed), 0);
        let applied = trace.applied.lock().unwrap();
        assert_eq!(applied.len(), 3);
        assert!(matches!(&applied[1].fan, FanMode::Manual { rpm: 3000 }));
        assert_eq!(applied[2], initial);
    }

    #[test]
    fn failed_rollback_falls_back_to_auto_and_blocks_target_writes() {
        let initial = AppliedState {
            fan: FanMode::Manual { rpm: 3000 },
            ..AppliedState::default()
        };
        let (mut core, trace) = scripted_core(
            initial.clone(),
            Sample {
                cpu_temp_c: Some(70.0),
                gpu_temp_c: Some(65.0),
                ..Sample::default()
            },
            vec![
                Ok(()),
                Err("forward write failed".into()),
                Err("rollback write failed".into()),
            ],
            vec![Ok(())],
        );
        core.reapply();

        let error = core
            .handle_set(&Command::SetPerfMode {
                perf_mode: fang_protocol::api::PerfMode::Gaming,
                cpu_boost: None,
                gpu_boost: None,
            })
            .expect_err("the failed update must be reported");

        assert!(
            error.contains("fell back to EC automatic fan control"),
            "{error}"
        );
        assert_eq!(core.state, initial);
        assert!(!core.hardware_state_applied);
        assert_eq!(trace.restores.load(Ordering::Relaxed), 1);

        core.sample();
        assert_eq!(trace.fan_targets.load(Ordering::Relaxed), 0);
        assert_eq!(core.fan_target_rpm, None);
    }

    #[test]
    fn failed_initial_apply_uses_auto_without_assuming_a_rollback_state() {
        let initial = AppliedState {
            fan: FanMode::Manual { rpm: 3000 },
            ..AppliedState::default()
        };
        let (mut core, trace) = scripted_core(
            initial,
            Sample::default(),
            vec![Err("startup write failed".into())],
            vec![Ok(())],
        );

        core.reapply();

        assert!(!core.hardware_state_applied);
        assert_eq!(trace.applied.lock().unwrap().len(), 1);
        assert_eq!(trace.restores.load(Ordering::Relaxed), 1);
        core.sample();
        assert_eq!(trace.fan_targets.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn partial_fan_target_failure_reapplies_both_zones_at_maximum() {
        let initial = AppliedState {
            fan: FanMode::Manual { rpm: 3000 },
            ..AppliedState::default()
        };
        let (mut core, trace) = scripted_core(
            initial,
            Sample {
                cpu_temp_c: Some(70.0),
                gpu_temp_c: Some(65.0),
                ..Sample::default()
            },
            vec![Ok(()), Ok(())],
            vec![],
        );
        trace
            .fan_target_results
            .lock()
            .unwrap()
            .push_back(Err("fan 2 target failed".into()));
        core.reapply();

        core.sample();

        assert!(core.hardware_state_applied);
        assert_eq!(trace.fan_targets.load(Ordering::Relaxed), 1);
        assert_eq!(trace.applied.lock().unwrap().len(), 2);
        assert_eq!(core.fan_target_rpm, Some(5000));
    }
}
