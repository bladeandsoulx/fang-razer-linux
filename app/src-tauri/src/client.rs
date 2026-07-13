//! Persistent connection to fangd with auto-reconnect.
//!
//! Owns the socket from a single task: requests come in over an mpsc channel,
//! responses are matched by id, and pushed daemon events are re-emitted as
//! Tauri events (`fang://telemetry`, `fang://status`, `fang://connected`).

use fang_protocol::api::{Command, Event, Request, Response, API_VERSION};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

type Waiter = oneshot::Sender<Result<Value, String>>;
const QUEUE_TIMEOUT: Duration = Duration::from_secs(2);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(35);

#[derive(Clone)]
pub struct Client {
    tx: mpsc::Sender<(Command, Waiter)>,
    connected: Arc<AtomicBool>,
    daemon_api_version: Arc<AtomicU16>,
}

#[derive(Clone, Debug, Serialize)]
pub struct VersionInfo {
    pub app_version: &'static str,
    pub app_api_version: u16,
    pub daemon_api_version: Option<u16>,
    pub compatible: bool,
}

impl Client {
    pub fn connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub async fn request(&self, cmd: Command) -> Result<Value, String> {
        if cmd.is_mutating() {
            let daemon = self.daemon_api_version.load(Ordering::Relaxed);
            if daemon != API_VERSION {
                return Err(if daemon == 0 {
                    "daemon API is unknown or outdated; update/restart both Fang packages".into()
                } else {
                    format!(
                        "incompatible Fang API: app {API_VERSION}, daemon {daemon}; update both packages"
                    )
                });
            }
        }
        let (tx, rx) = oneshot::channel();
        tokio::time::timeout(QUEUE_TIMEOUT, self.tx.send((cmd, tx)))
            .await
            .map_err(|_| "daemon request queue is busy".to_string())?
            .map_err(|_| "daemon connection task gone".to_string())?;
        tokio::time::timeout(REQUEST_TIMEOUT, rx)
            .await
            .map_err(|_| "daemon request timed out".to_string())?
            .map_err(|_| "daemon offline".to_string())?
    }

    pub fn version_info(&self) -> VersionInfo {
        version_info(&self.daemon_api_version)
    }
}

fn version_info(daemon_api_version: &AtomicU16) -> VersionInfo {
    let daemon = daemon_api_version.load(Ordering::Relaxed);
    VersionInfo {
        app_version: env!("CARGO_PKG_VERSION"),
        app_api_version: API_VERSION,
        daemon_api_version: (daemon != 0).then_some(daemon),
        compatible: daemon == API_VERSION,
    }
}

fn record_api_version(data: &Value, version: &AtomicU16, app: &AppHandle) {
    let api = data
        .get("api_version")
        .and_then(Value::as_u64)
        .and_then(|v| u16::try_from(v).ok())
        .unwrap_or(0);
    version.store(api, Ordering::Relaxed);
    let _ = app.emit("fang://compatibility", version_info(version));
}

/// `FANGD_ADDR` overrides: `unix:/run/fangd.sock` or `tcp:127.0.0.1:7331`.
fn daemon_addr() -> String {
    if let Ok(a) = std::env::var("FANGD_ADDR") {
        return a;
    }
    #[cfg(target_os = "linux")]
    {
        "unix:/run/fangd.sock".into()
    }
    #[cfg(not(target_os = "linux"))]
    {
        "tcp:127.0.0.1:7331".into()
    }
}

async fn open_stream(
    addr: &str,
) -> std::io::Result<(
    Box<dyn AsyncRead + Send + Unpin>,
    Box<dyn AsyncWrite + Send + Unpin>,
)> {
    if let Some(path) = addr.strip_prefix("unix:") {
        #[cfg(unix)]
        {
            let (r, w) = tokio::net::UnixStream::connect(path).await?.into_split();
            return Ok((Box::new(r), Box::new(w)));
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            return Err(std::io::Error::other("unix sockets unsupported here"));
        }
    }
    let target = addr.strip_prefix("tcp:").unwrap_or(addr);
    let (r, w) = tokio::net::TcpStream::connect(target).await?.into_split();
    Ok((Box::new(r), Box::new(w)))
}

pub fn spawn(app: AppHandle) -> Client {
    let (tx, mut rx) = mpsc::channel::<(Command, Waiter)>(16);
    let connected = Arc::new(AtomicBool::new(false));
    let daemon_api_version = Arc::new(AtomicU16::new(0));
    let client = Client {
        tx,
        connected: Arc::clone(&connected),
        daemon_api_version: Arc::clone(&daemon_api_version),
    };

    tauri::async_runtime::spawn(async move {
        let addr = daemon_addr();
        loop {
            let (read, mut write) = match open_stream(&addr).await {
                Ok(pair) => pair,
                Err(_) => {
                    // Drain queued requests with an error while offline.
                    while let Ok((_, waiter)) = rx.try_recv() {
                        let _ = waiter.send(Err("daemon offline".into()));
                    }
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            log::info!("connected to fangd at {addr}");
            connected.store(true, Ordering::Relaxed);
            let _ = app.emit("fang://connected", true);

            let mut lines = BufReader::new(read).lines();
            let mut pending: HashMap<u64, Waiter> = HashMap::new();
            let mut next_id: u64 = 1;

            // Prime: subscribe to events and fetch current status.
            for cmd in [Command::Subscribe, Command::GetStatus] {
                let req = Request {
                    id: next_id,
                    api_version: API_VERSION,
                    cmd,
                };
                next_id += 1;
                let mut line = serde_json::to_string(&req).expect("serializable");
                line.push('\n');
                if write.write_all(line.as_bytes()).await.is_err() {
                    break;
                }
            }

            loop {
                tokio::select! {
                    line = lines.next_line() => {
                        let Ok(Some(line)) = line else { break };
                        if let Ok(ev) = serde_json::from_str::<Event>(&line) {
                            match ev {
                                Event::Telemetry(t) => { let _ = app.emit("fang://telemetry", t); }
                                Event::StateChanged(s) => {
                                    daemon_api_version.store(s.api_version, Ordering::Relaxed);
                                    let _ = app.emit(
                                        "fang://compatibility",
                                        version_info(&daemon_api_version),
                                    );
                                    let _ = app.emit("fang://status", s);
                                }
                            }
                        } else if let Ok(resp) = serde_json::from_str::<Response>(&line) {
                            let result = if resp.ok {
                                Ok(resp.data.unwrap_or(Value::Null))
                            } else {
                                Err(resp.error.unwrap_or_else(|| "daemon error".into()))
                            };
                            if let Some(waiter) = pending.remove(&resp.id) {
                                let _ = waiter.send(result);
                            } else if let Ok(data) = result {
                                // Response to our own priming get_status.
                                if data.get("perf_mode").is_some() {
                                    record_api_version(&data, &daemon_api_version, &app);
                                    let _ = app.emit("fang://status", data);
                                }
                            }
                        }
                    }
                    req = rx.recv() => {
                        let Some((cmd, waiter)) = req else { return };
                        let req = Request {
                            id: next_id,
                            api_version: API_VERSION,
                            cmd,
                        };
                        next_id += 1;
                        let mut line = serde_json::to_string(&req).expect("serializable");
                        line.push('\n');
                        match write.write_all(line.as_bytes()).await {
                            Ok(()) => { pending.insert(req.id, waiter); }
                            Err(e) => { let _ = waiter.send(Err(format!("write: {e}"))); break; }
                        }
                    }
                }
            }

            connected.store(false, Ordering::Relaxed);
            daemon_api_version.store(0, Ordering::Relaxed);
            let _ = app.emit("fang://connected", false);
            let _ = app.emit("fang://compatibility", version_info(&daemon_api_version));
            for (_, waiter) in pending.drain() {
                let _ = waiter.send(Err("daemon disconnected".into()));
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    client
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_info_requires_an_exact_api_match() {
        let daemon = AtomicU16::new(0);
        assert!(!version_info(&daemon).compatible);
        daemon.store(API_VERSION, Ordering::Relaxed);
        let info = version_info(&daemon);
        assert!(info.compatible);
        assert_eq!(info.daemon_api_version, Some(API_VERSION));
    }
}
