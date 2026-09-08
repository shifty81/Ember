#![expect(
    clippy::too_many_arguments,
    reason = "headless edit commands explicitly receive world, history, project, source, scene, and cell payload"
)]

use haven_core::{AutotileOverride, GameWorld, ProjectSceneId};
use haven_world::autotile::normalize_mask;

use crate::{
    EditOperation, EditTransaction, EditorCommand, EditorCommandBus, EditorCommandKind,
    EditorCommandSource, GridPos, SceneEditOutcome,
};

pub fn set_scene_autotile_override(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
    mask: u8,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = world
        .scene_mut_by_id(&scene_id)
        .ok_or_else(|| format!("scene {} is not loaded", scene_id.label()))?;
    let tile = scene.map.get(x, y);
    let group = tile.autotile_group().ok_or_else(|| {
        format!(
            "{} at {}, {} does not use an autotile group",
            tile.label(),
            x,
            y
        )
    })?;
    let before = scene.autotile_override_at(x, y);
    let after = AutotileOverride::new(x, y, group, normalize_mask(mask));
    if before == Some(after) {
        return Err(format!(
            "{} already uses manual mask {:02x} at {}, {}",
            group.label(),
            after.mask,
            x,
            y
        ));
    }
    scene.set_autotile_override(after);
    let message = format!(
        "Set {} manual autotile mask {:02x} at {}, {} in {}",
        group.label(),
        after.mask,
        x,
        y,
        scene.name
    );
    let command = autotile_command(
        source,
        project_id,
        &scene_id,
        Some(group.code()),
        GridPos { x, y },
        &message,
    );
    let mut transaction = EditTransaction::new("Set autotile override", scene_id);
    transaction.push(EditOperation::SetAutotileOverride {
        cell: GridPos { x, y },
        before,
        after: Some(after),
    });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn clear_scene_autotile_override(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = world
        .scene_mut_by_id(&scene_id)
        .ok_or_else(|| format!("scene {} is not loaded", scene_id.label()))?;
    let before = scene
        .autotile_override_at(x, y)
        .ok_or_else(|| format!("no manual autotile override at {}, {}", x, y))?;
    scene.clear_autotile_override(x, y);
    let message = format!(
        "Returned {}, {} in {} to automatic {} adjacency",
        x,
        y,
        scene.name,
        before.group.label()
    );
    let command = autotile_command(
        source,
        project_id,
        &scene_id,
        Some(before.group.code()),
        GridPos { x, y },
        &message,
    );
    let mut transaction = EditTransaction::new("Clear autotile override", scene_id);
    transaction.push(EditOperation::SetAutotileOverride {
        cell: GridPos { x, y },
        before: Some(before),
        after: None,
    });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

fn autotile_command(
    source: EditorCommandSource,
    project_id: &str,
    scene_id: &ProjectSceneId,
    asset_id: Option<&str>,
    cell: GridPos,
    message: &str,
) -> EditorCommand {
    EditorCommand::new(
        EditorCommandKind::EditAutotile,
        source,
        project_id.to_string(),
        Some(scene_id.code().to_string()),
        asset_id.map(str::to_string),
        vec![cell],
        message.to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::{SceneId, TileKind};

    #[test]
    fn manual_override_round_trips_through_typed_history() {
        let mut world = GameWorld::starter();
        let scene_id = ProjectSceneId::from(SceneId::Farmstead);
        world
            .scene_mut_by_id(&scene_id)
            .expect("farmstead")
            .map
            .set(8, 8, TileKind::Road);
        let mut bus = EditorCommandBus::with_limit(8);
        set_scene_autotile_override(
            &mut world,
            &mut bus,
            "test",
            EditorCommandSource::MainEditor,
            scene_id.clone(),
            8,
            8,
            5,
        )
        .expect("set override");
        assert_eq!(
            world
                .scene_by_id(&scene_id)
                .expect("farmstead")
                .autotile_override_at(8, 8)
                .expect("override")
                .mask,
            5
        );
        bus.undo_world(&mut world).expect("undo override");
        assert!(world
            .scene_by_id(&scene_id)
            .expect("farmstead")
            .autotile_override_at(8, 8)
            .is_none());
    }
}
