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
        return Box::new(MockGpu {
            mode: GpuMode::Hybrid,
            pending: false,
        });
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
        Err(
            "no supported GPU switching tool found (install nvidia-prime's \
             prime-select or envycontrol)"
                .into(),
        )
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
    use std::time::Duration;

    const GPU_TOOL_TIMEOUT: Duration = Duration::from_secs(30);

    enum Tool {
        /// Ubuntu `prime-select`: intel | on-demand | nvidia
        PrimeSelect,
        /// `envycontrol`: integrated | hybrid | nvidia
        EnvyControl,
    }

    pub struct PrimeTool {
        tool: Tool,
        mode: GpuMode,
        pending: bool,
    }

    fn run(cmd: &str, args: &[&str]) -> Result<String, String> {
        let out = crate::process::output_with_timeout(cmd, args, GPU_TOOL_TIMEOUT)?;
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

    fn parse_mode(answer: &str) -> Option<GpuMode> {
        match answer.trim().to_lowercase().as_str() {
            "intel" | "integrated" => Some(GpuMode::Integrated),
            "on-demand" | "hybrid" => Some(GpuMode::Hybrid),
            "nvidia" => Some(GpuMode::Dedicated),
            _ => None,
        }
    }

    impl PrimeTool {
        pub fn detect() -> Option<PrimeTool> {
            if let Ok(answer) = run("prime-select", &["query"]) {
                if let Some(mode) = parse_mode(&answer) {
                    log::info!("gpu switching via prime-select");
                    return Some(PrimeTool {
                        tool: Tool::PrimeSelect,
                        mode,
                        pending: false,
                    });
                }
            }
            if let Ok(answer) = run("envycontrol", &["--query"]) {
                if let Some(mode) = parse_mode(&answer) {
                    log::info!("gpu switching via envycontrol");
                    return Some(PrimeTool {
                        tool: Tool::EnvyControl,
                        mode,
                        pending: false,
                    });
                }
            }
            log::info!("no prime-select/envycontrol; gpu mode switching disabled");
            None
        }
    }

    impl GpuSwitch for PrimeTool {
        fn current(&self) -> Option<GpuMode> {
            Some(self.mode)
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
            self.mode = mode;
            self.pending = true;
            Ok(())
        }

        fn pending(&self) -> bool {
            self.pending
        }
    }

    #[cfg(test)]
    mod tests {
        use super::parse_mode;
        use crate::gpu::GpuMode;

        #[test]
        fn parses_supported_backend_names() {
            assert_eq!(parse_mode("on-demand\n"), Some(GpuMode::Hybrid));
            assert_eq!(parse_mode("integrated"), Some(GpuMode::Integrated));
            assert_eq!(parse_mode("nvidia"), Some(GpuMode::Dedicated));
            assert_eq!(parse_mode("unknown"), None);
        }
    }
}
