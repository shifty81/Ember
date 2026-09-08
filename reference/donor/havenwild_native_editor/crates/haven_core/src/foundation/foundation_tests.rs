use super::*;
use crate::ProjectSceneId;

#[test]
fn object_identity_survives_map_line_save_round_trip() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    let id = map
        .place_object(ObjectKind::Table, 8, 8)
        .expect("place object");
    let decoded = TavernMap::deserialize_lines(&map.serialize_lines()).expect("decode map");
    assert_eq!(decoded.object_id_at(8, 8), Some(id));
    assert!(decoded.object(id).is_some());
}

#[test]
fn multi_tile_stamp_round_trips_and_blocks_collision_cells() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    let footprint = ObjectFootprint {
        visual_offset_x: -1,
        visual_offset_y: -2,
        visual_w: 3,
        visual_h: 3,
        collision_offset_x: -1,
        collision_offset_y: 0,
        collision_w: 3,
        collision_h: 1,
        interaction_offset_x: -1,
        interaction_offset_y: -1,
        interaction_w: 3,
        interaction_h: 2,
        blocks_movement: true,
        occludes_player: true,
        fade_when_player_behind: true,
    };
    let id = map
        .place_stamp(PlacedStamp::new(
            "home_tavern_mountain_entrance",
            12,
            12,
            footprint,
        ))
        .expect("place stamp");
    assert!(!map.is_cell_walkable(11, 12));
    assert_eq!(map.stamp_id_at(12, 10), Some(id));

    let decoded = TavernMap::deserialize_lines(&map.serialize_lines()).expect("decode map");
    let stamp = decoded.stamp(id).expect("decoded stamp");
    assert_eq!(stamp.stamp_key, "home_tavern_mountain_entrance");
    assert_eq!(stamp.footprint, footprint);
    assert!(!decoded.is_cell_walkable(13, 12));
}

#[test]
fn legacy_48x32_map_centers_into_expanded_runtime_scene() {
    let mut legacy = format!("tavern_map 1 {LEGACY_MAP_W} {LEGACY_MAP_H}\n");
    for y in 0..LEGACY_MAP_H {
        legacy.push_str("row");
        for x in 0..LEGACY_MAP_W {
            legacy.push(' ');
            legacy.push_str(if x == 6 && y == 5 { "dirt" } else { "grass" });
        }
        legacy.push('\n');
    }
    legacy.push_str("object chair 6 6\n");

    let decoded = TavernMap::deserialize_lines(&legacy).expect("decode legacy 48x32 map");
    let (offset_x, offset_y) = scene_dimension_offset(LEGACY_MAP_W, LEGACY_MAP_H);
    assert_eq!(decoded.get(6 + offset_x, 5 + offset_y), TileKind::Dirt);
    assert!(decoded.object_id_at(6 + offset_x, 6 + offset_y).is_some());
    assert_eq!(decoded.tiles.len(), MAP_W * MAP_H);
}

#[test]
fn expanded_starter_templates_keep_generated_objects_in_bounds() {
    for scene_id in SceneId::ALL {
        let map = TavernMap::starter_for(scene_id);
        for object in &map.objects {
            for (x, y, w, h) in [
                object.visual_rect(),
                object.collision_rect(),
                object.interaction_rect(),
            ] {
                if w <= 0 || h <= 0 {
                    continue;
                }
                assert!(
                    TavernMap::idx(x, y).is_some()
                        && TavernMap::idx(x + w - 1, y + h - 1).is_some(),
                    "{} {} footprint leaves expanded scene: {}",
                    scene_id.label(),
                    object.kind.label(),
                    object.placement_label()
                );
            }
        }
    }
}

#[test]
fn legacy_object_lines_receive_nonzero_identity() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.place_object(ObjectKind::Chair, 6, 6)
        .expect("place object");
    let legacy = map
        .serialize_lines()
        .lines()
        .map(|line| {
            if line.starts_with("object obj_") {
                let mut parts = line.split_whitespace().collect::<Vec<_>>();
                parts.remove(1);
                parts.join(" ")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let decoded = TavernMap::deserialize_lines(&legacy).expect("decode legacy map");
    let id = decoded.object_id_at(6, 6).expect("migrated object id");
    assert!(id.is_assigned());
}

#[test]
fn terrain_consumer_queries_use_central_gameplay_profiles() {
    let mut map = TavernMap::new();
    map.set(4, 4, TileKind::Grass);
    map.set(5, 4, TileKind::WateredSoil);
    map.set(6, 4, TileKind::ShallowWater);
    map.set(7, 4, TileKind::Cliff);

    assert_eq!(map.terrain_movement_cost_at(4, 4), Some(100));
    assert_eq!(
        map.terrain_farming_class_at(5, 4),
        Some(FarmingClass::Watered)
    );
    assert_eq!(
        map.terrain_footstep_surface_at(4, 4),
        Some(FootstepSurface::Grass)
    );
    assert!(map.terrain_is_fishable_at(6, 4));
    assert!(!map.terrain_is_fishable_at(4, 4));
    assert!(map.terrain_allows_standard_building_at(4, 4));
    assert!(!map.terrain_allows_standard_building_at(6, 4));
    assert!(!map.terrain_allows_standard_building_at(7, 4));
}

#[test]
fn water_traversal_capabilities_preserve_object_and_stamp_blocking() {
    let mut map = TavernMap::new();
    map.set(4, 4, TileKind::ShallowWater);
    map.set(5, 4, TileKind::DeepWater);

    assert!(
        map.collision_at_with_water_access(4, 4, false, false)
            .blocked
    );
    assert!(
        !map.collision_at_with_water_access(4, 4, true, false)
            .blocked
    );
    assert!(
        map.collision_at_with_water_access(5, 4, true, false)
            .blocked
    );
    assert!(!map.collision_at_with_water_access(5, 4, true, true).blocked);
}

#[test]
fn water_building_support_requires_foundation_or_bridge() {
    let mut map = TavernMap::new();
    map.set(4, 4, TileKind::Grass);
    map.set(5, 4, TileKind::ShallowWater);
    map.set(6, 4, TileKind::DeepWater);
    map.set(7, 4, TileKind::Cliff);

    assert!(map.terrain_allows_building_support_at(4, 4, BuildingSupport::Standard));
    assert!(!map.terrain_allows_standard_building_at(5, 4));
    assert!(map.terrain_allows_foundation_at(5, 4));
    assert!(map.terrain_allows_bridge_at(5, 4));
    assert!(map.terrain_allows_foundation_at(6, 4));
    assert!(map.terrain_allows_bridge_at(6, 4));
    assert!(!map.terrain_allows_foundation_at(7, 4));
    assert!(!map.terrain_allows_bridge_at(7, 4));
}

#[test]
fn structural_levels_round_trip_without_rewriting_geological_height() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set_height(7, 9, 223);
    map.set_structural_level(7, 9, Some(MAX_STRUCTURAL_LEVEL));
    map.set_structural_level(8, 9, Some(0));

    let decoded = TavernMap::deserialize_lines(&map.serialize_lines()).expect("decode map");

    assert_eq!(decoded.get_height(7, 9), 223);
    assert_eq!(decoded.get_structural_level(7, 9), Some(MAX_STRUCTURAL_LEVEL));
    assert_eq!(decoded.get_structural_level(8, 9), Some(0));
    assert_eq!(decoded.get_structural_level(9, 9), None);
}

#[test]
fn legacy_maps_without_structural_section_remain_auto_classified() {
    let map = TavernMap::empty_with(TileKind::Grass);
    let serialized = map.serialize_lines();
    let legacy = serialized
        .split_once("structural_levels\n")
        .map(|(prefix, suffix)| {
            let remaining = suffix.lines().skip(MAP_H).collect::<Vec<_>>().join("\n");
            if remaining.is_empty() {
                prefix.to_string()
            } else {
                format!("{prefix}{remaining}\n")
            }
        })
        .expect("structural section");

    let decoded = TavernMap::deserialize_lines(&legacy).expect("decode legacy map");

    assert_eq!(decoded.get_structural_level(4, 4), None);
    assert_eq!(
        decoded.structural_level_storage(4, 4),
        STRUCTURAL_LEVEL_AUTO
    );
}

#[test]
fn interior_visual_mask_keeps_room_and_hides_unused_backing_grid() {
    let scene = SceneMap::starter(SceneId::TavernInterior, SceneKind::Interior, 23, 23);
    let (offset_x, offset_y) = scene_dimension_offset(LEGACY_MAP_W, LEGACY_MAP_H);

    assert!(!scene.is_renderable_cell(0, 0));
    assert!(!scene.is_renderable_cell(23, 13));
    assert!(scene.is_renderable_cell(23 + offset_x, 13 + offset_y));
    assert!(scene.is_renderable_cell(6 + offset_x, 10 + offset_y));

    let bounds = scene.renderable_bounds().expect("starter interior bounds");
    assert!(bounds.0 > 0 && bounds.1 > 0);
    assert!(bounds.2 < MAP_W as i32 - 1 && bounds.3 < MAP_H as i32 - 1);
}

#[test]
fn exterior_visual_mask_keeps_complete_storage_grid() {
    let scene = SceneMap::blank(
        ProjectSceneId::new("mask_test_exterior"),
        "Mask Test",
        SceneKind::Exterior,
        SceneBiome::Temperate,
    );
    assert!(scene.is_renderable_cell(0, 0));
    assert!(scene.is_renderable_cell(MAP_W as i32 - 1, MAP_H as i32 - 1));
    assert_eq!(
        scene.renderable_bounds(),
        Some((0, 0, MAP_W as i32 - 1, MAP_H as i32 - 1))
    );
}
