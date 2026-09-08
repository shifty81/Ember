use super::*;

#[test]
fn sand_on_grass_corner_uses_tiled_tuple_sampling() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(4, 4, TileKind::Sand);

    let entry = lpc_mapped_terrain_entry_for_map(&map, 4, 4)
        .expect("sand-on-grass corner should use mapped LPC corner tuple");
    assert!(entry.is_mixed);
}

#[test]
fn sand_on_grass_corner_marks_adjacent_cells_as_covered() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(4, 4, TileKind::Sand);

    assert!(lpc_mapped_terrain_transition_covers_map_cell(&map, 4, 4));
    assert!(lpc_mapped_terrain_transition_covers_map_cell(&map, 3, 3));
    assert!(!lpc_mapped_terrain_transition_covers_map_cell(&map, 0, 0));
}

#[test]
fn pure_mapped_cells_are_owned_even_when_no_mixed_transition_is_present() {
    let map = TavernMap::empty_with(TileKind::DeepWater);

    assert!(lpc_mapped_terrain_owns_map_cell(&map, 4, 4));
    assert!(!lpc_mapped_terrain_transition_covers_map_cell(&map, 4, 4));
}

#[test]
fn pure_mapped_fills_use_the_same_lpc_terrain_source() {
    let map = TavernMap::empty_with(TileKind::Grass);

    let entry = lpc_mapped_terrain_entry_for_map(&map, 4, 4)
        .expect("pure grass fill should use mapped LPC terrain source");

    assert!(!entry.is_mixed);
    assert!(!lpc_mapped_terrain_transition_covers_map_cell(&map, 4, 4));
}

#[test]
fn missing_mapped_water_tuples_fallback_to_lpc_owned_fills() {
    let manifest = lpc_mapped_terrain_manifest().expect("mapped manifest");
    let fallback = manifest
        .entry_for_corners_with_water_frame(
            [
                LpcMappedTerrainMaterial::Sand,
                LpcMappedTerrainMaterial::WaterShallowsSand,
                LpcMappedTerrainMaterial::Water,
                LpcMappedTerrainMaterial::WaterDeep,
            ],
            0x800,
            12,
        )
        .expect("missing but mapped shore/depth tuple should not fall back to legacy terrain");

    assert!(fallback.is_mixed);
}

#[test]
fn missing_mixed_tuple_fallbacks_stay_stable_across_water_frames() {
    let manifest = lpc_mapped_terrain_manifest().expect("mapped manifest");
    let corners = [
        LpcMappedTerrainMaterial::Sand,
        LpcMappedTerrainMaterial::WaterShallowsSand,
        LpcMappedTerrainMaterial::Water,
        LpcMappedTerrainMaterial::WaterDeep,
    ];
    let frame_a = manifest
        .entry_for_corners_with_water_frame(corners, 0, 0)
        .expect("missing mixed tuple fallback frame");
    let frame_b = manifest
        .entry_for_corners_with_water_frame(corners, 0, 24)
        .expect("missing mixed tuple fallback frame");

    assert_eq!(frame_a.rect, frame_b.rect);
    assert!(frame_a.is_mixed);
}

#[test]
fn mapped_water_tuple_fallback_keeps_cells_owned() {
    let mut map = TavernMap::empty_with(TileKind::DeepWater);
    map.set(4, 4, TileKind::Sand);
    map.set(5, 4, TileKind::ShoreFoam);
    map.set(4, 5, TileKind::ShallowWater);

    assert!(lpc_mapped_terrain_entry_for_map(&map, 4, 4).is_some());
    assert!(lpc_mapped_terrain_owns_map_cell(&map, 4, 4));
}

#[test]
fn missing_sand_water_tuples_do_not_fallback_to_square_land() {
    assert_eq!(
        fallback_material_for_missing_tuple([
            LpcMappedTerrainMaterial::Sand,
            LpcMappedTerrainMaterial::Sand,
            LpcMappedTerrainMaterial::Water,
            LpcMappedTerrainMaterial::WaterDeep,
        ]),
        LpcMappedTerrainMaterial::WaterShallowsSand
    );
}

#[test]
fn missing_dirt_water_tuples_do_not_fallback_to_square_land() {
    assert_eq!(
        fallback_material_for_missing_tuple([
            LpcMappedTerrainMaterial::DirtBrown,
            LpcMappedTerrainMaterial::DirtBrown,
            LpcMappedTerrainMaterial::Water,
            LpcMappedTerrainMaterial::WaterDeep,
        ]),
        LpcMappedTerrainMaterial::WaterShallowsDirt
    );
}

#[test]
fn missing_grass_water_tuples_do_not_fallback_to_square_land() {
    assert_eq!(
        fallback_material_for_missing_tuple([
            LpcMappedTerrainMaterial::Grass,
            LpcMappedTerrainMaterial::Grass,
            LpcMappedTerrainMaterial::Water,
            LpcMappedTerrainMaterial::WaterDeep,
        ]),
        LpcMappedTerrainMaterial::Water
    );
}

#[test]
fn missing_pebble_shore_water_tuples_do_not_fallback_to_square_land() {
    assert_eq!(
        fallback_material_for_missing_tuple([
            LpcMappedTerrainMaterial::StoneTan,
            LpcMappedTerrainMaterial::StoneTan,
            LpcMappedTerrainMaterial::Water,
            LpcMappedTerrainMaterial::WaterDeep,
        ]),
        LpcMappedTerrainMaterial::WaterShallowsDirt
    );
}

#[test]
fn missing_road_water_tuples_do_not_fallback_to_square_land() {
    assert_eq!(
        fallback_material_for_missing_tuple([
            LpcMappedTerrainMaterial::DirtTan,
            LpcMappedTerrainMaterial::DirtTan,
            LpcMappedTerrainMaterial::Water,
            LpcMappedTerrainMaterial::WaterDeep,
        ]),
        LpcMappedTerrainMaterial::WaterShallowsDirt
    );
}

#[test]
fn missing_mountain_path_water_tuples_do_not_fallback_to_square_land() {
    assert_eq!(
        fallback_material_for_missing_tuple([
            LpcMappedTerrainMaterial::DirtRoots,
            LpcMappedTerrainMaterial::DirtRoots,
            LpcMappedTerrainMaterial::Water,
            LpcMappedTerrainMaterial::WaterDeep,
        ]),
        LpcMappedTerrainMaterial::WaterShallowsDirt
    );
}

#[test]
fn deep_water_land_tuples_use_stable_shore_fallbacks_instead_of_stale_exacts() {
    assert!(should_force_shore_fallback([
        LpcMappedTerrainMaterial::Sand,
        LpcMappedTerrainMaterial::Sand,
        LpcMappedTerrainMaterial::Water,
        LpcMappedTerrainMaterial::WaterDeep,
    ]));
    assert!(!should_force_shore_fallback([
        LpcMappedTerrainMaterial::Sand,
        LpcMappedTerrainMaterial::WaterShallowsSand,
        LpcMappedTerrainMaterial::Water,
        LpcMappedTerrainMaterial::WaterDeep,
    ]));
}

#[test]
fn wet_sand_aliases_to_the_canonical_sand_source() {
    assert_eq!(
        mapped_terrain_name(TileKind::WetSand),
        Some(LpcMappedTerrainMaterial::Sand)
    );
}

#[test]
fn natural_terrain_aliases_share_the_editor_and_client_mapping() {
    assert_eq!(
        mapped_terrain_name(TileKind::Road),
        Some(LpcMappedTerrainMaterial::DirtTan)
    );
    assert_eq!(
        mapped_terrain_name(TileKind::StonePath),
        Some(LpcMappedTerrainMaterial::StonePath)
    );
    assert_eq!(
        mapped_terrain_name(TileKind::MountainPath),
        Some(LpcMappedTerrainMaterial::DirtRoots)
    );
    assert_eq!(
        mapped_terrain_name(TileKind::TilledSoil),
        Some(LpcMappedTerrainMaterial::Soil)
    );
    assert_eq!(
        mapped_terrain_name(TileKind::RiverMouthBlend),
        Some(LpcMappedTerrainMaterial::WaterShallowsDirt)
    );
}

#[test]
fn stone_path_is_semantically_distinct_from_pebble_shore() {
    assert_eq!(
        mapped_terrain_name(TileKind::PebbleShore),
        Some(LpcMappedTerrainMaterial::StoneTan)
    );
    assert_eq!(
        mapped_terrain_name(TileKind::StonePath),
        Some(LpcMappedTerrainMaterial::StonePath)
    );
}

#[test]
fn water_depth_semantics_use_authored_depth_pair() {
    assert_eq!(
        mapped_terrain_name(TileKind::ShallowWater),
        Some(LpcMappedTerrainMaterial::Water)
    );
    assert_eq!(
        mapped_terrain_name(TileKind::OceanShallow),
        Some(LpcMappedTerrainMaterial::Water)
    );
    assert_eq!(
        mapped_terrain_name(TileKind::DeepWater),
        Some(LpcMappedTerrainMaterial::WaterDeep)
    );
}

#[test]
fn pure_water_fills_can_cycle_authored_variants_without_animating_mixed_edges() {
    let manifest = lpc_mapped_terrain_manifest().expect("mapped manifest");
    let water_a = manifest
        .entry_for_corners_with_water_frame([LpcMappedTerrainMaterial::Water; 4], 0, 0)
        .expect("pure water frame");
    let water_b = manifest
        .entry_for_corners_with_water_frame([LpcMappedTerrainMaterial::Water; 4], 0, 6)
        .expect("pure water frame");
    assert_ne!(water_a.rect, water_b.rect);

    let quiet_a = manifest
        .entry_for_corners_with_water_frame([LpcMappedTerrainMaterial::Water; 4], 0x800, 0)
        .expect("quiet water frame");
    let quiet_b = manifest
        .entry_for_corners_with_water_frame([LpcMappedTerrainMaterial::Water; 4], 0x800, 60)
        .expect("quiet water frame");
    assert_eq!(quiet_a.rect, quiet_b.rect);

    let shallow_a = manifest
        .entry_for_corners_with_water_frame(
            [LpcMappedTerrainMaterial::WaterShallowsSand; 4],
            0,
            0,
        )
        .expect("pure shallow-water frame");
    let shallow_b = manifest
        .entry_for_corners_with_water_frame(
            [LpcMappedTerrainMaterial::WaterShallowsSand; 4],
            0,
            6,
        )
        .expect("pure shallow-water frame");
    assert_ne!(shallow_a.rect, shallow_b.rect);

    let mixed_a = manifest
        .entry_for_corners_with_water_frame(
            [
                LpcMappedTerrainMaterial::Water,
                LpcMappedTerrainMaterial::Water,
                LpcMappedTerrainMaterial::Water,
                LpcMappedTerrainMaterial::Sand,
            ],
            0,
            0,
        )
        .expect("mixed shore frame");
    let mixed_b = manifest
        .entry_for_corners_with_water_frame(
            [
                LpcMappedTerrainMaterial::Water,
                LpcMappedTerrainMaterial::Water,
                LpcMappedTerrainMaterial::Water,
                LpcMappedTerrainMaterial::Sand,
            ],
            0,
            1,
        )
        .expect("mixed shore frame");
    assert_eq!(mixed_a.rect, mixed_b.rect);
}

#[test]
fn animated_water_variants_are_sparse_and_slow() {
    assert_eq!(animated_water_variant(0x800, 0, 4), 0);
    assert_ne!(animated_water_variant(0, 0, 4), animated_water_variant(0, 6, 4));
    assert_eq!(animated_water_variant(0, 1, 4), animated_water_variant(0, 5, 4));
}

#[test]
fn structural_rock_tiles_do_not_use_ground_corner_mapping() {
    assert_eq!(mapped_terrain_name(TileKind::Cliff), Some("Rock_Gray"));
    assert_eq!(mapped_terrain_name(TileKind::MountainRock), Some("Rock_Dark"));
}

#[test]
fn pure_grass_uses_deterministic_detail_variants() {
    let manifest = lpc_mapped_terrain_manifest().expect("mapped manifest");
    let base = manifest
        .entry_for_corners([LpcMappedTerrainMaterial::Grass; 4], 0)
        .expect("grass base");
    let detail = manifest
        .entry_for_corners([LpcMappedTerrainMaterial::Grass; 4], 3)
        .expect("grass detail");
    assert_ne!(base.rect, detail.rect);
    assert!(!detail.is_mixed);
}
