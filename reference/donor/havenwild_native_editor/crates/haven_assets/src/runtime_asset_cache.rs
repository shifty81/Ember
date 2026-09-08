use crate::asset_pack::AssetCategory;
use crate::asset_pack::{AssetPackRegistry, AssetSourceKind, StableAssetRef};
use crate::asset_pack_discovery::{
    load_project_asset_packs, AssetPackDiscovery, PackDiscoveryReport,
};
use crate::semantic_asset_resolution::{
    AssetResolutionContext, SemanticAssetRequest, SemanticAssetResolver,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedAssetSource {
    pub stable_ref: StableAssetRef,
    pub source_kind: AssetSourceKind,
    pub source_path: PathBuf,
}

#[derive(Default)]
pub struct StableAssetSourceCache {
    entries: BTreeMap<StableAssetRef, ResolvedAssetSource>,
}

impl StableAssetSourceCache {
    pub fn build(project_root: &Path, registry: &AssetPackRegistry) -> Self {
        let mut cache = Self::default();
        for pack in registry.mounted_packs() {
            for asset in &pack.assets {
                let Some(source) = pack
                    .sources
                    .iter()
                    .find(|source| source.id == asset.source_id)
                else {
                    continue;
                };
                let stable_ref = StableAssetRef {
                    pack_id: pack.id.clone(),
                    category: asset.category.clone(),
                    asset_id: asset.id.clone(),
                    source_id: asset.source_id.clone(),
                    variant_id: None,
                };
                cache.entries.insert(
                    stable_ref.clone(),
                    ResolvedAssetSource {
                        stable_ref,
                        source_kind: source.kind.clone(),
                        source_path: resolve_source_path(project_root, &source.path),
                    },
                );
            }
        }
        cache
    }

    pub fn get(&self, stable_ref: &StableAssetRef) -> Option<&ResolvedAssetSource> {
        self.entries.get(stable_ref)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub struct RuntimeAssetSession {
    pub registry: AssetPackRegistry,
    pub discovery: PackDiscoveryReport,
    pub sources: StableAssetSourceCache,
}

impl RuntimeAssetSession {
    pub fn discover(project_root: &Path) -> Result<Self, PackDiscoveryReport> {
        let (registry, discovery) = load_project_asset_packs(project_root, false)?;
        let sources = StableAssetSourceCache::build(project_root, &registry);
        Ok(Self {
            registry,
            discovery,
            sources,
        })
    }

    /// Mount every valid production pack while retaining diagnostics for invalid,
    /// duplicate, or machine-local optional packs. This is the correct path for
    /// editor catalogs and runtime fallbacks: one broken user/mod pack must not
    /// hide Havenwild's built-in terrain, cliff, stamp, or placeable providers.
    pub fn discover_tolerant(project_root: &Path) -> Self {
        let discovery_driver = AssetPackDiscovery::project_default(project_root);
        let mut registry = AssetPackRegistry::default();
        let discovery = discovery_driver.discover_and_mount(&mut registry, false);
        let sources = StableAssetSourceCache::build(project_root, &registry);
        Self {
            registry,
            discovery,
            sources,
        }
    }

    pub fn source_for_ref(&self, stable_ref: &StableAssetRef) -> Option<&ResolvedAssetSource> {
        if let Some(source) = self.sources.get(stable_ref) {
            return Some(source);
        }
        let mut normalized = stable_ref.clone();
        normalized.variant_id = None;
        self.sources.get(&normalized)
    }

    pub fn resolve_source(
        &self,
        semantic_id: &str,
        category: AssetCategory,
        context: AssetResolutionContext,
    ) -> Option<&ResolvedAssetSource> {
        let result = SemanticAssetResolver::new(&self.registry).resolve(&SemanticAssetRequest {
            semantic_id: semantic_id.to_string(),
            category,
            context,
        });
        self.sources.get(result.selected.as_ref()?)
    }
}

fn resolve_source_path(project_root: &Path, source_path: &str) -> PathBuf {
    let path = Path::new(source_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        project_root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("havenwild-runtime-assets-{nonce}"))
    }

    #[test]
    fn relative_pack_sources_resolve_from_project_root() {
        let root = Path::new("C:/havenwild");
        assert_eq!(
            resolve_source_path(root, "assets/example.png"),
            root.join("assets/example.png")
        );
    }

    #[test]
    fn tolerant_discovery_keeps_valid_packs_when_an_optional_pack_is_invalid() {
        let root = temporary_root();
        let core = root.join("content/asset_packs/core");
        let broken = root.join("user/asset_packs/broken");
        fs::create_dir_all(&core).expect("create core pack directory");
        fs::create_dir_all(&broken).expect("create broken pack directory");
        fs::write(
            core.join("pack.json"),
            r#"{
  "schema": "havenwild.asset_pack.v1",
  "id": "core",
  "display_name": "Core",
  "version": "1",
  "license": {
    "license_id": "CC0-1.0",
    "production_approved": true,
    "attribution": [],
    "commercial_use": true,
    "redistribution": true
  },
  "production_enabled": true,
  "dependencies": [],
  "sources": [
    {
      "id": "sheet",
      "kind": "raw_sheet",
      "path": "content/core.png",
      "tile_size": [32, 32],
      "margin": 0,
      "spacing": 0
    }
  ],
  "assets": [
    {
      "id": "grass",
      "category": "terrain",
      "semantic_id": "terrain.grass",
      "source_id": "sheet",
      "variants": [],
      "tags": [],
      "metadata": {}
    }
  ],
  "priority": 10
}"#,
        )
        .expect("write core pack");
        fs::write(broken.join("pack.json"), "{ invalid json").expect("write invalid optional pack");

        assert!(RuntimeAssetSession::discover(&root).is_err());
        let session = RuntimeAssetSession::discover_tolerant(&root);
        assert_eq!(session.discovery.failed_count(), 1);
        assert_eq!(session.registry.mounted_pack_count(), 1);
        assert_eq!(session.sources.len(), 1);

        fs::remove_dir_all(root).ok();
    }
}
