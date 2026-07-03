//! Display color profile switching (Native / sRGB / Adobe RGB / Rec.709).
//!
//! Linux backend drives colord via `colormgr`, which Ubuntu desktops ship by
//! default together with the standard-space ICC profiles (sRGB.icc,
//! AdobeRGB1998.icc, Rec709.icc). "Native" selects the panel's auto-generated
//! EDID profile (full gamut, no remapping). The compositor (GNOME/KDE)
//! applies the device's default profile — on bare X11 WMs a helper like
//! xiccd is needed, which we surface as a hint rather than failing silently.

use serde::Serialize;

pub const PROFILES: [&str; 4] = ["native", "srgb", "adobe_rgb", "rec709"];

#[derive(Serialize, Clone, Debug, Default)]
pub struct ColorInfo {
    pub supported: bool,
    /// One of [`PROFILES`], or None when the active profile isn't one of ours.
    pub current: Option<String>,
    /// Human name of the active profile (whatever it is).
    pub current_name: String,
    /// Subset of [`PROFILES`] actually installed on this system.
    pub available: Vec<String>,
    pub hint: String,
}

#[cfg(not(target_os = "linux"))]
mod backend {
    use super::ColorInfo;
    use std::sync::Mutex;

    static CURRENT: Mutex<&str> = Mutex::new("native");

    pub fn get() -> ColorInfo {
        let current = *CURRENT.lock().unwrap();
        ColorInfo {
            supported: true,
            current: Some(current.into()),
            current_name: pretty(current),
            available: super::PROFILES.iter().map(|s| s.to_string()).collect(),
            hint: String::new(),
        }
    }

    pub fn set(profile: &str) -> Result<ColorInfo, String> {
        let known = super::PROFILES
            .iter()
            .find(|p| **p == profile)
            .ok_or_else(|| format!("unknown profile {profile}"))?;
        *CURRENT.lock().unwrap() = known;
        Ok(get())
    }

    fn pretty(id: &str) -> String {
        match id {
            "srgb" => "sRGB".into(),
            "adobe_rgb" => "Adobe RGB (1998)".into(),
            "rec709" => "Rec. 709".into(),
            _ => "Native (EDID)".into(),
        }
    }
}

#[cfg(target_os = "linux")]
mod backend {
    use super::{parse, ColorInfo};
    use std::process::Command;

    fn colormgr(args: &[&str]) -> Result<String, String> {
        let out = Command::new("colormgr")
            .args(args)
            .output()
            .map_err(|e| format!("colormgr: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            Err(format!(
                "colormgr {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }

    fn unsupported(hint: &str) -> ColorInfo {
        ColorInfo { supported: false, hint: hint.into(), ..Default::default() }
    }

    /// (device object path, installed profile map id→objpath)
    fn discover() -> Result<(String, Vec<(String, String)>), String> {
        let devices = colormgr(&["get-devices-by-kind", "display"])?;
        let device =
            parse::display_device(&devices).ok_or("no display device registered with colord")?;
        let profiles = colormgr(&["get-profiles"])?;
        Ok((device, parse::known_profiles(&profiles)))
    }

    pub fn get() -> ColorInfo {
        let (device, profiles) = match discover() {
            Ok(x) => x,
            Err(e) => {
                return unsupported(&format!(
                    "{e}. Color profiles need colord (preinstalled on Ubuntu \
                     GNOME/KDE; on bare X11 also run xiccd)."
                ))
            }
        };
        let available: Vec<String> = profiles.iter().map(|(id, _)| id.clone()).collect();
        let (current, current_name) = match colormgr(&["device-get-default-profile", &device]) {
            Ok(out) => parse::current_profile(&out),
            Err(_) => (None, "unknown".to_string()),
        };
        ColorInfo { supported: true, current, current_name, available, hint: String::new() }
    }

    pub fn set(profile: &str) -> Result<ColorInfo, String> {
        let (device, profiles) = discover()?;
        let (_, objpath) = profiles
            .iter()
            .find(|(id, _)| id == profile)
            .ok_or_else(|| format!("profile {profile} is not installed on this system"))?;
        // Adding an already-assigned profile errors; ignore that and let
        // make-default report real failures.
        let _ = colormgr(&["device-add-profile", &device, objpath]);
        colormgr(&["device-make-profile-default", &device, objpath])?;
        Ok(get())
    }
}

/// `colormgr` output parsers, testable on any platform.
#[allow(dead_code)]
mod parse {
    /// Pick the internal panel from `get-devices-by-kind display`: prefer the
    /// device block mentioning eDP, else the first device block.
    pub fn display_device(out: &str) -> Option<String> {
        let mut first = None;
        let mut current: Option<String> = None;
        let mut current_is_edp = false;
        for line in out.lines() {
            if let Some(path) = line.strip_prefix("Object Path:") {
                if current_is_edp {
                    return current;
                }
                current = Some(path.trim().to_string());
                current_is_edp = false;
                if first.is_none() {
                    first = current.clone();
                }
            } else if line.contains("eDP") {
                current_is_edp = true;
            }
        }
        if current_is_edp {
            return current;
        }
        first
    }

    /// Installed profiles we recognize: (our profile id, object path).
    pub fn known_profiles(out: &str) -> Vec<(String, String)> {
        let mut path: Option<String> = None;
        let mut found: Vec<(String, String)> = Vec::new();
        for line in out.lines() {
            if let Some(p) = line.strip_prefix("Object Path:") {
                path = Some(p.trim().to_string());
            } else if let Some(f) = line.strip_prefix("Filename:") {
                let Some(objpath) = path.clone() else { continue };
                if let Some(id) = classify(f.trim()) {
                    if !found.iter().any(|(i, _)| i == id) {
                        found.push((id.to_string(), objpath));
                    }
                }
            }
        }
        // Stable UI order: native, srgb, adobe_rgb, rec709.
        found.sort_by_key(|(id, _)| super::PROFILES.iter().position(|p| p == id));
        found
    }

    /// From `device-get-default-profile` output: (our id if recognized, display name).
    pub fn current_profile(out: &str) -> (Option<String>, String) {
        let mut filename = "";
        let mut title = "";
        for line in out.lines() {
            if let Some(f) = line.strip_prefix("Filename:") {
                filename = f.trim();
            } else if let Some(t) = line.strip_prefix("Title:") {
                title = t.trim();
            }
        }
        let id = classify(filename).map(str::to_string);
        let name = if !title.is_empty() {
            title.to_string()
        } else {
            filename.rsplit('/').next().unwrap_or("unknown").to_string()
        };
        (id, name)
    }

    fn classify(filename: &str) -> Option<&'static str> {
        let base = filename.rsplit('/').next()?;
        if base.contains("sRGB") {
            Some("srgb")
        } else if base.contains("AdobeRGB") {
            Some("adobe_rgb")
        } else if base.contains("Rec709") {
            Some("rec709")
        } else if base.starts_with("edid") {
            Some("native")
        } else {
            None
        }
    }

    #[cfg(test)]
    mod tests {
        const DEVICES: &str = "\
Object Path:   /org/freedesktop/ColorManager/devices/xrandr_HDMI_1
Type:          display
Device ID:     xrandr-HDMI-1
Metadata:      XRANDR_name=HDMI-1
Object Path:   /org/freedesktop/ColorManager/devices/xrandr_eDP_1
Type:          display
Device ID:     xrandr-eDP-1
Metadata:      XRANDR_name=eDP-1
";

        const PROFILES: &str = "\
Object Path:   /org/freedesktop/ColorManager/profiles/icc_a1
Filename:      /usr/share/color/icc/colord/sRGB.icc
Title:         sRGB
Object Path:   /org/freedesktop/ColorManager/profiles/icc_b2
Filename:      /usr/share/color/icc/colord/AdobeRGB1998.icc
Title:         Adobe RGB (1998)
Object Path:   /org/freedesktop/ColorManager/profiles/icc_c3
Filename:      /usr/share/color/icc/colord/Rec709.icc
Title:         Rec. 709
Object Path:   /org/freedesktop/ColorManager/profiles/icc_d4
Filename:      /home/user/.local/share/icc/edid-9f83a.icc
Title:         eDP-1
";

        #[test]
        fn picks_edp_device() {
            let d = super::display_device(DEVICES).unwrap();
            assert_eq!(d, "/org/freedesktop/ColorManager/devices/xrandr_eDP_1");
        }

        #[test]
        fn maps_profiles_in_ui_order() {
            let p = super::known_profiles(PROFILES);
            let ids: Vec<&str> = p.iter().map(|(id, _)| id.as_str()).collect();
            assert_eq!(ids, vec!["native", "srgb", "adobe_rgb", "rec709"]);
        }

        #[test]
        fn recognizes_current() {
            let out = "Object Path: /o/p\nFilename:      /usr/share/color/icc/colord/AdobeRGB1998.icc\nTitle:         Adobe RGB (1998)\n";
            let (id, name) = super::current_profile(out);
            assert_eq!(id.as_deref(), Some("adobe_rgb"));
            assert_eq!(name, "Adobe RGB (1998)");
        }
    }
}

pub fn get() -> ColorInfo {
    backend::get()
}

pub fn set(profile: &str) -> Result<ColorInfo, String> {
    backend::set(profile)
}
