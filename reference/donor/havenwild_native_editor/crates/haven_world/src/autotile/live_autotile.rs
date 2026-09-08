use haven_core::{
    AutotileOverride, ProjectSceneId, SceneMap, TileAutoGroup, TileKind, MAP_H, MAP_W,
};

use super::{resolve_terrain_transitions, ResolvedTerrainTransitions, E, N, NE, NW, S, SE, SW, W};
use crate::water_render_mask::{resolve_water_render_mask, WaterRenderMask};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutotileSource {
    Automatic,
    ManualOverride,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AutotileShape {
    Isolated,
    EndNorth,
    EndEast,
    EndSouth,
    EndWest,
    StraightVertical,
    StraightHorizontal,
    CornerNorthEast,
    CornerSouthEast,
    CornerSouthWest,
    CornerNorthWest,
    TeeNorthEastSouth,
    TeeEastSouthWest,
    TeeSouthWestNorth,
    TeeWestNorthEast,
    Cross,
}

impl AutotileShape {
    pub fn from_mask(mask: u8) -> Self {
        match mask & 0x0f {
            0 => Self::Isolated,
            1 => Self::EndNorth,
            2 => Self::EndEast,
            4 => Self::EndSouth,
            8 => Self::EndWest,
            5 => Self::StraightVertical,
            10 => Self::StraightHorizontal,
            3 => Self::CornerNorthEast,
            6 => Self::CornerSouthEast,
            12 => Self::CornerSouthWest,
            9 => Self::CornerNorthWest,
            7 => Self::TeeNorthEastSouth,
            14 => Self::TeeEastSouthWest,
            13 => Self::TeeSouthWestNorth,
            11 => Self::TeeWestNorthEast,
            _ => Self::Cross,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Isolated => "isolated",
            Self::EndNorth => "end north",
            Self::EndEast => "end east",
            Self::EndSouth => "end south",
            Self::EndWest => "end west",
            Self::StraightVertical => "vertical",
            Self::StraightHorizontal => "horizontal",
            Self::CornerNorthEast => "corner NE",
            Self::CornerSouthEast => "corner SE",
            Self::CornerSouthWest => "corner SW",
            Self::CornerNorthWest => "corner NW",
            Self::TeeNorthEastSouth => "tee NES",
            Self::TeeEastSouthWest => "tee ESW",
            Self::TeeSouthWestNorth => "tee SWN",
            Self::TeeWestNorthEast => "tee WNE",
            Self::Cross => "cross",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedAutotileCell {
    pub x: i32,
    pub y: i32,
    pub tile: TileKind,
    pub group: TileAutoGroup,
    pub mask: u8,
    pub shape: AutotileShape,
    pub source: AutotileSource,
    pub transitions: ResolvedTerrainTransitions,
    pub water_mask: WaterRenderMask,
}

impl ResolvedAutotileCell {
    pub fn manual_override(&self) -> bool {
        self.source == AutotileSource::ManualOverride
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AutotileSyncReport {
    pub changed_source_cells: usize,
    pub recalculated_cells: usize,
    pub resolved_cells: usize,
    pub manual_override_cells: usize,
    pub water_mask_cells: usize,
}

#[derive(Clone, Debug)]
pub struct LiveAutotileCache {
    scene_id: ProjectSceneId,
    observed_tiles: Vec<TileKind>,
    observed_overrides: Vec<Option<AutotileOverride>>,
    resolved: Vec<Option<ResolvedAutotileCell>>,
    dirty: Vec<bool>,
    last_dirty_cells: Vec<(i32, i32)>,
    last_report: AutotileSyncReport,
    revision: u64,
}

impl LiveAutotileCache {
    pub fn new(scene: &SceneMap) -> Self {
        let count = MAP_W * MAP_H;
        Self {
            scene_id: scene.id.clone(),
            observed_tiles: vec![TileKind::Wall; count],
            observed_overrides: vec![None; count],
            resolved: vec![None; count],
            dirty: vec![true; count],
            last_dirty_cells: Vec::new(),
            last_report: AutotileSyncReport::default(),
            revision: 0,
        }
    }

    pub fn scene_id(&self) -> &ProjectSceneId {
        &self.scene_id
    }

    pub fn resolved_at(&self, x: i32, y: i32) -> Option<&ResolvedAutotileCell> {
        index(x, y).and_then(|slot| self.resolved.get(slot)?.as_ref())
    }

    pub fn last_dirty_cells(&self) -> &[(i32, i32)] {
        &self.last_dirty_cells
    }

    pub fn last_report(&self) -> AutotileSyncReport {
        self.last_report
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn mark_all_dirty(&mut self) {
        self.dirty.fill(true);
    }

    pub fn mark_dirty_cell(&mut self, x: i32, y: i32) {
        for ny in y - 1..=y + 1 {
            for nx in x - 1..=x + 1 {
                if let Some(slot) = index(nx, ny) {
                    self.dirty[slot] = true;
                }
            }
        }
    }

    pub fn synchronize(&mut self, scene: &SceneMap) -> AutotileSyncReport {
        if self.scene_id != scene.id {
            *self = Self::new(scene);
        }

        let override_lookup = override_lookup(scene);
        let mut changed_source_cells = 0;
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                let slot = index(x, y).expect("map coordinate");
                let tile = scene.map.get(x, y);
                let override_value = override_lookup[slot];
                if self.observed_tiles[slot] != tile
                    || self.observed_overrides[slot] != override_value
                {
                    self.observed_tiles[slot] = tile;
                    self.observed_overrides[slot] = override_value;
                    self.mark_dirty_cell(x, y);
                    changed_source_cells += 1;
                }
            }
        }

        self.last_dirty_cells.clear();
        let mut recalculated_cells = 0;
        for y in 0..MAP_H as i32 {
            for x in 0..MAP_W as i32 {
                let slot = index(x, y).expect("map coordinate");
                if !self.dirty[slot] {
                    continue;
                }
                self.dirty[slot] = false;
                self.last_dirty_cells.push((x, y));
                self.resolved[slot] = resolve_autotile_cell(scene, x, y);
                recalculated_cells += 1;
            }
        }

        let resolved_cells = self.resolved.iter().flatten().count();
        let manual_override_cells = self
            .resolved
            .iter()
            .flatten()
            .filter(|cell| cell.manual_override())
            .count();
        let water_mask_cells = self
            .resolved
            .iter()
            .flatten()
            .filter(|cell| {
                cell.water_mask.shoreline_edges != 0 || cell.water_mask.has_depth_transition()
            })
            .count();
        // A polling synchronization with no dirty cells must not invalidate
        // every retained terrain representation. Only semantic or override
        // changes advance the presentation revision.
        if recalculated_cells > 0 {
            self.revision = self.revision.wrapping_add(1);
        }
        self.last_report = AutotileSyncReport {
            changed_source_cells,
            recalculated_cells,
            resolved_cells,
            manual_override_cells,
            water_mask_cells,
        };
        self.last_report
    }
}

pub fn resolve_autotile_cell(scene: &SceneMap, x: i32, y: i32) -> Option<ResolvedAutotileCell> {
    let tile = scene.map.get(x, y);
    let group = tile.autotile_group()?;
    let automatic_mask = adjacency_mask(scene, x, y, group);
    let compatible_override = scene
        .autotile_override_at(x, y)
        .filter(|entry| entry.group == group);
    let (mask, source) = compatible_override
        .map(|entry| (normalize_mask(entry.mask), AutotileSource::ManualOverride))
        .unwrap_or((automatic_mask, AutotileSource::Automatic));
    Some(ResolvedAutotileCell {
        x,
        y,
        tile,
        group,
        mask,
        shape: AutotileShape::from_mask(mask),
        source,
        transitions: resolve_terrain_transitions(&scene.map, x, y),
        water_mask: resolve_water_render_mask(&scene.map, x, y),
    })
}

pub fn adjacency_mask(scene: &SceneMap, x: i32, y: i32, group: TileAutoGroup) -> u8 {
    let mut mask = 0;
    let north = same_group(scene, x, y - 1, group);
    let east = same_group(scene, x + 1, y, group);
    let south = same_group(scene, x, y + 1, group);
    let west = same_group(scene, x - 1, y, group);
    if north {
        mask |= N;
    }
    if east {
        mask |= E;
    }
    if south {
        mask |= S;
    }
    if west {
        mask |= W;
    }
    if north && east && same_group(scene, x + 1, y - 1, group) {
        mask |= NE;
    }
    if south && east && same_group(scene, x + 1, y + 1, group) {
        mask |= SE;
    }
    if south && west && same_group(scene, x - 1, y + 1, group) {
        mask |= SW;
    }
    if north && west && same_group(scene, x - 1, y - 1, group) {
        mask |= NW;
    }
    mask
}

pub fn normalize_mask(mut mask: u8) -> u8 {
    if mask & N == 0 || mask & E == 0 {
        mask &= !NE;
    }
    if mask & S == 0 || mask & E == 0 {
        mask &= !SE;
    }
    if mask & S == 0 || mask & W == 0 {
        mask &= !SW;
    }
    if mask & N == 0 || mask & W == 0 {
        mask &= !NW;
    }
    mask
}

fn same_group(scene: &SceneMap, x: i32, y: i32, group: TileAutoGroup) -> bool {
    index(x, y).is_some() && scene.map.get(x, y).autotile_group() == Some(group)
}

fn override_lookup(scene: &SceneMap) -> Vec<Option<AutotileOverride>> {
    let mut result = vec![None; MAP_W * MAP_H];
    for entry in &scene.autotile_overrides {
        if let Some(slot) = index(entry.x, entry.y) {
            result[slot] = Some(*entry);
        }
    }
    result
}

fn index(x: i32, y: i32) -> Option<usize> {
    if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
        return None;
    }
    Some(y as usize * MAP_W + x as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::{SceneBiome, SceneKind};

    fn blank_scene() -> SceneMap {
        SceneMap::blank(
            "autotile_test",
            "Autotile Test",
            SceneKind::Exterior,
            SceneBiome::Temperate,
        )
    }

    #[test]
    fn road_neighbors_resolve_horizontal_shape() {
        let mut scene = blank_scene();
        scene.map.set(3, 3, TileKind::Road);
        scene.map.set(2, 3, TileKind::Road);
        scene.map.set(4, 3, TileKind::Road);
        let resolved = resolve_autotile_cell(&scene, 3, 3).expect("road autotile");
        assert_eq!(resolved.mask & (N | E | S | W), E | W);
        assert_eq!(resolved.shape, AutotileShape::StraightHorizontal);
    }

    #[test]
    fn dirty_sync_recalculates_only_neighbor_region_after_first_pass() {
        let mut scene = blank_scene();
        let mut cache = LiveAutotileCache::new(&scene);
        let first = cache.synchronize(&scene);
        assert_eq!(first.recalculated_cells, MAP_W * MAP_H);
        scene.map.set(8, 8, TileKind::Road);
        let second = cache.synchronize(&scene);
        assert_eq!(second.changed_source_cells, 1);
        assert_eq!(second.recalculated_cells, 9);
    }

    #[test]
    fn synchronized_cells_cache_v7_water_masks() {
        let mut scene = blank_scene();
        scene.map.set(4, 4, TileKind::DeepWater);
        scene.map.set(4, 3, TileKind::ShallowWater);
        let mut cache = LiveAutotileCache::new(&scene);
        let report = cache.synchronize(&scene);
        let cell = cache.resolved_at(4, 4).expect("deep-water autotile");
        assert_eq!(
            cell.water_mask.depth_edges,
            crate::water_render_mask::WATER_MASK_NORTH
        );
        assert!(report.water_mask_cells >= 1);

        let second = cache.synchronize(&scene);
        assert_eq!(second.recalculated_cells, 0);
        assert_eq!(second.water_mask_cells, report.water_mask_cells);
    }

    #[test]
    fn manual_override_wins_over_automatic_mask() {
        let mut scene = blank_scene();
        scene.map.set(5, 5, TileKind::Road);
        scene.map.set(6, 5, TileKind::Road);
        scene.set_autotile_override(AutotileOverride::new(5, 5, TileAutoGroup::Road, N | S));
        let resolved = resolve_autotile_cell(&scene, 5, 5).expect("road autotile");
        assert_eq!(resolved.mask, N | S);
        assert_eq!(resolved.source, AutotileSource::ManualOverride);
    }
}

#[cfg(test)]
mod revision_contract_tests {
    use super::*;
    use haven_core::{SceneBiome, SceneKind, SceneMap};

    #[test]
    fn idle_polling_does_not_advance_terrain_revision() {
        let scene = SceneMap::blank(
            "revision-test",
            "Revision Test",
            SceneKind::Exterior,
            SceneBiome::Temperate,
        );
        let mut cache = LiveAutotileCache::new(&scene);
        cache.synchronize(&scene);
        let revision = cache.revision();
        let report = cache.synchronize(&scene);
        assert_eq!(report.recalculated_cells, 0);
        assert_eq!(cache.revision(), revision);
    }
}
