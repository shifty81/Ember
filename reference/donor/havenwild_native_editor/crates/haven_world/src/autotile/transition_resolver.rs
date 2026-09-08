use haven_core::TavernMap;

use super::transition_pair_registry::transition_pair_policy;

use super::{
    family_neighbors, transition_rule_material, CardinalDirection, DiagonalDirection,
    FamilyNeighbors, TerrainFamily, TransitionRulePhase,
};

/// Renderer/editor-agnostic transition material. Concrete LPC art is selected
/// later from the ordered center/neighbor family pair, so the same semantic
/// material can remain useful to procedural fallbacks and debug overlays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionMaterial {
    WetSand,
    Foam,
    ShallowWaterEdge,
    SandBlend,
    GrassFringe,
    DirtBlend,
    RoadShoulder,
    StoneShoulder,
    RockShadow,
}

impl TransitionMaterial {
    pub fn code(self) -> &'static str {
        match self {
            Self::WetSand => "wet_sand",
            Self::Foam => "foam",
            Self::ShallowWaterEdge => "shallow_water_edge",
            Self::SandBlend => "sand_blend",
            Self::GrassFringe => "grass_fringe",
            Self::DirtBlend => "dirt_blend",
            Self::RoadShoulder => "road_shoulder",
            Self::StoneShoulder => "stone_shoulder",
            Self::RockShadow => "rock_shadow",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "wet_sand" => Some(Self::WetSand),
            "foam" => Some(Self::Foam),
            "shallow_water_edge" => Some(Self::ShallowWaterEdge),
            "sand_blend" => Some(Self::SandBlend),
            "grass_fringe" => Some(Self::GrassFringe),
            "dirt_blend" => Some(Self::DirtBlend),
            "road_shoulder" => Some(Self::RoadShoulder),
            "stone_shoulder" => Some(Self::StoneShoulder),
            "rock_shadow" => Some(Self::RockShadow),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainEdgeTransition {
    pub direction: CardinalDirection,
    pub neighbor: TerrainFamily,
    pub material: TransitionMaterial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainCornerTransition {
    pub direction: DiagonalDirection,
    pub neighbor: TerrainFamily,
    pub material: TransitionMaterial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainOuterCornerTransition {
    pub direction: DiagonalDirection,
    pub neighbor: TerrainFamily,
    pub material: TransitionMaterial,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTerrainTransitions {
    pub center: TerrainFamily,
    pub edges: Vec<TerrainEdgeTransition>,
    pub outer_corners: Vec<TerrainOuterCornerTransition>,
    pub corners: Vec<TerrainCornerTransition>,
}

impl ResolvedTerrainTransitions {
    pub fn empty(center: TerrainFamily) -> Self {
        Self {
            center,
            edges: Vec::new(),
            outer_corners: Vec::new(),
            corners: Vec::new(),
        }
    }

    pub fn has_any(&self) -> bool {
        !self.edges.is_empty() || !self.outer_corners.is_empty() || !self.corners.is_empty()
    }
}

pub fn resolve_terrain_transitions(map: &TavernMap, x: i32, y: i32) -> ResolvedTerrainTransitions {
    resolve_terrain_transitions_from_neighbors(family_neighbors(map, x, y))
}

pub fn resolve_terrain_transitions_from_neighbors(
    neighbors: FamilyNeighbors,
) -> ResolvedTerrainTransitions {
    let center = neighbors.center;
    if center == TerrainFamily::Void {
        return ResolvedTerrainTransitions::empty(center);
    }

    let mut resolved = ResolvedTerrainTransitions::empty(center);
    for direction in CardinalDirection::ALL {
        let neighbor = neighbors.family_at_direction(direction);
        if neighbor == TerrainFamily::Void || neighbor == center {
            continue;
        }
        if let Some(material) = edge_material(center, neighbor) {
            resolved.edges.push(TerrainEdgeTransition {
                direction,
                neighbor,
                material,
            });
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
        if card_a == diagonal && card_b == diagonal {
            if let Some(material) = corner_material(center, diagonal) {
                resolved.outer_corners.push(TerrainOuterCornerTransition {
                    direction,
                    neighbor: diagonal,
                    material,
                });
            }
        } else if same_owner_surface(center, card_a) && same_owner_surface(center, card_b) {
            if let Some(material) = corner_material(center, diagonal) {
                resolved.corners.push(TerrainCornerTransition {
                    direction,
                    neighbor: diagonal,
                    material,
                });
            }
        }
    }

    resolved
}

fn same_owner_surface(center: TerrainFamily, candidate: TerrainFamily) -> bool {
    center == candidate || (center.is_shallow_water() && candidate.is_shallow_water())
}

pub fn edge_material(center: TerrainFamily, neighbor: TerrainFamily) -> Option<TransitionMaterial> {
    transition_rule_material(TransitionRulePhase::Edge, center, neighbor)
        .or_else(|| builtin_edge_material(center, neighbor))
}

pub fn corner_material(
    center: TerrainFamily,
    diagonal: TerrainFamily,
) -> Option<TransitionMaterial> {
    transition_rule_material(TransitionRulePhase::Corner, center, diagonal)
        .or_else(|| builtin_corner_material(center, diagonal))
}

fn builtin_corner_material(
    center: TerrainFamily,
    diagonal: TerrainFamily,
) -> Option<TransitionMaterial> {
    // The reviewed LPC pebble-path family has a complete 3x3 outer block but
    // no verified 2x2 concave block. Do not fabricate an inner corner from an
    // unrelated fallback family.
    if matches!(center, TerrainFamily::PebblePath) && diagonal.is_land() {
        return None;
    }
    builtin_edge_material(center, diagonal)
}

/// One side owns each authored LPC boundary. This prevents the old behavior
/// where both adjacent cells painted competing shoreline overlays.
fn builtin_edge_material(
    center: TerrainFamily,
    neighbor: TerrainFamily,
) -> Option<TransitionMaterial> {
    transition_pair_policy(center, neighbor).map(|policy| policy.material)
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::TileKind;

    #[test]
    fn shallow_water_owns_grass_bank_edge() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(4, 4, TileKind::ShallowWater);
        map.set(5, 4, TileKind::Grass);

        let transitions = resolve_terrain_transitions(&map, 4, 4);
        assert!(transitions.edges.iter().any(|edge| {
            edge.direction == CardinalDirection::East
                && edge.material == TransitionMaterial::GrassFringe
        }));
        assert!(!resolve_terrain_transitions(&map, 5, 4).has_any());
    }

    #[test]
    fn shallow_and_normal_water_share_inner_corner_continuity() {
        let neighbors = FamilyNeighbors {
            center: TerrainFamily::ShallowWater,
            north: TerrainFamily::ShallowWater,
            east: TerrainFamily::Water,
            south: TerrainFamily::ShallowWater,
            west: TerrainFamily::ShallowWater,
            north_east: TerrainFamily::Road,
            south_east: TerrainFamily::ShallowWater,
            south_west: TerrainFamily::ShallowWater,
            north_west: TerrainFamily::ShallowWater,
        };
        let transitions = resolve_terrain_transitions_from_neighbors(neighbors);
        assert!(transitions.corners.iter().any(|corner| {
            corner.direction == DiagonalDirection::NorthEast
                && corner.neighbor == TerrainFamily::Road
                && corner.material == TransitionMaterial::DirtBlend
        }));
    }

    #[test]
    fn material_code_roundtrip_supports_manifest_values() {
        assert_eq!(
            TransitionMaterial::from_code("wet_sand"),
            Some(TransitionMaterial::WetSand)
        );
        assert_eq!(
            TransitionMaterial::from_code("road_shoulder"),
            Some(TransitionMaterial::RoadShoulder)
        );
        assert_eq!(TransitionMaterial::from_code("missing"), None);
    }

    #[test]
    fn deep_water_owns_shallow_depth_rim() {
        let mut map = TavernMap::empty_with(TileKind::ShallowWater);
        map.set(4, 4, TileKind::DeepWater);

        let transitions = resolve_terrain_transitions(&map, 4, 4);
        assert!(transitions.edges.iter().any(|edge| {
            edge.material == TransitionMaterial::ShallowWaterEdge
                && edge.neighbor == TerrainFamily::ShallowWater
        }));
    }

    #[test]
    fn pebble_path_resolves_against_grass() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(4, 4, TileKind::PebbleShore);

        let transitions = resolve_terrain_transitions(&map, 4, 4);
        assert!(transitions.edges.iter().any(|edge| {
            edge.neighbor == TerrainFamily::Grass
                && edge.material == TransitionMaterial::StoneShoulder
        }));
    }

    #[test]
    fn diagonal_grass_creates_water_inner_corner() {
        let mut map = TavernMap::empty_with(TileKind::ShallowWater);
        map.set(3, 4, TileKind::ShallowWater);
        map.set(4, 3, TileKind::ShallowWater);
        map.set(3, 3, TileKind::Grass);

        let transitions = resolve_terrain_transitions(&map, 4, 4);
        assert!(transitions.corners.iter().any(|corner| {
            corner.direction == DiagonalDirection::NorthWest
                && corner.material == TransitionMaterial::GrassFringe
        }));
    }

    #[test]
    fn diagonal_neighbor_confirms_full_outer_corner() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(4, 4, TileKind::Grass);
        map.set(4, 3, TileKind::Sand);
        map.set(5, 4, TileKind::Sand);
        map.set(5, 3, TileKind::Sand);

        let transitions = resolve_terrain_transitions(&map, 4, 4);
        assert!(transitions.outer_corners.iter().any(|corner| {
            corner.direction == DiagonalDirection::NorthEast
                && corner.neighbor == TerrainFamily::Sand
        }));
    }

    #[test]
    fn mixed_sand_edge_and_diagonal_does_not_layer_inner_corner() {
        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(4, 4, TileKind::Grass);
        map.set(4, 3, TileKind::Sand);
        map.set(5, 3, TileKind::Sand);

        let transitions = resolve_terrain_transitions(&map, 4, 4);
        assert!(transitions.edges.iter().any(|edge| {
            edge.direction == CardinalDirection::North
                && edge.neighbor == TerrainFamily::Sand
                && edge.material == TransitionMaterial::SandBlend
        }));
        assert!(
            transitions.corners.is_empty(),
            "mixed edge-plus-diagonal contacts need a dedicated 8-neighbor role; do not reuse pure 2x2 corner art"
        );
    }

    #[test]
    fn wet_sand_owns_dry_sand_inner_corner() {
        let mut map = TavernMap::empty_with(TileKind::WetSand);
        map.set(3, 3, TileKind::Sand);

        let transitions = resolve_terrain_transitions(&map, 4, 4);
        assert!(transitions.corners.iter().any(|corner| {
            corner.direction == DiagonalDirection::NorthWest
                && corner.neighbor == TerrainFamily::Sand
                && corner.material == TransitionMaterial::SandBlend
        }));
    }

    #[test]
    fn pebble_path_uses_outer_roles_without_unverified_inner_corner() {
        let mut map = TavernMap::empty_with(TileKind::StonePath);
        map.set(4, 3, TileKind::Dirt);
        let edge = resolve_terrain_transitions(&map, 4, 4);
        assert!(edge.edges.iter().any(|transition| {
            transition.direction == CardinalDirection::North
                && transition.neighbor == TerrainFamily::Dirt
                && transition.material == TransitionMaterial::StoneShoulder
        }));

        let mut diagonal = TavernMap::empty_with(TileKind::StonePath);
        diagonal.set(3, 3, TileKind::Dirt);
        assert!(resolve_terrain_transitions(&diagonal, 4, 4)
            .corners
            .is_empty());
    }
}
