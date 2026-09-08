use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::Path;

use haven_assets::asset_registry::GENERATED_WORLDGEN_MANIFEST_PATH;
use haven_core::GameWorld;

pub const PROJECT_FILE_SCHEMA_VERSION: &str = "v009";
pub const STARTER_PROJECT_FILE_PATH: &str = "content/editor/project_file_starter_v0_9.json";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditorProjectFile {
    pub project_id: String,
    pub name: String,
    pub schema_version: String,
    pub scenes: Vec<String>,
    pub assets: Vec<String>,
    pub databases: Vec<String>,
    pub editor_layout: Option<serde_json::Value>,
    pub runtime_config: Option<serde_json::Value>,
}

impl EditorProjectFile {
    pub fn load_from_path(path: &str) -> Result<Self, String> {
        let raw =
            read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
        serde_json::from_str(&raw).map_err(|error| format!("failed to parse {path}: {error}"))
    }

    pub fn save_to_path(&self, path: &str) -> Result<(), String> {
        let data = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to serialize project file: {error}"))?;
        if let Some(parent) = Path::new(path).parent() {
            create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        write(path, data).map_err(|error| format!("failed to write {path}: {error}"))
    }

    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.project_id.trim().is_empty() {
            warnings.push("project file has no project_id".to_string());
        }
        if self.name.trim().is_empty() {
            warnings.push("project file has no name".to_string());
        }
        if self.scenes.is_empty() {
            warnings.push("project file does not list any scenes".to_string());
        }
        if self.assets.is_empty() {
            warnings.push("project file does not reference any asset manifests".to_string());
        }
        if self.databases.is_empty() {
            warnings.push("project file does not list any database/content roots".to_string());
        }
        warnings
    }

    pub fn summary_lines(&self) -> Vec<String> {
        vec![
            format!("Project: {}", self.name),
            format!("Project ID: {}", self.project_id),
            format!("Scenes: {}", self.scenes.len()),
            format!("Asset refs: {}", self.assets.len()),
            format!("Content roots: {}", self.databases.len()),
        ]
    }
}

pub fn load_editor_project_file_from_path(path: &str) -> Result<EditorProjectFile, String> {
    EditorProjectFile::load_from_path(path)
}

pub fn starter_project_file(world: &GameWorld) -> EditorProjectFile {
    EditorProjectFile {
        project_id: "havenwild_starter".to_string(),
        name: "Havenwild Starter Island".to_string(),
        schema_version: PROJECT_FILE_SCHEMA_VERSION.to_string(),
        scenes: world
            .scenes
            .iter()
            .map(|scene| scene.id.code().to_string())
            .collect(),
        assets: vec![
            GENERATED_WORLDGEN_MANIFEST_PATH.to_string(),
            "content/animations/character_animation_contract_v0_10.json".to_string(),
        ],
        databases: vec![
            "content/packs".to_string(),
            "content/worldgen".to_string(),
            "content/editor".to_string(),
            "content/schemas".to_string(),
        ],
        editor_layout: None,
        runtime_config: Some(json!({
            "defaultWorldgenPack": "content/worldgen/packs/worldgen_home_island_v0_10.json",
            "worldSavePath": "workspace/saves/world.tworld",
            "layoutSavePath": "workspace/saves/editor_layout.tlayout"
        })),
    }
}
