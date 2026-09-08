//! ElizaWy/LPC cliff source vocabulary and certification state used by Havenwild.
//!
//! A 32x32 source cell is an address inside `Terrain/cliff_summer.png`; it is
//! not automatically an independently placeable world tile. Runtime promotion
//! happens at the connected-recipe level. W6 keeps older square-plateau cells
//! visible only as explicitly named compatibility-template references while the
//! continuous contour assembler replaces them with certified connected recipes.
//! No API in this module crops, stretches, mirrors, rotates, or synthesizes art.

use crate::authored_terrain_provider::{AuthoredSourceCell, AuthoredSourceStamp};

pub const ELIZAWY_CLIFF_SOURCE_COMMIT: &str = "f07f7f5892e67c932c68f70bb04472f2c64e46bc";
pub const ELIZAWY_CLIFF_SOURCE_PATH: &str = "Terrain/cliff_summer.png";
pub const ELIZAWY_CLIFF_COLUMNS: u8 = 16;
pub const ELIZAWY_CLIFF_ROWS: u8 = 14;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElizaWyCliffCertification {
    /// Exact source role/connected recipe is accepted for runtime placement.
    RuntimeCertified,
    /// Existing compatibility display only. The source region is a construction
    /// template and may not be promoted as a new standalone runtime recipe.
    CompatibilityTemplateOnly,
    /// Useful inspection/evidence window, but not independently placeable.
    SourceReferenceOnly,
    /// A formerly attempted partial crop/assembly that must stay retired.
    RejectedPartialAssembly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElizaWyCliffCellRole {
    // Square-plateau construction-template addresses. These are retained only
    // so the W5 compatibility renderer can remain visually stable until the
    // W7 connected contour assembler owns complete north/east/west recipes.
    SquareNorthLip,
    SquareWestSide,
    SquareEastSide,
    SquareSouthLip,

    // W8 exact connected-contour roles from the same square grass plateau
    // template. These duplicate source addresses intentionally: the older
    // Square* names remain historical construction-template references, while
    // Contour* names are runtime-certified only for their exact topological
    // use. No arbitrary template-cell stacking is implied.
    ContourNorthStraight,
    ContourWestStraight,
    ContourEastStraight,
    ContourNorthWestConvex,
    ContourNorthEastConvex,
    ContourSouthStraightLip,

    // Runtime-certified south-facing connected-family cells.
    RoundedSouthWestLip,
    RoundedSouthWestShoulder,
    RoundedSouthWestBody,
    RoundedSouthWestFoot,
    RoundedSouthEastLip,
    RoundedSouthEastShoulder,
    RoundedSouthEastBody,
    RoundedSouthEastFoot,
    StraightSouthTop,
    StraightSouthBody,
    StraightSouthFoot,
    NarrowCaveTop,
    NarrowCaveBody,
    NarrowCaveFoot,
    LadderATop,
    LadderABody,
    LadderAFoot,
    LadderBTop,
    LadderBBody,
    LadderBFoot,
}

impl ElizaWyCliffCellRole {
    pub const fn certification(self) -> ElizaWyCliffCertification {
        match self {
            Self::SquareNorthLip
            | Self::SquareWestSide
            | Self::SquareEastSide
            | Self::SquareSouthLip => ElizaWyCliffCertification::CompatibilityTemplateOnly,
            _ => ElizaWyCliffCertification::RuntimeCertified,
        }
    }

    /// Returns whether this authored 32x32 cell is a certified vertical middle
    /// module. Taller cliffs repeat these complete cells at natural scale; the
    /// top/rim and foot cells remain fixed and are never stretched.
    pub const fn is_repeatable_vertical_body(self) -> bool {
        matches!(
            self,
            Self::RoundedSouthWestBody
                | Self::RoundedSouthEastBody
                | Self::StraightSouthBody
                | Self::LadderABody
                | Self::LadderBBody
        )
    }

    pub const fn source_cell(self) -> AuthoredSourceCell {
        match self {
            Self::SquareNorthLip => AuthoredSourceCell::new(6, 5),
            Self::SquareWestSide => AuthoredSourceCell::new(5, 6),
            Self::SquareEastSide => AuthoredSourceCell::new(7, 6),
            Self::SquareSouthLip => AuthoredSourceCell::new(6, 7),
            Self::ContourNorthStraight => AuthoredSourceCell::new(2, 5),
            Self::ContourWestStraight => AuthoredSourceCell::new(0, 5),
            Self::ContourEastStraight => AuthoredSourceCell::new(4, 5),
            Self::ContourNorthWestConvex => AuthoredSourceCell::new(1, 5),
            Self::ContourNorthEastConvex => AuthoredSourceCell::new(3, 5),
            Self::ContourSouthStraightLip => AuthoredSourceCell::new(6, 7),
            Self::RoundedSouthWestLip => AuthoredSourceCell::new(1, 6),
            Self::RoundedSouthWestShoulder => AuthoredSourceCell::new(1, 7),
            Self::RoundedSouthWestBody => AuthoredSourceCell::new(1, 3),
            Self::RoundedSouthWestFoot => AuthoredSourceCell::new(1, 8),
            Self::RoundedSouthEastLip => AuthoredSourceCell::new(3, 6),
            Self::RoundedSouthEastShoulder => AuthoredSourceCell::new(3, 7),
            Self::RoundedSouthEastBody => AuthoredSourceCell::new(3, 3),
            Self::RoundedSouthEastFoot => AuthoredSourceCell::new(3, 8),
            // W14 source-authority correction: the official ElizaWy demo's
            // ordinary south wall is the c2 crest/body/foot family. The older
            // c10r9-r11 feature column is not generic south-wall authority.
            Self::StraightSouthTop => AuthoredSourceCell::new(2, 7),
            Self::StraightSouthBody => AuthoredSourceCell::new(2, 3),
            Self::StraightSouthFoot => AuthoredSourceCell::new(2, 8),
            Self::NarrowCaveTop => AuthoredSourceCell::new(6, 9),
            Self::NarrowCaveBody => AuthoredSourceCell::new(6, 10),
            Self::NarrowCaveFoot => AuthoredSourceCell::new(6, 11),
            Self::LadderATop => AuthoredSourceCell::new(11, 9),
            Self::LadderABody => AuthoredSourceCell::new(11, 10),
            Self::LadderAFoot => AuthoredSourceCell::new(11, 11),
            Self::LadderBTop => AuthoredSourceCell::new(13, 9),
            Self::LadderBBody => AuthoredSourceCell::new(13, 10),
            Self::LadderBFoot => AuthoredSourceCell::new(13, 11),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElizaWyCliffStampRole {
    BareRoundedPlateauReference,
    BareSquarePlateauReference,
    GrassRoundedPlateauReference,
    GrassSquarePlateauReference,
    ComplexTransitionStripReference,
    WaterValleyA,
    WaterValleyB,
    WaterBridgeBayA,
    WaterBridgeBayB,
    WideCave,
    VineWindowLeft,
    VineWindowCenter,
    RightSideTerminalReference,
}

impl ElizaWyCliffStampRole {
    pub const fn certification(self) -> ElizaWyCliffCertification {
        match self {
            Self::WideCave | Self::WaterValleyA | Self::WaterValleyB => {
                ElizaWyCliffCertification::RuntimeCertified
            }
            Self::ComplexTransitionStripReference => {
                ElizaWyCliffCertification::RejectedPartialAssembly
            }
            Self::BareRoundedPlateauReference
            | Self::BareSquarePlateauReference
            | Self::GrassRoundedPlateauReference
            | Self::GrassSquarePlateauReference
            | Self::WaterBridgeBayA
            | Self::WaterBridgeBayB
            | Self::VineWindowLeft
            | Self::VineWindowCenter
            | Self::RightSideTerminalReference => ElizaWyCliffCertification::SourceReferenceOnly,
        }
    }

    pub const fn source_stamp(self) -> AuthoredSourceStamp {
        match self {
            Self::BareRoundedPlateauReference => AuthoredSourceStamp::new(0, 0, 5, 5),
            Self::BareSquarePlateauReference => AuthoredSourceStamp::new(5, 0, 3, 5),
            Self::GrassRoundedPlateauReference => AuthoredSourceStamp::new(0, 5, 5, 4),
            Self::GrassSquarePlateauReference => AuthoredSourceStamp::new(5, 5, 3, 4),
            Self::ComplexTransitionStripReference => AuthoredSourceStamp::new(8, 0, 1, 9),
            Self::WaterValleyA => AuthoredSourceStamp::new(9, 0, 3, 3),
            Self::WaterValleyB => AuthoredSourceStamp::new(12, 0, 3, 3),
            Self::WaterBridgeBayA => AuthoredSourceStamp::new(9, 3, 2, 4),
            Self::WaterBridgeBayB => AuthoredSourceStamp::new(12, 3, 2, 4),
            Self::WideCave => AuthoredSourceStamp::new(7, 9, 3, 3),
            Self::VineWindowLeft => AuthoredSourceStamp::new(0, 9, 3, 5),
            Self::VineWindowCenter => AuthoredSourceStamp::new(3, 9, 3, 4),
            Self::RightSideTerminalReference => AuthoredSourceStamp::new(15, 0, 1, 5),
        }
    }
}

/// Runtime promotion is decided at the connected assembly/recipe level, not by
/// asking whether an arbitrary source cell looks useful in isolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElizaWyCliffConnectedRecipeRole {
    StraightSouthFace,
    RoundedSouthWestFace,
    RoundedSouthEastFace,
    RoundedSouthTerminal,
    NarrowCaveFace,
    WideCaveFace,
    LadderA,
    LadderB,
    WaterValleyA,
    WaterValleyB,
    SquarePlateauConstructionTemplate,
    ComplexTransitionStrip,
    RightSideTerminalReference,
    SideEntryRampPending,
}

impl ElizaWyCliffConnectedRecipeRole {
    pub const fn certification(self) -> ElizaWyCliffCertification {
        match self {
            Self::StraightSouthFace
            | Self::RoundedSouthWestFace
            | Self::RoundedSouthEastFace
            | Self::RoundedSouthTerminal
            | Self::NarrowCaveFace
            | Self::WideCaveFace
            | Self::LadderA
            | Self::LadderB
            | Self::WaterValleyA
            | Self::WaterValleyB => ElizaWyCliffCertification::RuntimeCertified,
            Self::SquarePlateauConstructionTemplate
            | Self::RightSideTerminalReference
            | Self::SideEntryRampPending => ElizaWyCliffCertification::SourceReferenceOnly,
            Self::ComplexTransitionStrip => ElizaWyCliffCertification::RejectedPartialAssembly,
        }
    }
}

/// W8 runtime certification for the stable fresh-PCG contour vocabulary.
///
/// Generated macro landforms are normalized before cliff baking, so cells with
/// three/four exposed edges (one-cell caps/posts) are removed. The remaining
/// ten masks map to exact authored cells/assemblies. `NorthSouthRidge` composes
/// opposite top/bottom modules that have disjoint alpha footprints;
/// `EastWestRidge` uses a dedicated complete 3x1 ridge row from the companion
/// LPC cliff-family sheet and therefore never overlays incompatible side cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ElizaWyGeneratedCliffContourRole {
    NorthStraight,
    EastStraight,
    NorthEastConvex,
    SouthStraight,
    NorthSouthRidge,
    SouthEastRounded,
    WestStraight,
    NorthWestConvex,
    EastWestRidge,
    SouthWestRounded,
}

impl ElizaWyGeneratedCliffContourRole {
    pub const fn certification(self) -> ElizaWyCliffCertification {
        let _ = self;
        ElizaWyCliffCertification::RuntimeCertified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn straight_south_face_uses_demo_grounded_authored_cells() {
        assert_eq!(
            ElizaWyCliffCellRole::StraightSouthTop.source_cell(),
            AuthoredSourceCell::new(2, 7)
        );
        assert_eq!(
            ElizaWyCliffCellRole::StraightSouthBody.source_cell(),
            AuthoredSourceCell::new(2, 3)
        );
        assert_eq!(
            ElizaWyCliffCellRole::StraightSouthFoot.source_cell(),
            AuthoredSourceCell::new(2, 8)
        );
    }

    #[test]
    fn square_template_cells_are_not_new_runtime_recipes() {
        for role in [
            ElizaWyCliffCellRole::SquareNorthLip,
            ElizaWyCliffCellRole::SquareWestSide,
            ElizaWyCliffCellRole::SquareEastSide,
            ElizaWyCliffCellRole::SquareSouthLip,
        ] {
            assert_eq!(
                role.certification(),
                ElizaWyCliffCertification::CompatibilityTemplateOnly
            );
        }
        assert_eq!(
            ElizaWyCliffConnectedRecipeRole::SquarePlateauConstructionTemplate.certification(),
            ElizaWyCliffCertification::SourceReferenceOnly
        );
    }

    #[test]
    fn rejected_complex_transition_strip_cannot_be_promoted_as_a_ramp() {
        assert_eq!(
            ElizaWyCliffStampRole::ComplexTransitionStripReference.certification(),
            ElizaWyCliffCertification::RejectedPartialAssembly
        );
        assert_eq!(
            ElizaWyCliffConnectedRecipeRole::SideEntryRampPending.certification(),
            ElizaWyCliffCertification::SourceReferenceOnly
        );
    }

    #[test]
    fn certified_vertical_middle_modules_are_explicit() {
        assert!(ElizaWyCliffCellRole::StraightSouthBody.is_repeatable_vertical_body());
        assert!(ElizaWyCliffCellRole::RoundedSouthWestBody.is_repeatable_vertical_body());
        assert!(ElizaWyCliffCellRole::RoundedSouthEastBody.is_repeatable_vertical_body());
        assert!(ElizaWyCliffCellRole::LadderABody.is_repeatable_vertical_body());
        assert!(ElizaWyCliffCellRole::LadderBBody.is_repeatable_vertical_body());
        assert!(!ElizaWyCliffCellRole::StraightSouthTop.is_repeatable_vertical_body());
        assert!(!ElizaWyCliffCellRole::StraightSouthFoot.is_repeatable_vertical_body());
    }

    #[test]
    fn all_catalogued_cells_are_inside_the_pinned_sheet_grid() {
        let roles = [
            ElizaWyCliffCellRole::SquareNorthLip,
            ElizaWyCliffCellRole::SquareWestSide,
            ElizaWyCliffCellRole::SquareEastSide,
            ElizaWyCliffCellRole::SquareSouthLip,
            ElizaWyCliffCellRole::StraightSouthTop,
            ElizaWyCliffCellRole::StraightSouthBody,
            ElizaWyCliffCellRole::StraightSouthFoot,
            ElizaWyCliffCellRole::RoundedSouthWestLip,
            ElizaWyCliffCellRole::RoundedSouthEastLip,
        ];
        for role in roles {
            let cell = role.source_cell();
            assert!(cell.column < ELIZAWY_CLIFF_COLUMNS);
            assert!(cell.row < ELIZAWY_CLIFF_ROWS);
        }
    }

    #[test]
    fn w8_exact_contour_roles_use_whole_authored_cells() {
        assert_eq!(
            ElizaWyCliffCellRole::ContourNorthStraight.source_cell(),
            AuthoredSourceCell::new(2, 5)
        );
        assert_eq!(
            ElizaWyCliffCellRole::ContourNorthWestConvex.source_cell(),
            AuthoredSourceCell::new(1, 5)
        );
        assert_eq!(
            ElizaWyCliffCellRole::ContourNorthEastConvex.source_cell(),
            AuthoredSourceCell::new(3, 5)
        );
        assert_eq!(
            ElizaWyCliffCellRole::ContourSouthStraightLip.source_cell(),
            AuthoredSourceCell::new(6, 7)
        );
        for role in [
            ElizaWyCliffCellRole::ContourNorthStraight,
            ElizaWyCliffCellRole::ContourWestStraight,
            ElizaWyCliffCellRole::ContourEastStraight,
            ElizaWyCliffCellRole::ContourNorthWestConvex,
            ElizaWyCliffCellRole::ContourNorthEastConvex,
            ElizaWyCliffCellRole::ContourSouthStraightLip,
        ] {
            assert_eq!(role.certification(), ElizaWyCliffCertification::RuntimeCertified);
        }
    }

}
