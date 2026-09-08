use std::collections::VecDeque;

use haven_authoring::{
    EditOperation, EditTransaction, EditorCommand, EditorCommandBus, EditorCommandKind,
    EditorCommandSource, GridPos, GridRect, SceneAuthoringLayer, SelectionItem,
};
use haven_core::{GameWorld, ProjectSceneId, SceneMap, TileKind, ZoneId, ZoneKind, MAP_H, MAP_W};

use crate::SceneEditOutcome;

pub fn selection_bounds_for_items(scene: &SceneMap, items: &[SelectionItem]) -> Option<GridRect> {
    let mut bounds: Option<GridRect> = None;
    for item in items {
        let item_bounds = match item {
            SelectionItem::Tile(cell) | SelectionItem::ZoneCell { cell, .. } => {
                Some(GridRect::single(*cell))
            }
            SelectionItem::Object(id) => scene.map.object(*id).map(|object| {
                let (x, y, w, h) = object.visual_rect();
                GridRect::from_points(
                    GridPos { x, y },
                    GridPos {
                        x: x + w - 1,
                        y: y + h - 1,
                    },
                )
            }),
            SelectionItem::Stamp(id) => scene.map.stamp(*id).map(|stamp| {
                let (x, y, w, h) = stamp.visual_rect();
                GridRect::from_points(
                    GridPos { x, y },
                    GridPos {
                        x: x + w - 1,
                        y: y + h - 1,
                    },
                )
            }),
            SelectionItem::Transition(id) => scene.transition(*id).map(|transition| {
                GridRect::from_points(
                    GridPos {
                        x: transition.x,
                        y: transition.y,
                    },
                    GridPos {
                        x: transition.x + transition.w - 1,
                        y: transition.y + transition.h - 1,
                    },
                )
            }),
            SelectionItem::Scene(_) | SelectionItem::RegionNode(_) => None,
        };
        if let Some(item_bounds) = item_bounds {
            bounds = Some(match bounds {
                Some(existing) => GridRect::from_points(
                    GridPos {
                        x: existing.min.x.min(item_bounds.min.x),
                        y: existing.min.y.min(item_bounds.min.y),
                    },
                    GridPos {
                        x: existing.max.x.max(item_bounds.max.x),
                        y: existing.max.y.max(item_bounds.max.y),
                    },
                ),
                None => item_bounds,
            });
        }
    }
    bounds
}

pub fn selection_items_in_rect(
    scene: &SceneMap,
    layer: SceneAuthoringLayer,
    rect: GridRect,
) -> Vec<SelectionItem> {
    let rect = rect.clamped(MAP_W as i32, MAP_H as i32);
    match layer {
        SceneAuthoringLayer::Terrain => rect.cells().into_iter().map(SelectionItem::Tile).collect(),
        SceneAuthoringLayer::Zones => rect
            .cells()
            .into_iter()
            .map(|cell| SelectionItem::ZoneCell {
                id: ZoneId::for_cell(cell.x, cell.y, MAP_W),
                cell,
            })
            .collect(),
        SceneAuthoringLayer::Objects => scene
            .map
            .objects
            .iter()
            .filter_map(|object| {
                let (x, y, w, h) = object.visual_rect();
                let object_rect = GridRect::from_points(
                    GridPos { x, y },
                    GridPos {
                        x: x + w.max(1) - 1,
                        y: y + h.max(1) - 1,
                    },
                );
                rect.intersects(object_rect)
                    .then_some(SelectionItem::Object(object.id))
            })
            .chain(scene.map.stamps.iter().filter_map(|stamp| {
                let (x, y, w, h) = stamp.visual_rect();
                let stamp_rect = GridRect::from_points(
                    GridPos { x, y },
                    GridPos {
                        x: x + w.max(1) - 1,
                        y: y + h.max(1) - 1,
                    },
                );
                rect.intersects(stamp_rect)
                    .then_some(SelectionItem::Stamp(stamp.id))
            }))
            .collect(),
        SceneAuthoringLayer::Transitions => scene
            .transitions
            .iter()
            .filter_map(|transition| {
                let transition_rect = GridRect::from_points(
                    GridPos {
                        x: transition.x,
                        y: transition.y,
                    },
                    GridPos {
                        x: transition.x + transition.w.max(1) - 1,
                        y: transition.y + transition.h.max(1) - 1,
                    },
                );
                rect.intersects(transition_rect)
                    .then_some(SelectionItem::Transition(transition.id))
            })
            .collect(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn paint_scene_rectangle(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    rect: GridRect,
    layer: SceneAuthoringLayer,
    tile: TileKind,
    zone: ZoneKind,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let rect = rect.clamped(MAP_W as i32, MAP_H as i32);
    let scene = scene_mut(world, &scene_id)?;
    let mut transaction = EditTransaction::new(
        match layer {
            SceneAuthoringLayer::Terrain => format!("Rectangle paint {}", tile.label()),
            SceneAuthoringLayer::Zones => format!("Rectangle paint {} zone", zone.label()),
            _ => return Err("Rectangle tool is available for terrain and zones".to_string()),
        },
        scene_id.clone(),
    );

    for cell in rect.cells() {
        match layer {
            SceneAuthoringLayer::Terrain => {
                let before = scene.map.get(cell.x, cell.y);
                if before != tile {
                    scene.map.set(cell.x, cell.y, tile);
                    transaction.push(EditOperation::SetTile {
                        cell,
                        before,
                        after: tile,
                    });
                }
            }
            SceneAuthoringLayer::Zones => {
                let before = scene.zone_at(cell.x, cell.y);
                if before != zone {
                    scene.set_zone(cell.x, cell.y, zone);
                    transaction.push(EditOperation::SetZone {
                        cell,
                        before,
                        after: zone,
                    });
                }
            }
            _ => unreachable!(),
        }
    }
    record_bulk_transaction(
        command_bus,
        project_id,
        source,
        &scene_id,
        transaction,
        EditorCommandKind::SceneMutation,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn flood_fill_scene(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    start: GridPos,
    layer: SceneAuthoringLayer,
    tile: TileKind,
    zone: ZoneKind,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    if !cell_in_bounds(start) {
        return Err("Flood-fill start is outside the scene".to_string());
    }
    let scene = scene_mut(world, &scene_id)?;
    let mut queue = VecDeque::from([start]);
    let mut visited = vec![false; MAP_W * MAP_H];
    let mut transaction = match layer {
        SceneAuthoringLayer::Terrain => {
            let source_tile = scene.map.get(start.x, start.y);
            if source_tile == tile {
                return Err(format!(
                    "Flood-fill source already contains {}",
                    tile.label()
                ));
            }
            EditTransaction::new(format!("Flood fill {}", tile.label()), scene_id.clone())
        }
        SceneAuthoringLayer::Zones => {
            let source_zone = scene.zone_at(start.x, start.y);
            if source_zone == zone {
                return Err(format!(
                    "Flood-fill source already contains {}",
                    zone.label()
                ));
            }
            EditTransaction::new(
                format!("Flood fill {} zone", zone.label()),
                scene_id.clone(),
            )
        }
        _ => return Err("Flood fill is available for terrain and zones".to_string()),
    };
    let source_tile = scene.map.get(start.x, start.y);
    let source_zone = scene.zone_at(start.x, start.y);

    while let Some(cell) = queue.pop_front() {
        if !cell_in_bounds(cell) {
            continue;
        }
        let index = cell.y as usize * MAP_W + cell.x as usize;
        if visited[index] {
            continue;
        }
        visited[index] = true;
        let matches_source = match layer {
            SceneAuthoringLayer::Terrain => scene.map.get(cell.x, cell.y) == source_tile,
            SceneAuthoringLayer::Zones => scene.zone_at(cell.x, cell.y) == source_zone,
            _ => false,
        };
        if !matches_source {
            continue;
        }

        match layer {
            SceneAuthoringLayer::Terrain => {
                scene.map.set(cell.x, cell.y, tile);
                transaction.push(EditOperation::SetTile {
                    cell,
                    before: source_tile,
                    after: tile,
                });
            }
            SceneAuthoringLayer::Zones => {
                scene.set_zone(cell.x, cell.y, zone);
                transaction.push(EditOperation::SetZone {
                    cell,
                    before: source_zone,
                    after: zone,
                });
            }
            _ => unreachable!(),
        }
        queue.push_back(GridPos {
            x: cell.x + 1,
            y: cell.y,
        });
        queue.push_back(GridPos {
            x: cell.x - 1,
            y: cell.y,
        });
        queue.push_back(GridPos {
            x: cell.x,
            y: cell.y + 1,
        });
        queue.push_back(GridPos {
            x: cell.x,
            y: cell.y - 1,
        });
    }

    record_bulk_transaction(
        command_bus,
        project_id,
        source,
        &scene_id,
        transaction,
        EditorCommandKind::SceneMutation,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn replace_scene_value(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    sample: GridPos,
    layer: SceneAuthoringLayer,
    tile: TileKind,
    zone: ZoneKind,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    if !cell_in_bounds(sample) {
        return Err("Replace sample is outside the scene".to_string());
    }
    let scene = scene_mut(world, &scene_id)?;
    let source_tile = scene.map.get(sample.x, sample.y);
    let source_zone = scene.zone_at(sample.x, sample.y);
    if layer == SceneAuthoringLayer::Terrain && source_tile == tile {
        return Err(format!("Scene already uses {} at the sample", tile.label()));
    }
    if layer == SceneAuthoringLayer::Zones && source_zone == zone {
        return Err(format!("Scene already uses {} at the sample", zone.label()));
    }
    let mut transaction = match layer {
        SceneAuthoringLayer::Terrain => EditTransaction::new(
            format!("Replace {} with {}", source_tile.label(), tile.label()),
            scene_id.clone(),
        ),
        SceneAuthoringLayer::Zones => EditTransaction::new(
            format!("Replace {} with {} zone", source_zone.label(), zone.label()),
            scene_id.clone(),
        ),
        _ => return Err("Replace is available for terrain and zones".to_string()),
    };

    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            let cell = GridPos { x, y };
            match layer {
                SceneAuthoringLayer::Terrain if scene.map.get(x, y) == source_tile => {
                    scene.map.set(x, y, tile);
                    transaction.push(EditOperation::SetTile {
                        cell,
                        before: source_tile,
                        after: tile,
                    });
                }
                SceneAuthoringLayer::Zones if scene.zone_at(x, y) == source_zone => {
                    scene.set_zone(x, y, zone);
                    transaction.push(EditOperation::SetZone {
                        cell,
                        before: source_zone,
                        after: zone,
                    });
                }
                _ => {}
            }
        }
    }

    record_bulk_transaction(
        command_bus,
        project_id,
        source,
        &scene_id,
        transaction,
        EditorCommandKind::SceneMutation,
    )
}

fn record_bulk_transaction(
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: &ProjectSceneId,
    transaction: EditTransaction,
    kind: EditorCommandKind,
) -> Result<SceneEditOutcome, String> {
    if transaction.is_empty() {
        return Err("Edit would not change the scene".to_string());
    }
    let cells = transaction_cells(&transaction);
    let message = format!(
        "{} in {} ({} operation{})",
        transaction.label,
        scene_id.label(),
        transaction.operation_count(),
        if transaction.operation_count() == 1 {
            ""
        } else {
            "s"
        }
    );
    let command = EditorCommand::new(
        kind,
        source,
        project_id.to_string(),
        Some(scene_id.code().to_string()),
        None,
        cells,
        message.clone(),
    );
    command_bus.record_transaction(command.clone(), transaction);
    Ok(SceneEditOutcome { command, message })
}

fn transaction_cells(transaction: &EditTransaction) -> Vec<GridPos> {
    transaction
        .operations
        .iter()
        .filter_map(|operation| match operation {
            EditOperation::SetTile { cell, .. }
            | EditOperation::SetHeight { cell, .. }
            | EditOperation::SetStructuralLevel { cell, .. }
            | EditOperation::SetAutotileOverride { cell, .. }
            | EditOperation::SetZone { cell, .. } => Some(*cell),
            EditOperation::InsertObject { object } | EditOperation::RemoveObject { object } => {
                Some(GridPos {
                    x: object.x,
                    y: object.y,
                })
            }
            EditOperation::MoveObject { after, .. } => Some(*after),
            EditOperation::UpdateObject { after, .. } => Some(GridPos {
                x: after.x,
                y: after.y,
            }),
            EditOperation::InsertTransition { transition }
            | EditOperation::RemoveTransition { transition } => Some(GridPos {
                x: transition.x,
                y: transition.y,
            }),
            EditOperation::ResizeTransition { after, .. } => Some(after.min),
            EditOperation::InsertScene { .. }
            | EditOperation::RemoveScene { .. }
            | EditOperation::RenameScene { .. }
            | EditOperation::InsertStamp { .. }
            | EditOperation::RemoveStamp { .. }
            | EditOperation::MoveStamp { .. } => None,
        })
        .collect()
}

fn scene_mut<'a>(
    world: &'a mut GameWorld,
    scene_id: &ProjectSceneId,
) -> Result<&'a mut SceneMap, String> {
    world
        .scene_mut_by_id(scene_id)
        .ok_or_else(|| format!("Scene {} is not loaded", scene_id.label()))
}

fn cell_in_bounds(cell: GridPos) -> bool {
    cell.x >= 0 && cell.y >= 0 && cell.x < MAP_W as i32 && cell.y < MAP_H as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::SceneId;

    #[test]
    fn rectangle_selection_uses_stable_object_ids() {
        let world = GameWorld::starter();
        let scene = world.scene(SceneId::Farmstead).expect("farmstead");
        let object = scene.map.objects.first().expect("starter object");
        let items = selection_items_in_rect(
            scene,
            SceneAuthoringLayer::Objects,
            GridRect::single(GridPos {
                x: object.x,
                y: object.y,
            }),
        );
        assert!(items.contains(&SelectionItem::Object(object.id)));
    }
}
