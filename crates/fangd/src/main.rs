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
#[cfg(unix)]
const HARDWARE_LOCK: &str = "/run/fangd.lock";

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

#[cfg(unix)]
struct InstanceLock {
    _file: std::fs::File,
}

#[cfg(unix)]
impl InstanceLock {
    fn acquire(path: &std::path::Path) -> std::io::Result<Self> {
        use std::os::fd::AsRawFd;
        use std::os::unix::fs::OpenOptionsExt;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
            .open(path)
            .map_err(|e| {
                std::io::Error::new(
                    e.kind(),
                    format!("open instance lock {}: {e}", path.display()),
                )
            })?;
        // SAFETY: `file` owns a valid descriptor for the lifetime of the lock.
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result != 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::WouldBlock {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!("another fangd instance holds {}", path.display()),
                ));
            }
            return Err(std::io::Error::new(
                error.kind(),
                format!("lock {}: {error}", path.display()),
            ));
        }
        Ok(InstanceLock { _file: file })
    }
}

#[cfg(unix)]
struct UnixSocketGuard {
    path: PathBuf,
    device: u64,
    inode: u64,
    _instance_lock: InstanceLock,
}

#[cfg(unix)]
impl Drop for UnixSocketGuard {
    fn drop(&mut self) {
        use std::os::unix::fs::{FileTypeExt, MetadataExt};

        let Ok(metadata) = std::fs::symlink_metadata(&self.path) else {
            return;
        };
        // Never unlink a path another process replaced while Fang was
        // shutting down. Only remove the exact socket inode we bound.
        if metadata.file_type().is_socket()
            && metadata.dev() == self.device
            && metadata.ino() == self.inode
        {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(unix)]
fn derived_socket_lock_path(socket_path: &std::path::Path) -> PathBuf {
    let mut path = socket_path.as_os_str().to_os_string();
    path.push(".lock");
    path.into()
}

/// Bind without ever deleting a live listener or a non-socket filesystem
/// entry. A refused connection proves that a leftover socket inode is stale.
#[cfg(unix)]
fn bind_unix_socket(path: &std::path::Path) -> std::io::Result<std::os::unix::net::UnixListener> {
    use std::os::unix::fs::FileTypeExt;
    use std::os::unix::net::{UnixListener, UnixStream};

    match UnixListener::bind(path) {
        Ok(listener) => return Ok(listener),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {}
        Err(e) => {
            return Err(std::io::Error::new(
                e.kind(),
                format!("bind {}: {e}", path.display()),
            ))
        }
    }

    match UnixStream::connect(path) {
        Ok(_) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("another fangd instance is listening on {}", path.display()),
            ))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return UnixListener::bind(path).map_err(|retry| {
                std::io::Error::new(
                    retry.kind(),
                    format!("bind {} after startup race: {retry}", path.display()),
                )
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {}
        Err(e) => {
            return Err(std::io::Error::new(
                e.kind(),
                format!(
                    "cannot prove that existing socket {} is stale: {e}",
                    path.display()
                ),
            ))
        }
    }

    let metadata = std::fs::symlink_metadata(path).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("inspect existing socket {}: {e}", path.display()),
        )
    })?;
    if !metadata.file_type().is_socket() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("refusing to replace non-socket path {}", path.display()),
        ));
    }
    std::fs::remove_file(path).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("remove stale socket {}: {e}", path.display()),
        )
    })?;
    UnixListener::bind(path).map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("bind {} after removing stale socket: {e}", path.display()),
        )
    })
}

#[cfg(unix)]
fn prepare_unix_socket(
    path: &std::path::Path,
    lock_path: &std::path::Path,
) -> std::io::Result<(UnixSocketGuard, std::os::unix::net::UnixListener)> {
    use std::os::unix::fs::MetadataExt;

    let instance_lock = InstanceLock::acquire(lock_path)?;
    let listener = bind_unix_socket(path)?;
    let metadata = std::fs::symlink_metadata(path).map_err(|e| {
        let _ = std::fs::remove_file(path);
        std::io::Error::new(
            e.kind(),
            format!("inspect newly bound socket {}: {e}", path.display()),
        )
    })?;
    let guard = UnixSocketGuard {
        path: path.to_path_buf(),
        device: metadata.dev(),
        inode: metadata.ino(),
        _instance_lock: instance_lock,
    };
    Ok((guard, listener))
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

    // Claim the process-wide hardware lock and bind the socket before opening
    // HID or applying any persisted state. A rejected second process therefore
    // cannot issue even one EC command. Mock restore helpers do not touch real
    // hardware and retain their unprivileged, socket-free behavior in tests.
    #[cfg(unix)]
    let unix_server = if args.tcp.is_none() && !(args.mock && args.restore_auto) {
        let path = args
            .socket
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET));
        let lock_path = if args.mock {
            derived_socket_lock_path(&path)
        } else {
            PathBuf::from(HARDWARE_LOCK)
        };
        let (guard, listener) = match prepare_unix_socket(&path, &lock_path) {
            Ok(prepared) => prepared,
            Err(e) => {
                eprintln!("cannot start fangd: {e}");
                std::process::exit(1);
            }
        };
        listener
            .set_nonblocking(true)
            .unwrap_or_else(|e| panic!("set {} nonblocking: {e}", path.display()));
        let listener = tokio::net::UnixListener::from_std(listener)
            .unwrap_or_else(|e| panic!("register {} with async runtime: {e}", path.display()));
        grant_socket_access(&path);
        Some((path, listener, guard))
    } else {
        None
    };

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
    if let Some((path, listener, _socket_guard)) = unix_server {
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
    #[cfg(unix)]
    use super::{prepare_unix_socket, InstanceLock};

    #[test]
    fn tcp_is_mock_only_and_loopback_only() {
        assert!(validate_tcp("127.0.0.1:7331", true).is_ok());
        assert!(validate_tcp("[::1]:7331", true).is_ok());
        assert!(validate_tcp("127.0.0.1:7331", false).is_err());
        assert!(validate_tcp("0.0.0.0:7331", true).is_err());
        assert!(validate_tcp("192.168.1.5:7331", true).is_err());
        assert!(validate_tcp("localhost:7331", true).is_err());
    }

    #[cfg(unix)]
    fn socket_test_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fangd-{name}-socket-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    #[test]
    fn instance_lock_rejects_a_second_holder_and_releases_on_drop() {
        let dir = socket_test_dir("instance-lock");
        let path = dir.join("fangd.lock");
        let first = InstanceLock::acquire(&path).unwrap();
        let error = InstanceLock::acquire(&path).err().unwrap();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);

        drop(first);
        InstanceLock::acquire(&path).expect("lock must release with its file");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn live_socket_is_refused_and_never_unlinked() {
        let dir = socket_test_dir("live");
        let socket = dir.join("fangd.sock");
        let lock = dir.join("fangd.lock");
        let live = std::os::unix::net::UnixListener::bind(&socket).unwrap();

        let error = prepare_unix_socket(&socket, &lock).err().unwrap();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert!(socket.exists(), "the live socket path was removed");

        drop(live);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn stale_socket_is_replaced_and_owned_socket_is_cleaned_up() {
        let dir = socket_test_dir("stale");
        let socket = dir.join("fangd.sock");
        let lock = dir.join("fangd.lock");
        let stale = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        drop(stale);
        assert!(socket.exists());

        let (guard, listener) = prepare_unix_socket(&socket, &lock).unwrap();
        std::os::unix::net::UnixStream::connect(&socket)
            .expect("replacement listener must accept connections");
        drop(listener);
        drop(guard);
        assert!(!socket.exists(), "owned socket was not cleaned up");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(unix)]
    #[test]
    fn regular_file_at_socket_path_is_never_removed() {
        let dir = socket_test_dir("regular-file");
        let socket = dir.join("fangd.sock");
        let lock = dir.join("fangd.lock");
        std::fs::write(&socket, b"keep me").unwrap();

        let error = prepare_unix_socket(&socket, &lock).err().unwrap();
        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&socket).unwrap(), b"keep me");
        let _ = std::fs::remove_dir_all(dir);
    }
}
