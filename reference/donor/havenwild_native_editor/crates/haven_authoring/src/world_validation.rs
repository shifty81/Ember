use std::collections::HashSet;

use haven_core::{GameWorld, ObjectKind, SceneId, SceneMap, TavernMap, TileKind, ZoneKind};
use haven_world::autotile::normalize_mask;

pub fn validate_world(world: &GameWorld) -> Vec<String> {
    let mut warnings = Vec::new();
    if world.scenes.is_empty() {
        warnings.push("world has no scenes".to_string());
        return warnings;
    }

    for scene in &world.scenes {
        if scene.transitions.is_empty() {
            warnings.push(format!("{} has no transitions", scene.name));
        }
        if !scene.is_cell_walkable(scene.spawn_x, scene.spawn_y) {
            warnings.push(format!("{} spawn is blocked", scene.name));
        }
        if scene.zone_at(scene.spawn_x, scene.spawn_y) == ZoneKind::None
            && matches!(
                scene.id.legacy_scene_id(),
                Some(
                    SceneId::TavernInterior
                        | SceneId::Cellar
                        | SceneId::GuestFloor
                        | SceneId::SouthField
                        | SceneId::CaveMouth
                        | SceneId::CaveDepths
                )
            )
        {
            warnings.push(format!("{} spawn is outside a gameplay zone", scene.name));
        }
        for transition in &scene.transitions {
            if let Some(target_scene) = world.scene_by_reference(&transition.target) {
                if !target_scene
                    .map
                    .get(transition.spawn_x, transition.spawn_y)
                    .walkable()
                {
                    warnings.push(format!(
                        "{} transition '{}' lands on blocked tile in {}",
                        scene.name, transition.label, target_scene.name
                    ));
                }
                let has_return = target_scene
                    .transitions
                    .iter()
                    .any(|return_transition| return_transition.target == scene.id);
                if !has_return {
                    warnings.push(format!(
                        "{} transition '{}' has no return link from {}",
                        scene.name, transition.label, target_scene.name
                    ));
                }
            } else {
                warnings.push(format!(
                    "{} transition '{}' targets missing scene {}",
                    scene.name,
                    transition.label,
                    transition.target.label()
                ));
            }
            if transition.w <= 0 || transition.h <= 0 {
                warnings.push(format!(
                    "{} transition '{}' has invalid size",
                    scene.name, transition.label
                ));
            }
            if !transition_has_walkable_origin(
                scene,
                transition.x,
                transition.y,
                transition.w,
                transition.h,
            ) {
                warnings.push(format!(
                    "{} transition '{}' origin is fully blocked",
                    scene.name, transition.label
                ));
            }
        }

        validate_object_footprints(scene, &mut warnings);
        warnings.extend(validate_scene_autotile_overrides(scene));

        match scene.id.legacy_scene_id() {
            Some(SceneId::TavernInterior) => {
                require_zone(scene, ZoneKind::Tavern, "main tavern room", &mut warnings);
                require_zone(scene, ZoneKind::Kitchen, "kitchen", &mut warnings);
                require_object(scene, ObjectKind::Bar, "bar", &mut warnings);
                require_object(scene, ObjectKind::Table, "table", &mut warnings);
                require_object(scene, ObjectKind::Chair, "chair", &mut warnings);
            }
            Some(SceneId::Cellar) => {
                require_zone(scene, ZoneKind::Cellar, "cellar", &mut warnings);
                require_object(scene, ObjectKind::Keg, "keg/storage", &mut warnings);
            }
            Some(SceneId::GuestFloor) => {
                require_zone(scene, ZoneKind::GuestRoom, "guest room", &mut warnings);
                require_object(scene, ObjectKind::Bed, "guest bed", &mut warnings);
            }
            Some(SceneId::Farmstead | SceneId::SouthField) => {
                if count_tiles(scene, TileKind::TilledSoil) == 0 {
                    warnings.push(format!("{} has no tilled crop soil", scene.name));
                }
                if count_zones(scene, ZoneKind::Field) == 0 {
                    warnings.push(format!("{} has no field zone", scene.name));
                }
            }
            Some(SceneId::CaveMouth | SceneId::CaveDepths) => {
                require_zone(scene, ZoneKind::Cave, "cave", &mut warnings);
                require_object(scene, ObjectKind::OreNode, "ore node", &mut warnings);
            }
            _ => {}
        }
    }

    warnings
}

pub fn validate_scene_autotile_overrides(scene: &SceneMap) -> Vec<String> {
    let mut warnings = Vec::new();
    let mut seen = HashSet::new();
    for entry in &scene.autotile_overrides {
        if TavernMap::idx(entry.x, entry.y).is_none() {
            warnings.push(format!(
                "{} autotile override at {}, {} is outside scene bounds",
                scene.name, entry.x, entry.y
            ));
            continue;
        }
        if !seen.insert((entry.x, entry.y)) {
            warnings.push(format!(
                "{} has duplicate autotile overrides at {}, {}",
                scene.name, entry.x, entry.y
            ));
        }
        let tile = scene.map.get(entry.x, entry.y);
        if tile.autotile_group() != Some(entry.group) {
            warnings.push(format!(
                "{} autotile override at {}, {} targets {} but cell contains {}",
                scene.name,
                entry.x,
                entry.y,
                entry.group.label(),
                tile.label()
            ));
        }
        if normalize_mask(entry.mask) != entry.mask {
            warnings.push(format!(
                "{} autotile override at {}, {} has invalid diagonal mask {:02x}",
                scene.name, entry.x, entry.y, entry.mask
            ));
        }
    }
    warnings
}

fn validate_object_footprints(scene: &SceneMap, warnings: &mut Vec<String>) {
    for (index, object) in scene.map.objects.iter().enumerate() {
        let (cx, cy, cw, ch) = object.collision_rect();
        if cw < 0 || ch < 0 {
            warnings.push(format!(
                "{} object {} has invalid negative collision footprint",
                scene.name,
                object.kind.label()
            ));
        }
        if cw > 0 && ch > 0 {
            for y in cy..cy + ch {
                for x in cx..cx + cw {
                    if TavernMap::idx(x, y).is_none() {
                        warnings.push(format!(
                            "{} object {} collision extends outside map at {},{}",
                            scene.name,
                            object.kind.label(),
                            x,
                            y
                        ));
                    }
                }
            }
        }
        let (ix, iy, iw, ih) = object.interaction_rect();
        if iw > 0 && ih > 0 {
            let mut has_in_bounds_interaction = false;
            for y in iy..iy + ih {
                for x in ix..ix + iw {
                    has_in_bounds_interaction |= TavernMap::idx(x, y).is_some();
                }
            }
            if !has_in_bounds_interaction {
                warnings.push(format!(
                    "{} object {} has no in-bounds interaction tile",
                    scene.name,
                    object.kind.label()
                ));
            }
        }
        for other in scene.map.objects.iter().skip(index + 1) {
            if footprint_rects_overlap(object.collision_rect(), other.collision_rect()) {
                warnings.push(format!(
                    "{} object collision overlap: {} and {}",
                    scene.name,
                    object.kind.label(),
                    other.kind.label()
                ));
            }
        }
    }
}

fn footprint_rects_overlap(a: (i32, i32, i32, i32), b: (i32, i32, i32, i32)) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    aw > 0
        && ah > 0
        && bw > 0
        && bh > 0
        && ax < bx + bw
        && ax + aw > bx
        && ay < by + bh
        && ay + ah > by
}

fn require_zone(scene: &SceneMap, zone: ZoneKind, label: &str, warnings: &mut Vec<String>) {
    if count_zones(scene, zone) == 0 {
        warnings.push(format!("{} has no {label} zone", scene.name));
    }
}

fn require_object(scene: &SceneMap, object: ObjectKind, label: &str, warnings: &mut Vec<String>) {
    if scene.map.objects.iter().all(|placed| placed.kind != object) {
        warnings.push(format!("{} has no {label}", scene.name));
    }
}

fn count_zones(scene: &SceneMap, zone: ZoneKind) -> usize {
    scene
        .zones
        .iter()
        .filter(|candidate| **candidate == zone)
        .count()
}

fn count_tiles(scene: &SceneMap, tile: TileKind) -> usize {
    scene
        .map
        .tiles
        .iter()
        .filter(|candidate| **candidate == tile)
        .count()
}

fn transition_has_walkable_origin(scene: &SceneMap, x: i32, y: i32, w: i32, h: i32) -> bool {
    for ty in y..y + h {
        for tx in x..x + w {
            if scene.map.get(tx, ty).walkable() {
                return true;
            }
        }
    }
    false
}
