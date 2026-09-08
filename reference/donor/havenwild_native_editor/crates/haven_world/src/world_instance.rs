//! Unified world-instance identity and persistence authority.
//!
//! Havenwild should not have separate engines for mainland, Home Estate,
//! adventure worlds, caves, or interiors. They all use the same chunk/terrain/
//! object runtime; this descriptor says what an instance *is*, how large it may
//! be, how it persists, and whether procedural generation may extend it.

use serde::{Deserialize, Serialize};

pub const WORLD_DESCRIPTOR_SCHEMA: &str = "havenwild.world_descriptor.v0_1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldScope {
    SharedSurface,
    HomeEstate,
    Adventure,
    AuthoredInstance,
    Interior,
    Cave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum WorldExtent {
    Finite {
        width_tiles: u32,
        height_tiles: u32,
    },
    Endless {
        /// Macro-geographic planning size. Storage chunks remain the project's
        /// normal chunk size and never become geography boundaries.
        region_size_tiles: u32,
    },
}

impl WorldExtent {
    pub const fn finite(width_tiles: u32, height_tiles: u32) -> Self {
        Self::Finite {
            width_tiles,
            height_tiles,
        }
    }

    pub const fn is_endless(self) -> bool {
        matches!(self, Self::Endless { .. })
    }

    pub const fn finite_dimensions(self) -> Option<[u32; 2]> {
        match self {
            Self::Finite {
                width_tiles,
                height_tiles,
            } => Some([width_tiles, height_tiles]),
            Self::Endless { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistencePolicy {
    Permanent,
    PersistentUntilDeleted,
    Regenerating,
    SessionOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationPolicy {
    /// Generate the initial base once. Player/authored deltas permanently win.
    InitialGenerateOnly,
    /// Deterministically generate new regions/chunks as traversal requires.
    StreamedDeterministic,
    /// Fully authored; generation may only provide explicit requested helpers.
    Authored,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldDescriptor {
    pub schema: String,
    pub world_id: String,
    pub display_name: String,
    pub seed: u64,
    pub scope: WorldScope,
    pub extent: WorldExtent,
    pub persistence: PersistencePolicy,
    pub generation: GenerationPolicy,
    pub chunk_size_tiles: u32,
    /// Optional owner key for instanced private spaces such as Home Estate.
    pub owner_key: Option<String>,
    /// Optional parent/entrance world. Spatial size is intentionally not tied
    /// to this parent, enabling TARDIS-style estates and interiors.
    pub parent_world_id: Option<String>,
}

impl WorldDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        world_id: impl Into<String>,
        display_name: impl Into<String>,
        seed: u64,
        scope: WorldScope,
        extent: WorldExtent,
        persistence: PersistencePolicy,
        generation: GenerationPolicy,
        chunk_size_tiles: u32,
    ) -> Self {
        Self {
            schema: WORLD_DESCRIPTOR_SCHEMA.to_string(),
            world_id: world_id.into(),
            display_name: display_name.into(),
            seed,
            scope,
            extent,
            persistence,
            generation,
            chunk_size_tiles,
            owner_key: None,
            parent_world_id: None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != WORLD_DESCRIPTOR_SCHEMA {
            return Err(format!("unsupported world descriptor schema {}", self.schema));
        }
        if self.world_id.trim().is_empty() || self.display_name.trim().is_empty() {
            return Err("world id and display name must not be empty".to_string());
        }
        if self.seed == 0 || self.chunk_size_tiles == 0 {
            return Err("world seed and chunk size must be greater than zero".to_string());
        }
        match self.extent {
            WorldExtent::Finite {
                width_tiles,
                height_tiles,
            } => {
                if width_tiles == 0 || height_tiles == 0 {
                    return Err("finite world dimensions must be greater than zero".to_string());
                }
                if width_tiles % self.chunk_size_tiles != 0
                    || height_tiles % self.chunk_size_tiles != 0
                {
                    return Err(
                        "finite world dimensions must be divisible by chunk size".to_string(),
                    );
                }
            }
            WorldExtent::Endless { region_size_tiles } => {
                if region_size_tiles < self.chunk_size_tiles
                    || region_size_tiles % self.chunk_size_tiles != 0
                {
                    return Err(
                        "endless geographic region size must be a chunk-aligned multiple"
                            .to_string(),
                    );
                }
            }
        }
        if self.scope == WorldScope::HomeEstate {
            if self.persistence != PersistencePolicy::Permanent {
                return Err("Home Estate instances must be permanent".to_string());
            }
            if self.owner_key.as_deref().is_none_or(str::is_empty) {
                return Err("Home Estate instances require an owner key".to_string());
            }
        }
        Ok(())
    }

    pub fn with_owner(mut self, owner_key: impl Into<String>) -> Self {
        self.owner_key = Some(owner_key.into());
        self
    }

    pub fn with_parent_world(mut self, parent_world_id: impl Into<String>) -> Self {
        self.parent_world_id = Some(parent_world_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permanent_home_estate_is_a_normal_owned_world_instance() {
        let descriptor = WorldDescriptor::new(
            "estate:player-1",
            "Home Estate",
            42,
            WorldScope::HomeEstate,
            WorldExtent::finite(512, 512),
            PersistencePolicy::Permanent,
            GenerationPolicy::InitialGenerateOnly,
            64,
        )
        .with_owner("player-1")
        .with_parent_world("mainland");
        assert!(descriptor.validate().is_ok());
    }

    #[test]
    fn endless_worlds_require_only_region_alignment_not_global_dimensions() {
        let descriptor = WorldDescriptor::new(
            "adventure:endless",
            "Endless Adventure",
            99,
            WorldScope::Adventure,
            WorldExtent::Endless {
                region_size_tiles: 1024,
            },
            PersistencePolicy::PersistentUntilDeleted,
            GenerationPolicy::StreamedDeterministic,
            64,
        );
        assert!(descriptor.validate().is_ok());
        assert!(descriptor.extent.is_endless());
    }
}
