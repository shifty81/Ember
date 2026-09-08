use super::{BuildingLevelDefinition, BuildingRecipeDefinition, BuildingRecipePiece, BuildingRecipePieceKind, BuildingWallEdge};
use crate::placeable_asset_registry::{
    PublishedWorldAssetCertification, PublishedWorldAssetRegistry, PublishedWorldAssetRole,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum BuildingFurnishingKind {
    #[default]
    Furniture,
    Fixture,
    WallMounted,
    Workstation,
    Container,
    Decoration,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingInteractionSocket {
    pub id: String,
    pub purpose: String,
    #[serde(default)]
    pub offset: [i32; 2],
    #[serde(default)]
    pub reservation: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BuildingFurnishingPlacement {
    pub id: String,
    #[serde(default)]
    pub asset_id: Option<String>,
    pub room_id: String,
    pub tile: [i32; 2],
    #[serde(default)]
    pub kind: BuildingFurnishingKind,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub wall_attachment: Option<BuildingWallEdge>,
    #[serde(default)]
    pub visual_status: String,
    #[serde(default)]
    pub blocks_navigation: bool,
    #[serde(default)]
    pub interaction_sockets: Vec<BuildingInteractionSocket>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct BuildingLevelNavigation {
    #[serde(default)]
    pub walkable_rects: Vec<[i32; 4]>,
    #[serde(default)]
    pub blocked_tiles: Vec<[i32; 2]>,
}

impl BuildingRecipeDefinition {
    pub fn furnishing(&self, furnishing_id: &str) -> Option<(i32, &BuildingFurnishingPlacement)> {
        self.levels.iter().find_map(|level| {
            level
                .furnishings
                .iter()
                .find(|furnishing| furnishing.id == furnishing_id)
                .map(|furnishing| (level.level, furnishing))
        })
    }

    pub fn furnishing_at(
        &self,
        level_number: i32,
        local_tile: [i32; 2],
    ) -> Option<&BuildingFurnishingPlacement> {
        self.level(level_number)?
            .furnishings
            .iter()
            .find(|furnishing| furnishing.tile == local_tile)
    }

    pub fn furnishing_asset_ids(&self) -> BTreeSet<&str> {
        self.levels
            .iter()
            .flat_map(|level| level.furnishings.iter())
            .filter_map(|furnishing| furnishing.asset_id.as_deref())
            .collect()
    }
}

pub(super) fn materialize_level_furnishings(
    recipe_id: &str,
    level: &BuildingLevelDefinition,
) -> Vec<BuildingRecipePiece> {
    level
        .furnishings
        .iter()
        .filter_map(|furnishing| {
            let asset_id = furnishing.asset_id.as_ref()?;
            Some(BuildingRecipePiece {
                id: format!("{}:furnishing:{}", recipe_id, furnishing.id),
                kind: BuildingRecipePieceKind::Furnishing,
                asset_id: asset_id.clone(),
                level: level.level,
                tile: furnishing.tile,
                state: furnishing.state.clone(),
                occlusion_group: Some(format!("building.interior.level.{}", level.level)),
            })
        })
        .collect()
}

pub(super) fn validate_level_interior(
    recipe: &BuildingRecipeDefinition,
    level: &BuildingLevelDefinition,
    room_ids: &BTreeSet<&str>,
    furnishing_ids: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) {
    for rect in &level.navigation.walkable_rects {
        validate_local_rect(recipe, *rect, "navigation walkable rect", errors);
    }
    for tile in &level.navigation.blocked_tiles {
        validate_local_tile(recipe, *tile, "navigation blocked tile", errors);
    }
    for furnishing in &level.furnishings {
        if furnishing.id.trim().is_empty() {
            errors.push(format!("{} level {} has an empty furnishing id", recipe.id, level.id));
            continue;
        }
        if !furnishing_ids.insert(furnishing.id.clone()) {
            errors.push(format!("{} duplicates furnishing id {}", recipe.id, furnishing.id));
        }
        if !room_ids.contains(furnishing.room_id.as_str()) {
            errors.push(format!(
                "{} furnishing {} references unknown room {} on level {}",
                recipe.id, furnishing.id, furnishing.room_id, level.id
            ));
        }
        validate_local_tile(recipe, furnishing.tile, &format!("furnishing {}", furnishing.id), errors);
        if furnishing.asset_id.is_none() && furnishing.visual_status != "deferred_exact_source" {
            errors.push(format!(
                "{} furnishing {} has no asset and is not deferred_exact_source",
                recipe.id, furnishing.id
            ));
        }
        if furnishing.asset_id.is_some() && furnishing.visual_status == "deferred_exact_source" {
            errors.push(format!(
                "{} furnishing {} declares an asset while marked deferred_exact_source",
                recipe.id, furnishing.id
            ));
        }
        if furnishing.kind == BuildingFurnishingKind::WallMounted && furnishing.wall_attachment.is_none() {
            errors.push(format!(
                "{} wall-mounted furnishing {} requires wallAttachment",
                recipe.id, furnishing.id
            ));
        }
        let mut socket_ids = BTreeSet::new();
        for socket in &furnishing.interaction_sockets {
            if socket.id.trim().is_empty() || socket.purpose.trim().is_empty() {
                errors.push(format!("{} furnishing {} has an incomplete interaction socket", recipe.id, furnishing.id));
            }
            if !socket_ids.insert(socket.id.as_str()) {
                errors.push(format!("{} furnishing {} duplicates socket {}", recipe.id, furnishing.id, socket.id));
            }
        }
    }
}

pub(super) fn validate_interior_assets(
    recipe: &BuildingRecipeDefinition,
    assets: &PublishedWorldAssetRegistry,
    errors: &mut Vec<String>,
) {
    for level in &recipe.levels {
        for furnishing in &level.furnishings {
            let Some(asset_id) = furnishing.asset_id.as_deref() else { continue; };
            let Some(asset) = assets.resolve_alias(asset_id) else {
                errors.push(format!("{} furnishing {} references unknown PublishedWorldAsset {}", recipe.id, furnishing.id, asset_id));
                continue;
            };
            if matches!(
                asset.certification,
                PublishedWorldAssetCertification::Rejected
                    | PublishedWorldAssetCertification::Missing
                    | PublishedWorldAssetCertification::Placeholder
            ) {
                errors.push(format!(
                    "{} furnishing {} references unusable PublishedWorldAsset {} ({:?})",
                    recipe.id, furnishing.id, asset_id, asset.certification
                ));
            }
            match furnishing.kind {
                BuildingFurnishingKind::WallMounted => {
                    if asset.structure.is_none() && !matches!(&asset.role, PublishedWorldAssetRole::Placeable) {
                        errors.push(format!("{} wall-mounted furnishing {} has incompatible asset role", recipe.id, furnishing.id));
                    }
                }
                _ => {
                    if !matches!(&asset.role, PublishedWorldAssetRole::Placeable | PublishedWorldAssetRole::StructureComponent) {
                        errors.push(format!("{} furnishing {} has incompatible asset role {:?}", recipe.id, furnishing.id, &asset.role));
                    }
                }
            }
            if let Some(state) = furnishing.state.as_deref() {
                if !asset.supports_state(state) {
                    errors.push(format!("{} furnishing {} uses unsupported state {} for {}", recipe.id, furnishing.id, state, asset_id));
                }
            }
        }
    }
}

pub(crate) fn navigation_allows(level: &BuildingLevelDefinition, local_tile: [i32; 2]) -> bool {
    if level.navigation.blocked_tiles.iter().any(|tile| *tile == local_tile) {
        return false;
    }
    if level.navigation.walkable_rects.is_empty() {
        return true;
    }
    level.navigation.walkable_rects.iter().any(|rect| rect_contains(*rect, local_tile))
}

fn validate_local_rect(
    recipe: &BuildingRecipeDefinition,
    rect: [i32; 4],
    label: &str,
    errors: &mut Vec<String>,
) {
    let [x, y, w, h] = rect;
    if x < 0 || y < 0 || w <= 0 || h <= 0 || x + w > recipe.footprint[0] as i32 || y + h > recipe.footprint[1] as i32 {
        errors.push(format!("{} {} exceeds recipe footprint or is invalid", recipe.id, label));
    }
}

fn validate_local_tile(
    recipe: &BuildingRecipeDefinition,
    tile: [i32; 2],
    label: &str,
    errors: &mut Vec<String>,
) {
    if tile[0] < 0 || tile[1] < 0 || tile[0] >= recipe.footprint[0] as i32 || tile[1] >= recipe.footprint[1] as i32 {
        errors.push(format!("{} {} lies outside recipe footprint", recipe.id, label));
    }
}

fn rect_contains(rect: [i32; 4], tile: [i32; 2]) -> bool {
    tile[0] >= rect[0]
        && tile[1] >= rect[1]
        && tile[0] < rect[0] + rect[2]
        && tile[1] < rect[1] + rect[3]
}
