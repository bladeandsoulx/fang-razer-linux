//! Desired hardware state, persisted so it survives restarts and resume.

use fang_protocol::api::{Boost, FanMode, PerfMode};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AppliedState {
    pub perf_mode: PerfMode,
    pub cpu_boost: Boost,
    pub gpu_boost: Boost,
    pub fan: FanMode,
}

impl Default for AppliedState {
    fn default() -> Self {
        AppliedState {
            perf_mode: PerfMode::Balanced,
            cpu_boost: Boost::Medium,
            gpu_boost: Boost::Medium,
            fan: FanMode::Auto,
        }
    }
}

impl AppliedState {
    pub fn load(path: &Path) -> AppliedState {
        match std::fs::read_to_string(path) {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(state) => state,
                Err(e) => {
                    log::warn!("state file {} unreadable ({e}), using defaults", path.display());
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
