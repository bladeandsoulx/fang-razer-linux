//! Razer EC HID feature-report packets.
//!
//! Every command is a 90-byte report sent as a HID feature report with a
//! leading report number of `0x00`, i.e. a 91-byte buffer on the wire:
//!
//! ```text
//! [0]     report number (always 0x00)
//! [1]     status        (0x00 new command; responses: 0x02 ok, ...)
//! [2]     transaction id (0x1F for Blade laptops)
//! [3..5]  remaining packets (always 0)
//! [5]     protocol type (always 0)
//! [6]     data size     (number of meaningful arg bytes)
//! [7]     command class
//! [8]     command id    (get variants = set id | 0x80)
//! [9..89] args (80 bytes)
//! [89]    crc = XOR of bytes 3 through 88 (inclusive)
//! [90]    reserved (0)
//! ```

pub const RAZER_VID: u16 = 0x1532;
pub const REPORT_LEN: usize = 91;
const ARGS_LEN: usize = 80;
const TRANSACTION_ID: u8 = 0x1F;

/// Why a feature report could not be accepted as a response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReportError {
    Length {
        actual: usize,
    },
    ReportId {
        actual: u8,
    },
    RemainingPackets {
        actual: u16,
    },
    ProtocolType {
        actual: u8,
    },
    DataSize {
        actual: u8,
    },
    Checksum {
        expected: u8,
        actual: u8,
    },
    TransactionId {
        expected: u8,
        actual: u8,
    },
    Command {
        expected_class: u8,
        expected_id: u8,
        actual_class: u8,
        actual_id: u8,
    },
    ResponseDataSize {
        expected: u8,
        actual: u8,
    },
}

impl std::fmt::Display for ReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportError::Length { actual } => {
                write!(f, "feature report length {actual}, expected {REPORT_LEN}")
            }
            ReportError::ReportId { actual } => {
                write!(f, "feature report id {actual:#04x}, expected 0x00")
            }
            ReportError::RemainingPackets { actual } => {
                write!(f, "multi-packet response is unsupported ({actual} remaining)")
            }
            ReportError::ProtocolType { actual } => {
                write!(f, "feature report protocol {actual:#04x}, expected 0x00")
            }
            ReportError::DataSize { actual } => {
                write!(f, "feature report data size {actual} exceeds {ARGS_LEN}")
            }
            ReportError::Checksum { expected, actual } => write!(
                f,
                "feature report CRC {actual:#04x}, expected {expected:#04x}"
            ),
            ReportError::TransactionId { expected, actual } => write!(
                f,
                "response transaction id {actual:#04x}, expected {expected:#04x}"
            ),
            ReportError::Command {
                expected_class,
                expected_id,
                actual_class,
                actual_id,
            } => write!(
                f,
                "response command {actual_class:#04x}/{actual_id:#04x}, expected {expected_class:#04x}/{expected_id:#04x}"
            ),
            ReportError::ResponseDataSize { expected, actual } => write!(
                f,
                "response data size {actual}, expected {expected}"
            ),
        }
    }
}

impl std::error::Error for ReportError {}

/// Response status bytes from the EC.
pub mod status {
    pub const NEW: u8 = 0x00;
    pub const BUSY: u8 = 0x01;
    pub const SUCCESS: u8 = 0x02;
    pub const FAILURE: u8 = 0x03;
    pub const TIMEOUT: u8 = 0x04;
    pub const NOT_SUPPORTED: u8 = 0x05;
}

/// Fan/power zones. Blades drive two fans; per-zone commands are sent to both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Fan1 = 0x01,
    Fan2 = 0x02,
}

pub const ZONES: [Zone; 2] = [Zone::Fan1, Zone::Fan2];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    pub status: u8,
    pub transaction_id: u8,
    pub data_size: u8,
    pub command_class: u8,
    pub command_id: u8,
    pub args: [u8; ARGS_LEN],
}

impl Report {
    pub fn new(command_class: u8, command_id: u8, args: &[u8]) -> Self {
        debug_assert!(args.len() <= ARGS_LEN);
        let mut a = [0u8; ARGS_LEN];
        a[..args.len()].copy_from_slice(args);
        Report {
            status: status::NEW,
            transaction_id: TRANSACTION_ID,
            data_size: args.len() as u8,
            command_class,
            command_id,
            args: a,
        }
    }

    /// Serialize to the 91-byte buffer passed to `send_feature_report`.
    pub fn to_feature_report(&self) -> [u8; REPORT_LEN] {
        let mut buf = [0u8; REPORT_LEN];
        buf[0] = 0x00; // report number
        buf[1] = self.status;
        buf[2] = self.transaction_id;
        // buf[3..5] remaining packets = 0, buf[5] protocol type = 0
        buf[6] = self.data_size;
        buf[7] = self.command_class;
        buf[8] = self.command_id;
        buf[9..9 + ARGS_LEN].copy_from_slice(&self.args);
        buf[89] = crc(&buf);
        buf
    }

    /// Parse and validate a complete buffer returned by `get_feature_report`.
    pub fn from_feature_report(buf: &[u8]) -> Result<Report, ReportError> {
        if buf.len() != REPORT_LEN {
            return Err(ReportError::Length { actual: buf.len() });
        }
        if buf[0] != 0 {
            return Err(ReportError::ReportId { actual: buf[0] });
        }
        let remaining = u16::from_be_bytes([buf[3], buf[4]]);
        if remaining != 0 {
            return Err(ReportError::RemainingPackets { actual: remaining });
        }
        if buf[5] != 0 {
            return Err(ReportError::ProtocolType { actual: buf[5] });
        }
        if usize::from(buf[6]) > ARGS_LEN {
            return Err(ReportError::DataSize { actual: buf[6] });
        }
        let wire: &[u8; REPORT_LEN] = buf.try_into().expect("length checked above");
        let expected_crc = crc(wire);
        if buf[89] != expected_crc {
            return Err(ReportError::Checksum {
                expected: expected_crc,
                actual: buf[89],
            });
        }
        let mut args = [0u8; ARGS_LEN];
        args.copy_from_slice(&buf[9..9 + ARGS_LEN]);
        Ok(Report {
            status: buf[1],
            transaction_id: buf[2],
            data_size: buf[6],
            command_class: buf[7],
            command_id: buf[8],
            args,
        })
    }

    /// Parse a feature report and prove that it answers `request` rather than
    /// accepting a stale or unrelated EC response.
    pub fn response_from_feature_report(
        request: &Report,
        buf: &[u8],
    ) -> Result<Report, ReportError> {
        let response = Report::from_feature_report(buf)?;
        if response.transaction_id != request.transaction_id {
            return Err(ReportError::TransactionId {
                expected: request.transaction_id,
                actual: response.transaction_id,
            });
        }
        if !response.answers(request) {
            return Err(ReportError::Command {
                expected_class: request.command_class,
                expected_id: request.command_id,
                actual_class: response.command_class,
                actual_id: response.command_id,
            });
        }
        if response.data_size != request.data_size {
            return Err(ReportError::ResponseDataSize {
                expected: request.data_size,
                actual: response.data_size,
            });
        }
        Ok(response)
    }

    /// True when a response's class/id matches the request it answers.
    pub fn answers(&self, request: &Report) -> bool {
        self.command_class == request.command_class && self.command_id == request.command_id
    }
}

/// XOR checksum over bytes 3..=88 of the HID wire buffer. The Razer report
/// itself starts at wire byte 1 (after the HID report number), and the
/// reference protocol checks its bytes 2..=87: remaining packets through the
/// final argument. Status and transaction ID are deliberately not included.
pub fn crc(buf: &[u8; REPORT_LEN]) -> u8 {
    buf[3..89].iter().fold(0, |acc, b| acc ^ b)
}

// ---- EC commands (class 0x0d: performance / thermals) ----------------------

/// mode: 0 balanced, 1 gaming, 4 custom. Values 2 and 3 are not exposed.
/// `manual_fan` selects manual fan RPM control instead of the EC fan curve.
pub fn set_power_mode(zone: Zone, mode: u8, manual_fan: bool) -> Report {
    Report::new(0x0d, 0x02, &[0x00, zone as u8, mode, manual_fan as u8])
}

pub fn get_power_mode(zone: Zone) -> Report {
    Report::new(0x0d, 0x82, &[0x00, zone as u8, 0x00, 0x00])
}

/// `rpm_div_100`: target RPM divided by 100 (e.g. 4400 RPM -> 44).
pub fn set_fan_rpm(zone: Zone, rpm_div_100: u8) -> Report {
    Report::new(0x0d, 0x01, &[0x00, zone as u8, rpm_div_100])
}

pub fn get_fan_rpm(zone: Zone) -> Report {
    Report::new(0x0d, 0x81, &[0x00, zone as u8, 0x00])
}

/// boost: 0 low, 1 medium, 2 high, 3 boost (CPU only, models with the
/// "boost" feature). Only meaningful in custom power mode (4).
pub fn set_cpu_boost(boost: u8) -> Report {
    Report::new(0x0d, 0x07, &[0x00, 0x01, boost])
}

pub fn get_cpu_boost() -> Report {
    Report::new(0x0d, 0x87, &[0x00, 0x01, 0x00])
}

pub fn set_gpu_boost(boost: u8) -> Report {
    Report::new(0x0d, 0x07, &[0x00, 0x02, boost])
}

pub fn get_gpu_boost() -> Report {
    Report::new(0x0d, 0x87, &[0x00, 0x02, 0x00])
}

// ---- EC commands (class 0x03: lighting) -------------------------------------

const VARSTORE: u8 = 0x01;
const LOGO_LED: u8 = 0x04;
const BACKLIGHT_LED: u8 = 0x05;

/// Keyboard hardware effect ids (arg 0 of [`set_kbd_effect`]). Only the ids
/// the reference implementation actively exercises are exposed.
pub mod kbd_effect {
    pub const OFF: u8 = 0x00;
    /// param: direction (1 = left-to-right, 2 = right-to-left)
    pub const WAVE: u8 = 0x01;
    pub const SPECTRUM: u8 = 0x04;
    /// params: r, g, b
    pub const STATIC: u8 = 0x06;
}

/// Keyboard backlight brightness, 0..=255.
pub fn set_brightness(value: u8) -> Report {
    Report::new(0x03, 0x03, &[VARSTORE, BACKLIGHT_LED, value])
}

pub fn get_brightness() -> Report {
    Report::new(0x03, 0x83, &[VARSTORE, BACKLIGHT_LED, 0x00])
}

/// Logo LED on/off. When turning on, send [`set_logo_effect`] first.
pub fn set_logo_state(on: bool) -> Report {
    Report::new(0x03, 0x00, &[VARSTORE, LOGO_LED, on as u8])
}

/// Logo LED effect: 0x00 static, 0x02 breathing.
pub fn set_logo_effect(effect: u8) -> Report {
    Report::new(0x03, 0x02, &[VARSTORE, LOGO_LED, effect])
}

/// Keyboard hardware effect. The reference implementation always declares
/// the full 80-byte args payload for this command, so mirror that.
pub fn set_kbd_effect(effect_id: u8, params: &[u8]) -> Report {
    debug_assert!(params.len() < ARGS_LEN);
    let mut args = [0u8; ARGS_LEN];
    args[0] = effect_id;
    args[1..1 + params.len()].copy_from_slice(params);
    Report::new(0x03, 0x0a, &args)
}

// ---- EC commands (class 0x07: battery) --------------------------------------

/// Battery Health Optimizer (Synapse's charge limiter). One arg byte: top
/// bit = enabled, low 7 bits = charge threshold percent (Synapse offers
/// 50..=80). Byte layout from Razer-Control's device.rs (GPL-2.0).
pub fn set_bho(enabled: bool, threshold: u8) -> Report {
    Report::new(0x07, 0x12, &[((enabled as u8) << 7) | (threshold & 0x7F)])
}

pub fn get_bho() -> Report {
    Report::new(0x07, 0x92, &[0x00])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_mode_packet_bytes() {
        let buf = set_power_mode(Zone::Fan1, 1, false).to_feature_report();
        assert_eq!(buf.len(), REPORT_LEN);
        assert_eq!(buf[0], 0x00, "report number");
        assert_eq!(buf[1], 0x00, "status new");
        assert_eq!(buf[2], 0x1F, "transaction id");
        assert_eq!(&buf[3..6], &[0, 0, 0], "remaining + protocol");
        assert_eq!(buf[6], 0x04, "data size");
        assert_eq!(buf[7], 0x0d, "command class");
        assert_eq!(buf[8], 0x02, "command id");
        assert_eq!(&buf[9..13], &[0x00, 0x01, 0x01, 0x00], "args");
        assert!(buf[13..89].iter().all(|&b| b == 0));
        // 0x04 ^ 0x0d ^ 0x02 ^ 0x01 ^ 0x01
        assert_eq!(buf[89], 0x0B, "crc");
        assert_eq!(buf[90], 0x00, "reserved");
    }

    #[test]
    fn fan_rpm_packet_bytes() {
        // 4400 RPM -> 44 = 0x2C
        let buf = set_fan_rpm(Zone::Fan1, 44).to_feature_report();
        assert_eq!(buf[6], 0x03);
        assert_eq!(buf[7], 0x0d);
        assert_eq!(buf[8], 0x01);
        assert_eq!(&buf[9..12], &[0x00, 0x01, 0x2C]);
        // 0x03 ^ 0x0d ^ 0x01 ^ 0x01 ^ 0x2C
        assert_eq!(buf[89], 0x22);
    }

    #[test]
    fn cpu_boost_packet_bytes() {
        let buf = set_cpu_boost(2).to_feature_report();
        assert_eq!(buf[6], 0x03);
        assert_eq!(buf[7], 0x0d);
        assert_eq!(buf[8], 0x07);
        assert_eq!(&buf[9..12], &[0x00, 0x01, 0x02]);
        // 0x03 ^ 0x0d ^ 0x07 ^ 0x01 ^ 0x02
        assert_eq!(buf[89], 0x0A);
    }

    #[test]
    fn get_commands_set_high_bit() {
        assert_eq!(get_power_mode(Zone::Fan2).command_id, 0x82);
        assert_eq!(get_fan_rpm(Zone::Fan1).command_id, 0x81);
        assert_eq!(get_cpu_boost().command_id, 0x87);
        assert_eq!(get_gpu_boost().command_id, 0x87);
        assert_eq!(get_bho().command_id, 0x92);
    }

    #[test]
    fn lighting_packet_bytes() {
        let buf = set_brightness(153).to_feature_report();
        assert_eq!(&buf[6..9], &[0x03, 0x03, 0x03], "size/class/id");
        assert_eq!(&buf[9..12], &[0x01, 0x05, 153], "varstore/backlight/value");

        let buf = set_logo_state(true).to_feature_report();
        assert_eq!(&buf[6..9], &[0x03, 0x03, 0x00]);
        assert_eq!(&buf[9..12], &[0x01, 0x04, 0x01]);

        let buf = set_logo_effect(0x02).to_feature_report();
        assert_eq!(&buf[6..9], &[0x03, 0x03, 0x02]);
        assert_eq!(&buf[9..12], &[0x01, 0x04, 0x02]);

        // static razer green; full 80-byte payload like the reference
        let buf = set_kbd_effect(kbd_effect::STATIC, &[0x44, 0xD6, 0x2C]).to_feature_report();
        assert_eq!(buf[6], 80, "data size");
        assert_eq!(&buf[7..9], &[0x03, 0x0a]);
        assert_eq!(&buf[9..13], &[0x06, 0x44, 0xD6, 0x2C]);
        assert!(buf[13..89].iter().all(|&b| b == 0));
    }

    #[test]
    fn bho_packet_bytes() {
        // enabled at 80% -> 0x80 | 80 = 0xD0
        let buf = set_bho(true, 80).to_feature_report();
        assert_eq!(buf[6], 0x01, "data size");
        assert_eq!(buf[7], 0x07, "command class");
        assert_eq!(buf[8], 0x12, "command id");
        assert_eq!(buf[9], 0xD0, "args[0]");
        assert!(buf[10..89].iter().all(|&b| b == 0));
        // 0x01 ^ 0x07 ^ 0x12 ^ 0xD0
        assert_eq!(buf[89], 0xC4, "crc");

        // disabled keeps the threshold in the low bits
        assert_eq!(set_bho(false, 65).args[0], 65);
        // threshold can never bleed into the enable bit
        assert_eq!(set_bho(false, 0xFF).args[0], 0x7F);
    }

    #[test]
    fn response_roundtrip() {
        let req = get_fan_rpm(Zone::Fan1);
        let mut wire = req.to_feature_report();
        wire[1] = status::SUCCESS;
        wire[11] = 44; // EC echoes rpm/100 in args[2]
        wire[89] = crc(&wire);
        let resp = Report::response_from_feature_report(&req, &wire).unwrap();
        assert_eq!(resp.status, status::SUCCESS);
        assert!(resp.answers(&req));
        assert_eq!(resp.args[2], 44);
    }

    fn successful_response(request: &Report) -> [u8; REPORT_LEN] {
        let mut wire = request.to_feature_report();
        wire[1] = status::SUCCESS;
        wire
    }

    #[test]
    fn response_rejects_short_and_oversized_reports() {
        let req = get_fan_rpm(Zone::Fan1);
        let wire = successful_response(&req);
        assert_eq!(
            Report::response_from_feature_report(&req, &wire[..REPORT_LEN - 1]),
            Err(ReportError::Length {
                actual: REPORT_LEN - 1
            })
        );
        let mut oversized = wire.to_vec();
        oversized.push(0);
        assert_eq!(
            Report::response_from_feature_report(&req, &oversized),
            Err(ReportError::Length {
                actual: REPORT_LEN + 1
            })
        );
    }

    #[test]
    fn response_rejects_bad_crc() {
        let req = get_fan_rpm(Zone::Fan1);
        let mut wire = successful_response(&req);
        wire[11] = 44;
        let err = Report::response_from_feature_report(&req, &wire).unwrap_err();
        assert!(matches!(err, ReportError::Checksum { .. }));
    }

    #[test]
    fn response_rejects_wrong_transaction_command_and_size() {
        let req = get_fan_rpm(Zone::Fan1);

        let mut wrong_transaction = successful_response(&req);
        wrong_transaction[2] ^= 1;
        wrong_transaction[89] = crc(&wrong_transaction);
        assert!(matches!(
            Report::response_from_feature_report(&req, &wrong_transaction),
            Err(ReportError::TransactionId { .. })
        ));

        let mut wrong_command = successful_response(&req);
        wrong_command[8] ^= 1;
        wrong_command[89] = crc(&wrong_command);
        assert!(matches!(
            Report::response_from_feature_report(&req, &wrong_command),
            Err(ReportError::Command { .. })
        ));

        let mut wrong_size = successful_response(&req);
        wrong_size[6] -= 1;
        wrong_size[89] = crc(&wrong_size);
        assert!(matches!(
            Report::response_from_feature_report(&req, &wrong_size),
            Err(ReportError::ResponseDataSize { .. })
        ));
    }

    #[test]
    fn response_rejects_invalid_framing() {
        let req = get_fan_rpm(Zone::Fan1);

        let mut wrong_report_id = successful_response(&req);
        wrong_report_id[0] = 1;
        assert!(matches!(
            Report::response_from_feature_report(&req, &wrong_report_id),
            Err(ReportError::ReportId { .. })
        ));

        let mut packets_remaining = successful_response(&req);
        packets_remaining[4] = 1;
        packets_remaining[89] = crc(&packets_remaining);
        assert!(matches!(
            Report::response_from_feature_report(&req, &packets_remaining),
            Err(ReportError::RemainingPackets { .. })
        ));

        let mut wrong_protocol = successful_response(&req);
        wrong_protocol[5] = 1;
        wrong_protocol[89] = crc(&wrong_protocol);
        assert!(matches!(
            Report::response_from_feature_report(&req, &wrong_protocol),
            Err(ReportError::ProtocolType { .. })
        ));

        let mut oversized_data = successful_response(&req);
        oversized_data[6] = (ARGS_LEN + 1) as u8;
        oversized_data[89] = crc(&oversized_data);
        assert!(matches!(
            Report::response_from_feature_report(&req, &oversized_data),
            Err(ReportError::DataSize { .. })
        ));
    }

    #[test]
    fn crc_uses_razer_payload_after_hid_and_transaction_headers() {
        let mut buf = set_cpu_boost(2).to_feature_report();
        let before = crc(&buf);
        buf[0] = 0xCC; // HID report number
        buf[1] = 0x02; // status
        buf[2] = 0xA5; // transaction ID
        buf[89] = 0xAA; // crc byte itself
        buf[90] = 0xBB; // reserved
        assert_eq!(crc(&buf), before);

        buf[88] ^= 1; // final argument is covered
        assert_ne!(crc(&buf), before);
    }
}
