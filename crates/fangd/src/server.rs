//! JSON-lines socket server plus the 1 Hz telemetry loop.

use crate::core::Core;
use crate::peripherals::{read_snapshot, Peripherals, SnapshotStore};
use fang_protocol::api::{Command, Event, Request, Response, Telemetry, API_VERSION};
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
pub async fn telemetry_loop(core: SharedCore, peripherals: SnapshotStore, bus: EventBus) {
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

        let on_ac = crate::power::on_ac();
        let mut core = core.lock().await;
        if jumped {
            log::info!("wall clock jump detected (resume from suspend); reapplying state");
            core.reapply();
        }
        let s = core.sample();
        let auto_changed = core.power_tick(on_ac);
        let auto_status = auto_changed.then(|| core.status(&read_snapshot(&peripherals)));
        drop(core);

        let telemetry = Telemetry {
            cpu_temp_c: s.hw.cpu_temp_c,
            gpu_temp_c: s.hw.gpu_temp_c,
            cpu_power_w: s.hw.cpu_power_w,
            gpu_power_w: s.hw.gpu_power_w,
            on_ac,
            fan_rpm: s.hw.fan_rpm,
            fan_target_rpm: s.fan_target_rpm,
            thermal_override_active: s.thermal_override_active,
            thermal_sensor_ok: s.thermal_sensor_ok,
            thermal_override_reason: s.thermal_override_reason,
            ts_ms: now
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };
        let _ = bus.send(event_line(&Event::Telemetry(telemetry)));
        // A power-source transition may have auto-switched the profile.
        if let Some(status) = auto_status {
            let _ = bus.send(event_line(&Event::StateChanged(status)));
        }
    }
}

async fn current_status(
    core: &SharedCore,
    peripherals: &Peripherals,
) -> fang_protocol::api::Status {
    let snapshot = peripherals.snapshot();
    core.lock().await.status(&snapshot)
}

pub async fn handle_conn<S>(stream: S, core: SharedCore, peripherals: Peripherals, bus: EventBus)
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
                if req.cmd.is_mutating() && req.api_version != API_VERSION {
                    Response::err(
                        id,
                        format!(
                            "incompatible Fang API: client {}, daemon {API_VERSION}; update both packages",
                            req.api_version
                        ),
                    )
                } else {
                    match req.cmd {
                        Command::Ping => Response::ok(id, "pong"),
                        Command::GetStatus => {
                            Response::ok(id, current_status(&core, &peripherals).await)
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
                        Command::SetGpuMode { gpu_mode } => {
                            match peripherals.set_gpu_mode(gpu_mode).await {
                                Ok(()) => {
                                    let status = current_status(&core, &peripherals).await;
                                    let _ =
                                        bus.send(event_line(&Event::StateChanged(status.clone())));
                                    Response::ok(id, status)
                                }
                                Err(e) => Response::err(id, e),
                            }
                        }
                        Command::SetColorPreset { value } => {
                            match peripherals.set_color_preset(value).await {
                                Ok(()) => {
                                    let status = current_status(&core, &peripherals).await;
                                    let _ =
                                        bus.send(event_line(&Event::StateChanged(status.clone())));
                                    Response::ok(id, status)
                                }
                                Err(e) => Response::err(id, e),
                            }
                        }
                        Command::SetMonitorBrightness { value } => {
                            match peripherals.set_monitor_brightness(value).await {
                                Ok(()) => {
                                    let status = current_status(&core, &peripherals).await;
                                    let _ =
                                        bus.send(event_line(&Event::StateChanged(status.clone())));
                                    Response::ok(id, status)
                                }
                                Err(e) => Response::err(id, e),
                            }
                        }
                        ref cmd @ (Command::SetPerfMode { .. }
                        | Command::SetFan { .. }
                        | Command::SetBho { .. }
                        | Command::SetLighting { .. }
                        | Command::SetAutoPower { .. }) => {
                            let snapshot = peripherals.snapshot();
                            let mut core = core.lock().await;
                            match core.handle_set(cmd) {
                                Ok(changed) => {
                                    let status = core.status(&snapshot);
                                    drop(core);
                                    if changed {
                                        let _ = bus
                                            .send(event_line(&Event::StateChanged(status.clone())));
                                    }
                                    Response::ok(id, status)
                                }
                                Err(e) => Response::err(id, e),
                            }
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
