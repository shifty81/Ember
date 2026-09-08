//! Shared source-authored terrain/provider primitives.
//!
//! Havenwild resolves semantic topology first and then asks an authored source
//! provider for an existing 32x32 cell or natural-scale multi-cell stamp. This
//! module intentionally has no image synthesis, crop, mirror, stretch, or
//! rotation API: if a provider cannot name authored source cells for a role,
//! the role remains unresolved until the source catalog/world topology is fixed.

use crate::lpc_mapped_terrain::{
    lpc_mapped_terrain_runtime_entry_for_map, lpc_mapped_terrain_transition_entry_for_map,
    LpcMappedTerrainEntry,
};
use haven_core::TavernMap;

pub const AUTHORED_TERRAIN_CELL_PX: u16 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AuthoredSourceCell {
    pub column: u8,
    pub row: u8,
}

impl AuthoredSourceCell {
    pub const fn new(column: u8, row: u8) -> Self {
        Self { column, row }
    }

    pub const fn x_px(self) -> u16 {
        self.column as u16 * AUTHORED_TERRAIN_CELL_PX
    }

    pub const fn y_px(self) -> u16 {
        self.row as u16 * AUTHORED_TERRAIN_CELL_PX
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AuthoredSourceStamp {
    pub column: u8,
    pub row: u8,
    pub width_cells: u8,
    pub height_cells: u8,
}

impl AuthoredSourceStamp {
    pub const fn new(column: u8, row: u8, width_cells: u8, height_cells: u8) -> Self {
        Self {
            column,
            row,
            width_cells,
            height_cells,
        }
    }

    pub const fn width_px(self) -> u16 {
        self.width_cells as u16 * AUTHORED_TERRAIN_CELL_PX
    }

    pub const fn height_px(self) -> u16 {
        self.height_cells as u16 * AUTHORED_TERRAIN_CELL_PX
    }
}

/// Exact V7 source selections for one rendered terrain location. Base and
/// transition remain separate because V7 mixed corner tuples are drawn on the
/// intersection-aligned transition lane while the gameplay cell keeps its pure
/// semantic owner fill.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuthoredSurfaceResolution {
    pub base: Option<LpcMappedTerrainEntry>,
    pub transition: Option<LpcMappedTerrainEntry>,
}

pub fn resolve_authored_v7_surface_for_map(
    map: &TavernMap,
    x: i32,
    y: i32,
) -> AuthoredSurfaceResolution {
    AuthoredSurfaceResolution {
        base: lpc_mapped_terrain_runtime_entry_for_map(map, x, y),
        transition: lpc_mapped_terrain_transition_entry_for_map(map, x, y),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_cells_are_always_whole_32px_units() {
        let cell = AuthoredSourceCell::new(3, 7);
        assert_eq!(cell.x_px(), 96);
        assert_eq!(cell.y_px(), 224);
        let stamp = AuthoredSourceStamp::new(1, 6, 3, 3);
        assert_eq!(stamp.width_px(), 96);
        assert_eq!(stamp.height_px(), 96);
    }
}
