use serde::{Deserialize, Serialize};

use crate::{
    BuildingAnchor, BuildingDefinition, BuildingRuntimeScene, BuildingRuntimeSceneRegistry,
    BuildingSceneKey, BuildingTravelSession, ProjectSceneId, SceneReference,
};

/// Runtime navigation result for a building transition.  This is the bridge between
/// BuildingLayout-owned scene identities and the project-owned SceneReference system.
/// It deliberately does not route through SceneId, so adding houses/shops/floors never
/// grows the legacy enum.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildingNavigationDestination {
    pub scene: SceneReference,
    pub building_scene: BuildingSceneKey,
    pub spawn_x: u32,
    pub spawn_y: u32,
}

impl BuildingNavigationDestination {
    pub fn project_scene_id(&self) -> &ProjectSceneId {
        self.scene.project_id()
    }
}

/// Convert a materialized building scene to the canonical project scene reference.
/// BuildingRuntimeSceneId already uses the stable `building:<id>:...` identity; this
/// function is the only conversion needed by client/runtime navigation.
pub fn building_scene_reference(scene: &BuildingRuntimeScene) -> SceneReference {
    SceneReference::from_canonical_id(scene.id.as_str().to_string())
        .expect("BuildingRuntimeSceneId must always be a valid canonical project scene id")
}

/// Enter a building through a semantic transition.  The exact exterior origin is
/// captured by BuildingTravelSession, while the destination scene is lazily
/// materialized and exposed as a normal project-owned SceneReference.
pub fn enter_building_runtime(
    building: &BuildingDefinition,
    transition_id: &str,
    travel: &mut BuildingTravelSession,
    scenes: &mut BuildingRuntimeSceneRegistry,
) -> Result<BuildingNavigationDestination, String> {
    let target = travel.enter(building, transition_id)?;
    let anchor = destination_anchor(building, transition_id, &target)?;
    let runtime = scenes.get_or_materialize(building, target.space, target.floor);
    Ok(BuildingNavigationDestination {
        scene: building_scene_reference(runtime),
        building_scene: target,
        spawn_x: anchor.x,
        spawn_y: anchor.y,
    })
}

/// Exit to the exact exterior origin captured on entry.  No interior/exterior
/// coordinate conversion occurs, so expanded interiors cannot strand or offset the
/// player on return.
pub fn exit_building_runtime(
    travel: &mut BuildingTravelSession,
    scenes: &mut BuildingRuntimeSceneRegistry,
    building: &BuildingDefinition,
) -> Result<BuildingNavigationDestination, String> {
    let origin = travel
        .exit_to_origin()
        .ok_or_else(|| "building exit requested without an active return anchor".to_string())?;
    if origin.building != building.id {
        return Err(format!(
            "building exit identity mismatch: session belongs to {}, requested {}",
            origin.building.as_str(),
            building.id.as_str()
        ));
    }
    let runtime = scenes.get_or_materialize(building, origin.scene.space, origin.scene.floor);
    Ok(BuildingNavigationDestination {
        scene: building_scene_reference(runtime),
        building_scene: origin.scene,
        spawn_x: origin.x,
        spawn_y: origin.y,
    })
}

fn destination_anchor<'a>(
    building: &'a BuildingDefinition,
    transition_id: &str,
    target: &BuildingSceneKey,
) -> Result<&'a BuildingAnchor, String> {
    building
        .layout
        .transitions
        .iter()
        .find(|transition| {
            transition.id == transition_id
                && transition.to.space == target.space
                && transition.to.floor == target.floor
        })
        .map(|transition| &transition.to)
        .ok_or_else(|| format!("building transition {transition_id} has no matching destination anchor"))
}

/// Compatibility marker used while legacy seed scenes still exist. New building
/// navigation must never resolve a dynamic building scene through SceneId.
pub fn is_dynamic_building_scene(reference: &SceneReference) -> bool {
    reference.as_str().starts_with("building:")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BuildingFloorLayout, BuildingInstanceId, BuildingLayout, BuildingSpace, BuildingTransition,
        ExteriorLayout, InteriorLayout,
    };

    fn tavern() -> BuildingDefinition {
        BuildingDefinition {
            id: BuildingInstanceId::new("willowmere.tavern.001"),
            archetype: "tavern".into(),
            layout: BuildingLayout {
                exterior: ExteriorLayout { width: 7, height: 6, floors: 1 },
                interior: InteriorLayout {
                    width: 14,
                    height: 11,
                    floors: vec![BuildingFloorLayout { level: 0, rooms: vec![] }],
                },
                transitions: vec![BuildingTransition {
                    id: "entrance.main".into(),
                    from: BuildingAnchor { space: BuildingSpace::Exterior, floor: 0, x: 3, y: 5 },
                    to: BuildingAnchor { space: BuildingSpace::Interior, floor: 0, x: 7, y: 10 },
                }],
            },
        }
    }

    #[test]
    fn entry_returns_dynamic_scene_reference_and_authored_interior_anchor() {
        let building = tavern();
        let mut travel = BuildingTravelSession::default();
        let mut scenes = BuildingRuntimeSceneRegistry::default();
        let destination = enter_building_runtime(&building, "entrance.main", &mut travel, &mut scenes).unwrap();
        assert_eq!(destination.scene.as_str(), "building:willowmere.tavern.001:interior:floor:0");
        assert_eq!((destination.spawn_x, destination.spawn_y), (7, 10));
        assert!(is_dynamic_building_scene(&destination.scene));
        assert_eq!(destination.scene.legacy_scene_id(), None);
    }

    #[test]
    fn exit_restores_exact_exterior_anchor() {
        let building = tavern();
        let mut travel = BuildingTravelSession::default();
        let mut scenes = BuildingRuntimeSceneRegistry::default();
        enter_building_runtime(&building, "entrance.main", &mut travel, &mut scenes).unwrap();
        let destination = exit_building_runtime(&mut travel, &mut scenes, &building).unwrap();
        assert_eq!(destination.scene.as_str(), "building:willowmere.tavern.001:exterior:floor:0");
        assert_eq!((destination.spawn_x, destination.spawn_y), (3, 5));
    }

    #[test]
    fn expanded_interior_never_changes_return_position() {
        let building = tavern();
        assert_eq!((building.layout.exterior.width, building.layout.exterior.height), (7, 6));
        assert_eq!((building.layout.interior.width, building.layout.interior.height), (14, 11));
        let mut travel = BuildingTravelSession::default();
        let mut scenes = BuildingRuntimeSceneRegistry::default();
        enter_building_runtime(&building, "entrance.main", &mut travel, &mut scenes).unwrap();
        let back = exit_building_runtime(&mut travel, &mut scenes, &building).unwrap();
        assert_eq!((back.spawn_x, back.spawn_y), (3, 5));
    }
}
