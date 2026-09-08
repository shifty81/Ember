use haven_authoring::{GridPos, GridRect};
use haven_core::{GameWorld, ObjectFootprint, ProjectSceneId, MAP_H, MAP_W};
use haven_world::scene_rectangles::{SceneRectangleAssignmentsFile, SceneRectangleManifest};

use super::{is_surface_rectangle, rectangle_origin, WorldSurfaceCellAddress};

pub fn resolve_world_surface_cell(
    manifest: &SceneRectangleManifest,
    assignments: &SceneRectangleAssignmentsFile,
    world: &GameWorld,
    landmass_id: i32,
    global: GridPos,
) -> Result<WorldSurfaceCellAddress, String> {
    for (rectangle_index, rectangle) in manifest.scene_rectangles.iter().enumerate() {
        if rectangle.landmass_id != landmass_id || !is_surface_rectangle(rectangle) {
            continue;
        }
        let origin = rectangle_origin(rectangle);
        let width = rectangle.tile_size[0].min(MAP_W as i32).max(1);
        let height = rectangle.tile_size[1].min(MAP_H as i32).max(1);
        if global.x < origin.x
            || global.y < origin.y
            || global.x >= origin.x + width
            || global.y >= origin.y + height
        {
            continue;
        }
        let assignment = assignments
            .assignment_for_rectangle(&rectangle.scene_id)
            .ok_or_else(|| format!("partition {} has no scene assignment", rectangle.scene_id))?;
        let scene_id = ProjectSceneId::new(assignment.scene_code.as_str());
        if world.scene_by_id(&scene_id).is_none() {
            return Err(format!(
                "partition {} references unloaded scene {}",
                rectangle.scene_id,
                scene_id.label()
            ));
        }
        return Ok(WorldSurfaceCellAddress {
            rectangle_index,
            rectangle_id: rectangle.scene_id.clone(),
            scene_id,
            global,
            local: GridPos {
                x: global.x - origin.x,
                y: global.y - origin.y,
            },
            partition_origin: origin,
        });
    }
    Err(format!(
        "global tile {}, {} is outside assigned landmass {}",
        global.x, global.y, landmass_id
    ))
}

pub fn world_surface_bounds(
    manifest: &SceneRectangleManifest,
    landmass_id: i32,
) -> Option<GridRect> {
    let mut min: Option<GridPos> = None;
    let mut max: Option<GridPos> = None;
    for rectangle in manifest
        .scene_rectangles
        .iter()
        .filter(|rectangle| rectangle.landmass_id == landmass_id && is_surface_rectangle(rectangle))
    {
        let origin = rectangle_origin(rectangle);
        let far = GridPos {
            x: origin.x + rectangle.tile_size[0].min(MAP_W as i32).max(1) - 1,
            y: origin.y + rectangle.tile_size[1].min(MAP_H as i32).max(1) - 1,
        };
        min = Some(match min {
            Some(current) => GridPos {
                x: current.x.min(origin.x),
                y: current.y.min(origin.y),
            },
            None => origin,
        });
        max = Some(match max {
            Some(current) => GridPos {
                x: current.x.max(far.x),
                y: current.y.max(far.y),
            },
            None => far,
        });
    }
    Some(GridRect::from_points(min?, max?))
}

pub fn validate_world_surface_footprint(
    manifest: &SceneRectangleManifest,
    assignments: &SceneRectangleAssignmentsFile,
    world: &GameWorld,
    landmass_id: i32,
    anchor: GridPos,
    footprint: ObjectFootprint,
) -> Result<WorldSurfaceCellAddress, String> {
    let anchor_address =
        resolve_world_surface_cell(manifest, assignments, world, landmass_id, anchor)?;
    let min_x = 0
        .min(footprint.visual_offset_x)
        .min(footprint.collision_offset_x)
        .min(footprint.interaction_offset_x);
    let min_y = 0
        .min(footprint.visual_offset_y)
        .min(footprint.collision_offset_y)
        .min(footprint.interaction_offset_y);
    let max_x = 1
        .max(footprint.visual_offset_x + footprint.visual_w.max(1))
        .max(footprint.collision_offset_x + footprint.collision_w.max(1))
        .max(footprint.interaction_offset_x + footprint.interaction_w.max(1));
    let max_y = 1
        .max(footprint.visual_offset_y + footprint.visual_h.max(1))
        .max(footprint.collision_offset_y + footprint.collision_h.max(1))
        .max(footprint.interaction_offset_y + footprint.interaction_h.max(1));

    for offset_y in min_y..max_y {
        for offset_x in min_x..max_x {
            let global = GridPos {
                x: anchor.x + offset_x,
                y: anchor.y + offset_y,
            };
            let address =
                resolve_world_surface_cell(manifest, assignments, world, landmass_id, global)
                    .map_err(|error| {
                        format!(
                            "complete footprint at {}, {} reaches unavailable global cell {}, {}: {}",
                            anchor.x, anchor.y, global.x, global.y, error
                        )
                    })?;
            if address.scene_id != anchor_address.scene_id {
                return Err(format!(
                    "complete footprint at {}, {} crosses storage partitions {} and {}; move the anchor inward so the full authored asset remains atomic",
                    anchor.x,
                    anchor.y,
                    anchor_address.rectangle_id,
                    address.rectangle_id
                ));
            }
        }
    }

    Ok(anchor_address)
}
