//! End-to-end test: mock daemon over TCP, real client traffic.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

struct DaemonGuard(Child);

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn start_daemon(port: u16, state: &std::path::Path) -> DaemonGuard {
    let bin = env!("CARGO_BIN_EXE_fangd");
    let child = Command::new(bin)
        .args(["--mock", "--tcp", &format!("127.0.0.1:{port}")])
        .args(["--state".as_ref(), state.as_os_str()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn fangd");
    DaemonGuard(child)
}

fn connect(port: u16) -> TcpStream {
    for _ in 0..50 {
        if let Ok(s) = TcpStream::connect(("127.0.0.1", port)) {
            return s;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("fangd did not start listening");
}

fn roundtrip(reader: &mut impl BufRead, writer: &mut impl Write, req: &str) -> serde_json::Value {
    writer.write_all(req.as_bytes()).unwrap();
    writer.write_all(b"\n").unwrap();
    writer.flush().unwrap();
    let mut line = String::new();
    // Skip pushed event lines; find the response to our request.
    loop {
        line.clear();
        reader.read_line(&mut line).unwrap();
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        if v.get("event").is_none() {
            return v;
        }
    }
}

#[test]
fn daemon_end_to_end() {
    let dir = std::env::temp_dir().join(format!("fangd-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let state = dir.join("state.json");
    let port = 47331;
    let _daemon = start_daemon(port, &state);

    let stream = connect(port);
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;

    // get_status
    let v = roundtrip(&mut reader, &mut writer, r#"{"id":1,"cmd":"get_status"}"#);
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["data"]["mock"], true);
    assert_eq!(v["data"]["perf_mode"], "balanced");

    // switch to gaming
    let v = roundtrip(
        &mut reader,
        &mut writer,
        r#"{"id":2,"cmd":"set_perf_mode","perf_mode":"gaming"}"#,
    );
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["data"]["perf_mode"], "gaming");

    // manual fan gets clamped to model limits (Blade 18 mock: 2200..5000)
    let v = roundtrip(
        &mut reader,
        &mut writer,
        r#"{"id":3,"cmd":"set_fan","mode":"manual","rpm":9000}"#,
    );
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["data"]["fan"]["mode"], "manual");
    assert_eq!(v["data"]["fan"]["rpm"], 5000);

    // manual fan is independent of the power mode and survives a mode switch
    let v = roundtrip(
        &mut reader,
        &mut writer,
        r#"{"id":4,"cmd":"set_perf_mode","perf_mode":"silent"}"#,
    );
    assert_eq!(v["data"]["fan"]["mode"], "manual", "{v}");
    assert_eq!(v["data"]["fan"]["rpm"], 5000);
    let v = roundtrip(
        &mut reader,
        &mut writer,
        r#"{"id":5,"cmd":"set_fan","mode":"auto"}"#,
    );
    assert_eq!(v["data"]["fan"]["mode"], "auto", "{v}");

    // gpu mode: mock starts hybrid, switching marks a pending reboot
    let v = roundtrip(&mut reader, &mut writer, r#"{"id":10,"cmd":"get_status"}"#);
    assert_eq!(v["data"]["gpu_mode"], "hybrid", "{v}");
    assert_eq!(v["data"]["gpu_mode_pending"], false);
    let v = roundtrip(
        &mut reader,
        &mut writer,
        r#"{"id":11,"cmd":"set_gpu_mode","gpu_mode":"dedicated"}"#,
    );
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["data"]["gpu_mode"], "dedicated");
    assert_eq!(v["data"]["gpu_mode_pending"], true);

    // subscribe → telemetry event arrives within a few seconds
    let v = roundtrip(&mut reader, &mut writer, r#"{"id":6,"cmd":"subscribe"}"#);
    assert_eq!(v["ok"], true, "{v}");
    let mut got_telemetry = false;
    for _ in 0..5 {
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        if v["event"] == "telemetry" {
            assert!(v["data"]["fan_rpm"].as_array().is_some());
            got_telemetry = true;
            break;
        }
    }
    assert!(got_telemetry, "no telemetry event received");

    // state persisted
    drop(_daemon);
    std::thread::sleep(Duration::from_millis(200));
    let persisted = std::fs::read_to_string(&state).unwrap();
    assert!(persisted.contains("silent"), "{persisted}");
    let _ = std::fs::remove_dir_all(&dir);
}
