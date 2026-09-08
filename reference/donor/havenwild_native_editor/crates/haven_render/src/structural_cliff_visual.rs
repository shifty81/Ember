use haven_assets::{
    authored_terrain_provider::AuthoredSourceCell,
    elizawy_cliff_provider::ElizaWyCliffCellRole,
};
use macroquad::prelude::Rect;

pub const ELIZAWY_SUMMER_CLIFF_SOURCE_PATH: &str =
    "assets/generated/worldgen_v0_1/terrain/elizawy_cliff_runtime_overlay_summer.png";
pub const ELIZAWY_WATERFALL_SOURCE_PATH: &str =
    "assets/source/licensed/lpc_revised/Terrain/Waterfall.png";

/// Shared runtime/editor visual interpretation layered over the canonical 15-shape
/// structural mask. Collision, traversal, saves, worldgen, and the player
/// world-builder continue to use `haven_world::CliffShape15` directly.
///
/// South-facing visual families are connected recipes. North/east/west cells
/// from the square construction template are compatibility-only until W7 owns
/// complete contour recipes; they must never be treated as a generic autotile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliffVisualShape {
    Orthogonal,
    SouthWestDiagonal,
    SouthEastDiagonal,
    /// Raised tips that expose south + both sides use the natural-scale
    /// authored rounded terminal stamp. Whole source cells are placed at their
    /// original left/center/right offsets; nothing is sliced or recomposed.
    SouthAuthoredTerminal,
}

/// Continuous ownership role for the perspective-visible south cliff band.
///
/// The structural four-neighbor mask already tells us whether a south-facing
/// host continues into same-tier terrain to the west/east. Resolve that once
/// into a band role instead of treating masks 4/5/6/7/12/13/14/15 as unrelated
/// visual cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SouthBandRole {
    None,
    Straight,
    WestTerminal,
    EastTerminal,
    DoubleTerminal,
}

/// Presentation-only ownership role for a rounded south corner inside a
/// generated stair-step diagonal contour.
///
/// This role controls connected-recipe ownership only. It must never compress
/// vertical height: a 1/2/3/4-tier diagonal uses the same structural module
/// count as every straight/corner/terminal segment on that contour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagonalChainRole {
    Isolated,
    Start,
    Middle,
    End,
}

/// Shared structural-cliff visual recipe for one semantic host cell.
///
/// This is deliberately renderer-agnostic: it resolves the canonical
/// `CliffShape15`, authored south-face family, uniform tier count and diagonal
/// chain ownership, but issues no GPU commands. Runtime and native editor must
/// consume this same record so they cannot drift into separate cliff topology
/// interpretations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CliffVisualRecipeV1 {
    pub shape: haven_world::CliffShape15,
    pub visual_shape: CliffVisualShape,
    pub south_band_role: SouthBandRole,
    pub diagonal_chain_role: DiagonalChainRole,
    pub north_exposed: bool,
    pub east_exposed: bool,
    pub south_exposed: bool,
    pub west_exposed: bool,
    pub north_face_segments: u8,
    pub east_face_segments: u8,
    pub south_face_segments: u8,
    pub west_face_segments: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct VerticalFaceRecipe {
    /// Optional authored crest/rim drawn on the structural host row. It is
    /// presentation on the upper level and does not consume a receiver-facing
    /// cliff-height row.
    pub leading: Option<Rect>,
    pub body: Rect,
    pub foot: Rect,
}

#[derive(Clone, Copy, Debug)]
pub struct DiagonalFaceRecipe {
    /// Authored diagonal crest/rim drawn on the structural host row.
    pub crest: Rect,
    /// Repeatable diagonal rock body. A height-N cliff uses N-1 body rows.
    pub body: Rect,
    /// Receiver-facing foot. Its low-side grass is transparent in the
    /// generated runtime projection so the actual V7 receiver terrain wins.
    pub foot: Rect,
}

const fn source_cell(cell: AuthoredSourceCell) -> Rect {
    Rect {
        x: cell.x_px() as f32,
        y: cell.y_px() as f32,
        w: 32.0,
        h: 32.0,
    }
}


const fn role_cell(role: ElizaWyCliffCellRole) -> Rect {
    source_cell(role.source_cell())
}


/// W8 exact connected-contour cells. The same source addresses also appear in
/// the historical square-plateau construction template, but these role names
/// are certified only for their matching contour topology. Whole 32x32 cells
/// are used at natural scale; adjacent corner shapes use the dedicated authored
/// corner cell rather than stacking straight-edge cells.
pub const NORTH_LIP_CELL: Rect = role_cell(ElizaWyCliffCellRole::ContourNorthStraight);
pub const WEST_EDGE_CELL: Rect = role_cell(ElizaWyCliffCellRole::ContourWestStraight);
pub const EAST_EDGE_CELL: Rect = role_cell(ElizaWyCliffCellRole::ContourEastStraight);
pub const NORTH_WEST_CORNER_CELL: Rect =
    role_cell(ElizaWyCliffCellRole::ContourNorthWestConvex);
pub const NORTH_EAST_CORNER_CELL: Rect =
    role_cell(ElizaWyCliffCellRole::ContourNorthEastConvex);

/// Default perspective-visible straight south wall, grounded in the official
/// ElizaWy Summer demo. Every source reference is an entire 32x32 authored cell.
pub const SOUTH_STRAIGHT_FACE: VerticalFaceRecipe = VerticalFaceRecipe {
    leading: Some(role_cell(ElizaWyCliffCellRole::StraightSouthTop)),
    body: role_cell(ElizaWyCliffCellRole::StraightSouthBody),
    foot: role_cell(ElizaWyCliffCellRole::StraightSouthFoot),
};

/// Authored rounded/diagonal assemblies, grounded in the official Summer demo.
/// The derived overlay removes semantic ground pixels but never changes the
/// rock/fringe geometry or source-cell footprint.
pub const SOUTH_WEST_DIAGONAL_FACE: DiagonalFaceRecipe = DiagonalFaceRecipe {
    crest: role_cell(ElizaWyCliffCellRole::RoundedSouthWestShoulder),
    body: role_cell(ElizaWyCliffCellRole::RoundedSouthWestBody),
    foot: role_cell(ElizaWyCliffCellRole::RoundedSouthWestFoot),
};
pub const SOUTH_EAST_DIAGONAL_FACE: DiagonalFaceRecipe = DiagonalFaceRecipe {
    crest: role_cell(ElizaWyCliffCellRole::RoundedSouthEastShoulder),
    body: role_cell(ElizaWyCliffCellRole::RoundedSouthEastBody),
    foot: role_cell(ElizaWyCliffCellRole::RoundedSouthEastFoot),
};

/// Natural-scale three-column rounded south terminal. W14 uses the same
/// crest/body/foot height grammar as straight and diagonal south faces so the
/// same structural tier terminates on the same world row all the way around.
pub const SOUTH_TERMINAL_CREST_ROW: [Rect; 3] = [
    role_cell(ElizaWyCliffCellRole::RoundedSouthWestShoulder),
    source_cell(AuthoredSourceCell::new(2, 7)),
    role_cell(ElizaWyCliffCellRole::RoundedSouthEastShoulder),
];
pub const SOUTH_TERMINAL_BODY_ROW: [Rect; 3] = [
    role_cell(ElizaWyCliffCellRole::RoundedSouthWestBody),
    source_cell(AuthoredSourceCell::new(2, 3)),
    role_cell(ElizaWyCliffCellRole::RoundedSouthEastBody),
];
pub const SOUTH_TERMINAL_FOOT_ROW: [Rect; 3] = [
    role_cell(ElizaWyCliffCellRole::RoundedSouthWestFoot),
    source_cell(AuthoredSourceCell::new(2, 8)),
    role_cell(ElizaWyCliffCellRole::RoundedSouthEastFoot),
];

/// Number of repeatable authored rock-body rows above the fixed foot.
///
/// The crest/rim is drawn on the upper structural host row and therefore does
/// not add visual cliff height. The receiver-facing wall is exactly N rows for
/// an N-tier drop: N-1 body rows followed by one authored foot row.
pub const fn authored_body_rows(face_segments: u8) -> usize {
    if face_segments <= 1 {
        0
    } else {
        (face_segments - 1) as usize
    }
}

/// Receiver rows occupied below the structural host by every perspective-visible
/// south cliff family at a given tier delta. A 1-high cliff occupies one row,
/// 2-high occupies two, and so on. Straight, diagonal, and terminal recipes
/// therefore terminate on the identical bottom row for the same tier delta.
pub const fn uniform_south_face_receiver_rows(face_segments: u8) -> usize {
    1 + authored_body_rows(face_segments)
}

/// Extra rock continuation above a doorway-scale three-row cave recipe. A
/// normal two-tier cliff already fits the cave top/body/foot envelope; only
/// tiers above two move the fixed cave mouth farther down the wall.
pub const fn extra_authored_body_rows(face_segments: u8) -> usize {
    if face_segments <= 2 {
        0
    } else {
        (face_segments - 2) as usize
    }
}

/// Resolve the visual module count for one specific exposed structural edge.
///
/// `StructuralCellV2::face_segments` is the maximum drop on the host cell and
/// therefore cannot safely size every edge of a mixed-height corner. Fresh
/// structural levels are encoded in `edge_deltas` using
/// `STRUCTURAL_ELEVATION_RESOLVER_STEP_V2` resolver units per level. Use the
/// requested edge delta first so a Level 2 host bordering Level 1 on one side
/// and Level 0 on another renders one module on the 2->1 edge and two modules
/// on the 2->0 edge. Older compatibility/test cells without edge deltas fall
/// back to the historical maximum segment count.
pub fn authored_face_segments_for_edge(
    cell: haven_world::StructuralCellV2,
    edge: u8,
) -> u8 {
    let delta = cell.edge_delta(edge);
    if delta > 0 {
        let step = haven_world::STRUCTURAL_ELEVATION_RESOLVER_STEP_V2.max(1);
        let segments = (delta + step - 1) / step;
        segments.clamp(1, i16::from(u8::MAX)) as u8
    } else if cell.exposed_edges.contains(edge) {
        cell.face_segments.max(1)
    } else {
        0
    }
}

pub fn authored_south_face_segments(cell: haven_world::StructuralCellV2) -> u8 {
    authored_face_segments_for_edge(cell, haven_world::EdgeMaskV2::SOUTH)
}

pub fn shape_for(cell: haven_world::StructuralCellV2) -> Option<haven_world::CliffShape15> {
    cell.cliff_shape_15()
}

pub const fn shape_exposes(shape: haven_world::CliffShape15, edge: u8) -> bool {
    shape.exposes(edge)
}

pub const fn south_band_role(
    shape: haven_world::CliffShape15,
) -> SouthBandRole {
    use haven_world::EdgeMaskV2;
    let south = shape_exposes(shape, EdgeMaskV2::SOUTH);
    if !south {
        return SouthBandRole::None;
    }
    let west = shape_exposes(shape, EdgeMaskV2::WEST);
    let east = shape_exposes(shape, EdgeMaskV2::EAST);
    match (west, east) {
        (false, false) => SouthBandRole::Straight,
        (true, false) => SouthBandRole::WestTerminal,
        (false, true) => SouthBandRole::EastTerminal,
        (true, true) => SouthBandRole::DoubleTerminal,
    }
}

pub const fn visual_shape_for(
    shape: haven_world::CliffShape15,
    _face_segments: u8,
    _north_west: Option<haven_world::CliffShape15>,
    _north_east: Option<haven_world::CliffShape15>,
    _south_west: Option<haven_world::CliffShape15>,
    _south_east: Option<haven_world::CliffShape15>,
) -> CliffVisualShape {
    match south_band_role(shape) {
        SouthBandRole::WestTerminal => CliffVisualShape::SouthWestDiagonal,
        SouthBandRole::EastTerminal => CliffVisualShape::SouthEastDiagonal,
        SouthBandRole::DoubleTerminal => CliffVisualShape::SouthAuthoredTerminal,
        SouthBandRole::None | SouthBandRole::Straight => CliffVisualShape::Orthogonal,
    }
}

pub const fn is_south_authored_terminal(shape: haven_world::CliffShape15) -> bool {
    matches!(south_band_role(shape), SouthBandRole::DoubleTerminal)
}

pub const fn south_diagonal_orientation(
    shape: haven_world::CliffShape15,
) -> Option<CliffVisualShape> {
    match south_band_role(shape) {
        SouthBandRole::WestTerminal => Some(CliffVisualShape::SouthWestDiagonal),
        SouthBandRole::EastTerminal => Some(CliffVisualShape::SouthEastDiagonal),
        SouthBandRole::None | SouthBandRole::Straight | SouthBandRole::DoubleTerminal => None,
    }
}

/// Classify a rounded south corner as an isolated authored corner or as one
/// member of a repeated diagonal/stair chain.
///
/// Generated contours frequently approximate a diagonal with rounded corner
/// hosts separated by one to four cells. Treating every such host as a complete
/// isolated corner adds one extra projected row per step. We search both
/// diagonals for the same canonical corner shape, select the strongest axis,
/// and preserve deterministic Start/Middle/End ownership without changing the
/// underlying `CliffShape15`.
pub fn diagonal_chain_role_from_lookup<F>(
    shape: haven_world::CliffShape15,
    mut lookup: F,
) -> DiagonalChainRole
where
    F: FnMut(i32, i32) -> Option<haven_world::CliffShape15>,
{
    let Some(orientation) = south_diagonal_orientation(shape) else {
        return DiagonalChainRole::Isolated;
    };

    // Chain membership follows compatible contour orientation rather than an
    // identical raw mask. For example SouthWest and NorthSouthWest are the same
    // visible southwest turn even though the latter also exposes a back rim.
    // Requiring exact mask equality made those mixed-mask stair chains appear
    // as repeated isolated posts.
    const SEARCH: i32 = 4;
    let has_same = |lookup: &mut F, dx_sign: i32, dy_sign: i32| {
        (1..=SEARCH).any(|distance| {
            lookup(dx_sign * distance, dy_sign * distance)
                .and_then(south_diagonal_orientation)
                .is_some_and(|other| other == orientation)
        })
    };

    let nw = has_same(&mut lookup, -1, -1);
    let se = has_same(&mut lookup, 1, 1);
    let ne = has_same(&mut lookup, 1, -1);
    let sw = has_same(&mut lookup, -1, 1);

    let primary = if u8::from(nw) + u8::from(se) >= u8::from(ne) + u8::from(sw) {
        (nw, se)
    } else {
        (ne, sw)
    };

    match primary {
        (false, false) => DiagonalChainRole::Isolated,
        (false, true) => DiagonalChainRole::Start,
        (true, false) => DiagonalChainRole::End,
        (true, true) => DiagonalChainRole::Middle,
    }
}

/// Resolve one structural host to the exact shared visual grammar used by
/// Havenwild renderers. `lookup` is relative to the host and supplies nearby
/// structural cells for diagonal-chain ownership; it is intentionally a
/// semantic lookup rather than a texture/sample callback.
pub fn resolve_cliff_visual_recipe_v1<F>(
    cell: haven_world::StructuralCellV2,
    mut lookup: F,
) -> Option<CliffVisualRecipeV1>
where
    F: FnMut(i32, i32) -> Option<haven_world::StructuralCellV2>,
{
    let shape = shape_for(cell)?;
    let neighbor_shape = |lookup: &mut F, dx: i32, dy: i32| {
        lookup(dx, dy).and_then(shape_for)
    };
    let north_west = neighbor_shape(&mut lookup, -1, -1);
    let north_east = neighbor_shape(&mut lookup, 1, -1);
    let south_west = neighbor_shape(&mut lookup, -1, 1);
    let south_east = neighbor_shape(&mut lookup, 1, 1);
    let south_face_segments = authored_south_face_segments(cell);
    let visual_shape = visual_shape_for(
        shape,
        south_face_segments.max(1),
        north_west,
        north_east,
        south_west,
        south_east,
    );
    let diagonal_chain_role = diagonal_chain_role_from_lookup(shape, |dx, dy| {
        lookup(dx, dy).and_then(shape_for)
    });

    Some(CliffVisualRecipeV1 {
        shape,
        visual_shape,
        south_band_role: south_band_role(shape),
        diagonal_chain_role,
        north_exposed: shape_exposes(shape, haven_world::EdgeMaskV2::NORTH),
        east_exposed: shape_exposes(shape, haven_world::EdgeMaskV2::EAST),
        south_exposed: shape_exposes(shape, haven_world::EdgeMaskV2::SOUTH),
        west_exposed: shape_exposes(shape, haven_world::EdgeMaskV2::WEST),
        north_face_segments: authored_face_segments_for_edge(
            cell,
            haven_world::EdgeMaskV2::NORTH,
        ),
        east_face_segments: authored_face_segments_for_edge(
            cell,
            haven_world::EdgeMaskV2::EAST,
        ),
        south_face_segments,
        west_face_segments: authored_face_segments_for_edge(
            cell,
            haven_world::EdgeMaskV2::WEST,
        ),
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_nonzero_cardinal_mask_has_one_world_shape() {
        for mask in 1_u8..=15 {
            let cell = haven_world::StructuralCellV2 {
                exposed_edges: haven_world::EdgeMaskV2(mask),
                ..Default::default()
            };
            assert_eq!(shape_for(cell).expect("shape").mask(), mask);
        }
    }

    #[test]
    fn plateau_projection_uses_only_whole_authored_cells() {
        assert_eq!(NORTH_LIP_CELL, Rect::new(64.0, 160.0, 32.0, 32.0));
        assert_eq!(WEST_EDGE_CELL, Rect::new(0.0, 160.0, 32.0, 32.0));
        assert_eq!(EAST_EDGE_CELL, Rect::new(128.0, 160.0, 32.0, 32.0));
        assert_eq!(NORTH_WEST_CORNER_CELL, Rect::new(32.0, 160.0, 32.0, 32.0));
        assert_eq!(NORTH_EAST_CORNER_CELL, Rect::new(96.0, 160.0, 32.0, 32.0));
    }

    #[test]
    fn straight_south_face_matches_official_demo_sequence() {
        assert_eq!(SOUTH_STRAIGHT_FACE.leading, Some(role_cell(ElizaWyCliffCellRole::StraightSouthTop)));
        assert_eq!(SOUTH_STRAIGHT_FACE.body, role_cell(ElizaWyCliffCellRole::StraightSouthBody));
        assert_eq!(SOUTH_STRAIGHT_FACE.foot, role_cell(ElizaWyCliffCellRole::StraightSouthFoot));
    }

    #[test]
    fn diagonal_faces_include_the_missing_demo_body_row() {
        assert_eq!(SOUTH_WEST_DIAGONAL_FACE.crest, role_cell(ElizaWyCliffCellRole::RoundedSouthWestShoulder));
        assert_eq!(SOUTH_WEST_DIAGONAL_FACE.body, role_cell(ElizaWyCliffCellRole::RoundedSouthWestBody));
        assert_eq!(SOUTH_WEST_DIAGONAL_FACE.foot, role_cell(ElizaWyCliffCellRole::RoundedSouthWestFoot));
        assert_eq!(SOUTH_EAST_DIAGONAL_FACE.body, role_cell(ElizaWyCliffCellRole::RoundedSouthEastBody));
    }

    #[test]
    fn exact_south_corners_use_authored_rounded_recipes() {
        assert_eq!(
            visual_shape_for(
                haven_world::CliffShape15::SouthWest,
                1,
                None,
                None,
                None,
                None,
            ),
            CliffVisualShape::SouthWestDiagonal
        );
        assert_eq!(
            visual_shape_for(
                haven_world::CliffShape15::EastSouth,
                1,
                None,
                None,
                None,
                None,
            ),
            CliffVisualShape::SouthEastDiagonal
        );
    }

    #[test]
    fn repeated_diagonal_corners_are_chain_owned() {
        use haven_world::CliffShape15;
        let role = diagonal_chain_role_from_lookup(CliffShape15::SouthWest, |dx, dy| {
            ((dx, dy) == (-2, -2) || (dx, dy) == (3, 3))
                .then_some(CliffShape15::SouthWest)
        });
        assert_eq!(role, DiagonalChainRole::Middle);
    }

    #[test]
    fn isolated_rounded_corner_keeps_full_projection() {
        let role =
            diagonal_chain_role_from_lookup(haven_world::CliffShape15::EastSouth, |_, _| None);
        assert_eq!(role, DiagonalChainRole::Isolated);
    }

    #[test]
    fn north_exposure_does_not_destroy_visible_south_diagonal_turn() {
        assert_eq!(
            visual_shape_for(
                haven_world::CliffShape15::NorthSouthWest,
                1,
                None,
                None,
                None,
                None,
            ),
            CliffVisualShape::SouthWestDiagonal
        );
        assert_eq!(
            visual_shape_for(
                haven_world::CliffShape15::NorthEastSouth,
                1,
                None,
                None,
                None,
                None,
            ),
            CliffVisualShape::SouthEastDiagonal
        );
    }

    #[test]
    fn mixed_masks_share_diagonal_chain_ownership() {
        use haven_world::CliffShape15;
        let role = diagonal_chain_role_from_lookup(CliffShape15::SouthWest, |dx, dy| {
            ((dx, dy) == (-2, -2))
                .then_some(CliffShape15::NorthSouthWest)
                .or_else(|| ((dx, dy) == (2, 2)).then_some(CliffShape15::SouthWest))
        });
        assert_eq!(role, DiagonalChainRole::Middle);
    }

    #[test]
    fn all_south_masks_resolve_through_one_band_grammar() {
        use haven_world::CliffShape15;
        assert_eq!(south_band_role(CliffShape15::South), SouthBandRole::Straight);
        assert_eq!(south_band_role(CliffShape15::NorthSouth), SouthBandRole::Straight);
        assert_eq!(south_band_role(CliffShape15::SouthWest), SouthBandRole::WestTerminal);
        assert_eq!(south_band_role(CliffShape15::NorthSouthWest), SouthBandRole::WestTerminal);
        assert_eq!(south_band_role(CliffShape15::EastSouth), SouthBandRole::EastTerminal);
        assert_eq!(south_band_role(CliffShape15::NorthEastSouth), SouthBandRole::EastTerminal);
        assert_eq!(south_band_role(CliffShape15::EastSouthWest), SouthBandRole::DoubleTerminal);
        assert_eq!(south_band_role(CliffShape15::Isolated), SouthBandRole::DoubleTerminal);
        assert_eq!(south_band_role(CliffShape15::North), SouthBandRole::None);
    }

    #[test]
    fn double_sided_front_masks_use_authored_terminal_stamp() {
        use haven_world::CliffShape15;
        for shape in [CliffShape15::EastSouthWest, CliffShape15::Isolated] {
            assert!(is_south_authored_terminal(shape));
            assert_eq!(
                visual_shape_for(shape, 1, None, None, None, None),
                CliffVisualShape::SouthAuthoredTerminal
            );
        }
        assert!(!is_south_authored_terminal(CliffShape15::South));
    }

    #[test]
    fn shared_recipe_preserves_uniform_level_two_south_geometry() {
        let mut cell = haven_world::StructuralCellV2::default();
        cell.exposed_edges.insert(haven_world::EdgeMaskV2::SOUTH);
        cell.edge_deltas[2] = 4;
        cell.face_segments = 2;
        let recipe = resolve_cliff_visual_recipe_v1(cell, |_, _| None).expect("visual recipe");
        assert_eq!(recipe.shape, haven_world::CliffShape15::South);
        assert_eq!(recipe.south_face_segments, 2);
        assert_eq!(recipe.visual_shape, CliffVisualShape::Orthogonal);
    }

    #[test]
    fn receiver_rows_match_requested_structural_height_one_through_four() {
        for face_segments in 1_u8..=4 {
            assert_eq!(
                uniform_south_face_receiver_rows(face_segments),
                usize::from(face_segments),
                "a {face_segments}-high cliff must occupy exactly {face_segments} receiver rows",
            );
            assert_eq!(
                authored_body_rows(face_segments) + 1,
                usize::from(face_segments),
            );
        }
    }
}

