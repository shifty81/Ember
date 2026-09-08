use haven_core::Transition;

use crate::{GridPos, GridRect};

pub fn transition_grid_rect(transition: &Transition) -> GridRect {
    GridRect {
        min: GridPos {
            x: transition.x,
            y: transition.y,
        },
        max: GridPos {
            x: transition.x + transition.w.max(1) - 1,
            y: transition.y + transition.h.max(1) - 1,
        },
    }
}
