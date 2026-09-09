use ember_level::load_level_json;
use ember_runtime::{compile_level, BehaviorLibrary, RuntimeWorld};
use ember_runtime_bridge::{write_snapshot, RuntimeStateSnapshot};
use ember_session::{
    read_jsonl, write_jsonl, SessionBody, SessionCommand, SessionEnvelope, SessionEvent,
    SessionResponse, SESSION_PROTOCOL, SESSION_PROTOCOL_VERSION,
};
use serde_json::json;
use std::env;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

fn main() {
    if let Err(error) = run() {
        eprintln!("Ember Runtime failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|argument| argument == "--stdio-session") {
        return run_stdio_session();
    }
    let mut level_path: Option<PathBuf> = None;
    let mut session_dir: Option<PathBuf> = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--level" => {
                index += 1;
                level_path = args.get(index).map(PathBuf::from);
            }
            "--session-dir" => {
                index += 1;
                session_dir = args.get(index).map(PathBuf::from);
            }
            other => return Err(format!("unknown Ember argument: {other}")),
        }
        index += 1;
    }

    let mut snapshot = RuntimeStateSnapshot {
        schema_version: 1,
        runtime: "Ember Runtime".into(),
        ready: true,
        ..RuntimeStateSnapshot::default()
    };
    if let Some(level_path) = level_path {
        let level = load_level_json(&level_path).map_err(|error| error.to_string())?;
        let mut world =
            compile_level(&level, &BehaviorLibrary::new()).map_err(|error| error.to_string())?;
        world.tick_once()?;
        snapshot.active_level = Some(level.scene.id.to_string());
        snapshot.entity_count = world.entities.len();
        snapshot.semantic_cell_count = world.semantic_cells.len();
    }
    if let Some(session_dir) = session_dir {
        write_snapshot(&session_dir, &snapshot)?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&snapshot).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn run_stdio_session() -> Result<(), String> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = BufWriter::new(stdout.lock());
    let mut world: Option<RuntimeWorld> = None;
    let mut active_level: Option<String> = None;
    let mut playing = false;

    let ready = SessionEnvelope {
        protocol: SESSION_PROTOCOL.into(),
        protocol_version: SESSION_PROTOCOL_VERSION,
        session_id: "runtime".into(),
        message_id: "event-ready".into(),
        body: SessionBody::Event(SessionEvent::Ready),
    };
    write_jsonl(&mut writer, &ready)?;

    while let Some(request) = read_jsonl(&mut reader)? {
        let command = match &request.body {
            SessionBody::Command(command) => command.clone(),
            _ => continue,
        };
        let response = match command {
            SessionCommand::LoadLevel { path } => {
                let level_path = PathBuf::from(path);
                match load_level_json(&level_path) {
                    Ok(level) => match compile_level(&level, &BehaviorLibrary::new()) {
                        Ok(compiled) => {
                            active_level = Some(level.scene.id.to_string());
                            world = Some(compiled);
                            SessionResponse {
                                ok: true,
                                summary: format!("loaded {}", level.scene.id),
                                data: None,
                            }
                        }
                        Err(error) => SessionResponse {
                            ok: false,
                            summary: error.to_string(),
                            data: None,
                        },
                    },
                    Err(error) => SessionResponse {
                        ok: false,
                        summary: error.to_string(),
                        data: None,
                    },
                }
            }
            SessionCommand::Play | SessionCommand::Resume => {
                playing = true;
                SessionResponse {
                    ok: true,
                    summary: "playing".into(),
                    data: None,
                }
            }
            SessionCommand::Pause => {
                playing = false;
                SessionResponse {
                    ok: true,
                    summary: "paused".into(),
                    data: None,
                }
            }
            SessionCommand::Step { ticks } => {
                let mut completed = 0u32;
                if let Some(world) = world.as_mut() {
                    for _ in 0..ticks.max(1) {
                        world.tick_once()?;
                        completed += 1;
                    }
                }
                SessionResponse {
                    ok: true,
                    summary: format!("stepped {completed} tick(s)"),
                    data: None,
                }
            }
            SessionCommand::Stop => {
                playing = false;
                SessionResponse {
                    ok: true,
                    summary: "stopped".into(),
                    data: None,
                }
            }
            SessionCommand::Snapshot => {
                let data = json!({
                    "ready": true,
                    "playing": playing,
                    "active_level": active_level.clone(),
                    "entity_count": world.as_ref().map(|world| world.entities.len()).unwrap_or(0),
                    "semantic_cell_count": world.as_ref().map(|world| world.semantic_cells.len()).unwrap_or(0)
                });
                SessionResponse {
                    ok: true,
                    summary: "snapshot".into(),
                    data: Some(data),
                }
            }
            SessionCommand::ReloadDocument { id, revision } => SessionResponse {
                ok: false,
                summary: format!(
                    "hot reload for document {id} revision {revision} is not certified yet"
                ),
                data: None,
            },
            SessionCommand::ReloadAsset { id, revision } => SessionResponse {
                ok: false,
                summary: format!(
                    "hot reload for asset {id} revision {revision} is not certified yet"
                ),
                data: None,
            },
            SessionCommand::Capture { id, output_path } => SessionResponse {
                ok: false,
                summary: format!("capture {id} to {output_path} requires the render backend"),
                data: None,
            },
            SessionCommand::Shutdown => {
                let envelope = response_envelope(
                    &request,
                    SessionResponse {
                        ok: true,
                        summary: "shutdown".into(),
                        data: None,
                    },
                );
                write_jsonl(&mut writer, &envelope)?;
                break;
            }
        };
        let envelope = response_envelope(&request, response);
        write_jsonl(&mut writer, &envelope)?;
    }
    Ok(())
}

fn response_envelope(request: &SessionEnvelope, response: SessionResponse) -> SessionEnvelope {
    SessionEnvelope {
        protocol: SESSION_PROTOCOL.into(),
        protocol_version: SESSION_PROTOCOL_VERSION,
        session_id: request.session_id.clone(),
        message_id: request.message_id.clone(),
        body: SessionBody::Response(response),
    }
}
