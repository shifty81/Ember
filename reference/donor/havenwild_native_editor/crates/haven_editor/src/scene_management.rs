use haven_core::{GameWorld, ProjectSceneId, SceneMap};

use crate::{
    EditOperation, EditTransaction, EditorCommand, EditorCommandBus, EditorCommandKind,
    EditorCommandSource, SceneEditOutcome,
};

pub fn create_project_scene(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene: SceneMap,
) -> Result<SceneEditOutcome, String> {
    if world.scene_by_id(&scene.id).is_some() {
        return Err(format!("scene '{}' already exists", scene.id));
    }
    let index = world.scenes.len();
    world.insert_scene_at(index, scene.clone())?;
    let message = format!("Created scene {} ({})", scene.name, scene.id);
    let command = scene_command(source, project_id, &scene.id, &message);
    let mut transaction = EditTransaction::new("Create scene", scene.id.clone());
    transaction.push(EditOperation::InsertScene { index, scene });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn duplicate_project_scene(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    source_id: &ProjectSceneId,
    replacement: ProjectSceneId,
    display_name: impl Into<String>,
) -> Result<SceneEditOutcome, String> {
    if world.scene_by_id(&replacement).is_some() {
        return Err(format!("scene '{}' already exists", replacement));
    }
    let mut scene = world
        .scene_by_id(source_id)
        .cloned()
        .ok_or_else(|| format!("scene '{}' is not registered", source_id))?;
    scene.id = replacement;
    scene.name = display_name.into();
    create_project_scene(world, command_bus, project_id, source, scene)
}

pub fn rename_project_scene(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    current: &ProjectSceneId,
    replacement: ProjectSceneId,
    display_name: impl Into<String>,
) -> Result<SceneEditOutcome, String> {
    let display_name = display_name.into();
    let before_name = world
        .scene_by_id(current)
        .map(|scene| scene.name.clone())
        .ok_or_else(|| format!("scene '{}' is not registered", current))?;
    if current == &replacement && before_name == display_name {
        return Err("Scene identity and display name are unchanged".to_string());
    }
    let before_id = current.clone();
    world.rename_scene(current, replacement.clone())?;
    world
        .scene_mut_by_id(&replacement)
        .ok_or_else(|| format!("scene '{}' is missing after rename", replacement))?
        .name = display_name.clone();

    let message = format!(
        "Renamed scene {} to {} ({})",
        before_name, display_name, replacement
    );
    let command = scene_command(source, project_id, &replacement, &message);
    let mut transaction = EditTransaction::new("Rename scene", replacement.clone());
    transaction.push(EditOperation::RenameScene {
        before_id,
        after_id: replacement,
        before_name,
        after_name: display_name,
    });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn delete_project_scene(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: &ProjectSceneId,
) -> Result<SceneEditOutcome, String> {
    let index = world
        .scenes
        .position(scene_id)
        .ok_or_else(|| format!("scene '{}' is not registered", scene_id))?;
    let removed = world.remove_scene(scene_id)?;
    let message = format!("Deleted scene {} ({})", removed.name, removed.id);
    let command = scene_command(source, project_id, &removed.id, &message);
    let mut transaction = EditTransaction::new("Delete scene", removed.id.clone());
    transaction.push(EditOperation::RemoveScene {
        index,
        scene: removed,
    });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

fn scene_command(
    source: EditorCommandSource,
    project_id: &str,
    scene_id: &ProjectSceneId,
    message: &str,
) -> EditorCommand {
    EditorCommand::new(
        EditorCommandKind::SceneMutation,
        source,
        project_id.to_string(),
        Some(scene_id.code().to_string()),
        None,
        Vec::new(),
        message.to_string(),
    )
}
