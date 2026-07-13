//! Bounded execution for helper programs used by the root daemon.

use std::io::Read;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn reader_thread<R: Read + Send + 'static>(mut reader: R) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = reader.read_to_end(&mut bytes);
        bytes
    })
}

/// Run a trusted local helper, kill it at the deadline, and collect its output.
/// Reader threads drain both pipes so a verbose helper cannot deadlock on a
/// full stdout/stderr buffer while the parent waits for it to exit.
pub fn output_with_timeout(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<Output, String> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // A dedicated process group lets the timeout terminate helper
        // descendants too (prime-select and envycontrol may spawn tools).
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|e| format!("{program}: {e}"))?;
    let stdout = child
        .stdout
        .take()
        .map(reader_thread)
        .ok_or_else(|| format!("{program}: stdout pipe unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .map(reader_thread)
        .ok_or_else(|| format!("{program}: stderr pipe unavailable"))?;
    let deadline = Instant::now() + timeout;

    let status: ExitStatus = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            Ok(None) => {
                kill_process_group(&mut child);
                let _ = child.wait();
                let _ = stdout.join();
                let _ = stderr.join();
                return Err(format!(
                    "{program} timed out after {} seconds",
                    timeout.as_secs()
                ));
            }
            Err(e) => {
                kill_process_group(&mut child);
                let _ = child.wait();
                let _ = stdout.join();
                let _ = stderr.join();
                return Err(format!("{program}: {e}"));
            }
        }
    };

    Ok(Output {
        status,
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
    })
}

#[cfg(unix)]
fn kill_process_group(child: &mut std::process::Child) {
    // SAFETY: the child was placed in a new process group whose id is its pid;
    // passing the negated id to kill targets that group and no other process.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
}

#[cfg(not(unix))]
fn kill_process_group(child: &mut std::process::Child) {
    let _ = child.kill();
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn captures_successful_output() {
        let out = output_with_timeout("sh", &["-c", "printf fang"], Duration::from_secs(1))
            .expect("helper output");
        assert!(out.status.success());
        assert_eq!(out.stdout, b"fang");
    }

    #[test]
    fn kills_a_timed_out_helper() {
        let started = Instant::now();
        let err = output_with_timeout("sh", &["-c", "sleep 2"], Duration::from_millis(50))
            .expect_err("must time out");
        assert!(err.contains("timed out"), "{err}");
        assert!(started.elapsed() < Duration::from_millis(500));
    }
}
