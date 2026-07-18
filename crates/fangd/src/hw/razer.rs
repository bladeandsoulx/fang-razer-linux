//! Real hardware backend: Razer EC over hidraw (Linux only).

use super::{sensors::Sensors, Hw, ModelInfo, Sample};
use crate::state::AppliedState;
use fang_protocol::api::{Boost, FanMode, KbdEffect, LogoMode, PerfMode};
use fang_protocol::models;
use fang_protocol::packet::{self, Report, RAZER_VID, REPORT_LEN, ZONES};
use hidapi::{HidApi, HidDevice};
use std::thread;
use std::time::Duration;

pub struct RazerHw {
    device: HidDevice,
    model: &'static models::LaptopModel,
    verified: bool,
    name: String,
    sensors: Sensors,
}

const UNVERIFIED_PID_ENV: &str = "FANGD_ALLOW_UNVERIFIED_PID";

/// Parse an exact hexadecimal PID approval. A broad boolean opt-in would make
/// it too easy to send laptop EC commands to an unrelated Razer peripheral.
fn parse_approved_pid(value: &str) -> Result<u16, String> {
    let value = value.trim();
    let value = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .unwrap_or(value);
    if value.is_empty() || value.len() > 4 {
        return Err("expected one hexadecimal USB product id (for example 02b8)".into());
    }
    let pid = u16::from_str_radix(value, 16)
        .map_err(|_| "expected one hexadecimal USB product id (for example 02b8)".to_string())?;
    if pid == 0 {
        return Err("USB product id 0000 cannot be approved".into());
    }
    Ok(pid)
}

/// Choose the laptop among Razer HID candidates on interface 0, each given as
/// `(product_id, usage_page)`. A recognized laptop always wins. An unknown
/// vendor-defined interface is eligible only when its exact PID was explicitly
/// approved; otherwise the daemon remains monitor-only. Returns the index into
/// `candidates`, or `None` when no safe control target is present.
fn choose_laptop(candidates: &[(u16, u16)], approved_unverified: Option<u16>) -> Option<usize> {
    candidates
        .iter()
        .position(|&(pid, _)| models::by_pid(pid).is_some())
        .or_else(|| {
            let approved = approved_unverified?;
            candidates
                .iter()
                .position(|&(pid, usage_page)| pid == approved && usage_page >= 0xFF00)
        })
}

impl RazerHw {
    pub fn open() -> Result<RazerHw, String> {
        let api = HidApi::new().map_err(|e| format!("hidapi init: {e}"))?;
        // A Razer mouse or keyboard also has vendor id 0x1532 and presents an
        // interface 0, so the first match isn't necessarily the laptop — and
        // firing class-0x0d EC commands at a mouse is not what we want. Collect
        // every candidate, then let `choose_laptop` accept only a recognized
        // laptop PID or an exact, explicitly approved unknown PID.
        let candidates: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == RAZER_VID && d.interface_number() == 0)
            .collect();
        let sig: Vec<(u16, u16)> = candidates
            .iter()
            .map(|d| (d.product_id(), d.usage_page()))
            .collect();
        let approved_unverified = std::env::var(UNVERIFIED_PID_ENV).ok().and_then(|value| {
            match parse_approved_pid(&value) {
                Ok(pid) => Some(pid),
                Err(error) => {
                    log::warn!("ignoring invalid {UNVERIFIED_PID_ENV}={value:?}: {error}");
                    None
                }
            }
        });
        let dev_info = choose_laptop(&sig, approved_unverified)
            .map(|i| candidates[i])
            .ok_or_else(|| {
                if candidates.is_empty() {
                    "no USB device with Razer vendor id 0x1532".to_string()
                } else if let Some((pid, _)) = sig.iter().find(|(pid, usage_page)| {
                    models::by_pid(*pid).is_none() && *usage_page >= 0xFF00
                }) {
                    format!(
                        "unrecognized Razer EC interface (pid {pid:04x}); controls are disabled by \
                         default. Add a verified model entry or explicitly approve this exact PID \
                         with {UNVERIFIED_PID_ENV}={pid:04x}"
                    )
                } else {
                    "found a Razer USB device but no laptop EC — a Razer mouse or \
                     keyboard is not a Blade (add your model's PID to models.rs if \
                     this is an unrecognized laptop)"
                        .to_string()
                }
            })?;
        let pid = dev_info.product_id();
        let device = dev_info
            .open_device(&api)
            .map_err(|e| format!("open hidraw for {pid:04x}: {e} (running as root?)"))?;

        let (model, verified) = match models::by_pid(pid) {
            Some(m) => (m, true),
            None => (&models::FALLBACK, false),
        };
        let name = if verified {
            model.name.to_string()
        } else {
            let product = dev_info.product_string().unwrap_or("Razer laptop");
            format!("{product} (pid {pid:04x}, unverified)")
        };
        if !verified {
            log::warn!(
                "controlling explicitly approved, unverified Razer PID {pid:04x} with conservative limits"
            );
        }
        log::info!("found {name}");
        Ok(RazerHw {
            device,
            model,
            verified,
            name,
            sensors: Sensors::discover(),
        })
    }

    /// Send one report and read back the EC's answer. Retry once when the EC
    /// is busy or returns a malformed/mismatched response.
    fn command(&self, report: Report) -> Result<Report, String> {
        for attempt in 0..2 {
            let buf = report.to_feature_report();
            self.device
                .send_feature_report(&buf)
                .map_err(|e| format!("send_feature_report: {e}"))?;
            thread::sleep(Duration::from_micros(1500));

            let mut resp = [0u8; REPORT_LEN];
            let bytes_read = self
                .device
                .get_feature_report(&mut resp)
                .map_err(|e| format!("get_feature_report: {e}"))?;
            let response = resp
                .get(..bytes_read)
                .ok_or_else(|| format!("invalid feature report length {bytes_read}"))?;
            let parsed = match Report::response_from_feature_report(&report, response) {
                Ok(parsed) => parsed,
                Err(e) if attempt == 0 => {
                    log::warn!(
                        "invalid EC response for {:#04x}/{:#04x}: {e}; retrying",
                        report.command_class,
                        report.command_id
                    );
                    thread::sleep(Duration::from_millis(20));
                    continue;
                }
                Err(e) => {
                    return Err(format!(
                        "invalid EC response for {:#04x}/{:#04x}: {e}",
                        report.command_class, report.command_id
                    ))
                }
            };

            match parsed.status {
                packet::status::SUCCESS => return Ok(parsed),
                packet::status::BUSY if attempt == 0 => {
                    thread::sleep(Duration::from_millis(20));
                    continue;
                }
                packet::status::NOT_SUPPORTED => {
                    return Err(format!(
                        "EC rejected command {:#04x}/{:#04x} as unsupported",
                        report.command_class, report.command_id
                    ))
                }
                other => {
                    return Err(format!(
                        "EC status {other:#04x} for command {:#04x}/{:#04x}",
                        report.command_class, report.command_id
                    ))
                }
            }
        }
        Err("EC busy".into())
    }

    fn clamp_rpm(&self, rpm: u16) -> u8 {
        (rpm.clamp(self.model.fan_rpm_min, self.model.fan_rpm_max) / 100) as u8
    }
}

/// Send a complete desired state once. The caller supplies the command
/// transport so every failure boundary can be exercised without real EC
/// hardware.
fn apply_state_once<C>(
    state: &AppliedState,
    model: &models::LaptopModel,
    command: &mut C,
) -> Result<(), String>
where
    C: FnMut(Report) -> Result<(), String>,
{
    let manual = matches!(&state.fan, FanMode::Manual { .. } | FanMode::Curve { .. });
    let mode = state.perf_mode.to_ec();
    // Silent maps to the EC's Custom mode with both boosts pinned Low:
    // the reduced power budget is what keeps the fans quiet, while the
    // EC's automatic fan curve (and its thermal failsafes) stay active.
    let boosts = match state.perf_mode {
        PerfMode::Silent => Some((Boost::Low, Boost::Low)),
        PerfMode::Custom => {
            let cpu = if model.has_cpu_boost_oc {
                state.cpu_boost
            } else {
                // Cap at High on models without the overclock level.
                match state.cpu_boost {
                    Boost::Boost => Boost::High,
                    b => b,
                }
            };
            Some((cpu, state.gpu_boost))
        }
        _ => None,
    };
    for zone in ZONES {
        command(packet::set_power_mode(zone, mode, manual))?;
        if manual {
            // Enter every software-controlled mode at the safest target. If
            // this write fails after manual mode was enabled, the outer
            // recovery immediately restores EC automatic control.
            let rpm = (model.fan_rpm_max / 100) as u8;
            command(packet::set_fan_rpm(zone, rpm))?;
        }
    }
    if let Some((cpu, gpu)) = boosts {
        command(packet::set_cpu_boost(cpu.to_ec()))?;
        command(packet::set_gpu_boost(gpu.to_ec()))?;
    }
    if model.has_bho {
        command(packet::set_bho(state.bho_enabled, state.bho_threshold))?;
    }

    // Lighting: brightness percent scaled to the EC's 0..=255 range,
    // then the keyboard hardware effect, then the logo LED.
    command(packet::set_brightness(
        (state.kbd_brightness.min(100) as u16 * 255 / 100) as u8,
    ))?;
    let (effect_id, params): (u8, Vec<u8>) = match state.kbd_effect {
        KbdEffect::Off => (packet::kbd_effect::OFF, vec![]),
        KbdEffect::Static { r, g, b } => (packet::kbd_effect::STATIC, vec![r, g, b]),
        KbdEffect::Spectrum => (packet::kbd_effect::SPECTRUM, vec![]),
        KbdEffect::Wave => (packet::kbd_effect::WAVE, vec![0x01]),
    };
    command(packet::set_kbd_effect(effect_id, &params))?;
    if model.has_logo {
        if state.logo_led != LogoMode::Off {
            let effect = match state.logo_led {
                LogoMode::Breathing => 0x02,
                _ => 0x00,
            };
            command(packet::set_logo_effect(effect))?;
        }
        command(packet::set_logo_state(state.logo_led != LogoMode::Off))?;
    }
    Ok(())
}

/// Restore both fan zones even if one command fails. Stopping at the first
/// error could otherwise leave one fan under manual control.
fn restore_auto_commands<C>(mode: PerfMode, command: &mut C) -> Result<(), String>
where
    C: FnMut(Report) -> Result<(), String>,
{
    let mut errors = Vec::new();
    for zone in ZONES {
        if let Err(e) = command(packet::set_power_mode(zone, mode.to_ec(), false)) {
            errors.push(format!("{zone:?}: {e}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// Apply a state with an immediate fan-safe fallback. The core subsequently
/// attempts to restore its complete previous state; this local recovery closes
/// the dangerous window where manual mode succeeded but its RPM write failed.
fn apply_state_with_recovery<C>(
    state: &AppliedState,
    model: &models::LaptopModel,
    mut command: C,
) -> Result<(), String>
where
    C: FnMut(Report) -> Result<(), String>,
{
    if let Err(apply_error) = apply_state_once(state, model, &mut command) {
        return match restore_auto_commands(state.perf_mode, &mut command) {
            Ok(()) => Err(format!("{apply_error}; restored EC automatic fan control")),
            Err(recovery_error) => Err(format!(
                "{apply_error}; restoring EC automatic fan control also failed: {recovery_error}"
            )),
        };
    }
    Ok(())
}

impl Hw for RazerHw {
    fn info(&self) -> ModelInfo {
        ModelInfo {
            name: self.name.clone(),
            device_present: true,
            verified: self.verified,
            mock: false,
            fan_rpm_min: self.model.fan_rpm_min,
            fan_rpm_max: self.model.fan_rpm_max,
            has_cpu_boost_oc: self.model.has_cpu_boost_oc,
            has_bho: self.model.has_bho,
            has_logo: self.model.has_logo,
        }
    }

    fn apply(&mut self, state: &AppliedState) -> Result<(), String> {
        apply_state_with_recovery(state, self.model, |report| self.command(report).map(|_| ()))
    }

    fn set_fan_target(&mut self, rpm: u16) -> Result<(), String> {
        let rpm = self.clamp_rpm(rpm);
        for zone in ZONES {
            self.command(packet::set_fan_rpm(zone, rpm))?;
        }
        Ok(())
    }

    fn restore_auto_fan(&mut self, perf_mode: PerfMode) -> Result<(), String> {
        restore_auto_commands(perf_mode, &mut |report| self.command(report).map(|_| ()))
    }

    fn sample(&mut self) -> Sample {
        let r = self.sensors.read();
        let mut fan_rpm = Vec::with_capacity(2);
        for zone in ZONES {
            match self.command(packet::get_fan_rpm(zone)) {
                Ok(resp) => fan_rpm.push(resp.args[2] as u32 * 100),
                Err(e) => log::debug!("fan rpm read ({zone:?}): {e}"),
            }
        }
        Sample {
            cpu_temp_c: r.cpu_temp_c,
            gpu_temp_c: r.gpu_temp_c,
            cpu_power_w: r.cpu_power_w,
            gpu_power_w: r.gpu_power_w,
            fan_rpm,
        }
    }
}

/// Fallback when no Razer USB device is found: telemetry without control.
pub struct MonitorOnly {
    sensors: Sensors,
}

impl MonitorOnly {
    pub fn new() -> MonitorOnly {
        MonitorOnly {
            sensors: Sensors::discover(),
        }
    }
}

impl Hw for MonitorOnly {
    fn info(&self) -> ModelInfo {
        ModelInfo {
            name: "No Razer device found".into(),
            device_present: false,
            verified: false,
            mock: false,
            fan_rpm_min: 0,
            fan_rpm_max: 0,
            has_cpu_boost_oc: false,
            has_bho: false,
            has_logo: false,
        }
    }

    fn apply(&mut self, _state: &AppliedState) -> Result<(), String> {
        Err("no Razer laptop device present".into())
    }

    fn set_fan_target(&mut self, _rpm: u16) -> Result<(), String> {
        Err("no Razer laptop device present".into())
    }

    fn restore_auto_fan(&mut self, _perf_mode: PerfMode) -> Result<(), String> {
        Ok(())
    }

    fn sample(&mut self) -> Sample {
        let r = self.sensors.read();
        Sample {
            cpu_temp_c: r.cpu_temp_c,
            gpu_temp_c: r.gpu_temp_c,
            cpu_power_w: r.cpu_power_w,
            gpu_power_w: r.gpu_power_w,
            fan_rpm: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_state_once, apply_state_with_recovery, choose_laptop, parse_approved_pid,
        restore_auto_commands,
    };
    use crate::state::AppliedState;
    use fang_protocol::api::{FanMode, PerfMode};
    use fang_protocol::models;
    use fang_protocol::packet::{Report, Zone};

    // 0x02B8 = Blade 18 2024 (in the model table); 0x0084 / 0x0067 are Razer
    // mouse PIDs (not in the table); 0x9999 stands in for an unlisted laptop.
    #[test]
    fn prefers_known_laptop_over_attached_peripheral() {
        // mouse enumerates first, laptop second — the laptop must still win
        assert_eq!(
            choose_laptop(&[(0x0084, 0x0001), (0x02B8, 0xFF00)], None),
            Some(1)
        );
    }

    #[test]
    fn known_laptop_selected_even_without_vendor_usage_page() {
        assert_eq!(choose_laptop(&[(0x02B8, 0x0000)], None), Some(0));
    }

    #[test]
    fn unlisted_laptop_defaults_to_monitor_only() {
        assert_eq!(
            choose_laptop(&[(0x0084, 0x0001), (0x9999, 0xFF00)], None),
            None
        );
    }

    #[test]
    fn exact_unlisted_pid_can_be_explicitly_approved() {
        assert_eq!(
            choose_laptop(
                &[(0x0084, 0x0001), (0x9999, 0xFF00), (0xaaaa, 0xFF00)],
                Some(0x9999)
            ),
            Some(1)
        );
    }

    #[test]
    fn known_laptop_wins_over_an_approved_unknown_device() {
        assert_eq!(
            choose_laptop(&[(0x9999, 0xFF00), (0x02B8, 0x0000)], Some(0x9999)),
            Some(1)
        );
    }

    #[test]
    fn approval_requires_a_vendor_defined_interface() {
        assert_eq!(choose_laptop(&[(0x9999, 0x0001)], Some(0x9999)), None);
    }

    #[test]
    fn parses_exact_hex_pid_approval() {
        assert_eq!(parse_approved_pid("02b8").unwrap(), 0x02b8);
        assert_eq!(parse_approved_pid("0X9999").unwrap(), 0x9999);
        assert!(parse_approved_pid("true").is_err());
        assert!(parse_approved_pid("0000").is_err());
        assert!(parse_approved_pid("12345").is_err());
    }

    #[test]
    fn peripherals_only_selects_nothing() {
        // only a Razer mouse/keyboard present — never open one as a laptop
        assert_eq!(
            choose_laptop(&[(0x0084, 0x0001), (0x0067, 0x0001)], None),
            None
        );
    }

    fn manual_state() -> AppliedState {
        AppliedState {
            perf_mode: PerfMode::Custom,
            fan: FanMode::Manual { rpm: 3000 },
            ..AppliedState::default()
        }
    }

    fn assert_auto_report(report: &Report, zone: Zone) {
        assert_eq!((report.command_class, report.command_id), (0x0d, 0x02));
        assert_eq!(report.args[1], zone as u8);
        assert_eq!(report.args[3], 0, "manual flag must be cleared");
    }

    #[test]
    fn every_apply_failure_boundary_restores_both_fans_to_auto() {
        let state = manual_state();
        let model = models::LaptopModel {
            pid: 0,
            name: "failure-injection model",
            fan_rpm_min: 2200,
            fan_rpm_max: 5000,
            has_cpu_boost_oc: true,
            has_bho: true,
            has_logo: true,
        };
        let mut forward = Vec::new();
        apply_state_once(&state, &model, &mut |report| {
            forward.push(report);
            Ok(())
        })
        .unwrap();
        assert!(!forward.is_empty());

        for fail_at in 0..forward.len() {
            let mut seen = Vec::new();
            let mut call = 0;
            let result = apply_state_with_recovery(&state, &model, |report| {
                seen.push(report);
                let this_call = call;
                call += 1;
                if this_call == fail_at {
                    Err(format!("injected failure {fail_at}"))
                } else {
                    Ok(())
                }
            });

            let error = result.expect_err("injected apply failure must surface");
            assert!(
                error.contains("restored EC automatic fan control"),
                "{error}"
            );
            assert_eq!(seen.len(), fail_at + 3);
            assert_auto_report(&seen[seen.len() - 2], Zone::Fan1);
            assert_auto_report(&seen[seen.len() - 1], Zone::Fan2);
        }
    }

    #[test]
    fn auto_recovery_attempts_the_second_zone_when_the_first_fails() {
        let mut seen = Vec::new();
        let mut call = 0;
        let result = restore_auto_commands(PerfMode::Balanced, &mut |report| {
            seen.push(report);
            call += 1;
            if call == 1 {
                Err("fan 1 unavailable".into())
            } else {
                Ok(())
            }
        });

        let error = result.expect_err("first zone failure must surface");
        assert!(error.contains("Fan1"), "{error}");
        assert_eq!(seen.len(), 2);
        assert_auto_report(&seen[0], Zone::Fan1);
        assert_auto_report(&seen[1], Zone::Fan2);
    }
}
