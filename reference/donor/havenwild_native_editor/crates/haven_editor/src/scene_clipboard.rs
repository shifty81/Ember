use std::collections::BTreeMap;

use haven_authoring::{
    EditOperation, EditTransaction, EditorCommand, EditorCommandBus, EditorCommandKind,
    EditorCommandSource, GridPos, GridRect, SceneAuthoringLayer, SelectionItem,
};
use haven_core::{
    GameWorld, PlacedObject, PlacedStamp, ProjectSceneId, SceneMap, TileKind, Transition, ZoneId,
    ZoneKind, MAP_H, MAP_W,
};

use crate::SceneEditOutcome;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardObject {
    pub offset: GridPos,
    pub object: PlacedObject,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardStamp {
    pub offset: GridPos,
    pub stamp: PlacedStamp,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClipboardTransition {
    pub offset: GridPos,
    pub transition: Transition,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneClipboard {
    pub layer: SceneAuthoringLayer,
    pub bounds: GridRect,
    pub tiles: Vec<(GridPos, TileKind)>,
    pub zones: Vec<(GridPos, ZoneKind)>,
    pub objects: Vec<ClipboardObject>,
    pub stamps: Vec<ClipboardStamp>,
    pub transitions: Vec<ClipboardTransition>,
}

impl SceneClipboard {
    pub fn width(&self) -> i32 {
        self.bounds.width()
    }

    pub fn height(&self) -> i32 {
        self.bounds.height()
    }

    pub fn item_count(&self) -> usize {
        self.tiles.len()
            + self.zones.len()
            + self.objects.len()
            + self.stamps.len()
            + self.transitions.len()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneBulkEditOutcome {
    pub edit: SceneEditOutcome,
    pub selected_items: Vec<SelectionItem>,
    pub bounds: Option<GridRect>,
}

pub fn copy_scene_selection(
    scene: &SceneMap,
    layer: SceneAuthoringLayer,
    items: &[SelectionItem],
    bounds: GridRect,
) -> Result<SceneClipboard, String> {
    let mut clipboard = SceneClipboard {
        layer,
        bounds,
        tiles: Vec::new(),
        zones: Vec::new(),
        objects: Vec::new(),
        stamps: Vec::new(),
        transitions: Vec::new(),
    };
    for item in items {
        match (layer, item) {
            (SceneAuthoringLayer::Terrain, SelectionItem::Tile(cell)) => {
                clipboard.tiles.push((
                    GridPos {
                        x: cell.x - bounds.min.x,
                        y: cell.y - bounds.min.y,
                    },
                    scene.map.get(cell.x, cell.y),
                ));
            }
            (SceneAuthoringLayer::Zones, SelectionItem::ZoneCell { cell, .. }) => {
                clipboard.zones.push((
                    GridPos {
                        x: cell.x - bounds.min.x,
                        y: cell.y - bounds.min.y,
                    },
                    scene.zone_at(cell.x, cell.y),
                ));
            }
            (SceneAuthoringLayer::Objects, SelectionItem::Object(id)) => {
                if let Some(object) = scene.map.object(*id) {
                    clipboard.objects.push(ClipboardObject {
                        offset: GridPos {
                            x: object.x - bounds.min.x,
                            y: object.y - bounds.min.y,
                        },
                        object: *object,
                    });
                }
            }
            (SceneAuthoringLayer::Objects, SelectionItem::Stamp(id)) => {
                if let Some(stamp) = scene.map.stamp(*id) {
                    clipboard.stamps.push(ClipboardStamp {
                        offset: GridPos {
                            x: stamp.x - bounds.min.x,
                            y: stamp.y - bounds.min.y,
                        },
                        stamp: stamp.clone(),
                    });
                }
            }
            (SceneAuthoringLayer::Transitions, SelectionItem::Transition(id)) => {
                if let Some(transition) = scene.transition(*id) {
                    clipboard.transitions.push(ClipboardTransition {
                        offset: GridPos {
                            x: transition.x - bounds.min.x,
                            y: transition.y - bounds.min.y,
                        },
                        transition: transition.clone(),
                    });
                }
            }
            _ => {}
        }
    }
    if clipboard.item_count() == 0 {
        return Err("Selection contains no copyable items on the active layer".to_string());
    }
    Ok(clipboard)
}

#[allow(clippy::too_many_arguments)]
pub fn paste_scene_clipboard(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    clipboard: &SceneClipboard,
    anchor: GridPos,
) -> Result<SceneBulkEditOutcome, String> {
    let scene_id = scene_id.into();
    let backup = world.clone();
    let result = paste_scene_clipboard_inner(world, &scene_id, clipboard, anchor);
    let (transaction, selected_items, bounds) = match result {
        Ok(result) => result,
        Err(error) => {
            *world = backup;
            return Err(error);
        }
    };
    let edit = record_bulk_transaction(
        command_bus,
        project_id,
        source,
        &scene_id,
        transaction,
        EditorCommandKind::SceneMutation,
    )?;
    Ok(SceneBulkEditOutcome {
        edit,
        selected_items,
        bounds: Some(bounds),
    })
}

#[allow(clippy::too_many_arguments)]
pub fn delete_scene_selection(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    items: &[SelectionItem],
    fill_tile: TileKind,
) -> Result<SceneEditOutcome, String> {
    let scene_id = scene_id.into();
    let scene = scene_mut(world, &scene_id)?;
    let mut transaction = EditTransaction::new("Delete selection", scene_id.clone());
    for item in items {
        match item {
            SelectionItem::Tile(cell) if cell_in_bounds(*cell) => {
                let before = scene.map.get(cell.x, cell.y);
                if before != fill_tile {
                    scene.map.set(cell.x, cell.y, fill_tile);
                    transaction.push(EditOperation::SetTile {
                        cell: *cell,
                        before,
                        after: fill_tile,
                    });
                }
            }
            SelectionItem::ZoneCell { cell, .. } if cell_in_bounds(*cell) => {
                let before = scene.zone_at(cell.x, cell.y);
                if before != ZoneKind::None {
                    scene.set_zone(cell.x, cell.y, ZoneKind::None);
                    transaction.push(EditOperation::SetZone {
                        cell: *cell,
                        before,
                        after: ZoneKind::None,
                    });
                }
            }
            SelectionItem::Object(id) => {
                if let Some(object) = scene.map.remove_object(*id) {
                    transaction.push(EditOperation::RemoveObject { object });
                }
            }
            SelectionItem::Stamp(id) => {
                if let Some(stamp) = scene.map.remove_stamp(*id) {
                    transaction.push(EditOperation::RemoveStamp { stamp });
                }
            }
            SelectionItem::Transition(id) => {
                if let Some(transition) = scene.remove_transition(*id) {
                    transaction.push(EditOperation::RemoveTransition { transition });
                }
            }
            _ => {}
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
pub fn move_scene_selection(
    world: &mut GameWorld,
    command_bus: &mut EditorCommandBus,
    project_id: &str,
    source: EditorCommandSource,
    scene_id: impl Into<ProjectSceneId>,
    layer: SceneAuthoringLayer,
    items: &[SelectionItem],
    bounds: GridRect,
    delta: GridPos,
    fill_tile: TileKind,
) -> Result<SceneBulkEditOutcome, String> {
    if delta.x == 0 && delta.y == 0 {
        return Err("Selection did not move".to_string());
    }
    let scene_id = scene_id.into();
    let target_bounds = bounds.translated(delta);
    if target_bounds.min.x < 0
        || target_bounds.min.y < 0
        || target_bounds.max.x >= MAP_W as i32
        || target_bounds.max.y >= MAP_H as i32
    {
        return Err("Moved selection would leave the scene bounds".to_string());
    }
    let backup = world.clone();
    let result =
        move_scene_selection_inner(world, &scene_id, layer, items, bounds, delta, fill_tile);
    let (transaction, selected_items) = match result {
        Ok(result) => result,
        Err(error) => {
            *world = backup;
            return Err(error);
        }
    };
    let edit = record_bulk_transaction(
        command_bus,
        project_id,
        source,
        &scene_id,
        transaction,
        EditorCommandKind::SceneMutation,
    )?;
    Ok(SceneBulkEditOutcome {
        edit,
        selected_items,
        bounds: Some(target_bounds),
    })
}

fn paste_scene_clipboard_inner(
    world: &mut GameWorld,
    scene_id: &ProjectSceneId,
    clipboard: &SceneClipboard,
    anchor: GridPos,
) -> Result<(EditTransaction, Vec<SelectionItem>, GridRect), String> {
    let target_bounds = GridRect::from_points(
        anchor,
        GridPos {
            x: anchor.x + clipboard.width() - 1,
            y: anchor.y + clipboard.height() - 1,
        },
    );
    if target_bounds.min.x < 0
        || target_bounds.min.y < 0
        || target_bounds.max.x >= MAP_W as i32
        || target_bounds.max.y >= MAP_H as i32
    {
        return Err("Clipboard paste would leave the scene bounds".to_string());
    }
    let scene = scene_mut(world, scene_id)?;
    let mut transaction = EditTransaction::new("Paste selection", scene_id.clone());
    let mut selected_items = Vec::new();

    for (offset, tile) in &clipboard.tiles {
        let cell = GridPos {
            x: anchor.x + offset.x,
            y: anchor.y + offset.y,
        };
        let before = scene.map.get(cell.x, cell.y);
        if before != *tile {
            scene.map.set(cell.x, cell.y, *tile);
            transaction.push(EditOperation::SetTile {
                cell,
                before,
                after: *tile,
            });
        }
        selected_items.push(SelectionItem::Tile(cell));
    }
    for (offset, zone) in &clipboard.zones {
        let cell = GridPos {
            x: anchor.x + offset.x,
            y: anchor.y + offset.y,
        };
        let before = scene.zone_at(cell.x, cell.y);
        if before != *zone {
            scene.set_zone(cell.x, cell.y, *zone);
            transaction.push(EditOperation::SetZone {
                cell,
                before,
                after: *zone,
            });
        }
        selected_items.push(SelectionItem::ZoneCell {
            id: ZoneId::for_cell(cell.x, cell.y, MAP_W),
            cell,
        });
    }
    for entry in &clipboard.objects {
        let mut object = entry.object;
        object.id = scene.map.next_object_id();
        object.x = anchor.x + entry.offset.x;
        object.y = anchor.y + entry.offset.y;
        let issues = scene.map.placement_issues_for_object(object);
        if !issues.is_empty() {
            return Err(format!(
                "Cannot paste {} at {}, {}: {}",
                object.kind.label(),
                object.x,
                object.y,
                issues
                    .iter()
                    .map(|issue| issue.label())
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        scene.map.objects.push(object);
        transaction.push(EditOperation::InsertObject { object });
        selected_items.push(SelectionItem::Object(object.id));
    }
    for entry in &clipboard.stamps {
        let mut stamp = entry.stamp.clone();
        stamp.id = scene.map.next_stamp_id();
        stamp.x = anchor.x + entry.offset.x;
        stamp.y = anchor.y + entry.offset.y;
        let issues = scene.map.placement_issues_for_stamp(&stamp);
        if !issues.is_empty() {
            return Err(format!(
                "Cannot paste {} at {}, {}: {}",
                stamp.stamp_key,
                stamp.x,
                stamp.y,
                issues
                    .iter()
                    .map(|issue| issue.label())
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        scene.map.stamps.push(stamp.clone());
        transaction.push(EditOperation::InsertStamp {
            stamp: stamp.clone(),
        });
        selected_items.push(SelectionItem::Stamp(stamp.id));
    }
    for entry in &clipboard.transitions {
        let mut transition = entry.transition.clone();
        transition.id = scene.next_transition_id();
        transition.x = anchor.x + entry.offset.x;
        transition.y = anchor.y + entry.offset.y;
        if transition.x + transition.w > MAP_W as i32 || transition.y + transition.h > MAP_H as i32
        {
            return Err(format!(
                "Cannot paste transition {} outside the scene",
                transition.label
            ));
        }
        scene.transitions.push(transition.clone());
        transaction.push(EditOperation::InsertTransition {
            transition: transition.clone(),
        });
        selected_items.push(SelectionItem::Transition(transition.id));
    }
    if transaction.is_empty() {
        return Err("Clipboard paste would not change the scene".to_string());
    }
    Ok((transaction, selected_items, target_bounds))
}

#[allow(clippy::too_many_arguments)]
fn move_scene_selection_inner(
    world: &mut GameWorld,
    scene_id: &ProjectSceneId,
    layer: SceneAuthoringLayer,
    items: &[SelectionItem],
    _bounds: GridRect,
    delta: GridPos,
    fill_tile: TileKind,
) -> Result<(EditTransaction, Vec<SelectionItem>), String> {
    let scene = scene_mut(world, scene_id)?;
    let mut transaction = EditTransaction::new("Move selection", scene_id.clone());
    let mut selected_items = Vec::new();
    match layer {
        SceneAuthoringLayer::Terrain => {
            let mut final_values = BTreeMap::<GridPos, TileKind>::new();
            let mut source_values = Vec::new();
            for item in items {
                let SelectionItem::Tile(cell) = item else {
                    continue;
                };
                source_values.push((*cell, scene.map.get(cell.x, cell.y)));
                final_values.insert(*cell, fill_tile);
            }
            for (cell, value) in source_values {
                let target = GridPos {
                    x: cell.x + delta.x,
                    y: cell.y + delta.y,
                };
                final_values.insert(target, value);
                selected_items.push(SelectionItem::Tile(target));
            }
            for (cell, after) in final_values {
                let before = scene.map.get(cell.x, cell.y);
                if before != after {
                    scene.map.set(cell.x, cell.y, after);
                    transaction.push(EditOperation::SetTile {
                        cell,
                        before,
                        after,
                    });
                }
            }
        }
        SceneAuthoringLayer::Zones => {
            let mut final_values = BTreeMap::<GridPos, ZoneKind>::new();
            let mut source_values = Vec::new();
            for item in items {
                let SelectionItem::ZoneCell { cell, .. } = item else {
                    continue;
                };
                source_values.push((*cell, scene.zone_at(cell.x, cell.y)));
                final_values.insert(*cell, ZoneKind::None);
            }
            for (cell, value) in source_values {
                let target = GridPos {
                    x: cell.x + delta.x,
                    y: cell.y + delta.y,
                };
                final_values.insert(target, value);
                selected_items.push(SelectionItem::ZoneCell {
                    id: ZoneId::for_cell(target.x, target.y, MAP_W),
                    cell: target,
                });
            }
            for (cell, after) in final_values {
                let before = scene.zone_at(cell.x, cell.y);
                if before != after {
                    scene.set_zone(cell.x, cell.y, after);
                    transaction.push(EditOperation::SetZone {
                        cell,
                        before,
                        after,
                    });
                }
            }
        }
        SceneAuthoringLayer::Objects => {
            let ids = items
                .iter()
                .filter_map(|item| match item {
                    SelectionItem::Object(id) => Some(*id),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let mut objects = Vec::new();
            for id in &ids {
                let object = scene
                    .map
                    .remove_object(*id)
                    .ok_or_else(|| format!("Selected object {} is missing", id))?;
                objects.push(object);
            }
            for mut object in objects {
                let before = GridPos {
                    x: object.x,
                    y: object.y,
                };
                object.x += delta.x;
                object.y += delta.y;
                let issues = scene.map.placement_issues_for_object(object);
                if !issues.is_empty() {
                    return Err(format!(
                        "Cannot move {} to {}, {}: {}",
                        object.kind.label(),
                        object.x,
                        object.y,
                        issues
                            .iter()
                            .map(|issue| issue.label())
                            .collect::<Vec<_>>()
                            .join("; ")
                    ));
                }
                scene.map.objects.push(object);
                transaction.push(EditOperation::MoveObject {
                    id: object.id,
                    before,
                    after: GridPos {
                        x: object.x,
                        y: object.y,
                    },
                });
                selected_items.push(SelectionItem::Object(object.id));
            }

            let stamp_ids = items
                .iter()
                .filter_map(|item| match item {
                    SelectionItem::Stamp(id) => Some(*id),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let mut stamps = Vec::new();
            for id in &stamp_ids {
                let stamp = scene
                    .map
                    .remove_stamp(*id)
                    .ok_or_else(|| format!("Selected stamp {} is missing", id))?;
                stamps.push(stamp);
            }
            for mut stamp in stamps {
                let before = GridPos {
                    x: stamp.x,
                    y: stamp.y,
                };
                stamp.x += delta.x;
                stamp.y += delta.y;
                let issues = scene.map.placement_issues_for_stamp(&stamp);
                if !issues.is_empty() {
                    return Err(format!(
                        "Cannot move {} to {}, {}: {}",
                        stamp.stamp_key,
                        stamp.x,
                        stamp.y,
                        issues
                            .iter()
                            .map(|issue| issue.label())
                            .collect::<Vec<_>>()
                            .join("; ")
                    ));
                }
                let id = stamp.id;
                let after = GridPos {
                    x: stamp.x,
                    y: stamp.y,
                };
                scene.map.stamps.push(stamp);
                transaction.push(EditOperation::MoveStamp { id, before, after });
                selected_items.push(SelectionItem::Stamp(id));
            }
        }
        SceneAuthoringLayer::Transitions => {
            let ids = items
                .iter()
                .filter_map(|item| match item {
                    SelectionItem::Transition(id) => Some(*id),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for id in ids {
                let transition = scene
                    .transition_mut(id)
                    .ok_or_else(|| format!("Selected transition {} is missing", id))?;
                let before = GridRect::from_points(
                    GridPos {
                        x: transition.x,
                        y: transition.y,
                    },
                    GridPos {
                        x: transition.x + transition.w - 1,
                        y: transition.y + transition.h - 1,
                    },
                );
                transition.x += delta.x;
                transition.y += delta.y;
                let after = before.translated(delta);
                transaction.push(EditOperation::ResizeTransition { id, before, after });
                selected_items.push(SelectionItem::Transition(id));
            }
        }
    }
    if transaction.is_empty() {
        return Err("Selection contains no movable content on the active layer".to_string());
    }
    Ok((transaction, selected_items))
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
            EditOperation::InsertStamp { stamp } | EditOperation::RemoveStamp { stamp } => {
                Some(GridPos {
                    x: stamp.x,
                    y: stamp.y,
                })
            }
            EditOperation::MoveStamp { after, .. } => Some(*after),
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
            | EditOperation::RenameScene { .. } => None,
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
    use crate::selection_items_in_rect;
    use haven_core::SceneId;

    #[test]
    fn terrain_clipboard_uses_relative_coordinates() {
        let world = GameWorld::starter();
        let scene = world.scene(SceneId::Farmstead).expect("farmstead");
        let bounds = GridRect::from_points(GridPos { x: 2, y: 3 }, GridPos { x: 3, y: 3 });
        let items = selection_items_in_rect(scene, SceneAuthoringLayer::Terrain, bounds);
        let clipboard = copy_scene_selection(scene, SceneAuthoringLayer::Terrain, &items, bounds)
            .expect("clipboard");
        assert_eq!(clipboard.tiles[0].0, GridPos { x: 0, y: 0 });
        assert_eq!(clipboard.width(), 2);
    }
}
