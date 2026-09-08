use super::*;

impl Default for TavernMap {
    fn default() -> Self {
        Self::new()
    }
}

impl TavernMap {
    pub fn filled(tile: TileKind) -> Self {
        Self {
            tiles: vec![tile; MAP_W * MAP_H],
            heights: vec![48; MAP_W * MAP_H],
            structural_levels: vec![STRUCTURAL_LEVEL_AUTO; MAP_W * MAP_H],
            objects: Vec::new(),
            stamps: Vec::new(),
            object_asset_refs: BTreeMap::new(),
            object_states: BTreeMap::new(),
        }
    }

    pub fn new() -> Self {
        let fill = TileKind::Grass;
        let mut map = Self::filled(fill);
        map.generate_farmstead_layout(scene_seed(SceneId::Farmstead));
        map.center_legacy_template(fill);
        map
    }

    pub fn starter_for(scene: SceneId) -> Self {
        Self::starter_for_seed(scene, scene_seed(scene))
    }

    pub fn starter_for_seed(scene: SceneId, seed: u32) -> Self {
        let fill = match scene {
            SceneId::Cellar | SceneId::CaveMouth | SceneId::CaveDepths => TileKind::CaveWall,
            SceneId::TavernInterior | SceneId::GuestFloor => TileKind::Wall,
            _ => TileKind::Grass,
        };
        let mut map = Self::filled(fill);

        match scene {
            SceneId::Farmstead => map.generate_farmstead_layout(seed),
            SceneId::TavernInterior => map.generate_tavern_interior(seed),
            SceneId::Cellar => map.generate_cellar(seed),
            SceneId::GuestFloor => map.generate_guest_floor(seed),
            SceneId::NorthRoad => map.generate_road_scene(seed),
            SceneId::SouthField => map.generate_south_field(seed),
            SceneId::EastWoods => map.generate_east_woods(seed),
            SceneId::CaveMouth => map.generate_cave_mouth(seed),
            SceneId::CaveDepths => map.generate_cave_depths(seed),
        }

        map.center_legacy_template(fill);
        map
    }

    pub fn empty_with(tile: TileKind) -> Self {
        Self {
            tiles: vec![tile; MAP_W * MAP_H],
            heights: vec![48; MAP_W * MAP_H],
            structural_levels: vec![STRUCTURAL_LEVEL_AUTO; MAP_W * MAP_H],
            objects: Vec::new(),
            stamps: Vec::new(),
            object_asset_refs: BTreeMap::new(),
            object_states: BTreeMap::new(),
        }
    }

    pub fn idx(x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
            return None;
        }
        Some(y as usize * MAP_W + x as usize)
    }

    pub fn get(&self, x: i32, y: i32) -> TileKind {
        Self::idx(x, y).map_or(TileKind::Wall, |idx| self.tiles[idx])
    }

    pub fn set(&mut self, x: i32, y: i32, tile: TileKind) {
        if let Some(idx) = Self::idx(x, y) {
            self.tiles[idx] = tile;
        }
    }

    pub fn get_height(&self, x: i32, y: i32) -> u8 {
        Self::idx(x, y).map_or(0, |idx| self.heights[idx])
    }

    pub fn set_height(&mut self, x: i32, y: i32, height: u8) {
        if let Some(idx) = Self::idx(x, y) {
            self.heights[idx] = height;
        }
    }

    pub fn add_height(&mut self, x: i32, y: i32, delta: i32) {
        let current = self.get_height(x, y) as i32;
        self.set_height(x, y, (current + delta).clamp(0, 100) as u8);
    }

    /// Returns an explicitly authored structural level. `None` means the cell
    /// still follows legacy/generated fallback classification.
    pub fn get_structural_level(&self, x: i32, y: i32) -> Option<u8> {
        Self::idx(x, y).and_then(|idx| {
            let value = self
                .structural_levels
                .get(idx)
                .copied()
                .unwrap_or(STRUCTURAL_LEVEL_AUTO);
            (value <= MAX_STRUCTURAL_LEVEL).then_some(value)
        })
    }

    pub fn structural_level_storage(&self, x: i32, y: i32) -> u8 {
        Self::idx(x, y).map_or(STRUCTURAL_LEVEL_AUTO, |idx| {
            self.structural_levels
                .get(idx)
                .copied()
                .unwrap_or(STRUCTURAL_LEVEL_AUTO)
        })
    }

    pub fn set_structural_level(&mut self, x: i32, y: i32, level: Option<u8>) {
        if let Some(idx) = Self::idx(x, y) {
            self.structural_levels[idx] = level
                .map(|value| value.min(MAX_STRUCTURAL_LEVEL))
                .unwrap_or(STRUCTURAL_LEVEL_AUTO);
        }
    }
}
