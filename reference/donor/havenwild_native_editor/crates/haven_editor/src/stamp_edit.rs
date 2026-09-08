#![expect(
    clippy::too_many_arguments,
    reason = "headless stamp edit commands explicitly receive world, history, project, source, scene, position, and payload"
)]

use haven_assets::stamp_registry::StampDefinition;
use haven_core::{GameWorld, ProjectSceneId, StampInstanceId};

use crate::{
    scene_edit::{scene_command, scene_mut},
    EditOperation, EditTransaction, EditorCommandBus, EditorCommandKind, EditorCommandSource,
    GridPos, SceneEditOutcome,
};

pub fn place_scene_stamp(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
    definition: &StampDefinition,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let stamp = definition.placed_at(x, y);
    let stamp_id = scene.map.place_stamp(stamp).map_err(|issues| {
        format!(
            "cannot place {} at {}, {} in {}: {}",
            definition.label,
            x,
            y,
            scene.name,
            issues
                .iter()
                .map(|issue| issue.label())
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;
    let placed = scene
        .map
        .stamp(stamp_id)
        .cloned()
        .ok_or_else(|| "placed stamp could not be resolved by stable ID".to_string())?;
    let message = format!(
        "Placed {} ({}) at {}, {} in {}",
        definition.label, stamp_id, x, y, scene.name
    );
    let command = scene_command(
        EditorCommandKind::PlaceStamp,
        source,
        project_id,
        &scene_id,
        Some(&definition.stable_id),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction = EditTransaction::new(format!("Place {}", definition.label), scene_id);
    transaction.push(EditOperation::InsertStamp { stamp: placed });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn move_scene_stamp(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    stamp_id: StampInstanceId,
    x: i32,
    y: i32,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let stamp = scene
        .map
        .stamp(stamp_id)
        .cloned()
        .ok_or_else(|| format!("{} stamp {} is missing", scene.name, stamp_id))?;
    scene.map.move_stamp(stamp_id, x, y).map_err(|issues| {
        format!(
            "cannot move {} to {}, {} in {}: {}",
            stamp.stamp_key,
            x,
            y,
            scene.name,
            issues
                .iter()
                .map(|issue| issue.label())
                .collect::<Vec<_>>()
                .join("; ")
        )
    })?;
    let message = format!(
        "Moved {} from {}, {} to {}, {} in {}",
        stamp.stamp_key, stamp.x, stamp.y, x, y, scene.name
    );
    let command = scene_command(
        EditorCommandKind::EditStamp,
        source,
        project_id,
        &scene_id,
        Some(&stamp.stamp_key),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction = EditTransaction::new(format!("Move {}", stamp.stamp_key), scene_id);
    transaction.push(EditOperation::MoveStamp {
        id: stamp_id,
        before: GridPos {
            x: stamp.x,
            y: stamp.y,
        },
        after: GridPos { x, y },
    });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn erase_scene_stamp(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let stamp_id = scene
        .map
        .stamp_id_at(x, y)
        .ok_or_else(|| format!("no stamp at {}, {} in {}", x, y, scene.name))?;
    let stamp = scene
        .map
        .remove_stamp(stamp_id)
        .ok_or_else(|| format!("stamp {} disappeared before erase", stamp_id))?;
    let message = format!(
        "Removed {} ({}) from {}, {} in {}",
        stamp.stamp_key, stamp.id, x, y, scene.name
    );
    let command = scene_command(
        EditorCommandKind::EditStamp,
        source,
        project_id,
        &scene_id,
        Some(&stamp.stamp_key),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction = EditTransaction::new(format!("Remove {}", stamp.stamp_key), scene_id);
    transaction.push(EditOperation::RemoveStamp { stamp });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_assets::stamp_registry::StampRegistry;
    use haven_core::SceneId;

    #[test]
    fn place_move_erase_stamp_records_typed_transactions() {
        let mut world = GameWorld::starter();
        let scene = world.scene_mut(SceneId::Farmstead).expect("farmstead");
        scene.map.objects.clear();
        scene.map.stamps.clear();
        for y in 4..20 {
            for x in 4..24 {
                scene.map.set(x, y, haven_core::TileKind::Grass);
            }
        }
        let registry = StampRegistry::load_default().expect("stamp registry");
        let definition = registry
            .entries()
            .first()
            .expect("current stamp registry should expose a stamp")
            .clone();
        let mut command_bus = EditorCommandBus::with_limit(8);

        place_scene_stamp(
            &mut world,
            &mut command_bus,
            "test_project",
            EditorCommandSource::MainEditor,
            SceneId::Farmstead,
            10,
            10,
            &definition,
        )
        .expect("place stamp");
        let stamp_id = world
            .scene(SceneId::Farmstead)
            .expect("farmstead")
            .map
            .stamps
            .first()
            .expect("placed stamp")
            .id;
        assert_eq!(command_bus.undo_len(), 1);

        move_scene_stamp(
            &mut world,
            &mut command_bus,
            "test_project",
            EditorCommandSource::MainEditor,
            SceneId::Farmstead,
            stamp_id,
            18,
            12,
        )
        .expect("move stamp");
        assert_eq!(command_bus.undo_len(), 2);

        let (erase_x, erase_y, _, _) = world
            .scene(SceneId::Farmstead)
            .expect("farmstead")
            .map
            .stamp(stamp_id)
            .expect("moved stamp")
            .visual_rect();
        erase_scene_stamp(
            &mut world,
            &mut command_bus,
            "test_project",
            EditorCommandSource::MainEditor,
            SceneId::Farmstead,
            erase_x,
            erase_y,
        )
        .expect("erase stamp");
        assert_eq!(command_bus.undo_len(), 3);
        command_bus.undo_world(&mut world).expect("undo erase");
        assert!(world
            .scene(SceneId::Farmstead)
            .expect("farmstead")
            .map
            .stamp(stamp_id)
            .is_some());
    }
}
