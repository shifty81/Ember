//! Deterministic mainland feature materialization for the continuous PCG surface.
//!
//! Roads, Willowmere civic reservations, rocky resource scatter, and cave
//! entrance hosts are generated in global surface coordinates. Exterior scene
//! rectangles remain storage partitions only; every feature may cross their
//! boundaries without creating scene-transition seams.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use haven_core::{ObjectKind, ProjectSceneId, SceneMap, TileKind, ZoneKind, MAP_H, MAP_W};

use crate::open_world::ChunkCoord;

mod alderreach_layout;

pub const MAINLAND_FEATURE_AUTHORITY_SCHEMA: &str = "havenwild.mainland_surface_features.v0_2";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MainlandFeatureReport {
    pub road_tiles: usize,
    pub road_partitions: usize,
    /// Legacy Z83 field retained for log/API compatibility. Z84 no longer
    /// paints city reservations into the terrain.
    pub city_foundation_tiles: usize,
    pub city_plot_reservations: usize,
    pub city_reserved_tiles: usize,
    pub harbor_reserved_tiles: usize,
    pub civic_objects: usize,
    pub cave_entrances: usize,
    pub repaired_legacy_tiles: usize,
    pub removed_legacy_objects: usize,
}

impl MainlandFeatureReport {
    pub const fn total_mutations(self) -> usize {
        self.road_tiles
            + self.city_foundation_tiles
            + self.city_reserved_tiles
            + self.harbor_reserved_tiles
            + self.civic_objects
            + self.cave_entrances
            + self.repaired_legacy_tiles
            + self.removed_legacy_objects
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SurfaceCell {
    x: i32,
    y: i32,
}

impl SurfaceCell {
    const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

struct SurfaceAssembly<'a> {
    scenes: &'a mut [SceneMap],
    scene_by_chunk: BTreeMap<ChunkCoord, usize>,
    min_x: i32,
    min_y: i32,
    max_x: i32,
    max_y: i32,
    touched_road_partitions: BTreeSet<usize>,
}

impl<'a> SurfaceAssembly<'a> {
    fn new(scenes: &'a mut [SceneMap], chunks: &[ChunkCoord]) -> Result<Self, String> {
        if scenes.len() != chunks.len() || scenes.is_empty() {
            return Err(format!(
                "mainland feature pass requires matching non-empty scene/chunk arrays ({} scenes, {} chunks)",
                scenes.len(),
                chunks.len()
            ));
        }
        let mut scene_by_chunk = BTreeMap::new();
        for (index, chunk) in chunks.iter().copied().enumerate() {
            if scene_by_chunk.insert(chunk, index).is_some() {
                return Err(format!(
                    "mainland feature pass received duplicate chunk {},{}",
                    chunk.x, chunk.y
                ));
            }
        }
        let min_chunk_x = chunks.iter().map(|chunk| chunk.x).min().unwrap_or(0);
        let min_chunk_y = chunks.iter().map(|chunk| chunk.y).min().unwrap_or(0);
        let max_chunk_x = chunks.iter().map(|chunk| chunk.x).max().unwrap_or(0);
        let max_chunk_y = chunks.iter().map(|chunk| chunk.y).max().unwrap_or(0);
        Ok(Self {
            scenes,
            scene_by_chunk,
            min_x: min_chunk_x * MAP_W as i32,
            min_y: min_chunk_y * MAP_H as i32,
            max_x: (max_chunk_x + 1) * MAP_W as i32 - 1,
            max_y: (max_chunk_y + 1) * MAP_H as i32 - 1,
            touched_road_partitions: BTreeSet::new(),
        })
    }

    fn local_cell(&self, cell: SurfaceCell) -> Option<(usize, i32, i32)> {
        let chunk = ChunkCoord::new(
            cell.x.div_euclid(MAP_W as i32),
            cell.y.div_euclid(MAP_H as i32),
        );
        let scene_index = *self.scene_by_chunk.get(&chunk)?;
        Some((
            scene_index,
            cell.x.rem_euclid(MAP_W as i32),
            cell.y.rem_euclid(MAP_H as i32),
        ))
    }

    fn tile(&self, cell: SurfaceCell) -> Option<TileKind> {
        let (scene_index, x, y) = self.local_cell(cell)?;
        Some(self.scenes[scene_index].map.get(x, y))
    }

    fn height(&self, cell: SurfaceCell) -> Option<u8> {
        let (scene_index, x, y) = self.local_cell(cell)?;
        Some(self.scenes[scene_index].map.get_height(x, y))
    }

    fn set_tile(&mut self, cell: SurfaceCell, tile: TileKind) -> bool {
        let Some((scene_index, x, y)) = self.local_cell(cell) else {
            return false;
        };
        if self.scenes[scene_index].map.get(x, y) == tile {
            return false;
        }
        let removable_ids = self.scenes[scene_index]
            .map
            .objects
            .iter()
            .filter(|object| object.contains_tile(x, y) && is_natural_surface_object(object.kind))
            .map(|object| object.id)
            .collect::<Vec<_>>();
        for object_id in removable_ids {
            let _ = self.scenes[scene_index].map.remove_object(object_id);
        }
        let previous = self.scenes[scene_index].map.get(x, y);
        self.scenes[scene_index].map.set(x, y, tile);
        if tile == TileKind::Road {
            self.scenes[scene_index].set_zone(x, y, ZoneKind::PublicPath);
            self.touched_road_partitions.insert(scene_index);
        } else if previous == TileKind::Road
            && self.scenes[scene_index].zone_at(x, y) == ZoneKind::PublicPath
        {
            self.scenes[scene_index].set_zone(x, y, ZoneKind::None);
        }
        true
    }

    fn set_zone(&mut self, cell: SurfaceCell, zone: ZoneKind) -> bool {
        let Some((scene_index, x, y)) = self.local_cell(cell) else {
            return false;
        };
        if self.scenes[scene_index].zone_at(x, y) == zone {
            return false;
        }
        self.scenes[scene_index].set_zone(x, y, zone);
        true
    }

    fn remove_object_at(&mut self, cell: SurfaceCell, kind: ObjectKind) -> bool {
        let Some((scene_index, x, y)) = self.local_cell(cell) else {
            return false;
        };
        let object_id = self.scenes[scene_index]
            .map
            .objects
            .iter()
            .find(|object| object.x == x && object.y == y && object.kind == kind)
            .map(|object| object.id);
        object_id.is_some_and(|id| self.scenes[scene_index].map.remove_object(id).is_some())
    }

    fn clear_natural_objects_at(&mut self, cell: SurfaceCell) -> usize {
        let Some((scene_index, x, y)) = self.local_cell(cell) else {
            return 0;
        };
        let ids = self.scenes[scene_index]
            .map
            .objects
            .iter()
            .filter(|object| object.contains_tile(x, y) && is_natural_surface_object(object.kind))
            .map(|object| object.id)
            .collect::<Vec<_>>();
        let mut removed = 0;
        for id in ids {
            removed += usize::from(self.scenes[scene_index].map.remove_object(id).is_some());
        }
        removed
    }

    fn place_object(&mut self, cell: SurfaceCell, kind: ObjectKind) -> bool {
        let Some((scene_index, x, y)) = self.local_cell(cell) else {
            return false;
        };
        self.scenes[scene_index]
            .map
            .place_object(kind, x, y)
            .is_some()
    }

    fn mark_cave_zone(&mut self, top_left: SurfaceCell) {
        for y in 0..4 {
            for x in 0..4 {
                let cell = SurfaceCell::new(top_left.x + x, top_left.y + y);
                let Some((scene_index, local_x, local_y)) = self.local_cell(cell) else {
                    continue;
                };
                self.scenes[scene_index].set_zone(local_x, local_y, ZoneKind::Cave);
            }
        }
    }

    fn has_protected_placement(&self, cell: SurfaceCell) -> bool {
        let Some((scene_index, x, y)) = self.local_cell(cell) else {
            return true;
        };
        let map = &self.scenes[scene_index].map;
        if map.blocking_stamp_at(x, y).is_some() {
            return true;
        }
        map.blocking_object_at(x, y).is_some_and(|index| {
            map.objects
                .get(index)
                .is_some_and(|object| !is_natural_surface_object(object.kind))
        })
    }

    fn is_road_land(&self, cell: SurfaceCell) -> bool {
        !self.has_protected_placement(cell)
            && self.tile(cell).is_some_and(|tile| {
                !tile.is_water()
                    && !matches!(
                        tile,
                        TileKind::Wall | TileKind::CaveWall | TileKind::Cliff | TileKind::Bridge
                    )
            })
    }

    fn is_city_land(&self, cell: SurfaceCell) -> bool {
        !self.has_protected_placement(cell)
            && self.tile(cell).is_some_and(|tile| {
                matches!(
                    tile,
                    TileKind::Grass
                        | TileKind::Dirt
                        | TileKind::PebbleShore
                        | TileKind::Road
                        | TileKind::StonePath
                        | TileKind::MountainPath
                        | TileKind::MudBank
                        | TileKind::Sand
                )
            })
    }

    fn nearest_road_land(&self, target: SurfaceCell, maximum_radius: i32) -> Option<SurfaceCell> {
        if self.is_road_land(target) {
            return Some(target);
        }
        for radius in 1..=maximum_radius {
            for x in target.x - radius..=target.x + radius {
                for y in [target.y - radius, target.y + radius] {
                    let cell = SurfaceCell::new(x, y);
                    if self.is_road_land(cell) {
                        return Some(cell);
                    }
                }
            }
            for y in target.y - radius + 1..target.y + radius {
                for x in [target.x - radius, target.x + radius] {
                    let cell = SurfaceCell::new(x, y);
                    if self.is_road_land(cell) {
                        return Some(cell);
                    }
                }
            }
        }
        None
    }

    fn find_town_center(&self, harbor: SurfaceCell) -> Option<SurfaceCell> {
        let target = SurfaceCell::new(
            harbor.x,
            (harbor.y - MAP_H as i32 * 2).clamp(self.min_y + 28, self.max_y - 28),
        );
        let mut best: Option<(i32, SurfaceCell)> = None;
        for radius in (0..=96).step_by(4) {
            for y in target.y - radius..=target.y + radius {
                for x in target.x - radius..=target.x + radius {
                    if radius > 0
                        && x != target.x - radius
                        && x != target.x + radius
                        && y != target.y - radius
                        && y != target.y + radius
                    {
                        continue;
                    }
                    let candidate = SurfaceCell::new(x, y);
                    let mut buildable = 0;
                    let mut inspected = 0;
                    for oy in (-18..=18).step_by(3) {
                        for ox in (-24..=24).step_by(3) {
                            inspected += 1;
                            if self.is_city_land(SurfaceCell::new(x + ox, y + oy)) {
                                buildable += 1;
                            }
                        }
                    }
                    if buildable * 100 < inspected * 78 {
                        continue;
                    }
                    let elevation_penalty = i32::from(self.height(candidate).unwrap_or(255));
                    let target_distance = (x - target.x).abs() + (y - target.y).abs();
                    let score = target_distance + elevation_penalty / 8;
                    if best.is_none_or(|(best_score, _)| score < best_score) {
                        best = Some((score, candidate));
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map(|(_, cell)| cell)
    }

    fn shortest_land_path(
        &self,
        start: SurfaceCell,
        target: SurfaceCell,
    ) -> Option<Vec<SurfaceCell>> {
        let width = usize::try_from(self.max_x - self.min_x + 1).ok()?;
        let height = usize::try_from(self.max_y - self.min_y + 1).ok()?;
        let cell_count = width.checked_mul(height)?;
        let index = |cell: SurfaceCell| -> Option<usize> {
            if cell.x < self.min_x
                || cell.y < self.min_y
                || cell.x > self.max_x
                || cell.y > self.max_y
            {
                return None;
            }
            let x = usize::try_from(cell.x - self.min_x).ok()?;
            let y = usize::try_from(cell.y - self.min_y).ok()?;
            y.checked_mul(width)?.checked_add(x)
        };
        let start_index = index(start)?;
        let target_index = index(target)?;
        let mut previous = vec![usize::MAX; cell_count];
        let mut queue = VecDeque::new();
        previous[start_index] = start_index;
        queue.push_back(start);

        while let Some(current) = queue.pop_front() {
            let current_index = index(current)?;
            if current_index == target_index {
                break;
            }
            for next in [
                SurfaceCell::new(current.x + 1, current.y),
                SurfaceCell::new(current.x - 1, current.y),
                SurfaceCell::new(current.x, current.y + 1),
                SurfaceCell::new(current.x, current.y - 1),
            ] {
                let Some(next_index) = index(next) else {
                    continue;
                };
                if previous[next_index] != usize::MAX || !self.is_road_land(next) {
                    continue;
                }
                previous[next_index] = current_index;
                queue.push_back(next);
            }
        }
        if previous[target_index] == usize::MAX {
            return None;
        }

        let mut path = Vec::new();
        let mut cursor = target_index;
        loop {
            let x = (cursor % width) as i32 + self.min_x;
            let y = (cursor / width) as i32 + self.min_y;
            path.push(SurfaceCell::new(x, y));
            if cursor == start_index {
                break;
            }
            cursor = previous[cursor];
        }
        path.reverse();
        Some(path)
    }

    fn paint_road_path(&mut self, path: &[SurfaceCell], radius: i32) -> usize {
        let mut changed = 0;
        for cell in path {
            for oy in -radius..=radius {
                for ox in -radius..=radius {
                    if ox.abs() + oy.abs() > radius + 1 {
                        continue;
                    }
                    let target = SurfaceCell::new(cell.x + ox, cell.y + oy);
                    if self.is_road_land(target) && self.set_tile(target, TileKind::Road) {
                        changed += 1;
                    }
                }
            }
        }
        changed
    }

    fn paint_city_streets(&mut self, center: SurfaceCell) -> usize {
        let mut changed = 0;
        let mut street_cells = Vec::new();
        for x in center.x - 38..=center.x + 38 {
            for y in center.y - 1..=center.y + 1 {
                street_cells.push(SurfaceCell::new(x, y));
            }
        }
        for y in center.y - 29..=center.y + 29 {
            for x in center.x - 1..=center.x + 1 {
                street_cells.push(SurfaceCell::new(x, y));
            }
        }
        for x in center.x - 34..=center.x + 34 {
            for y in [center.y - 25, center.y + 25] {
                street_cells.push(SurfaceCell::new(x, y));
            }
        }
        for y in center.y - 25..=center.y + 25 {
            for x in [center.x - 34, center.x + 34] {
                street_cells.push(SurfaceCell::new(x, y));
            }
        }
        street_cells.sort_unstable();
        street_cells.dedup();
        for cell in street_cells {
            if self.is_road_land(cell) && self.set_tile(cell, TileKind::Road) {
                changed += 1;
            }
        }
        changed
    }

    fn find_cave_host(&self, town: SurfaceCell) -> Option<SurfaceCell> {
        let mut best: Option<(i32, SurfaceCell)> = None;
        for y in self.min_y..=self.max_y - 4 {
            for x in self.min_x..=self.max_x - 4 {
                let top_left = SurfaceCell::new(x, y);
                let mut rock = 0;
                let mut minimum_height = u8::MAX;
                for oy in 0..4 {
                    for ox in 0..4 {
                        let cell = SurfaceCell::new(x + ox, y + oy);
                        if self.tile(cell) == Some(TileKind::MountainRock) {
                            rock += 1;
                        }
                        minimum_height = minimum_height.min(self.height(cell).unwrap_or(0));
                    }
                }
                if rock < 13 || minimum_height < 158 {
                    continue;
                }
                let distance = (x - town.x).abs() + (y - town.y).abs();
                if distance < 54 {
                    continue;
                }
                let north_preference = y - self.min_y;
                let score = north_preference + distance / 8;
                if best.is_none_or(|(best_score, _)| score < best_score) {
                    best = Some((score, top_left));
                }
            }
        }
        best.map(|(_, cell)| cell)
    }
}

fn is_natural_surface_object(kind: ObjectKind) -> bool {
    matches!(
        kind,
        ObjectKind::Tree
            | ObjectKind::Bush
            | ObjectKind::Boulder
            | ObjectKind::OreNode
            | ObjectKind::Mushroom
            | ObjectKind::Herb
            | ObjectKind::Stump
            | ObjectKind::Log
    )
}

/// Applies the deterministic Willowmere/mainland surface pass.
///
/// `preserve_existing_infrastructure` leaves already materialized road and
/// civic layouts intact while still allowing missing cave anchors to be added.
pub fn apply_mainland_surface_features(
    scenes: &mut [SceneMap],
    chunks: &[ChunkCoord],
    harbor_scene_id: Option<&ProjectSceneId>,
    _seed: u64,
    preserve_existing_infrastructure: bool,
) -> Result<MainlandFeatureReport, String> {
    let mut surface = SurfaceAssembly::new(scenes, chunks)?;
    let harbor_scene_index = harbor_scene_id
        .and_then(|id| surface.scenes.iter().position(|scene| &scene.id == id))
        .unwrap_or(0);
    let harbor_chunk = chunks[harbor_scene_index];
    let harbor_scene = &surface.scenes[harbor_scene_index];
    let preferred_harbor = SurfaceCell::new(
        harbor_chunk.x * MAP_W as i32 + harbor_scene.spawn_x,
        harbor_chunk.y * MAP_H as i32 + harbor_scene.spawn_y,
    );
    let harbor = surface
        .find_harbor_landfall(preferred_harbor)
        .or_else(|| surface.nearest_road_land(preferred_harbor, 128))
        .ok_or_else(|| {
            "mainland feature pass could not locate a dry harbor landfall".to_string()
        })?;
    let existing_center = preserve_existing_infrastructure
        .then(|| surface.existing_willowmere_center())
        .flatten();
    let town = existing_center
        .or_else(|| surface.find_town_center(harbor))
        .or_else(|| surface.nearest_road_land(harbor, 128))
        .ok_or_else(|| "mainland feature pass could not locate Willowmere site".to_string())?;

    let existing_road_tiles = surface
        .scenes
        .iter()
        .map(|scene| {
            scene
                .map
                .tiles
                .iter()
                .filter(|tile| **tile == TileKind::Road)
                .count()
        })
        .sum::<usize>();
    // A single player-painted road must not suppress Willowmere generation on
    // a pre-Z83 save. The Z83 capital is identified by its former center object
    // or by the substantial road footprint produced by the capital/route pass.
    let has_existing_generated_infrastructure =
        existing_center.is_some() || existing_road_tiles >= 256;
    let build_infrastructure =
        !(preserve_existing_infrastructure && has_existing_generated_infrastructure);
    let mut report = MainlandFeatureReport::default();

    let plot_specs = [
        (-23, -14, 7, 5, ZoneKind::CivicLot),
        (23, -14, 7, 5, ZoneKind::MarketLot),
        (-23, 14, 7, 5, ZoneKind::ResidentialLot),
        (23, 14, 7, 5, ZoneKind::ArtisanLot),
        (-10, -14, 5, 5, ZoneKind::CivicLot),
        (10, -14, 5, 5, ZoneKind::MarketLot),
        (-10, 14, 5, 5, ZoneKind::ResidentialLot),
        (10, 14, 5, 5, ZoneKind::ArtisanLot),
    ];

    if preserve_existing_infrastructure {
        report.road_tiles += surface.paint_civic_plaza_apron(town);
        for (offset_x, offset_y, half_w, half_h, _) in plot_specs {
            report.repaired_legacy_tiles += surface.clear_legacy_plot_foundation(
                SurfaceCell::new(town.x + offset_x, town.y + offset_y),
                half_w,
                half_h,
            );
        }
        report.removed_legacy_objects +=
            usize::from(surface.remove_object_at(town, ObjectKind::Well));
        report.repaired_legacy_tiles += surface.repair_harbor_road_intrusions(harbor);
    }

    if build_infrastructure {
        if let Some(path) = surface.shortest_land_path(harbor, town) {
            report.road_tiles += surface.paint_road_path(&path, 1);
        }
        report.road_tiles += surface.paint_city_streets(town);
        report.city_foundation_tiles += surface.paint_civic_plaza(town);
        report.road_tiles += surface.paint_civic_plaza_apron(town);

        let edge_targets = [
            SurfaceCell::new(town.x, surface.min_y + 8),
            SurfaceCell::new(surface.min_x + 8, town.y),
            SurfaceCell::new(surface.max_x - 8, town.y),
        ];
        for target in edge_targets {
            let Some(target) = surface.nearest_road_land(target, 64) else {
                continue;
            };
            if let Some(path) = surface.shortest_land_path(town, target) {
                report.road_tiles += surface.paint_road_path(&path, 1);
            }
        }

        for (offset_x, offset_y, kind) in [
            (-4, -4, ObjectKind::Bench),
            (4, 4, ObjectKind::Bench),
            (-8, 0, ObjectKind::Lamp),
            (8, 0, ObjectKind::Lamp),
            (0, -8, ObjectKind::Lamp),
            (0, 8, ObjectKind::Lamp),
            (0, 12, ObjectKind::Sign),
        ] {
            if surface.place_object(SurfaceCell::new(town.x + offset_x, town.y + offset_y), kind) {
                report.civic_objects += 1;
            }
        }
    }

    for (offset_x, offset_y, half_w, half_h, zone) in plot_specs {
        let (reserved, removed) = surface.reserve_city_plot(
            SurfaceCell::new(town.x + offset_x, town.y + offset_y),
            half_w,
            half_h,
            zone,
        );
        if reserved > 0 {
            report.city_plot_reservations += 1;
            report.city_reserved_tiles += reserved;
            report.removed_legacy_objects += removed;
        }
    }
    let (harbor_reserved, harbor_removed) = surface.reserve_harbor_district(harbor);
    report.harbor_reserved_tiles += harbor_reserved;
    report.removed_legacy_objects += harbor_removed;

    let farm_center = SurfaceCell::new(town.x, town.y + 46);
    let (farm_reserved, farm_removed) =
        surface.reserve_city_plot(farm_center, 28, 12, ZoneKind::AgriculturalLot);
    report.city_reserved_tiles += farm_reserved;
    report.removed_legacy_objects += farm_removed;

    let existing_cave = surface.scenes.iter().any(|scene| {
        scene
            .map
            .objects
            .iter()
            .any(|object| object.kind == ObjectKind::CaveEntrance)
    });
    if !existing_cave {
        if let Some(cave) = surface.find_cave_host(town) {
            if surface.place_object(cave, ObjectKind::CaveEntrance) {
                surface.mark_cave_zone(cave);
                report.cave_entrances = 1;
                let approach = SurfaceCell::new(cave.x + 1, cave.y + 4);
                if build_infrastructure {
                    if let Some(approach) = surface.nearest_road_land(approach, 18) {
                        if let Some(path) = surface.shortest_land_path(town, approach) {
                            report.road_tiles += surface.paint_road_path(&path, 1);
                        }
                    }
                }
            }
        }
    }

    report.road_partitions = surface.touched_road_partitions.len();
    Ok(report)
}

#[cfg(test)]
mod tests;
