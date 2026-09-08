use crate::{
    scene_dimension_offset, GameWorld, ObjectId, ObjectKind, PlacedObject, ProjectSceneId,
    SceneBiome, SceneId, SceneKind, SceneMap, SceneReference, SceneRegistry, StablePlaceableAssetRef, TavernMap, TileKind,
    Transition, TransitionId, ZoneKind, MAP_H, MAP_W,
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[path = "worldgen_loader_structural.rs"]
mod structural;
use self::structural::parse_structural_levels;

#[derive(Clone, Debug)]
pub struct WorldgenLoadReport {
    pub pack_id: String,
    pub version: String,
    pub scene_count: usize,
    pub warning_count: usize,
    pub warnings: Vec<String>,
}

impl WorldgenLoadReport {
    pub fn status_line(&self) -> String {
        if self.warning_count == 0 {
            format!(
                "Loaded worldgen pack {} v{}: {} scene(s)",
                self.pack_id, self.version, self.scene_count
            )
        } else {
            format!(
                "Loaded worldgen pack {} v{}: {} scene(s), {} warning(s)",
                self.pack_id, self.version, self.scene_count, self.warning_count
            )
        }
    }
}

pub fn load_worldgen_pack_from_path(
    path: impl AsRef<Path>,
) -> Result<(GameWorld, WorldgenLoadReport), String> {
    let pack_path = path.as_ref();
    let pack = read_json(pack_path)?;
    let repo_root = infer_repo_root(pack_path);
    let pack_id = string_field(&pack, "id")
        .unwrap_or("unknown_worldgen_pack")
        .to_string();
    let version = string_field(&pack, "version")
        .unwrap_or("0.0.0")
        .to_string();
    let scene_files = array_field(&pack, "sceneFiles")?;
    let mut warnings = Vec::new();
    let mut scenes = Vec::new();

    for scene_file in scene_files {
        let rel = scene_file
            .as_str()
            .ok_or_else(|| "sceneFiles must contain string paths".to_string())?;
        let scene_path = repo_root.join(rel);
        let scene_json = read_json(&scene_path)?;
        scenes.push(parse_scene(&scene_json, rel, &mut warnings)?);
    }

    if scenes.is_empty() {
        return Err(format!(
            "worldgen pack {pack_id} did not contain any scenes"
        ));
    }

    if legacy_scene_set_required(&pack) {
        for required in SceneId::ALL {
            if !scenes.iter().any(|scene| scene.id == required) {
                warnings.push(format!(
                    "worldgen pack {pack_id} is missing scene {}",
                    required.code()
                ));
            }
        }
    }

    let active_scene = if let Some(default_code) = preferred_default_scene_code(&pack) {
        let requested = ProjectSceneId::new(default_code);
        if let Some(scene) = scenes.iter().find(|scene| scene.id == requested) {
            scene.id.clone()
        } else {
            warnings.push(format!(
                "worldgen pack {pack_id} default scene '{}' is not loaded; using the first scene",
                requested.code()
            ));
            scenes[0].id.clone()
        }
    } else {
        scenes
            .iter()
            .find(|scene| scene.id == SceneId::Farmstead)
            .map(|scene| scene.id.clone())
            .unwrap_or_else(|| scenes[0].id.clone())
    };

    let scenes = SceneRegistry::from_scenes(scenes)?;
    for scene in &scenes {
        for transition in &scene.transitions {
            if !scenes.contains(transition.target.project_id()) {
                warnings.push(format!(
                    "scene {} transition '{}' targets unloaded scene {}",
                    scene.id, transition.label, transition.target
                ));
            }
        }
    }

    let report = WorldgenLoadReport {
        pack_id,
        version,
        scene_count: scenes.len(),
        warning_count: warnings.len(),
        warnings,
    };

    Ok((
        GameWorld {
            scenes,
            active_scene: active_scene.into(),
            tile_rules: TileKind::ALL
                .iter()
                .map(|tile| tile.default_interaction())
                .collect(),
        },
        report,
    ))
}

fn preferred_default_scene_code(pack: &Value) -> Option<&str> {
    string_field(pack, "defaultScene")
        .or_else(|| {
            pack.get("testWorld")
                .and_then(|value| string_field(value, "defaultScene"))
        })
        .or_else(|| {
            pack.get("editor")
                .and_then(|value| string_field(value, "defaultScene"))
        })
}

fn legacy_scene_set_required(pack: &Value) -> bool {
    pack.get("requiresLegacySceneSet")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn parse_scene(
    value: &Value,
    source_path: &str,
    warnings: &mut Vec<String>,
) -> Result<SceneMap, String> {
    let scene_code =
        string_field(value, "sceneId").ok_or_else(|| format!("{source_path}: missing sceneId"))?;
    let id = ProjectSceneId::new(normalize_code(scene_code));

    let kind_code = string_field(value, "sceneKind").unwrap_or("exterior");
    let kind = SceneKind::from_code(&normalize_code(kind_code))
        .ok_or_else(|| format!("{source_path}: unknown sceneKind {kind_code}"))?;

    let biome_code = string_field(value, "biome").unwrap_or("temperate");
    let biome = SceneBiome::from_code(&normalize_code(biome_code)).unwrap_or_else(|| {
        warnings.push(format!(
            "{source_path}: unsupported biome '{biome_code}', using temperate"
        ));
        SceneBiome::Temperate
    });

    let size = array_field(value, "sceneSize")?;
    if size.len() != 2 {
        return Err(format!(
            "{source_path}: sceneSize must contain width and height"
        ));
    }
    let source_w = size[0]
        .as_u64()
        .ok_or_else(|| format!("{source_path}: sceneSize width must be an integer"))?
        as usize;
    let source_h = size[1]
        .as_u64()
        .ok_or_else(|| format!("{source_path}: sceneSize height must be an integer"))?
        as usize;
    if source_w == 0 || source_h == 0 || source_w > MAP_W || source_h > MAP_H {
        return Err(format!(
            "{source_path}: sceneSize {source_w}x{source_h} exceeds runtime maximum {MAP_W}x{MAP_H}"
        ));
    }
    let (offset_x, offset_y) = scene_dimension_offset(source_w, source_h);
    if source_w != MAP_W || source_h != MAP_H {
        warnings.push(format!(
            "{source_path}: migrated legacy scene {source_w}x{source_h} into {MAP_W}x{MAP_H} at offset {offset_x},{offset_y}"
        ));
    }

    let layers = value
        .get("layers")
        .ok_or_else(|| format!("{source_path}: missing layers"))?;
    let terrain = layers
        .get("terrain")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{source_path}: missing layers.terrain"))?;

    let mut map = TavernMap::empty_with(scene_fill_for_kind(kind));
    for y in 0..source_h {
        let row = terrain
            .get(y)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{source_path}: missing terrain row {y}"))?;
        if row.len() != source_w {
            return Err(format!(
                "{source_path}: terrain row {y} is {} wide, expected {}",
                row.len(),
                source_w
            ));
        }
        for (x, cell) in row.iter().enumerate().take(source_w) {
            let raw_tile = cell
                .as_str()
                .ok_or_else(|| format!("{source_path}: terrain cell {x},{y} is not a string"))?;
            let tile = parse_tile(raw_tile).ok_or_else(|| {
                format!("{source_path}: unsupported tile '{raw_tile}' at {x},{y}")
            })?;
            let target_x = x as i32 + offset_x;
            let target_y = y as i32 + offset_y;
            map.set(target_x, target_y, tile);
            map.set_height(target_x, target_y, default_height(tile));
        }
    }

    parse_structural_levels(
        layers,
        source_path,
        source_w,
        source_h,
        offset_x,
        offset_y,
        &mut map,
    )?;

    for object in value
        .get("objects")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(placed) = parse_object(object, source_path, warnings, offset_x, offset_y) {
            let object_id = string_field(object, "id").unwrap_or("unnamed_object");
            let asset_id = string_field(object, "assetId").unwrap_or(object_id);
            let initial_state = string_field(object, "state");
            let _ = map.place_pack_defined_object_with_state(
                placed,
                StablePlaceableAssetRef::from_scene_asset_alias(asset_id),
                initial_state,
            );
        }
    }

    let zones = parse_zones(
        layers,
        source_path,
        warnings,
        source_w,
        source_h,
        offset_x,
        offset_y,
    )?;
    let transitions = parse_transitions(value, source_path, warnings, offset_x, offset_y)?;
    let (spawn_x, spawn_y) = parse_spawn(value)
        .map(|(x, y)| (x + offset_x, y + offset_y))
        .unwrap_or((MAP_W as i32 / 2, MAP_H as i32 / 2));
    let name = string_field(value, "title")
        .map(str::to_string)
        .unwrap_or_else(|| id.label())
        .replace('_', " ");

    Ok(SceneMap {
        id,
        kind,
        biome,
        name,
        dimensions: crate::SceneDimensions::new(source_w, source_h).validate()?,
        map,
        zones,
        transitions,
        autotile_overrides: Vec::new(),
        spawn_x,
        spawn_y,
    })
}

fn parse_zones(
    layers: &Value,
    source_path: &str,
    warnings: &mut Vec<String>,
    source_w: usize,
    source_h: usize,
    offset_x: i32,
    offset_y: i32,
) -> Result<Vec<ZoneKind>, String> {
    let mut zones = vec![ZoneKind::None; MAP_W * MAP_H];
    let Some(zone_rows) = layers.get("zones").and_then(Value::as_array) else {
        return Ok(zones);
    };

    if zone_rows.len() != source_h {
        return Err(format!(
            "{source_path}: zones layer has {} rows, expected {}",
            zone_rows.len(),
            source_h
        ));
    }

    for (y, zone_row) in zone_rows.iter().enumerate().take(source_h) {
        let row = zone_row
            .as_array()
            .ok_or_else(|| format!("{source_path}: zone row {y} is not an array"))?;
        if row.len() != source_w {
            return Err(format!(
                "{source_path}: zone row {y} is {} wide, expected {}",
                row.len(),
                source_w
            ));
        }
        for (x, zone_cell) in row.iter().enumerate().take(source_w) {
            let raw_zone = zone_cell
                .as_str()
                .ok_or_else(|| format!("{source_path}: zone cell {x},{y} is not a string"))?;
            let zone_code = normalize_code(raw_zone);
            let zone = ZoneKind::from_code(&zone_code).unwrap_or_else(|| {
                warnings.push(format!(
                    "{source_path}: unknown zone '{raw_zone}' at {x},{y}, using none"
                ));
                ZoneKind::None
            });
            let target_x = x as i32 + offset_x;
            let target_y = y as i32 + offset_y;
            if let Some(index) = TavernMap::idx(target_x, target_y) {
                zones[index] = zone;
            }
        }
    }

    Ok(zones)
}

fn parse_transitions(
    value: &Value,
    source_path: &str,
    _warnings: &mut Vec<String>,
    offset_x: i32,
    offset_y: i32,
) -> Result<Vec<Transition>, String> {
    let mut transitions: Vec<Transition> = Vec::new();
    for transition in value
        .get("transitions")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let rect = transition
            .get("rect")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{source_path}: transition missing rect"))?;
        if rect.len() != 4 {
            return Err(format!(
                "{source_path}: transition rect must have 4 numbers"
            ));
        }
        let target_code = string_field(transition, "toScene")
            .ok_or_else(|| format!("{source_path}: transition missing toScene"))?;
        let target = SceneReference::from(normalize_code(target_code));
        let spawn = transition
            .get("toSpawn")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{source_path}: transition missing toSpawn"))?;
        if spawn.len() != 2 {
            return Err(format!(
                "{source_path}: transition toSpawn must have 2 numbers"
            ));
        }
        let transition_key = string_field(transition, "id")
            .map(str::to_string)
            .unwrap_or_else(|| format!("{}:{}:{}", target.code(), rect[0], rect[1]));
        let label = string_field(transition, "label")
            .or_else(|| string_field(transition, "id"))
            .unwrap_or("Transition")
            .replace('_', " ");
        let mut transition_id = TransitionId::from_stable_key(&transition_key);
        if transitions
            .iter()
            .any(|transition| transition.id == transition_id)
        {
            transition_id =
                TransitionId::next_after(transitions.iter().map(|transition| transition.id));
        }
        transitions.push(Transition {
            id: transition_id,
            x: int_at(rect, 0, "transition rect x")? + offset_x,
            y: int_at(rect, 1, "transition rect y")? + offset_y,
            w: int_at(rect, 2, "transition rect w")?,
            h: int_at(rect, 3, "transition rect h")?,
            target,
            spawn_x: int_at(spawn, 0, "transition spawn x")? + offset_x,
            spawn_y: int_at(spawn, 1, "transition spawn y")? + offset_y,
            label,
        });
    }
    Ok(transitions)
}

fn parse_spawn(value: &Value) -> Option<(i32, i32)> {
    let spawns = value.get("spawns")?.as_array()?;
    let chosen = spawns
        .iter()
        .find(|spawn| string_field(spawn, "id") == Some("player_default"))
        .or_else(|| spawns.first())?;
    let tile = chosen.get("tile")?.as_array()?;
    if tile.len() != 2 {
        return None;
    }
    Some((tile[0].as_i64()? as i32, tile[1].as_i64()? as i32))
}

fn parse_object(
    value: &Value,
    source_path: &str,
    warnings: &mut Vec<String>,
    offset_x: i32,
    offset_y: i32,
) -> Option<PlacedObject> {
    let object_id = string_field(value, "id").unwrap_or("unnamed_object");
    let asset_id = string_field(value, "assetId").unwrap_or(object_id);
    let kind = object_kind_from_ids(object_id, asset_id).or_else(|| {
        warnings.push(format!(
            "{source_path}: object '{object_id}' / asset '{asset_id}' has no runtime ObjectKind mapping; skipped"
        ));
        None
    })?;

    let visual_rect = parse_rect(value.get("visualRect"));
    let collision_rect = parse_rect(value.get("collisionRect"));
    let interaction_rect = value
        .get("interactions")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|interaction| parse_rect(interaction.get("rect")));

    let Some(anchor_rect) = collision_rect.or(visual_rect).or(interaction_rect) else {
        warnings.push(format!(
            "{source_path}: object '{object_id}' has no visualRect/collisionRect/interaction rect; skipped"
        ));
        return None;
    };

    let mut footprint = kind.default_footprint();
    let (anchor_x, anchor_y, _, _) = anchor_rect;
    let migrated_anchor_x = anchor_x + offset_x;
    let migrated_anchor_y = anchor_y + offset_y;

    if let Some((x, y, w, h)) = visual_rect {
        footprint.visual_offset_x = x - anchor_x;
        footprint.visual_offset_y = y - anchor_y;
        footprint.visual_w = w.max(1);
        footprint.visual_h = h.max(1);
    }
    if let Some((x, y, w, h)) = collision_rect {
        footprint.collision_offset_x = x - anchor_x;
        footprint.collision_offset_y = y - anchor_y;
        footprint.collision_w = w.max(0);
        footprint.collision_h = h.max(0);
    }
    if let Some((x, y, w, h)) = interaction_rect {
        footprint.interaction_offset_x = x - anchor_x;
        footprint.interaction_offset_y = y - anchor_y;
        footprint.interaction_w = w.max(0);
        footprint.interaction_h = h.max(0);
    }

    if let Some(value) = value.get("blocksMovement").and_then(Value::as_bool) {
        footprint.blocks_movement = value;
    }
    if let Some(value) = value.get("occludesPlayer").and_then(Value::as_bool) {
        footprint.occludes_player = value;
    }
    if let Some(value) = value.get("fadeWhenPlayerBehind").and_then(Value::as_bool) {
        footprint.fade_when_player_behind = value;
        footprint.occludes_player = footprint.occludes_player || value;
    }

    Some(PlacedObject::with_id_and_footprint(
        ObjectId::from_stable_key(object_id),
        kind,
        migrated_anchor_x,
        migrated_anchor_y,
        footprint,
    ))
}

fn parse_rect(value: Option<&Value>) -> Option<(i32, i32, i32, i32)> {
    let rect = value?.as_array()?;
    if rect.len() < 2 {
        return None;
    }
    let x = rect[0].as_i64()? as i32;
    let y = rect[1].as_i64()? as i32;
    let w = rect.get(2).and_then(Value::as_i64).unwrap_or(1) as i32;
    let h = rect.get(3).and_then(Value::as_i64).unwrap_or(1) as i32;
    Some((x, y, w, h))
}

fn object_kind_from_ids(object_id: &str, asset_id: &str) -> Option<ObjectKind> {
    let text = normalize_code(&format!("{object_id} {asset_id}"));
    if text.contains("cave_entrance") || text.contains("cave_mouth") {
        Some(ObjectKind::CaveEntrance)
    } else if text.contains("stairs") || text.contains("stair") {
        Some(ObjectKind::Stairs)
    } else if text.contains("scarecrow") {
        Some(ObjectKind::Scarecrow)
    } else if text.contains("lamp_post") || text.contains("street_lamp") {
        Some(ObjectKind::Lamp)
    } else if text.contains("signboard") || text.contains("sign_post") {
        Some(ObjectKind::Sign)
    } else if text.contains("fallen_log") {
        Some(ObjectKind::Log)
    } else if text.contains("tree_stump") || text.contains("stump") {
        Some(ObjectKind::Stump)
    } else if text.contains("berry_bush")
        || text.contains("shrub_berry")
        || text.contains("bush")
        || text.contains("shrub")
    {
        Some(ObjectKind::Bush)
    } else if text.contains("mushroom") {
        Some(ObjectKind::Mushroom)
    } else if text.contains("wild_herb")
        || text.contains("wildflower")
        || text.contains("flower_patch")
        || text.contains("reed_patch")
        || text.contains("plant_patch")
        || text.contains("herb")
    {
        Some(ObjectKind::Herb)
    } else if text.contains("boulder") || text.contains("rock") {
        Some(ObjectKind::Boulder)
    } else if text.contains("ore") {
        Some(ObjectKind::OreNode)
    } else if text.contains("crate") || text.contains("shipping_box") {
        Some(ObjectKind::Crate)
    } else if text.contains("keg") {
        Some(ObjectKind::Keg)
    } else if text.contains("barrel") {
        Some(ObjectKind::Barrel)
    } else if text.contains("well") {
        Some(ObjectKind::Well)
    } else if text.contains("fence") {
        Some(ObjectKind::Fence)
    } else if text.contains("bench") {
        Some(ObjectKind::Bench)
    } else if text.contains("bar_counter") || text.contains("outside_bar") {
        Some(ObjectKind::Bar)
    } else if text.contains("door") || text.contains("entrance") {
        Some(ObjectKind::Door)
    } else if text.contains("bed") {
        Some(ObjectKind::Bed)
    } else if text.contains("stove") || text.contains("oven") || text.contains("fireplace") {
        Some(ObjectKind::Fireplace)
    } else if text.contains("greenhouse") {
        Some(ObjectKind::GreenhouseMarker)
    } else if text.contains("tree") {
        Some(ObjectKind::Tree)
    } else if text.contains("table") {
        Some(ObjectKind::Table)
    } else if text.contains("chair") || text.contains("seat") {
        Some(ObjectKind::Chair)
    } else {
        None
    }
}

fn scene_fill_for_kind(kind: SceneKind) -> TileKind {
    match kind {
        SceneKind::Exterior => TileKind::Grass,
        SceneKind::Interior => TileKind::Wall,
        SceneKind::Cave => TileKind::CaveWall,
    }
}

fn parse_tile(raw: &str) -> Option<TileKind> {
    TileKind::from_code(&normalize_code(raw))
}

fn default_height(tile: TileKind) -> u8 {
    match tile {
        TileKind::OceanDeep | TileKind::DeepWater => 18,
        TileKind::Water => 22,
        TileKind::OceanShallow
        | TileKind::ShallowWater
        | TileKind::RiverWater
        | TileKind::RiverMouthBlend
        | TileKind::ShoreFoam => 28,
        TileKind::Sand | TileKind::WetSand | TileKind::PebbleShore | TileKind::MudBank => 42,
        TileKind::Road | TileKind::StonePath | TileKind::MountainPath | TileKind::Dirt => 48,
        TileKind::Grass
        | TileKind::TallGrass
        | TileKind::TilledSoil
        | TileKind::WateredSoil
        | TileKind::Crop => 50,
        TileKind::WoodFloor
        | TileKind::PlankFloor
        | TileKind::StoneFloor
        | TileKind::BrickFloor => 52,
        TileKind::Bridge => 56,
        TileKind::Cliff | TileKind::MountainRock => 82,
        TileKind::Wall | TileKind::CaveWall => 78,
        TileKind::CaveFloor => 44,
        TileKind::GreenhouseZone => 50,
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let data = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str::<Value>(&data)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn infer_repo_root(pack_path: &Path) -> PathBuf {
    let mut cursor = pack_path
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

fn array_field<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("missing array field '{key}'"))
}

fn string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn int_at(values: &[Value], index: usize, label: &str) -> Result<i32, String> {
    values
        .get(index)
        .and_then(Value::as_i64)
        .map(|value| value as i32)
        .ok_or_else(|| format!("missing integer {label}"))
}

fn normalize_code(input: &str) -> String {
    let mut out = String::new();
    let mut previous_was_lower_or_digit = false;

    for ch in input.trim().chars() {
        if ch == '_' || ch == '-' || ch == ' ' {
            if !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            previous_was_lower_or_digit = false;
            continue;
        }

        if ch.is_ascii_uppercase() {
            if !out.is_empty() && previous_was_lower_or_digit && !out.ends_with('_') {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
            previous_was_lower_or_digit = false;
        } else {
            out.push(ch.to_ascii_lowercase());
            previous_was_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        }
    }

    out.trim_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pack_default_scene_overrides_legacy_farmstead() {
        let pack = json!({
            "defaultScene": "willowmere_outskirts_open_world",
            "editor": {"defaultScene": "farmstead"}
        });
        assert_eq!(
            preferred_default_scene_code(&pack),
            Some("willowmere_outskirts_open_world")
        );
    }

    #[test]
    fn published_shrub_family_maps_to_bush_runtime_kind() {
        assert_eq!(
            object_kind_from_ids("acceptance_shrub", "shrub_berry_01"),
            Some(ObjectKind::Bush)
        );
        assert_eq!(
            object_kind_from_ids("legacy_berry_bush", "berry_bush_variant_02"),
            Some(ObjectKind::Bush)
        );
    }

    #[test]
    fn authored_scene_loader_preserves_exact_asset_alias_for_runtime_canonicalization() {
        let scene = json!({
            "sceneId": "asset_identity_acceptance",
            "sceneKind": "exterior",
            "sceneSize": [4, 4],
            "layers": {
                "terrain": [
                    ["Grass", "Grass", "Grass", "Grass"],
                    ["Grass", "Grass", "Grass", "Grass"],
                    ["Grass", "Grass", "Grass", "Grass"],
                    ["Grass", "Grass", "Grass", "Grass"]
                ]
            },
            "objects": [{
                "id": "acceptance_oak",
                "assetId": "tree_oak_mature_04",
                "visualRect": [1, 0, 3, 4],
                "collisionRect": [2, 3, 1, 1]
            }],
            "spawns": [{"id": "player_default", "tile": [0, 0]}],
            "transitions": []
        });
        let mut warnings = Vec::new();
        let parsed = parse_scene(&scene, "asset_identity_acceptance.json", &mut warnings).unwrap();
        let object_id = ObjectId::from_stable_key("acceptance_oak");
        let asset_ref = parsed.map.object_asset_ref(object_id).expect("authored asset ref");
        assert_eq!(asset_ref.scene_asset_alias_id(), Some("tree_oak_mature_04"));
    }


    #[test]
    fn authored_scene_loader_preserves_explicit_structural_levels() {
        let scene = json!({
            "sceneId": "structural_level_acceptance",
            "sceneKind": "exterior",
            "sceneSize": [4, 4],
            "layers": {
                "terrain": [
                    ["Grass", "Grass", "Grass", "Grass"],
                    ["Grass", "Grass", "Grass", "Grass"],
                    ["Grass", "Grass", "Grass", "Grass"],
                    ["Grass", "Grass", "Grass", "Grass"]
                ],
                "structuralLevels": [
                    [0, 1, 2, 3],
                    [4, "auto", null, 0],
                    [0, 0, 0, 0],
                    [0, 0, 0, 0]
                ]
            },
            "objects": [],
            "spawns": [{"id": "player_default", "tile": [0, 0]}],
            "transitions": []
        });
        let mut warnings = Vec::new();
        let parsed = parse_scene(&scene, "structural_level_acceptance.json", &mut warnings)
            .expect("parse structural level acceptance scene");
        let (offset_x, offset_y) = scene_dimension_offset(4, 4);
        assert_eq!(parsed.map.get_structural_level(offset_x, offset_y), Some(0));
        assert_eq!(parsed.map.get_structural_level(offset_x + 1, offset_y), Some(1));
        assert_eq!(parsed.map.get_structural_level(offset_x + 2, offset_y), Some(2));
        assert_eq!(parsed.map.get_structural_level(offset_x + 3, offset_y), Some(3));
        assert_eq!(parsed.map.get_structural_level(offset_x, offset_y + 1), Some(4));
        assert_eq!(parsed.map.get_structural_level(offset_x + 1, offset_y + 1), None);
        assert_eq!(parsed.map.get_structural_level(offset_x + 2, offset_y + 1), None);
    }

    #[test]
    fn open_world_pack_can_disable_legacy_scene_requirements() {
        let pack = json!({"requiresLegacySceneSet": false});
        assert!(!legacy_scene_set_required(&pack));
        assert!(legacy_scene_set_required(&json!({})));
    }
}
