//! Atomic persistence transactions for complete generated surface-world bakes.
//!
//! Version 2 persists hydrology and structural terrain in one transaction so
//! runtime, editor, collision, cave placement, and rendering consume the same
//! derived world state. Version 1 remains available as a compatibility wrapper.

use haven_core::{
    GameWorld, SceneBiome, SceneKind, SceneMap, SceneReference, SceneRegistry, TavernMap,
};
use haven_world::{
    bake_full_world_hydrology_v2, bake_full_world_structural_terrain_v2, ElevationCliffSettingsV2,
    FullWorldHydrologyBakeReportV2, FullWorldStructuralBakeV2, HydrologySettingsV2,
    WorldManifestV2,
};
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, remove_dir_all, rename};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::atomic_write;

pub const WORLD_BAKE_TRANSACTION_SCHEMA: &str = "havenwild.world_bake_transaction.v1";
pub const WORLD_BAKE_TRANSACTION_V2_SCHEMA: &str = "havenwild.world_bake_transaction.v2";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldBakeTransactionReportV1 {
    pub schema: String,
    pub world_seed: u64,
    pub width_chunks: u32,
    pub height_chunks: u32,
    pub chunk_count: usize,
    pub hydrology: FullWorldHydrologyBakeReportV2,
    pub output_root: String,
    pub recovery_backup: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldBakeTransactionReportV2 {
    pub schema: String,
    pub world_seed: u64,
    pub width_chunks: u32,
    pub height_chunks: u32,
    pub chunk_count: usize,
    pub hydrology: FullWorldHydrologyBakeReportV2,
    pub structural: FullWorldStructuralBakeV2,
    pub output_root: String,
    pub recovery_backup: Option<String>,
}

/// Compatibility entry point. New production generation should call
/// `commit_full_world_bake_v2` so structural caches are persisted atomically.
pub fn commit_full_world_bake_v1(
    output_root: impl AsRef<Path>,
    manifest: &WorldManifestV2,
    chunks: &mut Vec<TavernMap>,
    hydrology_settings: HydrologySettingsV2,
) -> Result<WorldBakeTransactionReportV1, String> {
    let v2 = commit_full_world_bake_v2(
        output_root,
        manifest,
        chunks,
        hydrology_settings,
        ElevationCliffSettingsV2::default(),
    )?;
    Ok(WorldBakeTransactionReportV1 {
        schema: WORLD_BAKE_TRANSACTION_SCHEMA.to_owned(),
        world_seed: v2.world_seed,
        width_chunks: v2.width_chunks,
        height_chunks: v2.height_chunks,
        chunk_count: v2.chunk_count,
        hydrology: v2.hydrology,
        output_root: v2.output_root,
        recovery_backup: v2.recovery_backup,
    })
}

/// Validates, resolves, stages, and atomically commits hydrology, structural
/// terrain, and every generated chunk baseline.
pub fn commit_full_world_bake_v2(
    output_root: impl AsRef<Path>,
    manifest: &WorldManifestV2,
    chunks: &mut Vec<TavernMap>,
    hydrology_settings: HydrologySettingsV2,
    cliff_settings: ElevationCliffSettingsV2,
) -> Result<WorldBakeTransactionReportV2, String> {
    manifest.validate()?;
    validate_chunk_count(manifest, chunks.len())?;

    // Hydrology mutates the legacy-compatible water representation first.
    // Structural terrain is then derived from the normalized maps.
    let hydrology = bake_full_world_hydrology_v2(manifest, chunks, hydrology_settings)?;
    let structural = bake_full_world_structural_terrain_v2(manifest, chunks, cliff_settings)?;

    let output_root = output_root.as_ref();
    let parent = output_root
        .parent()
        .ok_or_else(|| "world output root must have a parent directory".to_owned())?;
    create_dir_all(parent).map_err(|error| error.to_string())?;

    let token = transaction_token();
    let stem = output_root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "world output root must have a valid directory name".to_owned())?;
    let staging = parent.join(format!(".{stem}.staging-{token}"));
    let backup = parent.join(format!(".{stem}.recovery-{token}"));

    if staging.exists() {
        remove_dir_all(&staging).map_err(|error| error.to_string())?;
    }
    create_dir_all(&staging).map_err(|error| error.to_string())?;

    if let Err(error) = stage_world_v2(&staging, manifest, chunks, &hydrology, &structural) {
        let _ = remove_dir_all(&staging);
        return Err(error);
    }

    let mut recovery_backup = None;
    if output_root.exists() {
        rename(output_root, &backup).map_err(|error| {
            let _ = remove_dir_all(&staging);
            format!("failed to move existing world into recovery backup: {error}")
        })?;
        recovery_backup = Some(backup.to_string_lossy().to_string());
    }

    if let Err(error) = rename(&staging, output_root) {
        if backup.exists() {
            let _ = rename(&backup, output_root);
        }
        let _ = remove_dir_all(&staging);
        return Err(format!("failed to commit staged world directory: {error}"));
    }

    let report = WorldBakeTransactionReportV2 {
        schema: WORLD_BAKE_TRANSACTION_V2_SCHEMA.to_owned(),
        world_seed: manifest.world_seed,
        width_chunks: manifest.width_chunks,
        height_chunks: manifest.height_chunks,
        chunk_count: chunks.len(),
        hydrology,
        structural,
        output_root: output_root.to_string_lossy().to_string(),
        recovery_backup,
    };
    write_json(&output_root.join("world_bake_transaction_v2.json"), &report)?;
    Ok(report)
}

fn validate_chunk_count(manifest: &WorldManifestV2, actual: usize) -> Result<(), String> {
    let expected = usize::try_from(manifest.width_chunks)
        .map_err(|_| "world width does not fit this platform".to_owned())?
        .checked_mul(
            usize::try_from(manifest.height_chunks)
                .map_err(|_| "world height does not fit this platform".to_owned())?,
        )
        .ok_or_else(|| "world chunk count overflow".to_owned())?;
    if actual != expected {
        return Err(format!(
            "world bake transaction expected {expected} chunks, found {actual}"
        ));
    }
    Ok(())
}

fn stage_world_v2(
    staging: &Path,
    manifest: &WorldManifestV2,
    chunks: &[TavernMap],
    hydrology: &FullWorldHydrologyBakeReportV2,
    structural: &FullWorldStructuralBakeV2,
) -> Result<(), String> {
    write_json(&staging.join("world_manifest_v2.json"), manifest)?;
    write_json(
        &staging.join("generation/full_world_hydrology_v2.json"),
        hydrology,
    )?;
    write_json(
        &staging.join("generation/full_world_structural_bake_v2.json"),
        structural,
    )?;

    let width = usize::try_from(manifest.width_chunks)
        .map_err(|_| "world width does not fit this platform".to_owned())?;
    for (index, map) in chunks.iter().enumerate() {
        let chunk_x = index % width;
        let chunk_y = index / width;
        let chunk_root = staging
            .join("chunks")
            .join(format!("x_{chunk_x}_y_{chunk_y}"));
        write_tavern_map(&chunk_root.join("baseline.tworld"), map)?;
        let cache = structural
            .chunks
            .get(index)
            .ok_or_else(|| format!("missing structural cache for chunk index {index}"))?;
        if cache.chunk_x as usize != chunk_x || cache.chunk_y as usize != chunk_y {
            return Err(format!(
                "structural cache order mismatch at index {index}: expected ({chunk_x}, {chunk_y}), found ({}, {})",
                cache.chunk_x, cache.chunk_y
            ));
        }
        write_json(&chunk_root.join("structural_cache_v2.json"), cache)?;
    }
    Ok(())
}

fn write_tavern_map(path: &Path, map: &TavernMap) -> Result<(), String> {
    let mut scene = SceneMap::blank(
        "generated_surface_chunk",
        "Generated Surface Chunk",
        SceneKind::Exterior,
        SceneBiome::Temperate,
    );
    scene.map = map.clone();
    let world = GameWorld {
        scenes: SceneRegistry::from(vec![scene.clone()]),
        active_scene: SceneReference::from(scene.id.clone()),
        tile_rules: Vec::new(),
    };
    write_bytes(path, world.serialize_lines().as_bytes())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    write_bytes(path, &bytes)
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    atomic_write(&path.to_string_lossy(), bytes)
}

fn transaction_token() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::TileKind;

    #[test]
    fn rejects_incomplete_chunk_sets_before_writing() {
        let root = std::env::temp_dir().join(format!(
            "havenwild_world_bake_incomplete_{}",
            std::process::id()
        ));
        let manifest = WorldManifestV2::new(77, 2, 1, haven_core::MAP_W as u16);
        let mut chunks = vec![TavernMap::filled(TileKind::Grass)];
        let error = commit_full_world_bake_v2(
            &root,
            &manifest,
            &mut chunks,
            HydrologySettingsV2::default(),
            ElevationCliffSettingsV2::default(),
        )
        .expect_err("incomplete world must fail");
        assert!(error.contains("expected 2 chunks"));
        assert!(!root.exists());
    }
}
