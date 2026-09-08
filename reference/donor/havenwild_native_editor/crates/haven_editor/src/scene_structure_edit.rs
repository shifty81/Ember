#![expect(
    clippy::too_many_arguments,
    reason = "structural edit commands explicitly receive world, history, project, source, scene, geometry, and target data"
)]

use haven_core::{GameWorld, ProjectSceneId, Transition, TransitionId, MAP_H, MAP_W};

use crate::{
    scene_edit::{scene_command, scene_mut, SceneEditOutcome},
    transition_grid_rect, EditOperation, EditTransaction, EditorCommandBus, EditorCommandKind,
    EditorCommandSource, GridPos,
};

pub fn set_scene_height(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
    height: u8,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let previous_height = scene.map.get_height(x, y);
    if previous_height == height {
        return Err(format!(
            "{} already has elevation {} at {}, {}",
            scene.name, height, x, y
        ));
    }

    scene.map.set_height(x, y, height);
    let message = format!(
        "Set elevation from {} to {} at {}, {} in {}",
        previous_height, height, x, y, scene.name
    );
    let asset_id = format!("height:{height}");
    let command = scene_command(
        EditorCommandKind::SetHeight,
        source,
        project_id,
        &scene_id,
        Some(&asset_id),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction = EditTransaction::new("Set elevation stroke", scene_id);
    transaction.push(EditOperation::SetHeight {
        cell: GridPos { x, y },
        before: previous_height,
        after: height,
    });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

#[allow(clippy::too_many_arguments)]
pub fn create_scene_transition(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    target: impl Into<ProjectSceneId>,
    spawn_x: i32,
    spawn_y: i32,
    label: impl Into<String>,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let target = target.into();
    if world.scene_by_id(&target).is_none() {
        return Err(format!("target scene {} is not loaded", target.label()));
    }
    if w <= 0 || h <= 0 || x < 0 || y < 0 || x + w > MAP_W as i32 || y + h > MAP_H as i32 {
        return Err(format!(
            "transition rectangle {}, {} {}x{} is outside the map",
            x, y, w, h
        ));
    }

    let scene = scene_mut(world, &scene_id)?;
    if scene.transition_at(x, y).is_some() {
        return Err(format!(
            "{} already has a transition at {}, {}",
            scene.name, x, y
        ));
    }

    let label = label.into();
    let transition_id = scene.insert_transition(Transition {
        id: TransitionId::from_raw(0),
        x,
        y,
        w,
        h,
        target: target.clone().into(),
        spawn_x,
        spawn_y,
        label: label.clone(),
    });
    let transition = scene
        .transition(transition_id)
        .cloned()
        .ok_or_else(|| "created transition could not be resolved by stable ID".to_string())?;
    let message = format!(
        "Created transition {} ({}) at {}, {} in {}",
        label, transition_id, x, y, scene.name
    );
    let command = scene_command(
        EditorCommandKind::CreateTransition,
        source,
        project_id,
        &scene_id,
        Some(target.code()),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction = EditTransaction::new(format!("Create transition {}", label), scene_id);
    transaction.push(EditOperation::InsertTransition { transition });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn erase_scene_transition(
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
    let Some(transition_id) = scene.transition_id_at(x, y) else {
        return Err(format!("{} has no transition at {}, {}", scene.name, x, y));
    };
    let removed = scene
        .remove_transition(transition_id)
        .ok_or_else(|| format!("{} transition {} is missing", scene.name, transition_id))?;
    let message = format!(
        "Erased transition {} at {}, {} in {}",
        removed.label, x, y, scene.name
    );
    let command = scene_command(
        EditorCommandKind::EditTransition,
        source,
        project_id,
        &scene_id,
        Some(removed.target.code()),
        vec![GridPos { x, y }],
        &message,
    );
    let label = removed.label.clone();
    let mut transaction = EditTransaction::new(format!("Erase transition {}", label), scene_id);
    transaction.push(EditOperation::RemoveTransition {
        transition: removed,
    });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn resize_scene_transition(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    transition_id: TransitionId,
    delta_w: i32,
    delta_h: i32,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let scene_name = scene.name.clone();
    let Some(transition) = scene.transition_mut(transition_id) else {
        return Err(format!(
            "{} transition {} is missing",
            scene_name, transition_id
        ));
    };
    let before = transition_grid_rect(transition);
    let new_w = (transition.w + delta_w).max(1);
    let new_h = (transition.h + delta_h).max(1);
    if transition.x + new_w > MAP_W as i32 || transition.y + new_h > MAP_H as i32 {
        return Err(format!(
            "cannot resize {} beyond map bounds from {}, {}",
            transition.label, transition.x, transition.y
        ));
    }
    if transition.w == new_w && transition.h == new_h {
        return Err(format!(
            "{} is already {}x{}",
            transition.label, transition.w, transition.h
        ));
    }

    transition.w = new_w;
    transition.h = new_h;
    let after = transition_grid_rect(transition);
    let label = transition.label.clone();
    let target = transition.target.clone();
    let cell = GridPos {
        x: transition.x,
        y: transition.y,
    };
    let message = format!(
        "Resized transition {} to {}x{} in {}",
        label, new_w, new_h, scene_name
    );
    let command = scene_command(
        EditorCommandKind::EditTransition,
        source,
        project_id,
        &scene_id,
        Some(target.code()),
        vec![cell],
        &message,
    );
    let mut transaction = EditTransaction::new(format!("Resize transition {}", label), scene_id);
    transaction.push(EditOperation::ResizeTransition {
        id: transition_id,
        before,
        after,
    });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}
