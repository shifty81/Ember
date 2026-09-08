#![expect(
    clippy::too_many_arguments,
    reason = "headless edit commands explicitly receive world, history, project, source, scene, position, and payload"
)]

use crate::{
    EditOperation, EditTransaction, EditorCommand, EditorCommandBus, EditorCommandKind,
    EditorCommandSource, GridPos,
};
use haven_assets::user_asset_registry::authored_object_footprint;
use haven_authoring::apply_terrain_paint_mode_to_map;
use haven_core::{
    GameWorld, ObjectId, ObjectKind, PlacedObject, ProjectSceneId, StablePlaceableAssetRef,
    TerrainPaintMode, TileKind, ZoneKind, MAP_W,
};

#[derive(Clone, Debug, PartialEq)]
pub struct SceneEditOutcome {
    pub command: EditorCommand,
    pub message: String,
}

pub fn paint_scene_tile(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
    tile: TileKind,
) -> Result<SceneEditOutcome, String> {
    paint_scene_tile_with_mode(
        world,
        command_bus,
        project_id,
        source,
        scene_id,
        x,
        y,
        tile,
        TerrainPaintMode::Exact,
    )
}

pub fn paint_scene_tile_with_mode(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
    tile: TileKind,
    mode: TerrainPaintMode,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let before_tiles = scene.map.tiles.clone();
    let previous_tile = scene.map.get(x, y);
    if previous_tile == tile {
        return Err(format!(
            "{} already contains {} at {}, {}",
            scene.name,
            tile.label(),
            x,
            y
        ));
    }

    scene.map.set(x, y, tile);
    let repair = match apply_terrain_paint_mode_to_map(&mut scene.map, x, y, x, y, mode) {
        Ok(report) => report,
        Err(error) => {
            scene.map.tiles = before_tiles;
            return Err(error);
        }
    };

    let mut changed_cells = Vec::new();
    let mut transaction =
        EditTransaction::new(format!("Paint {} stroke", tile.label()), scene_id.clone());
    for (index, (&before, &after)) in before_tiles.iter().zip(scene.map.tiles.iter()).enumerate() {
        if before == after {
            continue;
        }
        let cell_x = (index % MAP_W) as i32;
        let cell_y = (index / MAP_W) as i32;
        changed_cells.push(GridPos {
            x: cell_x,
            y: cell_y,
        });
        transaction.push(EditOperation::SetTile {
            cell: GridPos {
                x: cell_x,
                y: cell_y,
            },
            before,
            after,
        });
    }

    let mut message = if repair.total_mutations() == 0 {
        format!(
            "Painted {} at {}, {} in {} [{}]",
            tile.label(),
            x,
            y,
            scene.name,
            mode.label()
        )
    } else {
        format!(
            "Painted {} at {}, {} in {} [{}; {} neighboring repair(s)]",
            tile.label(),
            x,
            y,
            scene.name,
            mode.label(),
            repair.total_mutations()
        )
    };
    if repair.unsupported_contacts > 0 {
        message.push_str(&format!(
            "; {} unsupported authored contact(s) left for review",
            repair.unsupported_contacts
        ));
    }
    let command = scene_command(
        EditorCommandKind::PaintTerrain,
        source,
        project_id,
        &scene_id,
        Some(tile.code()),
        changed_cells,
        &message,
    );
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn place_scene_object(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
    object: ObjectKind,
) -> Result<SceneEditOutcome, String> {
    place_scene_object_with_footprint(
        world,
        command_bus,
        project_id,
        source,
        scene_id,
        x,
        y,
        object,
        authored_object_footprint(object),
    )
}

pub fn place_scene_object_with_footprint(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
    object: ObjectKind,
    footprint: haven_core::ObjectFootprint,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let mut placed = PlacedObject::new(object, x, y);
    placed.footprint = footprint;
    let placement_issues = scene.map.placement_issues_for_object(placed);
    if !placement_issues.is_empty() {
        return Err(format!(
            "cannot place {} at {}, {} in {}: {}",
            object.label(),
            x,
            y,
            scene.name,
            placement_issues
                .iter()
                .map(|issue| issue.label())
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    let object_id = scene
        .map
        .place_custom_object(placed)
        .ok_or_else(|| "object placement failed after validation".to_string())?;
    let placed = scene
        .map
        .object(object_id)
        .copied()
        .ok_or_else(|| "placed object could not be resolved by stable ID".to_string())?;
    let message = format!(
        "Placed {} ({}) at {}, {} in {}",
        object.label(),
        object_id,
        x,
        y,
        scene.name
    );
    let command = scene_command(
        EditorCommandKind::PlaceObject,
        source,
        project_id,
        &scene_id,
        Some(object.code()),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction = EditTransaction::new(format!("Place {}", object.label()), scene_id);
    transaction.push(EditOperation::InsertObject { object: placed });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn place_scene_pack_asset(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
    fallback_kind: ObjectKind,
    footprint: haven_core::ObjectFootprint,
    asset_ref: StablePlaceableAssetRef,
    initial_state: Option<&str>,
    label: &str,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let placed = PlacedObject::with_footprint(fallback_kind, x, y, footprint);
    let placement_issues = scene.map.placement_issues_for_object(placed);
    if !placement_issues.is_empty() {
        return Err(format!(
            "cannot place {label} at {x}, {y} in {}: {}",
            scene.name,
            placement_issues
                .iter()
                .map(|issue| issue.label())
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    let object_id = scene
        .map
        .place_pack_defined_object_with_state(placed, asset_ref.clone(), initial_state)
        .ok_or_else(|| "pack-defined object placement failed after validation".to_string())?;
    let placed = scene
        .map
        .object(object_id)
        .copied()
        .ok_or_else(|| "placed object could not be resolved by stable ID".to_string())?;
    let message = format!(
        "Placed {label} ({}) at {x}, {y} in {}",
        asset_ref.stable_key(),
        scene.name
    );
    let command = scene_command(
        EditorCommandKind::PlaceObject,
        source,
        project_id,
        &scene_id,
        Some(&asset_ref.stable_key()),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction = EditTransaction::new(format!("Place {label}"), scene_id);
    transaction.push(EditOperation::InsertObject { object: placed });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn move_scene_object(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    object_id: ObjectId,
    x: i32,
    y: i32,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let Some(object) = scene.map.object(object_id).copied() else {
        return Err(format!("{} object {} is missing", scene.name, object_id));
    };
    scene.map.move_object(object_id, x, y).map_err(|issues| {
        format!(
            "cannot move {} to {}, {} in {}: {}",
            object.kind.label(),
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
        object.kind.label(),
        object.x,
        object.y,
        x,
        y,
        scene.name
    );
    let command = scene_command(
        EditorCommandKind::SceneMutation,
        source,
        project_id,
        &scene_id,
        Some(object.kind.code()),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction = EditTransaction::new(format!("Move {}", object.kind.label()), scene_id);
    transaction.push(EditOperation::MoveObject {
        id: object_id,
        before: GridPos {
            x: object.x,
            y: object.y,
        },
        after: GridPos { x, y },
    });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn duplicate_scene_object(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    object_id: ObjectId,
    x: i32,
    y: i32,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let Some(object) = scene.map.object(object_id).copied() else {
        return Err(format!("{} object {} is missing", scene.name, object_id));
    };
    let duplicate = PlacedObject::with_footprint(object.kind, x, y, object.footprint);
    let placement_issues = scene.map.placement_issues_for_object(duplicate);
    if !placement_issues.is_empty() {
        return Err(format!(
            "cannot duplicate {} to {}, {} in {}: {}",
            object.kind.label(),
            x,
            y,
            scene.name,
            placement_issues
                .iter()
                .map(|issue| issue.label())
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }

    let duplicate_id = scene
        .map
        .place_custom_object(duplicate)
        .ok_or_else(|| "object duplication failed after validation".to_string())?;
    let duplicate = scene
        .map
        .object(duplicate_id)
        .copied()
        .ok_or_else(|| "duplicated object could not be resolved by stable ID".to_string())?;
    let message = format!(
        "Duplicated {} as {} to {}, {} in {}",
        object.kind.label(),
        duplicate_id,
        x,
        y,
        scene.name
    );
    let command = scene_command(
        EditorCommandKind::PlaceObject,
        source,
        project_id,
        &scene_id,
        Some(object.kind.code()),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction =
        EditTransaction::new(format!("Duplicate {}", object.kind.label()), scene_id);
    transaction.push(EditOperation::InsertObject { object: duplicate });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn erase_scene_object(
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
    let Some(object_id) = scene.map.object_id_at(x, y) else {
        return Err(format!("{} has no object at {}, {}", scene.name, x, y));
    };
    let removed = scene
        .map
        .remove_object(object_id)
        .ok_or_else(|| format!("{} object {} is missing", scene.name, object_id))?;
    let message = format!(
        "Erased {} at {}, {} in {}",
        removed.kind.label(),
        x,
        y,
        scene.name
    );
    let command = scene_command(
        EditorCommandKind::SceneMutation,
        source,
        project_id,
        &scene_id,
        Some(removed.kind.code()),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction = EditTransaction::new(format!("Erase {}", removed.kind.label()), scene_id);
    transaction.push(EditOperation::RemoveObject { object: removed });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn erase_scene_cell(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
    fill_tile: TileKind,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let previous_tile = scene.map.get(x, y);
    let removed_objects = scene
        .map
        .objects
        .iter()
        .copied()
        .filter(|object| object.contains_tile(x, y))
        .collect::<Vec<_>>();
    if previous_tile == fill_tile && removed_objects.is_empty() {
        return Err(format!(
            "{} already contains {} with no object at {}, {}",
            scene.name,
            fill_tile.label(),
            x,
            y
        ));
    }

    scene.map.set(x, y, fill_tile);
    for object in &removed_objects {
        scene.map.remove_object(object.id);
    }
    let message = format!(
        "Erased cell {}, {} in {} to {}",
        x,
        y,
        scene.name,
        fill_tile.label()
    );
    let command = scene_command(
        EditorCommandKind::SceneMutation,
        source,
        project_id,
        &scene_id,
        Some(fill_tile.code()),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction = EditTransaction::new("Erase terrain stroke", scene_id);
    if previous_tile != fill_tile {
        transaction.push(EditOperation::SetTile {
            cell: GridPos { x, y },
            before: previous_tile,
            after: fill_tile,
        });
    }
    for object in removed_objects {
        transaction.push(EditOperation::RemoveObject { object });
    }
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub fn paint_scene_zone(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    x: i32,
    y: i32,
    zone: ZoneKind,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let previous_zone = scene.zone_at(x, y);
    if previous_zone == zone {
        return Err(format!(
            "{} already has {} zone at {}, {}",
            scene.name,
            zone.label(),
            x,
            y
        ));
    }

    scene.set_zone(x, y, zone);
    let message = format!(
        "Painted {} zone at {}, {} in {}",
        zone.label(),
        x,
        y,
        scene.name
    );
    let command = scene_command(
        EditorCommandKind::AssignRoom,
        source,
        project_id,
        &scene_id,
        Some(zone.code()),
        vec![GridPos { x, y }],
        &message,
    );
    let mut transaction =
        EditTransaction::new(format!("Paint {} zone stroke", zone.label()), scene_id);
    transaction.push(EditOperation::SetZone {
        cell: GridPos { x, y },
        before: previous_zone,
        after: zone,
    });
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

pub(crate) fn scene_mut<'a>(
    world: &'a mut GameWorld,
    scene_id: &ProjectSceneId,
) -> Result<&'a mut haven_core::SceneMap, String> {
    world
        .scene_mut_by_id(scene_id)
        .ok_or_else(|| format!("scene {} is not loaded", scene_id.label()))
}

pub(crate) fn scene_command(
    kind: EditorCommandKind,
    source: EditorCommandSource,
    project_id: &str,
    scene_id: &ProjectSceneId,
    asset_id: Option<&str>,
    grid_cells: Vec<GridPos>,
    message: &str,
) -> EditorCommand {
    EditorCommand::new(
        kind,
        source,
        project_id.to_string(),
        Some(scene_id.code().to_string()),
        asset_id.map(str::to_string),
        grid_cells,
        message.to_string(),
    )
}

#[cfg(test)]
#[path = "scene_edit_tests.rs"]
mod tests;
