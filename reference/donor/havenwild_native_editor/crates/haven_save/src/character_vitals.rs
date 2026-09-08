use serde::{Deserialize, Serialize};

pub const CHARACTER_VITALS_SCHEMA: &str = "havenwild.character_vitals.v0_1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CharacterVitalsState {
    #[serde(default = "default_schema")]
    pub schema: String,
    #[serde(default = "default_hundred")]
    pub health: f32,
    #[serde(default = "default_hundred")]
    pub health_max: f32,
    #[serde(default = "default_hundred")]
    pub stamina: f32,
    #[serde(default = "default_hundred")]
    pub stamina_base_max: f32,
    #[serde(default = "default_hundred")]
    pub stamina_current_max: f32,
    #[serde(default)]
    pub fatigue_capacity_loss: f32,
    #[serde(default = "default_hundred")]
    pub hunger: f32,
    #[serde(default = "default_hundred")]
    pub thirst: f32,
    #[serde(default)]
    pub rested_seconds_remaining: f32,
    #[serde(default)]
    pub comfort_ratio: f32,
}

impl Default for CharacterVitalsState {
    fn default() -> Self {
        Self {
            schema: default_schema(),
            health: 100.0,
            health_max: 100.0,
            stamina: 100.0,
            stamina_base_max: 100.0,
            stamina_current_max: 100.0,
            fatigue_capacity_loss: 0.0,
            hunger: 100.0,
            thirst: 100.0,
            rested_seconds_remaining: 0.0,
            comfort_ratio: 0.0,
        }
    }
}

impl CharacterVitalsState {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != CHARACTER_VITALS_SCHEMA {
            return Err(format!(
                "unsupported character vitals schema {}",
                self.schema
            ));
        }
        if self.health_max <= 0.0 || self.stamina_base_max <= 0.0 || self.stamina_current_max <= 0.0
        {
            return Err("vital maxima must be positive".to_string());
        }
        Ok(())
    }

    pub fn stamina_ratio(&self) -> f32 {
        if self.stamina_current_max <= f32::EPSILON {
            0.0
        } else {
            (self.stamina / self.stamina_current_max).clamp(0.0, 1.0)
        }
    }
}

fn default_schema() -> String {
    CHARACTER_VITALS_SCHEMA.to_string()
}
fn default_hundred() -> f32 {
    100.0
}
