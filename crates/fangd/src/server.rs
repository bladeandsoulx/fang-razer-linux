//! JSON-lines socket server plus the 1 Hz telemetry loop.

use crate::core::Core;
use fang_protocol::api::{Command, Event, Request, Response, Telemetry};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, Mutex};

pub type SharedCore = Arc<Mutex<Core>>;
pub type EventBus = broadcast::Sender<String>;

pub fn event_bus() -> EventBus {
    broadcast::channel(64).0
}

fn event_line(event: &Event) -> String {
    let mut s = serde_json::to_string(event).expect("serializable");
    s.push('\n');
    s
}

/// 1 Hz: sample sensors, broadcast telemetry, and reapply state after a
/// suspend/resume (detected as a wall-clock jump between ticks).
pub async fn telemetry_loop(core: SharedCore, bus: EventBus) {
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut last_wall = SystemTime::now();
    loop {
        tick.tick().await;
        let now = SystemTime::now();
        let jumped = now
            .duration_since(last_wall)
            .map(|d| d > Duration::from_secs(20))
            .unwrap_or(false);
        last_wall = now;

        let mut core = core.lock().await;
        if jumped {
            log::info!("wall clock jump detected (resume from suspend); reapplying state");
            core.reapply();
        }
        let s = core.sample();
        drop(core);

        let telemetry = Telemetry {
            cpu_temp_c: s.cpu_temp_c,
            gpu_temp_c: s.gpu_temp_c,
            fan_rpm: s.fan_rpm,
            ts_ms: now
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };
        let _ = bus.send(event_line(&Event::Telemetry(telemetry)));
    }
}

pub async fn handle_conn<S>(stream: S, core: SharedCore, bus: EventBus)
where
    S: AsyncRead + AsyncWrite + Send + 'static,
{
    let (read, write) = tokio::io::split(stream);
    let write = Arc::new(Mutex::new(write));
    let mut lines = BufReader::new(read).lines();
    let mut forwarder: Option<tokio::task::JoinHandle<()>> = None;

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let resp = match serde_json::from_str::<Request>(&line) {
            Ok(req) => {
                let id = req.id;
                match req.cmd {
                    Command::Ping => Response::ok(id, "pong"),
                    Command::GetStatus => {
                        let core = core.lock().await;
                        Response::ok(id, core.status())
                    }
                    Command::Subscribe => {
                        if forwarder.is_none() {
                            let mut rx = bus.subscribe();
                            let w = Arc::clone(&write);
                            forwarder = Some(tokio::spawn(async move {
                                while let Ok(msg) = rx.recv().await {
                                    let mut w = w.lock().await;
                                    if w.write_all(msg.as_bytes()).await.is_err() {
                                        break;
                                    }
                                }
                            }));
                        }
                        Response::ok(id, "subscribed")
                    }
                    ref cmd @ (Command::SetPerfMode { .. }
                    | Command::SetFan { .. }
                    | Command::SetGpuMode { .. }) => {
                        let mut core = core.lock().await;
                        match core.handle_set(cmd) {
                            Ok(changed) => {
                                let status = core.status();
                                drop(core);
                                if changed {
                                    let _ =
                                        bus.send(event_line(&Event::StateChanged(status.clone())));
                                }
                                Response::ok(id, status)
                            }
                            Err(e) => Response::err(id, e),
                        }
                    }
                }
            }
            Err(e) => Response::err(0, format!("bad request: {e}")),
        };

        let mut line = serde_json::to_string(&resp).expect("serializable");
        line.push('\n');
        let mut w = write.lock().await;
        if w.write_all(line.as_bytes()).await.is_err() {
            break;
        }
    }
    if let Some(f) = forwarder {
        f.abort();
    }
}
