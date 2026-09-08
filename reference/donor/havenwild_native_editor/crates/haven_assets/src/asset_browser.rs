use crate::asset_pack::{
    AssetCategory, AssetPackId, AssetPackManifest, AssetSourceKind, StableAssetRef,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssetReadiness {
    ManualOnly,
    PartiallyConfigured,
    RuntimeReady,
    ProductionVerified,
    ReferenceOnly,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetBrowserEntry {
    pub stable_ref: StableAssetRef,
    pub pack_display_name: String,
    pub pack_priority: i32,
    pub semantic_id: String,
    pub source_path: String,
    pub source_kind: AssetSourceKind,
    pub tags: BTreeSet<String>,
    pub readiness: AssetReadiness,
    pub license_id: String,
    pub production_enabled: bool,
    pub production_approved: bool,
    #[serde(default)]
    pub placeable_stable_id: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetPackBrowserSummary {
    pub pack_id: String,
    pub display_name: String,
    pub version: String,
    pub priority: i32,
    pub production_enabled: bool,
    pub production_approved: bool,
    pub license_id: String,
    pub categories: BTreeSet<AssetCategory>,
    pub asset_count: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetBrowserSnapshot {
    pub packs: Vec<AssetPackBrowserSummary>,
    pub entries: Vec<AssetBrowserEntry>,
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetBrowserFilter {
    pub query: String,
    pub pack_id: Option<String>,
    pub category: Option<AssetCategory>,
    pub tag: Option<String>,
    pub production_only: bool,
    pub approved_license_only: bool,
    pub runtime_ready_only: bool,
}

impl AssetBrowserSnapshot {
    pub fn load_project(project_root: &Path) -> Self {
        let mut snapshot = Self::default();
        for root in [
            project_root.join("content/asset_packs"),
            project_root.join("user/asset_packs"),
            project_root.join("mods/asset_packs"),
        ] {
            snapshot.collect_root(&root);
        }
        snapshot.collect_lpc_mapping_queues(project_root);
        snapshot.packs.sort_by(|left, right| {
            right
                .priority
                .cmp(&left.priority)
                .then_with(|| left.display_name.cmp(&right.display_name))
        });
        snapshot.entries.sort_by(|left, right| {
            right
                .pack_priority
                .cmp(&left.pack_priority)
                .then_with(|| left.semantic_id.cmp(&right.semantic_id))
                .then_with(|| left.stable_ref.asset_id.0.cmp(&right.stable_ref.asset_id.0))
        });
        snapshot
    }

    pub fn filtered<'a>(&'a self, filter: &AssetBrowserFilter) -> Vec<&'a AssetBrowserEntry> {
        let query = filter.query.trim().to_ascii_lowercase();
        let tag = filter.tag.as_ref().map(|value| value.to_ascii_lowercase());
        self.entries
            .iter()
            .filter(|entry| {
                filter
                    .pack_id
                    .as_ref()
                    .is_none_or(|pack| entry.stable_ref.pack_id.0 == *pack)
                    && filter
                        .category
                        .as_ref()
                        .is_none_or(|category| entry.stable_ref.category == *category)
                    && tag.as_ref().is_none_or(|wanted| {
                        entry
                            .tags
                            .iter()
                            .any(|candidate| candidate.to_ascii_lowercase() == *wanted)
                    })
                    && (!filter.production_only || entry.production_enabled)
                    && (!filter.approved_license_only || entry.production_approved)
                    && (!filter.runtime_ready_only
                        || matches!(
                            entry.readiness,
                            AssetReadiness::RuntimeReady | AssetReadiness::ProductionVerified
                        ))
                    && (query.is_empty()
                        || entry.semantic_id.to_ascii_lowercase().contains(&query)
                        || entry
                            .stable_ref
                            .asset_id
                            .0
                            .to_ascii_lowercase()
                            .contains(&query)
                        || entry
                            .pack_display_name
                            .to_ascii_lowercase()
                            .contains(&query)
                        || entry
                            .tags
                            .iter()
                            .any(|candidate| candidate.to_ascii_lowercase().contains(&query)))
            })
            .collect()
    }

    pub fn categories(&self) -> BTreeSet<AssetCategory> {
        self.entries
            .iter()
            .map(|entry| entry.stable_ref.category.clone())
            .collect()
    }

    pub fn tags(&self) -> BTreeSet<String> {
        self.entries
            .iter()
            .flat_map(|entry| entry.tags.iter().cloned())
            .collect()
    }

    pub fn summary(&self) -> String {
        format!(
            "{} packs | {} assets | {} diagnostic{}",
            self.packs.len(),
            self.entries.len(),
            self.diagnostics.len(),
            if self.diagnostics.len() == 1 { "" } else { "s" }
        )
    }

    fn collect_lpc_mapping_queues(&mut self, project_root: &Path) {
        let path = project_root.join("WORKSPACE/generated/lpc_revised_mapping_queues_v1.json");
        let queues = match crate::lpc_mapping_queue::LpcMappingQueues::load(&path) {
            Ok(queues) => queues,
            Err(error) => {
                self.diagnostics.push(format!(
                    "LPC mapping queues unavailable; run Build-LpcMappingQueuesV148Y.py: {error}"
                ));
                return;
            }
        };
        for item in queues.items() {
            let mut tags: BTreeSet<String> = item.tags.iter().cloned().collect();
            tags.insert("lpc_revised".to_string());
            tags.insert(format!("queue:{}", item.queue));
            tags.insert(format!("promotion:{}", item.promotion_state));
            tags.insert(format!("action:{}", item.recommended_action));
            self.entries.push(AssetBrowserEntry {
                stable_ref: item.stable_ref(),
                pack_display_name: "LPC Revised Mapping Queue".to_string(),
                pack_priority: 49,
                semantic_id: item.proposed_semantic_id.clone(),
                source_path: format!("assets/source/licensed/lpc_revised/{}", item.relative_path),
                source_kind: source_kind_for_queue(&item.source_kind),
                tags,
                readiness: AssetReadiness::ReferenceOnly,
                license_id: if item.license_evidence.is_empty() {
                    "license-evidence-missing".to_string()
                } else {
                    "source-specific-review".to_string()
                },
                production_enabled: false,
                production_approved: false,
                placeable_stable_id: None,
            });
        }
    }

    fn collect_root(&mut self, root: &Path) {
        if !root.is_dir() {
            return;
        }
        let mut stack = vec![root.to_path_buf()];
        while let Some(directory) = stack.pop() {
            let entries = match fs::read_dir(&directory) {
                Ok(entries) => entries,
                Err(error) => {
                    self.diagnostics.push(format!(
                        "unable to inspect asset-pack directory {}: {error}",
                        directory.display()
                    ));
                    continue;
                }
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.file_name().and_then(|name| name.to_str()) == Some("pack.json") {
                    self.collect_manifest(path);
                }
            }
        }
    }

    fn collect_manifest(&mut self, path: PathBuf) {
        let manifest: AssetPackManifest = match fs::read(&path)
            .map_err(|error| error.to_string())
            .and_then(|bytes| serde_json::from_slice(&bytes).map_err(|error| error.to_string()))
        {
            Ok(manifest) => manifest,
            Err(error) => {
                self.diagnostics
                    .push(format!("unable to read {}: {error}", path.display()));
                return;
            }
        };
        if let Err(errors) = manifest.validate() {
            self.diagnostics.extend(
                errors
                    .into_iter()
                    .map(|error| format!("{}: {error}", path.display())),
            );
            return;
        }

        let source_by_id: BTreeMap<_, _> = manifest
            .sources
            .iter()
            .map(|source| (source.id.clone(), source))
            .collect();
        let categories: BTreeSet<_> = manifest
            .assets
            .iter()
            .map(|asset| asset.category.clone())
            .collect();
        self.packs.push(AssetPackBrowserSummary {
            pack_id: manifest.id.0.clone(),
            display_name: manifest.display_name.clone(),
            version: manifest.version.clone(),
            priority: manifest.priority,
            production_enabled: manifest.production_enabled,
            production_approved: manifest.license.production_approved,
            license_id: manifest.license.license_id.clone(),
            categories,
            asset_count: manifest.assets.len(),
        });

        for asset in &manifest.assets {
            let Some(source) = source_by_id.get(&asset.source_id) else {
                self.diagnostics.push(format!(
                    "{}:{} references missing source {}",
                    manifest.id.0, asset.id.0, asset.source_id.0
                ));
                continue;
            };
            let readiness = readiness_for(&manifest, asset.metadata.get("readiness"));
            self.entries.push(AssetBrowserEntry {
                stable_ref: StableAssetRef {
                    pack_id: AssetPackId(manifest.id.0.clone()),
                    category: asset.category.clone(),
                    asset_id: asset.id.clone(),
                    source_id: asset.source_id.clone(),
                    variant_id: None,
                },
                pack_display_name: manifest.display_name.clone(),
                pack_priority: manifest.priority,
                semantic_id: asset.semantic_id.clone(),
                source_path: source.path.clone(),
                source_kind: source.kind.clone(),
                tags: asset.tags.clone(),
                readiness,
                license_id: manifest.license.license_id.clone(),
                production_enabled: manifest.production_enabled,
                production_approved: manifest.license.production_approved,
                placeable_stable_id: None,
            });
        }

        for asset in manifest.assets.iter().filter(|asset| {
            asset.category == AssetCategory::EditorTemplate
                && asset.semantic_id.starts_with("placeable.catalog.")
        }) {
            let Some(source) = source_by_id.get(&asset.source_id) else {
                continue;
            };
            let source_path = PathBuf::from(&source.path);
            let Ok(bytes) = fs::read(&source_path) else {
                continue;
            };
            let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                continue;
            };
            let Some(entries) = value.get("entries").and_then(|value| value.as_array()) else {
                continue;
            };
            for entry in entries {
                let Some(id) = entry.get("id").and_then(|value| value.as_str()) else {
                    continue;
                };
                let Some(semantic_id) = entry.get("semantic_id").and_then(|value| value.as_str())
                else {
                    continue;
                };
                let category = entry
                    .get("category")
                    .cloned()
                    .and_then(|value| serde_json::from_value::<AssetCategory>(value).ok())
                    .unwrap_or(AssetCategory::Other);
                let stable_id = format!("{}::{id}", manifest.id.0);
                let mut tags = BTreeSet::new();
                tags.insert("placeable".to_string());
                if let Some(values) = entry
                    .get("placement_tags")
                    .and_then(|value| value.as_array())
                {
                    tags.extend(
                        values
                            .iter()
                            .filter_map(|value| value.as_str().map(str::to_string)),
                    );
                }
                self.entries.push(AssetBrowserEntry {
                    stable_ref: StableAssetRef {
                        pack_id: AssetPackId(manifest.id.0.clone()),
                        category,
                        asset_id: crate::asset_pack::AssetId(id.to_string()),
                        source_id: asset.source_id.clone(),
                        variant_id: None,
                    },
                    pack_display_name: manifest.display_name.clone(),
                    pack_priority: manifest.priority,
                    semantic_id: semantic_id.to_string(),
                    source_path: source.path.clone(),
                    source_kind: source.kind.clone(),
                    tags,
                    readiness: readiness_for(&manifest, entry.get("readiness")),
                    license_id: manifest.license.license_id.clone(),
                    production_enabled: manifest.production_enabled,
                    production_approved: manifest.license.production_approved,
                    placeable_stable_id: Some(stable_id),
                });
            }
        }
    }
}

fn source_kind_for_queue(value: &str) -> AssetSourceKind {
    match value {
        "image" => AssetSourceKind::Image,
        "audio" => AssetSourceKind::Audio,
        "tiled_tileset" => AssetSourceKind::TiledTileset,
        "sidecar" | "tiled_map" | "documentation" | "license" => AssetSourceKind::Sidecar,
        _ => AssetSourceKind::ExternalDependency,
    }
}

fn readiness_for(
    manifest: &AssetPackManifest,
    value: Option<&serde_json::Value>,
) -> AssetReadiness {
    if !manifest.production_enabled {
        return AssetReadiness::ReferenceOnly;
    }
    if manifest.license.production_approved {
        return AssetReadiness::ProductionVerified;
    }
    match value.and_then(serde_json::Value::as_str) {
        Some("runtime_ready") => AssetReadiness::RuntimeReady,
        Some("partially_configured") => AssetReadiness::PartiallyConfigured,
        _ => AssetReadiness::ManualOnly,
    }
}

pub fn category_label(category: &AssetCategory) -> &'static str {
    match category {
        AssetCategory::Terrain => "Terrain",
        AssetCategory::TileObject => "Tile Objects",
        AssetCategory::Building => "Buildings",
        AssetCategory::Wall => "Walls",
        AssetCategory::Floor => "Floors",
        AssetCategory::Door => "Doors",
        AssetCategory::Furniture => "Furniture",
        AssetCategory::Crop => "Crops",
        AssetCategory::Tree => "Trees",
        AssetCategory::Foliage => "Foliage",
        AssetCategory::Character => "Characters",
        AssetCategory::Clothing => "Clothing",
        AssetCategory::Armor => "Armor",
        AssetCategory::Tool => "Tools",
        AssetCategory::Weapon => "Weapons",
        AssetCategory::Animal => "Animals",
        AssetCategory::Npc => "NPCs",
        AssetCategory::Animation => "Animations",
        AssetCategory::Effect => "Effects",
        AssetCategory::Ui => "UI",
        AssetCategory::Audio => "Audio",
        AssetCategory::Music => "Music",
        AssetCategory::Item => "Items",
        AssetCategory::Recipe => "Recipes",
        AssetCategory::Biome => "Biomes",
        AssetCategory::WorldGeneration => "World Generation",
        AssetCategory::Scene => "Scenes",
        AssetCategory::Interior => "Interiors",
        AssetCategory::Cave => "Caves",
        AssetCategory::Dungeon => "Dungeons",
        AssetCategory::EditorTemplate => "Editor Templates",
        AssetCategory::Other => "Other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_filter_is_category_and_pack_aware() {
        let snapshot = AssetBrowserSnapshot {
            packs: Vec::new(),
            diagnostics: Vec::new(),
            entries: vec![AssetBrowserEntry {
                stable_ref: StableAssetRef {
                    pack_id: AssetPackId("alpha".to_string()),
                    category: AssetCategory::Terrain,
                    asset_id: crate::asset_pack::AssetId("grass".to_string()),
                    source_id: crate::asset_pack::AssetSourceId("sheet".to_string()),
                    variant_id: None,
                },
                pack_display_name: "Alpha".to_string(),
                pack_priority: 1,
                semantic_id: "terrain.grass".to_string(),
                source_path: "grass.png".to_string(),
                source_kind: AssetSourceKind::Atlas,
                tags: BTreeSet::from(["summer".to_string()]),
                readiness: AssetReadiness::RuntimeReady,
                license_id: "CC0-1.0".to_string(),
                production_enabled: true,
                production_approved: true,
                placeable_stable_id: None,
            }],
        };
        let filter = AssetBrowserFilter {
            query: "grass".to_string(),
            pack_id: Some("alpha".to_string()),
            category: Some(AssetCategory::Terrain),
            runtime_ready_only: true,
            ..Default::default()
        };
        assert_eq!(snapshot.filtered(&filter).len(), 1);
    }
}
