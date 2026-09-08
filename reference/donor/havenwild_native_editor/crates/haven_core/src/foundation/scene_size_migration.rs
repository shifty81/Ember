use super::starter_scene_rules::starter_biome;
use super::{
    SceneBiome, SceneKind, TavernMap, TileKind, Transition, ZoneKind, LEGACY_MAP_H, LEGACY_MAP_W,
    MAP_H, MAP_W,
};
use crate::ProjectSceneId;

pub const fn scene_dimension_offset(source_w: usize, source_h: usize) -> (i32, i32) {
    (
        MAP_W.saturating_sub(source_w) as i32 / 2,
        MAP_H.saturating_sub(source_h) as i32 / 2,
    )
}

pub(crate) const fn legacy_scene_offset() -> (i32, i32) {
    scene_dimension_offset(LEGACY_MAP_W, LEGACY_MAP_H)
}

pub(crate) fn parse_tavern_map_dimensions(header: &str) -> Result<(usize, usize), String> {
    let parts: Vec<_> = header.split_whitespace().collect();
    if parts.len() != 4 || parts[0] != "tavern_map" || parts[1] != "1" {
        return Err(format!("unsupported map header: {header}"));
    }
    let width = parts[2]
        .parse::<usize>()
        .map_err(|_| format!("invalid map width in header: {header}"))?;
    let height = parts[3]
        .parse::<usize>()
        .map_err(|_| format!("invalid map height in header: {header}"))?;
    if width == 0 || height == 0 || width > MAP_W || height > MAP_H {
        return Err(format!(
            "unsupported map dimensions {width}x{height}; runtime supports up to {MAP_W}x{MAP_H}"
        ));
    }
    Ok((width, height))
}

pub(crate) fn scene_fill_for_kind(kind: SceneKind) -> TileKind {
    match kind {
        SceneKind::Exterior => TileKind::Grass,
        SceneKind::Interior => TileKind::Wall,
        SceneKind::Cave => TileKind::CaveWall,
    }
}

pub(crate) fn migrate_legacy_point(x: i32, y: i32) -> (i32, i32) {
    let (offset_x, offset_y) = legacy_scene_offset();
    (x + offset_x, y + offset_y)
}

pub(crate) fn migrate_legacy_transition(mut transition: Transition) -> Transition {
    let (offset_x, offset_y) = legacy_scene_offset();
    transition.x += offset_x;
    transition.y += offset_y;
    transition.spawn_x += offset_x;
    transition.spawn_y += offset_y;
    transition
}

pub(crate) fn center_legacy_zone_grid(source: Vec<ZoneKind>) -> Vec<ZoneKind> {
    if MAP_W == LEGACY_MAP_W && MAP_H == LEGACY_MAP_H {
        return source;
    }
    let mut output = vec![ZoneKind::None; MAP_W * MAP_H];
    let (offset_x, offset_y) = legacy_scene_offset();
    for y in 0..LEGACY_MAP_H {
        for x in 0..LEGACY_MAP_W {
            let source_index = y * MAP_W + x;
            let target_x = x as i32 + offset_x;
            let target_y = y as i32 + offset_y;
            if source_index < source.len() {
                if let Some(target_index) = TavernMap::idx(target_x, target_y) {
                    output[target_index] = source[source_index];
                }
            }
        }
    }
    output
}

pub(crate) fn default_biome_for_project_scene(id: &ProjectSceneId, kind: SceneKind) -> SceneBiome {
    id.legacy_scene_id()
        .map(|legacy| starter_biome(legacy, kind))
        .unwrap_or_else(|| match kind {
            SceneKind::Exterior | SceneKind::Interior => SceneBiome::Temperate,
            SceneKind::Cave => SceneBiome::Cave,
        })
}
