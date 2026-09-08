//! Authored directional ramp assemblies from the LPC grass-top cliff family.
//!
//! The ramp art is already complete in the source sheets. Havenwild places the
//! 3x4 stamps at natural scale and never mirrors, rotates, crops, or stretches
//! them. The left/right variants are distinct authored assemblies.

use crate::authored_terrain_provider::AuthoredSourceStamp;

pub const LPC_CLIFF_RAMP_GRASS_SOURCE_PATH: &str =
    "content/assets/oga_lpc/source/terrain/cliffs_grass_top/LPC_cliffs_grass.png";
pub const LPC_CLIFF_RAMP_DIRT_SOURCE_PATH: &str =
    "content/assets/oga_lpc/source/terrain/cliffs_grass_top/LPC_cliffs_ddirt.png";
pub const LPC_CLIFF_RAMP_SAND_SOURCE_PATH: &str =
    "content/assets/oga_lpc/source/terrain/cliffs_grass_top/LPC_cliffs_sand.png";
pub const LPC_CLIFF_RAMP_SNOW_SOURCE_PATH: &str =
    "content/assets/oga_lpc/source/terrain/cliffs_grass_top/LPC_cliffs_snow.png";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LpcDirectionalCliffRampRole {
    /// Green corridor climbs from lower-left toward upper-right.
    RiseRight,
    /// Green corridor climbs from lower-right toward upper-left.
    RiseLeft,
}

impl LpcDirectionalCliffRampRole {
    pub const fn source_stamp(self) -> AuthoredSourceStamp {
        match self {
            // Catalogued in the project source as the complete authored left
            // and right 3x4 directional ramp assemblies.
            Self::RiseRight => AuthoredSourceStamp::new(3, 5, 3, 4),
            Self::RiseLeft => AuthoredSourceStamp::new(6, 5, 3, 4),
        }
    }

    /// The structural south-edge connector is aligned to the center source
    /// column between source rows 1 and 2. This keeps the exact existing
    /// cardinal traversal edge inside the visible diagonal green corridor.
    pub const fn host_anchor_offset(self) -> (i8, i8) {
        let _ = self;
        (-1, -1)
    }
}

/// Exact connected contour modules from the same LPC grass-top cliff family.
/// These are not generated substitutes: each role returns a complete authored
/// source span placed at natural 32px cell scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LpcCliffContourStampRole {
    /// Repeatable interior row for a one-cell-wide north/south ridge. The
    /// center world cell remains walkable plateau; the authored left/right
    /// cliff flanks occupy the adjacent visual cells.
    VerticalRidgeMiddle,
}

impl LpcCliffContourStampRole {
    pub const fn source_stamp(self) -> AuthoredSourceStamp {
        match self {
            Self::VerticalRidgeMiddle => AuthoredSourceStamp::new(8, 2, 3, 1),
        }
    }

    pub const fn host_anchor_offset(self) -> (i8, i8) {
        let _ = self;
        (-1, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directional_ramps_are_complete_natural_scale_3x4_stamps() {
        assert_eq!(LpcDirectionalCliffRampRole::RiseRight.source_stamp(), AuthoredSourceStamp::new(3, 5, 3, 4));
        assert_eq!(LpcDirectionalCliffRampRole::RiseLeft.source_stamp(), AuthoredSourceStamp::new(6, 5, 3, 4));
        assert_eq!(LpcDirectionalCliffRampRole::RiseRight.host_anchor_offset(), (-1, -1));
        assert_eq!(LpcDirectionalCliffRampRole::RiseLeft.host_anchor_offset(), (-1, -1));
    }

    #[test]
    fn vertical_ridge_middle_is_an_exact_three_cell_authored_row() {
        assert_eq!(
            LpcCliffContourStampRole::VerticalRidgeMiddle.source_stamp(),
            AuthoredSourceStamp::new(8, 2, 3, 1)
        );
        assert_eq!(
            LpcCliffContourStampRole::VerticalRidgeMiddle.host_anchor_offset(),
            (-1, 0)
        );
    }

}
