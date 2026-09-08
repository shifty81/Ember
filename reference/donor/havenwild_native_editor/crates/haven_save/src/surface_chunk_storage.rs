//! Disk baseline cache and per-chunk replacement deltas for generated surface chunks.

use haven_core::{GameWorld, ProjectSceneId, SceneMap, SceneReference, SceneRegistry};
use haven_world::open_world::ChunkCoord;
use std::fs::{create_dir_all, read_to_string};
use std::path::{Path, PathBuf};

use crate::atomic_write;

pub const SURFACE_CHUNK_CACHE_VERSION: u32 = 1;

fn chunk_dir(root: &str, chunk: ChunkCoord) -> PathBuf {
    Path::new(root).join(format!("x_{}_y_{}", chunk.x, chunk.y))
}

pub fn surface_chunk_baseline_path(root: &str, chunk: ChunkCoord) -> PathBuf {
    chunk_dir(root, chunk).join("baseline.tworld")
}

pub fn surface_chunk_delta_path(root: &str, chunk: ChunkCoord) -> PathBuf {
    chunk_dir(root, chunk).join("player_delta.tworld")
}

pub fn save_surface_chunk_baseline(
    root: &str,
    chunk: ChunkCoord,
    scene: &SceneMap,
) -> Result<(), String> {
    let path = surface_chunk_baseline_path(root, chunk);
    save_scene_snapshot(&path, scene)
}

pub fn save_surface_chunk_delta(
    root: &str,
    chunk: ChunkCoord,
    scene: &SceneMap,
) -> Result<(), String> {
    let path = surface_chunk_delta_path(root, chunk);
    save_scene_snapshot(&path, scene)
}

pub fn load_surface_chunk(root: &str, chunk: ChunkCoord) -> Result<Option<SceneMap>, String> {
    let delta = surface_chunk_delta_path(root, chunk);
    if delta.is_file() {
        return load_scene_snapshot(&delta).map(Some);
    }
    let baseline = surface_chunk_baseline_path(root, chunk);
    if baseline.is_file() {
        return load_scene_snapshot(&baseline).map(Some);
    }
    Ok(None)
}

pub fn surface_chunk_is_cached(root: &str, chunk: ChunkCoord) -> bool {
    surface_chunk_baseline_path(root, chunk).is_file()
}

fn save_scene_snapshot(path: &Path, scene: &SceneMap) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let world = GameWorld {
        scenes: SceneRegistry::from(vec![scene.clone()]),
        active_scene: SceneReference::from(scene.id.clone()),
        tile_rules: Vec::new(),
    };
    atomic_write(&path.to_string_lossy(), world.serialize_lines().as_bytes())
}

fn load_scene_snapshot(path: &Path) -> Result<SceneMap, String> {
    let raw = read_to_string(path).map_err(|error| error.to_string())?;
    let world = GameWorld::deserialize_lines(&raw)?;
    world
        .scenes
        .iter()
        .next()
        .cloned()
        .ok_or_else(|| "chunk snapshot contains no scene".to_string())
}

pub fn remove_surface_chunk_delta(root: &str, chunk: ChunkCoord) -> Result<(), String> {
    let path = surface_chunk_delta_path(root, chunk);
    if path.exists() {
        std::fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn snapshot_scene_id(scene: &SceneMap) -> ProjectSceneId {
    scene.id.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::{SceneBiome, SceneKind};

    #[test]
    fn delta_takes_priority_over_baseline() {
        let root =
            std::env::temp_dir().join(format!("havenwild_chunk_cache_{}", std::process::id()));
        let root_text = root.to_string_lossy().to_string();
        let chunk = ChunkCoord::new(3, -2);
        let mut baseline = SceneMap::blank(
            "surface_x_p3_y_n2",
            "Baseline",
            SceneKind::Exterior,
            SceneBiome::Temperate,
        );
        save_surface_chunk_baseline(&root_text, chunk, &baseline).unwrap();
        baseline.name = "Edited".to_string();
        save_surface_chunk_delta(&root_text, chunk, &baseline).unwrap();
        assert_eq!(
            load_surface_chunk(&root_text, chunk).unwrap().unwrap().name,
            "Edited"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
