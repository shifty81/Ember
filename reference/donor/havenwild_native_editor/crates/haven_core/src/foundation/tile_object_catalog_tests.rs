use super::{TileCategory, TileKind};

#[test]
fn legacy_ocean_label_order_remains_loadable() {
    assert_eq!(
        TileKind::from_code("shallow_ocean"),
        Some(TileKind::OceanShallow)
    );
    assert_eq!(TileKind::from_code("deep_ocean"), Some(TileKind::OceanDeep));
}

#[test]
fn visual_land_transition_materials_remain_walkable() {
    for tile in [
        TileKind::Grass,
        TileKind::Sand,
        TileKind::WetSand,
        TileKind::PebbleShore,
        TileKind::Dirt,
        TileKind::Road,
        TileKind::StonePath,
        TileKind::MountainPath,
        TileKind::MountainRock,
        TileKind::MudBank,
        TileKind::ShoreFoam,
    ] {
        assert!(tile.walkable(), "{} must remain walkable", tile.code());
    }
}

#[test]
fn gravel_and_mud_are_ground_brushes_while_watered_soil_is_a_farm_state() {
    assert_eq!(TileKind::PebbleShore.label(), "Gravel");
    assert_eq!(TileKind::MudBank.label(), "Mud");
    assert!(TileKind::PebbleShore.is_lpc_mapped_editor_terrain());
    assert!(TileKind::MudBank.is_lpc_mapped_editor_terrain());
    assert!(!TileKind::WateredSoil.is_lpc_mapped_editor_terrain());
    assert_eq!(TileKind::from_code("gravel"), Some(TileKind::PebbleShore));
    assert_eq!(TileKind::from_code("mud"), Some(TileKind::MudBank));
}

#[test]
fn natural_rock_ground_and_cliff_faces_keep_distinct_roles() {
    assert_eq!(TileKind::MountainRock.category(), TileCategory::Terrain);
    assert!(TileKind::MountainRock.walkable());

    assert_eq!(TileKind::Cliff.category(), TileCategory::Wall);
    assert!(!TileKind::Cliff.walkable());
}

#[test]
fn semantic_water_and_solid_cliff_materials_remain_blocking() {
    for tile in [
        TileKind::Water,
        TileKind::ShallowWater,
        TileKind::DeepWater,
        TileKind::OceanDeep,
        TileKind::OceanShallow,
        TileKind::RiverWater,
        TileKind::RiverMouthBlend,
        TileKind::Wall,
        TileKind::Cliff,
        TileKind::CaveWall,
    ] {
        assert!(!tile.walkable(), "{} must remain blocking", tile.code());
    }
}
