use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{create_dir_all, read_to_string, write};
use std::path::{Path, PathBuf};

pub const WORLD_SOCIAL_STATE_SCHEMA: &str = "havenwild.world_social_state.v1";
pub const WORLD_SOCIAL_STATE_RELATIVE_PATH: &str = "social/world_social_state.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettlementKind {
    Capital,
    City,
    Village,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HousingListingMode {
    Rent,
    Purchase,
    RentOrPurchase,
    LeaseToOwn,
    Unlisted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HousingTenure {
    Guest,
    Tenant,
    LeaseToOwn,
    Owner,
    CoOwner,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HousingPropertyState {
    pub property_id: String,
    pub settlement_id: String,
    pub settlement_kind: SettlementKind,
    pub display_name: String,
    pub listing_mode: HousingListingMode,
    pub rent_per_season: u32,
    pub purchase_price: u32,
    #[serde(default)]
    pub household_id: Option<String>,
    #[serde(default)]
    pub occupant_ids: Vec<String>,
    #[serde(default)]
    pub protected_city_plot: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipTargetKind {
    Player,
    Npc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipStage {
    Stranger,
    Acquaintance,
    Friend,
    CloseFriend,
    Dating,
    Partner,
    Engaged,
    Married,
    Family,
    Estranged,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelationshipRecord {
    pub relationship_id: String,
    pub actor_id: String,
    pub target_id: String,
    pub target_kind: RelationshipTargetKind,
    pub stage: RelationshipStage,
    pub friendship: i32,
    pub trust: i32,
    pub romance: i32,
    pub consent_confirmed: bool,
    pub adult_eligibility_confirmed: bool,
    #[serde(default)]
    pub shared_household_id: Option<String>,
    #[serde(default)]
    pub last_major_event: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HouseholdMemberRole {
    Adult,
    Spouse,
    Partner,
    Child,
    Dependent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseholdMemberState {
    pub actor_id: String,
    pub role: HouseholdMemberRole,
    pub is_player: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FamilyFormationRoute {
    Pregnancy,
    Adoption,
    ExistingChildJoinsHousehold,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PregnancyState {
    pub carrying_actor_id: String,
    pub co_parent_actor_ids: Vec<String>,
    pub progress_days: u32,
    pub expected_duration_days: u32,
    pub visual_stage: u8,
    pub visual_asset_role: String,
    pub medically_paused: bool,
}

impl PregnancyState {
    pub fn default_visual_role() -> String {
        "elizawy_lpc.character.body.pregnancy".to_string()
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.carrying_actor_id.trim().is_empty() {
            return Err("pregnancy state requires a carrying adult actor".to_string());
        }
        if self.expected_duration_days == 0 || self.progress_days > self.expected_duration_days {
            return Err("pregnancy progress must fit within its expected duration".to_string());
        }
        if self.visual_stage > 3 {
            return Err("pregnancy visual stage must be in the range 0..=3".to_string());
        }
        if self.visual_asset_role.trim().is_empty() {
            return Err("pregnancy visual state requires an asset role".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HouseholdState {
    pub household_id: String,
    pub display_name: String,
    pub members: Vec<HouseholdMemberState>,
    #[serde(default)]
    pub primary_property_id: Option<String>,
    #[serde(default)]
    pub property_tenure: Option<HousingTenure>,
    #[serde(default)]
    pub shared_wallet_enabled: bool,
    #[serde(default)]
    pub family_formation_route: Option<FamilyFormationRoute>,
    #[serde(default)]
    pub pregnancy: Option<PregnancyState>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReputationRecord {
    pub subject_actor_id: String,
    pub scope_id: String,
    pub value: i32,
    pub rank: String,
    #[serde(default)]
    pub unlock_flags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldSocialState {
    pub schema: String,
    pub world_id: String,
    #[serde(default)]
    pub properties: BTreeMap<String, HousingPropertyState>,
    #[serde(default)]
    pub households: BTreeMap<String, HouseholdState>,
    #[serde(default)]
    pub relationships: BTreeMap<String, RelationshipRecord>,
    #[serde(default)]
    pub reputations: BTreeMap<String, ReputationRecord>,
}

impl WorldSocialState {
    pub fn new(world_id: impl Into<String>) -> Self {
        Self {
            schema: WORLD_SOCIAL_STATE_SCHEMA.to_string(),
            world_id: world_id.into(),
            properties: BTreeMap::new(),
            households: BTreeMap::new(),
            relationships: BTreeMap::new(),
            reputations: BTreeMap::new(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != WORLD_SOCIAL_STATE_SCHEMA {
            return Err(format!(
                "unsupported world social-state schema {}",
                self.schema
            ));
        }
        if self.world_id.trim().is_empty() {
            return Err("world social state requires a world id".to_string());
        }
        for (id, property) in &self.properties {
            if id != &property.property_id || property.settlement_id.trim().is_empty() {
                return Err(format!("housing property {id} has inconsistent identity"));
            }
        }
        for (id, relationship) in &self.relationships {
            if id != &relationship.relationship_id {
                return Err(format!("relationship {id} has inconsistent identity"));
            }
            if relationship.actor_id == relationship.target_id {
                return Err(format!("relationship {id} cannot target the same actor"));
            }
            if relationship.stage >= RelationshipStage::Dating
                && (!relationship.consent_confirmed || !relationship.adult_eligibility_confirmed)
            {
                return Err(format!(
                    "relationship {id} requires adult consent before romance"
                ));
            }
        }
        for (id, household) in &self.households {
            if id != &household.household_id || household.members.is_empty() {
                return Err(format!(
                    "household {id} has inconsistent identity or no members"
                ));
            }
            if let Some(pregnancy) = &household.pregnancy {
                pregnancy.validate()?;
            }
        }
        Ok(())
    }
}

pub fn world_social_state_path(world_root: &Path) -> PathBuf {
    world_root.join(WORLD_SOCIAL_STATE_RELATIVE_PATH)
}

pub fn save_world_social_state(world_root: &Path, state: &WorldSocialState) -> Result<(), String> {
    state.validate()?;
    let path = world_social_state_path(world_root);
    if let Some(parent) = path.parent() {
        create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("json.tmp");
    let payload = serde_json::to_string_pretty(state).map_err(|error| error.to_string())?;
    write(&temporary, payload).map_err(|error| error.to_string())?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|error| error.to_string())?;
    }
    std::fs::rename(&temporary, &path).map_err(|error| error.to_string())
}

pub fn load_world_social_state(world_root: &Path) -> Result<WorldSocialState, String> {
    let path = world_social_state_path(world_root);
    let raw = read_to_string(path).map_err(|error| error.to_string())?;
    let state: WorldSocialState = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    state.validate()?;
    Ok(state)
}

pub fn load_or_create_world_social_state(
    world_root: &Path,
    world_id: impl Into<String>,
) -> Result<WorldSocialState, String> {
    let path = world_social_state_path(world_root);
    if path.is_file() {
        return load_world_social_state(world_root);
    }
    let state = WorldSocialState::new(world_id);
    save_world_social_state(world_root, &state)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn romantic_relationships_require_adult_consent() {
        let mut state = WorldSocialState::new("world_test");
        state.relationships.insert(
            "relationship_a_b".to_string(),
            RelationshipRecord {
                relationship_id: "relationship_a_b".to_string(),
                actor_id: "a".to_string(),
                target_id: "b".to_string(),
                target_kind: RelationshipTargetKind::Npc,
                stage: RelationshipStage::Dating,
                friendship: 40,
                trust: 40,
                romance: 40,
                consent_confirmed: false,
                adult_eligibility_confirmed: true,
                shared_household_id: None,
                last_major_event: None,
            },
        );
        assert!(state.validate().is_err());
    }

    #[test]
    fn pregnancy_visual_role_is_asset_driven() {
        let state = PregnancyState {
            carrying_actor_id: "adult_a".to_string(),
            co_parent_actor_ids: vec!["adult_b".to_string()],
            progress_days: 10,
            expected_duration_days: 84,
            visual_stage: 1,
            visual_asset_role: PregnancyState::default_visual_role(),
            medically_paused: false,
        };
        assert!(state.validate().is_ok());
        assert!(state.visual_asset_role.contains("elizawy_lpc"));
    }
}
