use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub const SESSION_PROTOCOL: &str = "ember.session";
pub const SESSION_PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SessionEnvelope {
    pub protocol: String,
    pub protocol_version: u32,
    pub session_id: String,
    pub message_id: String,
    pub body: SessionBody,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum SessionBody {
    Command(SessionCommand),
    Event(SessionEvent),
    Response(SessionResponse),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "name", content = "arguments", rename_all = "snake_case")]
pub enum SessionCommand {
    LoadLevel { path: String },
    Play,
    Pause,
    Resume,
    Step { ticks: u32 },
    Stop,
    Snapshot,
    ReloadDocument { id: String, revision: u64 },
    ReloadAsset { id: String, revision: String },
    Capture { id: String, output_path: String },
    Shutdown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "name", content = "data", rename_all = "snake_case")]
pub enum SessionEvent {
    Starting,
    Ready,
    Loaded { level: String },
    Playing,
    Paused,
    Stopped,
    Snapshot { data: Value },
    CaptureReady { id: String, output_path: String },
    Log { message: String },
    Diagnostic { message: String },
    Crashed { message: String },
    Disconnected,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SessionResponse {
    pub ok: bool,
    pub summary: String,
    #[serde(default)]
    pub data: Option<Value>,
}

impl SessionEnvelope {
    pub fn command(
        session_id: impl Into<String>,
        message_id: impl Into<String>,
        command: SessionCommand,
    ) -> Self {
        Self {
            protocol: SESSION_PROTOCOL.into(),
            protocol_version: SESSION_PROTOCOL_VERSION,
            session_id: session_id.into(),
            message_id: message_id.into(),
            body: SessionBody::Command(command),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.protocol != SESSION_PROTOCOL {
            return Err(format!("unsupported session protocol {}", self.protocol));
        }
        if self.protocol_version != SESSION_PROTOCOL_VERSION {
            return Err(format!(
                "unsupported session protocol version {}",
                self.protocol_version
            ));
        }
        if self.session_id.trim().is_empty() || self.message_id.trim().is_empty() {
            return Err("session_id and message_id are required".into());
        }
        Ok(())
    }
}

pub fn write_jsonl<W: Write>(writer: &mut W, envelope: &SessionEnvelope) -> Result<(), String> {
    envelope.validate()?;
    serde_json::to_writer(&mut *writer, envelope).map_err(|error| error.to_string())?;
    writer.write_all(b"\n").map_err(|error| error.to_string())?;
    writer.flush().map_err(|error| error.to_string())
}

pub fn read_jsonl<R: BufRead>(reader: &mut R) -> Result<Option<SessionEnvelope>, String> {
    let mut line = String::new();
    if reader
        .read_line(&mut line)
        .map_err(|error| error.to_string())?
        == 0
    {
        return Ok(None);
    }
    let envelope: SessionEnvelope =
        serde_json::from_str(line.trim()).map_err(|error| error.to_string())?;
    envelope.validate()?;
    Ok(Some(envelope))
}

pub struct RuntimeProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    session_id: String,
    next_message: u64,
}

impl RuntimeProcess {
    pub fn spawn(executable: &Path, extra_args: &[String]) -> Result<Self, String> {
        let mut command = Command::new(executable);
        command.args(extra_args);
        command.arg("--stdio-session");
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| error.to_string())?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "runtime stdin unavailable".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "runtime stdout unavailable".to_string())?;
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            session_id: format!("session-{}", std::process::id()),
            next_message: 1,
        })
    }

    pub fn request(&mut self, command: SessionCommand) -> Result<SessionEnvelope, String> {
        let message_id = format!("msg-{}", self.next_message);
        self.next_message += 1;
        let envelope =
            SessionEnvelope::command(self.session_id.clone(), message_id.clone(), command);
        write_jsonl(&mut self.stdin, &envelope)?;
        loop {
            let response = read_jsonl(&mut self.stdout)?
                .ok_or_else(|| "runtime session closed before response".to_string())?;
            if response.message_id == message_id {
                return Ok(response);
            }
        }
    }

    pub fn shutdown(mut self) -> Result<(), String> {
        let _ = self.request(SessionCommand::Shutdown);
        self.child.wait().map_err(|error| error.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn jsonl_round_trip_preserves_command() {
        let envelope = SessionEnvelope::command("session", "1", SessionCommand::Step { ticks: 2 });
        let mut bytes = Vec::new();
        write_jsonl(&mut bytes, &envelope).unwrap();
        let mut cursor = Cursor::new(bytes);
        let decoded = read_jsonl(&mut cursor).unwrap().unwrap();
        assert_eq!(decoded, envelope);
    }
}
