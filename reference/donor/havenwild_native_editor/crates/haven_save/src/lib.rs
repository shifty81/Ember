pub mod character_progression;
pub use character_progression::*;
pub mod character_vitals;
pub use character_vitals::*;
pub mod character_profiles;
pub use character_profiles::*;
pub mod chunk_persistence;
pub use chunk_persistence::*;
pub mod building_persistence;
pub use building_persistence::*;
pub mod persistence_contract;
pub use persistence_contract::*;
pub mod surface_chunk_storage;
pub use surface_chunk_storage::*;
pub mod social_world;
use haven_core::{GameWorld, UiLayoutState};
use serde::{Deserialize, Serialize};
pub use social_world::*;
use std::fs::{create_dir_all, read_to_string, remove_dir_all, rename};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const ARCHITECTURE_STATUS: &str =
    "Active save crate for client-slot ownership, world/layout persistence, and future typed migrations.";
pub const LEGACY_CLIENT_SAVE_SLOT_COUNT: usize = 3;
pub const CLIENT_SAVE_SLOT_COUNT: usize = LEGACY_CLIENT_SAVE_SLOT_COUNT;
pub const CLIENT_SAVE_METADATA_SCHEMA: &str = "havenwild.client_save_slot.v0_1";
pub const CURRENT_CLIENT_GENERATION_VERSION: u32 = 16;

pub fn save_pipeline_note() -> &'static str {
    "Legacy three-slot saves remain readable. New character profiles are independent from dynamically discovered, unlimited world saves."
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientSaveSlot {
    Slot1,
    Slot2,
    Slot3,
}

impl ClientSaveSlot {
    pub const ALL: [Self; CLIENT_SAVE_SLOT_COUNT] = [Self::Slot1, Self::Slot2, Self::Slot3];

    pub const fn index(self) -> u8 {
        match self {
            Self::Slot1 => 1,
            Self::Slot2 => 2,
            Self::Slot3 => 3,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Slot1 => "Save Slot 1",
            Self::Slot2 => "Save Slot 2",
            Self::Slot3 => "Save Slot 3",
        }
    }

    pub const fn directory_name(self) -> &'static str {
        match self {
            Self::Slot1 => "slot_1",
            Self::Slot2 => "slot_2",
            Self::Slot3 => "slot_3",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientSaveMetadata {
    pub schema: String,
    pub slot: ClientSaveSlot,
    pub display_name: String,
    pub world_seed: u64,
    pub archipelago_seed: u64,
    pub generation_version: u32,
    pub generated_scene_count: usize,
    #[serde(default)]
    pub generated_exterior_scene_count: usize,
    #[serde(default)]
    pub generated_island_count: usize,
    #[serde(default)]
    pub starting_scene_code: String,
    #[serde(default = "default_world_width_tiles")]
    pub world_width_tiles: i32,
    #[serde(default = "default_world_height_tiles")]
    pub world_height_tiles: i32,
    #[serde(default = "default_world_chunk_size_tiles")]
    pub world_chunk_size_tiles: i32,
    #[serde(default = "default_true")]
    pub world_wrap_east_west: bool,
    pub created_unix_seconds: u64,
    pub last_played_unix_seconds: u64,
}

impl ClientSaveMetadata {
    pub fn new(slot: ClientSaveSlot, world_seed: u64, generated_scene_count: usize) -> Self {
        let now = unix_seconds_now();
        Self {
            schema: CLIENT_SAVE_METADATA_SCHEMA.to_string(),
            slot,
            display_name: format!("Havenwild {}", slot.index()),
            world_seed,
            archipelago_seed: world_seed,
            generation_version: CURRENT_CLIENT_GENERATION_VERSION,
            generated_scene_count,
            generated_exterior_scene_count: 0,
            generated_island_count: 0,
            starting_scene_code: String::new(),
            world_width_tiles: haven_world::open_world::DEFAULT_OVERWORLD_SIZE_TILES,
            world_height_tiles: haven_world::open_world::DEFAULT_OVERWORLD_SIZE_TILES,
            world_chunk_size_tiles: haven_world::open_world::DEFAULT_OVERWORLD_CHUNK_SIZE_TILES,
            world_wrap_east_west: true,
            created_unix_seconds: now,
            last_played_unix_seconds: now,
        }
    }

    pub fn domain_versions(&self) -> PersistenceDomainVersions {
        PersistenceDomainVersions::from_legacy_generation_version(self.generation_version)
    }

    pub fn touch_last_played(&mut self) {
        self.last_played_unix_seconds = unix_seconds_now();
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CLIENT_SAVE_METADATA_SCHEMA {
            return Err(format!("unsupported save metadata schema {}", self.schema));
        }
        if self.world_seed == 0 || self.archipelago_seed == 0 {
            return Err("save slot seed must be non-zero".to_string());
        }
        if self.generation_version == 0 {
            return Err("save slot generation version must be positive".to_string());
        }
        if self.world_width_tiles <= 0
            || self.world_height_tiles <= 0
            || self.world_chunk_size_tiles <= 0
        {
            return Err("world topology dimensions and chunk size must be positive".to_string());
        }
        if self.world_width_tiles % self.world_chunk_size_tiles != 0 {
            return Err(
                "world width must be divisible by chunk size for seam-stable wrapping".to_string(),
            );
        }
        if self.generated_scene_count == 0 {
            return Err("save slot generated scene count must be positive".to_string());
        }
        if self.generated_exterior_scene_count > self.generated_scene_count {
            return Err("exterior scene count cannot exceed total scene count".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSaveMetadata {
    pub schema: String,
    pub world_id: WorldSaveId,
    pub display_name: String,
    pub world_seed: u64,
    pub archipelago_seed: u64,
    pub generation_version: u32,
    pub generated_scene_count: usize,
    #[serde(default)]
    pub generated_exterior_scene_count: usize,
    #[serde(default)]
    pub generated_island_count: usize,
    #[serde(default)]
    pub starting_scene_code: String,
    #[serde(default = "default_world_width_tiles")]
    pub world_width_tiles: i32,
    #[serde(default = "default_world_height_tiles")]
    pub world_height_tiles: i32,
    #[serde(default = "default_world_chunk_size_tiles")]
    pub world_chunk_size_tiles: i32,
    #[serde(default = "default_true")]
    pub world_wrap_east_west: bool,
    pub created_unix_seconds: u64,
    pub last_played_unix_seconds: u64,
}

pub const WORLD_SAVE_METADATA_SCHEMA: &str = "havenwild.world_save.v1";

impl WorldSaveMetadata {
    pub fn new(
        world_id: WorldSaveId,
        display_name: impl Into<String>,
        world_seed: u64,
        generated_scene_count: usize,
    ) -> Self {
        let now = unix_seconds_now();
        Self {
            schema: WORLD_SAVE_METADATA_SCHEMA.to_string(),
            world_id,
            display_name: display_name.into(),
            world_seed,
            archipelago_seed: world_seed,
            generation_version: CURRENT_CLIENT_GENERATION_VERSION,
            generated_scene_count,
            generated_exterior_scene_count: 0,
            generated_island_count: 0,
            starting_scene_code: String::new(),
            world_width_tiles: haven_world::open_world::DEFAULT_OVERWORLD_SIZE_TILES,
            world_height_tiles: haven_world::open_world::DEFAULT_OVERWORLD_SIZE_TILES,
            world_chunk_size_tiles: haven_world::open_world::DEFAULT_OVERWORLD_CHUNK_SIZE_TILES,
            world_wrap_east_west: true,
            created_unix_seconds: now,
            last_played_unix_seconds: now,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != WORLD_SAVE_METADATA_SCHEMA {
            return Err(format!("unsupported world metadata schema {}", self.schema));
        }
        self.world_id.validate()?;
        if self.display_name.trim().is_empty() {
            return Err("world display name must not be empty".to_string());
        }
        if self.world_seed == 0 || self.archipelago_seed == 0 {
            return Err("world seed must be non-zero".to_string());
        }
        if self.generated_scene_count == 0 {
            return Err("world generated scene count must be positive".to_string());
        }
        Ok(())
    }

    pub fn domain_versions(&self) -> PersistenceDomainVersions {
        PersistenceDomainVersions::from_legacy_generation_version(self.generation_version)
    }

    pub fn touch_last_played(&mut self) {
        self.last_played_unix_seconds = unix_seconds_now();
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientSavePaths {
    pub root: String,
    pub metadata: String,
    pub world: String,
    pub scene_manifest: String,
    pub world_creation_settings: String,
    pub world_social_state: String,
    pub character_links: String,
    pub world_paint_delta: String,
    pub world_paint_material_state: String,
    pub world_paint_render_cache: String,
    pub world_topology: String,
    pub chunk_manifest: String,
    pub chunk_migrations: String,
    pub chunks_root: String,
    pub recovery_root: String,
    pub previews: String,
}

impl ClientSavePaths {
    pub fn for_world_id(base_root: &str, world_id: &WorldSaveId) -> Result<Self, String> {
        world_id.validate()?;
        Ok(Self::for_root(
            Path::new(base_root).join(&world_id.0),
            "world.json",
        ))
    }

    pub fn for_slot(base_root: &str, slot: ClientSaveSlot) -> Self {
        Self::for_root(
            Path::new(base_root).join(slot.directory_name()),
            "slot.json",
        )
    }

    fn for_root(root: PathBuf, metadata_name: &str) -> Self {
        Self {
            metadata: path_string(root.join(metadata_name)),
            world: path_string(root.join("world.tworld")),
            scene_manifest: path_string(root.join("worldgen/scene_rectangle_manifest.json")),
            world_creation_settings: path_string(
                root.join(haven_world::WORLD_CREATION_SETTINGS_FILENAME),
            ),
            world_social_state: path_string(root.join(WORLD_SOCIAL_STATE_RELATIVE_PATH)),
            character_links: path_string(root.join("characters")),
            world_paint_delta: path_string(root.join("world_paint/world_paint_deltas.json")),
            world_paint_material_state: path_string(
                root.join("world_paint/world_paint_material_state.json"),
            ),
            world_paint_render_cache: path_string(
                root.join("world_paint/world_paint_render_cache.json"),
            ),
            world_topology: path_string(root.join("worldgen/world_topology.json")),
            chunk_manifest: path_string(root.join("world/chunk_manifest.json")),
            chunk_migrations: path_string(root.join("world/migrations.json")),
            chunks_root: path_string(root.join("world/chunks")),
            recovery_root: path_string(root.join("recovery")),
            previews: path_string(root.join("worldgen/previews")),
            root: path_string(root),
        }
    }

    pub fn ensure_directories(&self) -> Result<(), String> {
        create_dir_all(&self.root).map_err(|error| error.to_string())?;
        ensure_parent_dir(&self.metadata)?;
        ensure_parent_dir(&self.world)?;
        ensure_parent_dir(&self.scene_manifest)?;
        ensure_parent_dir(&self.world_creation_settings)?;
        ensure_parent_dir(&self.world_social_state)?;
        create_dir_all(&self.character_links).map_err(|error| error.to_string())?;
        ensure_parent_dir(&self.world_paint_delta)?;
        ensure_parent_dir(&self.world_paint_material_state)?;
        ensure_parent_dir(&self.world_paint_render_cache)?;
        ensure_parent_dir(&self.world_topology)?;
        ensure_parent_dir(&self.chunk_manifest)?;
        ensure_parent_dir(&self.chunk_migrations)?;
        create_dir_all(&self.chunks_root).map_err(|error| error.to_string())?;
        create_dir_all(&self.recovery_root).map_err(|error| error.to_string())?;
        create_dir_all(&self.previews).map_err(|error| error.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct ClientSaveSlotSummary {
    pub slot: ClientSaveSlot,
    pub paths: ClientSavePaths,
    pub metadata: Option<ClientSaveMetadata>,
    pub world_exists: bool,
    pub error: Option<String>,
}

impl ClientSaveSlotSummary {
    pub fn occupied(&self) -> bool {
        self.metadata.is_some() && self.world_exists && self.error.is_none()
    }
}

pub fn client_save_paths(base_root: &str, slot: ClientSaveSlot) -> ClientSavePaths {
    ClientSavePaths::for_slot(base_root, slot)
}

pub fn migrate_legacy_client_save_slots(base_root: &str) -> Result<Vec<WorldSaveId>, String> {
    let mut migrated = Vec::new();
    for slot in ClientSaveSlot::ALL {
        let legacy = ClientSavePaths::for_slot(base_root, slot);
        if !Path::new(&legacy.root).is_dir() {
            continue;
        }
        let world_id = WorldSaveId(format!("world_legacy_slot_{}", slot.index()));
        let target = ClientSavePaths::for_world_id(base_root, &world_id)?;
        if !Path::new(&target.root).exists() {
            rename(&legacy.root, &target.root).map_err(|error| error.to_string())?;
        }
        let old_metadata_path = Path::new(&target.root).join("slot.json");
        if old_metadata_path.is_file() && !Path::new(&target.metadata).is_file() {
            let legacy_metadata = load_client_save_metadata(&path_string(old_metadata_path))?;
            let mut metadata = WorldSaveMetadata::new(
                world_id.clone(),
                legacy_metadata.display_name,
                legacy_metadata.world_seed,
                legacy_metadata.generated_scene_count,
            );
            metadata.archipelago_seed = legacy_metadata.archipelago_seed;
            metadata.generation_version = legacy_metadata.generation_version;
            metadata.generated_exterior_scene_count =
                legacy_metadata.generated_exterior_scene_count;
            metadata.generated_island_count = legacy_metadata.generated_island_count;
            metadata.starting_scene_code = legacy_metadata.starting_scene_code;
            metadata.world_width_tiles = legacy_metadata.world_width_tiles;
            metadata.world_height_tiles = legacy_metadata.world_height_tiles;
            metadata.world_chunk_size_tiles = legacy_metadata.world_chunk_size_tiles;
            metadata.world_wrap_east_west = legacy_metadata.world_wrap_east_west;
            metadata.created_unix_seconds = legacy_metadata.created_unix_seconds;
            metadata.last_played_unix_seconds = legacy_metadata.last_played_unix_seconds;
            save_world_save_metadata(&target.metadata, &metadata)?;
        }
        migrated.push(world_id);
    }
    Ok(migrated)
}

pub fn scan_client_save_slots(base_root: &str) -> Vec<ClientSaveSlotSummary> {
    ClientSaveSlot::ALL
        .iter()
        .copied()
        .map(|slot| {
            let paths = ClientSavePaths::for_slot(base_root, slot);
            let world_exists = Path::new(&paths.world).is_file();
            match load_client_save_metadata(&paths.metadata) {
                Ok(metadata) => ClientSaveSlotSummary {
                    slot,
                    paths,
                    metadata: Some(metadata),
                    world_exists,
                    error: if world_exists {
                        None
                    } else {
                        Some("metadata exists but world.tworld is missing".to_string())
                    },
                },
                Err(error) if Path::new(&paths.metadata).exists() => ClientSaveSlotSummary {
                    slot,
                    paths,
                    metadata: None,
                    world_exists,
                    error: Some(error),
                },
                Err(_) => ClientSaveSlotSummary {
                    slot,
                    paths,
                    metadata: None,
                    world_exists,
                    error: if world_exists {
                        Some("world exists but slot.json is missing".to_string())
                    } else {
                        None
                    },
                },
            }
        })
        .collect()
}

pub fn world_save_paths(
    base_root: &str,
    world_id: &WorldSaveId,
) -> Result<ClientSavePaths, String> {
    ClientSavePaths::for_world_id(base_root, world_id)
}

#[derive(Clone, Debug)]
pub struct WorldSaveSummary {
    pub world_id: WorldSaveId,
    pub paths: ClientSavePaths,
    pub metadata: Option<WorldSaveMetadata>,
    pub world_exists: bool,
    pub error: Option<String>,
}

impl WorldSaveSummary {
    pub fn occupied(&self) -> bool {
        self.metadata.is_some() && self.world_exists && self.error.is_none()
    }
}

pub fn scan_world_saves(base_root: &str) -> Result<Vec<WorldSaveSummary>, String> {
    let ids = scan_unlimited_world_saves(Path::new(base_root))?;
    let mut summaries = Vec::new();
    for world_id in ids {
        let paths = world_save_paths(base_root, &world_id)?;
        let world_exists = Path::new(&paths.world).is_file();
        let metadata = load_world_save_metadata(&paths.metadata);
        summaries.push(match metadata {
            Ok(metadata) => WorldSaveSummary {
                world_id,
                paths,
                metadata: Some(metadata),
                world_exists,
                error: if world_exists {
                    None
                } else {
                    Some("metadata exists but world.tworld is missing".to_string())
                },
            },
            Err(error) => WorldSaveSummary {
                world_id,
                paths,
                metadata: None,
                world_exists,
                error: Some(error),
            },
        });
    }
    summaries.sort_by(|left, right| {
        right
            .metadata
            .as_ref()
            .map(|value| value.last_played_unix_seconds)
            .unwrap_or_default()
            .cmp(
                &left
                    .metadata
                    .as_ref()
                    .map(|value| value.last_played_unix_seconds)
                    .unwrap_or_default(),
            )
            .then_with(|| left.world_id.cmp(&right.world_id))
    });
    Ok(summaries)
}

pub fn load_world_save_metadata(path: &str) -> Result<WorldSaveMetadata, String> {
    let raw = read_to_string(path).map_err(|error| error.to_string())?;
    let metadata: WorldSaveMetadata =
        serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    metadata.validate()?;
    Ok(metadata)
}

pub fn save_world_save_metadata(path: &str, metadata: &WorldSaveMetadata) -> Result<(), String> {
    metadata.validate()?;
    ensure_parent_dir(path)?;
    let raw = serde_json::to_string_pretty(metadata).map_err(|error| error.to_string())?;
    atomic_write(path, raw.as_bytes())
}

pub fn touch_world_save_metadata(path: &str) -> Result<(), String> {
    let mut metadata = load_world_save_metadata(path)?;
    metadata.touch_last_played();
    save_world_save_metadata(path, &metadata)
}

pub fn delete_world_save(base_root: &str, world_id: &WorldSaveId) -> Result<(), String> {
    let paths = world_save_paths(base_root, world_id)?;
    if Path::new(&paths.root).exists() {
        remove_dir_all(&paths.root).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn load_client_save_metadata(path: &str) -> Result<ClientSaveMetadata, String> {
    let raw = read_to_string(path).map_err(|error| error.to_string())?;
    let metadata: ClientSaveMetadata =
        serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    metadata.validate()?;
    Ok(metadata)
}

pub fn save_client_save_metadata(path: &str, metadata: &ClientSaveMetadata) -> Result<(), String> {
    metadata.validate()?;
    ensure_parent_dir(path)?;
    let raw = serde_json::to_string_pretty(metadata).map_err(|error| error.to_string())?;
    atomic_write(path, raw.as_bytes())
}

pub fn touch_client_save_metadata(path: &str) -> Result<ClientSaveMetadata, String> {
    let mut metadata = load_client_save_metadata(path)?;
    metadata.touch_last_played();
    save_client_save_metadata(path, &metadata)?;
    Ok(metadata)
}

pub fn delete_client_save_slot(base_root: &str, slot: ClientSaveSlot) -> Result<(), String> {
    let paths = ClientSavePaths::for_slot(base_root, slot);
    let root = Path::new(&paths.root);
    if root.exists() {
        remove_dir_all(root).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn load_world_from_path(path: &str) -> Result<GameWorld, String> {
    read_to_string(path)
        .map_err(|error| error.to_string())
        .and_then(|data| GameWorld::deserialize_lines(&data))
}

pub fn save_world_to_path(path: &str, world: &GameWorld) -> Result<(), String> {
    ensure_parent_dir(path)?;
    atomic_write(path, world.serialize_lines().as_bytes())
}

pub fn save_world_topology_to_path(
    path: &str,
    topology: &haven_world::WorldTopologyConfig,
) -> Result<(), String> {
    topology.validate()?;
    ensure_parent_dir(path)?;
    let raw = serde_json::to_string_pretty(topology).map_err(|error| error.to_string())?;
    atomic_write(path, raw.as_bytes())
}

pub fn load_world_topology_from_path(
    path: &str,
) -> Result<haven_world::WorldTopologyConfig, String> {
    let raw = read_to_string(path).map_err(|error| error.to_string())?;
    let topology: haven_world::WorldTopologyConfig =
        serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    topology.validate()?;
    Ok(topology)
}

pub fn load_ui_layout_from_path(path: &str) -> Result<UiLayoutState, String> {
    read_to_string(path)
        .map_err(|error| error.to_string())
        .and_then(|data| UiLayoutState::deserialize_lines(&data))
}

pub fn save_ui_layout_to_path(path: &str, layout: &UiLayoutState) -> Result<(), String> {
    ensure_parent_dir(path)?;
    atomic_write(path, layout.serialize_lines().as_bytes())
}

pub fn save_chunk_manifest_to_path(path: &str, manifest: &ChunkManifest) -> Result<(), String> {
    manifest.validate()?;
    ensure_parent_dir(path)?;
    let raw = serde_json::to_vec_pretty(manifest).map_err(|e| e.to_string())?;
    atomic_write(path, &raw)
}
pub fn load_chunk_manifest_from_path(path: &str) -> Result<ChunkManifest, String> {
    let raw = read_to_string(path).map_err(|e| e.to_string())?;
    let value: ChunkManifest = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    value.validate()?;
    Ok(value)
}
pub fn atomic_write(path: &str, bytes: &[u8]) -> Result<(), String> {
    ensure_parent_dir(path)?;
    let target = Path::new(path);
    let temp = target.with_extension("tmp");
    let recovery = target.with_extension("recovery");
    if target.exists() {
        std::fs::copy(target, &recovery).map_err(|e| e.to_string())?;
    }
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, target).map_err(|e| e.to_string())
}

fn default_world_width_tiles() -> i32 {
    haven_world::open_world::DEFAULT_OVERWORLD_SIZE_TILES
}

fn default_world_height_tiles() -> i32 {
    haven_world::open_world::DEFAULT_OVERWORLD_SIZE_TILES
}

fn default_world_chunk_size_tiles() -> i32 {
    haven_world::open_world::DEFAULT_OVERWORLD_CHUNK_SIZE_TILES
}

fn default_true() -> bool {
    true
}

fn ensure_parent_dir(path: &str) -> Result<(), String> {
    let Some(parent) = Path::new(path).parent() else {
        return Ok(());
    };
    create_dir_all(parent).map_err(|error| error.to_string())
}

fn path_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn unix_seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(1)
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_slots_are_three_distinct_directories() {
        let paths: Vec<ClientSavePaths> = ClientSaveSlot::ALL
            .iter()
            .copied()
            .map(|slot| ClientSavePaths::for_slot("WORKSPACE/saves", slot))
            .collect();
        assert_eq!(paths.len(), 3);
        assert!(paths[0].root.ends_with("slot_1"));
        assert!(paths[1].root.ends_with("slot_2"));
        assert!(paths[2].root.ends_with("slot_3"));
        assert_ne!(paths[0].world, paths[1].world);
        assert_ne!(paths[1].world_paint_delta, paths[2].world_paint_delta);
        assert!(paths[0]
            .world_topology
            .ends_with("worldgen/world_topology.json"));
    }

    #[test]
    fn metadata_requires_a_nonzero_seed() {
        let mut metadata = ClientSaveMetadata::new(ClientSaveSlot::Slot1, 42, 52);
        assert!(metadata.validate().is_ok());
        metadata.world_seed = 0;
        assert!(metadata.validate().is_err());
    }
    #[test]
    fn metadata_defaults_to_wrapped_finite_world_topology() {
        let metadata = ClientSaveMetadata::new(ClientSaveSlot::Slot1, 42, 52);
        assert!(metadata.world_wrap_east_west);
        assert_eq!(
            metadata.world_width_tiles % metadata.world_chunk_size_tiles,
            0
        );
        assert!(metadata.validate().is_ok());
    }
}
