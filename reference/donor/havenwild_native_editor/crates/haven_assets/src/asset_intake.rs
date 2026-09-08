use haven_core::{ObjectKind, TileKind};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{create_dir_all, read_dir, read_to_string, write},
    path::{Path, PathBuf},
};

pub const ASSET_INTAKE_CATALOG_PATH: &str = "content/assets/intake/asset_intake_catalog_v0_1.json";
pub const ASSET_INTAKE_INBOX_PATH: &str = "assets/source/intake";
pub const ASSET_INTAKE_ORIGINAL_PATH: &str = "assets/source/original";
pub const ASSET_INTAKE_ATLAS_PATH: &str =
    "assets/generated/user_imports/havenwild_intake_atlas_v0_1.png";
pub const ASSET_INTAKE_MANIFEST_PATH: &str =
    "assets/generated/user_imports/havenwild_intake_atlas_v0_1.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetIntakeTargetKind {
    Tile,
    Object,
}

impl AssetIntakeTargetKind {
    pub const ALL: [Self; 2] = [Self::Tile, Self::Object];

    pub fn label(self) -> &'static str {
        match self {
            Self::Tile => "Tile",
            Self::Object => "Object",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIntakeTarget {
    pub kind: AssetIntakeTargetKind,
    pub code: String,
}

impl AssetIntakeTarget {
    pub fn stable_id(&self) -> String {
        match self.kind {
            AssetIntakeTargetKind::Tile => format!("tile/{}", self.code),
            AssetIntakeTargetKind::Object => format!("object/{}", self.code),
        }
    }

    pub fn label(&self) -> String {
        match self.kind {
            AssetIntakeTargetKind::Tile => TileKind::from_code(&self.code)
                .map(|kind| format!("Tile: {}", kind.label()))
                .unwrap_or_else(|| format!("Tile: {}", self.code)),
            AssetIntakeTargetKind::Object => ObjectKind::from_code(&self.code)
                .map(|kind| format!("Object: {}", kind.label()))
                .unwrap_or_else(|| format!("Object: {}", self.code)),
        }
    }

    pub fn is_valid(&self) -> bool {
        match self.kind {
            AssetIntakeTargetKind::Tile => TileKind::from_code(&self.code).is_some(),
            AssetIntakeTargetKind::Object => ObjectKind::from_code(&self.code).is_some(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetLicenseStatus {
    ProjectOwned,
    Cc0,
    CcBy,
    ThirdPartyApproved,
    Unverified,
    Blocked,
}

impl AssetLicenseStatus {
    pub const ALL: [Self; 6] = [
        Self::ProjectOwned,
        Self::Cc0,
        Self::CcBy,
        Self::ThirdPartyApproved,
        Self::Unverified,
        Self::Blocked,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ProjectOwned => "Project Owned",
            Self::Cc0 => "CC0",
            Self::CcBy => "CC-BY",
            Self::ThirdPartyApproved => "Approved Third Party",
            Self::Unverified => "Unverified",
            Self::Blocked => "Blocked",
        }
    }

    pub fn permits_promotion(self) -> bool {
        matches!(
            self,
            Self::ProjectOwned | Self::Cc0 | Self::CcBy | Self::ThirdPartyApproved
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetPromotionState {
    Draft,
    Approved,
    Rejected,
}

impl AssetPromotionState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Draft => "Draft",
            Self::Approved => "Approved",
            Self::Rejected => "Rejected",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetSliceRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetPivot {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetFootprintRecipe {
    pub visual: [i32; 4],
    pub collision: [i32; 4],
    pub interaction: [i32; 4],
}

impl Default for AssetFootprintRecipe {
    fn default() -> Self {
        Self {
            visual: [0, 0, 1, 1],
            collision: [0, 0, 1, 1],
            interaction: [0, 0, 1, 1],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetLicenseDeclaration {
    pub status: AssetLicenseStatus,
    #[serde(default)]
    pub attribution: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub accepted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIntakeRecipe {
    pub stable_id: String,
    pub display_name: String,
    pub source_path: String,
    pub target: AssetIntakeTarget,
    pub slice: AssetSliceRect,
    pub pivot: AssetPivot,
    #[serde(default)]
    pub footprint: AssetFootprintRecipe,
    pub license: AssetLicenseDeclaration,
    pub promotion_state: AssetPromotionState,
}

impl AssetIntakeRecipe {
    pub fn promotion_issues(&self, repo_root: impl AsRef<Path>) -> Vec<String> {
        let repo_root = repo_root.as_ref();
        let mut issues = Vec::new();
        if self.stable_id.trim().is_empty() || !self.stable_id.starts_with("import/") {
            issues.push("stableId must begin with import/".to_string());
        }
        if self.display_name.trim().is_empty() {
            issues.push("displayName is required".to_string());
        }
        if !self.target.is_valid() {
            issues.push(format!("unknown target {}", self.target.stable_id()));
        }
        let normalized = self.source_path.replace('\\', "/");
        if !(normalized.starts_with(&format!("{ASSET_INTAKE_INBOX_PATH}/"))
            || normalized.starts_with(&format!("{ASSET_INTAKE_ORIGINAL_PATH}/")))
        {
            issues.push(
                "sourcePath must stay inside assets/source/intake or assets/source/original"
                    .to_string(),
            );
        }
        if !repo_root.join(&self.source_path).is_file() {
            issues.push(format!("missing source image {}", self.source_path));
        }
        if self.slice.width == 0 || self.slice.height == 0 {
            issues.push("slice width and height must be positive".to_string());
        }
        for (name, footprint) in [
            ("visual", self.footprint.visual),
            ("collision", self.footprint.collision),
            ("interaction", self.footprint.interaction),
        ] {
            if footprint[2] <= 0 || footprint[3] <= 0 {
                issues.push(format!(
                    "{name} footprint width and height must be positive"
                ));
            }
        }
        if !self.license.status.permits_promotion() {
            issues.push(format!(
                "license status {} does not permit runtime promotion",
                self.license.status.label()
            ));
        }
        if !self.license.accepted {
            issues.push("license acceptance must be explicitly confirmed".to_string());
        }
        if self.license.status == AssetLicenseStatus::CcBy
            && self.license.attribution.trim().is_empty()
        {
            issues.push("CC-BY assets require attribution".to_string());
        }
        issues
    }

    pub fn promotion_ready(&self, repo_root: impl AsRef<Path>) -> bool {
        self.promotion_state == AssetPromotionState::Approved
            && self.promotion_issues(repo_root).is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIntakeCatalog {
    pub schema: String,
    #[serde(default)]
    pub updated: String,
    pub atlas_output: String,
    pub manifest_output: String,
    #[serde(default)]
    pub recipes: Vec<AssetIntakeRecipe>,
}

impl Default for AssetIntakeCatalog {
    fn default() -> Self {
        Self {
            schema: "havenwild.asset_intake_catalog.v0_1".to_string(),
            updated: "editor-created".to_string(),
            atlas_output: ASSET_INTAKE_ATLAS_PATH.to_string(),
            manifest_output: ASSET_INTAKE_MANIFEST_PATH.to_string(),
            recipes: Vec::new(),
        }
    }
}

impl AssetIntakeCatalog {
    pub fn load_default() -> Result<Self, String> {
        let root = repo_root_dir();
        let path = root.join(ASSET_INTAKE_CATALOG_PATH);
        let raw = read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        let catalog: Self = serde_json::from_str(&raw)
            .map_err(|error| format!("failed to parse {}: {error}", path.display()))?;
        catalog.validate_contract()?;
        Ok(catalog)
    }

    pub fn save_default(&self) -> Result<(), String> {
        self.validate_contract()?;
        let root = repo_root_dir();
        let path = root.join(ASSET_INTAKE_CATALOG_PATH);
        if let Some(parent) = path.parent() {
            create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        let raw = serde_json::to_string_pretty(self)
            .map_err(|error| format!("failed to serialize asset intake catalog: {error}"))?;
        write(&path, format!("{raw}\n"))
            .map_err(|error| format!("failed to write {}: {error}", path.display()))
    }

    pub fn validate_contract(&self) -> Result<(), String> {
        if self.schema != "havenwild.asset_intake_catalog.v0_1" {
            return Err(format!("unsupported asset intake schema {}", self.schema));
        }
        if self.atlas_output != ASSET_INTAKE_ATLAS_PATH {
            return Err(format!("atlasOutput must be {ASSET_INTAKE_ATLAS_PATH}"));
        }
        if self.manifest_output != ASSET_INTAKE_MANIFEST_PATH {
            return Err(format!(
                "manifestOutput must be {ASSET_INTAKE_MANIFEST_PATH}"
            ));
        }
        let mut ids = BTreeSet::new();
        let mut targets = BTreeSet::new();
        for recipe in &self.recipes {
            if !ids.insert(recipe.stable_id.clone()) {
                return Err(format!("duplicate intake stableId {}", recipe.stable_id));
            }
            if recipe.promotion_state == AssetPromotionState::Approved
                && !targets.insert(recipe.target.stable_id())
            {
                return Err(format!(
                    "multiple approved intake recipes target {}",
                    recipe.target.stable_id()
                ));
            }
        }
        Ok(())
    }

    pub fn scan_inbox(&mut self) -> Result<usize, String> {
        let root = repo_root_dir();
        let inbox = root.join(ASSET_INTAKE_INBOX_PATH);
        create_dir_all(&inbox)
            .map_err(|error| format!("failed to create {}: {error}", inbox.display()))?;
        let known: BTreeSet<String> = self
            .recipes
            .iter()
            .map(|recipe| recipe.source_path.replace('\\', "/"))
            .collect();
        let mut candidates = Vec::new();
        for entry in read_dir(&inbox)
            .map_err(|error| format!("failed to scan {}: {error}", inbox.display()))?
        {
            let path = entry
                .map_err(|error| format!("failed to read intake entry: {error}"))?
                .path();
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !matches!(extension.as_str(), "png" | "jpg" | "jpeg") {
                continue;
            }
            let relative = path
                .strip_prefix(&root)
                .map_err(|error| format!("failed to make intake path relative: {error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            if !known.contains(&relative) {
                candidates.push((relative, path));
            }
        }
        candidates.sort_by(|a, b| a.0.cmp(&b.0));
        let added = candidates.len();
        for (relative, path) in candidates {
            let stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("asset");
            let slug = slugify(stem);
            let target = if slug.contains("cave") {
                AssetIntakeTarget {
                    kind: AssetIntakeTargetKind::Object,
                    code: "cave_entrance".to_string(),
                }
            } else {
                AssetIntakeTarget {
                    kind: AssetIntakeTargetKind::Object,
                    code: "table".to_string(),
                }
            };
            self.recipes.push(AssetIntakeRecipe {
                stable_id: unique_import_id(&self.recipes, &slug),
                display_name: title_case(stem),
                source_path: relative,
                target,
                slice: AssetSliceRect {
                    x: 0,
                    y: 0,
                    width: 32,
                    height: 32,
                },
                pivot: AssetPivot { x: 16, y: 28 },
                footprint: AssetFootprintRecipe::default(),
                license: AssetLicenseDeclaration {
                    status: AssetLicenseStatus::Unverified,
                    attribution: String::new(),
                    source_url: String::new(),
                    accepted: false,
                },
                promotion_state: AssetPromotionState::Draft,
            });
        }
        self.recipes.sort_by(|a, b| a.stable_id.cmp(&b.stable_id));
        Ok(added)
    }

    pub fn summary(&self) -> String {
        let approved = self
            .recipes
            .iter()
            .filter(|recipe| recipe.promotion_state == AssetPromotionState::Approved)
            .count();
        format!(
            "{} intake recipe{} | {} approved",
            self.recipes.len(),
            if self.recipes.len() == 1 { "" } else { "s" },
            approved
        )
    }
}

pub fn repo_root_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("workspace root should be reachable from haven_assets")
}

fn unique_import_id(recipes: &[AssetIntakeRecipe], slug: &str) -> String {
    let base = format!("import/{slug}");
    if !recipes.iter().any(|recipe| recipe.stable_id == base) {
        return base;
    }
    for suffix in 2..10_000 {
        let candidate = format!("{base}_{suffix}");
        if !recipes.iter().any(|recipe| recipe.stable_id == candidate) {
            return candidate;
        }
    }
    format!("{base}_copy")
}

fn slugify(value: &str) -> String {
    let mut output = String::new();
    let mut underscore = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            underscore = false;
        } else if !underscore && !output.is_empty() {
            output.push('_');
            underscore = true;
        }
    }
    output.trim_matches('_').to_string()
}

fn title_case(value: &str) -> String {
    value
        .split(|character: char| character == '_' || character == '-' || character.is_whitespace())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn license_gate_blocks_unverified_assets() {
        let mut catalog = AssetIntakeCatalog::default();
        catalog.recipes.push(AssetIntakeRecipe {
            stable_id: "import/test".to_string(),
            display_name: "Test".to_string(),
            source_path: "assets/source/intake/test.png".to_string(),
            target: AssetIntakeTarget {
                kind: AssetIntakeTargetKind::Tile,
                code: "grass".to_string(),
            },
            slice: AssetSliceRect {
                x: 0,
                y: 0,
                width: 32,
                height: 32,
            },
            pivot: AssetPivot { x: 16, y: 28 },
            footprint: AssetFootprintRecipe::default(),
            license: AssetLicenseDeclaration {
                status: AssetLicenseStatus::Unverified,
                attribution: String::new(),
                source_url: String::new(),
                accepted: false,
            },
            promotion_state: AssetPromotionState::Approved,
        });
        assert!(!catalog.recipes[0]
            .promotion_issues(repo_root_dir())
            .is_empty());
    }

    #[test]
    fn target_ids_match_palette_ids() {
        assert_eq!(
            AssetIntakeTarget {
                kind: AssetIntakeTargetKind::Object,
                code: "table".to_string(),
            }
            .stable_id(),
            "object/table"
        );
    }
}
