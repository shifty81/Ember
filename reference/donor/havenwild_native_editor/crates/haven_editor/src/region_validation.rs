use haven_core::{GameWorld, ProjectSceneId, SceneId, SceneKind};

pub(crate) fn scene_should_exist_in_region_graph(kind: SceneKind, id: SceneId) -> bool {
    matches!(kind, SceneKind::Exterior | SceneKind::Cave)
        && !matches!(
            id,
            SceneId::Cellar | SceneId::TavernInterior | SceneId::GuestFloor
        )
}

pub(crate) fn world_has_transition_between(
    world: &GameWorld,
    a: &ProjectSceneId,
    b: &ProjectSceneId,
) -> bool {
    world.scene_by_id(a).is_some_and(|scene| {
        scene
            .transitions
            .iter()
            .any(|transition| transition.target.project_id() == b)
    }) || world.scene_by_id(b).is_some_and(|scene| {
        scene
            .transitions
            .iter()
            .any(|transition| transition.target.project_id() == a)
    })
}
