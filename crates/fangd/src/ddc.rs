//! External-monitor color control over DDC/CI, via the `ddcutil` CLI.
//!
//! Synapse's "color profile" clamps the internal wide-gamut panel; that isn't
//! reachable on Linux (no compositor API for a gamut LUT). What *is* reachable
//! is the external monitor's own hardware color controls over DDC/CI — here
//! the color-temperature presets (VCP feature 0x14). Laptop eDP panels don't
//! speak DDC/CI, so this only ever drives an external display.
//!
//! DDC needs /dev/i2c access, so this lives in the root daemon rather than the
//! unprivileged app (which would otherwise need standing i2c permissions).

use fang_protocol::api::ColorPreset;
use std::process::Command;

/// Curated VCP 0x14 presets in UI order (value, label). Only those a monitor
/// actually advertises are offered. Standard MCCS color-temperature values.
const PRESETS: &[(u8, &str)] = &[
    (0x03, "Warm (5000K)"),
    (0x04, "sRGB · D65 (6500K)"),
    (0x05, "Neutral (7500K)"),
    (0x07, "Cool (9300K)"),
    (0x0B, "Custom (User)"),
];

pub struct Ddc {
    /// ddcutil display number of the external monitor, if one was found.
    display: Option<u8>,
    presets: Vec<ColorPreset>,
    /// Last known VCP 0x14 value (cached; DDC reads are ~0.5 s each).
    current: Option<u8>,
    /// Last known VCP 0x10 (luminance) as a percentage, if the monitor
    /// reports one. None when the monitor doesn't expose brightness.
    brightness: Option<u8>,
    /// Monitor's own max for VCP 0x10; a percentage is scaled to this on set
    /// (most panels report 100, some 255 or other).
    bright_max: u32,
    mock: bool,
}

pub fn open(mock: bool) -> Ddc {
    if mock {
        Ddc::mock()
    } else {
        Ddc::discover()
    }
}

impl Ddc {
    fn discover() -> Ddc {
        // Best-effort: load i2c-dev so DDC works even right after boot,
        // without a persistent modules-load config.
        let _ = Command::new("modprobe").arg("i2c-dev").status();

        let Some(display) = detect_external() else {
            log::info!("DDC color: no external DDC/CI monitor detected");
            return Ddc::none();
        };
        let supported = supported_presets(display);
        let presets: Vec<ColorPreset> = PRESETS
            .iter()
            // If capabilities couldn't be read, offer the full curated set.
            .filter(|(v, _)| supported.is_empty() || supported.contains(v))
            .map(|(v, n)| ColorPreset {
                value: *v,
                name: (*n).to_string(),
            })
            .collect();
        let current = read_current(display);
        let (brightness, bright_max) = read_brightness(display);
        log::info!(
            "DDC color: external display #{display}, {} presets, current {current:?}, brightness {brightness:?}",
            presets.len()
        );
        Ddc {
            display: Some(display),
            presets,
            current,
            brightness,
            bright_max,
            mock: false,
        }
    }

    fn none() -> Ddc {
        Ddc {
            display: None,
            presets: Vec::new(),
            current: None,
            brightness: None,
            bright_max: 100,
            mock: false,
        }
    }

    fn mock() -> Ddc {
        Ddc {
            display: None,
            presets: PRESETS
                .iter()
                .map(|(v, n)| ColorPreset {
                    value: *v,
                    name: (*n).to_string(),
                })
                .collect(),
            current: Some(0x04),
            brightness: Some(70),
            bright_max: 100,
            mock: true,
        }
    }

    pub fn available(&self) -> bool {
        self.mock || self.display.is_some()
    }

    pub fn presets(&self) -> Vec<ColorPreset> {
        self.presets.clone()
    }

    pub fn current(&self) -> Option<u8> {
        self.current
    }

    pub fn brightness(&self) -> Option<u8> {
        self.brightness
    }

    /// Set the external monitor's luminance (VCP 0x10) from a 0..=100 percent,
    /// scaling to the monitor's own max.
    pub fn set_brightness(&mut self, percent: u8) -> Result<(), String> {
        let percent = percent.min(100);
        if self.mock {
            self.brightness = Some(percent);
            return Ok(());
        }
        if self.brightness.is_none() {
            return Err("this monitor doesn't expose brightness over DDC/CI".into());
        }
        let display = self.display.ok_or("no external DDC/CI monitor")?;
        let raw = (percent as u32 * self.bright_max / 100).min(self.bright_max);
        ddcutil(&["-d", &display.to_string(), "setvcp", "10", &raw.to_string()])
            .ok_or("ddcutil setvcp 10 failed (monitor busy or DDC/CI disabled in its OSD?)")?;
        self.brightness = Some(percent);
        Ok(())
    }

    pub fn set(&mut self, value: u8) -> Result<(), String> {
        if !self.presets.iter().any(|p| p.value == value) {
            return Err("unsupported color preset for this monitor".into());
        }
        if self.mock {
            self.current = Some(value);
            return Ok(());
        }
        let display = self.display.ok_or("no external DDC/CI monitor")?;
        ddcutil(&[
            "-d",
            &display.to_string(),
            "setvcp",
            "14",
            &format!("0x{value:02x}"),
        ])
        .ok_or("ddcutil setvcp failed (monitor busy or DDC/CI disabled in its OSD?)")?;
        self.current = Some(value);
        Ok(())
    }
}

fn ddcutil(args: &[&str]) -> Option<String> {
    let out = Command::new("ddcutil").args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// First valid `Display N` in `ddcutil detect` — laptop panels list as
/// "Invalid display", so the first real one is the external monitor.
fn detect_external() -> Option<u8> {
    let out = ddcutil(&["detect"])?;
    for line in out.lines() {
        if let Some(rest) = line.trim_start().strip_prefix("Display ") {
            if let Ok(n) = rest.trim().parse::<u8>() {
                return Some(n);
            }
        }
    }
    None
}

/// VCP 0x14 values the monitor advertises, from its capability string
/// (`... vcp(... 14(03 04 05 ...) ...)`).
fn supported_presets(display: u8) -> Vec<u8> {
    let caps = ddcutil(&["-d", &display.to_string(), "capabilities"]).unwrap_or_default();
    if let Some(start) = caps.find("14(") {
        if let Some(len) = caps[start..].find(')') {
            return caps[start + 3..start + len]
                .split_whitespace()
                .filter_map(|t| u8::from_str_radix(t, 16).ok())
                .collect();
        }
    }
    Vec::new()
}

/// Current VCP 0x14 value: last token of `VCP 14 CNC mh ml sh sl`.
fn read_current(display: u8) -> Option<u8> {
    let out = ddcutil(&["-d", &display.to_string(), "getvcp", "14", "--brief"])?;
    out.split_whitespace()
        .next_back()
        .and_then(|t| u8::from_str_radix(t.trim_start_matches('x'), 16).ok())
}

/// VCP 0x10 (luminance) is continuous: `--brief` prints `VCP 10 C <cur> <max>`
/// in decimal. Returns (current as 0..=100 percent, raw max) so the UI shows a
/// percentage and [`Ddc::set_brightness`] can scale back. `(None, 100)` when
/// the monitor doesn't answer the feature.
fn read_brightness(display: u8) -> (Option<u8>, u32) {
    let Some(out) = ddcutil(&["-d", &display.to_string(), "getvcp", "10", "--brief"]) else {
        return (None, 100);
    };
    // Tokens: "VCP" "10" "C" <cur> <max>; the decimals are [10, cur, max].
    let nums: Vec<u32> = out.split_whitespace().filter_map(|t| t.parse().ok()).collect();
    if let [.., cur, max] = nums[..] {
        if max > 0 {
            return (Some((cur * 100 / max).min(100) as u8), max);
        }
    }
    (None, 100)
}
