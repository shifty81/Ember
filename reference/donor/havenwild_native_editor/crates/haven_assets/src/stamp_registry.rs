use crate::{
    asset_pack::{AssetCategory, StableAssetRef},
    asset_registry::AtlasRect,
    runtime_asset_cache::RuntimeAssetSession,
    semantic_asset_resolution::AssetResolutionContext,
};
use haven_core::{ObjectFootprint, PlacedStamp};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs::read_to_string,
    path::{Path, PathBuf},
};

const LEGACY_POND_MANIFEST: &str =
    "assets/generated/worldgen_v0_1/terrain/lpc_expandable_ponds_32.json";
const LEGACY_POND_SHEET: &str = "assets/source/licensed/lpc_revised/Terrain/terrain_summer.png";
const LEGACY_POND_CATEGORY: &str = "terrain/ponds";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExpandableStampDefinition {
    pub cell_size: f32,
    pub base_fill_tile: AtlasRect,
    pub outer_tiles: [AtlasRect; 9],
    pub inner_corner_tiles: [AtlasRect; 4],
    pub minimum_w: i32,
    pub minimum_h: i32,
    pub supports_freeform: bool,
}

impl ExpandableStampDefinition {
    pub fn source_rect_for_rect_cell(
        self,
        cell_x: i32,
        cell_y: i32,
        width: i32,
        height: i32,
    ) -> AtlasRect {
        let column = if cell_x <= 0 {
            0
        } else if cell_x >= width.saturating_sub(1) {
            2
        } else {
            1
        };
        let row = if cell_y <= 0 {
            0
        } else if cell_y >= height.saturating_sub(1) {
            2
        } else {
            1
        };
        self.outer_tiles[(row * 3 + column) as usize]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StampDefinition {
    pub stable_id: String,
    pub legacy_id: Option<String>,
    pub catalog_ref: Option<StableAssetRef>,
    pub sheet_ref: Option<StableAssetRef>,
    pub label: String,
    pub category: String,
    pub sheet: String,
    pub rect: AtlasRect,
    pub footprint: ObjectFootprint,
    pub license: String,
    pub expandable: Option<ExpandableStampDefinition>,
}

impl StampDefinition {
    pub fn placed_at(&self, x: i32, y: i32) -> PlacedStamp {
        PlacedStamp::new(self.stable_id.clone(), x, y, self.footprint)
    }

    pub fn visual_tiles(&self) -> i32 {
        self.footprint.visual_w.max(1) * self.footprint.visual_h.max(1)
    }

    pub fn is_expandable(&self) -> bool {
        self.expandable.is_some()
    }

    pub fn minimum_visual_size(&self) -> (i32, i32) {
        self.expandable
            .map(|definition| (definition.minimum_w, definition.minimum_h))
            .unwrap_or((
                self.footprint.visual_w.max(1),
                self.footprint.visual_h.max(1),
            ))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StampRegistry {
    entries: Vec<StampDefinition>,
}

impl StampRegistry {
    pub fn load_default() -> Result<Self, String> {
        let root = runtime_or_repository_root();
        let session = RuntimeAssetSession::discover_tolerant(&root);
        if let Ok(registry) = Self::load_discovered(&session) {
            if !registry.entries.is_empty() {
                return Ok(registry);
            }
        }
        Self::load_from_root(&root)
    }

    pub fn load_from_manifest_path(
        manifest_path: &Path,
        atlas_path: &str,
        fallback_category: &str,
    ) -> Result<Self, String> {
        let definitions = load_manifest(manifest_path, atlas_path, fallback_category, None, None)?;
        let mut entries = BTreeMap::<String, StampDefinition>::new();
        for definition in definitions {
            entries
                .entry(definition.stable_id.clone())
                .or_insert(definition);
        }
        Ok(Self {
            entries: entries.into_values().collect(),
        })
    }

    pub fn load_from_root(root: &Path) -> Result<Self, String> {
        let mut entries = BTreeMap::<String, StampDefinition>::new();
        let mut failures = Vec::new();
        let path = root.join(LEGACY_POND_MANIFEST);
        match load_manifest(&path, LEGACY_POND_SHEET, LEGACY_POND_CATEGORY, None, None) {
            Ok(definitions) => {
                for definition in definitions {
                    entries
                        .entry(definition.stable_id.clone())
                        .or_insert(definition);
                }
            }
            Err(error) => failures.push(error),
        }
        if entries.is_empty() && !failures.is_empty() {
            return Err(failures.join("; "));
        }
        Ok(Self {
            entries: entries.into_values().collect(),
        })
    }

    pub fn load_discovered(session: &RuntimeAssetSession) -> Result<Self, String> {
        let mut entries = BTreeMap::<String, StampDefinition>::new();
        let mut failures = Vec::new();
        for pack in session.registry.mounted_packs() {
            for asset in pack.assets.iter().filter(|asset| {
                asset.category == AssetCategory::EditorTemplate
                    && asset.semantic_id.starts_with("stamp.catalog.")
            }) {
                let catalog_ref = StableAssetRef {
                    pack_id: pack.id.clone(),
                    category: asset.category.clone(),
                    asset_id: asset.id.clone(),
                    source_id: asset.source_id.clone(),
                    variant_id: None,
                };
                let Some(catalog_source) = session.sources.get(&catalog_ref) else {
                    failures.push(format!(
                        "missing source for stamp catalog {}",
                        asset.semantic_id
                    ));
                    continue;
                };
                let Some(sheet_semantic_id) = asset
                    .metadata
                    .get("sheet_semantic_id")
                    .and_then(serde_json::Value::as_str)
                else {
                    failures.push(format!(
                        "stamp catalog {} is missing metadata.sheet_semantic_id",
                        asset.semantic_id
                    ));
                    continue;
                };
                let Some(sheet_source) = session.resolve_source(
                    sheet_semantic_id,
                    AssetCategory::TileObject,
                    AssetResolutionContext {
                        explicit_pack: Some(pack.id.clone()),
                        allow_reference_only: false,
                        ..AssetResolutionContext::default()
                    },
                ) else {
                    failures.push(format!(
                        "stamp catalog {} cannot resolve sheet provider {}",
                        asset.semantic_id, sheet_semantic_id
                    ));
                    continue;
                };
                let fallback_category = asset
                    .metadata
                    .get("fallback_category")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("stamps");
                match load_manifest(
                    &catalog_source.source_path,
                    &sheet_source.source_path.to_string_lossy(),
                    fallback_category,
                    Some(&catalog_ref),
                    Some(&sheet_source.stable_ref),
                ) {
                    Ok(definitions) => {
                        for definition in definitions {
                            if entries
                                .insert(definition.stable_id.clone(), definition)
                                .is_some()
                            {
                                failures
                                    .push("duplicate stable stamp identity discovered".to_string());
                            }
                        }
                    }
                    Err(error) => failures.push(error),
                }
            }
        }
        if entries.is_empty() && !failures.is_empty() {
            return Err(failures.join("; "));
        }
        Ok(Self {
            entries: entries.into_values().collect(),
        })
    }

    pub fn entries(&self) -> &[StampDefinition] {
        &self.entries
    }

    pub fn entry(&self, stable_id: &str) -> Option<&StampDefinition> {
        self.entries.iter().find(|entry| {
            entry.stable_id == stable_id || entry.legacy_id.as_deref() == Some(stable_id)
        })
    }

    pub fn sheets(&self) -> Vec<String> {
        let mut sheets = self
            .entries
            .iter()
            .map(|entry| entry.sheet.clone())
            .collect::<Vec<_>>();
        sheets.sort();
        sheets.dedup();
        sheets
    }

    pub fn coverage_summary(&self) -> String {
        let multi_tile = self
            .entries
            .iter()
            .filter(|entry| entry.visual_tiles() > 1)
            .count();
        format!(
            "{} registered stamps ({} multi-tile)",
            self.entries.len(),
            multi_tile
        )
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawManifest {
    #[serde(default)]
    license: String,
    #[serde(default)]
    objects: Vec<RawStamp>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawStamp {
    id: String,
    #[serde(default)]
    label: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    object_type: String,
    rect: [f32; 4],
    visual_footprint: [i32; 2],
    collision_footprint: [i32; 2],
    origin: [i32; 2],
    #[serde(default)]
    occludes_player: bool,
    #[serde(default)]
    fade_when_player_behind: bool,
    #[serde(default)]
    expandable: Option<RawExpandableStamp>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawExpandableStamp {
    cell_size: f32,
    outer_block: [i32; 4],
    inner_corner_block: [i32; 4],
    base_fill_cell: [i32; 2],
    minimum_size: [i32; 2],
    #[serde(default)]
    supports_freeform: bool,
}

fn load_manifest(
    path: &Path,
    atlas_path: &str,
    fallback_category: &str,
    catalog_ref: Option<&StableAssetRef>,
    sheet_ref: Option<&StableAssetRef>,
) -> Result<Vec<StampDefinition>, String> {
    let raw = read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let document: RawManifest = serde_json::from_str(&raw)
        .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
    let license = if document.license.trim().is_empty() {
        "Havenwild project-owned original".to_string()
    } else {
        document.license
    };
    let mut definitions = Vec::with_capacity(document.objects.len());
    for raw in document.objects {
        let visual_w = raw.visual_footprint[0].max(1);
        let visual_h = raw.visual_footprint[1].max(1);
        let collision_w = raw.collision_footprint[0].max(0);
        let collision_h = raw.collision_footprint[1].max(0);
        let blocks_movement = collision_w > 0 && collision_h > 0 && raw.object_type != "clutter";
        let footprint = ObjectFootprint {
            visual_offset_x: -raw.origin[0],
            visual_offset_y: -raw.origin[1],
            visual_w,
            visual_h,
            collision_offset_x: -(collision_w / 2),
            collision_offset_y: -(collision_h.saturating_sub(1)),
            collision_w,
            collision_h,
            interaction_offset_x: -(collision_w / 2),
            interaction_offset_y: -(collision_h.saturating_sub(1)),
            interaction_w: collision_w.max(1),
            interaction_h: collision_h.max(1),
            blocks_movement,
            occludes_player: raw.occludes_player,
            fade_when_player_behind: raw.fade_when_player_behind,
        };
        let category = if raw.category.trim().is_empty() {
            fallback_category.to_string()
        } else {
            raw.category
        };
        let label = if raw.label.trim().is_empty() {
            pretty_label(&raw.id)
        } else {
            raw.label
        };
        let expandable = raw
            .expandable
            .as_ref()
            .map(expandable_definition_from_raw)
            .transpose()?;
        let legacy_id = raw.id;
        let stable_id = catalog_ref
            .map(|reference| format!("{}::{}", reference.pack_id.0, legacy_id))
            .unwrap_or_else(|| legacy_id.clone());
        definitions.push(StampDefinition {
            stable_id,
            legacy_id: Some(legacy_id),
            catalog_ref: catalog_ref.cloned(),
            sheet_ref: sheet_ref.cloned(),
            label,
            category,
            sheet: atlas_path.to_string(),
            rect: AtlasRect {
                x: raw.rect[0],
                y: raw.rect[1],
                w: raw.rect[2],
                h: raw.rect[3],
            },
            footprint,
            license: license.clone(),
            expandable,
        });
    }
    Ok(definitions)
}

fn expandable_definition_from_raw(
    raw: &RawExpandableStamp,
) -> Result<ExpandableStampDefinition, String> {
    let [outer_x, outer_y, outer_w, outer_h] = raw.outer_block;
    if (outer_w, outer_h) != (3, 3) {
        return Err(format!(
            "expandable outer block must be 3x3, found {}x{}",
            outer_w, outer_h
        ));
    }
    let [inner_x, inner_y, inner_w, inner_h] = raw.inner_corner_block;
    if (inner_w, inner_h) != (2, 2) {
        return Err(format!(
            "expandable inner-corner block must be 2x2, found {}x{}",
            inner_w, inner_h
        ));
    }
    if raw.cell_size <= 0.0 {
        return Err("expandable cell size must be positive".to_string());
    }
    let mut outer_tiles = [AtlasRect {
        x: 0.0,
        y: 0.0,
        w: raw.cell_size,
        h: raw.cell_size,
    }; 9];
    for row in 0..3 {
        for column in 0..3 {
            outer_tiles[(row * 3 + column) as usize] =
                atlas_cell_rect(outer_x + column, outer_y + row, raw.cell_size);
        }
    }
    let mut inner_corner_tiles = [AtlasRect {
        x: 0.0,
        y: 0.0,
        w: raw.cell_size,
        h: raw.cell_size,
    }; 4];
    for row in 0..2 {
        for column in 0..2 {
            inner_corner_tiles[(row * 2 + column) as usize] =
                atlas_cell_rect(inner_x + column, inner_y + row, raw.cell_size);
        }
    }
    Ok(ExpandableStampDefinition {
        cell_size: raw.cell_size,
        base_fill_tile: atlas_cell_rect(
            raw.base_fill_cell[0],
            raw.base_fill_cell[1],
            raw.cell_size,
        ),
        outer_tiles,
        inner_corner_tiles,
        minimum_w: raw.minimum_size[0].max(3),
        minimum_h: raw.minimum_size[1].max(3),
        supports_freeform: raw.supports_freeform,
    })
}

fn atlas_cell_rect(column: i32, row: i32, cell_size: f32) -> AtlasRect {
    AtlasRect {
        x: column as f32 * cell_size,
        y: row as f32 * cell_size,
        w: cell_size,
        h: cell_size,
    }
}

fn pretty_label(id: &str) -> String {
    id.trim_start_matches("havenwild_")
        .trim_start_matches("home_")
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn runtime_or_repository_root() -> PathBuf {
    let current = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if current.join(LEGACY_POND_MANIFEST).exists() {
        return current;
    }
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expandable_lpc_ponds_preserve_source_roles() {
        let registry = StampRegistry::load_default().expect("load stamp registry");
        let pond = registry
            .entry("lpc_pond_grass_bank_a")
            .expect("expandable LPC pond");
        let expandable = pond.expandable.expect("expandable definition");
        assert_eq!(pond.minimum_visual_size(), (3, 3));
        assert!(expandable.supports_freeform);
        assert_eq!(
            expandable.source_rect_for_rect_cell(0, 0, 8, 6),
            AtlasRect {
                x: 0.0,
                y: 320.0,
                w: 32.0,
                h: 32.0,
            }
        );
        assert_eq!(
            expandable.source_rect_for_rect_cell(3, 2, 8, 6),
            AtlasRect {
                x: 32.0,
                y: 352.0,
                w: 32.0,
                h: 32.0,
            }
        );
    }

    #[test]
    fn default_registry_keeps_approved_lpc_ponds_and_quarantines_invalid_cliff_stamps() {
        let registry = StampRegistry::load_default().expect("load stamp registry");

        let pond = registry
            .entry("lpc_pond_grass_bank_a")
            .expect("approved LPC pond provider");
        let pond_sheet = pond.sheet.replace('\\', "/");
        assert!(pond_sheet.contains("lpc_revised/Terrain/terrain_summer.png"));
        assert_eq!(
            pond.catalog_ref
                .as_ref()
                .map(|reference| reference.pack_id.0.as_str()),
            Some("havenwild_objects")
        );

        assert!(registry.entry("oga_lpc.cliff.ladder").is_none());
        assert!(registry.entries().iter().all(|entry| entry
            .catalog_ref
            .as_ref()
            .is_none_or(|reference| { reference.pack_id.0 != "oga_lpc_cliffs" })));
    }
}
