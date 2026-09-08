use serde::{Deserialize, Serialize};

pub const CHARACTER_VITALS_CATALOG_SCHEMA: &str = "havenwild.character_vitals_catalog.v0_1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterVitalsCatalog {
    pub schema: String,
    pub base_health: f32,
    pub base_stamina: f32,
    pub hunger_max: f32,
    pub thirst_max: f32,
    pub standing_stamina_regen_per_second: f32,
    pub sheltered_stamina_regen_per_second: f32,
    pub rested_capacity_recovery_per_second: f32,
    pub sprint_stamina_cost_per_second: f32,
    pub hunger_drain_per_minute: f32,
    pub thirst_drain_per_minute: f32,
    pub exhaustion_threshold_ratio: f32,
    pub extended_action_capacity_loss_ratio: f32,
    pub minimum_stamina_capacity_ratio: f32,
    pub maximum_rested_seconds: f32,
    pub maximum_comfort_bonus_ratio: f32,
}

impl CharacterVitalsCatalog {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CHARACTER_VITALS_CATALOG_SCHEMA {
            return Err(format!(
                "unsupported character vitals schema {}",
                self.schema
            ));
        }
        if self.base_health <= 0.0 || self.base_stamina <= 0.0 {
            return Err("health and stamina bases must be positive".to_string());
        }
        if self.hunger_max <= 0.0 || self.thirst_max <= 0.0 {
            return Err("sustenance maxima must be positive".to_string());
        }
        if !(0.0..1.0).contains(&self.exhaustion_threshold_ratio) {
            return Err("exhaustion threshold must be between zero and one".to_string());
        }
        if !(0.0..1.0).contains(&self.extended_action_capacity_loss_ratio) {
            return Err("extended-action capacity loss must be between zero and one".to_string());
        }
        if !(0.0..=1.0).contains(&self.minimum_stamina_capacity_ratio) {
            return Err("minimum stamina capacity ratio must be between zero and one".to_string());
        }
        if self.maximum_rested_seconds <= 0.0 || self.maximum_rested_seconds > 900.0 {
            return Err(
                "rested duration must be positive and no longer than 15 minutes".to_string(),
            );
        }
        Ok(())
    }
}
