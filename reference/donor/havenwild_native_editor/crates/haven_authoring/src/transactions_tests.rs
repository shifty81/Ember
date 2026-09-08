use super::EditTransaction;
use crate::{EditOperation, GridPos};
use haven_core::{
    GameWorld, ObjectFootprint, ObjectKind, PlacedStamp, ProjectSceneId, SceneBiome, SceneId,
    SceneKind, SceneMap, StampInstanceId, TileKind,
};

#[test]
fn repeated_tile_edits_coalesce_to_one_operation() {
    let mut transaction = EditTransaction::new("Paint road stroke", SceneId::Farmstead);
    transaction.push(EditOperation::SetTile {
        cell: GridPos { x: 3, y: 4 },
        before: TileKind::Grass,
        after: TileKind::Road,
    });
    transaction.push(EditOperation::SetTile {
        cell: GridPos { x: 3, y: 4 },
        before: TileKind::Road,
        after: TileKind::StonePath,
    });
    assert_eq!(transaction.operation_count(), 1);
    assert!(matches!(
        transaction.operations[0],
        EditOperation::SetTile {
            before: TileKind::Grass,
            after: TileKind::StonePath,
            ..
        }
    ));
}

#[test]
fn transaction_revert_and_apply_round_trip_world_state() {
    let mut world = GameWorld::starter();
    let scene_id = ProjectSceneId::from(SceneId::Farmstead);
    let before = world
        .scene_by_id(&scene_id)
        .expect("farmstead")
        .map
        .get(3, 4);
    let mut transaction = EditTransaction::new("Paint road", scene_id.clone());
    transaction.push(EditOperation::SetTile {
        cell: GridPos { x: 3, y: 4 },
        before,
        after: TileKind::Road,
    });

    transaction.apply(&mut world).expect("apply");
    assert_eq!(
        world
            .scene_by_id(&scene_id)
            .expect("farmstead")
            .map
            .get(3, 4),
        TileKind::Road
    );
    transaction.revert(&mut world).expect("revert");
    assert_eq!(
        world
            .scene_by_id(&scene_id)
            .expect("farmstead")
            .map
            .get(3, 4),
        before
    );
}

#[test]
fn object_property_update_round_trips_by_stable_id() {
    let mut world = GameWorld::starter();
    let scene_id = ProjectSceneId::new("object_inspector_test");
    world
        .insert_scene_at(
            world.scenes.len(),
            SceneMap::blank(
                scene_id.clone(),
                "Object Inspector Test",
                SceneKind::Exterior,
                SceneBiome::Temperate,
            ),
        )
        .expect("insert test scene");
    let object_id = world
        .scene_mut_by_id(&scene_id)
        .expect("test scene")
        .map
        .place_object(ObjectKind::Table, 20, 20)
        .expect("place table");
    let before = *world
        .scene_by_id(&scene_id)
        .and_then(|scene| scene.map.object(object_id))
        .expect("placed object");
    let mut after = before;
    after.footprint.visual_w += 1;
    after.footprint.interaction_w += 1;

    let mut transaction = EditTransaction::new("Resize object", scene_id.clone());
    transaction.push(EditOperation::UpdateObject {
        id: object_id,
        before,
        after,
    });
    transaction.apply(&mut world).expect("apply object update");
    assert_eq!(
        *world
            .scene_by_id(&scene_id)
            .and_then(|scene| scene.map.object(object_id))
            .expect("updated object"),
        after
    );
    transaction
        .revert(&mut world)
        .expect("revert object update");
    assert_eq!(
        *world
            .scene_by_id(&scene_id)
            .and_then(|scene| scene.map.object(object_id))
            .expect("restored object"),
        before
    );
}

#[test]
fn scene_insert_and_rename_operations_round_trip() {
    let mut world = GameWorld::starter();
    let first_id = ProjectSceneId::new("authoring_test_scene");
    let renamed_id = ProjectSceneId::new("authoring_test_scene_renamed");
    let scene = SceneMap::blank(
        first_id.clone(),
        "Authoring Test Scene",
        SceneKind::Exterior,
        SceneBiome::Temperate,
    );
    let index = world.scenes.len();
    let mut insert = EditTransaction::new("Create scene", first_id.clone());
    insert.push(EditOperation::InsertScene {
        index,
        scene: scene.clone(),
    });
    insert.apply(&mut world).expect("insert scene");
    assert_eq!(world.scenes.position(&first_id), Some(index));

    let mut rename = EditTransaction::new("Rename scene", renamed_id.clone());
    rename.push(EditOperation::RenameScene {
        before_id: first_id.clone(),
        after_id: renamed_id.clone(),
        before_name: "Authoring Test Scene".to_string(),
        after_name: "Renamed Authoring Scene".to_string(),
    });
    rename.apply(&mut world).expect("rename scene");
    assert_eq!(
        world.scene_by_id(&renamed_id).expect("renamed scene").name,
        "Renamed Authoring Scene"
    );
    rename.revert(&mut world).expect("revert rename");
    assert_eq!(
        world.scene_by_id(&first_id).expect("original scene").name,
        "Authoring Test Scene"
    );
    insert.revert(&mut world).expect("remove inserted scene");
    assert!(world.scene_by_id(&first_id).is_none());
}

#[test]
fn stamp_insert_move_and_undo_round_trip() {
    let mut world = GameWorld::starter();
    let scene_id = ProjectSceneId::from(SceneId::Farmstead);
    let scene = world
        .scene_mut(SceneId::Farmstead)
        .expect("farmstead scene");
    scene.map.objects.clear();
    for y in 9..14 {
        for x in 9..17 {
            scene.map.set(x, y, haven_core::TileKind::Grass);
        }
    }
    let footprint = ObjectFootprint {
        visual_w: 2,
        visual_h: 2,
        collision_w: 2,
        collision_h: 1,
        interaction_w: 2,
        interaction_h: 1,
        blocks_movement: true,
        ..ObjectFootprint::single_tile()
    };
    let mut stamp = PlacedStamp::new("test_two_by_two", 10, 10, footprint);
    stamp.id = StampInstanceId::from_raw(9001);
    let mut insert = EditTransaction::new("Place stamp", scene_id.clone());
    insert.push(EditOperation::InsertStamp {
        stamp: stamp.clone(),
    });
    insert.apply(&mut world).expect("insert stamp");
    assert!(world
        .scene_by_id(&scene_id)
        .expect("scene")
        .map
        .stamp(stamp.id)
        .is_some());

    let mut movement = EditTransaction::new("Move stamp", scene_id.clone());
    movement.push(EditOperation::MoveStamp {
        id: stamp.id,
        before: GridPos { x: 10, y: 10 },
        after: GridPos { x: 14, y: 12 },
    });
    movement.apply(&mut world).expect("move stamp");
    assert_eq!(
        world
            .scene_by_id(&scene_id)
            .and_then(|scene| scene.map.stamp(stamp.id))
            .map(|placed| (placed.x, placed.y)),
        Some((14, 12))
    );
    movement.revert(&mut world).expect("undo movement");
    insert.revert(&mut world).expect("undo insert");
    assert!(world
        .scene_by_id(&scene_id)
        .expect("scene")
        .map
        .stamp(stamp.id)
        .is_none());
}

#[test]
fn expandable_stamp_resize_round_trips_by_stable_id() {
    let mut world = GameWorld::starter();
    let scene_id = ProjectSceneId::new("expandable_stamp_test");
    world
        .insert_scene_at(
            world.scenes.len(),
            SceneMap::blank(
                scene_id.clone(),
                "Expandable Stamp Test",
                SceneKind::Exterior,
                SceneBiome::Temperate,
            ),
        )
        .expect("insert test scene");
    let footprint = ObjectFootprint {
        visual_offset_x: -1,
        visual_offset_y: -2,
        visual_w: 3,
        visual_h: 3,
        collision_offset_x: -1,
        collision_offset_y: -2,
        collision_w: 3,
        collision_h: 3,
        interaction_offset_x: -1,
        interaction_offset_y: -2,
        interaction_w: 3,
        interaction_h: 3,
        blocks_movement: true,
        ..ObjectFootprint::single_tile()
    };
    let mut before = PlacedStamp::new("lpc_pond_grass_bank_a", 20, 20, footprint);
    before.id = StampInstanceId::from_raw(9101);
    world
        .scene_mut_by_id(&scene_id)
        .expect("test scene")
        .map
        .stamps
        .push(before.clone());
    let mut after = before.clone();
    after.footprint.visual_offset_x = -3;
    after.footprint.visual_offset_y = -4;
    after.footprint.visual_w = 7;
    after.footprint.visual_h = 5;
    after.footprint.collision_offset_x = -3;
    after.footprint.collision_offset_y = -4;
    after.footprint.collision_w = 7;
    after.footprint.collision_h = 5;
    after.footprint.interaction_offset_x = -3;
    after.footprint.interaction_offset_y = -4;
    after.footprint.interaction_w = 7;
    after.footprint.interaction_h = 5;

    let mut transaction = EditTransaction::new("Resize pond", scene_id.clone());
    transaction.push(EditOperation::RemoveStamp {
        stamp: before.clone(),
    });
    transaction.push(EditOperation::InsertStamp {
        stamp: after.clone(),
    });
    transaction.apply(&mut world).expect("apply resize");
    assert_eq!(
        world
            .scene_by_id(&scene_id)
            .and_then(|scene| scene.map.stamp(before.id))
            .expect("resized stamp"),
        &after
    );
    transaction.revert(&mut world).expect("revert resize");
    assert_eq!(
        world
            .scene_by_id(&scene_id)
            .and_then(|scene| scene.map.stamp(before.id))
            .expect("restored stamp"),
        &before
    );
}
