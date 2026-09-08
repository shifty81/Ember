use super::super::transition_atlas_groups::default_atlas_group_for_material;
use super::*;

#[test]
fn material_defaults_have_atlas_groups() {
    for code in MATERIAL_CODES {
        let material = TransitionMaterial::from_code(code).expect("material code should parse");
        assert!(!default_atlas_group_for_material(material).is_empty());
    }
}

#[test]
fn selector_cycle_codes_are_valid_manifest_selectors() {
    for code in SELECTOR_CODES {
        assert!(
            TerrainFamilySelector::from_code(code).is_some(),
            "{code} should be a valid transition selector"
        );
    }
}

#[test]
fn safe_fix_preview_detects_mismatched_atlas_group() {
    let mut rule = serde_json::Map::new();
    rule.insert("id".to_string(), Value::String("test_rule".to_string()));
    rule.insert("material".to_string(), Value::String("foam".to_string()));
    rule.insert(
        "atlasGroup".to_string(),
        Value::String("grass_over_dirt".to_string()),
    );
    assert_eq!(safe_fix_count_for_rule(&rule), 1);
}

#[test]
fn pair_specific_atlas_group_is_not_auto_fixed_to_material_default() {
    let mut rule = serde_json::Map::new();
    rule.insert("center".to_string(), Value::String("grass".to_string()));
    rule.insert("neighbor".to_string(), Value::String("sand".to_string()));
    rule.insert(
        "material".to_string(),
        Value::String("grass_fringe".to_string()),
    );
    rule.insert(
        "atlasGroup".to_string(),
        Value::String("grass_over_sand".to_string()),
    );
    assert_eq!(safe_fix_count_for_rule(&rule), 0);
}

#[test]
fn supported_atlas_groups_cover_material_defaults() {
    for code in MATERIAL_CODES {
        let material = TransitionMaterial::from_code(code).expect("material code should parse");
        let group = default_atlas_group_for_material(material);
        assert!(
            SUPPORTED_TRANSITION_ATLAS_GROUPS.contains(&group),
            "{group} should be supported by draft diagnostics"
        );
    }
}

#[test]
fn draft_undo_path_is_separate_from_draft_path() {
    assert_ne!(
        TERRAIN_TRANSITION_RULE_DRAFT_PATH,
        TERRAIN_TRANSITION_RULE_DRAFT_UNDO_PATH
    );
    assert!(TERRAIN_TRANSITION_RULE_DRAFT_UNDO_PATH.contains("undo"));
}
