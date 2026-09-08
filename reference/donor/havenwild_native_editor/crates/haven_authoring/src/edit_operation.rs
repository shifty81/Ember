use crate::{
    transaction_geometry::transition_grid_rect, transaction_stamp::set_stamp_position, GridPos,
    GridRect,
};
use haven_core::{
    AutotileOverride, GameWorld, ObjectId, PlacedObject, PlacedStamp, ProjectSceneId, SceneMap,
    StampInstanceId, TileKind, Transition, TransitionId, ZoneKind,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditOperation {
    SetTile {
        cell: GridPos,
        before: TileKind,
        after: TileKind,
    },
    SetHeight {
        cell: GridPos,
        before: u8,
        after: u8,
    },
    SetStructuralLevel {
        cell: GridPos,
        before: u8,
        after: u8,
    },
    SetAutotileOverride {
        cell: GridPos,
        before: Option<AutotileOverride>,
        after: Option<AutotileOverride>,
    },
    SetZone {
        cell: GridPos,
        before: ZoneKind,
        after: ZoneKind,
    },
    InsertObject {
        object: PlacedObject,
    },
    RemoveObject {
        object: PlacedObject,
    },
    MoveObject {
        id: ObjectId,
        before: GridPos,
        after: GridPos,
    },
    InsertStamp {
        stamp: PlacedStamp,
    },
    RemoveStamp {
        stamp: PlacedStamp,
    },
    MoveStamp {
        id: StampInstanceId,
        before: GridPos,
        after: GridPos,
    },
    UpdateObject {
        id: ObjectId,
        before: PlacedObject,
        after: PlacedObject,
    },
    InsertTransition {
        transition: Transition,
    },
    RemoveTransition {
        transition: Transition,
    },
    ResizeTransition {
        id: TransitionId,
        before: GridRect,
        after: GridRect,
    },
    InsertScene {
        index: usize,
        scene: SceneMap,
    },
    RemoveScene {
        index: usize,
        scene: SceneMap,
    },
    RenameScene {
        before_id: ProjectSceneId,
        after_id: ProjectSceneId,
        before_name: String,
        after_name: String,
    },
}

impl EditOperation {
    pub(crate) fn apply(
        &self,
        world: &mut GameWorld,
        scene_id: &ProjectSceneId,
    ) -> Result<(), String> {
        match self {
            EditOperation::SetTile {
                cell,
                before,
                after,
            } => {
                let scene = scene_mut(world, scene_id)?;
                expect_value("tile", scene.map.get(cell.x, cell.y), *before, *cell)?;
                scene.map.set(cell.x, cell.y, *after);
            }
            EditOperation::SetHeight {
                cell,
                before,
                after,
            } => {
                let scene = scene_mut(world, scene_id)?;
                expect_value(
                    "height",
                    scene.map.get_height(cell.x, cell.y),
                    *before,
                    *cell,
                )?;
                scene.map.set_height(cell.x, cell.y, *after);
            }
            EditOperation::SetStructuralLevel {
                cell,
                before,
                after,
            } => {
                let scene = scene_mut(world, scene_id)?;
                expect_value(
                    "structural level",
                    scene.map.structural_level_storage(cell.x, cell.y),
                    *before,
                    *cell,
                )?;
                scene.map.set_structural_level(
                    cell.x,
                    cell.y,
                    (*after <= haven_core::MAX_STRUCTURAL_LEVEL).then_some(*after),
                );
            }
            EditOperation::SetAutotileOverride {
                cell,
                before,
                after,
            } => {
                set_autotile_override(world, scene_id, *cell, *before, *after)?;
            }
            EditOperation::SetZone {
                cell,
                before,
                after,
            } => {
                let scene = scene_mut(world, scene_id)?;
                expect_value("zone", scene.zone_at(cell.x, cell.y), *before, *cell)?;
                scene.set_zone(cell.x, cell.y, *after);
            }
            EditOperation::InsertObject { object } => {
                let scene = scene_mut(world, scene_id)?;
                if scene.map.object(object.id).is_some() {
                    return Err(format!(
                        "object {} already exists in {}",
                        object.id, scene.name
                    ));
                }
                scene.map.objects.push(*object);
            }
            EditOperation::RemoveObject { object } => {
                let scene = scene_mut(world, scene_id)?;
                let scene_name = scene.name.clone();
                let removed = scene
                    .map
                    .remove_object(object.id)
                    .ok_or_else(|| format!("object {} is missing in {}", object.id, scene_name))?;
                if removed != *object {
                    return Err(format!(
                        "object {} changed before transaction replay",
                        object.id
                    ));
                }
            }
            EditOperation::MoveObject { id, before, after } => {
                set_object_position(world, scene_id, *id, *before, *after)?;
            }
            EditOperation::InsertStamp { stamp } => {
                let scene = scene_mut(world, scene_id)?;
                if scene.map.stamp(stamp.id).is_some() {
                    return Err(format!(
                        "stamp {} already exists in {}",
                        stamp.id, scene.name
                    ));
                }
                scene.map.stamps.push(stamp.clone());
            }
            EditOperation::RemoveStamp { stamp } => {
                let scene = scene_mut(world, scene_id)?;
                let scene_name = scene.name.clone();
                let removed = scene
                    .map
                    .remove_stamp(stamp.id)
                    .ok_or_else(|| format!("stamp {} is missing in {}", stamp.id, scene_name))?;
                if removed != *stamp {
                    return Err(format!(
                        "stamp {} changed before transaction replay",
                        stamp.id
                    ));
                }
            }
            EditOperation::MoveStamp { id, before, after } => {
                set_stamp_position(world, scene_id, *id, *before, *after)?;
            }
            EditOperation::UpdateObject { id, before, after } => {
                set_object_value(world, scene_id, *id, *before, *after)?;
            }
            EditOperation::InsertTransition { transition } => {
                let scene = scene_mut(world, scene_id)?;
                if scene.transition(transition.id).is_some() {
                    return Err(format!(
                        "transition {} already exists in {}",
                        transition.id, scene.name
                    ));
                }
                scene.transitions.push(transition.clone());
            }
            EditOperation::RemoveTransition { transition } => {
                let scene = scene_mut(world, scene_id)?;
                let scene_name = scene.name.clone();
                let removed = scene.remove_transition(transition.id).ok_or_else(|| {
                    format!("transition {} is missing in {}", transition.id, scene_name)
                })?;
                if removed != *transition {
                    return Err(format!(
                        "transition {} changed before transaction replay",
                        transition.id
                    ));
                }
            }
            EditOperation::ResizeTransition { id, before, after } => {
                set_transition_rect(world, scene_id, *id, *before, *after)?;
            }
            EditOperation::InsertScene { index, scene } => {
                world.insert_scene_at(*index, scene.clone())?;
            }
            EditOperation::RemoveScene { scene, .. } => {
                let removed = world.remove_scene(&scene.id)?;
                if removed != *scene {
                    return Err(format!(
                        "scene {} changed before transaction replay",
                        scene.id
                    ));
                }
            }
            EditOperation::RenameScene {
                before_id,
                after_id,
                before_name,
                after_name,
            } => {
                rename_scene_value(world, before_id, after_id, before_name, after_name)?;
            }
        }
        Ok(())
    }

    pub(crate) fn revert(
        &self,
        world: &mut GameWorld,
        scene_id: &ProjectSceneId,
    ) -> Result<(), String> {
        match self {
            EditOperation::SetTile {
                cell,
                before,
                after,
            } => {
                let scene = scene_mut(world, scene_id)?;
                expect_value("tile", scene.map.get(cell.x, cell.y), *after, *cell)?;
                scene.map.set(cell.x, cell.y, *before);
            }
            EditOperation::SetHeight {
                cell,
                before,
                after,
            } => {
                let scene = scene_mut(world, scene_id)?;
                expect_value(
                    "height",
                    scene.map.get_height(cell.x, cell.y),
                    *after,
                    *cell,
                )?;
                scene.map.set_height(cell.x, cell.y, *before);
            }
            EditOperation::SetStructuralLevel {
                cell,
                before,
                after,
            } => {
                let scene = scene_mut(world, scene_id)?;
                expect_value(
                    "structural level",
                    scene.map.structural_level_storage(cell.x, cell.y),
                    *after,
                    *cell,
                )?;
                scene.map.set_structural_level(
                    cell.x,
                    cell.y,
                    (*before <= haven_core::MAX_STRUCTURAL_LEVEL).then_some(*before),
                );
            }
            EditOperation::SetAutotileOverride {
                cell,
                before,
                after,
            } => {
                set_autotile_override(world, scene_id, *cell, *after, *before)?;
            }
            EditOperation::SetZone {
                cell,
                before,
                after,
            } => {
                let scene = scene_mut(world, scene_id)?;
                expect_value("zone", scene.zone_at(cell.x, cell.y), *after, *cell)?;
                scene.set_zone(cell.x, cell.y, *before);
            }
            EditOperation::InsertObject { object } => {
                let scene = scene_mut(world, scene_id)?;
                let scene_name = scene.name.clone();
                let removed = scene
                    .map
                    .remove_object(object.id)
                    .ok_or_else(|| format!("object {} is missing in {}", object.id, scene_name))?;
                if removed != *object {
                    return Err(format!("object {} changed before undo", object.id));
                }
            }
            EditOperation::RemoveObject { object } => {
                let scene = scene_mut(world, scene_id)?;
                if scene.map.object(object.id).is_some() {
                    return Err(format!(
                        "object {} already exists in {}",
                        object.id, scene.name
                    ));
                }
                scene.map.objects.push(*object);
            }
            EditOperation::MoveObject { id, before, after } => {
                set_object_position(world, scene_id, *id, *after, *before)?;
            }
            EditOperation::InsertStamp { stamp } => {
                let scene = scene_mut(world, scene_id)?;
                let scene_name = scene.name.clone();
                let removed = scene
                    .map
                    .remove_stamp(stamp.id)
                    .ok_or_else(|| format!("stamp {} is missing in {}", stamp.id, scene_name))?;
                if removed != *stamp {
                    return Err(format!("stamp {} changed before undo", stamp.id));
                }
            }
            EditOperation::RemoveStamp { stamp } => {
                let scene = scene_mut(world, scene_id)?;
                if scene.map.stamp(stamp.id).is_some() {
                    return Err(format!(
                        "stamp {} already exists in {}",
                        stamp.id, scene.name
                    ));
                }
                scene.map.stamps.push(stamp.clone());
            }
            EditOperation::MoveStamp { id, before, after } => {
                set_stamp_position(world, scene_id, *id, *after, *before)?;
            }
            EditOperation::UpdateObject { id, before, after } => {
                set_object_value(world, scene_id, *id, *after, *before)?;
            }
            EditOperation::InsertTransition { transition } => {
                let scene = scene_mut(world, scene_id)?;
                let scene_name = scene.name.clone();
                let removed = scene.remove_transition(transition.id).ok_or_else(|| {
                    format!("transition {} is missing in {}", transition.id, scene_name)
                })?;
                if removed != *transition {
                    return Err(format!("transition {} changed before undo", transition.id));
                }
            }
            EditOperation::RemoveTransition { transition } => {
                let scene = scene_mut(world, scene_id)?;
                if scene.transition(transition.id).is_some() {
                    return Err(format!(
                        "transition {} already exists in {}",
                        transition.id, scene.name
                    ));
                }
                scene.transitions.push(transition.clone());
            }
            EditOperation::ResizeTransition { id, before, after } => {
                set_transition_rect(world, scene_id, *id, *after, *before)?;
            }
            EditOperation::InsertScene { scene, .. } => {
                let removed = world.remove_scene(&scene.id)?;
                if removed != *scene {
                    return Err(format!("scene {} changed before undo", scene.id));
                }
            }
            EditOperation::RemoveScene { index, scene } => {
                world.insert_scene_at(*index, scene.clone())?;
            }
            EditOperation::RenameScene {
                before_id,
                after_id,
                before_name,
                after_name,
            } => {
                rename_scene_value(world, after_id, before_id, after_name, before_name)?;
            }
        }
        Ok(())
    }
}

fn scene_mut<'a>(
    world: &'a mut GameWorld,
    scene_id: &ProjectSceneId,
) -> Result<&'a mut haven_core::SceneMap, String> {
    world
        .scene_mut_by_id(scene_id)
        .ok_or_else(|| format!("scene {} is not loaded", scene_id.label()))
}

fn expect_value<T: PartialEq + std::fmt::Debug>(
    kind: &str,
    actual: T,
    expected: T,
    cell: GridPos,
) -> Result<(), String> {
    if actual != expected {
        return Err(format!(
            "{kind} at {}, {} changed before transaction replay: expected {:?}, found {:?}",
            cell.x, cell.y, expected, actual
        ));
    }
    Ok(())
}

fn set_autotile_override(
    world: &mut GameWorld,
    scene_id: &ProjectSceneId,
    cell: GridPos,
    expected: Option<AutotileOverride>,
    target: Option<AutotileOverride>,
) -> Result<(), String> {
    let scene = scene_mut(world, scene_id)?;
    let actual = scene.autotile_override_at(cell.x, cell.y);
    if actual != expected {
        return Err(format!(
            "autotile override at {}, {} changed before transaction replay",
            cell.x, cell.y
        ));
    }
    match target {
        Some(value) => scene.set_autotile_override(value),
        None => {
            scene.clear_autotile_override(cell.x, cell.y);
        }
    }
    Ok(())
}

fn set_object_position(
    world: &mut GameWorld,
    scene_id: &ProjectSceneId,
    id: ObjectId,
    expected: GridPos,
    target: GridPos,
) -> Result<(), String> {
    let scene = scene_mut(world, scene_id)?;
    let scene_name = scene.name.clone();
    let object = scene
        .map
        .object_mut(id)
        .ok_or_else(|| format!("object {} is missing in {}", id, scene_name))?;
    let actual = GridPos {
        x: object.x,
        y: object.y,
    };
    if actual != expected {
        return Err(format!(
            "object {} moved before transaction replay: expected {}, {}, found {}, {}",
            id, expected.x, expected.y, actual.x, actual.y
        ));
    }
    object.x = target.x;
    object.y = target.y;
    Ok(())
}

fn set_object_value(
    world: &mut GameWorld,
    scene_id: &ProjectSceneId,
    id: ObjectId,
    expected: PlacedObject,
    target: PlacedObject,
) -> Result<(), String> {
    let scene = scene_mut(world, scene_id)?;
    let scene_name = scene.name.clone();
    let slot = scene
        .map
        .object_mut(id)
        .ok_or_else(|| format!("object {} is missing in {}", id, scene_name))?;
    if *slot != expected {
        return Err(format!("object {} changed before transaction replay", id));
    }
    *slot = target;
    Ok(())
}

fn rename_scene_value(
    world: &mut GameWorld,
    expected_id: &ProjectSceneId,
    target_id: &ProjectSceneId,
    expected_name: &str,
    target_name: &str,
) -> Result<(), String> {
    let current_name = world
        .scene_by_id(expected_id)
        .map(|scene| scene.name.clone())
        .ok_or_else(|| format!("scene {} is not loaded", expected_id))?;
    if current_name != expected_name {
        return Err(format!(
            "scene {} changed before transaction replay",
            expected_id
        ));
    }
    world.rename_scene(expected_id, target_id.clone())?;
    let scene = world
        .scene_mut_by_id(target_id)
        .ok_or_else(|| format!("scene {} is not loaded after rename", target_id))?;
    scene.name = target_name.to_string();
    Ok(())
}

fn set_transition_rect(
    world: &mut GameWorld,
    scene_id: &ProjectSceneId,
    id: TransitionId,
    expected: GridRect,
    target: GridRect,
) -> Result<(), String> {
    let scene = scene_mut(world, scene_id)?;
    let scene_name = scene.name.clone();
    let transition = scene
        .transition_mut(id)
        .ok_or_else(|| format!("transition {} is missing in {}", id, scene_name))?;
    let actual = transition_grid_rect(transition);
    if actual != expected {
        return Err(format!(
            "transition {} changed before transaction replay",
            id
        ));
    }
    transition.x = target.min.x;
    transition.y = target.min.y;
    transition.w = target.max.x - target.min.x + 1;
    transition.h = target.max.y - target.min.y + 1;
    Ok(())
}
