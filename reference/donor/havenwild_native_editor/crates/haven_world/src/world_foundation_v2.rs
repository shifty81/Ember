//! Normalized terrain/world-generation foundation.
//!
//! This module is the future-facing source of truth for generation stages and
//! semantic surface state. Legacy `TileKind` maps remain supported through
//! adapters, but generated atlas tuples and presentation overlays are never
//! persisted as canonical world state here.

use serde::{Deserialize, Serialize};

pub const WORLD_MANIFEST_V2_SCHEMA: &str = "havenwild.world_manifest.v2";
pub const SURFACE_CELL_V1_SCHEMA: &str = "havenwild.surface_cell.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldManifestV2 {
    pub schema: String,
    pub generation_version: u32,
    pub world_seed: u64,
    pub width_chunks: u32,
    pub height_chunks: u32,
    pub chunk_size_tiles: u16,
    pub wrap_east_west: bool,
    pub capital: AuthoredAnchorReservation,
    pub stage_versions: Vec<GenerationStageVersion>,
}

impl WorldManifestV2 {
    pub fn new(
        world_seed: u64,
        width_chunks: u32,
        height_chunks: u32,
        chunk_size_tiles: u16,
    ) -> Self {
        Self {
            schema: WORLD_MANIFEST_V2_SCHEMA.to_owned(),
            generation_version: 2,
            world_seed,
            width_chunks,
            height_chunks,
            chunk_size_tiles,
            wrap_east_west: true,
            capital: AuthoredAnchorReservation::willowmere(),
            stage_versions: GenerationStageId::ORDERED
                .iter()
                .copied()
                .map(|stage| GenerationStageVersion { stage, version: 1 })
                .collect(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != WORLD_MANIFEST_V2_SCHEMA {
            return Err(format!(
                "unsupported world manifest schema: {}",
                self.schema
            ));
        }
        if self.width_chunks == 0 || self.height_chunks == 0 || self.chunk_size_tiles == 0 {
            return Err("world dimensions and chunk size must be non-zero".to_owned());
        }
        if !self.wrap_east_west {
            return Err("Havenwild surface worlds must wrap east/west".to_owned());
        }
        if self.capital.anchor_id != "willowmere" || !self.capital.protected {
            return Err("Willowmere must remain the protected capital anchor".to_owned());
        }
        for stage in GenerationStageId::ORDERED {
            if !self.stage_versions.iter().any(|entry| entry.stage == stage) {
                return Err(format!(
                    "missing generation stage version: {}",
                    stage.label()
                ));
            }
        }
        Ok(())
    }

    pub fn stage_seed(&self, stage: GenerationStageId) -> u64 {
        splitmix64(self.world_seed ^ stage.seed_domain())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoredAnchorReservation {
    pub anchor_id: String,
    pub protected: bool,
    pub requires_hand_authoring_pass: bool,
}

impl AuthoredAnchorReservation {
    pub fn willowmere() -> Self {
        Self {
            anchor_id: "willowmere".to_owned(),
            protected: true,
            requires_hand_authoring_pass: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationStageId {
    Topology,
    CapitalReservation,
    Landmasses,
    ClimateBiomes,
    Elevation,
    Hydrology,
    SurfaceMaterials,
    CliffsStructuralTerrain,
    Underground,
    Entrances,
    RoadsSettlements,
    VegetationResources,
    AuthoredOverlays,
    ValidationBake,
    Persistence,
}

impl GenerationStageId {
    pub const ORDERED: [Self; 15] = [
        Self::Topology,
        Self::CapitalReservation,
        Self::Landmasses,
        Self::ClimateBiomes,
        Self::Elevation,
        Self::Hydrology,
        Self::SurfaceMaterials,
        Self::CliffsStructuralTerrain,
        Self::Underground,
        Self::Entrances,
        Self::RoadsSettlements,
        Self::VegetationResources,
        Self::AuthoredOverlays,
        Self::ValidationBake,
        Self::Persistence,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Topology => "topology",
            Self::CapitalReservation => "capital_reservation",
            Self::Landmasses => "landmasses",
            Self::ClimateBiomes => "climate_biomes",
            Self::Elevation => "elevation",
            Self::Hydrology => "hydrology",
            Self::SurfaceMaterials => "surface_materials",
            Self::CliffsStructuralTerrain => "cliffs_structural_terrain",
            Self::Underground => "underground",
            Self::Entrances => "entrances",
            Self::RoadsSettlements => "roads_settlements",
            Self::VegetationResources => "vegetation_resources",
            Self::AuthoredOverlays => "authored_overlays",
            Self::ValidationBake => "validation_bake",
            Self::Persistence => "persistence",
        }
    }

    pub const fn seed_domain(self) -> u64 {
        match self {
            Self::Topology => 0x5a4f_4e45_0000_0001,
            Self::CapitalReservation => 0x5a4f_4e45_0000_0002,
            Self::Landmasses => 0x5a4f_4e45_0000_0003,
            Self::ClimateBiomes => 0x5a4f_4e45_0000_0004,
            Self::Elevation => 0x5a4f_4e45_0000_0005,
            Self::Hydrology => 0x5a4f_4e45_0000_0006,
            Self::SurfaceMaterials => 0x5a4f_4e45_0000_0007,
            Self::CliffsStructuralTerrain => 0x5a4f_4e45_0000_0008,
            Self::Underground => 0x5a4f_4e45_0000_0009,
            Self::Entrances => 0x5a4f_4e45_0000_000a,
            Self::RoadsSettlements => 0x5a4f_4e45_0000_000b,
            Self::VegetationResources => 0x5a4f_4e45_0000_000c,
            Self::AuthoredOverlays => 0x5a4f_4e45_0000_000d,
            Self::ValidationBake => 0x5a4f_4e45_0000_000e,
            Self::Persistence => 0x5a4f_4e45_0000_000f,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationStageVersion {
    pub stage: GenerationStageId,
    pub version: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceMaterialV1 {
    Grass,
    TallGrass,
    Dirt,
    Sand,
    WetSand,
    Pebble,
    Mud,
    Rock,
    CultivatedSoil,
    Constructed,
    Cave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaterKindV1 {
    None,
    Fresh,
    River,
    Ocean,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaterDepthV1 {
    Dry,
    Wading,
    Shallow,
    Deep,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreezeStateV1 {
    Liquid,
    SkimIce,
    Frozen,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CliffTopologyV1 {
    None,
    North,
    East,
    South,
    West,
    InnerCornerNe,
    InnerCornerSe,
    InnerCornerSw,
    InnerCornerNw,
    OuterCornerNe,
    OuterCornerSe,
    OuterCornerSw,
    OuterCornerNw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceCellV1 {
    pub material: SurfaceMaterialV1,
    pub elevation: i16,
    pub water_kind: WaterKindV1,
    pub water_depth: WaterDepthV1,
    pub water_body_id: Option<u64>,
    pub flow_x_milli: i16,
    pub flow_y_milli: i16,
    pub temperature_tenths_c: i16,
    pub moisture: u8,
    pub cliff: CliffTopologyV1,
    pub road: bool,
    pub cultivated: bool,
    pub protected_authored: bool,
    pub generation_stage: GenerationStageId,
    pub freeze_state: FreezeStateV1,
}

impl SurfaceCellV1 {
    pub fn dry(material: SurfaceMaterialV1, elevation: i16, stage: GenerationStageId) -> Self {
        Self {
            material,
            elevation,
            water_kind: WaterKindV1::None,
            water_depth: WaterDepthV1::Dry,
            water_body_id: None,
            flow_x_milli: 0,
            flow_y_milli: 0,
            temperature_tenths_c: 150,
            moisture: 64,
            cliff: CliffTopologyV1::None,
            road: false,
            cultivated: false,
            protected_authored: false,
            generation_stage: stage,
            freeze_state: FreezeStateV1::Liquid,
        }
    }

    pub fn is_water(self) -> bool {
        self.water_kind != WaterKindV1::None
    }
    pub fn is_swimmable(self) -> bool {
        self.is_water()
            && self.water_depth == WaterDepthV1::Deep
            && self.freeze_state != FreezeStateV1::Frozen
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirtyRegion {
    pub min_x: i32,
    pub min_y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl DirtyRegion {
    pub fn expanded(self, radius: i32) -> Self {
        Self {
            min_x: self.min_x - radius,
            min_y: self.min_y - radius,
            max_x: self.max_x + radius,
            max_y: self.max_y + radius,
        }
    }
}

pub trait GenerationStage {
    fn id(&self) -> GenerationStageId;
    fn execute(
        &mut self,
        context: &mut WorldGenerationContextV2,
    ) -> Result<GenerationStageReport, String>;
}

pub struct WorldGenerationContextV2 {
    pub manifest: WorldManifestV2,
    pub completed_stages: Vec<GenerationStageId>,
}

impl WorldGenerationContextV2 {
    pub fn new(manifest: WorldManifestV2) -> Result<Self, String> {
        manifest.validate()?;
        Ok(Self {
            manifest,
            completed_stages: Vec::new(),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenerationStageReport {
    pub stage: GenerationStageId,
    pub changed_cells: usize,
    pub warnings: Vec<String>,
}

pub struct WorldGenerationExecutor {
    stages: Vec<Box<dyn GenerationStage>>,
}

impl WorldGenerationExecutor {
    pub fn new(stages: Vec<Box<dyn GenerationStage>>) -> Result<Self, String> {
        let ids: Vec<_> = stages.iter().map(|stage| stage.id()).collect();
        let expected_prefix = &GenerationStageId::ORDERED[..ids.len()];
        if ids.as_slice() != expected_prefix {
            return Err(
                "generation stages must be registered in canonical order without gaps".to_owned(),
            );
        }
        Ok(Self { stages })
    }

    pub fn execute(
        &mut self,
        context: &mut WorldGenerationContextV2,
    ) -> Result<Vec<GenerationStageReport>, String> {
        let mut reports = Vec::with_capacity(self.stages.len());
        for stage in &mut self.stages {
            let report = stage.execute(context)?;
            if report.stage != stage.id() {
                return Err("generation stage returned a mismatched report id".to_owned());
            }
            context.completed_stages.push(stage.id());
            reports.push(report);
        }
        Ok(reports)
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_seeds_are_stable_and_separated() {
        let manifest = WorldManifestV2::new(42, 128, 64, 32);
        assert_ne!(
            manifest.stage_seed(GenerationStageId::Hydrology),
            manifest.stage_seed(GenerationStageId::VegetationResources)
        );
        assert_eq!(
            manifest.stage_seed(GenerationStageId::Hydrology),
            WorldManifestV2::new(42, 128, 64, 32).stage_seed(GenerationStageId::Hydrology)
        );
    }

    #[test]
    fn willowmere_is_always_protected() {
        let manifest = WorldManifestV2::new(7, 128, 64, 32);
        assert_eq!(manifest.capital.anchor_id, "willowmere");
        assert!(manifest.capital.protected);
        assert!(manifest.capital.requires_hand_authoring_pass);
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn deep_liquid_water_is_swimmable() {
        let mut cell = SurfaceCellV1::dry(SurfaceMaterialV1::Sand, 0, GenerationStageId::Hydrology);
        cell.water_kind = WaterKindV1::Fresh;
        cell.water_depth = WaterDepthV1::Deep;
        assert!(cell.is_swimmable());
        cell.freeze_state = FreezeStateV1::Frozen;
        assert!(!cell.is_swimmable());
    }
}
