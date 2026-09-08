use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs::read_to_string,
    path::{Path, PathBuf},
};

pub const LPC_TERRAIN_FAMILY_MAPPING_PATH: &str =
    "content/assets/intake/lpc_terrain_family_mapping_v0_3.json";

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct LpcBaseTileDefinition {
    pub cells: Vec<[u32; 2]>,
    #[serde(default, rename = "variantTransforms")]
    pub variant_transforms: Vec<[f32; 3]>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum LpcInnerCornerRuntime {
    Enabled(bool),
    Status(String),
}

impl LpcInnerCornerRuntime {
    pub fn is_active(&self) -> bool {
        match self {
            Self::Enabled(enabled) => *enabled,
            Self::Status(status) => status == "active",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct LpcTerrainTransitionFamily {
    pub id: String,
    pub owner: String,
    pub neighbor: String,
    #[serde(rename = "outerBlock")]
    pub outer_block: [u32; 4],
    #[serde(default, rename = "innerCornerBlock")]
    pub inner_corner_block: Option<[u32; 4]>,
    #[serde(rename = "foregroundClassifier")]
    pub foreground_classifier: String,
    #[serde(rename = "runtimeOuterMasks")]
    pub runtime_outer_masks: bool,
    #[serde(rename = "runtimeInnerCorners")]
    pub runtime_inner_corners: LpcInnerCornerRuntime,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct LpcTerrainFamilyMappingFile {
    schema: String,
    #[serde(rename = "sourceLock")]
    source_lock: String,
    source: String,
    #[serde(rename = "cellSize")]
    cell_size: u32,
    grid: [u32; 2],
    license: String,
    attribution: Vec<String>,
    #[serde(rename = "baseTiles")]
    base_tiles: HashMap<String, LpcBaseTileDefinition>,
    #[serde(rename = "transitionFamilies")]
    transition_families: Vec<LpcTerrainTransitionFamily>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LpcTerrainFamilyCatalog {
    pub schema: String,
    pub source_lock: String,
    pub source: String,
    pub cell_size: u32,
    pub grid: [u32; 2],
    pub license: String,
    pub attribution: Vec<String>,
    pub base_tiles: HashMap<String, LpcBaseTileDefinition>,
    pub transition_families: Vec<LpcTerrainTransitionFamily>,
    manifest_path: PathBuf,
}

impl LpcTerrainFamilyCatalog {
    pub fn load_default() -> Result<Self, String> {
        let manifest_path = repo_root_dir().join(LPC_TERRAIN_FAMILY_MAPPING_PATH);
        let file = load_json::<LpcTerrainFamilyMappingFile>(&manifest_path)?;
        let catalog = Self {
            schema: file.schema,
            source_lock: normalize_path(&file.source_lock),
            source: normalize_path(&file.source),
            cell_size: file.cell_size,
            grid: file.grid,
            license: file.license,
            attribution: file.attribution,
            base_tiles: file.base_tiles,
            transition_families: file.transition_families,
            manifest_path,
        };
        catalog.validate()?;
        Ok(catalog)
    }

    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    pub fn base_tile(&self, id: &str) -> Option<&LpcBaseTileDefinition> {
        self.base_tiles.get(id)
    }

    pub fn transition_family(&self, id: &str) -> Option<&LpcTerrainTransitionFamily> {
        self.transition_families
            .iter()
            .find(|family| family.id == id)
    }

    pub fn coverage_summary(&self) -> String {
        format!(
            "{} base tile roles, {} transition families",
            self.base_tiles.len(),
            self.transition_families.len()
        )
    }

    fn validate(&self) -> Result<(), String> {
        if self.cell_size == 0 || self.grid[0] == 0 || self.grid[1] == 0 {
            return Err(format!("{} has an invalid cell/grid contract", self.schema));
        }
        for (id, definition) in &self.base_tiles {
            if definition.cells.is_empty() {
                return Err(format!("base tile {id} has no source cells"));
            }
            for cell in &definition.cells {
                self.validate_cell(*cell, id)?;
            }
        }

        let mut ids = HashSet::new();
        for family in &self.transition_families {
            if !ids.insert(family.id.as_str()) {
                return Err(format!("duplicate LPC terrain family {}", family.id));
            }
            self.validate_block(family.outer_block, 3, 3, &family.id)?;
            if let Some(inner_corner_block) = family.inner_corner_block {
                self.validate_block(inner_corner_block, 2, 2, &family.id)?;
                if !family.runtime_inner_corners.is_active() {
                    return Err(format!(
                        "{} has authored inner corners but runtime activation is disabled",
                        family.id
                    ));
                }
            } else if family.runtime_inner_corners.is_active() {
                return Err(format!(
                    "{} enables inner corners without an authored block",
                    family.id
                ));
            }
            if family.owner.trim().is_empty() || family.neighbor.trim().is_empty() {
                return Err(format!("{} has an empty owner/neighbor", family.id));
            }
            if !family.runtime_outer_masks {
                return Err(format!("{} is not enabled for outer masks", family.id));
            }
        }
        Ok(())
    }

    fn validate_cell(&self, cell: [u32; 2], label: &str) -> Result<(), String> {
        if cell[0] >= self.grid[0] || cell[1] >= self.grid[1] {
            return Err(format!(
                "{label} cell [{},{}] is outside {}x{} source grid",
                cell[0], cell[1], self.grid[0], self.grid[1]
            ));
        }
        Ok(())
    }

    fn validate_block(
        &self,
        block: [u32; 4],
        expected_width: u32,
        expected_height: u32,
        label: &str,
    ) -> Result<(), String> {
        if block[2] != expected_width || block[3] != expected_height {
            return Err(format!(
                "{label} block is {}x{}, expected {expected_width}x{expected_height}",
                block[2], block[3]
            ));
        }
        if block[0] + block[2] > self.grid[0] || block[1] + block[3] > self.grid[1] {
            return Err(format!("{label} block falls outside the LPC source grid"));
        }
        Ok(())
    }
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

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_catalog_exposes_reviewed_water_and_ground_families() {
        let catalog = LpcTerrainFamilyCatalog::load_default()
            .expect("LPC terrain family mapping should load");
        assert!(catalog.base_tile("shallow_water").is_some());
        assert!(catalog.base_tile("deep_water").is_some());
        assert!(catalog
            .transition_family("sand_bank_over_shallow")
            .is_some());
        assert!(catalog.transition_family("shallow_rim_over_deep").is_some());
        let wet_sand = catalog
            .transition_family("sand_over_wet_sand")
            .expect("dry/wet sand family");
        assert!(wet_sand.inner_corner_block.is_some());
        assert!(wet_sand.runtime_inner_corners.is_active());
        let pebble = catalog
            .transition_family("pebble_path_over_dirt")
            .expect("pebble path family");
        assert!(pebble.inner_corner_block.is_none());
        assert!(!pebble.runtime_inner_corners.is_active());
        assert!(catalog.coverage_summary().contains("transition families"));
        assert!(catalog
            .manifest_path()
            .ends_with(LPC_TERRAIN_FAMILY_MAPPING_PATH));
    }
}
