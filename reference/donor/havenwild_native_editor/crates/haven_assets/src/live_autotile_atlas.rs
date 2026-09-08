use crate::asset_registry::AtlasRect;
use haven_core::{TileAutoGroup, TileKind};
use serde::Deserialize;
use std::{
    fs::read_to_string,
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub const LIVE_AUTOTILE_ATLAS_MANIFEST_PATH: &str =
    "assets/generated/worldgen_v0_1/terrain/live_autotile_16_32.json";
pub const LIVE_AUTOTILE_ATLAS_PATH: &str =
    "assets/generated/worldgen_v0_1/terrain/live_autotile_16_32.png";

static LIVE_AUTOTILE_ATLAS: OnceLock<Result<LiveAutotileAtlasRegistry, String>> = OnceLock::new();

#[derive(Clone, Debug, PartialEq)]
pub struct LiveAutotileAtlasEntry {
    pub stable_id: String,
    pub sheet: String,
    pub group: TileAutoGroup,
    pub tile: TileKind,
    pub mask4: u8,
    pub rect: AtlasRect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiveAutotileAtlasRegistry {
    id: String,
    manifest_path: PathBuf,
    atlas_path: String,
    entries: Vec<LiveAutotileAtlasEntry>,
}

impl LiveAutotileAtlasRegistry {
    pub fn load_default() -> Result<Self, String> {
        let manifest_path = repo_root_dir().join(LIVE_AUTOTILE_ATLAS_MANIFEST_PATH);
        let manifest = load_json::<LiveAutotileManifest>(&manifest_path)?;
        let atlas_path = normalize_repo_relative_path(&manifest.output);
        if atlas_path != LIVE_AUTOTILE_ATLAS_PATH {
            return Err(format!(
                "live autotile output mismatch: expected {LIVE_AUTOTILE_ATLAS_PATH}, found {atlas_path}"
            ));
        }
        if manifest.tile_size != 32 || manifest.columns != 16 {
            return Err(format!(
                "{} must remain a 32px, 16-column cardinal mask atlas",
                manifest.id
            ));
        }
        let mut entries = Vec::with_capacity(manifest.variants.len());
        for variant in manifest.variants {
            let group = TileAutoGroup::from_code(&variant.group)
                .ok_or_else(|| format!("unknown live autotile group {}", variant.group))?;
            let tile = TileKind::from_code(&variant.tile_kind)
                .ok_or_else(|| format!("unknown live autotile TileKind {}", variant.tile_kind))?;
            if variant.mask4 > 15 {
                return Err(format!(
                    "{} has invalid cardinal mask {}",
                    variant.id, variant.mask4
                ));
            }
            entries.push(LiveAutotileAtlasEntry {
                stable_id: variant.id,
                sheet: atlas_path.clone(),
                group,
                tile,
                mask4: variant.mask4,
                rect: AtlasRect {
                    x: variant.rect[0] as f32,
                    y: variant.rect[1] as f32,
                    w: variant.rect[2] as f32,
                    h: variant.rect[3] as f32,
                },
            });
        }
        let registry = Self {
            id: manifest.id,
            manifest_path,
            atlas_path,
            entries,
        };
        registry.validate()?;
        Ok(registry)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn atlas_path(&self) -> &str {
        &self.atlas_path
    }

    pub fn entry(&self, group: TileAutoGroup, mask: u8) -> Option<&LiveAutotileAtlasEntry> {
        let mask4 = mask & 0x0f;
        self.entries
            .iter()
            .find(|entry| entry.group == group && entry.mask4 == mask4)
    }

    pub fn entries(&self) -> &[LiveAutotileAtlasEntry] {
        &self.entries
    }

    pub fn coverage_summary(&self) -> String {
        format!(
            "{} -> {} groups, {} bound cardinal cells",
            self.id,
            TileAutoGroup::ALL.len(),
            self.entries.len()
        )
    }

    fn validate(&self) -> Result<(), String> {
        for group in TileAutoGroup::ALL {
            for mask in 0..16 {
                if self.entry(group, mask).is_none() {
                    return Err(format!(
                        "{} is missing {} mask {}",
                        self.id,
                        group.code(),
                        mask
                    ));
                }
            }
        }
        Ok(())
    }
}

pub fn live_autotile_atlas_registry() -> Result<&'static LiveAutotileAtlasRegistry, &'static str> {
    LIVE_AUTOTILE_ATLAS
        .get_or_init(LiveAutotileAtlasRegistry::load_default)
        .as_ref()
        .map_err(|error| error.as_str())
}

pub fn live_autotile_atlas_entry(
    group: TileAutoGroup,
    mask: u8,
) -> Option<&'static LiveAutotileAtlasEntry> {
    live_autotile_atlas_registry()
        .ok()
        .and_then(|registry| registry.entry(group, mask))
}

#[derive(Deserialize)]
struct LiveAutotileManifest {
    id: String,
    tile_size: u32,
    columns: u32,
    output: String,
    variants: Vec<LiveAutotileManifestVariant>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LiveAutotileManifestVariant {
    id: String,
    group: String,
    tile_kind: String,
    mask4: u8,
    rect: [u32; 4],
}

fn repo_root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root should be reachable from haven_assets")
}

fn load_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let raw = read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))
}

fn normalize_repo_relative_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_atlas_covers_every_group_and_cardinal_mask() {
        match LiveAutotileAtlasRegistry::load_default() {
            Ok(registry) => {
                assert_eq!(registry.entries().len(), TileAutoGroup::ALL.len() * 16);
                assert!(registry.entry(TileAutoGroup::Road, 10).is_some());
                assert!(registry.entry(TileAutoGroup::CaveWall, 15).is_some());
            }
            Err(error) => {
                assert!(
                    error.contains("live_autotile_16_32.json"),
                    "unexpected live autotile load error: {error}"
                );
            }
        }
    }
}
