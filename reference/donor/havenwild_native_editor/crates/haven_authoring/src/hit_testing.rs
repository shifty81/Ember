use haven_core::{ProjectSceneId, SceneMap, ZoneId, ZoneKind, MAP_H, MAP_W};
use serde::{Deserialize, Serialize};

use crate::{GridPos, GridRect, SelectionItem};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneAuthoringLayer {
    Terrain,
    Objects,
    Zones,
    Transitions,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanvasHit {
    pub scene_id: ProjectSceneId,
    pub cell: GridPos,
    pub item: SelectionItem,
    pub bounds: GridRect,
}

impl CanvasHit {
    pub fn selection_item(&self) -> SelectionItem {
        self.item.clone()
    }
}

pub fn hit_test_scene_cell(
    scene: &SceneMap,
    cell: GridPos,
    layer: SceneAuthoringLayer,
) -> Option<CanvasHit> {
    if cell.x < 0 || cell.y < 0 || cell.x >= MAP_W as i32 || cell.y >= MAP_H as i32 {
        return None;
    }

    let (item, bounds) = match layer {
        SceneAuthoringLayer::Terrain => (SelectionItem::Tile(cell), GridRect::single(cell)),
        SceneAuthoringLayer::Objects => {
            if let Some(stamp_id) = scene.map.stamp_id_at(cell.x, cell.y) {
                let stamp = scene.map.stamp(stamp_id)?;
                let (x, y, w, h) = stamp.visual_rect();
                (
                    SelectionItem::Stamp(stamp_id),
                    rect_from_origin_size(x, y, w, h),
                )
            } else {
                let object_id = scene.map.object_id_at(cell.x, cell.y)?;
                let object = scene.map.object(object_id)?;
                let (x, y, w, h) = object.visual_rect();
                (
                    SelectionItem::Object(object_id),
                    rect_from_origin_size(x, y, w, h),
                )
            }
        }
        SceneAuthoringLayer::Zones => {
            if scene.zone_at(cell.x, cell.y) == ZoneKind::None {
                return None;
            }
            (
                SelectionItem::ZoneCell {
                    id: ZoneId::for_cell(cell.x, cell.y, MAP_W),
                    cell,
                },
                GridRect::single(cell),
            )
        }
        SceneAuthoringLayer::Transitions => {
            let transition_id = scene.transition_id_at(cell.x, cell.y)?;
            let transition = scene.transition(transition_id)?;
            (
                SelectionItem::Transition(transition_id),
                rect_from_origin_size(transition.x, transition.y, transition.w, transition.h),
            )
        }
    };

    Some(CanvasHit {
        scene_id: scene.id.clone(),
        cell,
        item,
        bounds,
    })
}

fn rect_from_origin_size(x: i32, y: i32, w: i32, h: i32) -> GridRect {
    GridRect::from_points(
        GridPos { x, y },
        GridPos {
            x: x + w.max(1) - 1,
            y: y + h.max(1) - 1,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::{GameWorld, SceneId};

    #[test]
    fn object_hits_return_stable_object_identity() {
        let world = GameWorld::starter();
        let scene = world.scene(SceneId::Farmstead).expect("farmstead");
        let object = scene.map.objects.first().expect("starter object");
        let hit = hit_test_scene_cell(
            scene,
            GridPos {
                x: object.x,
                y: object.y,
            },
            SceneAuthoringLayer::Objects,
        )
        .expect("object hit");
        assert_eq!(hit.item, SelectionItem::Object(object.id));
    }
}
