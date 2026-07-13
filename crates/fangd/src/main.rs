mod core;
mod ddc;
mod gpu;
mod hw;
mod peripherals;
mod power;
mod process;
mod server;
mod state;

use crate::core::Core;
use crate::state::AppliedState;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

const DEFAULT_TCP: &str = "127.0.0.1:7331";
#[cfg(unix)]
const DEFAULT_SOCKET: &str = "/run/fangd.sock";

struct Args {
    mock: bool,
    restore_auto: bool,
    tcp: Option<String>,
    #[cfg_attr(not(unix), allow(dead_code))]
    socket: Option<PathBuf>,
    state: Option<PathBuf>,
}

fn parse_args() -> Args {
    let mut args = Args {
        mock: std::env::var("FANGD_MOCK")
            .map(|v| v == "1")
            .unwrap_or(false),
        restore_auto: false,
        tcp: None,
        socket: None,
        state: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--mock" => args.mock = true,
            "--restore-auto" => args.restore_auto = true,
            "--tcp" => args.tcp = it.next(),
            "--socket" => args.socket = it.next().map(PathBuf::from),
            "--state" => args.state = it.next().map(PathBuf::from),
            "--help" | "-h" => {
                println!(
                    "fangd {} — Fang daemon for Razer Blade laptops\n\n\
                     USAGE: fangd [--mock] [--tcp ADDR] [--socket PATH] [--state PATH]\n\n\
                     --mock          simulate hardware (also FANGD_MOCK=1)\n\
                     --restore-auto  restore EC automatic fan control and exit\n\
                     --tcp ADDR      mock-only loopback TCP instead of unix socket (dev)\n\
                     --socket PATH   unix socket path (default /run/fangd.sock)\n\
                     --state PATH    state file (default /var/lib/fangd/state.json)",
                    env!("CARGO_PKG_VERSION")
                );
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown argument: {other} (see --help)");
                std::process::exit(2);
            }
        }
    }
    args
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = parse_args();
    if let Some(addr) = args.tcp.as_deref() {
        if let Err(e) = validate_tcp(addr, args.mock) {
            eprintln!("refusing --tcp: {e}");
            std::process::exit(2);
        }
    }

    let state_path = args.state.clone().unwrap_or_else(state::default_state_path);
    let state = AppliedState::load(&state_path);
    let mut hw = hw::open(args.mock);
    if args.restore_auto {
        match hw.restore_auto_fan(state.perf_mode) {
            Ok(()) => log::info!("restored EC automatic fan control"),
            Err(e) => log::warn!("could not restore EC automatic fan control: {e}"),
        }
        // ExecStopPost must not turn an otherwise clean service stop into a
        // failed unit when hardware is absent or temporarily unavailable.
        return;
    }
    let mut core = Core::new(hw, state, state_path);
    core.reapply();
    let peripheral_snapshot = peripherals::snapshot_store();
    log::info!(
        "fangd {} — {}",
        env!("CARGO_PKG_VERSION"),
        core.status(&peripherals::read_snapshot(&peripheral_snapshot))
            .model
    );

    let core: server::SharedCore = Arc::new(Mutex::new(core));
    let bus = server::event_bus();
    let telemetry_task = tokio::spawn(server::telemetry_loop(
        Arc::clone(&core),
        Arc::clone(&peripheral_snapshot),
        bus.clone(),
    ));
    // DDC and GPU discovery can take seconds. Thermal sampling is already
    // active and remains independent of these subprocess-backed features.
    let peripherals = peripherals::Peripherals::open(args.mock, peripheral_snapshot).await;
    let ddc_rescan_task = tokio::spawn(server::ddc_rescan_loop(
        Arc::clone(&core),
        peripherals.clone(),
        bus.clone(),
    ));

    #[cfg(unix)]
    if args.tcp.is_none() {
        let path = args
            .socket
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET));
        let _ = std::fs::remove_file(&path);
        let listener = tokio::net::UnixListener::bind(&path)
            .unwrap_or_else(|e| panic!("bind {}: {e}", path.display()));
        grant_socket_access(&path);
        log::info!("listening on {}", path.display());
        let shutdown = shutdown_signal();
        tokio::pin!(shutdown);
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { continue };
                    tokio::spawn(server::handle_conn(
                        stream,
                        Arc::clone(&core),
                        peripherals.clone(),
                        bus.clone(),
                    ));
                }
                _ = &mut shutdown => break,
            }
        }
        telemetry_task.abort();
        ddc_rescan_task.abort();
        restore_auto_before_exit(&core).await;
        let _ = std::fs::remove_file(&path);
        return;
    }

    let addr = args.tcp.unwrap_or_else(|| DEFAULT_TCP.to_string());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| panic!("bind {addr}: {e}"));
    log::info!("listening on tcp://{addr}");
    let shutdown = shutdown_signal();
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else { continue };
                tokio::spawn(server::handle_conn(
                    stream,
                    Arc::clone(&core),
                    peripherals.clone(),
                    bus.clone(),
                ));
            }
            _ = &mut shutdown => break,
        }
    }
    telemetry_task.abort();
    ddc_rescan_task.abort();
    restore_auto_before_exit(&core).await;
}

async fn restore_auto_before_exit(core: &server::SharedCore) {
    let Ok(mut core) = tokio::time::timeout(std::time::Duration::from_secs(3), core.lock()).await
    else {
        log::error!("thermal core busy during shutdown; ExecStopPost will restore Auto");
        return;
    };
    match core.restore_auto_fan() {
        Ok(()) => log::info!("shutdown: restored EC automatic fan control"),
        Err(e) => log::error!("shutdown: could not restore EC automatic fan control: {e}"),
    }
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut terminate = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate.recv() => {}
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

/// TCP is a development transport: never expose the privileged real-hardware
/// API through it, and never listen beyond the local machine.
fn validate_tcp(addr: &str, mock_mode: bool) -> Result<SocketAddr, String> {
    if !mock_mode {
        return Err("TCP requires --mock; real hardware is Unix-socket only".into());
    }
    let addr: SocketAddr = addr
        .parse()
        .map_err(|_| "use a numeric loopback address such as 127.0.0.1:7331".to_string())?;
    if !addr.ip().is_loopback() {
        return Err("only loopback addresses are allowed".into());
    }
    Ok(addr)
}

/// Make the socket usable by the `fang` group (0660 root:fang) so the UI
/// doesn't need root. Falls back with a warning when the group is missing.
#[cfg(unix)]
fn grant_socket_access(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o660)) {
        log::warn!("chmod {}: {e}", path.display());
    }
    match fang_gid() {
        Some(gid) => {
            if let Err(e) = std::os::unix::fs::chown(path, None, Some(gid)) {
                log::warn!("chgrp fang {}: {e}", path.display());
            }
        }
        None => log::warn!(
            "group 'fang' not found; only root can talk to the socket. \
             Create it with: groupadd fang && usermod -aG fang $USER"
        ),
    }
}

#[cfg(unix)]
fn fang_gid() -> Option<u32> {
    let groups = std::fs::read_to_string("/etc/group").ok()?;
    groups.lines().find_map(|l| {
        let mut parts = l.split(':');
        (parts.next()? == "fang").then(|| parts.nth(1)?.parse().ok())?
    })
}

#[cfg(test)]
mod tests {
    use super::validate_tcp;

    #[test]
    fn tcp_is_mock_only_and_loopback_only() {
        assert!(validate_tcp("127.0.0.1:7331", true).is_ok());
        assert!(validate_tcp("[::1]:7331", true).is_ok());
        assert!(validate_tcp("127.0.0.1:7331", false).is_err());
        assert!(validate_tcp("0.0.0.0:7331", true).is_err());
        assert!(validate_tcp("192.168.1.5:7331", true).is_err());
        assert!(validate_tcp("localhost:7331", true).is_err());
    }
}
