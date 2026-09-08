use super::*;

#[test]
fn deep_water_touching_land_becomes_shallow() {
    let mut map = TavernMap::empty_with(TileKind::DeepWater);
    // Use a stable land mass rather than a one-cell spike, which the
    // cleanup pass intentionally erodes before forming the water band.
    for y in 3..=5 {
        for x in 5..=7 {
            map.set(x, y, TileKind::Grass);
        }
    }

    let report = apply_coastline_tile_pass(&mut map, SceneBiome::Coastal);
    assert!(TerrainFamily::from_tile(map.get(4, 4)).is_shallow_water());
    assert!(report.shallow_water_band_tiles > 0);
}

#[test]
fn coastal_grass_touching_water_becomes_beach_band() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    // Use a stable body of water rather than a one-cell speckle, which the
    // cleanup pass intentionally removes before forming the beach band.
    for y in 3..=5 {
        for x in 5..=7 {
            map.set(x, y, TileKind::OceanShallow);
        }
    }

    apply_coastline_tile_pass(&mut map, SceneBiome::Coastal);
    assert!(matches!(map.get(4, 4), TileKind::Sand | TileKind::WetSand));
}

#[test]
fn isolated_water_speckle_is_removed_from_land() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(4, 4, TileKind::Water);

    let report = apply_coastline_tile_pass(&mut map, SceneBiome::Coastal);
    assert_eq!(map.get(4, 4), TileKind::Sand);
    assert_eq!(report.removed_water_speckles, 1);
}

#[test]
fn single_tile_land_spike_is_eroded_back_into_water() {
    let mut map = TavernMap::empty_with(TileKind::DeepWater);
    map.set(4, 4, TileKind::Grass);

    let report = apply_coastline_tile_pass(&mut map, SceneBiome::Coastal);
    assert!(TerrainFamily::from_tile(map.get(4, 4)).is_water());
    assert_eq!(report.eroded_land_spikes, 1);
}

#[test]
fn authored_farm_tiles_are_not_erased_by_coast_cleanup() {
    let mut map = TavernMap::empty_with(TileKind::ShallowWater);
    map.set(4, 4, TileKind::TilledSoil);

    apply_coastline_tile_pass(&mut map, SceneBiome::Coastal);
    assert_eq!(map.get(4, 4), TileKind::TilledSoil);
}

#[test]
fn lifecycle_keeps_visual_shore_treatments_out_of_gameplay_map() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(4, 4, TileKind::Sand);
    map.set(5, 4, TileKind::ShallowWater);
    map.set(6, 4, TileKind::RiverWater);

    let report = normalize_shore_water_lifecycle_region(&mut map, 3, 3, 7, 5, 3);

    assert_eq!(map.get(4, 4), TileKind::Sand);
    assert_eq!(map.get(5, 4), TileKind::ShallowWater);
    assert_eq!(map.get(6, 4), TileKind::RiverWater);
    assert_eq!(report.dry_sand_to_wet, 0);
    assert_eq!(report.generated_foam, 0);
    assert_eq!(report.generated_river_mouths, 0);
}

#[test]
fn visual_shore_tiles_remain_walkable_when_loaded_from_legacy_content() {
    for tile in [TileKind::WetSand, TileKind::ShoreFoam, TileKind::MudBank] {
        assert!(
            tile.walkable(),
            "{} should be presentation-only ground",
            tile.code()
        );
    }
}

#[test]
fn lifecycle_keeps_ocean_deep_behind_ocean_shallow() {
    let mut map = TavernMap::empty_with(TileKind::OceanDeep);
    map.set(4, 4, TileKind::Sand);
    normalize_shore_water_lifecycle_region(&mut map, 3, 3, 5, 5, 3);
    assert_eq!(map.get(4, 3), TileKind::OceanShallow);
    assert_eq!(map.get(5, 5), TileKind::OceanShallow);
}

#[test]
fn diagonal_only_water_contact_is_normalized_before_tuple_resolution() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(4, 4, TileKind::ShallowWater);
    map.set(5, 5, TileKind::ShallowWater);

    apply_coastline_tile_pass(&mut map, SceneBiome::Coastal);

    assert!(
        !TerrainFamily::from_tile(map.get(4, 4)).is_water()
            || !TerrainFamily::from_tile(map.get(5, 5)).is_water()
            || TerrainFamily::from_tile(map.get(5, 4)).is_water()
            || TerrainFamily::from_tile(map.get(4, 5)).is_water(),
        "diagonal-only water contacts must be bridged or removed"
    );
}

#[test]
fn mainland_stabilization_profile_normalizes_all_water_to_shallow() {
    let mut map = TavernMap::empty_with(TileKind::OceanDeep);
    for y in 8..=14 {
        for x in 8..=14 {
            map.set(x, y, TileKind::Grass);
        }
    }

    let report = apply_coastline_tile_pass_with_profile(
        &mut map,
        SceneBiome::Coastal,
        CoastlineGenerationProfile::mainland_shallow_stabilization(),
    );

    assert_eq!(report.deep_water_tiles, 0);
    assert!(map.tiles.iter().all(|tile| !matches!(
        tile,
        TileKind::DeepWater | TileKind::OceanDeep | TileKind::Water
    )));
}

#[test]
fn stabilization_profile_does_not_reintroduce_deep_water_lifecycle() {
    let mut map = TavernMap::empty_with(TileKind::DeepWater);
    map.set(4, 4, TileKind::Sand);

    apply_coastline_tile_pass_with_profile(
        &mut map,
        SceneBiome::Coastal,
        CoastlineGenerationProfile::mainland_shallow_stabilization(),
    );

    assert!(map
        .tiles
        .iter()
        .all(|tile| !matches!(tile, TileKind::DeepWater | TileKind::OceanDeep)));
}

#[test]
fn coastline_uses_continuous_semantic_sand_band() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    for y in 0..MAP_H as i32 {
        map.set(8, y, TileKind::OceanShallow);
    }

    apply_coastline_tile_pass_with_profile(
        &mut map,
        SceneBiome::Coastal,
        CoastlineGenerationProfile::mainland_shallow_stabilization(),
    );

    assert_eq!(map.get(7, 6), TileKind::Sand);
    assert_eq!(map.get(6, 6), TileKind::Sand);
    assert_eq!(map.get(5, 6), TileKind::Sand);
    assert_eq!(map.get(4, 6), TileKind::Grass);
    assert!(!map.tiles.contains(&TileKind::WetSand));
}

#[test]
fn semantic_beach_band_remains_walkable() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    for y in 0..MAP_H as i32 {
        map.set(8, y, TileKind::OceanShallow);
    }
    apply_coastline_tile_pass(&mut map, SceneBiome::Coastal);

    for x in 5..=7 {
        assert!(
            !map.collision_at(x, 6).blocked,
            "beach cell {x},6 must remain walkable"
        );
    }
}

#[test]
fn mainland_profile_repairs_diagonal_checkerboard_micro_topology() {
    let mut map = TavernMap::empty_with(TileKind::Grass);
    map.set(8, 8, TileKind::Sand);
    map.set(9, 8, TileKind::ShallowWater);
    map.set(8, 9, TileKind::ShallowWater);
    map.set(9, 9, TileKind::Sand);

    apply_coastline_tile_pass_with_profile(
        &mut map,
        SceneBiome::Coastal,
        CoastlineGenerationProfile::mainland_shallow_stabilization(),
    );
    let tl = TerrainFamily::from_tile(map.get(8, 8));
    let tr = TerrainFamily::from_tile(map.get(9, 8));
    let bl = TerrainFamily::from_tile(map.get(8, 9));
    let br = TerrainFamily::from_tile(map.get(9, 9));
    assert!(!(tl == br && tr == bl && tl != tr));
}
