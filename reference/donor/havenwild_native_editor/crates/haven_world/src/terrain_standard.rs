//! Havenwild Terrain Standard v1 identity and mapping contract.
//!
//! The JSON registries under `content/terrain/` are data authority. This module
//! keeps stable runtime names and prevents the historical "V7" label from
//! leaking into new gameplay/editor code.

use haven_core::BaseTerrain;

pub const HAVENWILD_TERRAIN_STANDARD_SCHEMA: &str = "havenwild.terrain_standard.v1";
pub const HAVENWILD_TERRAIN_STANDARD_VERSION: u32 = 1;
pub const HAVENWILD_TERRAIN_TILE_SIZE: u32 = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainStandardCode {
    Grass,
    GrassDark,
    GrassLight,
    GrassDead,
    DirtTan,
    DirtBrown,
    DirtDark,
    Sand,
    Soil,
    RockGray,
    RockDark,
    Water,
    WaterDeep,
    WaterShallowsSand,
    WaterShallowsDirt,
    Snow1,
    Snow2,
    Ice,
}

impl TerrainStandardCode {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Grass => "Grass",
            Self::GrassDark => "Grass_Dark",
            Self::GrassLight => "Grass_Light",
            Self::GrassDead => "Grass_Dead",
            Self::DirtTan => "Dirt_Tan",
            Self::DirtBrown => "Dirt_Brown",
            Self::DirtDark => "Dirt_Dark",
            Self::Sand => "Sand",
            Self::Soil => "Soil",
            Self::RockGray => "Rock_Gray",
            Self::RockDark => "Rock_Dark",
            Self::Water => "Water",
            Self::WaterDeep => "Water_Deep",
            Self::WaterShallowsSand => "Water_Shallows_Sand",
            Self::WaterShallowsDirt => "Water_Shallows_Dirt",
            Self::Snow1 => "Snow_1",
            Self::Snow2 => "Snow_2",
            Self::Ice => "Ice",
        }
    }

    pub const fn base_terrain(self) -> BaseTerrain {
        match self {
            Self::Grass
            | Self::GrassDark
            | Self::GrassLight
            | Self::GrassDead
            | Self::Snow1
            | Self::Snow2 => BaseTerrain::Grass,
            Self::DirtTan | Self::DirtBrown | Self::DirtDark => BaseTerrain::Dirt,
            Self::Sand => BaseTerrain::Sand,
            Self::Soil => BaseTerrain::Farm,
            Self::RockGray | Self::RockDark => BaseTerrain::StoneFloor,
            Self::Water | Self::WaterShallowsSand | Self::WaterShallowsDirt | Self::Ice => {
                BaseTerrain::ShallowWater
            }
            Self::WaterDeep => BaseTerrain::DeepWater,
        }
    }

    pub const fn runtime_enabled(self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_remains_32_pixel_and_shallow_first() {
        assert_eq!(HAVENWILD_TERRAIN_TILE_SIZE, 32);
        assert!(TerrainStandardCode::Water.runtime_enabled());
        assert!(TerrainStandardCode::WaterDeep.runtime_enabled());
    }
}
