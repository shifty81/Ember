use super::*;

impl SceneMap {
    /// Returns whether a cell belongs to the visually authored portion of this
    /// scene. Exterior scenes own their complete storage grid. Interior and cave
    /// scenes treat unused backing material as void while retaining the one-cell
    /// structural boundary that touches authored floor/terrain.
    pub fn is_renderable_cell(&self, x: i32, y: i32) -> bool {
        if TavernMap::idx(x, y).is_none() {
            return false;
        }
        let void_tile = match self.kind {
            SceneKind::Exterior => return true,
            SceneKind::Interior => TileKind::Wall,
            SceneKind::Cave => TileKind::CaveWall,
        };
        if self.map.get(x, y) != void_tile {
            return true;
        }

        for neighbor_y in y - 1..=y + 1 {
            for neighbor_x in x - 1..=x + 1 {
                if neighbor_x == x && neighbor_y == y {
                    continue;
                }
                if TavernMap::idx(neighbor_x, neighbor_y).is_some()
                    && self.map.get(neighbor_x, neighbor_y) != void_tile
                {
                    return true;
                }
            }
        }
        false
    }

    /// Inclusive presentation bounds for editor-grid clipping and diagnostics.
    pub fn renderable_bounds(&self) -> Option<(i32, i32, i32, i32)> {
        if self.kind == SceneKind::Exterior {
            return Some((0, 0, MAP_W as i32 - 1, MAP_H as i32 - 1));
        }
        let mut min_x = MAP_W as i32;
        let mut min_y = MAP_H as i32;
        let mut max_x = -1;
        let mut max_y = -1;
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                if !self.is_renderable_cell(x, y) {
                    continue;
                }
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
        (max_x >= min_x && max_y >= min_y).then_some((min_x, min_y, max_x, max_y))
    }
}
