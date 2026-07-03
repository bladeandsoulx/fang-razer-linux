//! Persistent connection to fangd with auto-reconnect.
//!
//! Owns the socket from a single task: requests come in over an mpsc channel,
//! responses are matched by id, and pushed daemon events are re-emitted as
//! Tauri events (`fang://telemetry`, `fang://status`, `fang://connected`).

use fang_protocol::api::{Command, Event, Request, Response};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

type Waiter = oneshot::Sender<Result<Value, String>>;

#[derive(Clone)]
pub struct Client {
    tx: mpsc::Sender<(Command, Waiter)>,
    connected: Arc<AtomicBool>,
}

impl Client {
    pub fn connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub async fn request(&self, cmd: Command) -> Result<Value, String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send((cmd, tx))
            .await
            .map_err(|_| "daemon connection task gone".to_string())?;
        rx.await.map_err(|_| "daemon offline".to_string())?
    }
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
    let client = Client {
        tx,
        connected: Arc::clone(&connected),
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
                let req = Request { id: next_id, cmd };
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
                                Event::StateChanged(s) => { let _ = app.emit("fang://status", s); }
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
                                    let _ = app.emit("fang://status", data);
                                }
                            }
                        }
                    }
                    req = rx.recv() => {
                        let Some((cmd, waiter)) = req else { return };
                        let req = Request { id: next_id, cmd };
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
            let _ = app.emit("fang://connected", false);
            for (_, waiter) in pending.drain() {
                let _ = waiter.send(Err("daemon disconnected".into()));
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });

    client
}
