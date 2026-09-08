use haven_core::{ObjectId, ProjectSceneId, RegionNodeId, StampInstanceId, TransitionId, ZoneId};
use serde::{Deserialize, Serialize};

use crate::GridPos;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridRect {
    pub min: GridPos,
    pub max: GridPos,
}

impl GridRect {
    pub fn from_points(a: GridPos, b: GridPos) -> Self {
        Self {
            min: GridPos {
                x: a.x.min(b.x),
                y: a.y.min(b.y),
            },
            max: GridPos {
                x: a.x.max(b.x),
                y: a.y.max(b.y),
            },
        }
    }

    pub fn single(cell: GridPos) -> Self {
        Self {
            min: cell,
            max: cell,
        }
    }

    pub fn contains(self, cell: GridPos) -> bool {
        cell.x >= self.min.x && cell.y >= self.min.y && cell.x <= self.max.x && cell.y <= self.max.y
    }

    pub fn width(self) -> i32 {
        self.max.x - self.min.x + 1
    }

    pub fn height(self) -> i32 {
        self.max.y - self.min.y + 1
    }

    pub fn intersects(self, other: GridRect) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }

    pub fn translated(self, delta: GridPos) -> Self {
        Self {
            min: GridPos {
                x: self.min.x + delta.x,
                y: self.min.y + delta.y,
            },
            max: GridPos {
                x: self.max.x + delta.x,
                y: self.max.y + delta.y,
            },
        }
    }

    pub fn clamped(self, width: i32, height: i32) -> Self {
        let max_x = width.saturating_sub(1);
        let max_y = height.saturating_sub(1);
        Self {
            min: GridPos {
                x: self.min.x.clamp(0, max_x),
                y: self.min.y.clamp(0, max_y),
            },
            max: GridPos {
                x: self.max.x.clamp(0, max_x),
                y: self.max.y.clamp(0, max_y),
            },
        }
    }

    pub fn cells(self) -> Vec<GridPos> {
        let mut cells = Vec::with_capacity((self.width() * self.height()).max(0) as usize);
        for y in self.min.y..=self.max.y {
            for x in self.min.x..=self.max.x {
                cells.push(GridPos { x, y });
            }
        }
        cells
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionItem {
    Tile(GridPos),
    Object(ObjectId),
    Stamp(StampInstanceId),
    ZoneCell { id: ZoneId, cell: GridPos },
    Transition(TransitionId),
    Scene(ProjectSceneId),
    RegionNode(RegionNodeId),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorSelection {
    pub scene_id: Option<ProjectSceneId>,
    pub items: Vec<SelectionItem>,
    pub primary: Option<SelectionItem>,
    pub bounds: Option<GridRect>,
}

impl EditorSelection {
    pub fn clear(&mut self) {
        self.scene_id = None;
        self.items.clear();
        self.primary = None;
        self.bounds = None;
    }

    pub fn clear_items(&mut self) {
        self.items.clear();
        self.primary = None;
        self.bounds = None;
    }

    pub fn replace(&mut self, scene_id: ProjectSceneId, item: SelectionItem) {
        self.scene_id = Some(scene_id);
        self.items.clear();
        self.items.push(item.clone());
        self.primary = Some(item);
        self.bounds = None;
    }

    pub fn replace_with_bounds(
        &mut self,
        scene_id: ProjectSceneId,
        item: SelectionItem,
        bounds: GridRect,
    ) {
        self.replace(scene_id, item);
        self.bounds = Some(bounds);
    }

    pub fn replace_many(
        &mut self,
        scene_id: ProjectSceneId,
        items: Vec<SelectionItem>,
        bounds: Option<GridRect>,
    ) {
        self.scene_id = Some(scene_id);
        self.items = items;
        self.primary = self.items.first().cloned();
        self.bounds = bounds;
    }

    pub fn add_many(
        &mut self,
        scene_id: ProjectSceneId,
        items: impl IntoIterator<Item = SelectionItem>,
        bounds: Option<GridRect>,
    ) {
        if self.scene_id.as_ref() != Some(&scene_id) {
            self.clear();
            self.scene_id = Some(scene_id);
        }
        for item in items {
            if !self.items.contains(&item) {
                self.items.push(item.clone());
                self.primary = Some(item);
            }
        }
        self.bounds = match (self.bounds, bounds) {
            (Some(existing), Some(incoming)) => Some(GridRect::from_points(
                GridPos {
                    x: existing.min.x.min(incoming.min.x),
                    y: existing.min.y.min(incoming.min.y),
                },
                GridPos {
                    x: existing.max.x.max(incoming.max.x),
                    y: existing.max.y.max(incoming.max.y),
                },
            )),
            (Some(existing), None) => Some(existing),
            (None, incoming) => incoming,
        };
    }

    pub fn toggle(&mut self, scene_id: ProjectSceneId, item: SelectionItem, bounds: GridRect) {
        if self.scene_id.as_ref() != Some(&scene_id) {
            self.replace_with_bounds(scene_id, item, bounds);
            return;
        }
        if let Some(index) = self.items.iter().position(|candidate| candidate == &item) {
            self.items.remove(index);
            self.primary = self.items.last().cloned();
            if self.items.is_empty() {
                self.bounds = None;
            }
        } else {
            self.items.push(item.clone());
            self.primary = Some(item);
            self.bounds = Some(match self.bounds {
                Some(existing) => GridRect::from_points(
                    GridPos {
                        x: existing.min.x.min(bounds.min.x),
                        y: existing.min.y.min(bounds.min.y),
                    },
                    GridPos {
                        x: existing.max.x.max(bounds.max.x),
                        y: existing.max.y.max(bounds.max.y),
                    },
                ),
                None => bounds,
            });
        }
    }

    pub fn contains(&self, item: &SelectionItem) -> bool {
        self.items.contains(item)
    }

    pub fn set_region_node(&mut self, id: RegionNodeId) {
        self.scene_id = None;
        self.items.clear();
        let item = SelectionItem::RegionNode(id);
        self.items.push(item.clone());
        self.primary = Some(item);
        self.bounds = None;
    }

    pub fn primary_object_id(&self) -> Option<ObjectId> {
        match self.primary.as_ref() {
            Some(SelectionItem::Object(id)) => Some(*id),
            _ => None,
        }
    }

    pub fn primary_stamp_id(&self) -> Option<StampInstanceId> {
        match self.primary.as_ref() {
            Some(SelectionItem::Stamp(id)) => Some(*id),
            _ => None,
        }
    }

    pub fn primary_transition_id(&self) -> Option<TransitionId> {
        match self.primary.as_ref() {
            Some(SelectionItem::Transition(id)) => Some(*id),
            _ => None,
        }
    }

    pub fn primary_region_node_id(&self) -> Option<&RegionNodeId> {
        match self.primary.as_ref() {
            Some(SelectionItem::RegionNode(id)) => Some(id),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
