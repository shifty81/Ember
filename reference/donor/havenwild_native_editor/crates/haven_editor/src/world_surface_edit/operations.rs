use std::collections::{HashSet, VecDeque};

use haven_authoring::{
    EditOperation, EditTransaction, EditTransactionBatch, EditorCommand, EditorCommandBus,
    EditorCommandKind, EditorCommandSource, GridPos, GridRect,
};
use haven_core::{GameWorld, ProjectSceneId, MAP_H, MAP_W};
use haven_world::scene_rectangles::{SceneRectangleAssignmentsFile, SceneRectangleManifest};

use super::{
    is_surface_rectangle, rectangle_origin, resolve_world_surface_cell, world_surface_bounds,
    WorldSurfaceEditOutcome, WorldSurfaceLayer, WorldSurfaceValue,
};

#[allow(clippy::too_many_arguments)]
pub fn paint_world_surface_cells(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    manifest: &SceneRectangleManifest,
    assignments: &SceneRectangleAssignmentsFile,
    landmass_id: i32,
    global_cells: impl IntoIterator<Item = GridPos>,
    value: WorldSurfaceValue,
    label: impl Into<String>,
) -> Result<WorldSurfaceEditOutcome, String> {
    let label = label.into();
    let backup = world.clone();
    let mut batch = EditTransactionBatch::new(label.clone());
    let mut changed_global: Vec<GridPos> = global_cells.into_iter().collect();
    changed_global.sort();
    changed_global.dedup();

    let result = (|| -> Result<(), String> {
        let mut actual_changes = Vec::new();
        for global in changed_global.iter().copied() {
            let address =
                resolve_world_surface_cell(manifest, assignments, world, landmass_id, global)?;
            let scene = world
                .scene_mut_by_id(&address.scene_id)
                .ok_or_else(|| format!("scene {} is not loaded", address.scene_id.label()))?;
            let mut transaction = EditTransaction::new(label.clone(), address.scene_id.clone());
            match value {
                WorldSurfaceValue::Terrain(after) => {
                    let before = scene.map.get(address.local.x, address.local.y);
                    if before == after {
                        continue;
                    }
                    scene.map.set(address.local.x, address.local.y, after);
                    transaction.push(EditOperation::SetTile {
                        cell: address.local,
                        before,
                        after,
                    });
                }
                WorldSurfaceValue::Zone(after) => {
                    let before = scene.zone_at(address.local.x, address.local.y);
                    if before == after {
                        continue;
                    }
                    scene.set_zone(address.local.x, address.local.y, after);
                    transaction.push(EditOperation::SetZone {
                        cell: address.local,
                        before,
                        after,
                    });
                }
                WorldSurfaceValue::StructuralLevel(after) => {
                    let after = after.min(haven_core::MAX_STRUCTURAL_LEVEL);
                    let before = scene
                        .map
                        .structural_level_storage(address.local.x, address.local.y);
                    if before == after {
                        continue;
                    }
                    scene
                        .map
                        .set_structural_level(address.local.x, address.local.y, Some(after));
                    transaction.push(EditOperation::SetStructuralLevel {
                        cell: address.local,
                        before,
                        after,
                    });
                }
            }
            batch.push(transaction);
            actual_changes.push(global);
        }
        changed_global = actual_changes;
        Ok(())
    })();

    if let Err(error) = result {
        *world = backup;
        return Err(error);
    }
    if batch.is_empty() {
        return Err("Global edit would not change the world".to_string());
    }

    let scene_count = batch.scene_count();
    let operation_count = batch.operation_count();
    let message = format!(
        "{} across {} global cell{} in {} partition{}",
        label,
        operation_count,
        if operation_count == 1 { "" } else { "s" },
        scene_count,
        if scene_count == 1 { "" } else { "s" }
    );
    let command = EditorCommand::new(
        match value.layer() {
            WorldSurfaceLayer::Terrain => EditorCommandKind::PaintTerrain,
            WorldSurfaceLayer::Zones => EditorCommandKind::AssignRoom,
            WorldSurfaceLayer::StructuralLevels => EditorCommandKind::SetStructuralLevel,
        },
        source,
        project_id.to_string(),
        None,
        Some(value.label()),
        changed_global.clone(),
        message.clone(),
    );
    command_bus.record_transaction_batch(command, batch);
    Ok(WorldSurfaceEditOutcome {
        message,
        operation_count,
        scene_count,
        global_cells: changed_global,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn paint_world_surface_rectangle(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    manifest: &SceneRectangleManifest,
    assignments: &SceneRectangleAssignmentsFile,
    landmass_id: i32,
    rect: GridRect,
    value: WorldSurfaceValue,
) -> Result<WorldSurfaceEditOutcome, String> {
    paint_world_surface_cells(
        world,
        command_bus,
        project_id,
        source,
        manifest,
        assignments,
        landmass_id,
        rect.cells(),
        value,
        format!("Rectangle paint {}", value.label()),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn flood_fill_world_surface(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    manifest: &SceneRectangleManifest,
    assignments: &SceneRectangleAssignmentsFile,
    landmass_id: i32,
    start: GridPos,
    value: WorldSurfaceValue,
) -> Result<WorldSurfaceEditOutcome, String> {
    let source_address =
        resolve_world_surface_cell(manifest, assignments, world, landmass_id, start)?;
    let source_scene = world
        .scene_by_id(&source_address.scene_id)
        .ok_or_else(|| format!("scene {} is not loaded", source_address.scene_id.label()))?;
    let source_value = match value {
        WorldSurfaceValue::Terrain(_) => WorldSurfaceValue::Terrain(
            source_scene
                .map
                .get(source_address.local.x, source_address.local.y),
        ),
        WorldSurfaceValue::Zone(_) => WorldSurfaceValue::Zone(
            source_scene.zone_at(source_address.local.x, source_address.local.y),
        ),
        WorldSurfaceValue::StructuralLevel(_) => WorldSurfaceValue::StructuralLevel(
            source_scene
                .map
                .get_structural_level(source_address.local.x, source_address.local.y)
                .unwrap_or(0),
        ),
    };
    if source_value == value {
        return Err(format!(
            "Flood-fill source already contains {}",
            value.label()
        ));
    }
    let bounds = world_surface_bounds(manifest, landmass_id)
        .ok_or_else(|| format!("landmass {} has no surface bounds", landmass_id))?;
    let mut queue = VecDeque::from([start]);
    let mut visited = HashSet::new();
    let mut cells = Vec::new();
    while let Some(global) = queue.pop_front() {
        if global.x < bounds.min.x
            || global.y < bounds.min.y
            || global.x > bounds.max.x
            || global.y > bounds.max.y
            || !visited.insert(global)
        {
            continue;
        }
        let Ok(address) =
            resolve_world_surface_cell(manifest, assignments, world, landmass_id, global)
        else {
            continue;
        };
        let Some(scene) = world.scene_by_id(&address.scene_id) else {
            continue;
        };
        let matches = match source_value {
            WorldSurfaceValue::Terrain(tile) => {
                scene.map.get(address.local.x, address.local.y) == tile
            }
            WorldSurfaceValue::Zone(zone) => {
                scene.zone_at(address.local.x, address.local.y) == zone
            }
            WorldSurfaceValue::StructuralLevel(level) => {
                scene
                    .map
                    .get_structural_level(address.local.x, address.local.y)
                    .unwrap_or(0)
                    == level
            }
        };
        if !matches {
            continue;
        }
        cells.push(global);
        queue.push_back(GridPos {
            x: global.x - 1,
            y: global.y,
        });
        queue.push_back(GridPos {
            x: global.x + 1,
            y: global.y,
        });
        queue.push_back(GridPos {
            x: global.x,
            y: global.y - 1,
        });
        queue.push_back(GridPos {
            x: global.x,
            y: global.y + 1,
        });
    }
    paint_world_surface_cells(
        world,
        command_bus,
        project_id,
        source,
        manifest,
        assignments,
        landmass_id,
        cells,
        value,
        format!("Global flood fill {}", value.label()),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn adjust_world_structural_levels(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    manifest: &SceneRectangleManifest,
    assignments: &SceneRectangleAssignmentsFile,
    landmass_id: i32,
    global_cells: impl IntoIterator<Item = GridPos>,
    delta: i8,
    label: impl Into<String>,
) -> Result<WorldSurfaceEditOutcome, String> {
    if delta == 0 {
        return Err("Structural level adjustment must be non-zero".to_string());
    }
    let label = label.into();
    let backup = world.clone();
    let mut batch = EditTransactionBatch::new(label.clone());
    let mut changed_global: Vec<GridPos> = global_cells.into_iter().collect();
    changed_global.sort();
    changed_global.dedup();
    let mut actual_changes = Vec::new();

    let result = (|| -> Result<(), String> {
        for global in changed_global.iter().copied() {
            let address =
                resolve_world_surface_cell(manifest, assignments, world, landmass_id, global)?;
            let scene = world
                .scene_mut_by_id(&address.scene_id)
                .ok_or_else(|| format!("scene {} is not loaded", address.scene_id.label()))?;
            let before = scene
                .map
                .structural_level_storage(address.local.x, address.local.y);
            let current = scene
                .map
                .get_structural_level(address.local.x, address.local.y)
                .unwrap_or(0);
            let after = (i16::from(current) + i16::from(delta))
                .clamp(0, i16::from(haven_core::MAX_STRUCTURAL_LEVEL))
                as u8;
            if current == after {
                continue;
            }
            scene
                .map
                .set_structural_level(address.local.x, address.local.y, Some(after));
            let mut transaction = EditTransaction::new(label.clone(), address.scene_id.clone());
            transaction.push(EditOperation::SetStructuralLevel {
                cell: address.local,
                before,
                after,
            });
            batch.push(transaction);
            actual_changes.push(global);
        }
        Ok(())
    })();

    if let Err(error) = result {
        *world = backup;
        return Err(error);
    }
    if batch.is_empty() {
        return Err("Structural level adjustment would not change the world".to_string());
    }
    let scene_count = batch.scene_count();
    let operation_count = batch.operation_count();
    let message = format!(
        "{} across {} global cell{} in {} partition{}",
        label,
        operation_count,
        if operation_count == 1 { "" } else { "s" },
        scene_count,
        if scene_count == 1 { "" } else { "s" }
    );
    let command = EditorCommand::new(
        EditorCommandKind::SetStructuralLevel,
        source,
        project_id.to_string(),
        None,
        Some("structural_levels".to_string()),
        actual_changes.clone(),
        message.clone(),
    );
    command_bus.record_transaction_batch(command, batch);
    Ok(WorldSurfaceEditOutcome {
        message,
        operation_count,
        scene_count,
        global_cells: actual_changes,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn replace_world_surface_value(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    manifest: &SceneRectangleManifest,
    assignments: &SceneRectangleAssignmentsFile,
    landmass_id: i32,
    sample: GridPos,
    value: WorldSurfaceValue,
) -> Result<WorldSurfaceEditOutcome, String> {
    let address = resolve_world_surface_cell(manifest, assignments, world, landmass_id, sample)?;
    let scene = world
        .scene_by_id(&address.scene_id)
        .ok_or_else(|| format!("scene {} is not loaded", address.scene_id.label()))?;
    let source_value = match value {
        WorldSurfaceValue::Terrain(_) => {
            WorldSurfaceValue::Terrain(scene.map.get(address.local.x, address.local.y))
        }
        WorldSurfaceValue::Zone(_) => {
            WorldSurfaceValue::Zone(scene.zone_at(address.local.x, address.local.y))
        }
        WorldSurfaceValue::StructuralLevel(_) => WorldSurfaceValue::StructuralLevel(
            scene
                .map
                .get_structural_level(address.local.x, address.local.y)
                .unwrap_or(0),
        ),
    };
    if source_value == value {
        return Err(format!(
            "Selected landmass already uses {} at the sample",
            value.label()
        ));
    }
    let mut cells = Vec::new();
    for rectangle in manifest
        .scene_rectangles
        .iter()
        .filter(|rectangle| rectangle.landmass_id == landmass_id && is_surface_rectangle(rectangle))
    {
        let origin = rectangle_origin(rectangle);
        let assignment = match assignments.assignment_for_rectangle(&rectangle.scene_id) {
            Some(assignment) => assignment,
            None => continue,
        };
        let scene_id = ProjectSceneId::new(assignment.scene_code.as_str());
        let Some(scene) = world.scene_by_id(&scene_id) else {
            continue;
        };
        let width = rectangle.tile_size[0].min(MAP_W as i32).max(1);
        let height = rectangle.tile_size[1].min(MAP_H as i32).max(1);
        for y in 0..height {
            for x in 0..width {
                let matches = match source_value {
                    WorldSurfaceValue::Terrain(tile) => scene.map.get(x, y) == tile,
                    WorldSurfaceValue::Zone(zone) => scene.zone_at(x, y) == zone,
                    WorldSurfaceValue::StructuralLevel(level) => {
                        scene.map.get_structural_level(x, y).unwrap_or(0) == level
                    }
                };
                if matches {
                    cells.push(GridPos {
                        x: origin.x + x,
                        y: origin.y + y,
                    });
                }
            }
        }
    }
    paint_world_surface_cells(
        world,
        command_bus,
        project_id,
        source,
        manifest,
        assignments,
        landmass_id,
        cells,
        value,
        format!("Replace {} with {}", source_value.label(), value.label()),
    )
}
