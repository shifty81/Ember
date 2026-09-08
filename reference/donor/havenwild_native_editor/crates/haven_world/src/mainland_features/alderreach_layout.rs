//! Alderreach capital and harbor layout helpers for the mainland surface pass.

use haven_core::{ObjectKind, TileKind, ZoneKind, MAP_H, MAP_W};

use super::{SurfaceAssembly, SurfaceCell};

impl SurfaceAssembly<'_> {
    pub(super) fn neighboring_water_count(&self, cell: SurfaceCell) -> usize {
        let mut count = 0;
        for oy in -1..=1 {
            for ox in -1..=1 {
                if ox == 0 && oy == 0 {
                    continue;
                }
                count += usize::from(
                    self.tile(SurfaceCell::new(cell.x + ox, cell.y + oy))
                        .is_some_and(TileKind::is_water),
                );
            }
        }
        count
    }

    pub(super) fn is_coastal_land(&self, cell: SurfaceCell) -> bool {
        self.is_road_land(cell) && self.neighboring_water_count(cell) > 0
    }

    pub(super) fn find_harbor_landfall(&self, preferred: SurfaceCell) -> Option<SurfaceCell> {
        let surface_height = self.max_y - self.min_y + 1;
        let southern_band = self.min_y + surface_height * 3 / 5;
        let mut best: Option<(i32, SurfaceCell)> = None;
        for y in southern_band..=self.max_y {
            for x in self.min_x..=self.max_x {
                let cell = SurfaceCell::new(x, y);
                if !self.is_coastal_land(cell) {
                    continue;
                }
                let water_contacts = self.neighboring_water_count(cell) as i32;
                let distance = (x - preferred.x).abs() * 2 + (y - preferred.y).abs();
                let score = distance - water_contacts * 3;
                if best.is_none_or(|(best_score, _)| score < best_score) {
                    best = Some((score, cell));
                }
            }
        }
        best.map(|(_, cell)| cell)
    }

    pub(super) fn existing_willowmere_center(&self) -> Option<SurfaceCell> {
        for (chunk, index) in &self.scene_by_chunk {
            if let Some(object) = self.scenes[*index]
                .map
                .objects
                .iter()
                .find(|object| object.kind == ObjectKind::Well)
            {
                return Some(SurfaceCell::new(
                    chunk.x * MAP_W as i32 + object.x,
                    chunk.y * MAP_H as i32 + object.y,
                ));
            }
        }

        // Z84 removed the misclassified water-cooler well, so current saves
        // recover Willowmere from the symmetric civic/market/residential/
        // artisan lot metadata instead of depending on a decorative object.
        let mut sum_x = 0_i64;
        let mut sum_y = 0_i64;
        let mut count = 0_i64;
        for (chunk, index) in &self.scene_by_chunk {
            let scene = &self.scenes[*index];
            for y in 0..MAP_H as i32 {
                for x in 0..MAP_W as i32 {
                    if !matches!(
                        scene.zone_at(x, y),
                        ZoneKind::CivicLot
                            | ZoneKind::MarketLot
                            | ZoneKind::ResidentialLot
                            | ZoneKind::ArtisanLot
                    ) {
                        continue;
                    }
                    sum_x += i64::from(chunk.x * MAP_W as i32 + x);
                    sum_y += i64::from(chunk.y * MAP_H as i32 + y);
                    count += 1;
                }
            }
        }
        if count > 0 {
            return Some(SurfaceCell::new(
                (sum_x / count) as i32,
                (sum_y / count) as i32,
            ));
        }

        // Pre-zone saves may still contain the compact 11x11 StonePath plaza.
        // Its centroid is a stronger capital marker than the road network,
        // which may extend across the entire mainland.
        let mut stone_sum_x = 0_i64;
        let mut stone_sum_y = 0_i64;
        let mut stone_count = 0_i64;
        for (chunk, index) in &self.scene_by_chunk {
            let scene = &self.scenes[*index];
            for y in 0..MAP_H as i32 {
                for x in 0..MAP_W as i32 {
                    if scene.map.get(x, y) != TileKind::StonePath {
                        continue;
                    }
                    stone_sum_x += i64::from(chunk.x * MAP_W as i32 + x);
                    stone_sum_y += i64::from(chunk.y * MAP_H as i32 + y);
                    stone_count += 1;
                }
            }
        }
        (49..=256).contains(&stone_count).then(|| {
            SurfaceCell::new(
                (stone_sum_x / stone_count) as i32,
                (stone_sum_y / stone_count) as i32,
            )
        })
    }

    pub(super) fn paint_civic_plaza(&mut self, center: SurfaceCell) -> usize {
        let mut changed = 0;
        for y in center.y - 5..=center.y + 5 {
            for x in center.x - 5..=center.x + 5 {
                let cell = SurfaceCell::new(x, y);
                if self.is_city_land(cell) && self.set_tile(cell, TileKind::StonePath) {
                    changed += 1;
                }
            }
        }
        changed
    }

    pub(super) fn paint_civic_plaza_apron(&mut self, center: SurfaceCell) -> usize {
        let stone_cells = (center.y - 5..=center.y + 5)
            .flat_map(|y| (center.x - 5..=center.x + 5).map(move |x| (x, y)))
            .filter(|(x, y)| self.tile(SurfaceCell::new(*x, *y)) == Some(TileKind::StonePath))
            .count();
        if stone_cells < 81 {
            return 0;
        }

        // Stone_Tan has complete authored V7 tuples against Dirt_Tan (Road),
        // while Stone_Tan touching Dirt_Brown directly has no source tuple.
        // A one-cell Road apron guarantees the authored sequence
        // StonePath -> Road -> natural ground at every plaza edge and corner.
        let mut changed = 0;
        for y in center.y - 6..=center.y + 6 {
            for x in center.x - 6..=center.x + 6 {
                let dx = (x - center.x).abs();
                let dy = (y - center.y).abs();
                if dx != 6 && dy != 6 {
                    continue;
                }
                let cell = SurfaceCell::new(x, y);
                if self.is_city_land(cell)
                    && self.tile(cell) != Some(TileKind::StonePath)
                    && self.set_tile(cell, TileKind::Road)
                {
                    changed += 1;
                }
            }
        }
        changed
    }

    pub(super) fn reserve_city_plot(
        &mut self,
        center: SurfaceCell,
        half_w: i32,
        half_h: i32,
        zone: ZoneKind,
    ) -> (usize, usize) {
        let mut eligible = 0;
        let area = (half_w * 2 + 1) * (half_h * 2 + 1);
        for y in center.y - half_h..=center.y + half_h {
            for x in center.x - half_w..=center.x + half_w {
                let cell = SurfaceCell::new(x, y);
                if self.is_city_land(cell) && self.tile(cell) != Some(TileKind::Road) {
                    eligible += 1;
                }
            }
        }
        if eligible * 100 < area * 78 {
            return (0, 0);
        }

        let mut changed = 0;
        let mut removed = 0;
        for y in center.y - half_h..=center.y + half_h {
            for x in center.x - half_w..=center.x + half_w {
                let cell = SurfaceCell::new(x, y);
                if !self.is_city_land(cell) || self.tile(cell) == Some(TileKind::Road) {
                    continue;
                }
                removed += self.clear_natural_objects_at(cell);
                changed += usize::from(self.set_zone(cell, zone));
            }
        }
        (changed, removed)
    }

    pub(super) fn dominant_ground_around(
        &self,
        center: SurfaceCell,
        half_w: i32,
        half_h: i32,
    ) -> TileKind {
        const CANDIDATES: [TileKind; 4] = [
            TileKind::Grass,
            TileKind::Dirt,
            TileKind::Sand,
            TileKind::MudBank,
        ];
        let mut counts = [0usize; CANDIDATES.len()];
        for y in center.y - half_h - 2..=center.y + half_h + 2 {
            for x in center.x - half_w - 2..=center.x + half_w + 2 {
                if x > center.x - half_w - 1
                    && x < center.x + half_w + 1
                    && y > center.y - half_h - 1
                    && y < center.y + half_h + 1
                {
                    continue;
                }
                let Some(tile) = self.tile(SurfaceCell::new(x, y)) else {
                    continue;
                };
                if let Some(index) = CANDIDATES.iter().position(|candidate| *candidate == tile) {
                    counts[index] += 1;
                }
            }
        }
        counts
            .iter()
            .enumerate()
            .max_by_key(|(_, count)| **count)
            .map(|(index, _)| CANDIDATES[index])
            .unwrap_or(TileKind::Grass)
    }

    pub(super) fn clear_legacy_plot_foundation(
        &mut self,
        center: SurfaceCell,
        half_w: i32,
        half_h: i32,
    ) -> usize {
        let area = ((half_w * 2 + 1) * (half_h * 2 + 1)) as usize;
        let stone_cells = (center.y - half_h..=center.y + half_h)
            .flat_map(|y| (center.x - half_w..=center.x + half_w).map(move |x| (x, y)))
            .filter(|(x, y)| self.tile(SurfaceCell::new(*x, *y)) == Some(TileKind::StonePath))
            .count();
        if stone_cells * 100 < area * 70 {
            return 0;
        }
        let replacement = self.dominant_ground_around(center, half_w, half_h);
        let mut changed = 0;
        for y in center.y - half_h..=center.y + half_h {
            for x in center.x - half_w..=center.x + half_w {
                let cell = SurfaceCell::new(x, y);
                if self.tile(cell) == Some(TileKind::StonePath) && self.set_tile(cell, replacement)
                {
                    changed += 1;
                }
            }
        }
        changed
    }

    pub(super) fn reserve_harbor_district(&mut self, landfall: SurfaceCell) -> (usize, usize) {
        let center = SurfaceCell::new(landfall.x, landfall.y - 10);
        self.reserve_city_plot(center, 18, 10, ZoneKind::HarborLot)
    }

    pub(super) fn repair_harbor_road_intrusions(&mut self, landfall: SurfaceCell) -> usize {
        let mut changed = 0;
        for y in landfall.y - 4..=landfall.y + 10 {
            for x in landfall.x - 6..=landfall.x + 6 {
                let cell = SurfaceCell::new(x, y);
                if self.tile(cell) != Some(TileKind::Road) || self.neighboring_water_count(cell) < 4
                {
                    continue;
                }
                let replacement = [
                    SurfaceCell::new(x - 1, y),
                    SurfaceCell::new(x + 1, y),
                    SurfaceCell::new(x, y - 1),
                    SurfaceCell::new(x, y + 1),
                ]
                .into_iter()
                .filter_map(|neighbor| self.tile(neighbor))
                .find(|tile| tile.is_water())
                .unwrap_or(TileKind::OceanShallow);
                changed += usize::from(self.set_tile(cell, replacement));
            }
        }
        changed
    }
}
