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

/// Choose the laptop among Razer HID candidates on interface 0, each given as
/// `(product_id, usage_page)`. Prefers a device whose PID is a recognized
/// laptop; otherwise falls back to one presenting a vendor-defined usage page
/// (the EC control interface), so an unlisted Blade still works while a Razer
/// mouse or keyboard (generic-desktop usage page) is never selected. Returns
/// the index into `candidates`, or `None` when only peripherals are present.
fn choose_laptop(candidates: &[(u16, u16)]) -> Option<usize> {
    candidates
        .iter()
        .position(|&(pid, _)| models::by_pid(pid).is_some())
        .or_else(|| {
            candidates
                .iter()
                .position(|&(_, usage_page)| usage_page >= 0xFF00)
        })
}

impl RazerHw {
    pub fn open() -> Result<RazerHw, String> {
        let api = HidApi::new().map_err(|e| format!("hidapi init: {e}"))?;
        // A Razer mouse or keyboard also has vendor id 0x1532 and presents an
        // interface 0, so the first match isn't necessarily the laptop — and
        // firing class-0x0d EC commands at a mouse is not what we want. Collect
        // every candidate, then let `choose_laptop` prefer a recognized laptop
        // PID and otherwise fall back to a vendor-usage-page EC interface.
        let candidates: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == RAZER_VID && d.interface_number() == 0)
            .collect();
        let sig: Vec<(u16, u16)> = candidates
            .iter()
            .map(|d| (d.product_id(), d.usage_page()))
            .collect();
        let dev_info = choose_laptop(&sig).map(|i| candidates[i]).ok_or_else(|| {
            if candidates.is_empty() {
                "no USB device with Razer vendor id 0x1532".to_string()
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
        log::info!("found {name}");
        Ok(RazerHw {
            device,
            model,
            verified,
            name,
            sensors: Sensors::discover(),
        })
    }

    /// Send one report and read back the EC's answer. One retry on busy.
    fn command(&self, report: Report) -> Result<Report, String> {
        for attempt in 0..2 {
            let buf = report.to_feature_report();
            self.device
                .send_feature_report(&buf)
                .map_err(|e| format!("send_feature_report: {e}"))?;
            thread::sleep(Duration::from_micros(1500));

            let mut resp = [0u8; REPORT_LEN];
            self.device
                .get_feature_report(&mut resp)
                .map_err(|e| format!("get_feature_report: {e}"))?;
            let parsed = Report::from_feature_report(&resp)
                .ok_or_else(|| "short feature report response".to_string())?;

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
            has_creator_mode: self.model.has_creator_mode,
            has_logo: self.model.has_logo,
        }
    }

    fn apply(&mut self, state: &AppliedState) -> Result<(), String> {
        let manual = matches!(state.fan, FanMode::Manual { .. });
        let mode = state.perf_mode.to_ec();
        // Silent maps to the EC's Custom mode with both boosts pinned Low:
        // the reduced power budget is what keeps the fans quiet, while the
        // EC's automatic fan curve (and its thermal failsafes) stay active.
        let boosts = match state.perf_mode {
            PerfMode::Silent => Some((Boost::Low, Boost::Low)),
            PerfMode::Custom => {
                let cpu = if self.model.has_cpu_boost_oc {
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
            self.command(packet::set_power_mode(zone, mode, manual))?;
            if let FanMode::Manual { rpm } = state.fan {
                self.command(packet::set_fan_rpm(zone, self.clamp_rpm(rpm)))?;
            }
        }
        if let Some((cpu, gpu)) = boosts {
            self.command(packet::set_cpu_boost(cpu.to_ec()))?;
            self.command(packet::set_gpu_boost(gpu.to_ec()))?;
        }
        if self.model.has_bho {
            self.command(packet::set_bho(state.bho_enabled, state.bho_threshold))?;
        }

        // Lighting: brightness percent scaled to the EC's 0..=255 range,
        // then the keyboard hardware effect, then the logo LED.
        self.command(packet::set_brightness(
            (state.kbd_brightness.min(100) as u16 * 255 / 100) as u8,
        ))?;
        let (effect_id, params): (u8, Vec<u8>) = match state.kbd_effect {
            KbdEffect::Off => (packet::kbd_effect::OFF, vec![]),
            KbdEffect::Static { r, g, b } => (packet::kbd_effect::STATIC, vec![r, g, b]),
            KbdEffect::Spectrum => (packet::kbd_effect::SPECTRUM, vec![]),
            KbdEffect::Wave => (packet::kbd_effect::WAVE, vec![0x01]),
        };
        self.command(packet::set_kbd_effect(effect_id, &params))?;
        if self.model.has_logo {
            if state.logo_led != LogoMode::Off {
                let effect = match state.logo_led {
                    LogoMode::Breathing => 0x02,
                    _ => 0x00,
                };
                self.command(packet::set_logo_effect(effect))?;
            }
            self.command(packet::set_logo_state(state.logo_led != LogoMode::Off))?;
        }
        Ok(())
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
            has_creator_mode: false,
            has_logo: false,
        }
    }

    fn apply(&mut self, _state: &AppliedState) -> Result<(), String> {
        Err("no Razer laptop device present".into())
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
    use super::choose_laptop;

    // 0x02B8 = Blade 18 2024 (in the model table); 0x0084 / 0x0067 are Razer
    // mouse PIDs (not in the table); 0x9999 stands in for an unlisted laptop.
    #[test]
    fn prefers_known_laptop_over_attached_peripheral() {
        // mouse enumerates first, laptop second — the laptop must still win
        assert_eq!(
            choose_laptop(&[(0x0084, 0x0001), (0x02B8, 0xFF00)]),
            Some(1)
        );
    }

    #[test]
    fn known_laptop_selected_even_without_vendor_usage_page() {
        assert_eq!(choose_laptop(&[(0x02B8, 0x0000)]), Some(0));
    }

    #[test]
    fn unlisted_laptop_falls_back_to_ec_usage_page() {
        assert_eq!(
            choose_laptop(&[(0x0084, 0x0001), (0x9999, 0xFF00)]),
            Some(1)
        );
    }

    #[test]
    fn peripherals_only_selects_nothing() {
        // only a Razer mouse/keyboard present — never open one as a laptop
        assert_eq!(choose_laptop(&[(0x0084, 0x0001), (0x0067, 0x0001)]), None);
    }
}
