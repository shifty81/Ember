use haven_core::TileKind;

/// Canonical semantic role used by worldgen, editor palettes, and runtime
/// consumers. Presentation-only legacy tiles never become floor authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainIdentityV2 {
    SurfaceFloor,
    LiquidSurface,
    DerivedStructure,
    DerivedOverlay,
    InteriorStructure,
    ObjectOrCrop,
}

pub const fn terrain_identity_v2(tile: TileKind) -> TerrainIdentityV2 {
    match tile {
        TileKind::Grass
        | TileKind::Sand
        | TileKind::PebbleShore
        | TileKind::Road
        | TileKind::StonePath
        | TileKind::MountainPath
        | TileKind::MountainRock
        | TileKind::Dirt
        | TileKind::WoodFloor
        | TileKind::PlankFloor
        | TileKind::StoneFloor
        | TileKind::BrickFloor
        | TileKind::Bridge
        | TileKind::CaveFloor
        | TileKind::TilledSoil
        | TileKind::WateredSoil
        | TileKind::MudBank => TerrainIdentityV2::SurfaceFloor,
        TileKind::Water
        | TileKind::ShallowWater
        | TileKind::DeepWater
        | TileKind::OceanDeep
        | TileKind::OceanShallow
        | TileKind::RiverWater
        | TileKind::RiverMouthBlend => TerrainIdentityV2::LiquidSurface,
        TileKind::Cliff => TerrainIdentityV2::DerivedStructure,
        TileKind::WetSand | TileKind::ShoreFoam | TileKind::GreenhouseZone => {
            TerrainIdentityV2::DerivedOverlay
        }
        TileKind::Wall | TileKind::CaveWall => TerrainIdentityV2::InteriorStructure,
        TileKind::TallGrass | TileKind::Crop => TerrainIdentityV2::ObjectOrCrop,
    }
}

pub const fn is_paintable_surface_v2(tile: TileKind) -> bool {
    !matches!(tile, TileKind::WateredSoil)
        && matches!(
            terrain_identity_v2(tile),
            TerrainIdentityV2::SurfaceFloor | TerrainIdentityV2::LiquidSurface
        )
}

pub const fn is_legacy_derived_cliff_tile_v2(tile: TileKind) -> bool {
    matches!(tile, TileKind::Cliff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mountain_rock_is_walkable_floor_authority() {
        assert_eq!(
            terrain_identity_v2(TileKind::MountainRock),
            TerrainIdentityV2::SurfaceFloor
        );
        assert!(TileKind::MountainRock.walkable());
        assert!(TileKind::MountainRock.is_production_terrain_brush());
    }

    #[test]
    fn mud_is_paintable_surface_and_watered_soil_is_state_only() {
        assert_eq!(
            terrain_identity_v2(TileKind::MudBank),
            TerrainIdentityV2::SurfaceFloor
        );
        assert!(is_paintable_surface_v2(TileKind::MudBank));
        assert!(!is_paintable_surface_v2(TileKind::WateredSoil));
        assert!(!TileKind::WateredSoil.is_production_terrain_brush());
    }

    #[test]
    fn cliff_is_derived_and_not_a_palette_brush() {
        assert_eq!(
            terrain_identity_v2(TileKind::Cliff),
            TerrainIdentityV2::DerivedStructure
        );
        assert!(!TileKind::Cliff.is_production_terrain_brush());
        assert!(!TileKind::LPC_MAPPED_EDITOR_TERRAIN.contains(&TileKind::Cliff));
    }
}
