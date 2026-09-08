use serde::{Deserialize, Serialize};
use crate::{BuildingAnchor, BuildingDefinition, BuildingInstanceId, BuildingSceneKey, BuildingSpace};

/// Runtime-safe return information captured when a player crosses a building boundary.
/// It deliberately stores semantic building coordinates instead of assuming that the
/// interior and exterior share the same dimensions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingReturnAnchor {
    pub building: BuildingInstanceId,
    pub transition_id: String,
    pub scene: BuildingSceneKey,
    pub x: u32,
    pub y: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingTravelSession {
    #[serde(default)]
    return_stack: Vec<BuildingReturnAnchor>,
}

impl BuildingTravelSession {
    pub fn is_inside_building(&self) -> bool { !self.return_stack.is_empty() }
    pub fn depth(&self) -> usize { self.return_stack.len() }
    pub fn current_return(&self) -> Option<&BuildingReturnAnchor> { self.return_stack.last() }

    /// Capture the exact semantic source anchor before entering an interior.
    pub fn enter(&mut self, building: &BuildingDefinition, transition_id: &str) -> Result<BuildingSceneKey, String> {
        let transition = building.layout.transitions.iter()
            .find(|t| t.id == transition_id && t.from.space == BuildingSpace::Exterior)
            .ok_or_else(|| format!("building transition {transition_id} has no exterior entrance"))?;
        let destination = BuildingSceneKey { building: building.id.clone(), space: transition.to.space, floor: transition.to.floor };
        self.return_stack.push(BuildingReturnAnchor {
            building: building.id.clone(),
            transition_id: transition.id.clone(),
            scene: BuildingSceneKey { building: building.id.clone(), space: transition.from.space, floor: transition.from.floor },
            x: transition.from.x,
            y: transition.from.y,
        });
        Ok(destination)
    }

    /// Leave the current building through the captured originating entrance.
    /// The returned anchor is authoritative even when the interior is larger than
    /// the exterior or when several buildings share the same interior archetype.
    pub fn exit_to_origin(&mut self) -> Option<BuildingReturnAnchor> { self.return_stack.pop() }

    pub fn clear(&mut self) { self.return_stack.clear(); }
}

/// Finds the authored interior-side anchor for an entrance without deriving it
/// from exterior coordinates.
pub fn interior_anchor_for_entrance<'a>(building: &'a BuildingDefinition, transition_id: &str) -> Option<&'a BuildingAnchor> {
    building.layout.transitions.iter()
        .find(|t| t.id == transition_id && t.from.space == BuildingSpace::Exterior && t.to.space == BuildingSpace::Interior)
        .map(|t| &t.to)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn house() -> BuildingDefinition {
        BuildingDefinition {
            id: BuildingInstanceId::new("willowmere.house.004"), archetype: "residence".into(),
            layout: BuildingLayout {
                exterior: ExteriorLayout { width: 7, height: 6, floors: 1 },
                interior: InteriorLayout { width: 12, height: 10, floors: vec![BuildingFloorLayout { level: 0, rooms: vec![] }] },
                transitions: vec![BuildingTransition {
                    id: "entrance.main".into(),
                    from: BuildingAnchor { space: BuildingSpace::Exterior, floor: 0, x: 3, y: 5 },
                    to: BuildingAnchor { space: BuildingSpace::Interior, floor: 0, x: 6, y: 9 },
                }],
            },
        }
    }

    #[test]
    fn expanded_interior_does_not_change_return_coordinate() {
        let b = house(); let mut travel = BuildingTravelSession::default();
        let target = travel.enter(&b, "entrance.main").unwrap();
        assert_eq!(target.space, BuildingSpace::Interior);
        assert_eq!((interior_anchor_for_entrance(&b, "entrance.main").unwrap().x, interior_anchor_for_entrance(&b, "entrance.main").unwrap().y), (6, 9));
        let back = travel.exit_to_origin().unwrap();
        assert_eq!((back.x, back.y), (3, 5));
        assert!(!travel.is_inside_building());
    }

    #[test]
    fn different_buildings_keep_distinct_return_identity() {
        let a = house(); let mut b = house(); b.id = BuildingInstanceId::new("willowmere.house.005");
        let mut ta = BuildingTravelSession::default(); let mut tb = BuildingTravelSession::default();
        ta.enter(&a, "entrance.main").unwrap(); tb.enter(&b, "entrance.main").unwrap();
        assert_ne!(ta.current_return().unwrap().building, tb.current_return().unwrap().building);
    }
}
