use super::{
    validate_rect, BuildingRecipeDefinition, BuildingRecipePiece, BuildingRecipePieceKind,
};
use std::collections::BTreeSet;

pub(super) fn materialize(
    recipe: &BuildingRecipeDefinition,
    pieces: &mut Vec<BuildingRecipePiece>,
) {
    if let Some(components) = &recipe.roof.components {
        let [x, y, w, h] = recipe.roof.rect;
        for dy in 0..h {
            for dx in 0..w {
                let asset_id = if dx == 0 && dy == 0 {
                    &components.north_west_corner
                } else if dx == w - 1 && dy == 0 {
                    &components.north_east_corner
                } else if dx == 0 && dy == h - 1 {
                    &components.south_west_corner
                } else if dx == w - 1 && dy == h - 1 {
                    &components.south_east_corner
                } else if dy == 0 {
                    &components.north_edge
                } else if dy == h - 1 {
                    &components.south_edge
                } else if dx == 0 {
                    &components.west_edge
                } else if dx == w - 1 {
                    &components.east_edge
                } else {
                    &components.field
                };
                pieces.push(BuildingRecipePiece {
                    id: format!("{}:roof:{}:{}", recipe.id, x + dx, y + dy),
                    kind: BuildingRecipePieceKind::Roof,
                    asset_id: asset_id.clone(),
                    level: recipe.roof.level,
                    tile: [x + dx, y + dy],
                    state: None,
                    occlusion_group: Some(recipe.roof.occlusion_group.clone()),
                });
            }
        }
    } else if !recipe.roof.authored_modules.is_empty() {
        for module in &recipe.roof.authored_modules {
            pieces.push(BuildingRecipePiece {
                id: format!("{}:roof:module:{}", recipe.id, module.id),
                kind: BuildingRecipePieceKind::Roof,
                asset_id: module.asset_id.clone(),
                level: recipe.roof.level,
                tile: module.tile,
                state: module.state.clone(),
                occlusion_group: Some(recipe.roof.occlusion_group.clone()),
            });
        }
    } else if let Some(asset_id) = &recipe.roof.authored_module_asset_id {
        pieces.push(BuildingRecipePiece {
            id: format!("{}:roof:module", recipe.id),
            kind: BuildingRecipePieceKind::Roof,
            asset_id: asset_id.clone(),
            level: recipe.roof.level,
            tile: [recipe.roof.rect[0], recipe.roof.rect[1]],
            state: None,
            occlusion_group: Some(recipe.roof.occlusion_group.clone()),
        });
    }
}

pub(super) fn validate_structure(recipe: &BuildingRecipeDefinition, errors: &mut Vec<String>) {
    validate_rect(recipe, recipe.roof.rect, "roof rect", errors);
    let mut module_ids = BTreeSet::new();
    for module in &recipe.roof.authored_modules {
        if module.id.trim().is_empty() || !module_ids.insert(module.id.as_str()) {
            errors.push(format!(
                "{} roof authored module ids must be unique and non-empty",
                recipe.id
            ));
        }
        if module.tile[0] < 0
            || module.tile[1] < 0
            || module.tile[0] >= recipe.footprint[0] as i32
            || module.tile[1] >= recipe.footprint[1] as i32
        {
            errors.push(format!(
                "{} roof authored module {} anchor {:?} lies outside the footprint",
                recipe.id, module.id, module.tile
            ));
        }
    }
    if recipe.roof.components.is_some()
        && (!recipe.roof.authored_modules.is_empty()
            || recipe.roof.authored_module_asset_id.is_some())
    {
        errors.push(format!(
            "{} roof may not mix nine-slice components with authored modules",
            recipe.id
        ));
    }
    if !recipe.roof.authored_modules.is_empty() && recipe.roof.authored_module_asset_id.is_some() {
        errors.push(format!(
            "{} roof may not mix authoredModules with legacy authoredModuleAssetId",
            recipe.id
        ));
    }
    if !recipe.roof.camera_local_occlusion {
        errors.push(format!("{} roof occlusion must be camera-local", recipe.id));
    }
}
