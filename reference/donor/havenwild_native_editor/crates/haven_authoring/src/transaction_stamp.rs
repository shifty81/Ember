use haven_core::{GameWorld, ProjectSceneId, StampInstanceId};

use crate::GridPos;

pub(crate) fn set_stamp_position(
    world: &mut GameWorld,
    scene_id: &ProjectSceneId,
    id: StampInstanceId,
    expected: GridPos,
    replacement: GridPos,
) -> Result<(), String> {
    let scene = world
        .scene_mut_by_id(scene_id)
        .ok_or_else(|| format!("scene {} is not loaded", scene_id.label()))?;
    let stamp = scene
        .map
        .stamp(id)
        .ok_or_else(|| format!("stamp {} is missing in {}", id, scene.name))?;
    if stamp.x != expected.x || stamp.y != expected.y {
        return Err(format!(
            "stamp {} moved before transaction replay: expected {},{} found {},{}",
            id, expected.x, expected.y, stamp.x, stamp.y
        ));
    }
    scene
        .map
        .move_stamp(id, replacement.x, replacement.y)
        .map_err(|issues| {
            issues
                .into_iter()
                .map(|issue| issue.label())
                .collect::<Vec<_>>()
                .join("; ")
        })
}
