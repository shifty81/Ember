//! Canonical gameplay/editor queries over derived structural terrain.
//!
//! Callers must consume `StructuralCellV2` instead of re-inferring cliffs from
//! legacy visual tiles. This keeps traversal, cave placement, waterfalls, and
//! editor diagnostics consistent with the full-world structural bake.

use serde::{Deserialize, Serialize};

use crate::{EdgeMaskV2, RampDirectionV2, StructuralCellV2};

pub const STRUCTURAL_QUERY_V2_SCHEMA: &str = "havenwild.structural_query.v2";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardinalDirectionV2 {
    North,
    East,
    South,
    West,
}

impl CardinalDirectionV2 {
    pub const fn edge_bit(self) -> u8 {
        match self {
            Self::North => EdgeMaskV2::NORTH,
            Self::East => EdgeMaskV2::EAST,
            Self::South => EdgeMaskV2::SOUTH,
            Self::West => EdgeMaskV2::WEST,
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralTraversalQueryV2 {
    pub blocked: bool,
    pub uses_ramp: bool,
    pub elevation_drop: i16,
}

/// Queries whether movement may leave one structural cell in `direction`.
///
/// A matching ramp explicitly opens the otherwise derived elevation edge.
/// Collision/pathfinding callers should query both source and destination
/// cells and reject movement if either side reports a blocked boundary.
pub fn traversal_from_structural_cell_v2(
    cell: StructuralCellV2,
    direction: CardinalDirectionV2,
) -> StructuralTraversalQueryV2 {
    let ramp = ramp_matches(cell.ramp, direction);
    StructuralTraversalQueryV2 {
        blocked: cell.blocked_edges.contains(direction.edge_bit()) && !ramp,
        uses_ramp: ramp,
        elevation_drop: if cell.exposed_edges.contains(direction.edge_bit()) {
            cell.edge_delta(direction.edge_bit()).max(0)
        } else {
            0
        },
    }
}

/// Returns true when the cell exposes a structural face in `direction`.
pub fn has_exposed_cliff_face_v2(cell: StructuralCellV2, direction: CardinalDirectionV2) -> bool {
    cell.exposed_edges.contains(direction.edge_bit())
}

/// Returns true only for resolver-certified cave hosts with the requested face.
pub fn cave_entrance_host_valid_v2(cell: StructuralCellV2, direction: CardinalDirectionV2) -> bool {
    cell.cave_host_eligible && has_exposed_cliff_face_v2(cell, direction)
}

/// Returns true for resolver-certified water drops suitable for a waterfall
/// visual/flow feature. Destination checks remain the caller's responsibility.
pub fn waterfall_source_valid_v2(cell: StructuralCellV2, direction: CardinalDirectionV2) -> bool {
    cell.waterfall_edges.contains(direction.edge_bit())
        && has_exposed_cliff_face_v2(cell, direction)
}

/// Returns true when this exact structural face terminates against water.
pub fn water_facing_cliff_v2(cell: StructuralCellV2, direction: CardinalDirectionV2) -> bool {
    cell.water_facing_edges.contains(direction.edge_bit())
        && has_exposed_cliff_face_v2(cell, direction)
}

fn ramp_matches(ramp: Option<RampDirectionV2>, direction: CardinalDirectionV2) -> bool {
    matches!(
        (ramp, direction),
        (Some(RampDirectionV2::North), CardinalDirectionV2::North)
            | (Some(RampDirectionV2::East), CardinalDirectionV2::East)
            | (Some(RampDirectionV2::South), CardinalDirectionV2::South)
            | (Some(RampDirectionV2::West), CardinalDirectionV2::West)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_ramp_opens_blocked_edge() {
        let cell = StructuralCellV2 {
            blocked_edges: EdgeMaskV2(EdgeMaskV2::NORTH),
            ramp: Some(RampDirectionV2::North),
            ..StructuralCellV2::default()
        };
        let query = traversal_from_structural_cell_v2(cell, CardinalDirectionV2::North);
        assert!(!query.blocked);
        assert!(query.uses_ramp);
    }

    #[test]
    fn cave_host_requires_matching_exposed_face() {
        let cell = StructuralCellV2 {
            exposed_edges: EdgeMaskV2(EdgeMaskV2::SOUTH),
            cave_host_eligible: true,
            ..StructuralCellV2::default()
        };
        assert!(cave_entrance_host_valid_v2(
            cell,
            CardinalDirectionV2::South
        ));
        assert!(!cave_entrance_host_valid_v2(
            cell,
            CardinalDirectionV2::North
        ));
    }
}
