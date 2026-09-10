use ember_core::StableId;
use ember_level::{save_level_json, LevelDocument};
use ember_session::{SessionBody, SessionCommand, SessionEnvelope, SessionEvent, SessionResponse};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
const EXIT_TIMEOUT: Duration = Duration::from_secs(5);

struct RuntimeHarness {
    child: Child,
    stdin: ChildStdin,
    stdout_lines: mpsc::Receiver<Result<String, String>>,
    stderr: Arc<Mutex<String>>,
    next_message: u64,
    finished: bool,
}

impl RuntimeHarness {
    fn spawn() -> Result<Self, String> {
        let executable = env!("CARGO_BIN_EXE_ember_runtime_host");
        let mut child = Command::new(executable)
            .arg("--stdio-session")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("failed to spawn runtime service: {error}"))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "runtime stdin was not piped".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "runtime stdout was not piped".to_string())?;
        let stderr_pipe = child
            .stderr
            .take()
            .ok_or_else(|| "runtime stderr was not piped".to_string())?;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let item = line.map_err(|error| error.to_string());
                if tx.send(item).is_err() {
                    break;
                }
            }
        });

        let stderr = Arc::new(Mutex::new(String::new()));
        let stderr_target = Arc::clone(&stderr);
        thread::spawn(move || {
            let mut reader = BufReader::new(stderr_pipe);
            let mut text = String::new();
            let _ = reader.read_to_string(&mut text);
            if let Ok(mut target) = stderr_target.lock() {
                *target = text;
            }
        });

        Ok(Self {
            child,
            stdin,
            stdout_lines: rx,
            stderr,
            next_message: 1,
            finished: false,
        })
    }

    fn stderr_text(&self) -> String {
        self.stderr
            .lock()
            .map(|value| value.clone())
            .unwrap_or_else(|_| "<stderr lock poisoned>".to_string())
    }

    fn next_envelope(&self) -> Result<SessionEnvelope, String> {
        let line = self
            .stdout_lines
            .recv_timeout(RESPONSE_TIMEOUT)
            .map_err(|error| {
                format!(
                    "runtime response timeout/disconnect: {error}; stderr={}",
                    self.stderr_text()
                )
            })??;
        serde_json::from_str(&line)
            .map_err(|error| format!("invalid runtime JSONL {line:?}: {error}"))
    }

    fn wait_ready(&self) -> Result<(), String> {
        let deadline = Instant::now() + RESPONSE_TIMEOUT;
        while Instant::now() < deadline {
            let envelope = self.next_envelope()?;
            if matches!(envelope.body, SessionBody::Event(SessionEvent::Ready)) {
                return Ok(());
            }
        }
        Err(format!(
            "runtime never emitted Ready; stderr={}",
            self.stderr_text()
        ))
    }

    fn request(&mut self, command: SessionCommand) -> Result<SessionResponse, String> {
        let message_id = format!("smoke-{}", self.next_message);
        self.next_message += 1;
        let envelope =
            SessionEnvelope::command("runtime-service-smoke", message_id.clone(), command);

        serde_json::to_writer(&mut self.stdin, &envelope).map_err(|error| error.to_string())?;
        self.stdin
            .write_all(b"\n")
            .map_err(|error| error.to_string())?;
        self.stdin.flush().map_err(|error| error.to_string())?;

        let deadline = Instant::now() + RESPONSE_TIMEOUT;
        while Instant::now() < deadline {
            let envelope = self.next_envelope()?;
            if envelope.message_id != message_id {
                continue;
            }
            return match envelope.body {
                SessionBody::Response(response) => Ok(response),
                other => Err(format!(
                    "runtime returned matching non-response envelope: {other:?}"
                )),
            };
        }

        Err(format!(
            "runtime did not answer {message_id}; stderr={}",
            self.stderr_text()
        ))
    }

    fn shutdown(&mut self) -> Result<(), String> {
        let response = self.request(SessionCommand::Shutdown)?;
        if !response.ok {
            return Err(format!("runtime rejected Shutdown: {}", response.summary));
        }

        let deadline = Instant::now() + EXIT_TIMEOUT;
        loop {
            if let Some(status) = self.child.try_wait().map_err(|error| error.to_string())? {
                self.finished = true;
                if status.success() {
                    return Ok(());
                }
                return Err(format!(
                    "runtime exited unsuccessfully: {status}; stderr={}",
                    self.stderr_text()
                ));
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "runtime did not exit after Shutdown; stderr={}",
                    self.stderr_text()
                ));
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for RuntimeHarness {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn unique_level_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ember-runtime-service-smoke-{}-{nonce}.level.json",
        std::process::id()
    ))
}

fn require_ok(response: SessionResponse, operation: &str) -> SessionResponse {
    assert!(
        response.ok,
        "runtime operation {operation} failed: {}",
        response.summary
    );
    response
}

#[test]
fn stdio_runtime_service_process_round_trip_is_bounded_and_stateful() {
    let path = unique_level_path();
    let level = LevelDocument::new(
        StableId::new("level", "stdio-smoke").expect("valid smoke level id"),
        "Stdio Smoke",
        320,
        180,
    );
    save_level_json(&path, &level).expect("write temporary smoke level");

    let result = (|| -> Result<(), String> {
        let mut runtime = RuntimeHarness::spawn()?;
        runtime.wait_ready()?;

        require_ok(
            runtime.request(SessionCommand::LoadLevel {
                path: path.to_string_lossy().into_owned(),
            })?,
            "LoadLevel",
        );

        let snapshot = require_ok(runtime.request(SessionCommand::Snapshot)?, "Snapshot");
        let data = snapshot
            .data
            .ok_or_else(|| "Snapshot returned no data".to_string())?;
        if data.get("ready").and_then(serde_json::Value::as_bool) != Some(true) {
            return Err(format!("Snapshot was not ready: {data}"));
        }
        if data.get("active_level").and_then(serde_json::Value::as_str) != Some("level:stdio-smoke")
        {
            return Err(format!("Snapshot did not retain loaded level: {data}"));
        }

        require_ok(runtime.request(SessionCommand::Play)?, "Play");
        let playing = require_ok(
            runtime.request(SessionCommand::Snapshot)?,
            "Snapshot playing",
        );
        if playing
            .data
            .as_ref()
            .and_then(|value| value.get("playing"))
            .and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            return Err(format!(
                "runtime did not enter playing state: {:?}",
                playing.data
            ));
        }

        require_ok(runtime.request(SessionCommand::Step { ticks: 2 })?, "Step");
        require_ok(runtime.request(SessionCommand::Stop)?, "Stop");

        let stopped = require_ok(
            runtime.request(SessionCommand::Snapshot)?,
            "Snapshot stopped",
        );
        if stopped
            .data
            .as_ref()
            .and_then(|value| value.get("playing"))
            .and_then(serde_json::Value::as_bool)
            != Some(false)
        {
            return Err(format!("runtime did not stop: {:?}", stopped.data));
        }

        runtime.shutdown()
    })();

    let _ = std::fs::remove_file(&path);
    if let Err(error) = result {
        panic!("runtime stdio process certification failed: {error}");
    }
}
