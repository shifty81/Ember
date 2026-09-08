use serde::{Deserialize, Serialize};

pub const PERSISTENCE_DOMAIN_VERSION_SCHEMA: &str = "havenwild.persistence_domain_versions.v1";

/// Typed numeric version with the same JSON representation as the legacy u32.
/// This lets save formats gain domain ownership without changing existing data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DomainVersion(pub u32);

impl DomainVersion {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u32 {
        self.0
    }

    pub fn validate(self, label: &str) -> Result<(), String> {
        if self.0 == 0 {
            return Err(format!("{label} version must be positive"));
        }
        Ok(())
    }
}

/// Computed domain-version view over today's single generation version.
/// It is deliberately not persisted into v1 save metadata yet, preserving
/// byte/schema compatibility while establishing the ownership boundary for
/// future migrations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistenceDomainVersions {
    pub world: DomainVersion,
    pub terrain: DomainVersion,
    pub ecology: DomainVersion,
    pub settlement: DomainVersion,
    pub simulation: DomainVersion,
    pub runtime_entity: DomainVersion,
}

impl PersistenceDomainVersions {
    pub const fn from_legacy_generation_version(generation_version: u32) -> Self {
        let version = DomainVersion::new(generation_version);
        Self {
            world: version,
            terrain: version,
            ecology: version,
            settlement: version,
            simulation: version,
            runtime_entity: version,
        }
    }

    pub fn validate(self) -> Result<(), String> {
        self.world.validate("world")?;
        self.terrain.validate("terrain")?;
        self.ecology.validate("ecology")?;
        self.settlement.validate("settlement")?;
        self.simulation.validate("simulation")?;
        self.runtime_entity.validate("runtime entity")
    }
}

macro_rules! string_delta_type {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub String);
    };
}

// Existing chunk delta payloads remain strings on disk for compatibility, but
// their Rust types now state which domain owns each mutation lane.
string_delta_type!(TerrainOverrideDelta);
string_delta_type!(AnchorOverrideDelta);
string_delta_type!(StructureOverrideDelta);
string_delta_type!(ScenePortalDelta);
string_delta_type!(TerrainPlayerDelta);
string_delta_type!(PlacedObjectDelta);
string_delta_type!(RemovedObjectDelta);
string_delta_type!(HarvestStateDelta);
string_delta_type!(OwnershipStateDelta);
string_delta_type!(NpcStateDelta);
string_delta_type!(CropStateDelta);
string_delta_type!(MachineStateDelta);
string_delta_type!(TimerDelta);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_versions_keep_legacy_number_shape() {
        assert_eq!(serde_json::to_string(&DomainVersion::new(16)).unwrap(), "16");
        PersistenceDomainVersions::from_legacy_generation_version(16)
            .validate()
            .unwrap();
    }

    #[test]
    fn typed_delta_keeps_legacy_string_shape() {
        assert_eq!(
            serde_json::to_string(&TerrainOverrideDelta("x=1,y=2".into())).unwrap(),
            "\"x=1,y=2\""
        );
    }
}
