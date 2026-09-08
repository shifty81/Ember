use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, read_to_string, write};
use std::path::Path;

pub const WORLD_CREATION_SETTINGS_SCHEMA: &str = "havenwild.world_creation_settings.v1";
pub const WORLD_CREATION_SETTINGS_FILENAME: &str = "worldgen/world_creation_settings.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldSizePreset {
    Compact,
    Standard,
    Large,
    Huge,
    Massive,
    Enormous,
}

impl WorldSizePreset {
    pub const ALL: [Self; 6] = [
        Self::Compact,
        Self::Standard,
        Self::Large,
        Self::Huge,
        Self::Massive,
        Self::Enormous,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Standard => "Standard",
            Self::Large => "Large",
            Self::Huge => "Huge",
            Self::Massive => "Massive",
            Self::Enormous => "Enormous",
        }
    }

    pub const fn dimensions_tiles(self) -> [u32; 2] {
        match self {
            Self::Compact => [4_096, 4_096],
            Self::Standard => [8_192, 8_192],
            Self::Large => [16_384, 16_384],
            Self::Huge => [24_576, 24_576],
            Self::Massive => [32_768, 32_768],
            Self::Enormous => [65_536, 65_536],
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        cycle_enum(self, delta, &Self::ALL)
    }
}


#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldExtentMode {
    #[default]
    Finite,
    Endless,
}

impl WorldExtentMode {
    pub const ALL: [Self; 2] = [Self::Finite, Self::Endless];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Finite => "Finite",
            Self::Endless => "Endless",
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        cycle_enum(self, delta, &Self::ALL)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldDifficultyPreset {
    Story,
    Peaceful,
    Relaxed,
    #[default]
    Standard,
    Adventurous,
    Rugged,
    Harsh,
    Wild,
    Custom,
}

impl WorldDifficultyPreset {
    pub const ALL: [Self; 9] = [
        Self::Story,
        Self::Peaceful,
        Self::Relaxed,
        Self::Standard,
        Self::Adventurous,
        Self::Rugged,
        Self::Harsh,
        Self::Wild,
        Self::Custom,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Story => "Story",
            Self::Peaceful => "Peaceful",
            Self::Relaxed => "Relaxed",
            Self::Standard => "Standard",
            Self::Adventurous => "Adventurous",
            Self::Rugged => "Rugged",
            Self::Harsh => "Harsh",
            Self::Wild => "Wild",
            Self::Custom => "Custom",
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        cycle_enum(self, delta, &Self::ALL)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldDifficultyTuning {
    pub resource_abundance_percent: u8,
    pub survival_pressure_percent: u8,
    pub hostile_pressure_percent: u8,
    pub weather_severity_percent: u8,
    pub economy_pressure_percent: u8,
    pub recovery_penalty_percent: u8,
}

impl Default for WorldDifficultyTuning {
    fn default() -> Self {
        Self::for_preset(WorldDifficultyPreset::Standard)
    }
}

impl WorldDifficultyTuning {
    pub const fn for_preset(preset: WorldDifficultyPreset) -> Self {
        match preset {
            WorldDifficultyPreset::Story => Self::new(90, 10, 5, 15, 15, 5),
            WorldDifficultyPreset::Peaceful => Self::new(82, 18, 0, 20, 22, 10),
            WorldDifficultyPreset::Relaxed => Self::new(72, 30, 22, 30, 32, 20),
            WorldDifficultyPreset::Standard | WorldDifficultyPreset::Custom => {
                Self::new(58, 50, 50, 50, 50, 40)
            }
            WorldDifficultyPreset::Adventurous => Self::new(52, 58, 62, 58, 58, 50),
            WorldDifficultyPreset::Rugged => Self::new(45, 68, 70, 68, 65, 62),
            WorldDifficultyPreset::Harsh => Self::new(38, 80, 82, 80, 74, 76),
            WorldDifficultyPreset::Wild => Self::new(30, 92, 94, 92, 84, 90),
        }
    }

    const fn new(
        resource_abundance_percent: u8,
        survival_pressure_percent: u8,
        hostile_pressure_percent: u8,
        weather_severity_percent: u8,
        economy_pressure_percent: u8,
        recovery_penalty_percent: u8,
    ) -> Self {
        Self {
            resource_abundance_percent,
            survival_pressure_percent,
            hostile_pressure_percent,
            weather_severity_percent,
            economy_pressure_percent,
            recovery_penalty_percent,
        }
    }

    pub fn validate(self) -> Result<(), String> {
        let values = [
            self.resource_abundance_percent,
            self.survival_pressure_percent,
            self.hostile_pressure_percent,
            self.weather_severity_percent,
            self.economy_pressure_percent,
            self.recovery_penalty_percent,
        ];
        if values.into_iter().any(|value| value > 100) {
            return Err("difficulty tuning values must remain between 0 and 100".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LandformPreset {
    BalancedContinent,
    GrandIsland,
    Archipelago,
    Riverlands,
    BrokenCoast,
    Highlands,
    InlandBasin,
}

impl LandformPreset {
    pub const ALL: [Self; 7] = [
        Self::BalancedContinent,
        Self::GrandIsland,
        Self::Archipelago,
        Self::Riverlands,
        Self::BrokenCoast,
        Self::Highlands,
        Self::InlandBasin,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::BalancedContinent => "Balanced Continent",
            Self::GrandIsland => "Grand Island",
            Self::Archipelago => "Archipelago",
            Self::Riverlands => "Riverlands",
            Self::BrokenCoast => "Broken Coast",
            Self::Highlands => "Highlands",
            Self::InlandBasin => "Inland Basin",
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        cycle_enum(self, delta, &Self::ALL)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GenerationDensity {
    Low,
    Normal,
    High,
}

impl GenerationDensity {
    pub const ALL: [Self; 3] = [Self::Low, Self::Normal, Self::High];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Normal => "Normal",
            Self::High => "High",
        }
    }

    pub const fn normalized(self) -> f32 {
        match self {
            Self::Low => 0.25,
            Self::Normal => 0.50,
            Self::High => 0.78,
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        cycle_enum(self, delta, &Self::ALL)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BiomeVariety {
    Minimal,
    Normal,
    Diverse,
}

impl BiomeVariety {
    pub const ALL: [Self; 3] = [Self::Minimal, Self::Normal, Self::Diverse];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Minimal => "Minimal",
            Self::Normal => "Normal",
            Self::Diverse => "Diverse",
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        cycle_enum(self, delta, &Self::ALL)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WillowmereLocationPreference {
    Automatic,
    Coast,
    River,
    Estuary,
    Lake,
}

impl WillowmereLocationPreference {
    pub const ALL: [Self; 5] = [
        Self::Automatic,
        Self::Coast,
        Self::River,
        Self::Estuary,
        Self::Lake,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Automatic => "Automatic",
            Self::Coast => "Coast",
            Self::River => "River",
            Self::Estuary => "Estuary",
            Self::Lake => "Lake",
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        cycle_enum(self, delta, &Self::ALL)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandGenerationSettings {
    pub land_coverage_percent: u8,
    pub coastline_complexity: GenerationDensity,
    pub mountain_coverage: GenerationDensity,
    pub island_frequency: GenerationDensity,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HydrologyGenerationSettings {
    pub marine_water_abundance: GenerationDensity,
    pub river_density: GenerationDensity,
    pub lake_density: GenerationDensity,
    pub pond_density: GenerationDensity,
    pub wetland_density: GenerationDensity,
    pub waterfall_density: GenerationDensity,
    #[serde(default = "default_true")]
    pub waterfalls_required_at_structural_drops: bool,
    pub estuaries_enabled: bool,
    pub navigable_waterways_required: bool,
    pub distinguish_marine_freshwater_and_brackish: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BiomeGenerationSettings {
    pub variety: BiomeVariety,
    pub forest_density: GenerationDensity,
    pub special_biome_frequency: GenerationDensity,
    pub enabled_primary_biomes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HousingGenerationSettings {
    pub city_available_homes_target: u8,
    pub village_available_homes_target: u8,
    pub village_second_home_chance_percent: u8,
    pub rentals_enabled: bool,
    pub purchases_enabled: bool,
    pub lease_to_own_enabled: bool,
    pub household_co_ownership_enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementGenerationSettings {
    pub secondary_city_count: u8,
    pub villages_per_city_min: u8,
    pub villages_per_city_max: u8,
    pub independent_village_count: u8,
    pub road_density: GenerationDensity,
    pub ruin_density: GenerationDensity,
    pub cave_and_dungeon_density: GenerationDensity,
    pub willowmere_location: WillowmereLocationPreference,
    pub housing: HousingGenerationSettings,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialWorldRules {
    pub npc_reputation_enabled: bool,
    pub npc_relationships_enabled: bool,
    pub player_player_partnership_enabled: bool,
    pub player_player_marriage_enabled: bool,
    pub player_npc_marriage_enabled: bool,
    pub households_enabled: bool,
    pub children_enabled: bool,
    pub adoption_enabled: bool,
    pub pregnancy_enabled: bool,
    pub pregnancy_visual_layers_enabled: bool,
    pub family_system_opt_out_supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldCreationSettings {
    pub schema: String,
    pub display_name: String,
    pub seed: u64,
    pub chunk_size_tiles: u32,
    pub size: WorldSizePreset,
    #[serde(default)]
    pub extent_mode: WorldExtentMode,
    #[serde(default)]
    pub difficulty: WorldDifficultyPreset,
    #[serde(default)]
    pub difficulty_tuning: WorldDifficultyTuning,
    pub landform: LandformPreset,
    pub land: LandGenerationSettings,
    pub hydrology: HydrologyGenerationSettings,
    pub biomes: BiomeGenerationSettings,
    pub settlements: SettlementGenerationSettings,
    pub social: SocialWorldRules,
    pub willowmere_required: bool,
    pub outdoor_world_is_continuous: bool,
    pub static_player_farmstead_generated: bool,
}

impl Default for WorldCreationSettings {
    fn default() -> Self {
        Self {
            schema: WORLD_CREATION_SETTINGS_SCHEMA.to_string(),
            display_name: "Havenwild World".to_string(),
            seed: 1_337,
            chunk_size_tiles: 64,
            size: WorldSizePreset::Standard,
            extent_mode: WorldExtentMode::Finite,
            difficulty: WorldDifficultyPreset::Standard,
            difficulty_tuning: WorldDifficultyTuning::default(),
            landform: LandformPreset::BalancedContinent,
            land: LandGenerationSettings {
                land_coverage_percent: 65,
                coastline_complexity: GenerationDensity::Normal,
                mountain_coverage: GenerationDensity::Normal,
                island_frequency: GenerationDensity::Normal,
            },
            hydrology: HydrologyGenerationSettings {
                marine_water_abundance: GenerationDensity::Normal,
                river_density: GenerationDensity::Normal,
                lake_density: GenerationDensity::Normal,
                pond_density: GenerationDensity::Normal,
                wetland_density: GenerationDensity::Normal,
                waterfall_density: GenerationDensity::Normal,
                waterfalls_required_at_structural_drops: true,
                estuaries_enabled: true,
                navigable_waterways_required: true,
                distinguish_marine_freshwater_and_brackish: true,
            },
            biomes: BiomeGenerationSettings {
                variety: BiomeVariety::Normal,
                forest_density: GenerationDensity::Normal,
                special_biome_frequency: GenerationDensity::Normal,
                enabled_primary_biomes: vec![
                    "meadow".to_string(),
                    "mixed_woodland".to_string(),
                    "deep_forest".to_string(),
                    "river_valley".to_string(),
                    "wetland".to_string(),
                    "coastal_lowland".to_string(),
                    "rocky_highlands".to_string(),
                    "alpine_upland".to_string(),
                ],
            },
            settlements: SettlementGenerationSettings {
                secondary_city_count: 2,
                villages_per_city_min: 1,
                villages_per_city_max: 4,
                independent_village_count: 1,
                road_density: GenerationDensity::Normal,
                ruin_density: GenerationDensity::Normal,
                cave_and_dungeon_density: GenerationDensity::Normal,
                willowmere_location: WillowmereLocationPreference::Automatic,
                housing: HousingGenerationSettings {
                    city_available_homes_target: 4,
                    village_available_homes_target: 1,
                    village_second_home_chance_percent: 20,
                    rentals_enabled: true,
                    purchases_enabled: true,
                    lease_to_own_enabled: true,
                    household_co_ownership_enabled: true,
                },
            },
            social: SocialWorldRules {
                npc_reputation_enabled: true,
                npc_relationships_enabled: true,
                player_player_partnership_enabled: true,
                player_player_marriage_enabled: true,
                player_npc_marriage_enabled: true,
                households_enabled: true,
                children_enabled: true,
                adoption_enabled: true,
                pregnancy_enabled: true,
                pregnancy_visual_layers_enabled: true,
                family_system_opt_out_supported: true,
            },
            willowmere_required: true,
            outdoor_world_is_continuous: true,
            static_player_farmstead_generated: false,
        }
    }
}

impl WorldCreationSettings {
    pub fn world_dimensions_tiles(&self) -> [u32; 2] {
        self.size.dimensions_tiles()
    }

    pub fn world_extent(&self) -> crate::world_instance::WorldExtent {
        match self.extent_mode {
            WorldExtentMode::Finite => {
                let [width_tiles, height_tiles] = self.world_dimensions_tiles();
                crate::world_instance::WorldExtent::finite(width_tiles, height_tiles)
            }
            WorldExtentMode::Endless => crate::world_instance::WorldExtent::Endless {
                region_size_tiles: crate::geographic_surface::GEOGRAPHIC_REGION_SIZE_TILES,
            },
        }
    }

    pub const fn is_endless(&self) -> bool {
        matches!(self.extent_mode, WorldExtentMode::Endless)
    }

    pub fn set_difficulty_preset(&mut self, preset: WorldDifficultyPreset) {
        self.difficulty = preset;
        if preset != WorldDifficultyPreset::Custom {
            self.difficulty_tuning = WorldDifficultyTuning::for_preset(preset);
        }
    }

    /// Convert player-facing creation settings into the unified runtime world
    /// descriptor. Finite and Endless surfaces use the same deterministic
    /// streamed generator; the extent only decides whether an outer boundary
    /// exists.
    pub fn shared_surface_descriptor(
        &self,
        world_id: impl Into<String>,
    ) -> crate::world_instance::WorldDescriptor {
        crate::world_instance::WorldDescriptor::new(
            world_id,
            self.display_name.clone(),
            self.seed,
            crate::world_instance::WorldScope::SharedSurface,
            self.world_extent(),
            crate::world_instance::PersistencePolicy::Permanent,
            crate::world_instance::GenerationPolicy::StreamedDeterministic,
            self.chunk_size_tiles,
        )
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != WORLD_CREATION_SETTINGS_SCHEMA {
            return Err(format!("unsupported world creation schema {}", self.schema));
        }
        if self.display_name.trim().is_empty() {
            return Err("world display name must not be empty".to_string());
        }
        if self.seed == 0 {
            return Err("world seed must be greater than zero".to_string());
        }
        let [width, height] = self.world_dimensions_tiles();
        if self.chunk_size_tiles == 0
            || width % self.chunk_size_tiles != 0
            || height % self.chunk_size_tiles != 0
        {
            return Err("world dimensions must be divisible by the chunk size".to_string());
        }
        if self.is_endless()
            && crate::geographic_surface::GEOGRAPHIC_REGION_SIZE_TILES % self.chunk_size_tiles != 0
        {
            return Err("endless geographic region size must align to chunk size".to_string());
        }
        self.difficulty_tuning.validate()?;
        if !self.hydrology.waterfalls_required_at_structural_drops {
            return Err(
                "Havenwild generated rivers require waterfalls at every structural drop".to_string(),
            );
        }
        if !(45..=80).contains(&self.land.land_coverage_percent) {
            return Err("land coverage must remain between 45 and 80 percent".to_string());
        }
        if self.settlements.villages_per_city_min > self.settlements.villages_per_city_max {
            return Err("minimum villages per city cannot exceed the maximum".to_string());
        }
        if self.settlements.housing.city_available_homes_target == 0
            || self.settlements.housing.village_available_homes_target == 0
        {
            return Err(
                "cities and villages must each expose at least one housing opportunity".to_string(),
            );
        }
        if self.settlements.housing.village_second_home_chance_percent > 100 {
            return Err("village second-home chance cannot exceed 100 percent".to_string());
        }
        if !self.willowmere_required {
            return Err("Willowmere is required in every Havenwild world".to_string());
        }
        if self.static_player_farmstead_generated {
            return Err(
                "Havenwild must not generate a mandatory static player farmstead".to_string(),
            );
        }
        if self.biomes.enabled_primary_biomes.is_empty() {
            return Err("at least one primary biome must be enabled".to_string());
        }
        Ok(())
    }

    pub fn mountain_radius_hint(&self) -> f32 {
        match self.landform {
            LandformPreset::Highlands => 0.68,
            LandformPreset::Riverlands | LandformPreset::InlandBasin => 0.28,
            _ => match self.land.mountain_coverage {
                GenerationDensity::Low => 0.30,
                GenerationDensity::Normal => 0.44,
                GenerationDensity::High => 0.60,
            },
        }
    }

    pub fn shoreline_width_hint(&self) -> f32 {
        match self.land.coastline_complexity {
            GenerationDensity::Low => 0.10,
            GenerationDensity::Normal => 0.12,
            GenerationDensity::High => 0.15,
        }
    }
}

pub fn load_world_creation_settings_from_path(
    path: impl AsRef<Path>,
) -> Result<WorldCreationSettings, String> {
    let payload = read_to_string(path.as_ref()).map_err(|error| error.to_string())?;
    let settings: WorldCreationSettings =
        serde_json::from_str(&payload).map_err(|error| error.to_string())?;
    settings.validate()?;
    Ok(settings)
}

pub fn save_world_creation_settings_to_path(
    path: impl AsRef<Path>,
    settings: &WorldCreationSettings,
) -> Result<(), String> {
    settings.validate()?;
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let payload = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
    write(path, payload).map_err(|error| error.to_string())
}

fn default_true() -> bool {
    true
}

fn cycle_enum<T: Copy + PartialEq>(value: T, delta: i32, values: &[T]) -> T {
    let current = values
        .iter()
        .position(|candidate| *candidate == value)
        .unwrap_or(0) as i32;
    let len = values.len() as i32;
    values[(current + delta).rem_euclid(len) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_world_is_valid_and_has_no_static_farmstead() {
        let settings = WorldCreationSettings::default();
        assert!(settings.validate().is_ok());
        assert!(!settings.static_player_farmstead_generated);
    }

    #[test]
    fn endless_worlds_use_the_same_streamed_descriptor_authority() {
        let mut settings = WorldCreationSettings::default();
        settings.extent_mode = WorldExtentMode::Endless;
        let descriptor = settings.shared_surface_descriptor("world:endless");
        assert!(descriptor.extent.is_endless());
        assert_eq!(
            descriptor.generation,
            crate::world_instance::GenerationPolicy::StreamedDeterministic
        );
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn eight_curated_difficulty_profiles_plus_custom_are_exposed() {
        assert_eq!(WorldDifficultyPreset::ALL.len(), 9);
        assert_eq!(WorldDifficultyPreset::ALL[0], WorldDifficultyPreset::Story);
        assert_eq!(WorldDifficultyPreset::ALL[7], WorldDifficultyPreset::Wild);
        assert_eq!(WorldDifficultyPreset::ALL[8], WorldDifficultyPreset::Custom);
    }

    #[test]
    fn villages_normally_offer_one_home_and_cities_offer_multiple() {
        let settings = WorldCreationSettings::default();
        assert_eq!(
            settings.settlements.housing.village_available_homes_target,
            1
        );
        assert!(settings.settlements.housing.city_available_homes_target > 1);
    }
}
