//! Havenwild 2.5D non-isometric grid helpers.
//!
//! Drop this into `crates/haven_core/src/grid_2p5d.rs` and add
//! `pub mod grid_2p5d;` near the top of `crates/haven_core/src/lib.rs`.
//!
//! This module intentionally keeps gameplay orthogonal/tile-snapped while allowing
//! taller sprites and tavern props to create a 2.5D view through Y-sorting.

use crate::{TavernMap, TileKind, TILE_SIZE};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TileCoord {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderLayer2p5d {
    VoidBackground,
    TerrainBase,
    TerrainOverlay,
    ObjectBack,
    ActorYSorted,
    ObjectFront,
    RoofCutaway,
    ToolPreview,
    UiOverlay,
}

impl RenderLayer2p5d {
    pub fn order(self) -> i32 {
        match self {
            RenderLayer2p5d::VoidBackground => -1000,
            RenderLayer2p5d::TerrainBase => 0,
            RenderLayer2p5d::TerrainOverlay => 100,
            RenderLayer2p5d::ObjectBack => 200,
            RenderLayer2p5d::ActorYSorted => 300,
            RenderLayer2p5d::ObjectFront => 400,
            RenderLayer2p5d::RoofCutaway => 500,
            RenderLayer2p5d::ToolPreview => 900,
            RenderLayer2p5d::UiOverlay => 1000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderSortKey {
    pub layer_order: i32,
    pub foot_y_px: i32,
    pub tie_x_px: i32,
    pub stable_id: u32,
}

impl RenderSortKey {
    pub fn for_ground(layer: RenderLayer2p5d, tile: TileCoord, stable_id: u32) -> Self {
        Self {
            layer_order: layer.order(),
            foot_y_px: tile.y * TILE_SIZE as i32,
            tie_x_px: tile.x * TILE_SIZE as i32,
            stable_id,
        }
    }

    pub fn for_sprite(
        layer: RenderLayer2p5d,
        tile: TileCoord,
        foot_offset_y_px: i32,
        stable_id: u32,
    ) -> Self {
        Self {
            layer_order: layer.order(),
            foot_y_px: tile.y * TILE_SIZE as i32 + foot_offset_y_px,
            tie_x_px: tile.x * TILE_SIZE as i32,
            stable_id,
        }
    }
}

pub fn grid_to_screen(tile: TileCoord, camera_px: ScreenPoint) -> ScreenPoint {
    ScreenPoint {
        x: tile.x as f32 * TILE_SIZE - camera_px.x,
        y: tile.y as f32 * TILE_SIZE - camera_px.y,
    }
}

pub fn screen_to_grid(screen: ScreenPoint, camera_px: ScreenPoint) -> TileCoord {
    TileCoord {
        x: ((screen.x + camera_px.x) / TILE_SIZE).floor() as i32,
        y: ((screen.y + camera_px.y) / TILE_SIZE).floor() as i32,
    }
}

pub fn tile_center_screen(tile: TileCoord, camera_px: ScreenPoint) -> ScreenPoint {
    let top_left = grid_to_screen(tile, camera_px);
    ScreenPoint {
        x: top_left.x + TILE_SIZE * 0.5,
        y: top_left.y + TILE_SIZE * 0.5,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridAction {
    Inspect,
    Hoe,
    Water,
    Dig,
    Plant,
    Build,
    PlaceObject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FootprintShape {
    Single,
    Line3Forward,
    Line5Forward,
    Square3x3,
    Cross5,
    Rect { w: i32, h: i32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Facing4 {
    North,
    East,
    South,
    West,
}

pub fn footprint_tiles(
    origin: TileCoord,
    shape: FootprintShape,
    facing: Facing4,
) -> Vec<TileCoord> {
    let mut offsets: Vec<(i32, i32)> = match shape {
        FootprintShape::Single => vec![(0, 0)],
        FootprintShape::Line3Forward => vec![(0, 0), (0, -1), (0, -2)],
        FootprintShape::Line5Forward => vec![(0, 0), (0, -1), (0, -2), (0, -3), (0, -4)],
        FootprintShape::Square3x3 => (-1..=1)
            .flat_map(|y| (-1..=1).map(move |x| (x, y)))
            .collect(),
        FootprintShape::Cross5 => vec![(0, 0), (0, -1), (-1, 0), (1, 0), (0, 1)],
        FootprintShape::Rect { w, h } => (0..h)
            .flat_map(|yy| (0..w).map(move |xx| (xx, yy)))
            .collect(),
    };

    for (x, y) in &mut offsets {
        let (rx, ry) = rotate_offset(*x, *y, facing);
        *x = rx;
        *y = ry;
    }

    offsets
        .into_iter()
        .map(|(x, y)| TileCoord {
            x: origin.x + x,
            y: origin.y + y,
        })
        .collect()
}

fn rotate_offset(x: i32, y: i32, facing: Facing4) -> (i32, i32) {
    match facing {
        Facing4::North => (x, y),
        Facing4::East => (-y, x),
        Facing4::South => (-x, -y),
        Facing4::West => (y, -x),
    }
}

pub fn tile_supports_action(tile: TileKind, action: GridAction) -> bool {
    match action {
        GridAction::Inspect => true,
        GridAction::Hoe => matches!(
            tile,
            TileKind::Grass | TileKind::TallGrass | TileKind::Dirt | TileKind::GreenhouseZone
        ),
        GridAction::Water => {
            matches!(
                tile,
                TileKind::TilledSoil | TileKind::WateredSoil | TileKind::Crop
            )
        }
        GridAction::Dig => matches!(
            tile,
            TileKind::Dirt
                | TileKind::MudBank
                | TileKind::Sand
                | TileKind::PebbleShore
                | TileKind::CaveFloor
        ),
        GridAction::Plant => matches!(
            tile,
            TileKind::TilledSoil | TileKind::WateredSoil | TileKind::GreenhouseZone
        ),
        GridAction::Build | GridAction::PlaceObject => {
            tile.walkable()
                && !matches!(
                    tile,
                    TileKind::Crop
                        | TileKind::TilledSoil
                        | TileKind::Water
                        | TileKind::ShallowWater
                        | TileKind::DeepWater
                        | TileKind::OceanDeep
                        | TileKind::OceanShallow
                        | TileKind::RiverWater
                        | TileKind::RiverMouthBlend
                        | TileKind::ShoreFoam
                )
        }
    }
}

pub fn validate_action_footprint(
    map: &TavernMap,
    origin: TileCoord,
    shape: FootprintShape,
    facing: Facing4,
    action: GridAction,
) -> Vec<(TileCoord, bool)> {
    footprint_tiles(origin, shape, facing)
        .into_iter()
        .map(|coord| {
            let valid = TavernMap::idx(coord.x, coord.y)
                .map(|_| tile_supports_action(map.get(coord.x, coord.y), action))
                .unwrap_or(false);
            (coord, valid)
        })
        .collect()
}

pub fn is_natural_fishable_water(tile: TileKind) -> bool {
    matches!(
        tile,
        TileKind::Water
            | TileKind::ShallowWater
            | TileKind::DeepWater
            | TileKind::OceanDeep
            | TileKind::OceanShallow
            | TileKind::RiverWater
            | TileKind::RiverMouthBlend
            | TileKind::ShoreFoam
    )
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoilMoistureState {
    Dry,
    Watered,
    Muddy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoilQuality {
    Poor,
    Normal,
    Fertile,
    Rich,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileFarmState {
    pub moisture: SoilMoistureState,
    pub quality: SoilQuality,
    pub crop_stage: u8,
    pub days_until_next_stage: u8,
}

impl Default for TileFarmState {
    fn default() -> Self {
        Self {
            moisture: SoilMoistureState::Dry,
            quality: SoilQuality::Normal,
            crop_stage: 0,
            days_until_next_stage: 0,
        }
    }
}
