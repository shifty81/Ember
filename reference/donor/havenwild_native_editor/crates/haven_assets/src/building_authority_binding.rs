use crate::{
    building_instance::BuildingInstanceRegistry,
    building_recipe::BuildingRecipeRegistry,
};
use haven_core::BuildingDefinition;
use serde::{Deserialize, Serialize};

/// Explicit cross-authority binding between W55 logical building topology and the W46
/// certified visual/placement pipeline. This is a reference bridge only: it does not copy
/// rooms, dimensions, collision, visual pieces, or mutable instance state between systems.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildingAuthorityBinding {
    pub logical_instance_id: String,
    pub visual_instance_id: String,
    pub recipe_id: String,
}

impl BuildingAuthorityBinding {
    pub fn validate(
        &self,
        building: &BuildingDefinition,
        recipes: &BuildingRecipeRegistry,
        instances: &BuildingInstanceRegistry,
    ) -> Vec<String> {
        let mut issues = Vec::new();
        if self.logical_instance_id != building.id.as_str() {
            issues.push(format!(
                "binding logical id {} does not match BuildingLayout instance {}",
                self.logical_instance_id,
                building.id.as_str()
            ));
        }
        let Some(recipe) = recipes.entry(&self.recipe_id) else {
            issues.push(format!("binding references unknown BuildingRecipe {}", self.recipe_id));
            return issues;
        };
        let Some(instance) = instances.entry(&self.visual_instance_id) else {
            issues.push(format!(
                "binding references unknown BuildingInstance {}",
                self.visual_instance_id
            ));
            return issues;
        };
        if instance.recipe_id != recipe.id {
            issues.push(format!(
                "visual instance {} uses recipe {}, binding expects {}",
                instance.id, instance.recipe_id, recipe.id
            ));
        }
        issues
    }
}
