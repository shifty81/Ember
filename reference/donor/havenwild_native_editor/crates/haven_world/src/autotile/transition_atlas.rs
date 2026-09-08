use super::{
    transition_atlas_groups::ordered_pair_atlas_group, transition_rule_atlas_group_for_material,
    CardinalDirection, DiagonalDirection, ResolvedTerrainTransitions, TerrainFamily,
    TransitionMaterial,
};

/// Renderer/editor-agnostic request for an atlas-backed outer edge or corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainTransitionAtlasRequest {
    pub material: TransitionMaterial,
    pub atlas_group: &'static str,
    pub mask4: u8,
    pub edge_count: u8,
}

impl TerrainTransitionAtlasRequest {
    pub fn has_mask(self) -> bool {
        self.mask4 != 0
    }
}

/// Dedicated request for one authored LPC 2x2 inner-corner role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainTransitionInnerCornerRequest {
    pub material: TransitionMaterial,
    pub atlas_group: &'static str,
    pub direction: DiagonalDirection,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MaterialMaskBuilder {
    material: Option<TransitionMaterial>,
    atlas_group: Option<&'static str>,
    mask4: u8,
    edge_count: u8,
}

impl MaterialMaskBuilder {
    fn new(material: TransitionMaterial, atlas_group: &'static str) -> Self {
        Self {
            material: Some(material),
            atlas_group: Some(atlas_group),
            ..Self::default()
        }
    }

    fn matches(self, material: TransitionMaterial, atlas_group: &'static str) -> bool {
        self.material == Some(material) && self.atlas_group == Some(atlas_group)
    }

    fn push_edge(&mut self, direction: CardinalDirection) {
        self.mask4 |= direction.bit();
        self.edge_count = self.edge_count.saturating_add(1);
    }

    fn finish(self) -> Option<TerrainTransitionAtlasRequest> {
        let material = self.material?;
        let atlas_group = self.atlas_group?;
        if self.mask4 == 0 {
            return None;
        }
        Some(TerrainTransitionAtlasRequest {
            material,
            atlas_group,
            mask4: self.mask4,
            edge_count: self.edge_count,
        })
    }
}

/// Select the normalized LPC family from the ordered owner/neighbor pair.
/// This is intentionally pair-specific; choosing a sheet from material alone
/// was the source of grass, sand, pond-bank, and depth-ring cross-wiring.
pub fn transition_pair_atlas_group(
    center: TerrainFamily,
    neighbor: TerrainFamily,
    _material: TransitionMaterial,
) -> Option<&'static str> {
    // Runtime rendering may only select an authored block from an exact
    // ordered owner/neighbor pair. Falling back from a generic material was
    // cross-wiring road, stone, and rock shoulders to pond-bank water art.
    ordered_pair_atlas_group(center, neighbor)
}

pub fn transition_material_atlas_group(material: TransitionMaterial) -> Option<&'static str> {
    transition_rule_atlas_group_for_material(material).or(match material {
        TransitionMaterial::WetSand
        | TransitionMaterial::Foam
        | TransitionMaterial::ShallowWaterEdge
        | TransitionMaterial::SandBlend => Some("sand_bank_over_shallow"),
        TransitionMaterial::GrassFringe => Some("grass_over_dirt"),
        TransitionMaterial::DirtBlend => Some("dirt_bank_over_shallow"),
        TransitionMaterial::RoadShoulder
        | TransitionMaterial::StoneShoulder
        | TransitionMaterial::RockShadow => Some("riverbank_mud"),
    })
}

/// Resolve only cardinal boundary ownership into the 4-way outer-role mask.
/// Diagonal-only topology is intentionally excluded and is emitted through
/// `resolve_transition_inner_corner_requests` instead of being folded into an
/// unrelated outer corner.
pub fn resolve_transition_atlas_requests(
    transitions: &ResolvedTerrainTransitions,
) -> Vec<TerrainTransitionAtlasRequest> {
    let mut builders = Vec::<MaterialMaskBuilder>::new();

    for edge in &transitions.edges {
        let Some(group) =
            transition_pair_atlas_group(transitions.center, edge.neighbor, edge.material)
        else {
            continue;
        };
        builder_for(&mut builders, edge.material, group).push_edge(edge.direction);
    }

    builders
        .into_iter()
        .filter_map(MaterialMaskBuilder::finish)
        .collect()
}

pub fn resolve_transition_inner_corner_requests(
    transitions: &ResolvedTerrainTransitions,
) -> Vec<TerrainTransitionInnerCornerRequest> {
    // Every mapped LPC 2x2 inner-corner block follows the same topology rule:
    // when both cardinal neighbors remain the owner material and only the
    // diagonal is the neighboring material, the owner tile must draw the
    // matching authored concave-corner role. This is required for a single
    // painted cell, holes, coves, narrow channels, and every seasonal mirror.
    transitions
        .corners
        .iter()
        .filter_map(|corner| {
            transition_pair_atlas_group(transitions.center, corner.neighbor, corner.material).map(
                |atlas_group| TerrainTransitionInnerCornerRequest {
                    material: corner.material,
                    atlas_group,
                    direction: corner.direction,
                },
            )
        })
        .collect()
}

fn builder_for<'a>(
    builders: &'a mut Vec<MaterialMaskBuilder>,
    material: TransitionMaterial,
    atlas_group: &'static str,
) -> &'a mut MaterialMaskBuilder {
    if let Some(index) = builders
        .iter()
        .position(|builder| builder.matches(material, atlas_group))
    {
        return &mut builders[index];
    }
    builders.push(MaterialMaskBuilder::new(material, atlas_group));
    builders
        .last_mut()
        .expect("new transition request builder should exist")
}

#[cfg(test)]
mod tests {
    use super::super::{TerrainCornerTransition, TerrainEdgeTransition};
    use super::*;

    #[test]
    fn unsupported_non_water_pair_does_not_fall_back_to_pond_art() {
        assert_eq!(
            transition_pair_atlas_group(
                TerrainFamily::Road,
                TerrainFamily::Grass,
                TransitionMaterial::RoadShoulder,
            ),
            None
        );
        assert_eq!(
            transition_pair_atlas_group(
                TerrainFamily::RockWall,
                TerrainFamily::Grass,
                TransitionMaterial::RockShadow,
            ),
            None
        );
    }

    #[test]
    fn sand_bank_uses_pair_specific_lpc_group() {
        let transitions = ResolvedTerrainTransitions {
            center: TerrainFamily::ShallowWater,
            edges: vec![TerrainEdgeTransition {
                direction: CardinalDirection::North,
                neighbor: TerrainFamily::Sand,
                material: TransitionMaterial::WetSand,
            }],
            outer_corners: vec![],
            corners: vec![],
        };

        let requests = resolve_transition_atlas_requests(&transitions);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].atlas_group, "sand_bank_over_shallow");
        assert_eq!(requests[0].mask4, CardinalDirection::North.bit());
    }

    #[test]
    fn depth_transition_does_not_reuse_coastline_art() {
        let transitions = ResolvedTerrainTransitions {
            center: TerrainFamily::DeepWater,
            edges: vec![TerrainEdgeTransition {
                direction: CardinalDirection::East,
                neighbor: TerrainFamily::ShallowWater,
                material: TransitionMaterial::ShallowWaterEdge,
            }],
            outer_corners: vec![],
            corners: vec![],
        };
        let requests = resolve_transition_atlas_requests(&transitions);
        assert_eq!(requests[0].atlas_group, "shallow_rim_over_deep");
    }

    #[test]
    fn diagonal_grass_sand_contact_uses_authored_inner_corner() {
        let transitions = ResolvedTerrainTransitions {
            center: TerrainFamily::Grass,
            edges: vec![],
            outer_corners: vec![],
            corners: vec![TerrainCornerTransition {
                direction: DiagonalDirection::NorthEast,
                neighbor: TerrainFamily::Sand,
                material: TransitionMaterial::SandBlend,
            }],
        };

        assert!(resolve_transition_atlas_requests(&transitions).is_empty());
        let requests = resolve_transition_inner_corner_requests(&transitions);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].atlas_group, "grass_over_sand");
        assert_eq!(requests[0].direction, DiagonalDirection::NorthEast);
    }

    #[test]
    fn diagonal_water_bank_uses_dedicated_inner_corner_request() {
        let transitions = ResolvedTerrainTransitions {
            center: TerrainFamily::ShallowWater,
            edges: vec![],
            outer_corners: vec![],
            corners: vec![TerrainCornerTransition {
                direction: DiagonalDirection::NorthEast,
                neighbor: TerrainFamily::Sand,
                material: TransitionMaterial::WetSand,
            }],
        };

        assert!(resolve_transition_atlas_requests(&transitions).is_empty());
        let requests = resolve_transition_inner_corner_requests(&transitions);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].atlas_group, "sand_bank_over_shallow");
        assert_eq!(requests[0].direction, DiagonalDirection::NorthEast);
    }

    #[test]
    fn adjacent_edges_use_authored_lpc_corner_mask() {
        let transitions = ResolvedTerrainTransitions {
            center: TerrainFamily::Grass,
            edges: vec![
                TerrainEdgeTransition {
                    direction: CardinalDirection::North,
                    neighbor: TerrainFamily::Sand,
                    material: TransitionMaterial::SandBlend,
                },
                TerrainEdgeTransition {
                    direction: CardinalDirection::East,
                    neighbor: TerrainFamily::Sand,
                    material: TransitionMaterial::SandBlend,
                },
            ],
            outer_corners: vec![],
            corners: vec![],
        };

        let requests = resolve_transition_atlas_requests(&transitions);
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].mask4,
            CardinalDirection::North.bit() | CardinalDirection::East.bit()
        );
    }

    #[test]
    fn closed_adjacent_edges_do_not_force_global_compound_fill() {
        use super::super::TerrainOuterCornerTransition;

        let transitions = ResolvedTerrainTransitions {
            center: TerrainFamily::Grass,
            edges: vec![
                TerrainEdgeTransition {
                    direction: CardinalDirection::North,
                    neighbor: TerrainFamily::Sand,
                    material: TransitionMaterial::SandBlend,
                },
                TerrainEdgeTransition {
                    direction: CardinalDirection::East,
                    neighbor: TerrainFamily::Sand,
                    material: TransitionMaterial::SandBlend,
                },
            ],
            outer_corners: vec![TerrainOuterCornerTransition {
                direction: DiagonalDirection::NorthEast,
                neighbor: TerrainFamily::Sand,
                material: TransitionMaterial::SandBlend,
            }],
            corners: vec![],
        };

        let requests = resolve_transition_atlas_requests(&transitions);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].atlas_group, "grass_over_sand");
        assert_eq!(
            requests[0].mask4,
            CardinalDirection::North.bit() | CardinalDirection::East.bit()
        );
    }

    #[test]
    fn mixed_edge_and_diagonal_contact_does_not_layer_pure_diagonal_corner() {
        use haven_core::{TavernMap, TileKind};

        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(4, 3, TileKind::Sand);
        map.set(5, 3, TileKind::Sand);

        let transitions = super::super::resolve_terrain_transitions(&map, 4, 4);
        let outer = resolve_transition_atlas_requests(&transitions);
        assert_eq!(outer.len(), 1);
        assert_eq!(outer[0].atlas_group, "grass_over_sand");
        assert_eq!(outer[0].mask4, CardinalDirection::North.bit());

        // Mixed edge-plus-diagonal contacts are ordinary edge topology until a
        // dedicated 8-neighbor role exists. Reusing the pure 2x2 corner role
        // over an edge produces repeated scalloped bites along long borders.
        let inner = resolve_transition_inner_corner_requests(&transitions);
        assert!(
            inner.is_empty(),
            "do not reuse pure diagonal corner art for mixed edge-plus-diagonal contacts"
        );
    }

    #[test]
    fn single_sand_cell_resolves_complete_eight_neighbor_ring() {
        use haven_core::{TavernMap, TileKind};

        let mut map = TavernMap::empty_with(TileKind::Grass);
        map.set(4, 4, TileKind::Sand);

        let cardinal_expectations = [
            ((4, 3), CardinalDirection::South),
            ((5, 4), CardinalDirection::West),
            ((4, 5), CardinalDirection::North),
            ((3, 4), CardinalDirection::East),
        ];
        for ((x, y), direction) in cardinal_expectations {
            let transitions = super::super::resolve_terrain_transitions(&map, x, y);
            let requests = resolve_transition_atlas_requests(&transitions);
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].atlas_group, "grass_over_sand");
            assert_eq!(requests[0].mask4, direction.bit());
        }

        let diagonal_expectations = [
            ((3, 3), DiagonalDirection::SouthEast),
            ((5, 3), DiagonalDirection::SouthWest),
            ((5, 5), DiagonalDirection::NorthWest),
            ((3, 5), DiagonalDirection::NorthEast),
        ];
        for ((x, y), direction) in diagonal_expectations {
            let transitions = super::super::resolve_terrain_transitions(&map, x, y);
            let requests = resolve_transition_inner_corner_requests(&transitions);
            assert_eq!(requests.len(), 1);
            assert_eq!(requests[0].atlas_group, "grass_over_sand");
            assert_eq!(requests[0].direction, direction);
        }
    }

    #[test]
    fn dry_wet_sand_uses_distinct_authored_group() {
        let transitions = ResolvedTerrainTransitions {
            center: TerrainFamily::WetSand,
            edges: vec![TerrainEdgeTransition {
                direction: CardinalDirection::West,
                neighbor: TerrainFamily::Sand,
                material: TransitionMaterial::SandBlend,
            }],
            outer_corners: vec![],
            corners: vec![TerrainCornerTransition {
                direction: DiagonalDirection::NorthEast,
                neighbor: TerrainFamily::Sand,
                material: TransitionMaterial::SandBlend,
            }],
        };

        let outer = resolve_transition_atlas_requests(&transitions);
        assert_eq!(outer.len(), 1);
        assert_eq!(outer[0].atlas_group, "sand_over_wet_sand");
        let inner = resolve_transition_inner_corner_requests(&transitions);
        assert_eq!(inner.len(), 1);
        assert_eq!(inner[0].atlas_group, "sand_over_wet_sand");
    }
}
