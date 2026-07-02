mod core;
mod gpu;
mod hw;
mod server;
mod state;

use crate::core::Core;
use crate::state::AppliedState;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

const DEFAULT_TCP: &str = "127.0.0.1:7331";
#[cfg(unix)]
const DEFAULT_SOCKET: &str = "/run/fangd.sock";

struct Args {
    mock: bool,
    tcp: Option<String>,
    #[cfg_attr(not(unix), allow(dead_code))]
    socket: Option<PathBuf>,
    state: Option<PathBuf>,
}

fn parse_args() -> Args {
    let mut args = Args {
        mock: std::env::var("FANGD_MOCK").map(|v| v == "1").unwrap_or(false),
        tcp: None,
        socket: None,
        state: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--mock" => args.mock = true,
            "--tcp" => args.tcp = it.next(),
            "--socket" => args.socket = it.next().map(PathBuf::from),
            "--state" => args.state = it.next().map(PathBuf::from),
            "--help" | "-h" => {
                println!(
                    "fangd {} — Fang daemon for Razer Blade laptops\n\n\
                     USAGE: fangd [--mock] [--tcp ADDR] [--socket PATH] [--state PATH]\n\n\
                     --mock          simulate hardware (also FANGD_MOCK=1)\n\
                     --tcp ADDR      listen on TCP instead of the unix socket (dev)\n\
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

    let state_path = args.state.clone().unwrap_or_else(state::default_state_path);
    let state = AppliedState::load(&state_path);
    let hw = hw::open(args.mock);
    let gpu = gpu::open(args.mock);
    let mut core = Core::new(hw, gpu, state, state_path);
    core.reapply();
    log::info!("fangd {} — {}", env!("CARGO_PKG_VERSION"), core.status().model);

    let core: server::SharedCore = Arc::new(Mutex::new(core));
    let bus = server::event_bus();
    tokio::spawn(server::telemetry_loop(Arc::clone(&core), bus.clone()));

    #[cfg(unix)]
    if args.tcp.is_none() {
        let path = args.socket.clone().unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET));
        let _ = std::fs::remove_file(&path);
        let listener = tokio::net::UnixListener::bind(&path)
            .unwrap_or_else(|e| panic!("bind {}: {e}", path.display()));
        grant_socket_access(&path);
        log::info!("listening on {}", path.display());
        loop {
            tokio::select! {
                accepted = listener.accept() => {
                    let Ok((stream, _)) = accepted else { continue };
                    tokio::spawn(server::handle_conn(stream, Arc::clone(&core), bus.clone()));
                }
                _ = tokio::signal::ctrl_c() => break,
            }
        }
        let _ = std::fs::remove_file(&path);
        return;
    }

    let addr = args.tcp.unwrap_or_else(|| DEFAULT_TCP.to_string());
    let listener =
        tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| panic!("bind {addr}: {e}"));
    log::info!("listening on tcp://{addr}");
    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let Ok((stream, _)) = accepted else { continue };
                tokio::spawn(server::handle_conn(stream, Arc::clone(&core), bus.clone()));
            }
            _ = tokio::signal::ctrl_c() => break,
        }
    }
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
