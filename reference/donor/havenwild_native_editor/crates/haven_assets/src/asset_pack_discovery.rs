use crate::asset_pack::{AssetPackId, AssetPackManifest, AssetPackRegistry};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackDiscoveryStatus {
    Mounted,
    Disabled,
    Invalid,
    Duplicate,
    MissingDependency,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackDiscoveryRecord {
    pub manifest_path: String,
    pub pack_id: Option<AssetPackId>,
    pub status: PackDiscoveryStatus,
    #[serde(default)]
    pub diagnostics: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackDiscoveryReport {
    pub roots: Vec<String>,
    pub records: Vec<PackDiscoveryRecord>,
}

impl PackDiscoveryReport {
    pub fn mounted_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.status == PackDiscoveryStatus::Mounted)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| {
                matches!(
                    record.status,
                    PackDiscoveryStatus::Invalid
                        | PackDiscoveryStatus::Duplicate
                        | PackDiscoveryStatus::MissingDependency
                )
            })
            .count()
    }
}

#[derive(Clone, Debug)]
pub struct AssetPackDiscovery {
    roots: Vec<PathBuf>,
    manifest_name: String,
}

impl AssetPackDiscovery {
    pub fn new(roots: impl IntoIterator<Item = PathBuf>) -> Self {
        Self {
            roots: roots.into_iter().collect(),
            manifest_name: "pack.json".to_string(),
        }
    }

    pub fn project_default(project_root: &Path) -> Self {
        Self::new([
            project_root.join("content/asset_packs"),
            project_root.join("user/asset_packs"),
            project_root.join("mods/asset_packs"),
        ])
    }

    pub fn discover(&self) -> PackDiscoveryReport {
        let mut report = PackDiscoveryReport {
            roots: self
                .roots
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect(),
            records: Vec::new(),
        };
        for root in &self.roots {
            self.collect_manifests(root, &mut report.records);
        }
        report
            .records
            .sort_by(|left, right| left.manifest_path.cmp(&right.manifest_path));
        report
    }

    pub fn discover_and_mount(
        &self,
        registry: &mut AssetPackRegistry,
        include_disabled: bool,
    ) -> PackDiscoveryReport {
        let mut report = self.discover();
        let mut manifests = BTreeMap::<AssetPackId, (usize, AssetPackManifest)>::new();

        for (index, record) in report.records.iter_mut().enumerate() {
            let path = PathBuf::from(&record.manifest_path);
            let manifest = match read_manifest(&path) {
                Ok(manifest) => manifest,
                Err(error) => {
                    record.status = PackDiscoveryStatus::Invalid;
                    record.diagnostics.push(error);
                    continue;
                }
            };
            record.pack_id = Some(manifest.id.clone());
            if let Err(errors) = manifest.validate() {
                record.status = PackDiscoveryStatus::Invalid;
                record.diagnostics.extend(errors);
                continue;
            }
            if manifests.contains_key(&manifest.id) {
                record.status = PackDiscoveryStatus::Duplicate;
                record
                    .diagnostics
                    .push(format!("duplicate pack id {}", manifest.id.0));
                continue;
            }
            manifests.insert(manifest.id.clone(), (index, manifest));
        }

        let available: BTreeSet<_> = manifests.keys().cloned().collect();
        let mut pending: BTreeSet<_> = available.clone();
        let mut mounted = BTreeSet::<AssetPackId>::new();

        while !pending.is_empty() {
            let mut progressed = false;
            let ids: Vec<_> = pending.iter().cloned().collect();
            for id in ids {
                let (record_index, manifest) = &manifests[&id];
                let missing: Vec<_> = manifest
                    .dependencies
                    .iter()
                    .filter(|dependency| {
                        !dependency.optional && !available.contains(&dependency.pack_id)
                    })
                    .map(|dependency| dependency.pack_id.0.clone())
                    .collect();
                if !missing.is_empty() {
                    let record = &mut report.records[*record_index];
                    record.status = PackDiscoveryStatus::MissingDependency;
                    record.diagnostics.push(format!(
                        "missing required pack dependencies: {}",
                        missing.join(", ")
                    ));
                    pending.remove(&id);
                    progressed = true;
                    continue;
                }

                let waiting = manifest.dependencies.iter().any(|dependency| {
                    !dependency.optional
                        && available.contains(&dependency.pack_id)
                        && !mounted.contains(&dependency.pack_id)
                });
                if waiting {
                    continue;
                }

                if !manifest.production_enabled && !include_disabled {
                    report.records[*record_index].status = PackDiscoveryStatus::Disabled;
                    pending.remove(&id);
                    mounted.insert(id);
                    progressed = true;
                    continue;
                }

                match registry.mount(manifest.clone()) {
                    Ok(()) => {
                        report.records[*record_index].status = PackDiscoveryStatus::Mounted;
                        mounted.insert(id.clone());
                    }
                    Err(errors) => {
                        let record = &mut report.records[*record_index];
                        record.status = PackDiscoveryStatus::Invalid;
                        record.diagnostics.extend(errors);
                    }
                }
                pending.remove(&id);
                progressed = true;
            }

            if !progressed {
                for id in pending.clone() {
                    let (record_index, _) = &manifests[&id];
                    let record = &mut report.records[*record_index];
                    record.status = PackDiscoveryStatus::MissingDependency;
                    record
                        .diagnostics
                        .push("dependency cycle or unresolved dependency ordering".to_string());
                    pending.remove(&id);
                }
            }
        }

        report
    }

    fn collect_manifests(&self, root: &Path, records: &mut Vec<PackDiscoveryRecord>) {
        if !root.is_dir() {
            return;
        }
        let mut stack = vec![root.to_path_buf()];
        while let Some(directory) = stack.pop() {
            let entries = match fs::read_dir(&directory) {
                Ok(entries) => entries,
                Err(error) => {
                    records.push(PackDiscoveryRecord {
                        manifest_path: directory.to_string_lossy().into_owned(),
                        pack_id: None,
                        status: PackDiscoveryStatus::Invalid,
                        diagnostics: vec![format!("unable to read directory: {error}")],
                    });
                    continue;
                }
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.file_name().and_then(|name| name.to_str())
                    == Some(self.manifest_name.as_str())
                {
                    records.push(PackDiscoveryRecord {
                        manifest_path: path.to_string_lossy().into_owned(),
                        pack_id: None,
                        status: PackDiscoveryStatus::Disabled,
                        diagnostics: Vec::new(),
                    });
                }
            }
        }
    }
}

pub fn load_project_asset_packs(
    project_root: &Path,
    include_disabled: bool,
) -> Result<(AssetPackRegistry, PackDiscoveryReport), PackDiscoveryReport> {
    let discovery = AssetPackDiscovery::project_default(project_root);
    let mut registry = AssetPackRegistry::default();
    let report = discovery.discover_and_mount(&mut registry, include_disabled);
    if report.failed_count() == 0 {
        Ok((registry, report))
    } else {
        Err(report)
    }
}

fn read_manifest(path: &Path) -> Result<AssetPackManifest, String> {
    let bytes = fs::read(path).map_err(|error| format!("unable to read manifest: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid pack manifest JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset_pack::{
        AssetCategory, AssetDefinition, AssetId, AssetSource, AssetSourceId, AssetSourceKind,
        LicenseRecord,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_root() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("havenwild-pack-discovery-{nonce}"))
    }

    fn write_pack(root: &Path, id: &str, priority: i32, enabled: bool) {
        let directory = root.join(id);
        fs::create_dir_all(&directory).expect("create pack directory");
        let manifest = AssetPackManifest {
            schema: AssetPackManifest::SCHEMA.to_string(),
            id: AssetPackId(id.to_string()),
            display_name: id.to_string(),
            version: "1".to_string(),
            license: LicenseRecord {
                license_id: "CC0-1.0".to_string(),
                production_approved: true,
                attribution: Vec::new(),
                commercial_use: true,
                redistribution: true,
                source_url: None,
            },
            production_enabled: enabled,
            dependencies: Vec::new(),
            sources: vec![AssetSource {
                id: AssetSourceId("sheet".to_string()),
                kind: AssetSourceKind::RawSheet,
                path: "sheet.png".to_string(),
                tile_size: Some([32, 32]),
                margin: 0,
                spacing: 0,
            }],
            assets: vec![AssetDefinition {
                id: AssetId("asset".to_string()),
                category: AssetCategory::Terrain,
                semantic_id: "terrain.grass".to_string(),
                source_id: AssetSourceId("sheet".to_string()),
                atlas_region: None,
                variants: Vec::new(),
                tags: BTreeSet::new(),
                metadata: BTreeMap::new(),
            }],
            priority,
        };
        fs::write(
            directory.join("pack.json"),
            serde_json::to_vec_pretty(&manifest).expect("serialize manifest"),
        )
        .expect("write manifest");
    }

    #[test]
    fn discovers_and_mounts_unknown_pack_directories() {
        let root = temporary_root();
        write_pack(&root, "alpha", 10, true);
        write_pack(&root, "beta", 20, true);
        let discovery = AssetPackDiscovery::new([root.clone()]);
        let mut registry = AssetPackRegistry::default();
        let report = discovery.discover_and_mount(&mut registry, false);
        assert_eq!(report.mounted_count(), 2);
        assert_eq!(registry.mounted_pack_count(), 2);
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn disabled_packs_are_discovered_without_entering_production_registry() {
        let root = temporary_root();
        write_pack(&root, "reference", 10, false);
        let discovery = AssetPackDiscovery::new([root.clone()]);
        let mut registry = AssetPackRegistry::default();
        let report = discovery.discover_and_mount(&mut registry, false);
        assert_eq!(registry.mounted_pack_count(), 0);
        assert!(report
            .records
            .iter()
            .any(|record| record.status == PackDiscoveryStatus::Disabled));
        fs::remove_dir_all(root).ok();
    }
}
