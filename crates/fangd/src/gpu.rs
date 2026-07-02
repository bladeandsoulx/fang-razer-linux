//! GPU mode switching ("Advanced Optimus" on Windows; PRIME on Linux).
//!
//! The daemon delegates to whichever supported tool the host has:
//! Ubuntu's `prime-select` (ships with the NVIDIA driver) or the
//! cross-distro `envycontrol`. Both persist graphics-stack configuration;
//! changes apply at the next logout/reboot, which we surface via `pending`.

use fang_protocol::api::GpuMode;

pub trait GpuSwitch: Send {
    /// Configured mode; None when switching isn't supported on this host.
    fn current(&self) -> Option<GpuMode>;
    fn set(&mut self, mode: GpuMode) -> Result<(), String>;
    /// True when a switch happened this boot (logout/reboot still needed).
    fn pending(&self) -> bool;
}

pub fn open(mock: bool) -> Box<dyn GpuSwitch> {
    if mock {
        return Box::new(MockGpu { mode: GpuMode::Hybrid, pending: false });
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(tool) = linux::PrimeTool::detect() {
            return Box::new(tool);
        }
    }
    Box::new(Unsupported)
}

struct Unsupported;

impl GpuSwitch for Unsupported {
    fn current(&self) -> Option<GpuMode> {
        None
    }

    fn set(&mut self, _mode: GpuMode) -> Result<(), String> {
        Err("no supported GPU switching tool found (install nvidia-prime's \
             prime-select or envycontrol)"
            .into())
    }

    fn pending(&self) -> bool {
        false
    }
}

struct MockGpu {
    mode: GpuMode,
    pending: bool,
}

impl GpuSwitch for MockGpu {
    fn current(&self) -> Option<GpuMode> {
        Some(self.mode)
    }

    fn set(&mut self, mode: GpuMode) -> Result<(), String> {
        if self.mode != mode {
            self.mode = mode;
            self.pending = true;
        }
        Ok(())
    }

    fn pending(&self) -> bool {
        self.pending
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{GpuMode, GpuSwitch};
    use std::process::Command;

    enum Tool {
        /// Ubuntu `prime-select`: intel | on-demand | nvidia
        PrimeSelect,
        /// `envycontrol`: integrated | hybrid | nvidia
        EnvyControl,
    }

    pub struct PrimeTool {
        tool: Tool,
        pending: bool,
    }

    fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
        let out = Command::new(cmd)
            .args(args)
            .output()
            .map_err(|e| format!("{cmd}: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            Err(format!(
                "{cmd} {}: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
    }

    impl PrimeTool {
        pub fn detect() -> Option<PrimeTool> {
            if run("prime-select", &["query"]).is_ok() {
                log::info!("gpu switching via prime-select");
                return Some(PrimeTool { tool: Tool::PrimeSelect, pending: false });
            }
            if run("envycontrol", &["--query"]).is_ok() {
                log::info!("gpu switching via envycontrol");
                return Some(PrimeTool { tool: Tool::EnvyControl, pending: false });
            }
            log::info!("no prime-select/envycontrol; gpu mode switching disabled");
            None
        }
    }

    impl GpuSwitch for PrimeTool {
        fn current(&self) -> Option<GpuMode> {
            let answer = match self.tool {
                Tool::PrimeSelect => run("prime-select", &["query"]).ok()?,
                Tool::EnvyControl => run("envycontrol", &["--query"]).ok()?,
            };
            match answer.to_lowercase().as_str() {
                "intel" | "integrated" => Some(GpuMode::Integrated),
                "on-demand" | "hybrid" => Some(GpuMode::Hybrid),
                "nvidia" => Some(GpuMode::Dedicated),
                other => {
                    log::warn!("unrecognized gpu mode answer: {other}");
                    None
                }
            }
        }

        fn set(&mut self, mode: GpuMode) -> Result<(), String> {
            match self.tool {
                Tool::PrimeSelect => {
                    let arg = match mode {
                        GpuMode::Integrated => "intel",
                        GpuMode::Hybrid => "on-demand",
                        GpuMode::Dedicated => "nvidia",
                    };
                    run("prime-select", &[arg])?;
                }
                Tool::EnvyControl => {
                    let arg = match mode {
                        GpuMode::Integrated => "integrated",
                        GpuMode::Hybrid => "hybrid",
                        GpuMode::Dedicated => "nvidia",
                    };
                    run("envycontrol", &["-s", arg])?;
                }
            }
            self.pending = true;
            Ok(())
        }

        fn pending(&self) -> bool {
            self.pending
        }
    }
}
