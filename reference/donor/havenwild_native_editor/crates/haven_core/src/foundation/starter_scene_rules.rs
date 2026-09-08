use super::*;

pub(super) fn tile_index(tile: TileKind) -> usize {
    TileKind::ALL
        .iter()
        .position(|candidate| *candidate == tile)
        .unwrap_or(0)
}

pub(super) fn default_tile_rules() -> Vec<TileInteraction> {
    TileKind::ALL
        .iter()
        .map(|tile| default_tile_rule(*tile))
        .collect()
}

pub(super) fn default_tile_rule(tile: TileKind) -> TileInteraction {
    tile.default_interaction()
}

pub(super) fn starter_biome(id: SceneId, kind: SceneKind) -> SceneBiome {
    match kind {
        SceneKind::Cave => SceneBiome::Cave,
        SceneKind::Interior => SceneBiome::Temperate,
        SceneKind::Exterior => match id {
            SceneId::Farmstead => SceneBiome::Coastal,
            SceneId::NorthRoad => SceneBiome::Highlands,
            SceneId::SouthField => SceneBiome::Coastal,
            SceneId::EastWoods => SceneBiome::Temperate,
            _ => SceneBiome::Temperate,
        },
    }
}

pub(super) fn layered_scene_for(
    id: SceneId,
    seed: u32,
) -> Option<addons::worldgen_layered::GeneratedSceneLayers> {
    match id {
        SceneId::Farmstead => Some(addons::worldgen_layered::generate_starter_island_layers(
            seed,
        )),
        SceneId::SouthField => Some(addons::worldgen_layered::generate_south_field_layers(seed)),
        SceneId::EastWoods => Some(addons::worldgen_layered::generate_woods_layers(seed)),
        _ => None,
    }
}

pub(super) fn ensure_scene_access(
    map: &mut TavernMap,
    zones: &mut [ZoneKind],
    id: SceneId,
    kind: SceneKind,
    spawn_x: i32,
    spawn_y: i32,
    transitions: &[Transition],
) {
    let access_tile = match kind {
        SceneKind::Exterior => TileKind::Road,
        SceneKind::Interior => TileKind::WoodFloor,
        SceneKind::Cave => TileKind::CaveFloor,
    };

    carve_access_rect(map, spawn_x - 1, spawn_y - 1, 3, 3, access_tile);
    set_zone_if_required(zones, id, spawn_x, spawn_y);

    for transition in transitions {
        carve_access_rect(
            map,
            transition.x,
            transition.y,
            transition.w.max(1),
            transition.h.max(1),
            access_tile,
        );
    }

    for &(legacy_x, legacy_y) in extra_landing_points(id) {
        let (offset_x, offset_y) = scene_dimension_offset(LEGACY_MAP_W, LEGACY_MAP_H);
        let x = legacy_x + offset_x;
        let y = legacy_y + offset_y;
        carve_access_rect(map, x - 1, y - 1, 3, 3, access_tile);
    }
}

pub(super) fn carve_access_rect(
    map: &mut TavernMap,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    tile: TileKind,
) {
    for ty in y..y + h {
        for tx in x..x + w {
            if TavernMap::idx(tx, ty).is_some() {
                map.set(tx, ty, tile);
                map.set_height(tx, ty, 52);
            }
        }
    }
}

pub(super) fn set_zone_if_required(zones: &mut [ZoneKind], id: SceneId, x: i32, y: i32) {
    let zone = match id {
        SceneId::TavernInterior => ZoneKind::Tavern,
        SceneId::Cellar => ZoneKind::Cellar,
        SceneId::GuestFloor => ZoneKind::GuestRoom,
        SceneId::SouthField => ZoneKind::Field,
        SceneId::CaveMouth | SceneId::CaveDepths => ZoneKind::Cave,
        _ => ZoneKind::None,
    };
    if zone == ZoneKind::None {
        return;
    }
    if let Some(idx) = TavernMap::idx(x, y) {
        zones[idx] = zone;
    }
}

pub(super) fn extra_landing_points(id: SceneId) -> &'static [(i32, i32)] {
    match id {
        SceneId::Farmstead => &[(24, 29)],
        _ => &[],
    }
}

pub(super) fn starter_zones(id: SceneId) -> Vec<ZoneKind> {
    let mut zones = vec![ZoneKind::None; MAP_W * MAP_H];
    let mut set = |x: i32, y: i32, zone: ZoneKind| {
        if let Some(idx) = TavernMap::idx(x, y) {
            zones[idx] = zone;
        }
    };

    match id {
        SceneId::Farmstead => {
            for y in 20..28 {
                for x in 5..18 {
                    set(x, y, ZoneKind::Field);
                }
            }
            for y in 20..23 {
                for x in 10..15 {
                    set(x, y, ZoneKind::Greenhouse);
                }
            }
        }
        SceneId::TavernInterior => {
            for y in 7..23 {
                for x in 9..28 {
                    set(x, y, ZoneKind::Tavern);
                }
            }
            for y in 6..12 {
                for x in 29..38 {
                    set(x, y, ZoneKind::Kitchen);
                }
            }
        }
        SceneId::Cellar => {
            for y in 8..24 {
                for x in 9..38 {
                    set(x, y, ZoneKind::Cellar);
                }
            }
        }
        SceneId::GuestFloor => {
            for y in 8..22 {
                for x in 11..36 {
                    set(x, y, ZoneKind::GuestRoom);
                }
            }
        }
        SceneId::CaveMouth | SceneId::CaveDepths => {
            for y in 0..MAP_H as i32 {
                for x in 0..MAP_W as i32 {
                    set(x, y, ZoneKind::Cave);
                }
            }
        }
        SceneId::SouthField => {
            for y in 7..26 {
                for x in 4..42 {
                    set(x, y, ZoneKind::Field);
                }
            }
        }
        _ => {}
    }
    zones
}

pub(super) fn starter_transitions(id: SceneId) -> Vec<Transition> {
    match id {
        SceneId::Farmstead => vec![
            transition(23, 12, 2, 1, SceneId::TavernInterior, 23, 23, "Tavern Door"),
            transition(0, 14, 1, 2, SceneId::NorthRoad, 45, 15, "West Road"),
            transition(47, 14, 1, 2, SceneId::EastWoods, 2, 15, "East Road"),
            transition(22, 31, 4, 1, SceneId::SouthField, 24, 3, "South Field"),
            transition(43, 7, 2, 2, SceneId::CaveMouth, 8, 22, "Cave Entrance"),
        ],
        SceneId::TavernInterior => vec![
            transition(23, 24, 2, 1, SceneId::Farmstead, 23, 13, "Outside"),
            transition(9, 22, 2, 2, SceneId::Cellar, 10, 21, "Cellar Stairs"),
            transition(35, 22, 2, 2, SceneId::GuestFloor, 34, 20, "Guest Floor"),
        ],
        SceneId::Cellar => vec![transition(
            10,
            22,
            2,
            2,
            SceneId::TavernInterior,
            9,
            21,
            "Tavern",
        )],
        SceneId::GuestFloor => vec![transition(
            34,
            21,
            2,
            2,
            SceneId::TavernInterior,
            35,
            21,
            "Tavern",
        )],
        SceneId::NorthRoad => vec![
            transition(47, 14, 1, 3, SceneId::Farmstead, 2, 15, "Estate"),
            transition(0, 14, 1, 3, SceneId::Farmstead, 45, 15, "Future Town"),
        ],
        SceneId::SouthField => vec![transition(
            22,
            0,
            4,
            1,
            SceneId::Farmstead,
            24,
            29,
            "Estate",
        )],
        SceneId::EastWoods => vec![
            transition(0, 14, 1, 3, SceneId::Farmstead, 45, 15, "Estate"),
            transition(47, 14, 1, 3, SceneId::CaveMouth, 8, 22, "Cave Trail"),
        ],
        SceneId::CaveMouth => vec![
            transition(8, 23, 2, 2, SceneId::Farmstead, 43, 8, "Outside"),
            transition(0, 14, 1, 3, SceneId::EastWoods, 45, 15, "Woods Trail"),
            transition(40, 14, 2, 3, SceneId::CaveDepths, 6, 23, "Deeper Cave"),
        ],
        SceneId::CaveDepths => vec![transition(
            4,
            22,
            2,
            3,
            SceneId::CaveMouth,
            39,
            15,
            "Cave Mouth",
        )],
    }
}
