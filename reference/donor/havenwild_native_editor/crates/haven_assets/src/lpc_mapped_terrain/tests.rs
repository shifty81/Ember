use super::*;

#[test]
fn global_sampler_resolves_authored_tuple_across_storage_partition_edge() {
    let tile_at = |x: i32, _y: i32| {
        Some(if x < haven_core::MAP_W as i32 {
            TileKind::Grass
        } else {
            TileKind::Water
        })
    };
    let entry = lpc_mapped_terrain_transition_entry_for_tile_sampler(
        &tile_at,
        haven_core::MAP_W as i32 - 1,
        4,
    )
    .expect("authored grass/water tuple across partition edge");
    assert!(entry.is_mixed);
}

#[test]
fn global_sampler_resolves_gravel_bridge_across_storage_partition_edge() {
    let tile_at = |x: i32, _y: i32| {
        Some(if x < haven_core::MAP_W as i32 {
            TileKind::Grass
        } else {
            TileKind::PebbleShore
        })
    };
    let entry = lpc_mapped_terrain_transition_entry_for_tile_sampler(
        &tile_at,
        haven_core::MAP_W as i32 - 1,
        4,
    )
    .expect("authored gravel connector across partition edge");
    assert!(entry.is_mixed);
}

#[test]
fn sand_on_grass_corner_uses_tiled_tuple_sampling() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(4, 4, TileKind::Sand);

    let entry = lpc_mapped_terrain_entry_for_map(&map, 4, 4)
        .expect("sand-on-grass corner should use mapped LPC corner tuple");
    assert!(entry.is_mixed);
}

#[test]
fn sand_on_grass_diagnostic_reports_exact_tuple_and_source_rect() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(4, 4, TileKind::Sand);

    let diagnostic =
        lpc_mapped_terrain_diagnostic_for_map(&map, 4, 4).expect("sand-on-grass diagnostic");
    assert!(diagnostic.exact_match);
    assert!(diagnostic.is_mixed);
    assert!(diagnostic.rect.is_some());
    assert_eq!(diagnostic.corners[0], "Sand");
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
fn pure_mapped_fills_use_the_same_lpc_terrain_source() {
    let map = TavernMap::empty_with(TileKind::Grass);

    let entry = lpc_mapped_terrain_entry_for_map(&map, 4, 4)
        .expect("pure grass fill should use mapped LPC terrain source");

    assert!(!entry.is_mixed);
    assert!(!lpc_mapped_terrain_transition_covers_map_cell(&map, 4, 4));
}

#[test]
fn deep_shallow_boundary_uses_v7_mid_water_rim_and_authored_tuple_overlay() {
    let mut map = TavernMap::empty_with(TileKind::DeepWater);
    map.set(4, 4, TileKind::ShallowWater);

    let shallow_base = lpc_mapped_terrain_runtime_entry_for_map(&map, 4, 4)
        .expect("pure shallow-water owner fill");
    let shallow_overlay = lpc_mapped_terrain_transition_entry_for_map(&map, 4, 4)
        .expect("authored V7 shallow-to-medium tuple");
    let medium_base = lpc_mapped_terrain_runtime_entry_for_map(&map, 5, 4)
        .expect("derived medium-water rim fill");
    let deep_overlay = lpc_mapped_terrain_transition_entry_for_map(&map, 5, 3)
        .expect("authored V7 medium-to-deep tuple");

    let manifest = lpc_mapped_terrain_manifest().expect("mapped manifest");
    let expected_medium = manifest
        .quiet_entry_for_corners(["Water"; 4])
        .expect("quiet V7 medium water");

    assert!(!shallow_base.is_mixed);
    assert!(shallow_overlay.is_mixed);
    assert!(deep_overlay.is_mixed);
    assert_eq!(medium_base.rect, expected_medium.rect);
}

#[test]
fn pure_shallow_water_tuple_resolves_as_a_pure_fill() {
    let map = TavernMap::empty_with(TileKind::ShallowWater);
    let entry = lpc_mapped_terrain_entry_for_map(&map, 4, 4).expect("pure shallow-water tuple");

    assert!(!entry.is_mixed);
}

#[test]
fn ocean_shallow_sand_tuple_uses_authored_mixed_corner() {
    let mut map = TavernMap::empty_with(TileKind::Sand);
    map.set(4, 4, TileKind::OceanShallow);
    let entry = lpc_mapped_terrain_entry_for_map(&map, 4, 4).expect("authored ocean-shore tuple");

    assert!(entry.is_mixed);
}

#[test]
fn legacy_wet_sand_aliases_to_v7_sand_geometry() {
    assert_eq!(mapped_terrain_name(TileKind::WetSand), Some("Sand"));
}

#[test]
fn natural_terrain_aliases_share_the_editor_and_client_mapping() {
    assert_eq!(mapped_terrain_name(TileKind::Road), Some("Dirt_Tan"));
    assert_eq!(mapped_terrain_name(TileKind::StonePath), Some("Stone_Tan"));
    assert_eq!(
        mapped_terrain_name(TileKind::MountainRock),
        Some("Rock_Dark")
    );
    assert_eq!(mapped_terrain_name(TileKind::TilledSoil), Some("Soil"));
    assert_eq!(
        mapped_terrain_name(TileKind::RiverMouthBlend),
        Some("Water_Shallows_Dirt")
    );
}

#[test]
fn gravel_uses_authored_v7_edge_bridge_against_grass_and_sand() {
    for surrounding in [TileKind::Grass, TileKind::Sand] {
        let mut map = TavernMap::empty_with(surrounding);
        map.set(5, 4, TileKind::PebbleShore);

        assert!(lpc_mapped_terrain_exact_entry_for_map(&map, 4, 4).is_none());
        let edge = lpc_mapped_terrain_transition_entry_for_map(&map, 4, 4)
            .expect("authored V7 gravel connector edge");
        let diagnostic =
            lpc_mapped_terrain_diagnostic_for_map(&map, 4, 4).expect("gravel edge diagnostic");

        assert!(edge.is_mixed);
        assert_eq!(diagnostic.status, LpcTupleResolutionStatus::AuthoredBridge);
        assert_eq!(map.get(5, 4), TileKind::PebbleShore);
    }
}

#[test]
fn rock_ground_uses_authored_v7_edge_bridge_against_grass_sand_and_water() {
    for surrounding in [TileKind::Grass, TileKind::Sand, TileKind::Water] {
        let mut map = TavernMap::empty_with(surrounding);
        map.set(5, 4, TileKind::MountainRock);

        assert!(lpc_mapped_terrain_exact_entry_for_map(&map, 4, 4).is_none());
        let edge = lpc_mapped_terrain_transition_entry_for_map(&map, 4, 4)
            .expect("authored V7 rock-ground connector edge");

        assert!(edge.is_mixed);
        assert_eq!(map.get(5, 4), TileKind::MountainRock);
    }
}

#[test]
fn mud_uses_direct_or_bridged_v7_edges_without_semantic_mutation() {
    for surrounding in [TileKind::Grass, TileKind::Sand, TileKind::Water] {
        let mut map = TavernMap::empty_with(surrounding);
        map.set(5, 4, TileKind::MudBank);

        let edge =
            lpc_mapped_terrain_transition_entry_for_map(&map, 4, 4).expect("authored V7 mud edge");
        let diagnostic =
            lpc_mapped_terrain_diagnostic_for_map(&map, 4, 4).expect("mud edge diagnostic");

        assert!(edge.is_mixed);
        assert!(matches!(
            diagnostic.status,
            LpcTupleResolutionStatus::Exact | LpcTupleResolutionStatus::AuthoredBridge
        ));
        assert_eq!(map.get(5, 4), TileKind::MudBank);
    }
}

#[test]
fn pure_grass_uses_deterministic_detail_variants() {
    let manifest = lpc_mapped_terrain_manifest().expect("mapped manifest");
    let base = manifest
        .entry_for_corners(["Grass"; 4], 0)
        .expect("grass base");
    let detail = manifest
        .entry_for_corners(["Grass"; 4], 17)
        .expect("grass detail");
    assert_ne!(base.rect, detail.rect);
    assert!(!detail.is_mixed);
}

#[test]
fn non_grass_fills_do_not_select_partial_detail_variants() {
    let manifest = lpc_mapped_terrain_manifest().expect("mapped manifest");
    for material in [
        "Sand",
        "Dirt_Roots",
        "Gravel_1",
        "Rock_Dark",
        "Mud_Brown",
        "Mudstone_Brown",
    ] {
        let quiet = manifest
            .quiet_entry_for_corners([material; 4])
            .unwrap_or_else(|| panic!("quiet fill for {material}"));
        for seed in [0, 1, 17, 99, u32::MAX] {
            let selected = manifest
                .entry_for_corners([material; 4], seed)
                .unwrap_or_else(|| panic!("selected fill for {material}"));
            assert_eq!(
                selected.rect, quiet.rect,
                "{material} selected a partial detail"
            );
        }
    }
}

#[test]
fn transition_adjacent_land_uses_quiet_owner_fill() {
    let manifest = lpc_mapped_terrain_manifest().expect("mapped manifest");
    let quiet = manifest
        .quiet_entry_for_corners(["Grass"; 4])
        .expect("quiet grass fill");
    let detail = manifest
        .entry_for_corners(["Grass"; 4], 17)
        .expect("detail grass fill");
    assert_ne!(quiet.rect, detail.rect);

    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(5, 4, TileKind::Sand);
    let owner = lpc_mapped_terrain_owner_fill_entry_for_map(&map, 4, 4)
        .expect("quiet owner fill beside transition");
    assert_eq!(owner.rect, quiet.rect);
}

#[test]
fn transition_adjacent_water_uses_quiet_owner_fill() {
    let manifest = lpc_mapped_terrain_manifest().expect("mapped manifest");
    let quiet = manifest
        .quiet_entry_for_corners(["Water"; 4])
        .expect("quiet medium-water fill");
    let animated = manifest
        .entry_for_corners_with_water_frame(["Water"; 4], 17, 1)
        .expect("animated medium-water fill");
    assert_ne!(quiet.rect, animated.rect);

    let mut map = TavernMap::empty_with(TileKind::Water);
    map.set(5, 4, TileKind::OceanDeep);
    let owner = lpc_mapped_terrain_owner_fill_entry_for_map_with_water_frame(&map, 4, 4, 1)
        .expect("quiet owner fill beside depth transition");

    assert_eq!(owner.rect, quiet.rect);
}

#[test]
fn exact_grass_in_deep_ocean_uses_authored_v7_edge_without_semantic_mutation() {
    let mut map = TavernMap::empty_with(TileKind::OceanDeep);
    map.set(5, 5, TileKind::Grass);

    assert_eq!(map.get(5, 5), TileKind::Grass);
    assert_eq!(mapped_terrain_at(&map, 5, 5), Some("Grass"));

    for y in 4..=6 {
        for x in 4..=6 {
            if x == 5 && y == 5 {
                continue;
            }
            assert_eq!(
                map.get(x, y),
                TileKind::OceanDeep,
                "presentation must not rewrite ocean semantics"
            );
            assert_eq!(
                mapped_terrain_at(&map, x, y),
                Some("Water"),
                "adjacent deep ocean should present as the one-cell V7 medium rim"
            );
        }
    }

    assert_eq!(mapped_terrain_at(&map, 3, 3), Some("Water_Deep"));
    for sample_y in 4..=5 {
        for sample_x in 4..=5 {
            let edge = lpc_mapped_terrain_transition_entry_for_map(&map, sample_x, sample_y)
                .expect("authored Grass/Water edge tuple");
            assert!(edge.is_mixed);
        }
    }
}

#[test]
fn jagged_ocean_coast_uses_only_exact_v7_tuples() {
    let mut map = TavernMap::empty_with(TileKind::OceanDeep);
    for y in 2..=10 {
        let shallow_end = if y % 4 < 2 { 6 } else { 7 };
        for x in 2..=3 {
            map.set(x, y, TileKind::Sand);
        }
        for x in 4..=shallow_end {
            map.set(x, y, TileKind::OceanShallow);
        }
    }

    for y in 2..10 {
        for x in 2..9 {
            let tuple =
                canonical_corner_tuple_for_map_tile(&map, x, y).expect("mapped ocean tuple");
            let entry = lpc_mapped_terrain_exact_entry_for_map(&map, x, y);
            assert!(
                entry.is_some(),
                "unsupported V7 ocean tuple at {x},{y}: {}",
                tuple.signature()
            );
        }
    }
}

#[test]
fn unsupported_three_material_junction_uses_owner_fill_without_hiding_diagnostic() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(5, 4, TileKind::Dirt);
    map.set(4, 5, TileKind::MountainPath);
    map.set(5, 5, TileKind::Grass);

    let diagnostic =
        lpc_mapped_terrain_diagnostic_for_map(&map, 4, 4).expect("mapped junction diagnostic");
    let entry = lpc_mapped_terrain_entry_for_map(&map, 4, 4).expect("owner fill fallback");

    assert_eq!(diagnostic.status, LpcTupleResolutionStatus::Unsupported);
    assert!(!entry.is_mixed);
}

#[test]
fn ocean_depth_contact_uses_v7_medium_water_variant_without_losing_semantic_identity() {
    let mut map = TavernMap::empty_with(TileKind::OceanDeep);
    map.set(5, 4, TileKind::OceanShallow);

    let medium_base = lpc_mapped_terrain_runtime_entry_for_map(&map, 4, 4)
        .expect("derived medium-water owner fill");
    let overlay = lpc_mapped_terrain_transition_entry_for_map(&map, 4, 4)
        .expect("authored V7 medium-to-ocean-shallow overlay");
    let deep_overlay = lpc_mapped_terrain_transition_entry_for_map(&map, 3, 4)
        .expect("authored V7 ocean-deep-to-medium overlay");
    let manifest = lpc_mapped_terrain_manifest().expect("mapped manifest");
    let medium_variants: Vec<AtlasRect> = manifest
        .entries
        .iter()
        .filter(|entry| entry_matches(entry, ["Water"; 4]))
        .map(|entry| entry.rect)
        .collect();

    assert_eq!(map.get(4, 4), TileKind::OceanDeep);
    assert_eq!(mapped_terrain_at(&map, 4, 4), Some("Water"));
    assert!(medium_variants.len() > 1);
    assert!(medium_variants.contains(&medium_base.rect));
    assert!(!medium_base.is_mixed);
    assert!(overlay.is_mixed);
    assert!(deep_overlay.is_mixed);
}

#[test]
fn land_transition_keeps_pure_base_and_separate_authored_overlay() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(4, 4, TileKind::Sand);

    let base = lpc_mapped_terrain_runtime_entry_for_map(&map, 4, 4).expect("pure sand owner fill");
    let overlay = lpc_mapped_terrain_transition_entry_for_map(&map, 4, 4)
        .expect("authored sand/grass tuple overlay");

    assert!(!base.is_mixed);
    assert!(overlay.is_mixed);
}

#[test]
fn canonical_tuple_uses_fixed_tl_tr_bl_br_order() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(4, 4, TileKind::Sand);
    map.set(5, 5, TileKind::ShallowWater);

    let tuple = canonical_corner_tuple_for_map_tile(&map, 4, 4).expect("canonical tuple");
    assert_eq!(
        tuple.corners(),
        ["Sand", "Grass", "Grass", "Water_Shallows_Dirt"]
    );
    assert_eq!(tuple.signature(), "Sand|Grass|Grass|Water_Shallows_Dirt");
    assert!(tuple.is_mixed());
}

#[test]
fn exact_dirt_sand_corner_uses_authored_tuple() {
    let mut map = TavernMap::empty_with(TileKind::Dirt);
    map.set(5, 5, TileKind::Sand);

    let entry = lpc_mapped_terrain_exact_entry_for_map(&map, 4, 4)
        .expect("terrain-map-v7 contains the authored dirt/sand outer corner");
    assert!(entry.is_mixed);
}

#[test]
fn exact_grass_sand_island_corner_uses_authored_tuple() {
    let mut map = TavernMap::empty_with(TileKind::Sand);
    map.set(5, 5, TileKind::Grass);

    let entry = lpc_mapped_terrain_exact_entry_for_map(&map, 4, 4)
        .expect("terrain-map-v7 contains the authored grass/sand island corner");
    assert!(entry.is_mixed);
}

#[test]
fn exact_lookup_accepts_authored_three_material_junction() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(5, 4, TileKind::Sand);
    map.set(4, 5, TileKind::Dirt);

    let entry = lpc_mapped_terrain_exact_entry_for_map(&map, 4, 4)
        .expect("terrain-map-v7 contains this authored three-material junction");
    assert!(entry.is_mixed);
}

#[test]
fn compatibility_report_groups_unsupported_signatures() {
    let map = TavernMap::empty_with(TileKind::GreenhouseZone);
    let report = audit_lpc_tuple_compatibility(&map);
    assert_eq!(report.inspected, 0);
    assert!(report.is_compatible());
    assert!(report.unsupported_signatures.is_empty());
}
