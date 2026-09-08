//! Finite/endless overworld coordinate contract.
//!
//! New Havenwild surfaces use a chunk-aligned geographic origin and may be
//! realistically bounded or endless. Legacy east/west wrapping remains
//! loadable for older saves. Interiors, caves, ruins, and dungeons continue to
//! use scene transitions and do not use this topology directly.

use serde::{Deserialize, Serialize};

use crate::open_world::{ChunkCoord, WorldSurfaceConfig, WorldTileCoord};

pub const WORLD_TOPOLOGY_SCHEMA: &str = "havenwild.world_topology.v1";


#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldTopologyExtent {
    #[default]
    Finite,
    Endless,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HorizontalWrapMode {
    Disabled,
    EastWest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerticalBoundaryMode {
    Clamped,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldTopologyConfig {
    pub schema: String,
    #[serde(default)]
    pub extent: WorldTopologyExtent,
    /// Finite dimensions, or the player-facing preview/planning span for an
    /// Endless world. Endless canonicalization deliberately ignores these.
    pub width_tiles: i32,
    pub height_tiles: i32,
    /// Chunk-aligned global origin of a finite world's minimum tile. Older
    /// saves default to the legacy 0,0 origin.
    #[serde(default)]
    pub origin_x_tiles: i32,
    #[serde(default)]
    pub origin_y_tiles: i32,
    pub chunk_size_tiles: i32,
    pub horizontal_wrap: HorizontalWrapMode,
    pub vertical_boundary: VerticalBoundaryMode,
}

impl WorldTopologyConfig {
    pub fn from_surface(surface: &WorldSurfaceConfig) -> Self {
        Self {
            schema: WORLD_TOPOLOGY_SCHEMA.to_string(),
            extent: WorldTopologyExtent::Finite,
            width_tiles: surface.width_tiles.max(1),
            height_tiles: surface.height_tiles.max(1),
            origin_x_tiles: 0,
            origin_y_tiles: 0,
            chunk_size_tiles: surface.chunk_size_tiles.max(1),
            horizontal_wrap: HorizontalWrapMode::EastWest,
            vertical_boundary: VerticalBoundaryMode::Clamped,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != WORLD_TOPOLOGY_SCHEMA {
            return Err(format!("unsupported world topology schema {}", self.schema));
        }
        if self.width_tiles <= 0 || self.height_tiles <= 0 || self.chunk_size_tiles <= 0 {
            return Err("world dimensions and chunk size must be positive".to_string());
        }
        if self.extent == WorldTopologyExtent::Finite
            && (self.width_tiles % self.chunk_size_tiles != 0
                || self.height_tiles % self.chunk_size_tiles != 0
                || self.origin_x_tiles % self.chunk_size_tiles != 0
                || self.origin_y_tiles % self.chunk_size_tiles != 0)
        {
            return Err("finite world dimensions and origin must be chunk aligned".to_string());
        }
        Ok(())
    }

    pub fn chunk_count_x(&self) -> i32 {
        ceil_div(self.width_tiles, self.chunk_size_tiles)
    }

    pub fn chunk_count_y(&self) -> i32 {
        ceil_div(self.height_tiles, self.chunk_size_tiles)
    }

    pub fn canonical_tile(&self, tile: WorldTileCoord) -> Option<WorldTileCoord> {
        if self.extent == WorldTopologyExtent::Endless {
            return Some(tile);
        }
        let max_y = self.origin_y_tiles.saturating_add(self.height_tiles);
        if tile.y < self.origin_y_tiles || tile.y >= max_y {
            return None;
        }
        let max_x = self.origin_x_tiles.saturating_add(self.width_tiles);
        let x = match self.horizontal_wrap {
            HorizontalWrapMode::Disabled if tile.x < self.origin_x_tiles || tile.x >= max_x => {
                return None
            }
            HorizontalWrapMode::Disabled => tile.x,
            HorizontalWrapMode::EastWest => {
                self.origin_x_tiles
                    + (tile.x - self.origin_x_tiles).rem_euclid(self.width_tiles)
            }
        };
        Some(WorldTileCoord::new(x, tile.y))
    }

    pub fn canonical_chunk(&self, chunk: ChunkCoord) -> Option<ChunkCoord> {
        if self.extent == WorldTopologyExtent::Endless {
            return Some(chunk);
        }
        let origin_chunk_x = self.origin_x_tiles.div_euclid(self.chunk_size_tiles);
        let origin_chunk_y = self.origin_y_tiles.div_euclid(self.chunk_size_tiles);
        let max_chunk_y = origin_chunk_y + self.chunk_count_y();
        if chunk.y < origin_chunk_y || chunk.y >= max_chunk_y {
            return None;
        }
        let count_x = self.chunk_count_x();
        let max_chunk_x = origin_chunk_x + count_x;
        let x = match self.horizontal_wrap {
            HorizontalWrapMode::Disabled
                if chunk.x < origin_chunk_x || chunk.x >= max_chunk_x =>
            {
                return None
            }
            HorizontalWrapMode::Disabled => chunk.x,
            HorizontalWrapMode::EastWest => {
                origin_chunk_x + (chunk.x - origin_chunk_x).rem_euclid(count_x)
            }
        };
        Some(ChunkCoord::new(x, chunk.y))
    }

    pub fn tile_to_chunk(&self, tile: WorldTileCoord) -> Option<ChunkCoord> {
        self.canonical_tile(tile)
            .map(|tile| tile.chunk(self.chunk_size_tiles))
            .and_then(|chunk| self.canonical_chunk(chunk))
    }

    pub fn cardinal_neighbors(&self, tile: WorldTileCoord) -> [Option<WorldTileCoord>; 4] {
        [
            self.canonical_tile(WorldTileCoord::new(tile.x, tile.y - 1)),
            self.canonical_tile(WorldTileCoord::new(tile.x + 1, tile.y)),
            self.canonical_tile(WorldTileCoord::new(tile.x, tile.y + 1)),
            self.canonical_tile(WorldTileCoord::new(tile.x - 1, tile.y)),
        ]
    }

    /// Returns the shortest signed horizontal displacement from `from_x` to
    /// `to_x`, crossing the seam when that is shorter.
    pub fn shortest_wrapped_delta_x(&self, from_x: i32, to_x: i32) -> i32 {
        if self.extent == WorldTopologyExtent::Endless
            || self.horizontal_wrap == HorizontalWrapMode::Disabled
        {
            return to_x - from_x;
        }
        let width = self.width_tiles.max(1);
        let direct = to_x.rem_euclid(width) - from_x.rem_euclid(width);
        let half = width / 2;
        if direct > half {
            direct - width
        } else if direct < -half {
            direct + width
        } else {
            direct
        }
    }

    /// Maps a canonical tile X to the nearest unwrapped copy relative to a
    /// camera/player X. This keeps rendering and editor navigation continuous
    /// while persistence remains canonical.
    pub fn nearest_unwrapped_x(&self, reference_x: i32, canonical_x: i32) -> i32 {
        reference_x + self.shortest_wrapped_delta_x(reference_x, canonical_x)
    }

    pub fn persistence_key(&self, chunk: ChunkCoord) -> Option<WorldChunkPersistenceKey> {
        self.canonical_chunk(chunk)
            .map(|chunk| WorldChunkPersistenceKey {
                topology_schema: self.schema.clone(),
                chunk_x: chunk.x,
                chunk_y: chunk.y,
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorldChunkPersistenceKey {
    pub topology_schema: String,
    pub chunk_x: i32,
    pub chunk_y: i32,
}

impl WorldChunkPersistenceKey {
    pub fn stable_id(&self) -> String {
        format!(
            "{}:chunk:{},{}",
            self.topology_schema, self.chunk_x, self.chunk_y
        )
    }
}

fn ceil_div(value: i32, divisor: i32) -> i32 {
    (value + divisor - 1) / divisor
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_world::WorldSurfaceConfig;

    fn topology() -> WorldTopologyConfig {
        let mut surface = WorldSurfaceConfig::standard(42);
        surface.width_tiles = 1024;
        surface.height_tiles = 512;
        surface.chunk_size_tiles = 64;
        WorldTopologyConfig::from_surface(&surface)
    }

    #[test]
    fn finite_worlds_support_chunk_aligned_negative_origins() {
        let mut topology = topology();
        topology.origin_x_tiles = -512;
        topology.origin_y_tiles = -448;
        topology.horizontal_wrap = HorizontalWrapMode::Disabled;
        assert_eq!(
            topology.canonical_tile(WorldTileCoord::new(-512, -448)),
            Some(WorldTileCoord::new(-512, -448))
        );
        assert_eq!(topology.canonical_tile(WorldTileCoord::new(-513, -448)), None);
        assert_eq!(topology.canonical_tile(WorldTileCoord::new(512, -448)), None);
        assert!(topology.validate().is_ok());
    }

    #[test]
    fn east_and_west_tiles_wrap_to_the_same_canonical_surface() {
        let topology = topology();
        assert_eq!(
            topology.canonical_tile(WorldTileCoord::new(-1, 20)),
            Some(WorldTileCoord::new(1023, 20))
        );
        assert_eq!(
            topology.canonical_tile(WorldTileCoord::new(1024, 20)),
            Some(WorldTileCoord::new(0, 20))
        );
    }

    #[test]
    fn north_and_south_remain_bounded() {
        let topology = topology();
        assert_eq!(topology.canonical_tile(WorldTileCoord::new(10, -1)), None);
        assert_eq!(topology.canonical_tile(WorldTileCoord::new(10, 512)), None);
    }

    #[test]
    fn seam_neighbors_are_contiguous() {
        let topology = topology();
        let neighbors = topology.cardinal_neighbors(WorldTileCoord::new(0, 100));
        assert_eq!(neighbors[3], Some(WorldTileCoord::new(1023, 100)));
    }

    #[test]
    fn shortest_delta_crosses_the_seam() {
        let topology = topology();
        assert_eq!(topology.shortest_wrapped_delta_x(1020, 3), 7);
        assert_eq!(topology.shortest_wrapped_delta_x(3, 1020), -7);
    }

    #[test]
    fn endless_coordinates_are_never_clamped_or_wrapped() {
        let mut topology = topology();
        topology.extent = WorldTopologyExtent::Endless;
        topology.horizontal_wrap = HorizontalWrapMode::Disabled;
        assert_eq!(
            topology.canonical_tile(WorldTileCoord::new(-50_000, 80_000)),
            Some(WorldTileCoord::new(-50_000, 80_000))
        );
        assert_eq!(
            topology.canonical_chunk(ChunkCoord::new(-900, 1_200)),
            Some(ChunkCoord::new(-900, 1_200))
        );
    }

    #[test]
    fn persistence_keys_are_canonical() {
        let topology = topology();
        let key = topology.persistence_key(ChunkCoord::new(-1, 2)).unwrap();
        assert_eq!(key.chunk_x, 15);
        assert_eq!(key.stable_id(), "havenwild.world_topology.v1:chunk:15,2");
    }
}
