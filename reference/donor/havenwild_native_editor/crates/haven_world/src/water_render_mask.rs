use haven_core::{TavernMap, TileKind};

pub const WATER_MASK_NORTH: u8 = 1 << 0;
pub const WATER_MASK_EAST: u8 = 1 << 1;
pub const WATER_MASK_SOUTH: u8 = 1 << 2;
pub const WATER_MASK_WEST: u8 = 1 << 3;

pub const WATER_CORNER_NORTH_EAST: u8 = 1 << 0;
pub const WATER_CORNER_SOUTH_EAST: u8 = 1 << 1;
pub const WATER_CORNER_SOUTH_WEST: u8 = 1 << 2;
pub const WATER_CORNER_NORTH_WEST: u8 = 1 << 3;

/// Shader-ready topology for one water cell. Pass 153A deliberately keeps this
/// record independent from GPU APIs so runtime, editor previews, and future
/// chunk baking all consume the same V7-derived ownership contract.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WaterRenderMask {
    pub shoreline_edges: u8,
    pub depth_edges: u8,
    pub depth_corners: u8,
}

impl WaterRenderMask {
    pub const fn has_depth_transition(self) -> bool {
        self.depth_edges != 0 || self.depth_corners != 0
    }
}

/// Deep water exclusively owns the shallow/deep visual blend. This prevents
/// both cells from drawing competing overlays and preserves V7 semantic tiles
/// as inputs rather than rewriting either water material.
pub fn resolve_water_render_mask(map: &TavernMap, x: i32, y: i32) -> WaterRenderMask {
    let tile = map.get(x, y);
    if !is_water(tile) {
        return WaterRenderMask::default();
    }

    let mut mask = WaterRenderMask::default();
    let cardinals = [
        (0, -1, WATER_MASK_NORTH),
        (1, 0, WATER_MASK_EAST),
        (0, 1, WATER_MASK_SOUTH),
        (-1, 0, WATER_MASK_WEST),
    ];
    for (dx, dy, bit) in cardinals {
        let neighbor = map.get(x + dx, y + dy);
        if is_land_or_shore(neighbor) {
            mask.shoreline_edges |= bit;
        }
        if is_deep(tile) && is_shallow(neighbor) {
            mask.depth_edges |= bit;
        }
    }

    if is_deep(tile) {
        let diagonals = [
            (
                1,
                -1,
                WATER_CORNER_NORTH_EAST,
                WATER_MASK_NORTH | WATER_MASK_EAST,
            ),
            (
                1,
                1,
                WATER_CORNER_SOUTH_EAST,
                WATER_MASK_SOUTH | WATER_MASK_EAST,
            ),
            (
                -1,
                1,
                WATER_CORNER_SOUTH_WEST,
                WATER_MASK_SOUTH | WATER_MASK_WEST,
            ),
            (
                -1,
                -1,
                WATER_CORNER_NORTH_WEST,
                WATER_MASK_NORTH | WATER_MASK_WEST,
            ),
        ];
        for (dx, dy, bit, adjacent_edges) in diagonals {
            if is_shallow(map.get(x + dx, y + dy)) && mask.depth_edges & adjacent_edges == 0 {
                mask.depth_corners |= bit;
            }
        }
    }

    mask
}

fn is_water(tile: TileKind) -> bool {
    matches!(
        tile,
        TileKind::Water
            | TileKind::ShallowWater
            | TileKind::DeepWater
            | TileKind::OceanShallow
            | TileKind::OceanDeep
            | TileKind::RiverWater
            | TileKind::RiverMouthBlend
            | TileKind::ShoreFoam
    )
}

fn is_deep(tile: TileKind) -> bool {
    matches!(tile, TileKind::DeepWater | TileKind::OceanDeep)
}

fn is_shallow(tile: TileKind) -> bool {
    matches!(
        tile,
        TileKind::Water
            | TileKind::ShallowWater
            | TileKind::OceanShallow
            | TileKind::RiverWater
            | TileKind::RiverMouthBlend
            | TileKind::ShoreFoam
    )
}

fn is_land_or_shore(tile: TileKind) -> bool {
    !is_water(tile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_cell_exclusively_owns_shallow_depth_edge() {
        let mut map = TavernMap::empty_with(TileKind::DeepWater);
        map.set(4, 3, TileKind::ShallowWater);
        let deep = resolve_water_render_mask(&map, 4, 4);
        let shallow = resolve_water_render_mask(&map, 4, 3);
        assert_eq!(deep.depth_edges, WATER_MASK_NORTH);
        assert_eq!(shallow.depth_edges, 0);
    }

    #[test]
    fn diagonal_depth_contact_is_a_corner_not_a_circle_stamp() {
        let mut map = TavernMap::empty_with(TileKind::DeepWater);
        map.set(5, 3, TileKind::ShallowWater);
        let mask = resolve_water_render_mask(&map, 4, 4);
        assert_eq!(mask.depth_edges, 0);
        assert_eq!(mask.depth_corners, WATER_CORNER_NORTH_EAST);
    }
}
