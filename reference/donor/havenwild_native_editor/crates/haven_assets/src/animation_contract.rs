use serde::{Deserialize, Serialize};
use std::fs::read_to_string;

pub const CHARACTER_ANIMATION_CONTRACT_PATH: &str =
    "content/animations/character_animation_contract_v0_10.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterAnimationContractManifest {
    pub schema: String,
    pub name: String,
    pub locked_first_sheet: LockedFirstSheet,
    pub direction_order: Vec<String>,
    pub core_rule: String,
    pub previews: CharacterAnimationPreviews,
    pub specs: Vec<String>,
    pub schemas: Vec<String>,
    pub rust_pseudocode: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct LockedFirstSheet {
    pub cell_size: [i32; 2],
    pub directions: usize,
    pub frames_per_direction: usize,
    pub sheet_size: [i32; 2],
    pub root_anchor: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CharacterAnimationPreviews {
    pub socket_template: String,
    pub architecture: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AnimationSheetMetadata {
    pub asset_id: String,
    pub image: String,
    pub cell_size: [i32; 2],
    pub directions: Vec<String>,
    pub frames_per_direction: usize,
    pub grid_alignment: Option<serde_json::Value>,
    pub frame_rigs: Vec<FrameRigMetadata>,
    pub layer_kind: Option<String>,
    pub conforms_to_template: Option<String>,
    pub validation_state: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct FrameRigMetadata {
    pub animation: String,
    pub direction: String,
    pub frame: usize,
    pub cell: Option<[i32; 2]>,
    pub root_anchor: [f32; 2],
    pub sockets: serde_json::Value,
    pub depth_flags: Option<Vec<String>>,
    pub events: Option<Vec<serde_json::Value>>,
}

impl CharacterAnimationContractManifest {
    pub fn load_from_path(path: &str) -> Result<Self, String> {
        let raw =
            read_to_string(path).map_err(|error| format!("failed to read {path}: {error}"))?;
        serde_json::from_str(&raw).map_err(|error| format!("failed to parse {path}: {error}"))
    }
}

pub fn load_character_animation_contract_from_path(
    path: &str,
) -> Result<CharacterAnimationContractManifest, String> {
    CharacterAnimationContractManifest::load_from_path(path)
}

pub fn validate_character_animation_contract_manifest(
    manifest: &CharacterAnimationContractManifest,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if manifest.locked_first_sheet.cell_size != [64, 96] {
        warnings.push(format!(
            "locked first animation cell size is {:?}, expected [64, 96]",
            manifest.locked_first_sheet.cell_size
        ));
    }
    if manifest.locked_first_sheet.directions != 8 {
        warnings.push(format!(
            "locked first animation sheet defines {} directions, expected 8",
            manifest.locked_first_sheet.directions
        ));
    }
    if manifest.locked_first_sheet.frames_per_direction != 8 {
        warnings.push(format!(
            "locked first animation sheet defines {} walk frames, expected 8",
            manifest.locked_first_sheet.frames_per_direction
        ));
    }
    if manifest.locked_first_sheet.sheet_size != [512, 768] {
        warnings.push(format!(
            "locked first animation sheet size is {:?}, expected [512, 768]",
            manifest.locked_first_sheet.sheet_size
        ));
    }
    if manifest.direction_order.len() != manifest.locked_first_sheet.directions {
        warnings.push(format!(
            "animation direction order lists {} entries but the contract expects {} directions",
            manifest.direction_order.len(),
            manifest.locked_first_sheet.directions
        ));
    }
    if !manifest
        .locked_first_sheet
        .root_anchor
        .to_ascii_lowercase()
        .contains("bottom-center")
    {
        warnings.push(
            "animation contract root anchor should stay aligned to the bottom-center foot anchor"
                .to_string(),
        );
    }
    warnings
}

pub fn validate_animation_sheet_metadata(
    sheet: &AnimationSheetMetadata,
    manifest: &CharacterAnimationContractManifest,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if sheet.cell_size != manifest.locked_first_sheet.cell_size {
        warnings.push(format!(
            "{} uses cell size {:?}, expected {:?}",
            sheet.asset_id, sheet.cell_size, manifest.locked_first_sheet.cell_size
        ));
    }
    if sheet.frames_per_direction != manifest.locked_first_sheet.frames_per_direction {
        warnings.push(format!(
            "{} uses {} frames per direction, expected {}",
            sheet.asset_id,
            sheet.frames_per_direction,
            manifest.locked_first_sheet.frames_per_direction
        ));
    }
    if sheet.directions.len() != manifest.locked_first_sheet.directions {
        warnings.push(format!(
            "{} lists {} directions, expected {}",
            sheet.asset_id,
            sheet.directions.len(),
            manifest.locked_first_sheet.directions
        ));
    }
    if sheet.frame_rigs.is_empty() {
        warnings.push(format!("{} does not define any frame rigs", sheet.asset_id));
    }
    warnings
}
