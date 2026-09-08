use crate::water_render_mask::WaterRenderMask;
use haven_core::{SceneBiome, TileAutoGroup, TileKind};

/// Reusable topology classification. Appearance is deliberately excluded so
/// the same V7-resolved shape can be rendered with any compatible material.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainShape {
    Full,
    Isolated,
    EndNorth,
    EndEast,
    EndSouth,
    EndWest,
    StraightHorizontal,
    StraightVertical,
    OuterNorthEast,
    OuterSouthEast,
    OuterSouthWest,
    OuterNorthWest,
    TeeNorth,
    TeeEast,
    TeeSouth,
    TeeWest,
    Cross,
    Complex(u8),
}

impl TerrainShape {
    pub const fn from_cardinal_mask(mask: u8) -> Self {
        match mask & 0x0f {
            0x0 => Self::Isolated,
            0x1 => Self::EndNorth,
            0x2 => Self::EndEast,
            0x4 => Self::EndSouth,
            0x8 => Self::EndWest,
            0x5 => Self::StraightVertical,
            0xa => Self::StraightHorizontal,
            0x3 => Self::OuterNorthEast,
            0x6 => Self::OuterSouthEast,
            0xc => Self::OuterSouthWest,
            0x9 => Self::OuterNorthWest,
            0x7 => Self::TeeSouth,
            0xb => Self::TeeWest,
            0xd => Self::TeeNorth,
            0xe => Self::TeeEast,
            0xf => Self::Cross,
            value => Self::Complex(value),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainMaterialId {
    GrassTemperate,
    GrassHighland,
    SandDry,
    SandWet,
    PebbleShore,
    SoilLoam,
    SoilWatered,
    Mud,
    Stone,
    MountainRock,
    Road,
    Wood,
    Cave,
    WaterFreshShallow,
    WaterFreshDeep,
    WaterOceanShallow,
    WaterOceanDeep,
    WaterRiver,
    Foam,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainMaterialProfile {
    pub id: TerrainMaterialId,
    pub code: &'static str,
    pub animated: bool,
    pub blend_softness: f32,
    pub detail_scale: f32,
    pub wetness_response: f32,
    pub reflection_strength: f32,
    pub foam_strength: f32,
}

impl TerrainMaterialProfile {
    pub const fn for_id(id: TerrainMaterialId) -> Self {
        let (code, animated, blend_softness, detail_scale, wetness, reflection, foam) = match id {
            TerrainMaterialId::GrassTemperate => {
                ("grass_temperate", false, 0.18, 1.0, 0.28, 0.0, 0.0)
            }
            TerrainMaterialId::GrassHighland => {
                ("grass_highland", false, 0.14, 0.85, 0.18, 0.0, 0.0)
            }
            TerrainMaterialId::SandDry => ("sand_dry", false, 0.24, 1.15, 0.62, 0.0, 0.0),
            TerrainMaterialId::SandWet => ("sand_wet", false, 0.28, 1.05, 1.0, 0.08, 0.16),
            TerrainMaterialId::PebbleShore => ("gravel", false, 0.12, 0.75, 0.72, 0.0, 0.08),
            TerrainMaterialId::SoilLoam => ("soil_loam", false, 0.16, 1.0, 0.56, 0.0, 0.0),
            TerrainMaterialId::SoilWatered => ("soil_watered", false, 0.16, 1.0, 1.0, 0.04, 0.0),
            TerrainMaterialId::Mud => ("mud", false, 0.22, 0.9, 1.0, 0.06, 0.0),
            TerrainMaterialId::Stone => ("stone", false, 0.08, 0.7, 0.08, 0.0, 0.0),
            TerrainMaterialId::MountainRock => ("mountain_rock", false, 0.08, 0.65, 0.06, 0.0, 0.0),
            TerrainMaterialId::Road => ("road", false, 0.12, 1.0, 0.24, 0.0, 0.0),
            TerrainMaterialId::Wood => ("wood", false, 0.04, 1.0, 0.18, 0.0, 0.0),
            TerrainMaterialId::Cave => ("cave", false, 0.08, 0.72, 0.12, 0.0, 0.0),
            TerrainMaterialId::WaterFreshShallow => {
                ("water_fresh_shallow", true, 0.34, 0.8, 1.0, 0.20, 0.45)
            }
            TerrainMaterialId::WaterFreshDeep => {
                ("water_fresh_deep", true, 0.36, 0.68, 1.0, 0.32, 0.16)
            }
            TerrainMaterialId::WaterOceanShallow => {
                ("water_ocean_shallow", true, 0.38, 0.72, 1.0, 0.34, 0.58)
            }
            TerrainMaterialId::WaterOceanDeep => {
                ("water_ocean_deep", true, 0.40, 0.60, 1.0, 0.46, 0.22)
            }
            TerrainMaterialId::WaterRiver => ("water_river", true, 0.30, 0.9, 1.0, 0.14, 0.34),
            TerrainMaterialId::Foam => ("shore_foam", true, 0.42, 1.0, 1.0, 0.06, 1.0),
        };
        Self {
            id,
            code,
            animated,
            blend_softness,
            detail_scale,
            wetness_response: wetness,
            reflection_strength: reflection,
            foam_strength: foam,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainRenderRecord {
    pub world_x: i32,
    pub world_y: i32,
    pub tile: TileKind,
    pub shape: TerrainShape,
    pub primary_material: TerrainMaterialId,
    pub secondary_material: Option<TerrainMaterialId>,
    pub transition_mask: u8,
    pub water_mask: Option<WaterRenderMask>,
    pub elevation: f32,
    pub moisture: f32,
}

pub fn resolve_material(tile: TileKind, biome: SceneBiome) -> TerrainMaterialId {
    match tile {
        TileKind::Grass | TileKind::TallGrass => match biome {
            SceneBiome::Highlands => TerrainMaterialId::GrassHighland,
            _ => TerrainMaterialId::GrassTemperate,
        },
        TileKind::Sand => TerrainMaterialId::SandDry,
        TileKind::WetSand => TerrainMaterialId::SandWet,
        TileKind::PebbleShore => TerrainMaterialId::PebbleShore,
        TileKind::Dirt | TileKind::TilledSoil => TerrainMaterialId::SoilLoam,
        TileKind::WateredSoil => TerrainMaterialId::SoilWatered,
        TileKind::MudBank => TerrainMaterialId::Mud,
        TileKind::MountainRock | TileKind::Cliff => TerrainMaterialId::MountainRock,
        TileKind::Road | TileKind::StonePath | TileKind::MountainPath => TerrainMaterialId::Road,
        TileKind::WoodFloor | TileKind::PlankFloor | TileKind::Bridge => TerrainMaterialId::Wood,
        TileKind::CaveFloor | TileKind::CaveWall => TerrainMaterialId::Cave,
        TileKind::ShallowWater | TileKind::Water => TerrainMaterialId::WaterFreshShallow,
        TileKind::DeepWater => TerrainMaterialId::WaterFreshDeep,
        TileKind::OceanShallow => TerrainMaterialId::WaterOceanShallow,
        TileKind::OceanDeep => TerrainMaterialId::WaterOceanDeep,
        TileKind::RiverWater | TileKind::RiverMouthBlend => TerrainMaterialId::WaterRiver,
        TileKind::ShoreFoam => TerrainMaterialId::Foam,
        _ => TerrainMaterialId::Stone,
    }
}

pub const fn resolve_shape(group: Option<TileAutoGroup>, mask: u8) -> TerrainShape {
    if group.is_none() && mask == 0 {
        TerrainShape::Full
    } else {
        TerrainShape::from_cardinal_mask(mask)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_and_material_are_independent() {
        let shape = TerrainShape::from_cardinal_mask(0x3);
        assert_eq!(shape, TerrainShape::OuterNorthEast);
        assert_ne!(
            resolve_material(TileKind::Sand, SceneBiome::Coastal),
            resolve_material(TileKind::Grass, SceneBiome::Coastal)
        );
    }

    #[test]
    fn water_families_resolve_explicitly() {
        assert_eq!(
            resolve_material(TileKind::OceanDeep, SceneBiome::Coastal),
            TerrainMaterialId::WaterOceanDeep
        );
        assert!(TerrainMaterialProfile::for_id(TerrainMaterialId::WaterOceanDeep).animated);
    }
}
