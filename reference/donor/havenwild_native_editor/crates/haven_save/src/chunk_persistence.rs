use crate::{
    AnchorOverrideDelta, CropStateDelta, DomainVersion, HarvestStateDelta, MachineStateDelta,
    NpcStateDelta, OwnershipStateDelta, PlacedObjectDelta, RemovedObjectDelta, ScenePortalDelta,
    StructureOverrideDelta, TerrainOverrideDelta, TerrainPlayerDelta, TimerDelta,
};
use haven_world::{open_world::ChunkCoord, WorldTopologyConfig};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CHUNK_PERSISTENCE_SCHEMA: &str = "havenwild.chunk_persistence.v1";
pub const CHUNK_MANIFEST_SCHEMA: &str = "havenwild.chunk_manifest.v1";
pub const CURRENT_CHUNK_PERSISTENCE_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CanonicalChunkKey {
    pub x: i32,
    pub y: i32,
}
impl CanonicalChunkKey {
    pub fn new(topology: &WorldTopologyConfig, x: i32, y: i32) -> Result<Self, String> {
        let chunk = topology
            .canonical_chunk(ChunkCoord::new(x, y))
            .ok_or_else(|| "chunk coordinate is outside bounded world height".to_string())?;
        Ok(Self {
            x: chunk.x,
            y: chunk.y,
        })
    }
    pub fn directory_name(self) -> String {
        format!("x_{}_y_{}", self.x, self.y)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkBaselineReference {
    pub world_seed: u64,
    pub generator_version: DomainVersion,
    pub pipeline_version: DomainVersion,
    pub terrain_registry_version: DomainVersion,
    pub baseline_hash: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkAuthoredOverrides {
    pub terrain_overrides: Vec<TerrainOverrideDelta>,
    pub anchor_overrides: Vec<AnchorOverrideDelta>,
    pub structure_overrides: Vec<StructureOverrideDelta>,
    pub scene_portals: Vec<ScenePortalDelta>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkPlayerDeltas {
    pub terrain_deltas: Vec<TerrainPlayerDelta>,
    pub placed_objects: Vec<PlacedObjectDelta>,
    pub removed_objects: Vec<RemovedObjectDelta>,
    pub harvest_state: Vec<HarvestStateDelta>,
    pub ownership_state: Vec<OwnershipStateDelta>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkPersistentSimulation {
    pub npc_state: Vec<NpcStateDelta>,
    pub crop_state: Vec<CropStateDelta>,
    pub machine_state: Vec<MachineStateDelta>,
    pub timers: Vec<TimerDelta>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkPersistenceEnvelope {
    pub schema: String,
    pub version: u32,
    pub key: CanonicalChunkKey,
    pub baseline: ChunkBaselineReference,
    #[serde(default)]
    pub authored: ChunkAuthoredOverrides,
    #[serde(default)]
    pub player: ChunkPlayerDeltas,
    #[serde(default)]
    pub simulation: ChunkPersistentSimulation,
}
impl ChunkPersistenceEnvelope {
    pub fn validate(&self, topology: &WorldTopologyConfig) -> Result<(), String> {
        if self.schema != CHUNK_PERSISTENCE_SCHEMA
            || self.version != CURRENT_CHUNK_PERSISTENCE_VERSION
        {
            return Err("unsupported chunk persistence schema/version".into());
        }
        if self.key != CanonicalChunkKey::new(topology, self.key.x, self.key.y)? {
            return Err("chunk key is not canonical".into());
        }
        if self.baseline.world_seed == 0 || self.baseline.generator_version.get() == 0 {
            return Err("baseline seed and generator version must be positive".into());
        }
        if self.baseline.baseline_hash.trim().is_empty() {
            return Err("baseline hash is required".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkManifest {
    pub schema: String,
    pub version: u32,
    pub world_seed: u64,
    pub generator_version: DomainVersion,
    pub chunks: BTreeMap<String, CanonicalChunkKey>,
}
impl ChunkManifest {
    pub fn new(world_seed: u64, generator_version: u32) -> Self {
        Self {
            schema: CHUNK_MANIFEST_SCHEMA.into(),
            version: 1,
            world_seed,
            generator_version: DomainVersion::new(generator_version),
            chunks: BTreeMap::new(),
        }
    }
    pub fn register(&mut self, key: CanonicalChunkKey) {
        self.chunks.insert(key.directory_name(), key);
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CHUNK_MANIFEST_SCHEMA || self.version != 1 {
            return Err("unsupported chunk manifest schema/version".into());
        }
        if self.world_seed == 0 || self.generator_version.get() == 0 {
            return Err("manifest seed and generator version must be positive".into());
        }
        if self
            .chunks
            .iter()
            .any(|(name, key)| name != &key.directory_name())
        {
            return Err("chunk manifest key drift".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wrapped_keys_are_canonical() {
        let t = WorldTopologyConfig {
            schema: haven_world::WORLD_TOPOLOGY_SCHEMA.into(),
            extent: haven_world::WorldTopologyExtent::Finite,
            width_tiles: 1024,
            height_tiles: 512,
            origin_x_tiles: 0,
            origin_y_tiles: 0,
            chunk_size_tiles: 64,
            horizontal_wrap: haven_world::HorizontalWrapMode::EastWest,
            vertical_boundary: haven_world::VerticalBoundaryMode::Clamped,
        };
        let width = t.chunk_count_x();
        let a = CanonicalChunkKey::new(&t, -1, 0).unwrap();
        let b = CanonicalChunkKey::new(&t, width - 1, 0).unwrap();
        assert_eq!(a, b);
    }
    #[test]
    fn transient_and_replication_state_are_not_serialized() {
        let src = include_str!("../../../content/worldgen/chunk_persistence_contract_v1.json");
        assert!(src.contains("runtimeTransientNeverSaved"));
        assert!(src.contains("replicationStateNeverAuthoritativeOnDisk"));
    }
}
