use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEV_BRIDGE_SCHEMA: &str = "havenwild.dev_bridge.v0_1";
pub const DEV_BRIDGE_ROOT_RELATIVE: &str = "WORKSPACE/dev_bridge";
pub const DEV_BRIDGE_STATUS_STALE_MS: u64 = 4_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DevBridgeCommand {
    ReloadAssets { reason: String },
    ReloadEditorWorld { reason: String },
    ReloadAssetsAndWorld { reason: String },
    SetRevealAllMap { enabled: bool },
    QuitClient,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevBridgeCommandEnvelope {
    pub schema: String,
    pub sequence: u64,
    pub issued_unix_ms: u64,
    pub command: DevBridgeCommand,
}

impl DevBridgeCommandEnvelope {
    pub fn new(sequence: u64, command: DevBridgeCommand) -> Self {
        Self {
            schema: DEV_BRIDGE_SCHEMA.to_string(),
            sequence,
            issued_unix_ms: unix_time_ms(),
            command,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DevBridgeClientStatus {
    pub schema: String,
    pub process_id: u32,
    pub started_unix_ms: u64,
    pub heartbeat_unix_ms: u64,
    pub phase: String,
    pub world_id: Option<String>,
    pub character_id: Option<String>,
    pub active_scene: Option<String>,
    pub player_tile: Option<[i32; 2]>,
    pub last_command_sequence: u64,
    pub reveal_all_map: bool,
    pub message: String,
}

impl DevBridgeClientStatus {
    pub fn is_fresh(&self, now_ms: u64) -> bool {
        self.schema == DEV_BRIDGE_SCHEMA
            && now_ms.saturating_sub(self.heartbeat_unix_ms) <= DEV_BRIDGE_STATUS_STALE_MS
    }
}

pub fn bridge_root(repo_root: &Path) -> PathBuf {
    repo_root.join(DEV_BRIDGE_ROOT_RELATIVE)
}

pub fn commands_dir(repo_root: &Path) -> PathBuf {
    bridge_root(repo_root).join("commands")
}

pub fn client_status_path(repo_root: &Path) -> PathBuf {
    bridge_root(repo_root).join("client_status.json")
}

pub fn queue_dev_bridge_command(
    repo_root: &Path,
    envelope: &DevBridgeCommandEnvelope,
) -> Result<PathBuf, String> {
    let dir = commands_dir(repo_root);
    fs::create_dir_all(&dir).map_err(|error| format!("create dev bridge command dir: {error}"))?;
    let target = dir.join(format!("{:020}.json", envelope.sequence));
    write_json_atomic(&target, envelope)?;
    Ok(target)
}

pub fn pending_dev_bridge_commands(
    repo_root: &Path,
    after_sequence: u64,
) -> Result<Vec<DevBridgeCommandEnvelope>, String> {
    let dir = commands_dir(repo_root);
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut commands = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|error| format!("read dev bridge commands: {error}"))? {
        let entry = entry.map_err(|error| format!("read dev bridge command entry: {error}"))?;
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let bytes = fs::read(&path)
            .map_err(|error| format!("read dev bridge command {}: {error}", path.display()))?;
        let envelope: DevBridgeCommandEnvelope = serde_json::from_slice(&bytes)
            .map_err(|error| format!("parse dev bridge command {}: {error}", path.display()))?;
        if envelope.schema == DEV_BRIDGE_SCHEMA && envelope.sequence > after_sequence {
            commands.push(envelope);
        }
    }
    commands.sort_by_key(|command| command.sequence);
    Ok(commands)
}

pub fn prune_dev_bridge_commands(repo_root: &Path, through_sequence: u64) -> Result<usize, String> {
    let dir = commands_dir(repo_root);
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut removed = 0usize;
    for entry in fs::read_dir(&dir).map_err(|error| format!("read dev bridge commands: {error}"))? {
        let entry = entry.map_err(|error| format!("read dev bridge command entry: {error}"))?;
        let path = entry.path();
        let sequence = path
            .file_stem()
            .and_then(|value| value.to_str())
            .and_then(|value| value.parse::<u64>().ok());
        if sequence.is_some_and(|value| value <= through_sequence) && path.is_file() {
            fs::remove_file(&path)
                .map_err(|error| format!("remove dev bridge command {}: {error}", path.display()))?;
            removed += 1;
        }
    }
    Ok(removed)
}

pub fn write_dev_bridge_status(repo_root: &Path, status: &DevBridgeClientStatus) -> Result<(), String> {
    let path = client_status_path(repo_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create dev bridge root: {error}"))?;
    }
    write_json_atomic(&path, status)
}

pub fn read_dev_bridge_status(repo_root: &Path) -> Result<Option<DevBridgeClientStatus>, String> {
    let path = client_status_path(repo_root);
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(&path)
        .map_err(|error| format!("read dev bridge status {}: {error}", path.display()))?;
    let status = serde_json::from_slice::<DevBridgeClientStatus>(&bytes)
        .map_err(|error| format!("parse dev bridge status {}: {error}", path.display()))?;
    Ok((status.schema == DEV_BRIDGE_SCHEMA).then_some(status))
}

pub fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("serialize {}: {error}", path.display()))?;
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, bytes)
        .map_err(|error| format!("write temporary {}: {error}", temp.display()))?;
    if path.exists() {
        fs::remove_file(path)
            .map_err(|error| format!("replace existing {}: {error}", path.display()))?;
    }
    fs::rename(&temp, path)
        .map_err(|error| format!("commit {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_freshness_uses_bounded_heartbeat_age() {
        let status = DevBridgeClientStatus {
            schema: DEV_BRIDGE_SCHEMA.to_string(),
            process_id: 7,
            started_unix_ms: 100,
            heartbeat_unix_ms: 1_000,
            phase: "gameplay".to_string(),
            world_id: None,
            character_id: None,
            active_scene: None,
            player_tile: None,
            last_command_sequence: 0,
            reveal_all_map: false,
            message: String::new(),
        };
        assert!(status.is_fresh(1_000 + DEV_BRIDGE_STATUS_STALE_MS));
        assert!(!status.is_fresh(1_001 + DEV_BRIDGE_STATUS_STALE_MS));
    }
}
