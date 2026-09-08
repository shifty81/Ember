use std::collections::HashSet;

use haven_authoring::{
    EditOperation, EditTransaction, EditorCommand, EditorCommandBus, EditorCommandKind,
    EditorCommandSource, GridPos, GridRect,
};
use haven_core::{GameWorld, PlacedObject, PlacedStamp};
use haven_world::scene_rectangles::{SceneRectangleAssignmentsFile, SceneRectangleManifest};

use super::{
    resolve_world_surface_cell, validate_world_surface_footprint, WorldSurfaceClipboard,
    WorldSurfaceClipboardCell, WorldSurfaceClipboardObject, WorldSurfaceClipboardStamp,
    WorldSurfaceEditOutcome,
};

pub fn copy_world_surface_rectangle(
    manifest: &SceneRectangleManifest,
    assignments: &SceneRectangleAssignmentsFile,
    world: &GameWorld,
    landmass_id: i32,
    rect: GridRect,
) -> Result<WorldSurfaceClipboard, String> {
    let rect = GridRect::from_points(rect.min, rect.max);
    let mut cells = Vec::new();
    let mut objects = Vec::new();
    let mut stamps = Vec::new();
    let mut visited_scenes = HashSet::new();

    for global in rect.cells() {
        let Ok(address) =
            resolve_world_surface_cell(manifest, assignments, world, landmass_id, global)
        else {
            continue;
        };
        let scene = world
            .scene_by_id(&address.scene_id)
            .ok_or_else(|| format!("scene {} is not loaded", address.scene_id.label()))?;
        cells.push(WorldSurfaceClipboardCell {
            offset: GridPos {
                x: global.x - rect.min.x,
                y: global.y - rect.min.y,
            },
            tile: scene.map.get(address.local.x, address.local.y),
            height: scene.map.get_height(address.local.x, address.local.y),
            structural_level: scene
                .map
                .structural_level_storage(address.local.x, address.local.y),
            zone: scene.zone_at(address.local.x, address.local.y),
        });
        if visited_scenes.insert(address.scene_id.code().to_string()) {
            let origin = address.partition_origin;
            for object in &scene.map.objects {
                let global_anchor = GridPos {
                    x: origin.x + object.x,
                    y: origin.y + object.y,
                };
                if rect.contains(global_anchor) {
                    objects.push(WorldSurfaceClipboardObject {
                        offset: GridPos {
                            x: global_anchor.x - rect.min.x,
                            y: global_anchor.y - rect.min.y,
                        },
                        kind: object.kind,
                        footprint: object.footprint,
                    });
                }
            }
            for stamp in &scene.map.stamps {
                let global_anchor = GridPos {
                    x: origin.x + stamp.x,
                    y: origin.y + stamp.y,
                };
                if rect.contains(global_anchor) {
                    stamps.push(WorldSurfaceClipboardStamp {
                        offset: GridPos {
                            x: global_anchor.x - rect.min.x,
                            y: global_anchor.y - rect.min.y,
                        },
                        stamp_key: stamp.stamp_key.clone(),
                        footprint: stamp.footprint,
                    });
                }
            }
        }
    }

    let clipboard = WorldSurfaceClipboard {
        width: rect.width(),
        height: rect.height(),
        cells,
        objects,
        stamps,
    };
    if clipboard.is_empty() {
        return Err("World selection contains no assigned surface cells".to_string());
    }
    Ok(clipboard)
}

#[allow(clippy::too_many_arguments)]
pub fn paste_world_surface_clipboard(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    manifest: &SceneRectangleManifest,
    assignments: &SceneRectangleAssignmentsFile,
    landmass_id: i32,
    anchor: GridPos,
    clipboard: &WorldSurfaceClipboard,
) -> Result<WorldSurfaceEditOutcome, String> {
    let backup = world.clone();
    let mut batch = haven_authoring::EditTransactionBatch::new(format!(
        "Paste {}x{} world selection",
        clipboard.width, clipboard.height
    ));
    let mut changed_cells = Vec::new();

    let result = (|| -> Result<(), String> {
        for cell in &clipboard.cells {
            let global = GridPos {
                x: anchor.x + cell.offset.x,
                y: anchor.y + cell.offset.y,
            };
            let address =
                resolve_world_surface_cell(manifest, assignments, world, landmass_id, global)?;
            let scene = world
                .scene_mut_by_id(&address.scene_id)
                .ok_or_else(|| format!("scene {} is not loaded", address.scene_id.label()))?;
            let mut transaction =
                EditTransaction::new("Paste world cell data", address.scene_id.clone());

            let before_tile = scene.map.get(address.local.x, address.local.y);
            if before_tile != cell.tile {
                scene.map.set(address.local.x, address.local.y, cell.tile);
                transaction.push(EditOperation::SetTile {
                    cell: address.local,
                    before: before_tile,
                    after: cell.tile,
                });
            }
            let before_height = scene.map.get_height(address.local.x, address.local.y);
            if before_height != cell.height {
                scene
                    .map
                    .set_height(address.local.x, address.local.y, cell.height);
                transaction.push(EditOperation::SetHeight {
                    cell: address.local,
                    before: before_height,
                    after: cell.height,
                });
            }
            let before_structural_level = scene
                .map
                .structural_level_storage(address.local.x, address.local.y);
            if before_structural_level != cell.structural_level {
                scene.map.set_structural_level(
                    address.local.x,
                    address.local.y,
                    (cell.structural_level <= haven_core::MAX_STRUCTURAL_LEVEL)
                        .then_some(cell.structural_level),
                );
                transaction.push(EditOperation::SetStructuralLevel {
                    cell: address.local,
                    before: before_structural_level,
                    after: cell.structural_level,
                });
            }
            let before_zone = scene.zone_at(address.local.x, address.local.y);
            if before_zone != cell.zone {
                scene.set_zone(address.local.x, address.local.y, cell.zone);
                transaction.push(EditOperation::SetZone {
                    cell: address.local,
                    before: before_zone,
                    after: cell.zone,
                });
            }
            if !transaction.is_empty() {
                batch.push(transaction);
                changed_cells.push(global);
            }
        }

        for object in &clipboard.objects {
            let global = GridPos {
                x: anchor.x + object.offset.x,
                y: anchor.y + object.offset.y,
            };
            let address = validate_world_surface_footprint(
                manifest,
                assignments,
                world,
                landmass_id,
                global,
                object.footprint,
            )?;
            let scene = world
                .scene_mut_by_id(&address.scene_id)
                .ok_or_else(|| format!("scene {} is not loaded", address.scene_id.label()))?;
            let mut placed = PlacedObject::with_footprint(
                object.kind,
                address.local.x,
                address.local.y,
                object.footprint,
            );
            let issues = scene.map.placement_issues_for_object(placed);
            if !issues.is_empty() {
                return Err(format!(
                    "cannot paste {} at {}, {}: {}",
                    object.kind.label(),
                    global.x,
                    global.y,
                    issues
                        .iter()
                        .map(|issue| issue.label())
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
            let id = scene
                .map
                .place_custom_object(placed)
                .ok_or_else(|| "object paste failed after validation".to_string())?;
            placed.id = id;
            let mut transaction =
                EditTransaction::new("Paste world objects", address.scene_id.clone());
            transaction.push(EditOperation::InsertObject { object: placed });
            batch.push(transaction);
            changed_cells.push(global);
        }

        for stamp in &clipboard.stamps {
            let global = GridPos {
                x: anchor.x + stamp.offset.x,
                y: anchor.y + stamp.offset.y,
            };
            let address = validate_world_surface_footprint(
                manifest,
                assignments,
                world,
                landmass_id,
                global,
                stamp.footprint,
            )?;
            let scene = world
                .scene_mut_by_id(&address.scene_id)
                .ok_or_else(|| format!("scene {} is not loaded", address.scene_id.label()))?;
            let placed = PlacedStamp::new(
                stamp.stamp_key.clone(),
                address.local.x,
                address.local.y,
                stamp.footprint,
            );
            let id = scene.map.place_stamp(placed).map_err(|issues| {
                format!(
                    "cannot paste stamp {} at {}, {}: {}",
                    stamp.stamp_key,
                    global.x,
                    global.y,
                    issues
                        .iter()
                        .map(|issue| issue.label())
                        .collect::<Vec<_>>()
                        .join("; ")
                )
            })?;
            let placed = scene
                .map
                .stamp(id)
                .cloned()
                .ok_or_else(|| "pasted stamp could not be resolved".to_string())?;
            let mut transaction =
                EditTransaction::new("Paste world stamps", address.scene_id.clone());
            transaction.push(EditOperation::InsertStamp { stamp: placed });
            batch.push(transaction);
            changed_cells.push(global);
        }
        Ok(())
    })();

    if let Err(error) = result {
        *world = backup;
        return Err(error);
    }
    if batch.is_empty() {
        return Err("Paste would not change the world".to_string());
    }

    changed_cells.sort();
    changed_cells.dedup();
    let scene_count = batch.scene_count();
    let operation_count = batch.operation_count();
    let message = format!("Pasted {}", clipboard.summary());
    let command = EditorCommand::new(
        EditorCommandKind::SceneMutation,
        source,
        project_id.to_string(),
        None,
        Some("world_surface_clipboard".to_string()),
        changed_cells.clone(),
        message.clone(),
    );
    command_bus.record_transaction_batch(command, batch);
    Ok(WorldSurfaceEditOutcome {
        message,
        operation_count,
        scene_count,
        global_cells: changed_cells,
    })
}
