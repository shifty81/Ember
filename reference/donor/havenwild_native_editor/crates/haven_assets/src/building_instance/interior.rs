use super::*;
use crate::{
    building_recipe::{
        navigation_allows, BuildingFurnishingPlacement, BuildingOpeningKind,
        BuildingRecipeDefinition,
    },
    placeable_asset_registry::PublishedWorldAssetRegistry,
};

impl BuildingInstanceDefinition {
    pub fn navigation_allows_world_tile(
        &self,
        recipe: &BuildingRecipeDefinition,
        level_number: i32,
        world_tile: [i32; 2],
    ) -> bool {
        let Some(level) = recipe.level(level_number) else { return false; };
        let local_tile = self.local_tile(world_tile);

        // Door and archway cells are navigation thresholds, not ordinary room
        // interior cells. Their open/closed collision state is resolved by the
        // BuildingInstance opening-state lane, so a room walkable rectangle
        // must never trap a player on the inside of a valid doorway.
        if level.openings.iter().any(|opening| {
            opening.tile == local_tile
                && matches!(
                    opening.kind,
                    BuildingOpeningKind::Door | BuildingOpeningKind::Archway
                )
        }) {
            return true;
        }

        navigation_allows(level, local_tile)
    }

    pub fn furnishing_blocks_world_tile(
        &self,
        recipe: &BuildingRecipeDefinition,
        assets: &PublishedWorldAssetRegistry,
        level_number: i32,
        world_tile: [i32; 2],
        state_for: impl Fn(&BuildingFurnishingPlacement) -> Option<String>,
    ) -> bool {
        let Some(level) = recipe.level(level_number) else { return false; };
        let local = self.local_tile(world_tile);
        for furnishing in &level.furnishings {
            if furnishing.blocks_navigation && furnishing.tile == local {
                return true;
            }
            let Some(asset_id) = furnishing.asset_id.as_deref() else { continue; };
            let Some(asset) = assets.resolve_alias(asset_id) else { continue; };
            let state = state_for(furnishing);
            let footprint = asset.footprint_for_state(state.as_deref().or(furnishing.state.as_deref()));
            if !footprint.blocks_movement {
                continue;
            }
            let x0 = furnishing.tile[0] + footprint.collision_offset_x;
            let y0 = furnishing.tile[1] + footprint.collision_offset_y;
            let x1 = x0 + footprint.collision_w;
            let y1 = y0 + footprint.collision_h;
            if local[0] >= x0 && local[1] >= y0 && local[0] < x1 && local[1] < y1 {
                return true;
            }
        }
        false
    }

    pub fn furnishing_interaction_at<'a>(
        &self,
        recipe: &'a BuildingRecipeDefinition,
        assets: &PublishedWorldAssetRegistry,
        level_number: i32,
        world_tile: [i32; 2],
    ) -> Option<&'a BuildingFurnishingPlacement> {
        let level = recipe.level(level_number)?;
        let local = self.local_tile(world_tile);
        level.furnishings.iter().find(|furnishing| {
            if furnishing.tile == local && furnishing.asset_id.is_none() {
                return !furnishing.interaction_sockets.is_empty();
            }
            let Some(asset_id) = furnishing.asset_id.as_deref() else { return false; };
            let Some(asset) = assets.resolve_alias(asset_id) else { return false; };
            let footprint = asset.footprint_for_state(furnishing.state.as_deref());
            let x0 = furnishing.tile[0] + footprint.interaction_offset_x;
            let y0 = furnishing.tile[1] + footprint.interaction_offset_y;
            let x1 = x0 + footprint.interaction_w.max(1);
            let y1 = y0 + footprint.interaction_h.max(1);
            local[0] >= x0 && local[1] >= y0 && local[0] < x1 && local[1] < y1
        })
    }
}
