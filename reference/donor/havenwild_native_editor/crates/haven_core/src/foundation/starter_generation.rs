use super::*;

impl TavernMap {
    pub(super) fn center_legacy_template(&mut self, fill: TileKind) {
        if MAP_W == LEGACY_MAP_W && MAP_H == LEGACY_MAP_H {
            return;
        }

        let source_tiles = self.tiles.clone();
        let source_heights = self.heights.clone();
        let (offset_x, offset_y) = scene_dimension_offset(LEGACY_MAP_W, LEGACY_MAP_H);
        self.tiles = vec![fill; MAP_W * MAP_H];
        self.heights = vec![48; MAP_W * MAP_H];

        for y in 0..LEGACY_MAP_H {
            for x in 0..LEGACY_MAP_W {
                let source_index = y * MAP_W + x;
                let target_x = x as i32 + offset_x;
                let target_y = y as i32 + offset_y;
                if let Some(target_index) = Self::idx(target_x, target_y) {
                    self.tiles[target_index] = source_tiles[source_index];
                    self.heights[target_index] = source_heights[source_index];
                }
            }
        }

        for object in &mut self.objects {
            object.x += offset_x;
            object.y += offset_y;
        }
        for stamp in &mut self.stamps {
            stamp.x += offset_x;
            stamp.y += offset_y;
        }
    }

    fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, tile: TileKind) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                self.set(x, y, tile);
            }
        }
    }

    fn add_noise_tiles(&mut self, seed: u32, tile: TileKind, chance_mod: u32) {
        // Starter templates are authored in the legacy 48x32 bank, then centered
        // into the expanded runtime scene by `center_legacy_template`.
        for y in 0..LEGACY_MAP_H as i32 {
            for x in 0..LEGACY_MAP_W as i32 {
                if hash_cell(seed, x, y).is_multiple_of(chance_mod) && self.get(x, y).walkable() {
                    self.set(x, y, tile);
                }
            }
        }
    }

    fn add_trees(&mut self, seed: u32, chance_mod: u32) {
        for y in 1..LEGACY_MAP_H as i32 - 1 {
            for x in 1..LEGACY_MAP_W as i32 - 1 {
                if hash_cell(seed, x, y).is_multiple_of(chance_mod)
                    && matches!(
                        self.get(x, y),
                        TileKind::Grass | TileKind::TallGrass | TileKind::Dirt
                    )
                    && self.object_at(x, y).is_none()
                {
                    let _ = self.place_object(ObjectKind::Tree, x, y);
                }
            }
        }
    }

    fn add_tall_grass(&mut self, seed: u32, chance_mod: u32) {
        for y in 1..LEGACY_MAP_H as i32 - 1 {
            for x in 1..LEGACY_MAP_W as i32 - 1 {
                if hash_cell(seed, x, y).is_multiple_of(chance_mod)
                    && self.get(x, y) == TileKind::Grass
                {
                    self.set(x, y, TileKind::TallGrass);
                }
            }
        }
    }

    /// Legacy compatibility entry point for the player Home Estate development fixture.
    ///
    /// The `farmstead` scene code remains stable for saves/transitions, but its
    /// user-facing identity is Estate. W43C deliberately removes the old exterior
    /// Wall/WoodFloor/GreenhouseMarker sketch: structures are published through
    /// certified building/connector authority instead of masquerading as terrain.
    pub(super) fn generate_farmstead_layout(&mut self, seed: u32) {
        self.generate_estate_development_layout(seed);
    }

    fn generate_estate_development_layout(&mut self, seed: u32) {
        // Main Estate approach. These are real mapped terrain materials and are
        // safe to display before the production Estate rebuild.
        for x in 0..LEGACY_MAP_W as i32 {
            self.set(x, 14, TileKind::Road);
            self.set(x, 15, TileKind::Road);
        }
        for x in 18..29 {
            self.set(x, 13, TileKind::Road);
        }

        // Reserve the future home/tavern pad using ordinary ground semantics.
        // Do not paint exterior Wall/WoodFloor cells; those were the cyan/gray
        // legacy-autotile artifacts visible in the native editor.
        self.fill_rect(16, 4, 30, 11, TileKind::Dirt);
        self.fill_rect(19, 7, 27, 11, TileKind::Sand);
        for x in 21..25 {
            self.set(x, 12, TileKind::Road);
        }

        // Early Estate field. Greenhouse construction is intentionally absent
        // until W45/W46 structure and building recipes are certified.
        self.fill_rect(5, 20, 18, 28, TileKind::Dirt);
        for y in 21..28 {
            for x in 6..18 {
                if (x + y) % 5 != 0 {
                    self.set(x, y, TileKind::TilledSoil);
                }
            }
        }

        // Freshwater creek remains semantic terrain. No generated live-autotile
        // artwork is required to represent these cells.
        for y in 0..LEGACY_MAP_H as i32 {
            let creek_x = 37 + ((y + seed as i32) % 5 == 0) as i32;
            self.set(creek_x, y, TileKind::DeepWater);
            self.set((creek_x - 1).max(0), y, TileKind::ShallowWater);
            if y % 3 == 0 {
                self.set((creek_x - 1).max(0), y, TileKind::Dirt);
            }
        }

        self.add_tall_grass(seed.wrapping_add(19), 11);
        self.add_trees(seed.wrapping_add(31), 17);

        // Do not place standalone Door/CaveEntrance/GreenhouseMarker placeholders
        // in the Estate fixture. Their transitions remain navigable through the
        // scene transition rectangles until structural connectors are published.
    }

    pub(super) fn generate_tavern_interior(&mut self, seed: u32) {
        for y in 4..26 {
            for x in 6..41 {
                let cut_corner = (x < 9 && y < 7) || (x > 37 && y > 22);
                let wall = !cut_corner && (x == 6 || x == 40 || y == 4 || y == 25);
                if !cut_corner {
                    self.set(
                        x,
                        y,
                        if wall {
                            TileKind::Wall
                        } else {
                            TileKind::WoodFloor
                        },
                    );
                }
            }
        }
        self.fill_rect(29, 6, 38, 12, TileKind::StoneFloor);
        self.fill_rect(9, 18, 15, 23, TileKind::StoneFloor);
        for y in 8..23 {
            if y % 5 == 0 {
                self.set(28, y, TileKind::Wall);
            }
        }
        self.set(23, 25, TileKind::WoodFloor);
        let _ = self.place_object(ObjectKind::Door, 23, 25);
        let _ = self.place_object(ObjectKind::Bar, 11, 10);
        let _ = self.place_object(ObjectKind::Keg, 32, 8);
        let _ = self.place_object(ObjectKind::Keg, 36, 8);
        let _ = self.place_object(ObjectKind::Fireplace, 37, 18);
        for (x, y) in [(18, 13), (23, 13), (18, 18), (24, 18), (13, 20)] {
            let _ = self.place_object(ObjectKind::Table, x, y);
            let _ = self.place_object(ObjectKind::Chair, x, y + 2);
            if hash_cell(seed, x, y).is_multiple_of(2) {
                let _ = self.place_object(ObjectKind::Chair, x + 2, y);
            }
        }
        let _ = self.place_object(ObjectKind::Stairs, 35, 22);
        let _ = self.place_object(ObjectKind::Door, 9, 22);
    }

    pub(super) fn generate_cellar(&mut self, seed: u32) {
        for y in 6..26 {
            for x in 7..40 {
                let pillar = (x == 18 || x == 29) && (11..=21).contains(&y) && y % 4 != 0;
                let wall = x == 7 || x == 39 || y == 6 || y == 25 || pillar;
                self.set(
                    x,
                    y,
                    if wall {
                        TileKind::CaveWall
                    } else {
                        TileKind::StoneFloor
                    },
                );
            }
        }
        let _ = self.place_object(ObjectKind::Stairs, 10, 22);
        for x in [12, 16, 22, 26, 32, 36] {
            let _ = self.place_object(ObjectKind::Keg, x, 14 + (hash_cell(seed, x, 14) % 3) as i32);
        }
    }

    pub(super) fn generate_guest_floor(&mut self, _seed: u32) {
        for y in 6..24 {
            for x in 8..39 {
                let divider = (x == 18 || x == 28) && (7..=21).contains(&y);
                let wall = x == 8 || x == 38 || y == 6 || y == 23 || divider;
                self.set(
                    x,
                    y,
                    if wall {
                        TileKind::Wall
                    } else {
                        TileKind::WoodFloor
                    },
                );
            }
        }
        for y in 12..18 {
            self.set(18, y, TileKind::WoodFloor);
            self.set(28, y, TileKind::WoodFloor);
        }
        for x in [13, 23, 33] {
            let _ = self.place_object(ObjectKind::Bed, x, 12);
            let _ = self.place_object(ObjectKind::Table, x + 1, 16);
        }
        let _ = self.place_object(ObjectKind::Stairs, 34, 21);
    }

    pub(super) fn generate_road_scene(&mut self, seed: u32) {
        for y in 12..17 {
            for x in 0..LEGACY_MAP_W as i32 {
                self.set(x, y, TileKind::StonePath);
            }
        }
        for x in 0..LEGACY_MAP_W as i32 {
            let y = 12 + ((x + seed as i32) % 5);
            self.set(x, y, TileKind::Road);
            self.set(x, (y + 1).min(MAP_H as i32 - 1), TileKind::StonePath);
        }
        self.add_noise_tiles(seed, TileKind::Dirt, 23);
        self.add_trees(seed.wrapping_add(71), 13);
    }

    pub(super) fn generate_south_field(&mut self, seed: u32) {
        for y in 7..26 {
            for x in 4..42 {
                self.set(x, y, TileKind::Dirt);
            }
        }
        for y in 9..24 {
            for x in 7..31 {
                if !hash_cell(seed, x, y).is_multiple_of(7) {
                    self.set(x, y, TileKind::TilledSoil);
                }
            }
        }
        for y in 7..26 {
            self.set(34 + (y % 2), y, TileKind::ShallowWater);
        }
        for x in 0..LEGACY_MAP_W as i32 {
            self.set(x, 2, TileKind::StonePath);
        }
        let _ = self.place_object(ObjectKind::GreenhouseMarker, 15, 16);
    }

    pub(super) fn generate_east_woods(&mut self, seed: u32) {
        for y in 0..LEGACY_MAP_H as i32 {
            for x in 0..LEGACY_MAP_W as i32 {
                if (12..=17).contains(&y) {
                    self.set(x, y, TileKind::StonePath);
                } else if hash_cell(seed, x, y).is_multiple_of(19) {
                    self.set(x, y, TileKind::Dirt);
                }
            }
        }
        for y in 0..LEGACY_MAP_H as i32 {
            let stream_x = 30 + ((y * 3 + seed as i32) % 4);
            self.set(stream_x, y, TileKind::DeepWater);
            self.set((stream_x - 1).max(0), y, TileKind::ShallowWater);
        }
        self.add_tall_grass(seed.wrapping_add(43), 7);
        self.add_trees(seed.wrapping_add(103), 5);
    }

    fn carve_cave_room(&mut self, x0: i32, y0: i32, x1: i32, y1: i32) {
        self.fill_rect(x0, y0, x1, y1, TileKind::CaveFloor);
    }

    fn carve_cave_corridor(&mut self, from: (i32, i32), to: (i32, i32), width: i32) {
        let half = (width.max(1) - 1) / 2;
        let (mut x, mut y) = from;
        let (tx, ty) = to;
        let step_x = if tx >= x { 1 } else { -1 };
        while x != tx {
            for oy in -half..=half {
                self.set(x, y + oy, TileKind::CaveFloor);
            }
            x += step_x;
        }
        let step_y = if ty >= y { 1 } else { -1 };
        while y != ty {
            for ox in -half..=half {
                self.set(x + ox, y, TileKind::CaveFloor);
            }
            y += step_y;
        }
        for ox in -half..=half {
            for oy in -half..=half {
                self.set(tx + ox, ty + oy, TileKind::CaveFloor);
            }
        }
    }

    fn cave_tile_is_wall_adjacent(&self, x: i32, y: i32) -> bool {
        matches!(self.get(x, y), TileKind::CaveFloor)
            && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .any(|(dx, dy)| matches!(self.get(x + dx, y + dy), TileKind::CaveWall))
    }

    fn place_cave_ore_candidates(&mut self, candidates: &[(i32, i32)]) {
        for &(x, y) in candidates {
            if self.cave_tile_is_wall_adjacent(x, y) && self.object_at(x, y).is_none() {
                let _ = self.place_object(ObjectKind::OreNode, x, y);
            }
        }
    }

    pub(super) fn generate_cave_mouth(&mut self, seed: u32) {
        // W50: cave topology is carved from a solid occupancy field. Random noise may
        // decorate optional alcoves, but it may never cut the guaranteed traversal path.
        self.fill_rect(5, 5, 41, 26, TileKind::CaveWall);

        self.carve_cave_room(6, 20, 12, 25);  // exterior landing / mouth chamber
        self.carve_cave_room(16, 13, 29, 21); // primary chamber
        self.carve_cave_room(35, 11, 40, 18); // deeper-cave connector chamber
        self.carve_cave_corridor((10, 22), (18, 18), 3);
        self.carve_cave_corridor((27, 17), (36, 15), 3);

        // Seeded side alcove changes discovery shape without risking connectivity.
        let alcove_y = 8 + (hash_cell(seed, 21, 9) % 3) as i32;
        self.carve_cave_room(20, alcove_y, 25, alcove_y + 3);
        self.carve_cave_corridor((22, 13), (22, alcove_y + 3), 1);

        // Transition landing areas remain explicit and walkable.
        self.carve_cave_room(7, 21, 10, 24);
        self.carve_cave_room(39, 13, 41, 16);

        // The ordinary generated mouth is the exact W49 1x3 ElizaWy connector.
        let _ = self.place_object(ObjectKind::CaveEntrance, 8, 23);
        self.place_cave_ore_candidates(&[(16, 15), (29, 19), (25, alcove_y + 1)]);
    }

    pub(super) fn generate_cave_depths(&mut self, seed: u32) {
        // Deterministic room/corridor hybrid: every required room is connected before
        // optional detail is applied, matching the authored cave generator contract.
        self.fill_rect(3, 3, 44, 28, TileKind::CaveWall);

        self.carve_cave_room(4, 20, 10, 25);  // return chamber
        self.carve_cave_room(14, 13, 23, 20); // central chamber
        self.carve_cave_room(29, 7, 38, 14);  // upper resource chamber
        self.carve_cave_room(28, 20, 39, 26); // lower chamber
        self.carve_cave_corridor((8, 22), (16, 17), 3);
        self.carve_cave_corridor((21, 15), (31, 11), 3);
        self.carve_cave_corridor((21, 18), (30, 22), 3);

        // Optional damp pocket remains off the critical route and uses the existing
        // terrain-water authority rather than creating cave-specific fake water art.
        let water_x = 32 + (hash_cell(seed, 32, 23) % 3) as i32;
        for y in 23..=24 {
            for x in water_x..=(water_x + 1).min(38) {
                self.set(x, y, TileKind::ShallowWater);
            }
        }

        self.carve_cave_room(4, 21, 7, 24); // explicit return transition landing
        self.place_cave_ore_candidates(&[(14, 15), (23, 18), (29, 9), (38, 12), (28, 24)]);
    }

}
