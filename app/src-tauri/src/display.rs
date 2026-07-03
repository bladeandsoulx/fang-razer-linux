//! Internal-panel refresh rate control.
//!
//! This is a user-session capability (unlike fan/EC control), so it lives in
//! the app, not the daemon. Backends: `kscreen-doctor` (KDE) and `xrandr`
//! (X11). GNOME Wayland's mutter API needs a full monitors-config
//! round-trip and is out of scope for v1 — reported as unsupported with a
//! hint. Non-Linux builds simulate a 240 Hz panel for development.

use serde::Serialize;

#[derive(Serialize, Clone, Debug, Default)]
pub struct DisplayInfo {
    pub supported: bool,
    pub output: String,
    pub resolution: String,
    pub current_hz: u32,
    pub available_hz: Vec<u32>,
    /// Filled when unsupported, explains what to do instead.
    pub hint: String,
}

#[cfg(not(target_os = "linux"))]
mod backend {
    use super::DisplayInfo;
    use std::sync::Mutex;

    static CURRENT: Mutex<u32> = Mutex::new(240);

    pub fn get() -> DisplayInfo {
        DisplayInfo {
            supported: true,
            output: "eDP-1 (simulated)".into(),
            resolution: "2560x1600".into(),
            current_hz: *CURRENT.lock().unwrap(),
            available_hz: vec![60, 120, 240],
            hint: String::new(),
        }
    }

    pub fn set(hz: u32) -> Result<DisplayInfo, String> {
        if ![60, 120, 240].contains(&hz) {
            return Err(format!("{hz} Hz not available"));
        }
        *CURRENT.lock().unwrap() = hz;
        Ok(get())
    }
}

#[cfg(target_os = "linux")]
mod backend {
    use super::DisplayInfo;
    use std::process::Command;

    fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
        let out = Command::new(cmd)
            .args(args)
            .output()
            .map_err(|e| format!("{cmd}: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            Err(format!(
                "{cmd}: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }

    fn unsupported() -> DisplayInfo {
        DisplayInfo {
            supported: false,
            hint: "No supported tool found. On GNOME Wayland use Settings → \
                   Displays; otherwise install kscreen-doctor (KDE) or run an \
                   X11 session (xrandr)."
                .into(),
            ..Default::default()
        }
    }

    pub fn get() -> DisplayInfo {
        if let Ok(out) = run("kscreen-doctor", &["-o"]) {
            if let Some(info) = super::parse::kscreen(&out) {
                return info;
            }
        }
        if let Ok(out) = run("xrandr", &[]) {
            if let Some(info) = super::parse::xrandr(&out) {
                return info;
            }
        }
        unsupported()
    }

    pub fn set(hz: u32) -> Result<DisplayInfo, String> {
        let info = get();
        if !info.supported {
            return Err(info.hint);
        }
        if !info.available_hz.contains(&hz) {
            return Err(format!("{hz} Hz not available on {}", info.output));
        }
        if run("kscreen-doctor", &["-o"]).is_ok() {
            run(
                "kscreen-doctor",
                &[&format!(
                    "output.{}.mode.{}@{}",
                    info.output, info.resolution, hz
                )],
            )?;
        } else {
            run(
                "xrandr",
                &[
                    "--output",
                    &info.output,
                    "--mode",
                    &info.resolution,
                    "--rate",
                    &hz.to_string(),
                ],
            )?;
        }
        Ok(get())
    }
}

/// Output parsers, separated for unit testing on any platform.
#[allow(dead_code)]
mod parse {
    use super::DisplayInfo;

    /// `kscreen-doctor -o` — internal panel line looks like:
    /// `Output: 1 eDP-1 enabled connected priority 1 Panel Modes: 0:2560x1600@240*! 1:2560x1600@60 ...`
    pub fn kscreen(out: &str) -> Option<DisplayInfo> {
        let clean = strip_ansi(out);
        let line = clean
            .lines()
            .find(|l| l.contains("Output:") && l.contains("eDP"))?;
        let output = line.split_whitespace().nth(2)?.to_string();
        let modes = line.split("Modes:").nth(1)?;
        let mut current = (String::new(), 0u32);
        let mut rates: Vec<(String, u32)> = Vec::new();
        for token in modes.split_whitespace() {
            // token: "0:2560x1600@240*!" (current marked with *).
            // Non-mode tokens (e.g. the trailing "Geometry: ...") are skipped.
            let spec = token.split(':').next_back().unwrap_or(token);
            let Some((res, rest)) = spec.split_once('@') else {
                continue;
            };
            let Ok(hz) = rest.trim_end_matches(['*', '!']).parse::<f32>() else {
                continue;
            };
            let hz = hz.round() as u32;
            if rest.contains('*') {
                current = (res.to_string(), hz);
            }
            rates.push((res.to_string(), hz));
        }
        if current.1 == 0 {
            return None;
        }
        let mut available: Vec<u32> = rates
            .into_iter()
            .filter(|(res, _)| *res == current.0)
            .map(|(_, hz)| hz)
            .collect();
        available.sort_unstable();
        available.dedup();
        Some(DisplayInfo {
            supported: true,
            output,
            resolution: current.0,
            current_hz: current.1,
            available_hz: available,
            hint: String::new(),
        })
    }

    /// `xrandr` — internal panel block:
    /// ```text
    /// eDP-1 connected primary 2560x1600+0+0 ...
    ///    2560x1600    240.00*+  60.00
    /// ```
    pub fn xrandr(out: &str) -> Option<DisplayInfo> {
        let mut lines = out.lines().peekable();
        while let Some(line) = lines.next() {
            if !(line.starts_with("eDP") && line.contains(" connected")) {
                continue;
            }
            let output = line.split_whitespace().next()?.to_string();
            let mut resolution = String::new();
            let mut current = 0u32;
            let mut available = Vec::new();
            for mode in lines.by_ref() {
                if !mode.starts_with(' ') {
                    break;
                }
                let mut parts = mode.split_whitespace();
                let res = parts.next()?.to_string();
                let rates: Vec<&str> = parts.collect();
                let is_current_res = rates.iter().any(|r| r.contains('*'));
                if is_current_res {
                    resolution = res;
                    for r in &rates {
                        let hz = r.trim_end_matches(['*', '+']).parse::<f32>().ok()?;
                        let hz = hz.round() as u32;
                        if r.contains('*') {
                            current = hz;
                        }
                        available.push(hz);
                    }
                }
            }
            if current == 0 {
                return None;
            }
            available.sort_unstable();
            available.dedup();
            return Some(DisplayInfo {
                supported: true,
                output,
                resolution,
                current_hz: current,
                available_hz: available,
                hint: String::new(),
            });
        }
        None
    }

    fn strip_ansi(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c2 in chars.by_ref() {
                    if c2.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[cfg(test)]
    mod tests {
        #[test]
        fn parses_xrandr() {
            let out = "\
Screen 0: minimum 320 x 200, current 2560 x 1600, maximum 16384 x 16384
eDP-1 connected primary 2560x1600+0+0 (normal left inverted) 345mm x 215mm
   2560x1600    240.00*+  60.01    59.99
   1920x1200    240.00    60.00
HDMI-1 disconnected (normal left inverted)
";
            let info = super::super::parse::xrandr(out).unwrap();
            assert_eq!(info.output, "eDP-1");
            assert_eq!(info.resolution, "2560x1600");
            assert_eq!(info.current_hz, 240);
            assert_eq!(info.available_hz, vec![60, 240]);
        }

        #[test]
        fn parses_kscreen() {
            let out = "Output: 1 eDP-1 enabled connected priority 1 Panel Modes: 0:2560x1600@240*! 1:2560x1600@60 2:1920x1200@240 Geometry: 0,0 2560x1600\n";
            let info = super::super::parse::kscreen(out).unwrap();
            assert_eq!(info.output, "eDP-1");
            assert_eq!(info.current_hz, 240);
            assert_eq!(info.available_hz, vec![60, 240]);
        }
    }
}

pub fn get() -> DisplayInfo {
    backend::get()
}

pub fn set(hz: u32) -> Result<DisplayInfo, String> {
    backend::set(hz)
}
