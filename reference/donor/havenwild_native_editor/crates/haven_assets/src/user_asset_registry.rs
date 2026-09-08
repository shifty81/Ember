use crate::{
    asset_intake::{
        AssetFootprintRecipe, AssetIntakeTarget, AssetIntakeTargetKind, AssetLicenseDeclaration,
        AssetPivot, AssetSliceRect, ASSET_INTAKE_MANIFEST_PATH,
    },
    asset_registry::AtlasRect,
};
use haven_core::{ObjectFootprint, ObjectKind, TileKind};
use serde::Deserialize;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    time::SystemTime,
};

#[derive(Clone, Debug, PartialEq)]
pub struct UserAssetBinding {
    pub stable_id: String,
    pub target: AssetIntakeTarget,
    pub sheet: String,
    pub rect: AtlasRect,
    pub pivot: AssetPivot,
    pub footprint: AssetFootprintRecipe,
    pub source_path: String,
    pub source_rect: AssetSliceRect,
    pub license: AssetLicenseDeclaration,
}

impl UserAssetBinding {
    pub fn target_stable_id(&self) -> String {
        self.target.stable_id()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UserAssetRegistry {
    atlas_path: String,
    generated_by: String,
    entries: Vec<UserAssetBinding>,
    manifest_path: PathBuf,
    modified: Option<SystemTime>,
}

impl UserAssetRegistry {
    pub fn load_default() -> Result<Self, String> {
        let manifest_path = repo_root_dir().join(ASSET_INTAKE_MANIFEST_PATH);
        let raw = read_to_string(&manifest_path)
            .map_err(|error| format!("failed to read {}: {error}", manifest_path.display()))?;
        let manifest: UserAssetManifest = serde_json::from_str(&raw)
            .map_err(|error| format!("failed to parse {}: {error}", manifest_path.display()))?;
        if manifest.schema != "havenwild.user_asset_atlas.v0_1" {
            return Err(format!(
                "unsupported user asset manifest schema {}",
                manifest.schema
            ));
        }
        let modified = manifest_path
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        let atlas_path = manifest.atlas;
        let generated_by = manifest.generated_by;
        let entries = manifest
            .entries
            .into_iter()
            .map(|entry| UserAssetBinding {
                stable_id: entry.stable_id,
                target: entry.target,
                sheet: atlas_path.clone(),
                rect: AtlasRect {
                    x: entry.rect[0] as f32,
                    y: entry.rect[1] as f32,
                    w: entry.rect[2] as f32,
                    h: entry.rect[3] as f32,
                },
                pivot: entry.pivot,
                footprint: entry.footprint,
                source_path: entry.source_path,
                source_rect: entry.source_rect,
                license: entry.license,
            })
            .collect();
        Ok(Self {
            atlas_path,
            generated_by,
            entries,
            manifest_path,
            modified,
        })
    }

    pub fn empty() -> Self {
        Self {
            atlas_path: String::new(),
            generated_by: String::new(),
            entries: Vec::new(),
            manifest_path: repo_root_dir().join(ASSET_INTAKE_MANIFEST_PATH),
            modified: None,
        }
    }

    pub fn atlas_path(&self) -> &str {
        &self.atlas_path
    }

    pub fn generated_by(&self) -> &str {
        &self.generated_by
    }

    pub fn entries(&self) -> &[UserAssetBinding] {
        &self.entries
    }

    pub fn modified(&self) -> Option<SystemTime> {
        self.modified
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn binding_for_target(&self, target_stable_id: &str) -> Option<&UserAssetBinding> {
        self.entries
            .iter()
            .find(|entry| entry.target_stable_id() == target_stable_id)
    }

    pub fn binding_for_tile(&self, tile: TileKind) -> Option<&UserAssetBinding> {
        self.binding_for_target(&format!("tile/{}", tile.code()))
    }

    pub fn binding_for_object(&self, object: ObjectKind) -> Option<&UserAssetBinding> {
        self.binding_for_target(&format!("object/{}", object.code()))
    }

    pub fn object_footprint(&self, object: ObjectKind) -> Option<ObjectFootprint> {
        let binding = self.binding_for_object(object)?;
        let mut footprint = object.default_footprint();
        footprint.visual_offset_x = binding.footprint.visual[0];
        footprint.visual_offset_y = binding.footprint.visual[1];
        footprint.visual_w = binding.footprint.visual[2];
        footprint.visual_h = binding.footprint.visual[3];
        footprint.collision_offset_x = binding.footprint.collision[0];
        footprint.collision_offset_y = binding.footprint.collision[1];
        footprint.collision_w = binding.footprint.collision[2];
        footprint.collision_h = binding.footprint.collision[3];
        footprint.interaction_offset_x = binding.footprint.interaction[0];
        footprint.interaction_offset_y = binding.footprint.interaction[1];
        footprint.interaction_w = binding.footprint.interaction[2];
        footprint.interaction_h = binding.footprint.interaction[3];
        Some(footprint)
    }

    pub fn coverage_summary(&self) -> String {
        let tile_count = self
            .entries
            .iter()
            .filter(|entry| entry.target.kind == AssetIntakeTargetKind::Tile)
            .count();
        let object_count = self.entries.len().saturating_sub(tile_count);
        format!(
            "{} promoted bindings ({} tile, {} object)",
            self.entries.len(),
            tile_count,
            object_count
        )
    }
}

pub fn load_user_asset_registry_default() -> Result<UserAssetRegistry, String> {
    UserAssetRegistry::load_default()
}

pub fn authored_object_footprint(object: ObjectKind) -> ObjectFootprint {
    UserAssetRegistry::load_default()
        .ok()
        .and_then(|registry| registry.object_footprint(object))
        .unwrap_or_else(|| object.default_footprint())
}

pub fn user_asset_manifest_modified() -> Option<SystemTime> {
    repo_root_dir()
        .join(ASSET_INTAKE_MANIFEST_PATH)
        .metadata()
        .ok()
        .and_then(|metadata| metadata.modified().ok())
}

fn repo_root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root should be reachable from haven_assets")
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserAssetManifest {
    schema: String,
    atlas: String,
    generated_by: String,
    entries: Vec<UserAssetManifestEntry>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserAssetManifestEntry {
    stable_id: String,
    target: AssetIntakeTarget,
    rect: [u32; 4],
    pivot: AssetPivot,
    footprint: AssetFootprintRecipe,
    source_path: String,
    source_rect: AssetSliceRect,
    license: AssetLicenseDeclaration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_user_registry_degrades_to_default_footprints() {
        let registry =
            UserAssetRegistry::load_default().unwrap_or_else(|_| UserAssetRegistry::empty());
        if let Some(binding) = registry.binding_for_object(ObjectKind::CaveEntrance) {
            assert_eq!(binding.target_stable_id(), "object/cave_entrance");
        } else {
            assert_eq!(
                authored_object_footprint(ObjectKind::CaveEntrance),
                ObjectKind::CaveEntrance.default_footprint()
            );
        }
    }
}
