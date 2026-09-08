use super::super::shore_water_lifecycle::normalize_authored_depth_topology_region;
use super::*;

#[test]
fn opposite_depth_edges_normalize_to_authored_shallow_topology() {
    let mut map = TavernMap::empty_with(TileKind::DeepWater);
    map.set(10, 9, TileKind::ShallowWater);
    map.set(10, 11, TileKind::ShallowWater);

    let changed = normalize_authored_depth_topology_region(&mut map, 8, 8, 12, 12);

    assert_eq!(changed, 1);
    assert_eq!(map.get(10, 10), TileKind::ShallowWater);
}

#[test]
fn adjacent_depth_edges_keep_the_authored_outer_corner() {
    let mut map = TavernMap::empty_with(TileKind::DeepWater);
    map.set(10, 9, TileKind::ShallowWater);
    map.set(11, 10, TileKind::ShallowWater);

    let changed = normalize_authored_depth_topology_region(&mut map, 8, 8, 12, 12);

    assert_eq!(changed, 0);
    assert_eq!(map.get(10, 10), TileKind::DeepWater);
}

#[test]
fn multiple_diagonal_inner_corners_normalize_instead_of_overdrawing_full_cells() {
    let mut map = TavernMap::empty_with(TileKind::DeepWater);
    map.set(11, 9, TileKind::ShallowWater);
    map.set(9, 11, TileKind::ShallowWater);

    let changed = normalize_authored_depth_topology_region(&mut map, 8, 8, 12, 12);

    assert_eq!(changed, 1);
    assert_eq!(map.get(10, 10), TileKind::ShallowWater);
}

#[test]
fn outer_edge_plus_unrelated_inner_corner_normalizes_to_one_authored_cell() {
    let mut map = TavernMap::empty_with(TileKind::DeepWater);
    map.set(10, 9, TileKind::ShallowWater);
    map.set(11, 11, TileKind::ShallowWater);

    let changed = normalize_authored_depth_topology_region(&mut map, 8, 8, 12, 12);

    assert_eq!(changed, 1);
    assert_eq!(map.get(10, 10), TileKind::ShallowWater);
}

#[test]
fn marine_depth_topology_repair_preserves_ocean_identity() {
    let mut map = TavernMap::empty_with(TileKind::OceanDeep);
    map.set(10, 9, TileKind::OceanShallow);
    map.set(10, 11, TileKind::OceanShallow);

    let changed = normalize_authored_depth_topology_region(&mut map, 8, 8, 12, 12);

    assert_eq!(changed, 1);
    assert_eq!(map.get(10, 10), TileKind::OceanShallow);
}

#[test]
fn independent_unsupported_depth_contacts_repair_independently() {
    let mut map = TavernMap::empty_with(TileKind::DeepWater);
    map.set(10, 9, TileKind::ShallowWater);
    map.set(10, 11, TileKind::ShallowWater);
    map.set(20, 19, TileKind::ShallowWater);
    map.set(20, 21, TileKind::ShallowWater);

    let changed = normalize_authored_depth_topology_region(&mut map, 8, 8, 22, 22);

    assert_eq!(changed, 2);
    assert_eq!(map.get(10, 10), TileKind::ShallowWater);
    assert_eq!(map.get(20, 20), TileKind::ShallowWater);
}

#[test]
fn small_freshwater_pond_stays_entirely_shallow() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    for y in 10..13 {
        for x in 10..13 {
            map.set(x, y, TileKind::Water);
        }
    }
    normalize_shore_water_lifecycle_region(&mut map, 0, 0, MAP_W as i32 - 1, MAP_H as i32 - 1, 3);
    for y in 10..13 {
        for x in 10..13 {
            assert_eq!(map.get(x, y), TileKind::ShallowWater);
        }
    }
}

#[test]
fn six_by_six_pond_does_not_create_a_tiny_square_deep_core() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    for y in 10..16 {
        for x in 10..16 {
            map.set(x, y, TileKind::Water);
        }
    }
    normalize_shore_water_lifecycle_region(&mut map, 0, 0, MAP_W as i32 - 1, MAP_H as i32 - 1, 3);
    for y in 10..16 {
        for x in 10..16 {
            assert_eq!(map.get(x, y), TileKind::ShallowWater);
        }
    }
}

#[test]
fn seven_by_seven_pond_can_support_a_three_by_three_deep_core() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    for y in 10..17 {
        for x in 10..17 {
            map.set(x, y, TileKind::Water);
        }
    }
    normalize_shore_water_lifecycle_region(&mut map, 0, 0, MAP_W as i32 - 1, MAP_H as i32 - 1, 3);
    assert_eq!(map.get(10, 13), TileKind::ShallowWater);
    assert_eq!(map.get(11, 13), TileKind::ShallowWater);
    for y in 12..15 {
        for x in 12..15 {
            assert_eq!(map.get(x, y), TileKind::DeepWater);
        }
    }
}

#[test]
fn large_pond_has_a_two_tile_shallow_band_and_deep_center() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    for y in 10..21 {
        for x in 10..21 {
            map.set(x, y, TileKind::Water);
        }
    }
    normalize_shore_water_lifecycle_region(&mut map, 0, 0, MAP_W as i32 - 1, MAP_H as i32 - 1, 3);
    assert_eq!(map.get(10, 15), TileKind::ShallowWater);
    assert_eq!(map.get(11, 15), TileKind::ShallowWater);
    assert_eq!(map.get(12, 15), TileKind::DeepWater);
    assert_eq!(map.get(15, 15), TileKind::DeepWater);
}

#[test]
fn ocean_depth_keeps_ocean_identity() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    for y in 10..21 {
        for x in 10..21 {
            map.set(x, y, TileKind::OceanDeep);
        }
    }
    normalize_shore_water_lifecycle_region(&mut map, 0, 0, MAP_W as i32 - 1, MAP_H as i32 - 1, 3);
    assert_eq!(map.get(10, 15), TileKind::OceanShallow);
    assert_eq!(map.get(15, 15), TileKind::OceanDeep);
}

#[test]
fn open_ocean_world_edge_does_not_become_a_false_beach() {
    let mut map = TavernMap::empty_with(TileKind::OceanDeep);
    normalize_shore_water_lifecycle_region(&mut map, 0, 0, MAP_W as i32 - 1, MAP_H as i32 - 1, 3);
    assert_eq!(map.get(0, 0), TileKind::OceanDeep);
    assert_eq!(
        map.get(MAP_W as i32 - 1, MAP_H as i32 - 1),
        TileKind::OceanDeep
    );
}

#[test]
fn marine_topology_repairs_preserve_ocean_identity_and_sand_policy() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    for y in 8..=12 {
        for x in 8..=12 {
            map.set(x, y, TileKind::OceanShallow);
        }
    }
    map.set(10, 10, TileKind::Grass);
    apply_coastline_tile_pass(&mut map, SceneBiome::Coastal);
    assert!(matches!(
        map.get(10, 10),
        TileKind::OceanShallow | TileKind::Sand
    ));
    assert!(!matches!(
        map.get(10, 10),
        TileKind::ShallowWater | TileKind::DeepWater
    ));
}

#[test]
fn coastal_freshwater_uses_inland_bank_material() {
    assert_eq!(
        primary_shore_tile(
            SceneBiome::Coastal,
            ShoreWaterDomain::Inland,
            TileKind::Grass,
            1,
            0,
        ),
        TileKind::MudBank
    );
}

#[test]
fn production_profile_keeps_deep_water_in_large_bodies() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    for y in 10..21 {
        for x in 10..21 {
            map.set(x, y, TileKind::OceanDeep);
        }
    }
    apply_coastline_tile_pass_with_profile(
        &mut map,
        SceneBiome::Coastal,
        CoastlineGenerationProfile::mainland_hydrology(),
    );
    assert_eq!(map.get(10, 15), TileKind::OceanShallow);
    assert_eq!(map.get(15, 15), TileKind::OceanDeep);
}
