use super::*;
use crate::scene_structure_edit::{
    create_scene_transition, erase_scene_transition, resize_scene_transition,
};
use haven_core::{SceneId, MAP_H};
#[test]
fn paint_scene_tile_records_command_and_mutates_world() {
    let mut world = GameWorld::starter();
    for y in 5..8 {
        for x in 5..8 {
            world
                .scene_mut(SceneId::Farmstead)
                .expect("farmstead")
                .map
                .set(x, y, TileKind::Grass);
        }
    }
    world
        .scene_mut(SceneId::Farmstead)
        .expect("farmstead")
        .map
        .objects
        .clear();
    let mut command_bus = EditorCommandBus::with_limit(8);
    let result = paint_scene_tile(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        4,
        4,
        TileKind::Road,
    )
    .expect("paint should succeed");

    assert_eq!(result.command.command_type, "Paint terrain");
    assert_eq!(result.command.target.scene_id.as_deref(), Some("farmstead"));
    assert_eq!(result.command.target.asset_id.as_deref(), Some("road"));
    assert_eq!(
        world
            .scene(SceneId::Farmstead)
            .expect("farmstead")
            .map
            .get(4, 4),
        TileKind::Road
    );
    assert_eq!(command_bus.undo_len(), 1);
}

#[test]
fn place_scene_object_records_command_and_mutates_world() {
    let mut world = GameWorld::starter();
    for y in 5..8 {
        for x in 5..8 {
            world
                .scene_mut(SceneId::Farmstead)
                .expect("farmstead")
                .map
                .set(x, y, TileKind::Grass);
        }
    }
    world
        .scene_mut(SceneId::Farmstead)
        .expect("farmstead")
        .map
        .objects
        .clear();
    let mut command_bus = EditorCommandBus::with_limit(8);
    let result = place_scene_object(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        6,
        6,
        ObjectKind::Table,
    )
    .expect("place should succeed");

    assert_eq!(result.command.command_type, "Place object");
    assert_eq!(result.command.target.asset_id.as_deref(), Some("table"));
    assert!(world
        .scene(SceneId::Farmstead)
        .expect("farmstead")
        .map
        .object_at(6, 6)
        .is_some());
    assert_eq!(command_bus.undo_len(), 1);
}

#[test]
fn move_scene_object_records_command_and_mutates_world() {
    let mut world = GameWorld::starter();
    for y in 5..10 {
        for x in 5..10 {
            world
                .scene_mut(SceneId::Farmstead)
                .expect("farmstead")
                .map
                .set(x, y, TileKind::Grass);
        }
    }
    let scene = world.scene_mut(SceneId::Farmstead).expect("farmstead");
    scene.map.objects.clear();
    let object_id = scene
        .map
        .place_object(ObjectKind::Table, 5, 5)
        .expect("place source object");
    let mut command_bus = EditorCommandBus::with_limit(8);
    let result = move_scene_object(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        object_id,
        7,
        7,
    )
    .expect("move should succeed");

    assert_eq!(result.command.command_type, "Scene mutation");
    let scene = world.scene(SceneId::Farmstead).expect("farmstead");
    assert!(scene.map.object_at(5, 5).is_none());
    assert!(scene.map.object_at(7, 7).is_some());
    assert_eq!(command_bus.undo_len(), 1);
}

#[test]
fn duplicate_scene_object_records_command_and_mutates_world() {
    let mut world = GameWorld::starter();
    for y in 5..12 {
        for x in 5..12 {
            world
                .scene_mut(SceneId::Farmstead)
                .expect("farmstead")
                .map
                .set(x, y, TileKind::Grass);
        }
    }
    let scene = world.scene_mut(SceneId::Farmstead).expect("farmstead");
    scene.map.objects.clear();
    let object_id = scene
        .map
        .place_custom_object(PlacedObject::new(ObjectKind::Table, 5, 5))
        .expect("place source object");
    let mut command_bus = EditorCommandBus::with_limit(8);
    let result = duplicate_scene_object(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        object_id,
        8,
        8,
    )
    .expect("duplicate should succeed");

    assert_eq!(result.command.command_type, "Place object");
    let scene = world.scene(SceneId::Farmstead).expect("farmstead");
    assert_eq!(scene.map.objects.len(), 2);
    assert!(scene.map.object_at(5, 5).is_some());
    assert!(scene.map.object_at(8, 8).is_some());
    assert_eq!(command_bus.undo_len(), 1);
}

#[test]
fn erase_scene_object_removes_only_object() {
    let mut world = GameWorld::starter();
    world
        .scene_mut(SceneId::Farmstead)
        .expect("farmstead")
        .map
        .set(8, 8, TileKind::Road);
    world
        .scene_mut(SceneId::Farmstead)
        .expect("farmstead")
        .map
        .objects
        .clear();
    world
        .scene_mut(SceneId::Farmstead)
        .expect("farmstead")
        .map
        .place_custom_object(PlacedObject::new(ObjectKind::Table, 8, 8))
        .expect("place source object");
    let mut command_bus = EditorCommandBus::with_limit(8);
    erase_scene_object(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        8,
        8,
    )
    .expect("object erase should succeed");

    let scene = world.scene(SceneId::Farmstead).expect("farmstead");
    assert!(scene.map.object_at(8, 8).is_none());
    assert_eq!(scene.map.get(8, 8), TileKind::Road);
    assert_eq!(command_bus.undo_len(), 1);
}

#[test]
fn erase_scene_cell_removes_object_and_sets_fill_tile() {
    let mut world = GameWorld::starter();
    for y in 8..10 {
        for x in 8..10 {
            world
                .scene_mut(SceneId::Farmstead)
                .expect("farmstead")
                .map
                .set(x, y, TileKind::Grass);
        }
    }
    world
        .scene_mut(SceneId::Farmstead)
        .expect("farmstead")
        .map
        .place_object(ObjectKind::Table, 8, 8)
        .expect("place source object");
    let mut command_bus = EditorCommandBus::with_limit(8);
    erase_scene_cell(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        8,
        8,
        TileKind::Grass,
    )
    .expect("erase should succeed");

    let scene = world.scene(SceneId::Farmstead).expect("farmstead");
    assert_eq!(scene.map.get(8, 8), TileKind::Grass);
    assert!(scene.map.object_at(8, 8).is_none());
    assert_eq!(command_bus.undo_len(), 1);
}

#[test]
fn paint_scene_zone_records_command_and_mutates_world() {
    let mut world = GameWorld::starter();
    let mut command_bus = EditorCommandBus::with_limit(8);
    let result = paint_scene_zone(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        8,
        8,
        ZoneKind::Field,
    )
    .expect("zone paint should succeed");

    assert_eq!(result.command.command_type, "Assign room/zone");
    assert_eq!(result.command.target.scene_id.as_deref(), Some("farmstead"));
    assert_eq!(result.command.target.asset_id.as_deref(), Some("field"));
    assert_eq!(
        world
            .scene(SceneId::Farmstead)
            .expect("farmstead")
            .zone_at(8, 8),
        ZoneKind::Field
    );
    assert_eq!(command_bus.undo_len(), 1);
}

#[test]
fn create_scene_transition_records_command_and_mutates_world() {
    let mut world = GameWorld::starter();
    world
        .scene_mut(SceneId::Farmstead)
        .expect("farmstead")
        .transitions
        .clear();
    let mut command_bus = EditorCommandBus::with_limit(8);
    let result = create_scene_transition(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        10,
        10,
        2,
        2,
        SceneId::TavernInterior,
        23,
        23,
        "To Tavern",
    )
    .expect("transition create should succeed");

    assert_eq!(result.command.command_type, "Create transition");
    assert_eq!(result.command.target.scene_id.as_deref(), Some("farmstead"));
    assert_eq!(
        result.command.target.asset_id.as_deref(),
        Some("tavern_interior")
    );
    assert!(world
        .scene(SceneId::Farmstead)
        .expect("farmstead")
        .transition_at(10, 10)
        .is_some());
    assert_eq!(command_bus.undo_len(), 1);
}

#[test]
fn erase_scene_transition_removes_transition_at_cursor() {
    let mut world = GameWorld::starter();
    let (transition_x, transition_y) = {
        let scene = world.scene(SceneId::Farmstead).expect("farmstead");
        let transition = scene
            .transitions
            .iter()
            .find(|transition| transition.label == "Tavern Door")
            .expect("tavern transition");
        (transition.x, transition.y)
    };
    let mut command_bus = EditorCommandBus::with_limit(8);
    erase_scene_transition(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        transition_x,
        transition_y,
    )
    .expect("transition erase should succeed");

    assert!(world
        .scene(SceneId::Farmstead)
        .expect("farmstead")
        .transition_at(transition_x, transition_y)
        .is_none());
    assert_eq!(command_bus.undo_len(), 1);
}

#[test]
fn resize_scene_transition_records_command_and_mutates_world() {
    let mut world = GameWorld::starter();
    let (transition_id, transition_x, transition_y, original_w, original_h) = {
        let scene = world.scene(SceneId::Farmstead).expect("farmstead");
        let transition = scene
            .transitions
            .iter()
            .find(|transition| transition.label == "Tavern Door")
            .expect("tavern transition");
        (
            transition.id,
            transition.x,
            transition.y,
            transition.w,
            transition.h,
        )
    };
    let mut command_bus = EditorCommandBus::with_limit(8);
    let result = resize_scene_transition(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        transition_id,
        1,
        2,
    )
    .expect("transition resize should succeed");

    assert_eq!(result.command.command_type, "Edit transition");
    let transition = world
        .scene(SceneId::Farmstead)
        .expect("farmstead")
        .transition_at(transition_x, transition_y)
        .expect("transition");
    assert_eq!(
        (transition.w, transition.h),
        (original_w + 1, original_h + 2)
    );
    assert_eq!(command_bus.undo_len(), 1);
}

#[test]
fn typed_tile_transaction_undo_and_redo_round_trip() {
    let mut world = GameWorld::starter();
    let before = world
        .scene(SceneId::Farmstead)
        .expect("farmstead")
        .map
        .get(4, 4);
    let after = if before == TileKind::Road {
        TileKind::StonePath
    } else {
        TileKind::Road
    };
    let mut command_bus = EditorCommandBus::with_limit(8);
    paint_scene_tile(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        4,
        4,
        after,
    )
    .expect("paint should succeed");

    assert_eq!(command_bus.typed_undo_len(), 1);
    let undo = command_bus.undo_world(&mut world).expect("typed undo");
    assert!(undo.typed_transaction);
    assert_eq!(undo.operation_count, 1);
    assert_eq!(
        world
            .scene(SceneId::Farmstead)
            .expect("farmstead")
            .map
            .get(4, 4),
        before
    );

    let redo = command_bus.redo_world(&mut world).expect("typed redo");
    assert!(redo.typed_transaction);
    assert_eq!(
        world
            .scene(SceneId::Farmstead)
            .expect("farmstead")
            .map
            .get(4, 4),
        after
    );
}

#[test]
fn paint_drag_gesture_coalesces_into_one_undo_step() {
    let mut world = GameWorld::starter();
    let mut command_bus = EditorCommandBus::with_limit(8);
    let cells = [(12, 10), (13, 10), (14, 10)];
    let before = cells
        .iter()
        .map(|(x, y)| {
            world
                .scene(SceneId::Farmstead)
                .expect("farmstead")
                .map
                .get(*x, *y)
        })
        .collect::<Vec<_>>();

    command_bus.begin_gesture();
    for (x, y) in cells {
        let current = world
            .scene(SceneId::Farmstead)
            .expect("farmstead")
            .map
            .get(x, y);
        let target = if current == TileKind::Road {
            TileKind::StonePath
        } else {
            TileKind::Road
        };
        paint_scene_tile(
            &mut world,
            &mut command_bus,
            "test_project",
            EditorCommandSource::MainEditor,
            SceneId::Farmstead,
            x,
            y,
            target,
        )
        .expect("gesture paint");
    }
    assert_eq!(command_bus.undo_len(), 0);
    let committed = command_bus.commit_gesture().expect("gesture commit");
    assert_eq!(committed.operation_count, 3);
    assert_eq!(command_bus.undo_len(), 1);

    command_bus.undo_world(&mut world).expect("gesture undo");
    for ((x, y), tile) in cells.into_iter().zip(before) {
        assert_eq!(
            world
                .scene(SceneId::Farmstead)
                .expect("farmstead")
                .map
                .get(x, y),
            tile
        );
    }
}

#[test]
fn object_remove_undo_restores_stable_identity() {
    let mut world = GameWorld::starter();
    let scene = world.scene_mut(SceneId::Farmstead).expect("farmstead");
    scene.map.objects.clear();
    for y in 8..12 {
        for x in 8..12 {
            scene.map.set(x, y, TileKind::Grass);
        }
    }
    let object_id = scene
        .map
        .place_object(ObjectKind::Table, 8, 8)
        .expect("place table");
    let mut command_bus = EditorCommandBus::with_limit(8);
    erase_scene_object(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        8,
        8,
    )
    .expect("erase table");
    assert!(world
        .scene(SceneId::Farmstead)
        .expect("farmstead")
        .map
        .object(object_id)
        .is_none());

    command_bus.undo_world(&mut world).expect("undo erase");
    assert!(world
        .scene(SceneId::Farmstead)
        .expect("farmstead")
        .map
        .object(object_id)
        .is_some());
}

#[test]
fn exact_scene_tile_paint_changes_only_the_selected_cell_in_deep_water() {
    let mut world = GameWorld::starter();
    let scene = world
        .scene_mut(SceneId::Farmstead)
        .expect("farmstead scene");
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            scene.map.set(x, y, TileKind::DeepWater);
        }
    }
    let mut command_bus = EditorCommandBus::with_limit(8);

    paint_scene_tile_with_mode(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        8,
        8,
        TileKind::Dirt,
        haven_core::TerrainPaintMode::Exact,
    )
    .expect("exact paint");

    let map = &world.scene(SceneId::Farmstead).expect("farmstead").map;
    assert_eq!(map.get(8, 8), TileKind::Dirt);
    for (x, y) in [(7, 8), (9, 8), (8, 7), (8, 9), (7, 7), (9, 9)] {
        assert_eq!(map.get(x, y), TileKind::DeepWater);
    }
    assert_eq!(command_bus.undo_len(), 1);
}

#[test]
fn coastline_scene_tile_paint_records_neighbor_repairs_in_one_undo_step() {
    let mut world = GameWorld::starter();
    let scene = world
        .scene_mut(SceneId::Farmstead)
        .expect("farmstead scene");
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            scene.map.set(x, y, TileKind::DeepWater);
        }
    }
    let mut command_bus = EditorCommandBus::with_limit(8);

    let outcome = paint_scene_tile_with_mode(
        &mut world,
        &mut command_bus,
        "test_project",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        8,
        8,
        TileKind::Sand,
        haven_core::TerrainPaintMode::Coastline,
    )
    .expect("coastline paint");

    let map = &world.scene(SceneId::Farmstead).expect("farmstead").map;
    assert_eq!(map.get(8, 8), TileKind::Sand);
    assert_eq!(map.get(8, 7), TileKind::ShallowWater);
    assert!(outcome.command.target.grid_cells.len() > 1);
    assert_eq!(command_bus.undo_len(), 1);
}


#[test]
fn moved_scene_object_survives_world_serialization_round_trip() {
    let mut world = GameWorld::starter();
    for y in 5..12 {
        for x in 5..12 {
            world
                .scene_mut(SceneId::Farmstead)
                .expect("farmstead")
                .map
                .set(x, y, TileKind::Grass);
        }
    }
    let scene = world.scene_mut(SceneId::Farmstead).expect("farmstead");
    scene.map.objects.clear();
    let object_id = scene
        .map
        .place_object(ObjectKind::Crate, 5, 5)
        .expect("place acceptance crate");
    let mut command_bus = EditorCommandBus::with_limit(8);
    move_scene_object(
        &mut world,
        &mut command_bus,
        "development_acceptance",
        EditorCommandSource::MainEditor,
        SceneId::Farmstead,
        object_id,
        9,
        8,
    )
    .expect("move acceptance crate");

    let serialized = world.serialize_lines();
    let reloaded = GameWorld::deserialize_lines(&serialized).expect("reload serialized world");
    let object = reloaded
        .scene(SceneId::Farmstead)
        .expect("farmstead after reload")
        .map
        .object(object_id)
        .expect("acceptance crate after reload");
    assert_eq!((object.x, object.y), (9, 8));
}
