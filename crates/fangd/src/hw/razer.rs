//! Real hardware backend: Razer EC over hidraw (Linux only).

use super::{sensors::Sensors, Hw, ModelInfo, Sample};
use crate::state::AppliedState;
use fang_protocol::api::{FanMode, PerfMode};
use fang_protocol::models;
use fang_protocol::packet::{self, Report, Zone, RAZER_VID, REPORT_LEN, ZONES};
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

impl RazerHw {
    pub fn open() -> Result<RazerHw, String> {
        let api = HidApi::new().map_err(|e| format!("hidapi init: {e}"))?;
        let dev_info = api
            .device_list()
            .find(|d| d.vendor_id() == RAZER_VID && d.interface_number() == 0)
            .ok_or_else(|| "no USB device with Razer vendor id 0x1532".to_string())?;
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
        }
    }

    fn apply(&mut self, state: &AppliedState) -> Result<(), String> {
        let manual = matches!(state.fan, FanMode::Manual { .. });
        let mode = state.perf_mode.to_ec();
        for zone in ZONES {
            self.command(packet::set_power_mode(zone, mode, manual))?;
            if let FanMode::Manual { rpm } = state.fan {
                self.command(packet::set_fan_rpm(zone, self.clamp_rpm(rpm)))?;
            }
        }
        if state.perf_mode == PerfMode::Custom {
            let cpu = if self.model.has_cpu_boost_oc {
                state.cpu_boost
            } else {
                // Cap at High on models without the overclock level.
                match state.cpu_boost {
                    fang_protocol::api::Boost::Boost => fang_protocol::api::Boost::High,
                    b => b,
                }
            };
            self.command(packet::set_cpu_boost(cpu.to_ec()))?;
            self.command(packet::set_gpu_boost(state.gpu_boost.to_ec()))?;
        }
        Ok(())
    }

    fn sample(&mut self) -> Sample {
        let (cpu_temp_c, gpu_temp_c) = self.sensors.temps();
        let mut fan_rpm = Vec::with_capacity(2);
        for zone in ZONES {
            match self.command(packet::get_fan_rpm(zone)) {
                Ok(resp) => fan_rpm.push(resp.args[2] as u32 * 100),
                Err(e) => log::debug!("fan rpm read ({zone:?}): {e}"),
            }
        }
        Sample {
            cpu_temp_c,
            gpu_temp_c,
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
        }
    }

    fn apply(&mut self, _state: &AppliedState) -> Result<(), String> {
        Err("no Razer laptop device present".into())
    }

    fn sample(&mut self) -> Sample {
        let (cpu_temp_c, gpu_temp_c) = self.sensors.temps();
        Sample {
            cpu_temp_c,
            gpu_temp_c,
            fan_rpm: vec![],
        }
    }
}
