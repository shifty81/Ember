use super::*;
use haven_authoring::GridRect;
use haven_authoring::{EditorCommandBus, EditorCommandSource};
use haven_core::{
    GameWorld, ObjectFootprint, ProjectSceneId, SceneBiome, SceneKind, SceneMap, TileKind, MAP_H,
    MAP_W, MAX_STRUCTURAL_LEVEL,
};
use haven_world::scene_rectangles::{
    SceneRectangleAssignment, SceneRectangleAssignmentsFile, SceneRectangleManifest,
    SceneRectangleSpec,
};

fn test_manifest() -> SceneRectangleManifest {
    SceneRectangleManifest {
        schema: "test".to_string(),
        name: "test".to_string(),
        world_layout: "test".to_string(),
        scene_count: 2,
        special_scene_count: 0,
        scene_scale_targets: haven_world::scene_rectangles::SceneScaleTargets {
            small_test_chunk_tiles: [MAP_W as i32, MAP_H as i32],
            standard_outdoor_scene_tiles: [MAP_W as i32, MAP_H as i32],
            large_special_scene_tiles: [MAP_W as i32, MAP_H as i32],
            city_district_scene_tiles: [MAP_W as i32, MAP_H as i32],
            gameplay_camera_tiles_approx: vec![],
        },
        edge_contract: haven_world::scene_rectangles::SceneEdgeContract {
            seam_validation_band_tiles: 1,
            decoration_safe_band_tiles: 1,
            camera_void_rule: "none".to_string(),
        },
        archipelago_generation: Default::default(),
        scene_rectangles: vec![
            SceneRectangleSpec {
                scene_id: "r0".to_string(),
                landmass_id: 1,
                landmass_name: "Test".to_string(),
                kind: "surface".to_string(),
                grid_x: Some(0),
                grid_y: Some(0),
                world_rect_preview_px: [0, 0, 1, 1],
                tile_size: [MAP_W as i32, MAP_H as i32],
                edge_contract: "seamless".to_string(),
                streaming: "chunk".to_string(),
            },
            SceneRectangleSpec {
                scene_id: "r1".to_string(),
                landmass_id: 1,
                landmass_name: "Test".to_string(),
                kind: "surface".to_string(),
                grid_x: Some(1),
                grid_y: Some(0),
                world_rect_preview_px: [0, 0, 1, 1],
                tile_size: [MAP_W as i32, MAP_H as i32],
                edge_contract: "seamless".to_string(),
                streaming: "chunk".to_string(),
            },
        ],
    }
}

fn test_assignments() -> SceneRectangleAssignmentsFile {
    SceneRectangleAssignmentsFile {
        schema: "test".to_string(),
        assignments: vec![
            SceneRectangleAssignment {
                scene_code: "scene_a".to_string(),
                rectangle_id: "r0".to_string(),
                role: "wilderness".to_string(),
                ownership: "wilderness".to_string(),
                notes: None,
            },
            SceneRectangleAssignment {
                scene_code: "scene_b".to_string(),
                rectangle_id: "r1".to_string(),
                role: "wilderness".to_string(),
                ownership: "wilderness".to_string(),
                notes: None,
            },
        ],
    }
}

fn test_world() -> GameWorld {
    let mut world = GameWorld::starter();
    world
        .scenes
        .insert(SceneMap::blank(
            "scene_a",
            "Scene A",
            SceneKind::Exterior,
            SceneBiome::Temperate,
        ))
        .expect("scene a");
    world
        .scenes
        .insert(SceneMap::blank(
            "scene_b",
            "Scene B",
            SceneKind::Exterior,
            SceneBiome::Temperate,
        ))
        .expect("scene b");
    world
}

#[test]
fn global_address_crosses_partition_boundary() {
    let world = test_world();
    let address = resolve_world_surface_cell(
        &test_manifest(),
        &test_assignments(),
        &world,
        1,
        GridPos {
            x: MAP_W as i32,
            y: 5,
        },
    )
    .expect("address");
    assert_eq!(address.scene_id.code(), "scene_b");
    assert_eq!(address.local, GridPos { x: 0, y: 5 });
}

#[test]
fn rectangle_edit_records_one_cross_scene_undo_step() {
    let manifest = test_manifest();
    let assignments = test_assignments();
    let mut world = test_world();
    let mut bus = EditorCommandBus::with_limit(8);
    let outcome = paint_world_surface_rectangle(
        &mut world,
        &mut bus,
        "test",
        EditorCommandSource::MainEditor,
        &manifest,
        &assignments,
        1,
        GridRect::from_points(
            GridPos {
                x: MAP_W as i32 - 1,
                y: 4,
            },
            GridPos {
                x: MAP_W as i32,
                y: 4,
            },
        ),
        WorldSurfaceValue::Terrain(TileKind::Road),
    )
    .expect("edit");
    assert_eq!(outcome.scene_count, 2);
    assert_eq!(bus.undo_len(), 1);
    bus.undo_world(&mut world).expect("undo");
    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_a"))
            .expect("scene a")
            .map
            .get(MAP_W as i32 - 1, 4),
        TileKind::Grass
    );
    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_b"))
            .expect("scene b")
            .map
            .get(0, 4),
        TileKind::Grass
    );
}

#[test]
fn invalid_global_batch_rolls_back_every_partition() {
    let manifest = test_manifest();
    let assignments = test_assignments();
    let mut world = test_world();
    let mut bus = EditorCommandBus::with_limit(8);
    let valid = GridPos { x: 3, y: 4 };
    let outside = GridPos {
        x: MAP_W as i32 * 2,
        y: 4,
    };
    let result = paint_world_surface_cells(
        &mut world,
        &mut bus,
        "test",
        EditorCommandSource::MainEditor,
        &manifest,
        &assignments,
        1,
        [valid, outside],
        WorldSurfaceValue::Terrain(TileKind::Road),
        "Atomic global paint",
    );
    assert!(result.is_err());
    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_a"))
            .expect("scene a")
            .map
            .get(valid.x, valid.y),
        TileKind::Grass
    );
    assert_eq!(bus.undo_len(), 0);
}

#[test]
fn clipboard_paste_crosses_partition_as_one_undo_step() {
    let manifest = test_manifest();
    let assignments = test_assignments();
    let mut world = test_world();
    world
        .scene_mut_by_id(&ProjectSceneId::new("scene_a"))
        .expect("scene a")
        .map
        .set(MAP_W as i32 - 1, 5, TileKind::Dirt);
    world
        .scene_mut_by_id(&ProjectSceneId::new("scene_b"))
        .expect("scene b")
        .map
        .set(0, 5, TileKind::Dirt);
    let clipboard = copy_world_surface_rectangle(
        &manifest,
        &assignments,
        &world,
        1,
        GridRect::from_points(
            GridPos {
                x: MAP_W as i32 - 1,
                y: 5,
            },
            GridPos {
                x: MAP_W as i32,
                y: 5,
            },
        ),
    )
    .expect("copy");
    let mut bus = EditorCommandBus::with_limit(8);
    let outcome = paste_world_surface_clipboard(
        &mut world,
        &mut bus,
        "test",
        EditorCommandSource::MainEditor,
        &manifest,
        &assignments,
        1,
        GridPos {
            x: MAP_W as i32 - 1,
            y: 9,
        },
        &clipboard,
    )
    .expect("paste");
    assert_eq!(outcome.scene_count, 2);
    assert_eq!(bus.undo_len(), 1);
    bus.undo_world(&mut world).expect("undo paste");
    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_a"))
            .expect("scene a")
            .map
            .get(MAP_W as i32 - 1, 9),
        TileKind::Grass
    );
    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_b"))
            .expect("scene b")
            .map
            .get(0, 9),
        TileKind::Grass
    );
}

#[test]
fn complete_object_footprint_cannot_be_cropped_by_partition_boundary() {
    let world = test_world();
    let two_wide = ObjectFootprint {
        visual_w: 2,
        collision_w: 2,
        interaction_w: 2,
        ..ObjectFootprint::single_tile()
    };
    let error = validate_world_surface_footprint(
        &test_manifest(),
        &test_assignments(),
        &world,
        1,
        GridPos {
            x: MAP_W as i32 - 1,
            y: 5,
        },
        two_wide,
    )
    .expect_err("cross-partition footprint must be rejected");
    assert!(error.contains("crosses storage partitions"));
}

#[test]
fn structural_platform_edit_crosses_partitions_without_changing_geology() {
    let manifest = test_manifest();
    let assignments = test_assignments();
    let mut world = test_world();
    let left = GridPos {
        x: MAP_W as i32 - 1,
        y: 12,
    };
    let right = GridPos {
        x: MAP_W as i32,
        y: 12,
    };
    world
        .scene_mut_by_id(&ProjectSceneId::new("scene_a"))
        .expect("scene a")
        .map
        .set_height(MAP_W as i32 - 1, 12, 211);
    world
        .scene_mut_by_id(&ProjectSceneId::new("scene_b"))
        .expect("scene b")
        .map
        .set_height(0, 12, 37);

    let mut bus = EditorCommandBus::with_limit(8);
    let outcome = paint_world_surface_cells(
        &mut world,
        &mut bus,
        "test",
        EditorCommandSource::MainEditor,
        &manifest,
        &assignments,
        1,
        [left, right],
        WorldSurfaceValue::StructuralLevel(1),
        "Create Level 1 platform",
    )
    .expect("structural edit");

    assert_eq!(outcome.scene_count, 2);
    assert_eq!(bus.undo_len(), 1);
    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_a"))
            .expect("scene a")
            .map
            .get_structural_level(MAP_W as i32 - 1, 12),
        Some(1)
    );
    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_b"))
            .expect("scene b")
            .map
            .get_structural_level(0, 12),
        Some(1)
    );
    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_a"))
            .expect("scene a")
            .map
            .get_height(MAP_W as i32 - 1, 12),
        211
    );
    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_b"))
            .expect("scene b")
            .map
            .get_height(0, 12),
        37
    );

    bus.undo_world(&mut world).expect("undo structural edit");
    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_a"))
            .expect("scene a")
            .map
            .get_structural_level(MAP_W as i32 - 1, 12),
        None
    );
    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_b"))
            .expect("scene b")
            .map
            .get_structural_level(0, 12),
        None
    );
}

#[test]
fn raise_and_lower_platform_levels_are_clamped_and_transactional() {
    let manifest = test_manifest();
    let assignments = test_assignments();
    let mut world = test_world();
    let cell = GridPos { x: 8, y: 8 };
    let mut bus = EditorCommandBus::with_limit(8);

    for step in 1..=MAX_STRUCTURAL_LEVEL {
        adjust_world_structural_levels(
            &mut world,
            &mut bus,
            "test",
            EditorCommandSource::MainEditor,
            &manifest,
            &assignments,
            1,
            [cell],
            1,
            format!("Raise platform to Level {step}"),
        )
        .expect("raise structural level");
    }

    assert_eq!(
        world
            .scene_by_id(&ProjectSceneId::new("scene_a"))
            .expect("scene a")
            .map
            .get_structural_level(8, 8),
        Some(MAX_STRUCTURAL_LEVEL)
    );
    assert!(adjust_world_structural_levels(
        &mut world,
        &mut bus,
        "test",
        EditorCommandSource::MainEditor,
        &manifest,
        &assignments,
        1,
        [cell],
        1,
        "Raise beyond maximum",
    )
    .is_err());
}
