use crate::{GameWorld, ObjectKind, PlacedObject, SceneMap, TileKind, Transition, MAP_H, MAP_W};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug)]
pub struct WorldgenExportReport {
    pub pack_path: String,
    pub scene_count: usize,
    pub object_count: usize,
    pub transition_count: usize,
    pub backup_path: Option<String>,
    pub warnings: Vec<String>,
}

impl WorldgenExportReport {
    pub fn status_line(&self) -> String {
        let backup = self
            .backup_path
            .as_ref()
            .map(|path| format!("; backup: {path}"))
            .unwrap_or_default();
        if self.warnings.is_empty() {
            format!(
                "Exported worldgen JSON: {} scene(s), {} object(s), {} transition(s) -> {}{}",
                self.scene_count, self.object_count, self.transition_count, self.pack_path, backup
            )
        } else {
            format!(
                "Exported worldgen JSON with {} warning(s): {} scene(s), {} object(s), {} transition(s) -> {}{}",
                self.warnings.len(),
                self.scene_count,
                self.object_count,
                self.transition_count,
                self.pack_path,
                backup
            )
        }
    }
}

pub fn export_worldgen_pack_to_path(
    world: &GameWorld,
    pack_path: impl AsRef<Path>,
) -> Result<WorldgenExportReport, String> {
    let pack_path = pack_path.as_ref();
    let repo_root = infer_repo_root(pack_path);
    let scene_dir = repo_root.join("content/worldgen/exports/home_island_v0_10");
    let backup_path = backup_existing_export(&repo_root, pack_path, &scene_dir)?;

    fs::create_dir_all(&scene_dir).map_err(|err| {
        format!(
            "failed to create export scene folder {}: {err}",
            scene_dir.display()
        )
    })?;
    if let Some(parent) = pack_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create pack folder {}: {err}", parent.display()))?;
    }

    let mut warnings = Vec::new();
    let mut scene_files = Vec::new();
    let mut object_count = 0usize;
    let mut transition_count = 0usize;

    for scene in &world.scenes {
        let scene_file = format!(
            "content/worldgen/exports/home_island_v0_10/{}_scene_export_v0_10.json",
            scene.id.code()
        );
        let scene_path = repo_root.join(&scene_file);
        let scene_json = scene_to_json(scene, &mut warnings);
        let data = serde_json::to_string_pretty(&scene_json)
            .map_err(|err| format!("failed to serialize scene {}: {err}", scene.id.code()))?;
        fs::write(&scene_path, format!("{data}\n"))
            .map_err(|err| format!("failed to write {}: {err}", scene_path.display()))?;
        scene_files.push(scene_file);
        object_count += scene.map.objects.len();
        transition_count += scene.transitions.len();
    }

    let pack_json = json!({
        "id": "worldgen_home_island_runtime_export_v0_10",
        "name": "Worldgen Home Island Runtime Export v0.10",
        "version": "0.10.0",
        "kind": "worldgen_asset_pack",
        "source": "editor_runtime_export",
        "previousAuthoringPack": "content/worldgen/packs/worldgen_home_island_v0_10.json",
        "masterManifest": "assets/generated/worldgen_v0_1/worldgen_asset_manifest_v0_1.json",
        "runtimeBindings": "assets/generated/worldgen_v0_1/runtime_bindings/rust_tilekind_mapping_v0_1.json",
        "sceneFiles": scene_files,
        "editor": {
            "defaultScene": world.active_scene.code(),
            "exportedBy": "GameWorld::export_worldgen_pack_path",
            "exportContract": "content/worldgen/worldgen_scene_export_profile_v0_10.json",
            "reloadHotkey": "F10",
            "exportHotkey": "F11"
        },
        "runtimeSourceLoader": "crates/haven_core/src/worldgen_loader.rs",
        "runtimeSourceExporter": "crates/haven_core/src/worldgen_exporter.rs",
        "validationProfile": "content/worldgen/worldgen_scene_export_profile_v0_10.json"
    });
    let pack_data = serde_json::to_string_pretty(&pack_json)
        .map_err(|err| format!("failed to serialize export pack: {err}"))?;
    fs::write(pack_path, format!("{pack_data}\n"))
        .map_err(|err| format!("failed to write {}: {err}", pack_path.display()))?;

    Ok(WorldgenExportReport {
        pack_path: pack_path.display().to_string(),
        scene_count: world.scenes.len(),
        object_count,
        transition_count,
        backup_path: backup_path.map(|path| path.display().to_string()),
        warnings,
    })
}

fn scene_to_json(scene: &SceneMap, warnings: &mut Vec<String>) -> Value {
    let terrain: Vec<Value> = (0..MAP_H as i32)
        .map(|y| {
            Value::Array(
                (0..MAP_W as i32)
                    .map(|x| Value::String(tile_json_name(scene.map.get(x, y))))
                    .collect(),
            )
        })
        .collect();
    let zones: Vec<Value> = (0..MAP_H as i32)
        .map(|y| {
            Value::Array(
                (0..MAP_W as i32)
                    .map(|x| Value::String(scene.zone_at(x, y).code().to_string()))
                    .collect(),
            )
        })
        .collect();
    let structural_levels: Vec<Value> = (0..MAP_H as i32)
        .map(|y| {
            Value::Array(
                (0..MAP_W as i32)
                    .map(|x| {
                        scene
                            .map
                            .get_structural_level(x, y)
                            .map(|level| Value::from(u64::from(level)))
                            .unwrap_or(Value::Null)
                    })
                    .collect(),
            )
        })
        .collect();
    let objects: Vec<Value> = scene
        .map
        .objects
        .iter()
        .enumerate()
        .map(|(index, object)| {
            object_to_json(
                index,
                *object,
                scene.map.object_asset_ref(object.id),
                warnings,
            )
        })
        .collect();
    let transitions: Vec<Value> = scene
        .transitions
        .iter()
        .enumerate()
        .map(|(index, transition)| transition_to_json(index, transition))
        .collect();

    json!({
        "id": format!("{}_scene_export_v0_10", scene.id.code()),
        "version": "0.10.0",
        "kind": "worldgen_scene",
        "sceneId": scene.id.code(),
        "title": scene.name,
        "sceneKind": scene.kind.code(),
        "biome": scene.biome.code(),
        "role": format!("runtime_export_{}", scene.id.code()),
        "sceneSize": [MAP_W, MAP_H],
        "tileSize": [32, 32],
        "edgePolicy": {
            "mustAvoidVoid": true,
            "resolvedBorders": default_edge_borders(scene)
        },
        "layers": {
            "terrain": terrain,
            "structuralLevels": structural_levels,
            "zones": zones
        },
        "objects": objects,
        "transitions": transitions,
        "spawns": [
            {"id": "player_default", "tile": [scene.spawn_x, scene.spawn_y]}
        ],
        "editor": {
            "showLayers": ["terrain", "structural_levels", "zones", "objects", "collision", "interactions", "transitions", "scene_edges"],
            "defaultTool": "inspect_select",
            "allowPaintTerrain": true,
            "allowMoveObjects": true,
            "exportedFromRuntime": true
        },
        "validationRules": [
            "scene_size_matches_layers",
            "object_rects_inside_scene",
            "collision_rects_inside_scene",
            "runtime_export_round_trip_loads",
            "structural_levels_round_trip"
        ]
    })
}

fn object_to_json(
    index: usize,
    object: PlacedObject,
    asset_ref: Option<&crate::StablePlaceableAssetRef>,
    warnings: &mut Vec<String>,
) -> Value {
    let (vx, vy, vw, vh) = object.visual_rect();
    let (cx, cy, cw, ch) = object.collision_rect();
    let (ix, iy, iw, ih) = object.interaction_rect();
    if vw <= 0 || vh <= 0 {
        warnings.push(format!(
            "{} at {},{} exported with non-positive visual rect",
            object.kind.label(),
            object.x,
            object.y
        ));
    }
    let published_asset_id = asset_ref
        .map(|asset_ref| {
            asset_ref
                .scene_asset_alias_id()
                .unwrap_or(asset_ref.asset_id.as_str())
        })
        .unwrap_or_else(|| asset_id_for_object(object.kind));
    json!({
        "id": format!("runtime_{}_{}_{}", object.kind.code(), index, object.x),
        "assetId": published_asset_id,
        "runtimeKind": object.kind.code(),
        "anchor": [object.x, object.y],
        "visualRect": [vx, vy, vw, vh],
        "collisionRect": [cx, cy, cw, ch],
        "layer": layer_for_object(object.kind, object.footprint.occludes_player),
        "blocksMovement": object.footprint.blocks_movement,
        "occludesPlayer": object.footprint.occludes_player,
        "fadeWhenPlayerBehind": object.footprint.fade_when_player_behind,
        "interactions": [
            {
                "id": format!("interact_{}_{}", object.kind.code(), index),
                "kind": interaction_kind_for_object(object.kind),
                "rect": [ix, iy, iw, ih]
            }
        ]
    })
}

fn transition_to_json(index: usize, transition: &Transition) -> Value {
    json!({
        "id": format!("transition_{}_{}", transition.target.code(), index),
        "label": transition.label,
        "rect": [transition.x, transition.y, transition.w, transition.h],
        "toScene": transition.target.code(),
        "toSpawn": [transition.spawn_x, transition.spawn_y],
        "returnRequired": true
    })
}

fn tile_json_name(tile: TileKind) -> String {
    let mut name = String::new();
    for part in tile.code().split('_').filter(|part| !part.is_empty()) {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            name.push(first.to_ascii_uppercase());
            name.extend(chars);
        }
    }
    name
}

fn asset_id_for_object(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Table => "runtime_table",
        ObjectKind::Chair => "runtime_chair",
        ObjectKind::Bar => "runtime_bar_counter",
        ObjectKind::Keg => "runtime_keg_barrel",
        ObjectKind::Bed => "runtime_bed",
        ObjectKind::Fireplace => "runtime_fireplace",
        ObjectKind::GreenhouseMarker => "runtime_greenhouse_marker",
        ObjectKind::Tree => "runtime_tree",
        ObjectKind::Bush => "runtime_berry_bush",
        ObjectKind::Boulder => "runtime_boulder",
        ObjectKind::OreNode => "runtime_ore_node",
        ObjectKind::Mushroom => "runtime_mushroom",
        ObjectKind::Herb => "runtime_wild_herb",
        ObjectKind::Crate => "runtime_crate_stack",
        ObjectKind::Barrel => "runtime_barrel",
        ObjectKind::Well => "runtime_well",
        ObjectKind::Scarecrow => "runtime_scarecrow",
        ObjectKind::Fence => "runtime_fence_segment",
        ObjectKind::Lamp => "runtime_lamp_post",
        ObjectKind::Bench => "runtime_bench",
        ObjectKind::Stump => "runtime_tree_stump",
        ObjectKind::Log => "runtime_fallen_log",
        ObjectKind::Sign => "runtime_signboard",
        ObjectKind::Door => "runtime_door",
        ObjectKind::Stairs => "runtime_stairs",
        ObjectKind::CaveEntrance => "runtime_cave_entrance",
    }
}

fn layer_for_object(kind: ObjectKind, occludes_player: bool) -> &'static str {
    if occludes_player
        || matches!(
            kind,
            ObjectKind::Tree
                | ObjectKind::CaveEntrance
                | ObjectKind::Scarecrow
                | ObjectKind::Lamp
                | ObjectKind::Sign
        )
    {
        "tall_object"
    } else if matches!(
        kind,
        ObjectKind::Table
            | ObjectKind::Chair
            | ObjectKind::Bar
            | ObjectKind::Keg
            | ObjectKind::Bed
            | ObjectKind::Fireplace
            | ObjectKind::Boulder
            | ObjectKind::Crate
            | ObjectKind::Barrel
            | ObjectKind::Well
            | ObjectKind::Fence
            | ObjectKind::Bench
            | ObjectKind::Stump
            | ObjectKind::Log
    ) {
        "object"
    } else {
        "marker"
    }
}

fn interaction_kind_for_object(kind: ObjectKind) -> &'static str {
    match kind {
        ObjectKind::Tree
        | ObjectKind::Bush
        | ObjectKind::Boulder
        | ObjectKind::OreNode
        | ObjectKind::Mushroom
        | ObjectKind::Herb
        | ObjectKind::Stump
        | ObjectKind::Log => "harvest",
        ObjectKind::Door | ObjectKind::Stairs | ObjectKind::CaveEntrance => "enter",
        ObjectKind::Bar => "service",
        ObjectKind::Bed | ObjectKind::Bench => "rest",
        _ => "use",
    }
}

fn default_edge_borders(scene: &SceneMap) -> Value {
    let border = match scene.kind.code() {
        "cave" => "cave_darkness",
        "interior" => "interior_wall",
        _ => "forest",
    };
    json!({"north": border, "south": border, "east": border, "west": border})
}

fn backup_existing_export(
    repo_root: &Path,
    pack_path: &Path,
    scene_dir: &Path,
) -> Result<Option<PathBuf>, String> {
    if !pack_path.exists() && !scene_dir.exists() {
        return Ok(None);
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let backup_root = repo_root.join(format!("workspace/worldgen_backups/export_v0_10_{stamp}"));
    if pack_path.exists() {
        let relative = pack_path.strip_prefix(repo_root).unwrap_or(pack_path);
        let target = backup_root.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("failed to create backup folder {}: {err}", parent.display())
            })?;
        }
        fs::copy(pack_path, &target)
            .map_err(|err| format!("failed to back up {}: {err}", pack_path.display()))?;
    }
    if scene_dir.exists() {
        let relative = scene_dir.strip_prefix(repo_root).unwrap_or(scene_dir);
        copy_dir_recursive(scene_dir, &backup_root.join(relative))?;
    }
    Ok(Some(backup_root))
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|err| format!("failed to create backup folder {}: {err}", dst.display()))?;
    for entry in
        fs::read_dir(src).map_err(|err| format!("failed to read {}: {err}", src.display()))?
    {
        let entry = entry.map_err(|err| format!("failed to read directory entry: {err}"))?;
        let file_type = entry.file_type().map_err(|err| {
            format!(
                "failed to read file type for {}: {err}",
                entry.path().display()
            )
        })?;
        let target = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target).map_err(|err| {
                format!(
                    "failed to copy {} to {}: {err}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

fn infer_repo_root(path: &Path) -> PathBuf {
    let mut cursor = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        if cursor.join("Cargo.toml").exists() || cursor.join("content").exists() {
            return if cursor.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                cursor
            };
        }
        if !cursor.pop() {
            break;
        }
    }
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_worldgen_pack_from_path;
    use crate::ObjectFootprint;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .expect("workspace root")
            .to_path_buf()
    }

    fn unique_test_root() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        repo_root()
            .join("WORKSPACE")
            .join("test-output")
            .join(format!("worldgen_exporter_{stamp}"))
    }

    fn assert_footprint_eq(expected: ObjectFootprint, actual: ObjectFootprint) {
        assert_eq!(expected.visual_offset_x, actual.visual_offset_x);
        assert_eq!(expected.visual_offset_y, actual.visual_offset_y);
        assert_eq!(expected.visual_w, actual.visual_w);
        assert_eq!(expected.visual_h, actual.visual_h);
        assert_eq!(expected.collision_offset_x, actual.collision_offset_x);
        assert_eq!(expected.collision_offset_y, actual.collision_offset_y);
        assert_eq!(expected.collision_w, actual.collision_w);
        assert_eq!(expected.collision_h, actual.collision_h);
        assert_eq!(expected.interaction_offset_x, actual.interaction_offset_x);
        assert_eq!(expected.interaction_offset_y, actual.interaction_offset_y);
        assert_eq!(expected.interaction_w, actual.interaction_w);
        assert_eq!(expected.interaction_h, actual.interaction_h);
        assert_eq!(expected.blocks_movement, actual.blocks_movement);
        assert_eq!(expected.occludes_player, actual.occludes_player);
        assert_eq!(
            expected.fade_when_player_behind,
            actual.fade_when_player_behind
        );
    }

    #[test]
    fn exported_tile_names_follow_canonical_tile_code_order() {
        assert_eq!(tile_json_name(TileKind::OceanShallow), "OceanShallow");
        assert_eq!(tile_json_name(TileKind::OceanDeep), "OceanDeep");
        assert_eq!(tile_json_name(TileKind::ShallowWater), "ShallowWater");
        assert_eq!(tile_json_name(TileKind::Water), "Water");
    }

    #[test]
    fn export_round_trips_worldgen_scenes() {
        let source_pack =
            repo_root().join("content/worldgen/packs/worldgen_home_island_v0_10.json");
        let (world, _) =
            load_worldgen_pack_from_path(&source_pack).expect("load v0.10 authoring pack");
        let test_root = unique_test_root();
        let export_pack =
            test_root.join("content/worldgen/packs/worldgen_home_island_runtime_export_v0_10.json");
        fs::create_dir_all(test_root.join("content")).expect("create test export root");

        let report = export_worldgen_pack_to_path(&world, &export_pack).expect("export pack");
        assert_eq!(report.scene_count, world.scenes.len());
        assert!(report.object_count > 0);

        let (round_tripped, _) =
            load_worldgen_pack_from_path(&export_pack).expect("reload exported pack");
        assert_eq!(round_tripped.scenes.len(), world.scenes.len());

        for (expected_scene, actual_scene) in world.scenes.iter().zip(round_tripped.scenes.iter()) {
            assert_eq!(expected_scene.id, actual_scene.id);
            assert_eq!(expected_scene.kind, actual_scene.kind);
            assert_eq!(expected_scene.biome, actual_scene.biome);
            assert_eq!(expected_scene.spawn_x, actual_scene.spawn_x);
            assert_eq!(expected_scene.spawn_y, actual_scene.spawn_y);
            assert_eq!(
                expected_scene.map.objects.len(),
                actual_scene.map.objects.len(),
                "object count mismatch for {}",
                expected_scene.id.code()
            );
            assert_eq!(
                expected_scene.transitions.len(),
                actual_scene.transitions.len(),
                "transition count mismatch for {}",
                expected_scene.id.code()
            );

            for y in 0..MAP_H as i32 {
                for x in 0..MAP_W as i32 {
                    assert_eq!(
                        expected_scene.map.get_structural_level(x, y),
                        actual_scene.map.get_structural_level(x, y),
                        "structural level mismatch for {} at {},{}",
                        expected_scene.id.code(),
                        x,
                        y
                    );
                }
            }

            for (expected, actual) in expected_scene
                .map
                .objects
                .iter()
                .zip(actual_scene.map.objects.iter())
            {
                assert_eq!(expected.kind, actual.kind);
                assert_eq!(expected.x, actual.x);
                assert_eq!(expected.y, actual.y);
                assert_footprint_eq(expected.footprint, actual.footprint);
            }
        }

        let _ = fs::remove_dir_all(test_root);
    }
}
