use super::{TerrainFamily, TransitionMaterial};

pub const SUPPORTED_TRANSITION_ATLAS_GROUPS: [&str; 9] = [
    "grass_over_dirt",
    "grass_over_sand",
    "sand_over_wet_sand",
    "pebble_path_over_dirt",
    "grass_bank_over_shallow",
    "dirt_bank_over_shallow",
    "sand_bank_over_shallow",
    "shallow_rim_over_deep",
    "riverbank_mud",
];

pub fn ordered_pair_atlas_group(
    center: TerrainFamily,
    neighbor: TerrainFamily,
) -> Option<&'static str> {
    match (center, neighbor) {
        (TerrainFamily::Grass, TerrainFamily::Dirt | TerrainFamily::Farm | TerrainFamily::Road) => {
            Some("grass_over_dirt")
        }
        (TerrainFamily::Grass, TerrainFamily::Sand) => Some("grass_over_sand"),
        (TerrainFamily::WetSand, TerrainFamily::Sand) => Some("sand_over_wet_sand"),
        (
            TerrainFamily::PebblePath,
            TerrainFamily::Grass
            | TerrainFamily::Dirt
            | TerrainFamily::Sand
            | TerrainFamily::WetSand,
        ) => Some("pebble_path_over_dirt"),
        (TerrainFamily::ShallowWater | TerrainFamily::Water, TerrainFamily::Grass) => {
            Some("grass_bank_over_shallow")
        }
        (
            TerrainFamily::ShallowWater | TerrainFamily::Water,
            TerrainFamily::Dirt | TerrainFamily::Road | TerrainFamily::Farm,
        ) => Some("dirt_bank_over_shallow"),
        (
            TerrainFamily::ShallowWater | TerrainFamily::Water,
            TerrainFamily::Sand | TerrainFamily::WetSand,
        ) => Some("sand_bank_over_shallow"),
        (TerrainFamily::DeepWater, TerrainFamily::ShallowWater | TerrainFamily::Water) => {
            Some("shallow_rim_over_deep")
        }
        _ => None,
    }
}

pub fn expected_atlas_group_for_codes(
    center_code: Option<&str>,
    neighbor_code: Option<&str>,
    material: TransitionMaterial,
) -> &'static str {
    center_code
        .and_then(TerrainFamily::from_code)
        .zip(neighbor_code.and_then(TerrainFamily::from_code))
        .and_then(|(center, neighbor)| ordered_pair_atlas_group(center, neighbor))
        .unwrap_or_else(|| default_atlas_group_for_material(material))
}

pub fn default_atlas_group_for_material(material: TransitionMaterial) -> &'static str {
    match material {
        TransitionMaterial::WetSand | TransitionMaterial::SandBlend => "sand_bank_over_shallow",
        TransitionMaterial::Foam | TransitionMaterial::ShallowWaterEdge => "shallow_rim_over_deep",
        TransitionMaterial::GrassFringe => "grass_over_dirt",
        TransitionMaterial::DirtBlend => "dirt_bank_over_shallow",
        TransitionMaterial::RoadShoulder
        | TransitionMaterial::StoneShoulder
        | TransitionMaterial::RockShadow => "riverbank_mud",
    }
}

pub fn is_water_transition_atlas_group(group: Option<&str>) -> bool {
    matches!(
        group,
        Some(
            "grass_bank_over_shallow"
                | "dirt_bank_over_shallow"
                | "sand_bank_over_shallow"
                | "shallow_rim_over_deep"
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_pairs_preserve_distinct_lpc_families() {
        assert_eq!(
            ordered_pair_atlas_group(TerrainFamily::Grass, TerrainFamily::Sand),
            Some("grass_over_sand")
        );
        assert_eq!(
            ordered_pair_atlas_group(TerrainFamily::WetSand, TerrainFamily::Sand),
            Some("sand_over_wet_sand")
        );
        assert_eq!(
            ordered_pair_atlas_group(TerrainFamily::PebblePath, TerrainFamily::Dirt),
            Some("pebble_path_over_dirt")
        );
        assert_eq!(
            ordered_pair_atlas_group(TerrainFamily::PebblePath, TerrainFamily::Grass),
            Some("pebble_path_over_dirt")
        );
        assert_eq!(
            ordered_pair_atlas_group(TerrainFamily::ShallowWater, TerrainFamily::Sand),
            Some("sand_bank_over_shallow")
        );
        assert_eq!(
            ordered_pair_atlas_group(TerrainFamily::ShallowWater, TerrainFamily::Road),
            Some("dirt_bank_over_shallow")
        );
        assert_eq!(
            ordered_pair_atlas_group(TerrainFamily::DeepWater, TerrainFamily::ShallowWater),
            Some("shallow_rim_over_deep")
        );
    }

    #[test]
    fn non_water_shoulders_never_alias_to_water_bank_art() {
        assert_eq!(
            ordered_pair_atlas_group(TerrainFamily::Road, TerrainFamily::Grass),
            None
        );
        assert_eq!(
            ordered_pair_atlas_group(TerrainFamily::RockWall, TerrainFamily::Grass),
            None
        );
        assert_eq!(
            ordered_pair_atlas_group(TerrainFamily::DeepWater, TerrainFamily::Grass),
            None
        );
        assert_eq!(
            ordered_pair_atlas_group(TerrainFamily::Grass, TerrainFamily::Road),
            Some("grass_over_dirt")
        );
    }

    #[test]
    fn code_based_editor_defaults_preserve_ordered_pairs() {
        assert_eq!(
            expected_atlas_group_for_codes(
                Some("grass"),
                Some("sand"),
                TransitionMaterial::SandBlend,
            ),
            "grass_over_sand"
        );
    }

    #[test]
    fn material_defaults_are_supported() {
        for material in [
            TransitionMaterial::WetSand,
            TransitionMaterial::Foam,
            TransitionMaterial::ShallowWaterEdge,
            TransitionMaterial::SandBlend,
            TransitionMaterial::GrassFringe,
            TransitionMaterial::DirtBlend,
            TransitionMaterial::RoadShoulder,
            TransitionMaterial::StoneShoulder,
            TransitionMaterial::RockShadow,
        ] {
            assert!(SUPPORTED_TRANSITION_ATLAS_GROUPS
                .contains(&default_atlas_group_for_material(material)));
        }
    }
}
