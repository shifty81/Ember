//! Compatibility namespace. Command contracts now live in `haven_authoring` so
//! renderer, networking, and runtime crates do not depend on the editor UI crate.

pub use haven_authoring::{
    CommandHistoryStep, CommandPayload, CommandTarget, CommandUndoStep, EditorCommand,
    EditorCommandBus, EditorCommandKind, EditorCommandSource, GridPos,
};
