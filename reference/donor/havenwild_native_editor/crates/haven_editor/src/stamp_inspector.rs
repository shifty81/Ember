use haven_core::{GameWorld, PlacedStamp, ProjectSceneId};

use crate::{
    EditOperation, EditTransaction, EditorCommand, EditorCommandBus, EditorCommandKind,
    EditorCommandSource, GridPos, SceneEditOutcome,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StampUpdateRequest {
    pub source: EditorCommandSource,
    pub scene_id: ProjectSceneId,
    pub candidate: PlacedStamp,
    pub minimum_visual_size: (i32, i32),
    pub action: String,
}

impl StampUpdateRequest {
    pub fn new(
        source: EditorCommandSource,
        scene_id: impl Into<ProjectSceneId>,
        candidate: PlacedStamp,
        minimum_visual_size: (i32, i32),
        action: impl Into<String>,
    ) -> Self {
        Self {
            source,
            scene_id: scene_id.into(),
            candidate,
            minimum_visual_size,
            action: action.into(),
        }
    }
}

/// Applies one validated placed-stamp change and records it as a typed transaction.
pub fn update_scene_stamp(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    request: StampUpdateRequest,
) -> Result<SceneEditOutcome, String> {
    let StampUpdateRequest {
        source,
        scene_id,
        candidate,
        minimum_visual_size,
        action,
    } = request;
    let scene = world
        .scene_mut_by_id(&scene_id)
        .ok_or_else(|| format!("scene {} is not loaded", scene_id.label()))?;
    let index = scene
        .map
        .stamp_index(candidate.id)
        .ok_or_else(|| format!("{} stamp {} is missing", scene.name, candidate.id))?;
    let before = scene.map.stamps[index].clone();
    if before == candidate {
        return Err("Stamp properties are unchanged".to_string());
    }
    if before.id != candidate.id || before.stamp_key != candidate.stamp_key {
        return Err("Stamp identity and asset key cannot be changed by the inspector".to_string());
    }
    if candidate.footprint.visual_w < minimum_visual_size.0
        || candidate.footprint.visual_h < minimum_visual_size.1
    {
        return Err(format!(
            "stamp {} minimum size is {}x{}",
            candidate.stamp_key, minimum_visual_size.0, minimum_visual_size.1
        ));
    }
    let issues = scene
        .map
        .placement_issues_for_stamp_excluding(&candidate, Some(index));
    if !issues.is_empty() {
        return Err(format!(
            "cannot update {} in {}: {}",
            candidate.stamp_key,
            scene.name,
            issues
                .iter()
                .map(|issue| issue.label())
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }

    scene.map.stamps[index] = candidate.clone();
    let message = format!(
        "{} {} ({}) in {}",
        action, candidate.stamp_key, candidate.id, scene.name
    );
    let command = EditorCommand::new(
        EditorCommandKind::EditStamp,
        source,
        project_id.to_string(),
        Some(scene_id.code().to_string()),
        Some(candidate.stamp_key.clone()),
        vec![GridPos {
            x: candidate.x,
            y: candidate.y,
        }],
        message.clone(),
    );
    let mut transaction = EditTransaction::new(action, scene_id);
    transaction.push(EditOperation::RemoveStamp { stamp: before });
    transaction.push(EditOperation::InsertStamp { stamp: candidate });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}
