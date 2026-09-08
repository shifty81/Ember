use haven_core::TavernMap;

use super::{
    corner_material, edge_material, family_neighbors, terrain_transition_rule_manifest,
    transition_material_atlas_group, CardinalDirection, DiagonalDirection, TerrainFamily,
    TransitionMaterial, TransitionRulePhase,
};

/// Editor/debug-facing explanation for one resolved terrain transition.
///
/// This deliberately mirrors the resolver without owning render state. The game
/// overlay, standalone editor, validators, and future rule preview tools can all
/// ask world logic why a specific edge/corner resolved to a material, which rule
/// won, and whether the resolver had to fall back to built-in safety behavior.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainTransitionRuleHit {
    pub phase: TransitionRulePhase,
    pub direction: &'static str,
    pub center: TerrainFamily,
    pub neighbor: TerrainFamily,
    pub material: Option<TransitionMaterial>,
    pub atlas_group: Option<String>,
    pub rule_id: Option<String>,
    pub rule_priority: Option<i32>,
    pub rule_reason: Option<String>,
    pub used_builtin_fallback: bool,
}

impl TerrainTransitionRuleHit {
    pub fn material_code(&self) -> &'static str {
        self.material
            .map(TransitionMaterial::code)
            .unwrap_or("none")
    }

    pub fn rule_label(&self) -> &str {
        self.rule_id.as_deref().unwrap_or("builtin_fallback")
    }

    pub fn atlas_group_label(&self) -> &str {
        self.atlas_group.as_deref().unwrap_or("-")
    }

    pub fn priority_label(&self) -> String {
        self.rule_priority
            .map(|priority| priority.to_string())
            .unwrap_or_else(|| "-".to_string())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainTransitionRuleInspection {
    pub x: i32,
    pub y: i32,
    pub center: TerrainFamily,
    pub manifest_status: String,
    pub hits: Vec<TerrainTransitionRuleHit>,
}

impl TerrainTransitionRuleInspection {
    pub fn has_hits(&self) -> bool {
        !self.hits.is_empty()
    }

    pub fn manifest_hit_count(&self) -> usize {
        self.hits
            .iter()
            .filter(|hit| !hit.used_builtin_fallback && hit.rule_id.is_some())
            .count()
    }

    pub fn fallback_hit_count(&self) -> usize {
        self.hits
            .iter()
            .filter(|hit| hit.used_builtin_fallback)
            .count()
    }

    pub fn summary_line(&self) -> String {
        format!(
            "{} rule hit(s), {} fallback(s), center {}",
            self.manifest_hit_count(),
            self.fallback_hit_count(),
            self.center.code()
        )
    }
}

pub fn inspect_terrain_transition_rules(
    map: &TavernMap,
    x: i32,
    y: i32,
) -> Option<TerrainTransitionRuleInspection> {
    let neighbors = family_neighbors(map, x, y);
    let center = neighbors.center;
    if center == TerrainFamily::Void {
        return None;
    }

    let manifest_result = terrain_transition_rule_manifest();
    let manifest_status = manifest_result
        .map(|manifest| manifest.coverage_summary())
        .unwrap_or_else(|error| format!("terrain transition rule manifest unavailable: {error}"));
    let manifest = manifest_result.ok();
    let mut hits = Vec::new();

    for direction in CardinalDirection::ALL {
        let neighbor = neighbors.family_at_direction(direction);
        if neighbor == TerrainFamily::Void || neighbor == center {
            continue;
        }
        if let Some(hit) = inspect_transition_candidate(
            manifest,
            TransitionRulePhase::Edge,
            direction.code(),
            center,
            neighbor,
            edge_material(center, neighbor),
        ) {
            hits.push(hit);
        }
    }

    for direction in DiagonalDirection::ALL {
        let diagonal = neighbors.family_at_diagonal(direction);
        if diagonal == TerrainFamily::Void || diagonal == center {
            continue;
        }
        let (a, b) = direction.cardinals();
        let card_a = neighbors.family_at_direction(a);
        let card_b = neighbors.family_at_direction(b);
        if card_a != center || card_b != center {
            continue;
        }
        if let Some(hit) = inspect_transition_candidate(
            manifest,
            TransitionRulePhase::Corner,
            direction.code(),
            center,
            diagonal,
            corner_material(center, diagonal),
        ) {
            hits.push(hit);
        }
    }

    Some(TerrainTransitionRuleInspection {
        x,
        y,
        center,
        manifest_status,
        hits,
    })
}

fn inspect_transition_candidate(
    manifest: Option<&super::TerrainTransitionRuleManifest>,
    phase: TransitionRulePhase,
    direction: &'static str,
    center: TerrainFamily,
    neighbor: TerrainFamily,
    resolved_material: Option<TransitionMaterial>,
) -> Option<TerrainTransitionRuleHit> {
    let rule = manifest.and_then(|manifest| manifest.best_rule_for(phase, center, neighbor));
    let material = rule.map(|rule| rule.material).or(resolved_material)?;
    let atlas_group = rule
        .map(|rule| rule.atlas_group.clone())
        .or_else(|| transition_material_atlas_group(material).map(str::to_string));

    Some(TerrainTransitionRuleHit {
        phase,
        direction,
        center,
        neighbor,
        material: Some(material),
        atlas_group,
        rule_id: rule.map(|rule| rule.id.clone()),
        rule_priority: rule.map(|rule| rule.priority),
        rule_reason: rule.and_then(|rule| rule.reason.clone()),
        used_builtin_fallback: rule.is_none(),
    })
}

trait TransitionDirectionCode {
    fn code(self) -> &'static str;
}

impl TransitionDirectionCode for CardinalDirection {
    fn code(self) -> &'static str {
        match self {
            CardinalDirection::North => "north",
            CardinalDirection::East => "east",
            CardinalDirection::South => "south",
            CardinalDirection::West => "west",
        }
    }
}

impl TransitionDirectionCode for DiagonalDirection {
    fn code(self) -> &'static str {
        match self {
            DiagonalDirection::NorthEast => "north_east",
            DiagonalDirection::SouthEast => "south_east",
            DiagonalDirection::SouthWest => "south_west",
            DiagonalDirection::NorthWest => "north_west",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::TileKind;

    #[test]
    fn inspector_reports_owner_side_grass_bank_rule() {
        let mut map = TavernMap::empty_with(TileKind::ShallowWater);
        map.set(4, 4, TileKind::ShallowWater);
        map.set(5, 4, TileKind::Grass);

        let inspection = inspect_terrain_transition_rules(&map, 4, 4)
            .expect("shallow water tile should inspect");
        assert!(inspection.has_hits());
        assert!(inspection.hits.iter().any(|hit| {
            hit.phase == TransitionRulePhase::Edge
                && hit.direction == "east"
                && hit.neighbor == TerrainFamily::Grass
                && hit.material == Some(TransitionMaterial::GrassFringe)
        }));
    }

    #[test]
    fn inspector_reports_inner_corner_rule() {
        let mut map = TavernMap::empty_with(TileKind::ShallowWater);
        map.set(4, 4, TileKind::ShallowWater);
        map.set(5, 4, TileKind::ShallowWater);
        map.set(4, 3, TileKind::ShallowWater);
        map.set(5, 3, TileKind::Grass);

        let inspection = inspect_terrain_transition_rules(&map, 4, 4)
            .expect("shallow water tile should inspect");
        assert!(inspection.hits.iter().any(|hit| {
            hit.phase == TransitionRulePhase::Corner && hit.direction == "north_east"
        }));
    }
}
