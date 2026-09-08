//! File-backed runtime bridge contract used before the native IPC transport lands.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RuntimeStateSnapshot {
    pub schema_version: u32,
    pub runtime: String,
    pub project: Option<String>,
    pub active_level: Option<String>,
    pub entity_count: usize,
    pub semantic_cell_count: usize,
    pub camera: Option<CameraState>,
    pub player: Option<PlayerState>,
    pub diagnostics: Vec<String>,
    pub ready: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CameraState {
    pub position: [f32; 3],
    pub zoom: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerState {
    pub entity_id: String,
    pub position: [f32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaptureRequest {
    pub id: String,
    pub kind: CaptureKind,
    pub output_path: PathBuf,
    pub include_debug_overlay: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureKind {
    Viewport,
    Window,
    Region {
        x: i32,
        y: i32,
        width: u32,
        height: u32,
    },
}

pub fn write_snapshot(session_dir: &Path, snapshot: &RuntimeStateSnapshot) -> Result<(), String> {
    fs::create_dir_all(session_dir).map_err(|e| e.to_string())?;
    let path = session_dir.join("runtime_state.json");
    let bytes = serde_json::to_vec_pretty(snapshot).map_err(|e| e.to_string())?;
    fs::write(path, bytes).map_err(|e| e.to_string())
}

pub fn read_snapshot(session_dir: &Path) -> Result<RuntimeStateSnapshot, String> {
    let bytes = fs::read(session_dir.join("runtime_state.json")).map_err(|e| e.to_string())?;
    serde_json::from_slice(&bytes).map_err(|e| e.to_string())
}

pub fn queue_capture(session_dir: &Path, request: &CaptureRequest) -> Result<PathBuf, String> {
    let dir = session_dir.join("capture_requests");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join(format!("{}.json", request.id));
    fs::write(
        &path,
        serde_json::to_vec_pretty(request).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(path)
}
