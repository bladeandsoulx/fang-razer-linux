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
//! [89]    crc = XOR of bytes 2..88
//! [90]    reserved (0)
//! ```

pub const RAZER_VID: u16 = 0x1532;
pub const REPORT_LEN: usize = 91;
const ARGS_LEN: usize = 80;
const TRANSACTION_ID: u8 = 0x1F;

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

    /// Parse a 91-byte buffer returned by `get_feature_report`.
    pub fn from_feature_report(buf: &[u8]) -> Option<Report> {
        if buf.len() < REPORT_LEN {
            return None;
        }
        let mut args = [0u8; ARGS_LEN];
        args.copy_from_slice(&buf[9..9 + ARGS_LEN]);
        Some(Report {
            status: buf[1],
            transaction_id: buf[2],
            data_size: buf[6],
            command_class: buf[7],
            command_id: buf[8],
            args,
        })
    }

    /// True when a response's class/id matches the request it answers.
    pub fn answers(&self, request: &Report) -> bool {
        self.command_class == request.command_class && self.command_id == request.command_id
    }
}

/// XOR checksum over bytes 2..88 of the wire buffer (matches the reference
/// implementation; the EC rejects reports with a bad CRC).
pub fn crc(buf: &[u8; REPORT_LEN]) -> u8 {
    buf[2..88].iter().fold(0, |acc, b| acc ^ b)
}

// ---- EC commands (class 0x0d: performance / thermals) ----------------------

/// mode: 0 balanced, 1 gaming, 2 creator, 3 silent, 4 custom.
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
        // 0x1F ^ 0x04 ^ 0x0d ^ 0x02 ^ 0x01 ^ 0x01
        assert_eq!(buf[89], 0x14, "crc");
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
        // 0x1F ^ 0x03 ^ 0x0d ^ 0x01 ^ 0x01 ^ 0x2C
        assert_eq!(buf[89], 0x3D);
    }

    #[test]
    fn cpu_boost_packet_bytes() {
        let buf = set_cpu_boost(2).to_feature_report();
        assert_eq!(buf[6], 0x03);
        assert_eq!(buf[7], 0x0d);
        assert_eq!(buf[8], 0x07);
        assert_eq!(&buf[9..12], &[0x00, 0x01, 0x02]);
        // 0x1F ^ 0x03 ^ 0x0d ^ 0x07 ^ 0x01 ^ 0x02
        assert_eq!(buf[89], 0x15);
    }

    #[test]
    fn get_commands_set_high_bit() {
        assert_eq!(get_power_mode(Zone::Fan2).command_id, 0x82);
        assert_eq!(get_fan_rpm(Zone::Fan1).command_id, 0x81);
        assert_eq!(get_cpu_boost().command_id, 0x87);
        assert_eq!(get_gpu_boost().command_id, 0x87);
    }

    #[test]
    fn response_roundtrip() {
        let req = get_fan_rpm(Zone::Fan1);
        let mut wire = req.to_feature_report();
        wire[1] = status::SUCCESS;
        wire[11] = 44; // EC echoes rpm/100 in args[2]
        let resp = Report::from_feature_report(&wire).unwrap();
        assert_eq!(resp.status, status::SUCCESS);
        assert!(resp.answers(&req));
        assert_eq!(resp.args[2], 44);
    }

    #[test]
    fn crc_ignores_trailing_bytes() {
        let mut buf = set_cpu_boost(2).to_feature_report();
        let before = crc(&buf);
        buf[89] = 0xAA; // crc byte itself
        buf[90] = 0xBB; // reserved
        assert_eq!(crc(&buf), before);
    }
}
