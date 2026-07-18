//! Refresh rate control for the active display.
//!
//! This is a user-session capability (unlike fan/EC control), so it lives in
//! the app, not the daemon. Backends, tried in order:
//!   1. GNOME's `org.gnome.Mutter.DisplayConfig` D-Bus API — works on GNOME
//!      Wayland *and* Xorg, and drives the primary monitor (external screens
//!      included), not just the built-in panel.
//!   2. `kscreen-doctor` (KDE).
//!   3. `xrandr` (bare X11).
//!
//! Non-Linux builds simulate a 240 Hz panel for development.

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
    use std::io::Read;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    const COMMAND_TIMEOUT: Duration = Duration::from_secs(2);

    fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
        run_with_timeout(cmd, args, COMMAND_TIMEOUT)
    }

    /// Drain both pipes while polling the child so a helper cannot block on a
    /// full output buffer. On timeout the child is terminated and reaped.
    fn run_with_timeout(cmd: &str, args: &[&str], timeout: Duration) -> Result<String, String> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("{cmd}: {e}"))?;

        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| format!("{cmd}: no stdout"))?;
        let mut stderr = child
            .stderr
            .take()
            .ok_or_else(|| format!("{cmd}: no stderr"))?;
        let stdout_reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            stdout.read_to_end(&mut bytes).map(|_| bytes)
        });
        let stderr_reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).map(|_| bytes)
        });

        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if started.elapsed() < timeout => {
                    thread::sleep(Duration::from_millis(10));
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{cmd}: timed out after {} ms", timeout.as_millis()));
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("{cmd}: wait: {error}"));
                }
            }
        };

        let stdout = stdout_reader
            .join()
            .map_err(|_| format!("{cmd}: stdout reader panicked"))?
            .map_err(|e| format!("{cmd}: read stdout: {e}"))?;
        let stderr = stderr_reader
            .join()
            .map_err(|_| format!("{cmd}: stderr reader panicked"))?
            .map_err(|e| format!("{cmd}: read stderr: {e}"))?;

        if status.success() {
            Ok(String::from_utf8_lossy(&stdout).into_owned())
        } else {
            Err(format!(
                "{cmd}: {}",
                String::from_utf8_lossy(&stderr).trim()
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
        if let Some(info) = mutter::get() {
            return info;
        }
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
        // GNOME (Wayland or Xorg): the mutter backend handles everything.
        if mutter::available() {
            return mutter::set(hz);
        }

        if let Ok(out) = run("kscreen-doctor", &["-o"]) {
            if let Some(info) = super::parse::kscreen(&out) {
                if !info.available_hz.contains(&hz) {
                    return Err(format!("{hz} Hz not available on {}", info.output));
                }
                run(
                    "kscreen-doctor",
                    &[&format!(
                        "output.{}.mode.{}@{}",
                        info.output, info.resolution, hz
                    )],
                )?;
                return Ok(get());
            }
        }

        if let Ok(out) = run("xrandr", &[]) {
            let Some(info) = super::parse::xrandr(&out) else {
                return Err(unsupported().hint);
            };
            if !info.available_hz.contains(&hz) {
                return Err(format!("{hz} Hz not available on {}", info.output));
            }
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
            return Ok(get());
        }

        Err(unsupported().hint)
    }

    #[cfg(test)]
    mod tests {
        use super::run_with_timeout;
        use std::time::{Duration, Instant};

        #[test]
        fn helper_processes_have_a_hard_timeout() {
            let started = Instant::now();
            let error = run_with_timeout("sleep", &["2"], Duration::from_millis(40))
                .expect_err("sleep should time out");
            assert!(error.contains("timed out"), "{error}");
            assert!(started.elapsed() < Duration::from_secs(1));
        }
    }

    /// GNOME Mutter DisplayConfig over the session bus. Reads the current
    /// monitor layout, then re-applies it with a single monitor's mode
    /// swapped — the API demands the full logical-monitor config, not a
    /// delta, so the whole layout is reconstructed each time.
    mod mutter {
        use super::super::DisplayInfo;
        use serde::{Deserialize, Serialize};
        use std::collections::HashMap;
        use zbus::blocking::Connection;
        use zbus::zvariant::{OwnedValue, Type};

        const SVC: &str = "org.gnome.Mutter.DisplayConfig";
        const PATH: &str = "/org/gnome/Mutter/DisplayConfig";
        const IFACE: &str = "org.gnome.Mutter.DisplayConfig";
        /// ApplyMonitorsConfig method 2 = persistent: apply and save, with no
        /// "keep changes?" dialog (that is the Settings app's temporary path).
        const METHOD_PERSISTENT: u32 = 2;

        // GetCurrentState reply shapes (signatures in comments).
        #[derive(Deserialize, Type)]
        struct MonitorId {
            // (ssss)
            connector: String,
            _vendor: String,
            _product: String,
            _serial: String,
        }

        #[derive(Deserialize, Type)]
        struct Mode {
            // (siiddada{sv})
            id: String,
            width: i32,
            height: i32,
            refresh: f64,
            _preferred_scale: f64,
            _supported_scales: Vec<f64>,
            props: HashMap<String, OwnedValue>,
        }

        #[derive(Deserialize, Type)]
        struct Monitor {
            // ((ssss)a(siiddada{sv})a{sv})
            id: MonitorId,
            modes: Vec<Mode>,
            _props: HashMap<String, OwnedValue>,
        }

        #[derive(Deserialize, Type)]
        struct LogicalMonitor {
            // (iiduba(ssss)a{sv})
            x: i32,
            y: i32,
            scale: f64,
            transform: u32,
            primary: bool,
            monitors: Vec<MonitorId>,
            _props: HashMap<String, OwnedValue>,
        }

        type State = (
            u32,
            Vec<Monitor>,
            Vec<LogicalMonitor>,
            HashMap<String, OwnedValue>,
        );

        // ApplyMonitorsConfig request shapes.
        #[derive(Serialize, Type)]
        struct MonitorConfig {
            // (ssa{sv})
            connector: String,
            mode_id: String,
            props: HashMap<String, OwnedValue>,
        }

        #[derive(Serialize, Type)]
        struct LogicalMonitorConfig {
            // (iiduba(ssa{sv}))
            x: i32,
            y: i32,
            scale: f64,
            transform: u32,
            primary: bool,
            monitors: Vec<MonitorConfig>,
        }

        fn state(conn: &Connection) -> Result<State, String> {
            let reply = conn
                .call_method(Some(SVC), PATH, Some(IFACE), "GetCurrentState", &())
                .map_err(|e| e.to_string())?;
            reply.body().deserialize().map_err(|e| e.to_string())
        }

        fn prop_bool(props: &HashMap<String, OwnedValue>, key: &str) -> bool {
            props
                .get(key)
                .and_then(|v| bool::try_from(v).ok())
                .unwrap_or(false)
        }

        /// The connector we drive: the primary logical monitor's first
        /// output, else the first monitor — i.e. the screen the user treats
        /// as main, external displays included.
        fn target(logical: &[LogicalMonitor]) -> Option<String> {
            logical
                .iter()
                .find(|l| l.primary)
                .or_else(|| logical.first())
                .and_then(|l| l.monitors.first())
                .map(|m| m.connector.clone())
        }

        fn info(monitors: &[Monitor], logical: &[LogicalMonitor]) -> Option<DisplayInfo> {
            let conn = target(logical)?;
            let mon = monitors.iter().find(|m| m.id.connector == conn)?;
            let cur = mon
                .modes
                .iter()
                .find(|md| prop_bool(&md.props, "is-current"))?;
            let mut available: Vec<u32> = mon
                .modes
                .iter()
                .filter(|md| md.width == cur.width && md.height == cur.height)
                .map(|md| md.refresh.round() as u32)
                .collect();
            available.sort_unstable();
            available.dedup();
            Some(DisplayInfo {
                supported: true,
                output: conn,
                resolution: format!("{}x{}", cur.width, cur.height),
                current_hz: cur.refresh.round() as u32,
                available_hz: available,
                hint: String::new(),
            })
        }

        pub fn get() -> Option<DisplayInfo> {
            let conn = Connection::session().ok()?;
            let (_serial, monitors, logical, _props) = state(&conn).ok()?;
            info(&monitors, &logical)
        }

        /// True only on GNOME (the service answers). Elsewhere we fall through
        /// to kscreen-doctor / xrandr.
        pub fn available() -> bool {
            Connection::session()
                .ok()
                .is_some_and(|c| state(&c).is_ok())
        }

        pub fn set(hz: u32) -> Result<DisplayInfo, String> {
            let conn = Connection::session().map_err(|e| e.to_string())?;
            let (serial, monitors, logical, _props) = state(&conn)?;

            let tgt = target(&logical).ok_or("no active monitor")?;
            let mon = monitors
                .iter()
                .find(|m| m.id.connector == tgt)
                .ok_or("target monitor vanished")?;
            let cur = mon
                .modes
                .iter()
                .find(|md| prop_bool(&md.props, "is-current"))
                .ok_or("no current mode")?;
            // Highest exact refresh whose rounded value matches the request,
            // at the current resolution (so we never change resolution).
            let want = mon
                .modes
                .iter()
                .filter(|md| {
                    md.width == cur.width
                        && md.height == cur.height
                        && md.refresh.round() as u32 == hz
                })
                .max_by(|a, b| a.refresh.total_cmp(&b.refresh))
                .ok_or_else(|| format!("{hz} Hz not available on {tgt}"))?;
            let new_mode = want.id.clone();

            // Each monitor's current mode id, so the untouched displays keep
            // their mode when we hand mutter the full layout.
            let current_mode: HashMap<String, String> = monitors
                .iter()
                .filter_map(|m| {
                    m.modes
                        .iter()
                        .find(|md| prop_bool(&md.props, "is-current"))
                        .map(|md| (m.id.connector.clone(), md.id.clone()))
                })
                .collect();

            let configs: Vec<LogicalMonitorConfig> = logical
                .iter()
                .map(|l| LogicalMonitorConfig {
                    x: l.x,
                    y: l.y,
                    scale: l.scale,
                    transform: l.transform,
                    primary: l.primary,
                    monitors: l
                        .monitors
                        .iter()
                        .filter_map(|mi| {
                            let mode_id = if mi.connector == tgt {
                                new_mode.clone()
                            } else {
                                current_mode.get(&mi.connector).cloned()?
                            };
                            Some(MonitorConfig {
                                connector: mi.connector.clone(),
                                mode_id,
                                props: HashMap::new(),
                            })
                        })
                        .collect(),
                })
                .collect();

            let props: HashMap<String, OwnedValue> = HashMap::new();
            conn.call_method(
                Some(SVC),
                PATH,
                Some(IFACE),
                "ApplyMonitorsConfig",
                &(serial, METHOD_PERSISTENT, configs, props),
            )
            .map_err(|e| format!("apply refresh rate: {e}"))?;

            get().ok_or_else(|| "refresh rate applied, but re-reading state failed".to_string())
        }
    }
}

/// Output parsers, separated for unit testing on any platform.
#[allow(dead_code)]
mod parse {
    use super::DisplayInfo;

    /// `kscreen-doctor -o` — an active output line looks like:
    /// `Output: 1 eDP-1 enabled connected priority 1 Panel Modes: 0:2560x1600@240*! 1:2560x1600@60 ...`
    pub fn kscreen(out: &str) -> Option<DisplayInfo> {
        let clean = strip_ansi(out);
        clean
            .lines()
            .filter_map(kscreen_line)
            // KScreen priority 1 is the primary output. If a compositor does
            // not assign 1, the lowest positive priority is the best active
            // fallback; missing/zero priority sorts last.
            .min_by_key(|(priority, _)| match priority {
                0 => u32::MAX,
                priority => *priority,
            })
            .map(|(_, info)| info)
    }

    fn kscreen_line(line: &str) -> Option<(u32, DisplayInfo)> {
        if !(line.contains("Output:")
            && line.split_whitespace().any(|part| part == "enabled")
            && line.split_whitespace().any(|part| part == "connected"))
        {
            return None;
        }

        let output = line.split_whitespace().nth(2)?.to_string();
        let fields: Vec<&str> = line.split_whitespace().collect();
        let priority = fields
            .windows(2)
            .find(|pair| pair[0] == "priority")
            .and_then(|pair| pair[1].parse::<u32>().ok())
            .unwrap_or(0);
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
        Some((
            priority,
            DisplayInfo {
                supported: true,
                output,
                resolution: current.0,
                current_hz: current.1,
                available_hz: available,
                hint: String::new(),
            },
        ))
    }

    /// `xrandr` — active output block:
    /// ```text
    /// eDP-1 connected primary 2560x1600+0+0 ...
    ///    2560x1600    240.00*+  60.00
    /// ```
    pub fn xrandr(out: &str) -> Option<DisplayInfo> {
        let lines: Vec<&str> = out.lines().collect();
        let mut candidates = Vec::new();
        let mut index = 0;

        while index < lines.len() {
            let line = lines[index];
            index += 1;
            if line.starts_with(char::is_whitespace) || !line.contains(" connected") {
                continue;
            }
            let Some(output) = line.split_whitespace().next() else {
                continue;
            };
            let primary = line.split_whitespace().any(|field| field == "primary");
            let mut resolution = String::new();
            let mut current = 0u32;
            let mut available = Vec::new();

            while index < lines.len() {
                let mode = lines[index];
                if !mode.starts_with(' ') {
                    break;
                }
                index += 1;
                let mut parts = mode.split_whitespace();
                let Some(res) = parts.next() else {
                    continue;
                };
                let rates: Vec<&str> = parts.collect();
                let is_current_res = rates.iter().any(|r| r.contains('*'));
                if is_current_res {
                    resolution = res.to_string();
                    for r in &rates {
                        let Ok(hz) = r.trim_end_matches(['*', '+']).parse::<f32>() else {
                            continue;
                        };
                        let hz = hz.round() as u32;
                        if r.contains('*') {
                            current = hz;
                        }
                        available.push(hz);
                    }
                }
            }

            // Connected but disabled outputs have no current (`*`) mode and
            // must not outrank an actually active screen.
            if current != 0 {
                available.sort_unstable();
                available.dedup();
                candidates.push((
                    primary,
                    DisplayInfo {
                        supported: true,
                        output: output.to_string(),
                        resolution,
                        current_hz: current,
                        available_hz: available,
                        hint: String::new(),
                    },
                ));
            }
        }

        candidates
            .iter()
            .find(|(primary, _)| *primary)
            .or_else(|| candidates.first())
            .map(|(_, info)| info.clone())
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

        #[test]
        fn kscreen_selects_primary_active_external_output() {
            let out = "\
Output: 1 eDP-1 enabled connected priority 2 Panel Modes: 0:2560x1600@240*! 1:2560x1600@60 Geometry: 0,0 2560x1600
Output: 2 DP-2 enabled connected priority 1 DisplayPort Modes: 0:3840x2160@144*! 1:3840x2160@60 Geometry: 2560,0 3840x2160
Output: 3 HDMI-1 disabled connected priority 0 HDMI Modes: 0:1920x1080@60*! Geometry: 0,0 1920x1080
";
            let info = super::super::parse::kscreen(out).unwrap();
            assert_eq!(info.output, "DP-2");
            assert_eq!(info.resolution, "3840x2160");
            assert_eq!(info.current_hz, 144);
            assert_eq!(info.available_hz, vec![60, 144]);
        }

        #[test]
        fn xrandr_selects_primary_active_external_output() {
            let out = "\
Screen 0: minimum 320 x 200, current 6400 x 2160, maximum 16384 x 16384
eDP-1 connected 2560x1600+0+0 (normal left inverted) 345mm x 215mm
   2560x1600    240.00*+  60.00
DP-2 connected primary 3840x2160+2560+0 (normal left inverted) 600mm x 340mm
   3840x2160    143.98*+  120.00  60.00
HDMI-1 connected (normal left inverted)
   1920x1080     60.00 +
";
            let info = super::super::parse::xrandr(out).unwrap();
            assert_eq!(info.output, "DP-2");
            assert_eq!(info.resolution, "3840x2160");
            assert_eq!(info.current_hz, 144);
            assert_eq!(info.available_hz, vec![60, 120, 144]);
        }

        #[test]
        fn xrandr_falls_back_to_first_active_output_without_primary() {
            let out = "\
DP-1 connected (normal left inverted)
   1920x1080     60.00 +
HDMI-1 connected 1920x1080+0+0 (normal left inverted)
   1920x1080     75.00*+  60.00
";
            let info = super::super::parse::xrandr(out).unwrap();
            assert_eq!(info.output, "HDMI-1");
            assert_eq!(info.current_hz, 75);
        }
    }
}

pub fn get() -> DisplayInfo {
    backend::get()
}

pub fn set(hz: u32) -> Result<DisplayInfo, String> {
    backend::set(hz)
}
