use super::*;
use haven_core::{scene_dimension_offset, MAP_H, MAP_W};
use serde::Deserialize;
use std::{fs::read_to_string, path::{Path, PathBuf}};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldgenBuildingPlacementInput {
    #[serde(default)]
    id: Option<String>,
    recipe_id: String,
    anchor_tile: [i32; 2],
    #[serde(default)]
    initial_level: i32,
    #[serde(default)]
    placement_space: Option<BuildingPlacementSpace>,
    #[serde(default)]
    surface_region_id: Option<String>,
    #[serde(default)]
    global_anchor_tile: Option<[i32; 2]>,
}

impl BuildingInstanceRegistry {
    /// Merges BuildingInstance placement declarations embedded in worldgen scene
    /// documents. Core world loading safely ignores the extra top-level field;
    /// BuildingInstanceRegistry remains its single placement authority.
    pub fn merge_worldgen_pack_placements(
        &mut self,
        pack_path: impl AsRef<Path>,
        recipes: &BuildingRecipeRegistry,
    ) -> Result<usize, String> {
        let pack_path = pack_path.as_ref();
        let pack: serde_json::Value = serde_json::from_str(
            &read_to_string(pack_path).map_err(|error| format!("{}: {error}", pack_path.display()))?,
        )
        .map_err(|error| format!("{}: {error}", pack_path.display()))?;
        let repo_root = infer_repo_root(pack_path);
        let pack_id = pack
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("worldgen_pack");
        let scene_files = pack
            .get("sceneFiles")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(|| format!("{}: missing sceneFiles", pack_path.display()))?;
        self.generated_base.clear();
        let mut count = 0usize;
        for scene_file in scene_files {
            let relative = scene_file
                .as_str()
                .ok_or_else(|| "sceneFiles must contain string paths".to_string())?;
            let scene_path = repo_root.join(relative);
            let value: serde_json::Value = serde_json::from_str(
                &read_to_string(&scene_path)
                    .map_err(|error| format!("{}: {error}", scene_path.display()))?,
            )
            .map_err(|error| format!("{}: {error}", scene_path.display()))?;
            let Some(placements) = value
                .get("buildingInstances")
                .and_then(serde_json::Value::as_array)
            else {
                continue;
            };
            let scene_id = value
                .get("sceneId")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("{}: buildingInstances require sceneId", scene_path.display()))?;
            let (offset_x, offset_y) = scene_offset_from_json(&value)?;
            for (placement_index, raw) in placements.iter().enumerate() {
                let input: WorldgenBuildingPlacementInput = serde_json::from_value(raw.clone())
                    .map_err(|error| format!(
                        "{} buildingInstances[{placement_index}]: {error}",
                        scene_path.display()
                    ))?;
                let placement_space = input.placement_space.unwrap_or_default();
                let local_anchor = [
                    input.anchor_tile[0] + offset_x,
                    input.anchor_tile[1] + offset_y,
                ];
                let stable_anchor = if placement_space == BuildingPlacementSpace::ContinuousSurface {
                    input.global_anchor_tile.unwrap_or(local_anchor)
                } else {
                    local_anchor
                };
                let id = input.id.unwrap_or_else(|| {
                    deterministic_worldgen_instance_id(
                        pack_id,
                        scene_id,
                        &input.recipe_id,
                        stable_anchor,
                    )
                });
                let definition = BuildingInstanceDefinition {
                    schema: BUILDING_INSTANCE_SCHEMA.to_string(),
                    id: id.clone(),
                    recipe_id: input.recipe_id,
                    scene_id: scene_id.to_string(),
                    anchor_tile: local_anchor,
                    initial_level: input.initial_level,
                    diagnostic_only: false,
                    origin: BuildingInstanceOrigin::Worldgen,
                    placement_space,
                    surface_region_id: input.surface_region_id,
                    global_anchor_tile: if placement_space == BuildingPlacementSpace::ContinuousSurface {
                        Some(stable_anchor)
                    } else {
                        None
                    },
                };
                let errors = definition.validate(recipes);
                if !errors.is_empty() {
                    return Err(errors.join("; "));
                }
                if self.generated_base.insert(id.clone(), definition).is_some() {
                    return Err(format!("duplicate worldgen BuildingInstance id {id}"));
                }
                count += 1;
            }
        }
        self.rebuild_effective(recipes)?;
        Ok(count)
    }
}

fn scene_offset_from_json(value: &serde_json::Value) -> Result<(i32, i32), String> {
    let size = value
        .get("sceneSize")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "worldgen scene building placement requires sceneSize".to_string())?;
    if size.len() != 2 {
        return Err("worldgen scene sceneSize must contain width and height".to_string());
    }
    let width = size[0]
        .as_u64()
        .ok_or_else(|| "sceneSize width must be integer".to_string())? as usize;
    let height = size[1]
        .as_u64()
        .ok_or_else(|| "sceneSize height must be integer".to_string())? as usize;
    if width == 0 || height == 0 || width > MAP_W || height > MAP_H {
        return Err(format!("sceneSize {width}x{height} exceeds {MAP_W}x{MAP_H}"));
    }
    Ok(scene_dimension_offset(width, height))
}

fn deterministic_worldgen_instance_id(
    pack_id: &str,
    scene_id: &str,
    recipe_id: &str,
    anchor: [i32; 2],
) -> String {
    let input = format!("{pack_id}|{scene_id}|{recipe_id}|{},{}", anchor[0], anchor[1]);
    format!(
        "pcg.{}.{:016x}",
        slug(recipe_id.rsplit('.').next().unwrap_or(recipe_id)),
        fnv1a64(input.as_bytes())
    )
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn slug(value: &str) -> String {
    let mut result = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character.to_ascii_lowercase());
        } else if !result.ends_with('_') {
            result.push('_');
        }
    }
    result.trim_matches('_').to_string()
}

fn infer_repo_root(path: &Path) -> PathBuf {
    let start = if path.is_dir() {
        path.to_path_buf()
    } else {
        path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf()
    };
    for ancestor in start.ancestors() {
        if ancestor.join("Cargo.toml").exists() && ancestor.join("content").is_dir() {
            return ancestor.to_path_buf();
        }
    }
    PathBuf::from(".")
}
