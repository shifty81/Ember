use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const STARTER_TOWN_AUTHORITY_SCHEMA: &str = "havenwild.starter_town_authority.v0_2";
pub const STARTER_TOWN_MIN_CHUNK_DISTANCE: u32 = 2;
pub const STARTER_TOWN_PREFERRED_CHUNK_DISTANCE: u32 = 4;
pub const STARTER_TOWN_MAX_CHUNK_DISTANCE: u32 = 6;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StarterTownAuthority {
    pub schema: String,
    pub town_id: String,
    pub display_name: String,
    pub placement: StarterTownPlacementContract,
    #[serde(default)]
    pub capital: CapitalGenerationContract,
    #[serde(default)]
    pub required_services: Vec<StarterTownService>,
    #[serde(default)]
    pub authored_characters: Vec<StarterTownCharacter>,
    pub supporting_population: StarterTownPopulationRules,
    pub first_arrival: StarterTownArrivalContract,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapitalGenerationContract {
    pub permanent_capital: bool,
    pub generate_before_player_spawn: bool,
    pub authored_core_required: bool,
    pub require_hand_authoring_pass: bool,
    pub protect_authored_cells: bool,
    #[serde(default)]
    pub fixed_district_ids: Vec<String>,
    #[serde(default)]
    pub fixed_landmark_ids: Vec<String>,
    #[serde(default)]
    pub seed_variable_domains: Vec<String>,
}

impl Default for CapitalGenerationContract {
    fn default() -> Self {
        Self {
            permanent_capital: true,
            generate_before_player_spawn: true,
            authored_core_required: true,
            require_hand_authoring_pass: true,
            protect_authored_cells: true,
            fixed_district_ids: Vec::new(),
            fixed_landmark_ids: Vec::new(),
            seed_variable_domains: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StarterTownPlacementContract {
    pub minimum_spawn_distance_chunks: u32,
    pub preferred_spawn_distance_chunks: u32,
    pub maximum_spawn_distance_chunks: u32,
    pub require_traversable_route: bool,
    pub require_freshwater: bool,
    pub require_farmland: bool,
    pub require_woodland: bool,
    pub reserve_or_repair_when_missing: bool,
    #[serde(default)]
    pub forbidden_barriers: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarterTownService {
    pub id: String,
    pub building_id: String,
    pub required: bool,
    #[serde(default)]
    pub service_tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarterTownCharacter {
    pub id: String,
    pub display_name: String,
    pub sex: String,
    pub age_group: String,
    pub role: String,
    pub workplace_service_id: String,
    pub recipe_template_id: String,
    pub schedule_id: String,
    pub interaction_profile_id: String,
    #[serde(default)]
    pub relationship_links: Vec<String>,
    #[serde(default)]
    pub tutorial_tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarterTownPopulationRules {
    pub generation_profile_id: String,
    pub minimum_residents: u32,
    pub maximum_residents: u32,
    pub minimum_visitors: u32,
    pub maximum_visitors: u32,
    #[serde(default)]
    pub allowed_content_tiers: Vec<String>,
    #[serde(default)]
    pub blocked_content_tiers: Vec<String>,
    #[serde(default)]
    pub age_ranges: Vec<PopulationAgeRange>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationAgeRange {
    pub age_group: String,
    pub minimum: u32,
    pub maximum: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StarterTownArrivalContract {
    pub objective_id: String,
    pub initial_contact_character_id: String,
    pub civic_contact_character_id: String,
    #[serde(default)]
    pub guidance_methods: Vec<String>,
    #[serde(default)]
    pub tutorial_sequence: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StarterTownSiteCandidate {
    pub chunk_distance_from_spawn: u32,
    pub buildable_ratio: f32,
    pub freshwater_score: f32,
    pub farmland_score: f32,
    pub woodland_score: f32,
    pub route_score: f32,
    pub hazard_score: f32,
    pub expansion_score: f32,
    pub has_impassable_barrier: bool,
}

impl StarterTownAuthority {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != STARTER_TOWN_AUTHORITY_SCHEMA {
            return Err(format!("unsupported starter-town schema {}", self.schema));
        }
        if self.town_id.trim().is_empty() || self.display_name.trim().is_empty() {
            return Err("starter town requires an id and display name".to_string());
        }
        self.placement.validate()?;
        self.capital.validate()?;

        let mut service_ids = BTreeSet::new();
        for service in &self.required_services {
            if service.id.trim().is_empty()
                || service.building_id.trim().is_empty()
                || !service_ids.insert(service.id.clone())
            {
                return Err(format!(
                    "invalid or duplicate starter-town service {}",
                    service.id
                ));
            }
        }
        if self
            .required_services
            .iter()
            .filter(|service| service.required)
            .count()
            < 8
        {
            return Err("starter town requires at least eight guaranteed services".to_string());
        }

        let mut character_ids = BTreeSet::new();
        for character in &self.authored_characters {
            if character.id.trim().is_empty() || !character_ids.insert(character.id.clone()) {
                return Err(format!(
                    "invalid or duplicate authored character {}",
                    character.id
                ));
            }
            if !matches!(character.sex.as_str(), "male" | "female") {
                return Err(format!(
                    "{} has invalid sex {}",
                    character.id, character.sex
                ));
            }
            if !matches!(
                character.age_group.as_str(),
                "child" | "teen" | "adult" | "elder"
            ) {
                return Err(format!(
                    "{} has invalid age group {}",
                    character.id, character.age_group
                ));
            }
            if !service_ids.contains(&character.workplace_service_id) {
                return Err(format!(
                    "{} references unknown service {}",
                    character.id, character.workplace_service_id
                ));
            }
        }
        if self.authored_characters.len() < 10 {
            return Err(
                "starter town requires at least ten authored recurring characters".to_string(),
            );
        }
        for character in &self.authored_characters {
            for linked in &character.relationship_links {
                if !character_ids.contains(linked) {
                    return Err(format!(
                        "{} links to unknown character {}",
                        character.id, linked
                    ));
                }
            }
        }
        self.supporting_population.validate()?;
        if !character_ids.contains(&self.first_arrival.initial_contact_character_id)
            || !character_ids.contains(&self.first_arrival.civic_contact_character_id)
        {
            return Err("first-arrival contacts must reference authored characters".to_string());
        }
        Ok(())
    }
}

impl CapitalGenerationContract {
    pub fn validate(&self) -> Result<(), String> {
        if !self.permanent_capital
            || !self.generate_before_player_spawn
            || !self.authored_core_required
            || !self.require_hand_authoring_pass
            || !self.protect_authored_cells
        {
            return Err(
                "Willowmere must remain a permanent authored capital generated before player spawn"
                    .to_string(),
            );
        }
        if self.fixed_district_ids.len() < 4 {
            return Err(
                "Willowmere capital contract requires at least four fixed districts".to_string(),
            );
        }
        if self.fixed_landmark_ids.len() < 3 {
            return Err(
                "Willowmere capital contract requires at least three fixed landmarks".to_string(),
            );
        }
        Ok(())
    }
}

impl StarterTownPlacementContract {
    pub fn validate(&self) -> Result<(), String> {
        if self.minimum_spawn_distance_chunks < STARTER_TOWN_MIN_CHUNK_DISTANCE
            || self.preferred_spawn_distance_chunks < self.minimum_spawn_distance_chunks
            || self.maximum_spawn_distance_chunks < self.preferred_spawn_distance_chunks
            || self.maximum_spawn_distance_chunks > STARTER_TOWN_MAX_CHUNK_DISTANCE
        {
            return Err(
                "starter-town spawn-distance contract must remain within 2..=6 chunks".to_string(),
            );
        }
        if !self.require_traversable_route || !self.reserve_or_repair_when_missing {
            return Err(
                "starter town must guarantee a traversable route and repair fallback".to_string(),
            );
        }
        Ok(())
    }
}

impl StarterTownPopulationRules {
    pub fn validate(&self) -> Result<(), String> {
        if self.minimum_residents == 0 || self.maximum_residents < self.minimum_residents {
            return Err("invalid starter-town resident range".to_string());
        }
        if self.maximum_visitors < self.minimum_visitors {
            return Err("invalid starter-town visitor range".to_string());
        }
        let mut ages = BTreeSet::new();
        for range in &self.age_ranges {
            if !matches!(
                range.age_group.as_str(),
                "child" | "teen" | "adult" | "elder"
            ) || range.maximum < range.minimum
                || !ages.insert(range.age_group.clone())
            {
                return Err(format!("invalid population age range {}", range.age_group));
            }
        }
        if ages
            != BTreeSet::from([
                "child".to_string(),
                "teen".to_string(),
                "adult".to_string(),
                "elder".to_string(),
            ])
        {
            return Err("starter-town population must define all four age groups".to_string());
        }
        Ok(())
    }
}

pub fn score_starter_town_site(candidate: StarterTownSiteCandidate) -> Option<f32> {
    if candidate.has_impassable_barrier
        || candidate.chunk_distance_from_spawn < STARTER_TOWN_MIN_CHUNK_DISTANCE
        || candidate.chunk_distance_from_spawn > STARTER_TOWN_MAX_CHUNK_DISTANCE
        || candidate.buildable_ratio < 0.55
        || candidate.route_score < 0.45
    {
        return None;
    }
    let distance_score = 1.0
        - ((candidate.chunk_distance_from_spawn as f32
            - STARTER_TOWN_PREFERRED_CHUNK_DISTANCE as f32)
            .abs()
            / STARTER_TOWN_MAX_CHUNK_DISTANCE as f32);
    Some(
        candidate.buildable_ratio * 0.24
            + candidate.freshwater_score.clamp(0.0, 1.0) * 0.14
            + candidate.farmland_score.clamp(0.0, 1.0) * 0.14
            + candidate.woodland_score.clamp(0.0, 1.0) * 0.10
            + candidate.route_score.clamp(0.0, 1.0) * 0.18
            + (1.0 - candidate.hazard_score.clamp(0.0, 1.0)) * 0.08
            + candidate.expansion_score.clamp(0.0, 1.0) * 0.07
            + distance_score.clamp(0.0, 1.0) * 0.05,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_scoring_rejects_distant_or_blocked_candidates() {
        let candidate = StarterTownSiteCandidate {
            chunk_distance_from_spawn: 7,
            buildable_ratio: 0.9,
            freshwater_score: 1.0,
            farmland_score: 1.0,
            woodland_score: 1.0,
            route_score: 1.0,
            hazard_score: 0.0,
            expansion_score: 1.0,
            has_impassable_barrier: false,
        };
        assert!(score_starter_town_site(candidate).is_none());
    }
}
