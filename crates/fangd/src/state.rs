//! Desired hardware state, persisted so it survives restarts and resume.

use fang_protocol::api::{Boost, FanMode, KbdEffect, LogoMode, PerfMode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AppliedState {
    pub perf_mode: PerfMode,
    pub cpu_boost: Boost,
    pub gpu_boost: Boost,
    pub fan: FanMode,
    // serde defaults keep state files from before each feature loading.
    #[serde(default)]
    pub bho_enabled: bool,
    #[serde(default = "default_bho_threshold")]
    pub bho_threshold: u8,
    /// Keyboard backlight brightness percent (0..=100).
    #[serde(default = "default_brightness")]
    pub kbd_brightness: u8,
    #[serde(default = "default_kbd_effect")]
    pub kbd_effect: KbdEffect,
    #[serde(default = "default_logo")]
    pub logo_led: LogoMode,
    /// Auto-switch perf profile on AC ↔ battery transitions.
    #[serde(default)]
    pub auto_power: bool,
    #[serde(default = "default_ac_profile")]
    pub ac_profile: PerfMode,
    #[serde(default = "default_battery_profile")]
    pub battery_profile: PerfMode,
    /// Fan applied for each source when automation switches (Auto = mode curve).
    #[serde(default = "default_fan")]
    pub ac_fan: FanMode,
    #[serde(default = "default_fan")]
    pub battery_fan: FanMode,
}

fn default_ac_profile() -> PerfMode {
    PerfMode::Balanced
}

fn default_battery_profile() -> PerfMode {
    PerfMode::Silent
}

fn default_fan() -> FanMode {
    FanMode::Auto
}

fn default_bho_threshold() -> u8 {
    80
}

fn default_brightness() -> u8 {
    60
}

/// Synapse's out-of-box look: static Razer green.
fn default_kbd_effect() -> KbdEffect {
    KbdEffect::Static {
        r: 0x44,
        g: 0xD6,
        b: 0x2C,
    }
}

fn default_logo() -> LogoMode {
    LogoMode::Static
}

impl Default for AppliedState {
    fn default() -> Self {
        AppliedState {
            perf_mode: PerfMode::Balanced,
            cpu_boost: Boost::Medium,
            gpu_boost: Boost::Medium,
            fan: FanMode::Auto,
            bho_enabled: false,
            bho_threshold: default_bho_threshold(),
            kbd_brightness: default_brightness(),
            kbd_effect: default_kbd_effect(),
            logo_led: default_logo(),
            auto_power: false,
            ac_profile: default_ac_profile(),
            battery_profile: default_battery_profile(),
            ac_fan: default_fan(),
            battery_fan: default_fan(),
        }
    }
}

impl AppliedState {
    pub fn load(path: &Path) -> AppliedState {
        match std::fs::read_to_string(path) {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(state) => state,
                Err(e) => {
                    log::warn!(
                        "state file {} unreadable ({e}), using defaults",
                        path.display()
                    );
                    AppliedState::default()
                }
            },
            Err(_) => AppliedState::default(),
        }
    }

    /// Atomic write (tmp + rename) so a crash can't truncate the state file.
    pub fn save(&self, path: &Path) {
        let write = || -> std::io::Result<()> {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let tmp = path.with_extension("json.tmp");
            std::fs::write(&tmp, serde_json::to_vec_pretty(self).expect("serializable"))?;
            std::fs::rename(&tmp, path)?;
            Ok(())
        };
        if let Err(e) = write() {
            log::error!("failed to persist state to {}: {e}", path.display());
        }
    }
}

pub fn default_state_path() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/var/lib/fangd/state.json")
    }
    #[cfg(not(target_os = "linux"))]
    {
        std::env::var_os("LOCALAPPDATA")
            .map(|d| PathBuf::from(d).join("fangd").join("state.json"))
            .unwrap_or_else(|| PathBuf::from("fangd-state.json"))
    }
}
