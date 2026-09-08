use haven_core::{SceneBiome, TavernMap, TileKind, MAP_H, MAP_W};

use crate::apply_coastline_tile_pass;

/// Canonical material families exposed by the world paint backend.
///
/// Keep this smaller than `TerrainFamily`: this is the creator-facing brush
/// surface for the first strict 32x32 world environment pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldPaintFamily {
    Sand,
    Water,
    Cave,
    PavedBrick,
    WoodPlank,
}

impl WorldPaintFamily {
    pub const ALL: [WorldPaintFamily; 5] = [
        WorldPaintFamily::Sand,
        WorldPaintFamily::Water,
        WorldPaintFamily::Cave,
        WorldPaintFamily::PavedBrick,
        WorldPaintFamily::WoodPlank,
    ];

    pub fn code(self) -> &'static str {
        match self {
            WorldPaintFamily::Sand => "sand",
            WorldPaintFamily::Water => "water",
            WorldPaintFamily::Cave => "cave",
            WorldPaintFamily::PavedBrick => "paved_brick",
            WorldPaintFamily::WoodPlank => "wood_plank",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            WorldPaintFamily::Sand => "Sand",
            WorldPaintFamily::Water => "Water",
            WorldPaintFamily::Cave => "Cave",
            WorldPaintFamily::PavedBrick => "Grey Paved Brick",
            WorldPaintFamily::WoodPlank => "Wood Plank Floor",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "sand" => Some(WorldPaintFamily::Sand),
            "water" => Some(WorldPaintFamily::Water),
            "cave" => Some(WorldPaintFamily::Cave),
            "paved_brick" => Some(WorldPaintFamily::PavedBrick),
            "wood_plank" => Some(WorldPaintFamily::WoodPlank),
            _ => None,
        }
    }

    pub fn default_layer(self) -> WorldPaintLayer {
        match self {
            WorldPaintFamily::Sand => WorldPaintLayer::GroundBase,
            WorldPaintFamily::Water => WorldPaintLayer::WaterBase,
            WorldPaintFamily::Cave => WorldPaintLayer::CaveBase,
            WorldPaintFamily::PavedBrick => WorldPaintLayer::TownSurface,
            WorldPaintFamily::WoodPlank => WorldPaintLayer::IndoorFloor,
        }
    }

    pub fn default_tile(self) -> TileKind {
        match self {
            WorldPaintFamily::Sand => TileKind::Sand,
            WorldPaintFamily::Water => TileKind::ShallowWater,
            WorldPaintFamily::Cave => TileKind::CaveFloor,
            WorldPaintFamily::PavedBrick => TileKind::BrickFloor,
            WorldPaintFamily::WoodPlank => TileKind::PlankFloor,
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        let current = Self::ALL
            .iter()
            .position(|value| *value == self)
            .unwrap_or(0) as i32;
        let len = Self::ALL.len() as i32;
        Self::ALL[((current + delta).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldPaintLayer {
    GroundBase,
    WaterBase,
    CaveBase,
    TownSurface,
    IndoorFloor,
}

impl WorldPaintLayer {
    pub const ALL: [WorldPaintLayer; 5] = [
        WorldPaintLayer::GroundBase,
        WorldPaintLayer::WaterBase,
        WorldPaintLayer::CaveBase,
        WorldPaintLayer::TownSurface,
        WorldPaintLayer::IndoorFloor,
    ];

    pub fn code(self) -> &'static str {
        match self {
            WorldPaintLayer::GroundBase => "ground_base",
            WorldPaintLayer::WaterBase => "water_base",
            WorldPaintLayer::CaveBase => "cave_base",
            WorldPaintLayer::TownSurface => "town_surface",
            WorldPaintLayer::IndoorFloor => "indoor_floor",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            WorldPaintLayer::GroundBase => "Ground Base",
            WorldPaintLayer::WaterBase => "Water Base",
            WorldPaintLayer::CaveBase => "Cave Base",
            WorldPaintLayer::TownSurface => "Town Surface",
            WorldPaintLayer::IndoorFloor => "Indoor Floor",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "ground_base" => Some(WorldPaintLayer::GroundBase),
            "water_base" => Some(WorldPaintLayer::WaterBase),
            "cave_base" => Some(WorldPaintLayer::CaveBase),
            "town_surface" => Some(WorldPaintLayer::TownSurface),
            "indoor_floor" => Some(WorldPaintLayer::IndoorFloor),
            _ => None,
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        let current = Self::ALL
            .iter()
            .position(|value| *value == self)
            .unwrap_or(0) as i32;
        let len = Self::ALL.len() as i32;
        Self::ALL[((current + delta).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldPaintSubcellMode {
    Tile32,
    Cell16,
    Cell8,
    Cell4,
}

impl WorldPaintSubcellMode {
    pub const ALL: [WorldPaintSubcellMode; 4] = [
        WorldPaintSubcellMode::Tile32,
        WorldPaintSubcellMode::Cell16,
        WorldPaintSubcellMode::Cell8,
        WorldPaintSubcellMode::Cell4,
    ];

    pub fn code(self) -> &'static str {
        match self {
            WorldPaintSubcellMode::Tile32 => "32x32",
            WorldPaintSubcellMode::Cell16 => "16x16",
            WorldPaintSubcellMode::Cell8 => "8x8",
            WorldPaintSubcellMode::Cell4 => "4x4",
        }
    }

    pub fn grid(self) -> [u32; 2] {
        match self {
            WorldPaintSubcellMode::Tile32 => [1, 1],
            WorldPaintSubcellMode::Cell16 => [2, 2],
            WorldPaintSubcellMode::Cell8 => [4, 4],
            WorldPaintSubcellMode::Cell4 => [8, 8],
        }
    }

    pub fn subcell_count(self) -> u32 {
        let grid = self.grid();
        grid[0] * grid[1]
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "32x32" => Some(WorldPaintSubcellMode::Tile32),
            "16x16" => Some(WorldPaintSubcellMode::Cell16),
            "8x8" => Some(WorldPaintSubcellMode::Cell8),
            "4x4" => Some(WorldPaintSubcellMode::Cell4),
            _ => None,
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        let current = Self::ALL
            .iter()
            .position(|value| *value == self)
            .unwrap_or(0) as i32;
        let len = Self::ALL.len() as i32;
        Self::ALL[((current + delta).rem_euclid(len)) as usize]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WorldPaintBrushSettings {
    pub family: WorldPaintFamily,
    pub layer: WorldPaintLayer,
    pub radius_tiles: i32,
    pub strength: f32,
    pub subcell_mode: WorldPaintSubcellMode,
    pub autotile_refresh: bool,
    pub mirror_horizontal: bool,
    pub mirror_vertical: bool,
}

impl WorldPaintBrushSettings {
    pub fn normalized(mut self) -> Self {
        self.radius_tiles = self.radius_tiles.clamp(1, 9);
        self.strength = self.strength.clamp(0.05, 1.0);
        self
    }

    pub fn target_tile(self) -> TileKind {
        self.family.default_tile()
    }

    pub fn mirror_label(self) -> &'static str {
        match (self.mirror_horizontal, self.mirror_vertical) {
            (false, false) => "off",
            (true, false) => "horizontal",
            (false, true) => "vertical",
            (true, true) => "horizontal+vertical",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldPaintBounds {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl WorldPaintBounds {
    pub fn single(x: i32, y: i32) -> Self {
        Self {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        }
    }

    pub fn include(&mut self, x: i32, y: i32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    pub fn width(self) -> i32 {
        self.max_x - self.min_x + 1
    }

    pub fn height(self) -> i32 {
        self.max_y - self.min_y + 1
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorldPaintReport {
    pub family: WorldPaintFamily,
    pub layer: WorldPaintLayer,
    pub subcell_mode: WorldPaintSubcellMode,
    pub painted_tiles: usize,
    pub affected_subcells: usize,
    pub autotile_refreshed: bool,
    pub cleanup_mutations: usize,
    pub changed_bounds: Option<WorldPaintBounds>,
    pub mirror_horizontal: bool,
    pub mirror_vertical: bool,
    pub mirror_centers: Vec<[i32; 2]>,
    pub deterministic_note: String,
}

impl WorldPaintReport {
    pub fn status_line(&self) -> String {
        let bounds = self
            .changed_bounds
            .map(|bounds| {
                format!(
                    " bounds {}x{} @ {},{}",
                    bounds.width(),
                    bounds.height(),
                    bounds.min_x,
                    bounds.min_y
                )
            })
            .unwrap_or_default();
        let mirror = match (self.mirror_horizontal, self.mirror_vertical) {
            (false, false) => "mirror off".to_string(),
            (true, false) => format!("mirror H / {} center(s)", self.mirror_centers.len()),
            (false, true) => format!("mirror V / {} center(s)", self.mirror_centers.len()),
            (true, true) => format!("mirror H+V / {} center(s)", self.mirror_centers.len()),
        };
        format!(
            "Painted {} {} tile(s) on {} using {} masks; {} subcell(s); autotile {}; {}{}",
            self.painted_tiles,
            self.family.label(),
            self.layer.code(),
            self.subcell_mode.code(),
            self.affected_subcells,
            if self.autotile_refreshed {
                "refreshed"
            } else {
                "off"
            },
            mirror,
            bounds
        )
    }
}

/// Paints canonical 32x32 world tiles while preserving the future subcell mask
/// contract. The current prototype still stores the resolved `TileKind` in the
/// scene map; the report records deterministic subcell coverage so the same
/// call shape can later persist material weights/deltas for multiplayer saves.
pub fn apply_world_paint_brush(
    map: &mut TavernMap,
    center_x: i32,
    center_y: i32,
    biome: SceneBiome,
    settings: WorldPaintBrushSettings,
) -> WorldPaintReport {
    let settings = settings.normalized();
    let mut painted_tiles = 0usize;
    let radius = settings.radius_tiles.max(1);
    let radius_f = radius as f32 + 0.001;
    let target = settings.target_tile();
    let mut changed_bounds: Option<WorldPaintBounds> = None;
    let mirror_centers = world_paint_mirrored_centers(
        center_x,
        center_y,
        settings.mirror_horizontal,
        settings.mirror_vertical,
    );

    for [paint_center_x, paint_center_y] in &mirror_centers {
        for y in paint_center_y - radius..=paint_center_y + radius {
            for x in paint_center_x - radius..=paint_center_x + radius {
                if x < 0 || y < 0 || x >= MAP_W as i32 || y >= MAP_H as i32 {
                    continue;
                }
                let dx = x - paint_center_x;
                let dy = y - paint_center_y;
                let distance = ((dx * dx + dy * dy) as f32).sqrt();
                if distance > radius_f {
                    continue;
                }
                map.set(x, y, target);
                painted_tiles += 1;
                if let Some(bounds) = changed_bounds.as_mut() {
                    bounds.include(x, y);
                } else {
                    changed_bounds = Some(WorldPaintBounds::single(x, y));
                }
            }
        }
    }

    let mut cleanup_mutations = 0usize;
    let mut autotile_refreshed = false;
    if settings.autotile_refresh
        && matches!(
            settings.family,
            WorldPaintFamily::Sand | WorldPaintFamily::Water
        )
    {
        let report = apply_coastline_tile_pass(map, biome);
        cleanup_mutations = report.total_mutations();
        autotile_refreshed = true;
    }

    let affected_subcells = painted_tiles * settings.subcell_mode.subcell_count() as usize;
    WorldPaintReport {
        family: settings.family,
        layer: settings.layer,
        subcell_mode: settings.subcell_mode,
        painted_tiles,
        affected_subcells,
        autotile_refreshed,
        cleanup_mutations,
        changed_bounds,
        mirror_horizontal: settings.mirror_horizontal,
        mirror_vertical: settings.mirror_vertical,
        mirror_centers,
        deterministic_note: "authoritative edit stores material/tile IDs, mirror flags, and centers; clients derive transition visuals".to_string(),
    }
}

pub fn world_paint_mirrored_centers(
    center_x: i32,
    center_y: i32,
    mirror_horizontal: bool,
    mirror_vertical: bool,
) -> Vec<[i32; 2]> {
    let mut centers = vec![[center_x, center_y]];
    if mirror_horizontal {
        centers.push([(MAP_W as i32 - 1) - center_x, center_y]);
    }
    if mirror_vertical {
        centers.push([center_x, (MAP_H as i32 - 1) - center_y]);
    }
    if mirror_horizontal && mirror_vertical {
        centers.push([(MAP_W as i32 - 1) - center_x, (MAP_H as i32 - 1) - center_y]);
    }
    centers.sort_unstable();
    centers.dedup();
    centers
}
