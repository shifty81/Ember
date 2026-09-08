//! Canonical structural-level and cliff-edge resolver for Havenwild semantic terrain.
//!
//! Discrete structural levels are authoritative for traversal cliffs. The
//! compatibility `SurfaceCellV1::elevation` field carries those logical steps
//! through this resolver; raw geological height remains a separate hydrology/
//! material input and never creates traversal topology.

use crate::{CliffTopologyV1, SurfaceCellV1, WaterKindV1};
use serde::{Deserialize, Serialize};

pub const ELEVATION_CLIFF_V2_SCHEMA: &str = "havenwild.elevation_cliff.v2";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeMaskV2(pub u8);

impl EdgeMaskV2 {
    pub const NORTH: u8 = 1 << 0;
    pub const EAST: u8 = 1 << 1;
    pub const SOUTH: u8 = 1 << 2;
    pub const WEST: u8 = 1 << 3;

    pub fn contains(self, edge: u8) -> bool {
        self.0 & edge != 0
    }
    pub fn insert(&mut self, edge: u8) {
        self.0 |= edge;
    }
    pub fn count(self) -> u32 {
        self.0.count_ones()
    }
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// Canonical four-neighbor cliff autotile shape.
///
/// The structural level grid is the authority. Each exposed cardinal edge is
/// one bit (N=1, E=2, S=4, W=8); the fifteen non-zero masks are the complete
/// boundary-shape vocabulary used by world generation, rendering, collision
/// diagnostics, and the player world builder. Mask 0 is an interior platform
/// cell and needs no cliff boundary artwork.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliffShape15 {
    North,
    East,
    NorthEast,
    South,
    NorthSouth,
    EastSouth,
    NorthEastSouth,
    West,
    NorthWest,
    EastWest,
    NorthEastWest,
    SouthWest,
    NorthSouthWest,
    EastSouthWest,
    Isolated,
}

impl CliffShape15 {
    pub const fn from_mask(mask: u8) -> Option<Self> {
        match mask & 0x0f {
            0 => None,
            1 => Some(Self::North),
            2 => Some(Self::East),
            3 => Some(Self::NorthEast),
            4 => Some(Self::South),
            5 => Some(Self::NorthSouth),
            6 => Some(Self::EastSouth),
            7 => Some(Self::NorthEastSouth),
            8 => Some(Self::West),
            9 => Some(Self::NorthWest),
            10 => Some(Self::EastWest),
            11 => Some(Self::NorthEastWest),
            12 => Some(Self::SouthWest),
            13 => Some(Self::NorthSouthWest),
            14 => Some(Self::EastSouthWest),
            15 => Some(Self::Isolated),
            _ => None,
        }
    }

    pub const fn mask(self) -> u8 {
        match self {
            Self::North => 1,
            Self::East => 2,
            Self::NorthEast => 3,
            Self::South => 4,
            Self::NorthSouth => 5,
            Self::EastSouth => 6,
            Self::NorthEastSouth => 7,
            Self::West => 8,
            Self::NorthWest => 9,
            Self::EastWest => 10,
            Self::NorthEastWest => 11,
            Self::SouthWest => 12,
            Self::NorthSouthWest => 13,
            Self::EastSouthWest => 14,
            Self::Isolated => 15,
        }
    }

    pub const fn exposes(self, edge: u8) -> bool {
        self.mask() & edge != 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RampDirectionV2 {
    North,
    East,
    South,
    West,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElevationCliffSettingsV2 {
    /// Minimum logical structural-step delta that becomes a cliff edge.
    pub cliff_step_threshold: i16,
    /// Maximum elevation delta that remains eligible for a walkable ramp.
    pub maximum_ramp_step: i16,
    /// Minimum contiguous mountain depth behind a face for cave hosting.
    pub minimum_cave_host_depth: u16,
    /// Maximum number of elevation units represented by one visual face segment.
    pub face_segment_height: i16,
}

impl Default for ElevationCliffSettingsV2 {
    fn default() -> Self {
        Self {
            cliff_step_threshold: 2,
            maximum_ramp_step: 1,
            minimum_cave_host_depth: 2,
            face_segment_height: 2,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuralCellV2 {
    pub exposed_edges: EdgeMaskV2,
    pub blocked_edges: EdgeMaskV2,
    /// Signed structural elevation delta for N/E/S/W respectively. Positive
    /// means this cell is above the neighbor; negative means it is below.
    pub edge_deltas: [i16; 4],
    pub maximum_drop: i16,
    pub face_segments: u8,
    pub ramp: Option<RampDirectionV2>,
    /// Exact exposed edges whose lower neighbor is water.
    pub water_facing_edges: EdgeMaskV2,
    pub water_facing: bool,
    /// Exact exposed river edges eligible for animated waterfall connectors.
    pub waterfall_edges: EdgeMaskV2,
    pub waterfall_candidate: bool,
    pub cave_host_eligible: bool,
}

impl StructuralCellV2 {
    pub fn edge_delta(self, edge: u8) -> i16 {
        edge_index(edge)
            .and_then(|index| self.edge_deltas.get(index).copied())
            .unwrap_or(0)
    }

    /// Resolve this cell directly to the canonical 15-shape autotile key.
    /// No source-image dimensions or raw geological height participate.
    pub fn cliff_shape_15(self) -> Option<CliffShape15> {
        CliffShape15::from_mask(self.exposed_edges.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElevationCliffResolveReportV2 {
    pub schema: String,
    pub structural_cells: u32,
    pub cliff_edges: u32,
    pub multi_level_faces: u32,
    pub ramps: u32,
    pub water_facing_cliffs: u32,
    pub waterfall_candidates: u32,
    pub cave_host_faces: u32,
    pub changed_compatibility_cells: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElevationGridSpecV2 {
    pub width: usize,
    pub height: usize,
    pub wrap_east_west: bool,
}

impl ElevationGridSpecV2 {
    pub fn validate(self, cell_count: usize) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Err("elevation grid dimensions must be non-zero".to_owned());
        }
        let expected = self
            .width
            .checked_mul(self.height)
            .ok_or_else(|| "elevation grid dimensions overflow".to_owned())?;
        if expected != cell_count {
            return Err(format!(
                "elevation grid expected {expected} cells, found {cell_count}"
            ));
        }
        Ok(())
    }
}

pub fn resolve_elevation_cliffs_v2(
    cells: &mut [SurfaceCellV1],
    grid: ElevationGridSpecV2,
    settings: ElevationCliffSettingsV2,
) -> Result<(Vec<StructuralCellV2>, ElevationCliffResolveReportV2), String> {
    grid.validate(cells.len())?;
    if settings.cliff_step_threshold <= 0 {
        return Err("cliff_step_threshold must be positive".to_owned());
    }
    if settings.maximum_ramp_step < 0 {
        return Err("maximum_ramp_step cannot be negative".to_owned());
    }
    if settings.face_segment_height <= 0 {
        return Err("face_segment_height must be positive".to_owned());
    }

    let mut structures = vec![StructuralCellV2::default(); cells.len()];
    let mut report = ElevationCliffResolveReportV2 {
        schema: ELEVATION_CLIFF_V2_SCHEMA.to_owned(),
        structural_cells: 0,
        cliff_edges: 0,
        multi_level_faces: 0,
        ramps: 0,
        water_facing_cliffs: 0,
        waterfall_candidates: 0,
        cave_host_faces: 0,
        changed_compatibility_cells: 0,
    };

    for index in 0..cells.len() {
        let mut structure = StructuralCellV2::default();
        let current = cells[index];
        let mut gentle_downhill = Vec::new();

        for (edge, neighbor) in directional_neighbors(index, grid) {
            let Some(neighbor_index) = neighbor else {
                continue;
            };
            let other = cells[neighbor_index];
            let delta = current.elevation - other.elevation;
            if let Some(edge_index) = edge_index(edge) {
                structure.edge_deltas[edge_index] = delta;
            }

            if delta >= settings.cliff_step_threshold {
                structure.exposed_edges.insert(edge);
                structure.maximum_drop = structure.maximum_drop.max(delta);
                structure.face_segments = structure
                    .face_segments
                    .max(div_ceil_i16(delta, settings.face_segment_height) as u8);
                report.cliff_edges += 1;
                if delta > settings.face_segment_height {
                    report.multi_level_faces += 1;
                }
                if other.is_water() {
                    structure.water_facing_edges.insert(edge);
                    structure.water_facing = true;
                }
                if current.is_water()
                    && current.water_kind != WaterKindV1::Ocean
                    && other.is_water()
                {
                    structure.waterfall_edges.insert(edge);
                    structure.waterfall_candidate = true;
                }
            } else if delta <= -settings.cliff_step_threshold {
                structure.blocked_edges.insert(edge);
            } else if delta > 0 && delta <= settings.maximum_ramp_step {
                gentle_downhill.push(edge);
            }
        }

        if structure.exposed_edges.is_empty() && gentle_downhill.len() == 1 {
            structure.ramp = edge_to_ramp(gentle_downhill[0]);
            report.ramps += 1;
        }

        structure.cave_host_eligible = cave_host_eligible(
            index,
            cells,
            grid,
            structure,
            settings.minimum_cave_host_depth,
        );

        if !structure.exposed_edges.is_empty()
            || !structure.blocked_edges.is_empty()
            || structure.ramp.is_some()
        {
            report.structural_cells += 1;
        }
        if structure.water_facing {
            report.water_facing_cliffs += 1;
        }
        if structure.waterfall_candidate {
            report.waterfall_candidates += 1;
        }
        if structure.cave_host_eligible {
            report.cave_host_faces += 1;
        }

        let compatibility = compatibility_topology(structure.exposed_edges);
        if cells[index].cliff != compatibility {
            cells[index].cliff = compatibility;
            report.changed_compatibility_cells += 1;
        }
        structures[index] = structure;
    }

    Ok((structures, report))
}

fn cave_host_eligible(
    index: usize,
    cells: &[SurfaceCellV1],
    grid: ElevationGridSpecV2,
    structure: StructuralCellV2,
    minimum_depth: u16,
) -> bool {
    if structure.exposed_edges.count() != 1 || minimum_depth == 0 {
        return false;
    }
    let edge = first_edge(structure.exposed_edges);
    let Some((dx, dy)) = inward_vector(edge) else {
        return false;
    };
    let (x, y) = xy(index, grid.width);
    let host_elevation = cells[index].elevation;
    for step in 1..=minimum_depth as i32 {
        let Some(next_index) = index_at(x as i32 + dx * step, y as i32 + dy * step, grid) else {
            return false;
        };
        let cell = cells[next_index];
        if cell.is_water() || cell.elevation < host_elevation {
            return false;
        }
    }
    true
}

fn compatibility_topology(mask: EdgeMaskV2) -> CliffTopologyV1 {
    match mask.0 {
        EdgeMaskV2::NORTH => CliffTopologyV1::North,
        EdgeMaskV2::EAST => CliffTopologyV1::East,
        EdgeMaskV2::SOUTH => CliffTopologyV1::South,
        EdgeMaskV2::WEST => CliffTopologyV1::West,
        v if v == EdgeMaskV2::NORTH | EdgeMaskV2::EAST => CliffTopologyV1::OuterCornerNe,
        v if v == EdgeMaskV2::EAST | EdgeMaskV2::SOUTH => CliffTopologyV1::OuterCornerSe,
        v if v == EdgeMaskV2::SOUTH | EdgeMaskV2::WEST => CliffTopologyV1::OuterCornerSw,
        v if v == EdgeMaskV2::WEST | EdgeMaskV2::NORTH => CliffTopologyV1::OuterCornerNw,
        _ => CliffTopologyV1::None,
    }
}

fn directional_neighbors(index: usize, grid: ElevationGridSpecV2) -> [(u8, Option<usize>); 4] {
    let (x, y) = xy(index, grid.width);
    [
        (EdgeMaskV2::NORTH, index_at(x as i32, y as i32 - 1, grid)),
        (EdgeMaskV2::EAST, index_at(x as i32 + 1, y as i32, grid)),
        (EdgeMaskV2::SOUTH, index_at(x as i32, y as i32 + 1, grid)),
        (EdgeMaskV2::WEST, index_at(x as i32 - 1, y as i32, grid)),
    ]
}

fn index_at(x: i32, y: i32, grid: ElevationGridSpecV2) -> Option<usize> {
    if y < 0 || y >= grid.height as i32 {
        return None;
    }
    let x = if grid.wrap_east_west {
        x.rem_euclid(grid.width as i32)
    } else if x < 0 || x >= grid.width as i32 {
        return None;
    } else {
        x
    };
    Some(y as usize * grid.width + x as usize)
}

fn xy(index: usize, width: usize) -> (usize, usize) {
    (index % width, index / width)
}
fn div_ceil_i16(value: i16, divisor: i16) -> i16 {
    (value + divisor - 1) / divisor
}
fn first_edge(mask: EdgeMaskV2) -> u8 {
    [
        EdgeMaskV2::NORTH,
        EdgeMaskV2::EAST,
        EdgeMaskV2::SOUTH,
        EdgeMaskV2::WEST,
    ]
    .into_iter()
    .find(|edge| mask.contains(*edge))
    .unwrap_or(0)
}
fn inward_vector(exposed_edge: u8) -> Option<(i32, i32)> {
    match exposed_edge {
        EdgeMaskV2::NORTH => Some((0, 1)),
        EdgeMaskV2::EAST => Some((-1, 0)),
        EdgeMaskV2::SOUTH => Some((0, -1)),
        EdgeMaskV2::WEST => Some((1, 0)),
        _ => None,
    }
}

fn edge_index(edge: u8) -> Option<usize> {
    match edge {
        EdgeMaskV2::NORTH => Some(0),
        EdgeMaskV2::EAST => Some(1),
        EdgeMaskV2::SOUTH => Some(2),
        EdgeMaskV2::WEST => Some(3),
        _ => None,
    }
}

fn edge_to_ramp(edge: u8) -> Option<RampDirectionV2> {
    match edge {
        EdgeMaskV2::NORTH => Some(RampDirectionV2::North),
        EdgeMaskV2::EAST => Some(RampDirectionV2::East),
        EdgeMaskV2::SOUTH => Some(RampDirectionV2::South),
        EdgeMaskV2::WEST => Some(RampDirectionV2::West),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GenerationStageId, SurfaceMaterialV1, WaterDepthV1};

    fn dry(elevation: i16) -> SurfaceCellV1 {
        SurfaceCellV1::dry(
            SurfaceMaterialV1::Rock,
            elevation,
            GenerationStageId::Elevation,
        )
    }

    #[test]
    fn all_fifteen_boundary_masks_have_one_canonical_shape() {
        for mask in 1_u8..=15 {
            let shape = CliffShape15::from_mask(mask).expect("canonical cliff shape");
            assert_eq!(shape.mask(), mask);
            let cell = StructuralCellV2 {
                exposed_edges: EdgeMaskV2(mask),
                ..Default::default()
            };
            assert_eq!(cell.cliff_shape_15(), Some(shape));
        }
        assert_eq!(CliffShape15::from_mask(0), None);
    }

    #[test]
    fn derives_multi_level_south_cliff() {
        let mut cells = vec![dry(6), dry(6), dry(0), dry(0)];
        let grid = ElevationGridSpecV2 {
            width: 2,
            height: 2,
            wrap_east_west: false,
        };
        let (structures, report) =
            resolve_elevation_cliffs_v2(&mut cells, grid, ElevationCliffSettingsV2::default())
                .unwrap();
        assert!(structures[0].exposed_edges.contains(EdgeMaskV2::SOUTH));
        assert_eq!(structures[0].edge_delta(EdgeMaskV2::SOUTH), 6);
        assert_eq!(structures[2].edge_delta(EdgeMaskV2::NORTH), -6);
        assert!(structures[0].face_segments >= 3);
        assert!(report.multi_level_faces > 0);
    }

    #[test]
    fn recognizes_water_facing_cliff() {
        let mut water = dry(0);
        water.water_kind = WaterKindV1::Fresh;
        water.water_depth = WaterDepthV1::Shallow;
        let mut cells = vec![dry(4), water];
        let grid = ElevationGridSpecV2 {
            width: 2,
            height: 1,
            wrap_east_west: false,
        };
        let (structures, _) =
            resolve_elevation_cliffs_v2(&mut cells, grid, ElevationCliffSettingsV2::default())
                .unwrap();
        assert!(structures[0].water_facing);
        assert!(structures[0].water_facing_edges.contains(EdgeMaskV2::EAST));
    }

    #[test]
    fn river_drop_records_exact_waterfall_edge() {
        let mut river = dry(4);
        river.water_kind = WaterKindV1::River;
        river.water_depth = WaterDepthV1::Shallow;
        let mut lower = dry(0);
        lower.water_kind = WaterKindV1::River;
        lower.water_depth = WaterDepthV1::Shallow;
        let mut cells = vec![river, lower];
        let grid = ElevationGridSpecV2 {
            width: 1,
            height: 2,
            wrap_east_west: false,
        };
        let (structures, _) =
            resolve_elevation_cliffs_v2(&mut cells, grid, ElevationCliffSettingsV2::default())
                .unwrap();
        assert!(structures[0].waterfall_candidate);
        assert!(structures[0].waterfall_edges.contains(EdgeMaskV2::SOUTH));
    }

    #[test]
    fn freshwater_source_drop_also_records_waterfall_edge() {
        let mut source = dry(4);
        source.water_kind = WaterKindV1::Fresh;
        source.water_depth = WaterDepthV1::Shallow;
        let mut lower = dry(0);
        lower.water_kind = WaterKindV1::River;
        lower.water_depth = WaterDepthV1::Shallow;
        let mut cells = vec![source, lower];
        let grid = ElevationGridSpecV2 {
            width: 1,
            height: 2,
            wrap_east_west: false,
        };
        let (structures, _) =
            resolve_elevation_cliffs_v2(&mut cells, grid, ElevationCliffSettingsV2::default())
                .unwrap();
        assert!(structures[0].waterfall_candidate);
        assert!(structures[0].waterfall_edges.contains(EdgeMaskV2::SOUTH));
    }

    #[test]
    fn validates_cave_host_depth() {
        let mut cells = vec![dry(0), dry(4), dry(4), dry(4)];
        let grid = ElevationGridSpecV2 {
            width: 4,
            height: 1,
            wrap_east_west: false,
        };
        let settings = ElevationCliffSettingsV2 {
            minimum_cave_host_depth: 2,
            ..Default::default()
        };
        let (structures, _) = resolve_elevation_cliffs_v2(&mut cells, grid, settings).unwrap();
        assert!(structures[1].cave_host_eligible);
    }
}
