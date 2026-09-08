use haven_core::{GameWorld, PlacedObject, ProjectSceneId};

use crate::{
    EditOperation, EditTransaction, EditorCommand, EditorCommandBus, EditorCommandKind,
    EditorCommandSource, GridPos, SceneEditOutcome,
};

/// Applies one validated object-property change and records it as a typed transaction.
pub fn update_scene_object(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    candidate: PlacedObject,
    action: impl Into<String>,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let action = action.into();
    let scene = world
        .scene_mut_by_id(&scene_id)
        .ok_or_else(|| format!("scene {} is not loaded", scene_id.label()))?;
    let index = scene
        .map
        .object_index(candidate.id)
        .ok_or_else(|| format!("{} object {} is missing", scene.name, candidate.id))?;
    let before = scene.map.objects[index];
    if before == candidate {
        return Err("Object properties are unchanged".to_string());
    }
    if before.id != candidate.id || before.kind != candidate.kind {
        return Err(
            "Object identity and kind cannot be changed by the footprint inspector".to_string(),
        );
    }
    let issues = scene
        .map
        .placement_issues_for_object_excluding(candidate, Some(index));
    if !issues.is_empty() {
        return Err(format!(
            "cannot update {} in {}: {}",
            candidate.kind.label(),
            scene.name,
            issues
                .iter()
                .map(|issue| issue.label())
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }

    scene.map.objects[index] = candidate;
    let message = format!(
        "{} {} ({}) in {}",
        action,
        candidate.kind.label(),
        candidate.id,
        scene.name
    );
    let command = EditorCommand::new(
        EditorCommandKind::EditObjectFootprint,
        source,
        project_id.to_string(),
        Some(scene_id.code().to_string()),
        Some(candidate.kind.code().to_string()),
        vec![GridPos {
            x: candidate.x,
            y: candidate.y,
        }],
        message.clone(),
    );
    let mut transaction = EditTransaction::new(action, scene_id);
    transaction.push(EditOperation::UpdateObject {
        id: candidate.id,
        before,
        after: candidate,
    });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}
