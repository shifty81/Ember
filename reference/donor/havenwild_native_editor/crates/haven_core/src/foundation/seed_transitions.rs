use super::{SceneId, Transition, TransitionId};

#[expect(
    clippy::too_many_arguments,
    reason = "fixed seed transitions are clearest as rectangle, target, spawn, and label literals"
)]
pub(super) fn transition(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    target: SceneId,
    spawn_x: i32,
    spawn_y: i32,
    label: &str,
) -> Transition {
    Transition {
        id: TransitionId::from_stable_key(&format!(
            "{x}:{y}:{w}:{h}:{}:{spawn_x}:{spawn_y}",
            target.code()
        )),
        x,
        y,
        w,
        h,
        target: target.into(),
        spawn_x,
        spawn_y,
        label: label.to_string(),
    }
}
