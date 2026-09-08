//! Canonical ordered terrain-pair registry.
//!
//! A transition pair is directional: the owner is the cell that draws the
//! authored boundary and the neighbor is the material revealed around it.

use super::{terrain_family::TerrainFamily, transition_resolver::TransitionMaterial};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransitionOwnership {
    OwnerDrawsBoundary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransitionCollisionPolicy {
    BaseTerrainOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TerrainTransitionPairPolicy {
    pub owner: TerrainFamily,
    pub neighbor: TerrainFamily,
    pub material: TransitionMaterial,
    pub ownership: TransitionOwnership,
    pub collision: TransitionCollisionPolicy,
}

const fn policy(
    owner: TerrainFamily,
    neighbor: TerrainFamily,
    material: TransitionMaterial,
) -> TerrainTransitionPairPolicy {
    TerrainTransitionPairPolicy {
        owner,
        neighbor,
        material,
        ownership: TransitionOwnership::OwnerDrawsBoundary,
        collision: TransitionCollisionPolicy::BaseTerrainOnly,
    }
}

pub fn transition_pair_policy(
    owner: TerrainFamily,
    neighbor: TerrainFamily,
) -> Option<TerrainTransitionPairPolicy> {
    use TerrainFamily as F;
    use TransitionMaterial as M;

    let material = match (owner, neighbor) {
        (F::Grass, F::Dirt | F::Farm | F::Road) => M::DirtBlend,
        (F::Grass, F::Sand) => M::SandBlend,
        (F::WetSand, F::Sand) => M::SandBlend,
        (F::PebblePath, F::Grass | F::Dirt | F::Sand | F::WetSand) => M::StoneShoulder,

        (F::ShallowWater | F::Water, F::Grass) => M::GrassFringe,
        (F::ShallowWater | F::Water, F::Dirt | F::Road | F::Farm) => M::DirtBlend,
        (F::ShallowWater | F::Water, F::Sand | F::WetSand) => M::WetSand,
        (F::DeepWater, F::ShallowWater | F::Water) => M::ShallowWaterEdge,
        _ => return None,
    };
    Some(policy(owner, neighbor, material))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_pairs_do_not_silently_reverse_ownership() {
        let grass_sand = transition_pair_policy(TerrainFamily::Grass, TerrainFamily::Sand)
            .expect("grass owns authored sand cut");
        assert_eq!(grass_sand.material, TransitionMaterial::SandBlend);
        assert!(transition_pair_policy(TerrainFamily::Sand, TerrainFamily::Grass).is_none());
        assert_eq!(
            grass_sand.collision,
            TransitionCollisionPolicy::BaseTerrainOnly
        );
    }

    #[test]
    fn grass_owns_dirt_like_road_edge() {
        let road = transition_pair_policy(TerrainFamily::Grass, TerrainFamily::Road)
            .expect("grass should own the authored dirt-like road fringe");
        assert_eq!(road.material, TransitionMaterial::DirtBlend);
    }

    #[test]
    fn unsupported_pairs_do_not_emit_generic_transition_requests() {
        assert!(transition_pair_policy(TerrainFamily::Road, TerrainFamily::Grass).is_none());
        assert!(transition_pair_policy(TerrainFamily::RockWall, TerrainFamily::Grass).is_none());
        assert!(transition_pair_policy(TerrainFamily::DeepWater, TerrainFamily::Grass).is_none());
        assert!(transition_pair_policy(TerrainFamily::Road, TerrainFamily::Water).is_none());
    }

    #[test]
    fn shallow_water_uses_dirt_bank_art_against_roads_and_farm_soil() {
        for neighbor in [TerrainFamily::Road, TerrainFamily::Farm] {
            let policy = transition_pair_policy(TerrainFamily::ShallowWater, neighbor)
                .expect("dirt-like bank neighbor should be supported");
            assert_eq!(policy.material, TransitionMaterial::DirtBlend);
        }
    }

    #[test]
    fn shallow_water_owns_sand_coast() {
        let coast = transition_pair_policy(TerrainFamily::ShallowWater, TerrainFamily::Sand)
            .expect("shallow water should own sand shoreline");
        assert_eq!(coast.material, TransitionMaterial::WetSand);
    }
}
