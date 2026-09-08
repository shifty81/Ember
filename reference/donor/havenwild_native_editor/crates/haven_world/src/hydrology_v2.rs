//! Canonical hydrology resolver for semantic surface cells.
//!
//! This module owns connected-water identification, water-depth derivation,
//! river flow, freeze state, and gameplay-facing water queries. Rendering and
//! atlas selection consume these results; they do not mutate them.

use crate::{FreezeStateV1, SurfaceCellV1, WaterDepthV1, WaterKindV1};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

pub const HYDROLOGY_V2_SCHEMA: &str = "havenwild.hydrology.v2";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydrologySettingsV2 {
    /// Number of orthogonal water-cell steps from land that remain shallow.
    pub shallow_rim_tiles: u16,
    /// Minimum connected water-body area before a deep interior is allowed.
    pub minimum_deep_area: u32,
    /// Temperature at or below which skim ice is produced.
    pub skim_ice_tenths_c: i16,
    /// Temperature at or below which still shallow/fresh water freezes solid.
    pub frozen_tenths_c: i16,
    /// Maximum absolute flow component written to a cell.
    pub maximum_flow_milli: i16,
}

impl Default for HydrologySettingsV2 {
    fn default() -> Self {
        Self {
            shallow_rim_tiles: 1,
            minimum_deep_area: 9,
            skim_ice_tenths_c: 0,
            frozen_tenths_c: -40,
            maximum_flow_milli: 1_000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaterBodyClassV2 {
    Pond,
    Lake,
    River,
    Wetland,
    Ocean,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaterBodyRecordV2 {
    pub schema: String,
    pub water_body_id: u64,
    pub class: WaterBodyClassV2,
    pub cell_count: u32,
    pub shallow_cells: u32,
    pub deep_cells: u32,
    pub minimum_elevation: i16,
    pub maximum_elevation: i16,
    pub touches_north_boundary: bool,
    pub touches_south_boundary: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydrologyResolveReportV2 {
    pub schema: String,
    pub water_bodies: Vec<WaterBodyRecordV2>,
    pub changed_cells: u32,
    pub unresolved_water_cells: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HydrologyGridSpecV2 {
    pub width: usize,
    pub height: usize,
    pub wrap_east_west: bool,
}

impl HydrologyGridSpecV2 {
    pub fn validate(self, cell_count: usize) -> Result<(), String> {
        if self.width == 0 || self.height == 0 {
            return Err("hydrology grid dimensions must be non-zero".to_owned());
        }
        let expected = self
            .width
            .checked_mul(self.height)
            .ok_or_else(|| "hydrology grid dimensions overflow".to_owned())?;
        if expected != cell_count {
            return Err(format!(
                "hydrology grid expected {expected} cells, found {cell_count}"
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WaterQueryV2 {
    pub water_body_id: u64,
    pub water_kind: WaterKindV1,
    pub depth: WaterDepthV1,
    pub flow_x_milli: i16,
    pub flow_y_milli: i16,
    pub freeze_state: FreezeStateV1,
    pub navigable_depth_rank: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FishingHabitatQueryV2 {
    pub water_body_id: u64,
    pub freshwater: bool,
    pub flowing: bool,
    pub deep: bool,
    pub frozen: bool,
    pub habitat_score: u16,
}

pub fn resolve_hydrology_v2(
    cells: &mut [SurfaceCellV1],
    grid: HydrologyGridSpecV2,
    settings: HydrologySettingsV2,
) -> Result<HydrologyResolveReportV2, String> {
    grid.validate(cells.len())?;
    let mut changed = 0_u32;
    let mut visited = vec![false; cells.len()];
    let mut bodies = Vec::new();
    let mut next_body_ordinal = 1_u64;

    for start in 0..cells.len() {
        if visited[start] || !cells[start].is_water() {
            continue;
        }

        let mut queue = VecDeque::new();
        let mut component = Vec::new();
        visited[start] = true;
        queue.push_back(start);

        while let Some(index) = queue.pop_front() {
            component.push(index);
            for neighbor in orthogonal_neighbors(index, grid) {
                if !visited[neighbor] && cells[neighbor].is_water() {
                    visited[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }

        let body_id = stable_body_id(component[0], next_body_ordinal);
        next_body_ordinal += 1;
        let class = classify_body(&component, cells, grid);
        let distance_to_land = distance_from_land(&component, cells, grid);
        let allow_deep = component.len() as u32 >= settings.minimum_deep_area;

        let mut shallow_cells = 0_u32;
        let mut deep_cells = 0_u32;
        let mut min_elevation = i16::MAX;
        let mut max_elevation = i16::MIN;
        let mut touches_north = false;
        let mut touches_south = false;

        for &index in &component {
            let old = cells[index];
            let (x, y) = xy(index, grid.width);
            touches_north |= y == 0;
            touches_south |= y + 1 == grid.height;
            min_elevation = min_elevation.min(old.elevation);
            max_elevation = max_elevation.max(old.elevation);

            let distance = distance_to_land[index];
            let next_depth = if allow_deep && distance > settings.shallow_rim_tiles as u32 {
                WaterDepthV1::Deep
            } else if matches!(class, WaterBodyClassV2::Wetland) {
                WaterDepthV1::Wading
            } else {
                WaterDepthV1::Shallow
            };

            let (flow_x, flow_y) =
                derive_flow(index, cells, grid, class, settings.maximum_flow_milli);
            let next_freeze = derive_freeze_state(old, class, settings);

            cells[index].water_body_id = Some(body_id);
            cells[index].water_depth = next_depth;
            cells[index].flow_x_milli = flow_x;
            cells[index].flow_y_milli = flow_y;
            cells[index].freeze_state = next_freeze;

            if cells[index] != old {
                changed += 1;
            }
            match next_depth {
                WaterDepthV1::Deep => deep_cells += 1,
                WaterDepthV1::Wading | WaterDepthV1::Shallow => shallow_cells += 1,
                WaterDepthV1::Dry => {}
            }
            let _ = x;
        }

        bodies.push(WaterBodyRecordV2 {
            schema: HYDROLOGY_V2_SCHEMA.to_owned(),
            water_body_id: body_id,
            class,
            cell_count: component.len() as u32,
            shallow_cells,
            deep_cells,
            minimum_elevation: min_elevation,
            maximum_elevation: max_elevation,
            touches_north_boundary: touches_north,
            touches_south_boundary: touches_south,
        });
    }

    let unresolved = cells
        .iter()
        .filter(|cell| cell.is_water() && cell.water_body_id.is_none())
        .count() as u32;
    Ok(HydrologyResolveReportV2 {
        schema: HYDROLOGY_V2_SCHEMA.to_owned(),
        water_bodies: bodies,
        changed_cells: changed,
        unresolved_water_cells: unresolved,
    })
}

pub fn water_query_at_v2(
    cells: &[SurfaceCellV1],
    grid: HydrologyGridSpecV2,
    x: usize,
    y: usize,
) -> Option<WaterQueryV2> {
    if x >= grid.width || y >= grid.height {
        return None;
    }
    let cell = cells[y * grid.width + x];
    let water_body_id = cell.water_body_id?;
    Some(WaterQueryV2 {
        water_body_id,
        water_kind: cell.water_kind,
        depth: cell.water_depth,
        flow_x_milli: cell.flow_x_milli,
        flow_y_milli: cell.flow_y_milli,
        freeze_state: cell.freeze_state,
        navigable_depth_rank: match cell.water_depth {
            WaterDepthV1::Dry => 0,
            WaterDepthV1::Wading => 1,
            WaterDepthV1::Shallow => 2,
            WaterDepthV1::Deep => 3,
        },
    })
}

pub fn fishing_habitat_at_v2(
    cells: &[SurfaceCellV1],
    grid: HydrologyGridSpecV2,
    x: usize,
    y: usize,
) -> Option<FishingHabitatQueryV2> {
    let query = water_query_at_v2(cells, grid, x, y)?;
    let flowing = query.flow_x_milli != 0 || query.flow_y_milli != 0;
    let deep = query.depth == WaterDepthV1::Deep;
    let frozen = query.freeze_state == FreezeStateV1::Frozen;
    let mut score = 25_u16;
    if deep {
        score += 30;
    }
    if flowing {
        score += 15;
    }
    if query.water_kind == WaterKindV1::Fresh || query.water_kind == WaterKindV1::River {
        score += 20;
    }
    if frozen {
        score = score.saturating_sub(35);
    }
    Some(FishingHabitatQueryV2 {
        water_body_id: query.water_body_id,
        freshwater: matches!(query.water_kind, WaterKindV1::Fresh | WaterKindV1::River),
        flowing,
        deep,
        frozen,
        habitat_score: score,
    })
}

fn classify_body(
    component: &[usize],
    cells: &[SurfaceCellV1],
    grid: HydrologyGridSpecV2,
) -> WaterBodyClassV2 {
    let ocean = component
        .iter()
        .any(|&index| cells[index].water_kind == WaterKindV1::Ocean);
    if ocean {
        return WaterBodyClassV2::Ocean;
    }
    let river = component
        .iter()
        .any(|&index| cells[index].water_kind == WaterKindV1::River);
    if river {
        return WaterBodyClassV2::River;
    }
    let wet = component
        .iter()
        .filter(|&&index| cells[index].moisture >= 220)
        .count();
    if wet * 2 >= component.len() {
        return WaterBodyClassV2::Wetland;
    }
    if component.len() >= 64
        || component.iter().any(|&index| {
            let (_, y) = xy(index, grid.width);
            y == 0 || y + 1 == grid.height
        })
    {
        WaterBodyClassV2::Lake
    } else {
        WaterBodyClassV2::Pond
    }
}

fn distance_from_land(
    component: &[usize],
    cells: &[SurfaceCellV1],
    grid: HydrologyGridSpecV2,
) -> Vec<u32> {
    let mut distance = vec![u32::MAX; cells.len()];
    let mut queue = VecDeque::new();
    for &index in component {
        // The first shallow-water ring must include diagonal land contacts.
        // A cardinal-only boundary leaves deep-water cells in the four corners
        // around a single painted land tile, which prevents the authored LPC
        // inner-corner cells from ever being selected and produces a plus-shaped
        // grass/sand island. Propagation remains orthogonal after this boundary
        // seed so configured rim widths still count tile steps predictably.
        let boundary = shoreline_neighbors(index, grid)
            .into_iter()
            .any(|neighbor| !cells[neighbor].is_water())
            || (!grid.wrap_east_west && {
                let (x, _) = xy(index, grid.width);
                x == 0 || x + 1 == grid.width
            });
        if boundary {
            distance[index] = 1;
            queue.push_back(index);
        }
    }
    while let Some(index) = queue.pop_front() {
        let next = distance[index].saturating_add(1);
        for neighbor in orthogonal_neighbors(index, grid) {
            if cells[neighbor].is_water() && next < distance[neighbor] {
                distance[neighbor] = next;
                queue.push_back(neighbor);
            }
        }
    }
    distance
}

fn derive_flow(
    index: usize,
    cells: &[SurfaceCellV1],
    grid: HydrologyGridSpecV2,
    class: WaterBodyClassV2,
    max_flow: i16,
) -> (i16, i16) {
    if !matches!(class, WaterBodyClassV2::River) {
        return (0, 0);
    }
    let (x, y) = xy(index, grid.width);
    let mut best = None;
    for neighbor in orthogonal_neighbors(index, grid) {
        if !cells[neighbor].is_water() {
            continue;
        }
        let drop = cells[index].elevation - cells[neighbor].elevation;
        if drop > 0 && best.map(|(best_drop, _)| drop > best_drop).unwrap_or(true) {
            best = Some((drop, neighbor));
        }
    }
    let Some((drop, neighbor)) = best else {
        return (0, 0);
    };
    let (nx, ny) = xy(neighbor, grid.width);
    let magnitude = (i32::from(drop) * 125).clamp(125, i32::from(max_flow.max(125))) as i16;
    let dx = wrapped_delta_x(x, nx, grid.width, grid.wrap_east_west);
    let dy = ny as isize - y as isize;
    (
        dx.signum() as i16 * magnitude,
        dy.signum() as i16 * magnitude,
    )
}

fn derive_freeze_state(
    cell: SurfaceCellV1,
    class: WaterBodyClassV2,
    settings: HydrologySettingsV2,
) -> FreezeStateV1 {
    if cell.temperature_tenths_c <= settings.frozen_tenths_c
        && !matches!(class, WaterBodyClassV2::River | WaterBodyClassV2::Ocean)
        && cell.water_depth != WaterDepthV1::Deep
    {
        FreezeStateV1::Frozen
    } else if cell.temperature_tenths_c <= settings.skim_ice_tenths_c {
        FreezeStateV1::SkimIce
    } else {
        FreezeStateV1::Liquid
    }
}

fn shoreline_neighbors(index: usize, grid: HydrologyGridSpecV2) -> Vec<usize> {
    let (x, y) = xy(index, grid.width);
    let mut result = Vec::with_capacity(8);
    for dy in -1_isize..=1 {
        for dx in -1_isize..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let ny = y as isize + dy;
            if ny < 0 || ny >= grid.height as isize {
                continue;
            }
            let mut nx = x as isize + dx;
            if nx < 0 || nx >= grid.width as isize {
                if !grid.wrap_east_west {
                    continue;
                }
                nx = nx.rem_euclid(grid.width as isize);
            }
            result.push(ny as usize * grid.width + nx as usize);
        }
    }
    result
}

fn orthogonal_neighbors(index: usize, grid: HydrologyGridSpecV2) -> Vec<usize> {
    let (x, y) = xy(index, grid.width);
    let mut result = Vec::with_capacity(4);
    if y > 0 {
        result.push((y - 1) * grid.width + x);
    }
    if y + 1 < grid.height {
        result.push((y + 1) * grid.width + x);
    }
    if x > 0 {
        result.push(y * grid.width + (x - 1));
    } else if grid.wrap_east_west {
        result.push(y * grid.width + (grid.width - 1));
    }
    if x + 1 < grid.width {
        result.push(y * grid.width + (x + 1));
    } else if grid.wrap_east_west {
        result.push(y * grid.width);
    }
    result
}

fn xy(index: usize, width: usize) -> (usize, usize) {
    (index % width, index / width)
}

fn wrapped_delta_x(from: usize, to: usize, width: usize, wrap: bool) -> isize {
    let direct = to as isize - from as isize;
    if !wrap {
        return direct;
    }
    let wrapped_positive = direct + width as isize;
    let wrapped_negative = direct - width as isize;
    [direct, wrapped_positive, wrapped_negative]
        .into_iter()
        .min_by_key(|value| value.abs())
        .unwrap_or(direct)
}

fn stable_body_id(first_index: usize, ordinal: u64) -> u64 {
    let mut value = first_index as u64 ^ ordinal.rotate_left(23) ^ 0x4859_4452_4f32_0001;
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GenerationStageId, SurfaceMaterialV1};

    fn dry() -> SurfaceCellV1 {
        SurfaceCellV1::dry(SurfaceMaterialV1::Grass, 10, GenerationStageId::Hydrology)
    }
    fn water(kind: WaterKindV1) -> SurfaceCellV1 {
        let mut cell = SurfaceCellV1::dry(SurfaceMaterialV1::Sand, 0, GenerationStageId::Hydrology);
        cell.water_kind = kind;
        cell.water_depth = WaterDepthV1::Shallow;
        cell
    }

    #[test]
    fn enclosed_freshwater_gets_deep_interior() {
        let grid = HydrologyGridSpecV2 {
            width: 7,
            height: 7,
            wrap_east_west: false,
        };
        let mut cells = vec![dry(); 49];
        for y in 1..6 {
            for x in 1..6 {
                cells[y * 7 + x] = water(WaterKindV1::Fresh);
            }
        }
        let report =
            resolve_hydrology_v2(&mut cells, grid, HydrologySettingsV2::default()).unwrap();
        assert_eq!(report.water_bodies.len(), 1);
        assert_eq!(cells[3 * 7 + 3].water_depth, WaterDepthV1::Deep);
        assert_eq!(cells[8].water_depth, WaterDepthV1::Shallow);
    }

    #[test]
    fn diagonal_land_contacts_seed_complete_shallow_corner_ring() {
        let grid = HydrologyGridSpecV2 {
            width: 5,
            height: 5,
            wrap_east_west: false,
        };
        let mut cells = vec![water(WaterKindV1::Fresh); 25];
        cells[2 * 5 + 2] = dry();

        resolve_hydrology_v2(&mut cells, grid, HydrologySettingsV2::default()).unwrap();

        for (x, y) in [
            (1, 1),
            (2, 1),
            (3, 1),
            (1, 2),
            (3, 2),
            (1, 3),
            (2, 3),
            (3, 3),
        ] {
            assert_eq!(
                cells[y * 5 + x].water_depth,
                WaterDepthV1::Shallow,
                "water at {x},{y} should participate in the complete LPC shore ring"
            );
        }
    }

    #[test]
    fn east_west_wrap_connects_same_body() {
        let grid = HydrologyGridSpecV2 {
            width: 5,
            height: 3,
            wrap_east_west: true,
        };
        let mut cells = vec![dry(); 15];
        cells[5] = water(WaterKindV1::Ocean);
        cells[9] = water(WaterKindV1::Ocean);
        let report =
            resolve_hydrology_v2(&mut cells, grid, HydrologySettingsV2::default()).unwrap();
        assert_eq!(report.water_bodies.len(), 1);
        assert_eq!(cells[5].water_body_id, cells[9].water_body_id);
    }

    #[test]
    fn river_flows_downhill() {
        let grid = HydrologyGridSpecV2 {
            width: 3,
            height: 3,
            wrap_east_west: false,
        };
        let mut cells = vec![dry(); 9];
        cells[3] = water(WaterKindV1::River);
        cells[3].elevation = 3;
        cells[4] = water(WaterKindV1::River);
        cells[4].elevation = 2;
        cells[5] = water(WaterKindV1::River);
        cells[5].elevation = 1;
        resolve_hydrology_v2(&mut cells, grid, HydrologySettingsV2::default()).unwrap();
        assert!(cells[3].flow_x_milli > 0);
        assert!(cells[4].flow_x_milli > 0);
    }
}
