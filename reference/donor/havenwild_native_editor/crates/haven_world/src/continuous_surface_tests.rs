use super::*;

#[test]
fn global_surface_address_crosses_positive_partition_edges() {
    let address = surface_tile_address(WorldTileCoord::new(MAP_W as i32, MAP_H as i32));
    assert_eq!(address.chunk, ChunkCoord::new(1, 1));
    assert_eq!((address.local_x, address.local_y), (0, 0));
}

#[test]
fn global_surface_address_preserves_negative_partition_edges() {
    let address = surface_tile_address(WorldTileCoord::new(-1, -1));
    assert_eq!(address.chunk, ChunkCoord::new(-1, -1));
    assert_eq!(
        (address.local_x, address.local_y),
        (MAP_W as i32 - 1, MAP_H as i32 - 1)
    );
    assert_eq!(
        surface_global_tile(address.chunk, address.local_x, address.local_y),
        WorldTileCoord::new(-1, -1)
    );
}

#[test]
fn legacy_exteriors_are_bound_as_surface_chunks() {
    let manifest = ContinuousSurfaceManifest::legacy_starter_bridge();
    assert!(manifest.validate().is_ok());
    assert_eq!(
        manifest
            .binding_for_scene(&ProjectSceneId::from(SceneId::EastWoods))
            .unwrap()
            .chunk,
        ChunkCoord::new(1, 0)
    );
    assert!(manifest
        .binding_for_scene(&ProjectSceneId::from(SceneId::TavernInterior))
        .is_none());
}
#[test]
fn runtime_state_tracks_global_coordinates_and_windows() {
    let state = SurfaceRuntimeState::starter(
        &ProjectSceneId::from(SceneId::EastWoods),
        WorldTileCoord::new(2, 7),
    );
    assert_eq!(state.active_chunk, ChunkCoord::new(1, 0));
    assert_eq!(state.global_tile, WorldTileCoord::new(MAP_W as i32 + 2, 7));
    assert_eq!(state.active_window().len(), 9);
    assert_eq!(state.preload_window().len(), 25);
}

#[test]
fn pcg_surface_scene_ids_preserve_region_and_signed_grid_coordinates() {
    let parsed =
        parse_pcg_surface_scene_id(&ProjectSceneId::new("pcg_havenwild_mainland_n2_7"))
            .expect("pcg surface partition");

    assert_eq!(parsed.0, "havenwild_mainland");
    assert_eq!(parsed.1, ChunkCoord::new(-2, 7));
    assert_eq!(encode_pcg_grid_token(-2), "n2");
    assert_eq!(encode_pcg_grid_token(7), "7");
    assert_eq!(
        pcg_surface_scene_id("havenwild_mainland", ChunkCoord::new(-2, 7)).code(),
        "pcg_havenwild_mainland_n2_7"
    );
}

#[test]
fn pcg_surface_scene_ids_keep_legacy_nonnegative_coordinates_loadable() {
    let parsed = parse_pcg_surface_scene_id(&ProjectSceneId::new("pcg_havenwild_mainland_2_7"))
        .expect("legacy nonnegative pcg surface partition");

    assert_eq!(parsed.0, "havenwild_mainland");
    assert_eq!(parsed.1, ChunkCoord::new(2, 7));
}

#[test]
fn dormant_pcg_partition_cannot_replace_active_legacy_farmstead_chunk() {
    let mut world = GameWorld::starter();
    let pcg = SceneMap::blank(
        ProjectSceneId::new("pcg_havenwild_mainland_0_0"),
        "Dormant Mainland 0,0",
        SceneKind::Exterior,
        haven_core::SceneBiome::Coastal,
    );
    world.insert_scene(pcg).expect("dormant pcg partition");
    world
        .set_active_scene(ProjectSceneId::from(SceneId::Farmstead))
        .expect("activate authored farmstead");

    let manifest = ContinuousSurfaceManifest::for_world(&world);

    assert_eq!(manifest.pcg_region, None);
    assert_eq!(
        manifest.scene_id_for_chunk(ChunkCoord::new(0, 0)),
        ProjectSceneId::from(SceneId::Farmstead)
    );
    assert!(manifest
        .binding_for_scene(&ProjectSceneId::new("pcg_havenwild_mainland_0_0"))
        .is_none());
}

#[test]
fn runtime_manifest_binds_loaded_pcg_rectangles_as_one_surface_region() {
    let mut world = GameWorld::starter();
    let first = SceneMap::blank(
        ProjectSceneId::new("pcg_havenwild_mainland_0_0"),
        "Mainland 0,0",
        SceneKind::Exterior,
        haven_core::SceneBiome::Coastal,
    );
    let second = SceneMap::blank(
        ProjectSceneId::new("pcg_havenwild_mainland_1_0"),
        "Mainland 1,0",
        SceneKind::Exterior,
        haven_core::SceneBiome::Coastal,
    );
    let other_island = SceneMap::blank(
        ProjectSceneId::new("pcg_other_island_1_0"),
        "Other island 1,0",
        SceneKind::Exterior,
        haven_core::SceneBiome::Coastal,
    );
    world.insert_scene(first).expect("first pcg partition");
    world.insert_scene(second).expect("second pcg partition");
    world
        .insert_scene(other_island)
        .expect("other island partition");
    world
        .set_active_scene(ProjectSceneId::new("pcg_havenwild_mainland_0_0"))
        .expect("active mainland partition");

    let state = SurfaceRuntimeState::for_world(&world, WorldTileCoord::new(3, 4));

    assert_eq!(state.active_chunk, ChunkCoord::new(0, 0));
    assert_eq!(
        state.manifest.scene_id_for_chunk(ChunkCoord::new(1, 0)),
        ProjectSceneId::new("pcg_havenwild_mainland_1_0")
    );
    assert!(state
        .manifest
        .exterior_bindings
        .iter()
        .all(|binding| !binding.scene_id.code().contains("other_island")));
}

#[test]
fn authored_pcg_partition_wins_over_generated_fallback_at_same_coordinate() {
    let mut world = GameWorld::starter();
    let authored = SceneMap::blank(
        ProjectSceneId::new("pcg_havenwild_mainland_1_0"),
        "Mainland 1,0",
        SceneKind::Exterior,
        haven_core::SceneBiome::Coastal,
    );
    let generated = SceneMap::blank(
        generated_chunk_scene_id(ChunkCoord::new(1, 0)),
        "Generated 1,0",
        SceneKind::Exterior,
        haven_core::SceneBiome::Coastal,
    );
    world
        .insert_scene(authored)
        .expect("authored pcg partition");
    world
        .insert_scene(generated)
        .expect("generated fallback partition");
    world
        .set_active_scene(ProjectSceneId::new("pcg_havenwild_mainland_1_0"))
        .expect("active authored partition");

    let manifest = ContinuousSurfaceManifest::for_world(&world);
    let binding = manifest
        .binding_for_scene(&ProjectSceneId::new("pcg_havenwild_mainland_1_0"))
        .expect("authored binding remains authoritative");

    assert_eq!(binding.chunk, ChunkCoord::new(1, 0));
    assert!(binding.authored_override);
    assert_eq!(
        manifest.scene_id_for_chunk(ChunkCoord::new(1, 0)),
        ProjectSceneId::new("pcg_havenwild_mainland_1_0")
    );
}

#[test]
fn chunk_window_is_centered_and_deterministic() {
    let window = chunk_window(ChunkCoord::new(4, -2), 1);
    assert_eq!(window.first(), Some(&ChunkCoord::new(3, -3)));
    assert_eq!(window.last(), Some(&ChunkCoord::new(5, -1)));
}
#[test]
fn surface_partition_identity_is_recognized_without_loading_target_scene() {
    assert!(scene_id_is_surface_partition(&ProjectSceneId::new(
        "pcg_havenwild_mainland_n2_7"
    )));
    assert_eq!(
        parse_generated_chunk_scene_id(&ProjectSceneId::new("surface_x_n1_y_4")),
        Some(ChunkCoord::new(-1, 4))
    );
    assert_eq!(
        parse_generated_chunk_scene_id(&ProjectSceneId::new("surface_x_p3_y_n2")),
        Some(ChunkCoord::new(3, -2))
    );
    assert!(!scene_id_is_surface_partition(&ProjectSceneId::new(
        "willowmere_tavern_interior"
    )));
}

#[test]
fn legacy_pcg_ocean_migration_keeps_enclosed_pond_fresh() {
    let mut world = GameWorld::starter();
    let mut scene = SceneMap::blank(
        pcg_surface_scene_id("mainland", ChunkCoord::new(0, 0)),
        "PCG Mainland",
        SceneKind::Exterior,
        haven_core::SceneBiome::Coastal,
    );
    scene.map = haven_core::TavernMap::empty_with(TileKind::ShallowWater);
    for y in 20..44 {
        for x in 20..44 {
            scene.map.set(x, y, TileKind::Grass);
        }
    }
    for y in 30..33 {
        for x in 30..33 {
            scene.map.set(x, y, TileKind::ShallowWater);
        }
    }
    let id = scene.id.clone();
    world.insert_scene(scene).expect("insert pcg scene");
    world
        .set_active_scene(id.clone())
        .expect("activate pcg scene");

    let report = migrate_legacy_pcg_ocean_domains(&mut world);
    let map = &world.scene_by_id(&id).expect("migrated scene").map;
    assert!(report.marine_tiles_reclassified > 0);
    assert_eq!(map.get(0, 0), TileKind::OceanDeep);
    assert_eq!(map.get(19, 20), TileKind::OceanShallow);
    assert_eq!(map.get(31, 31), TileKind::ShallowWater);
}

