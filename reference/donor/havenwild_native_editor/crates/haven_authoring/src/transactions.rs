use crate::edit_operation::EditOperation;
use haven_core::{GameWorld, ProjectSceneId};

/// One editor gesture represented as a reversible logical unit.
///
/// command history once the gesture ends. Undo reverts operations in reverse
/// order; redo reapplies them in their original order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditTransaction {
    pub label: String,
    pub scene_id: ProjectSceneId,
    pub operations: Vec<EditOperation>,
}
impl EditTransaction {
    pub fn new(label: impl Into<String>, scene_id: impl Into<ProjectSceneId>) -> Self {
        Self {
            label: label.into(),
            scene_id: scene_id.into(),
            operations: Vec::new(),
        }
    }

    /// Adds an operation while coalescing repeated edits to the same logical target.
    pub fn push(&mut self, operation: EditOperation) {
        if self.coalesce(&operation) {
            return;
        }
        self.operations.push(operation);
    }

    pub fn extend(&mut self, operations: impl IntoIterator<Item = EditOperation>) {
        for operation in operations {
            self.push(operation);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }

    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }

    pub fn apply(&self, world: &mut GameWorld) -> Result<(), String> {
        self.execute_atomically(world, EditDirection::Forward)
    }

    pub fn revert(&self, world: &mut GameWorld) -> Result<(), String> {
        self.execute_atomically(world, EditDirection::Reverse)
    }

    fn execute_atomically(
        &self,
        world: &mut GameWorld,
        direction: EditDirection,
    ) -> Result<(), String> {
        let backup = world.clone();
        let result = match direction {
            EditDirection::Forward => self
                .operations
                .iter()
                .try_for_each(|operation| operation.apply(world, &self.scene_id)),
            EditDirection::Reverse => self
                .operations
                .iter()
                .rev()
                .try_for_each(|operation| operation.revert(world, &self.scene_id)),
        };
        if let Err(error) = result {
            *world = backup;
            return Err(error);
        }
        Ok(())
    }

    fn coalesce(&mut self, incoming: &EditOperation) -> bool {
        match incoming {
            EditOperation::SetTile { cell, after, .. } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::SetTile { cell: existing, .. } if existing == cell)
                }) {
                    let remove = match &mut self.operations[index] {
                        EditOperation::SetTile {
                            before,
                            after: existing_after,
                            ..
                        } => {
                            *existing_after = *after;
                            *before == *existing_after
                        }
                        _ => false,
                    };
                    if remove {
                        self.operations.remove(index);
                    }
                    return true;
                }
            }
            EditOperation::SetHeight { cell, after, .. } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::SetHeight { cell: existing, .. } if existing == cell)
                }) {
                    let remove = match &mut self.operations[index] {
                        EditOperation::SetHeight {
                            before,
                            after: existing_after,
                            ..
                        } => {
                            *existing_after = *after;
                            *before == *existing_after
                        }
                        _ => false,
                    };
                    if remove {
                        self.operations.remove(index);
                    }
                    return true;
                }
            }
            EditOperation::SetStructuralLevel { cell, after, .. } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::SetStructuralLevel { cell: existing, .. } if existing == cell)
                }) {
                    let remove = match &mut self.operations[index] {
                        EditOperation::SetStructuralLevel {
                            before,
                            after: existing_after,
                            ..
                        } => {
                            *existing_after = *after;
                            *before == *existing_after
                        }
                        _ => false,
                    };
                    if remove {
                        self.operations.remove(index);
                    }
                    return true;
                }
            }
            EditOperation::SetAutotileOverride { cell, after, .. } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::SetAutotileOverride { cell: existing, .. } if existing == cell)
                }) {
                    let remove = match &mut self.operations[index] {
                        EditOperation::SetAutotileOverride {
                            before,
                            after: existing_after,
                            ..
                        } => {
                            *existing_after = *after;
                            *before == *existing_after
                        }
                        _ => false,
                    };
                    if remove {
                        self.operations.remove(index);
                    }
                    return true;
                }
            }
            EditOperation::SetZone { cell, after, .. } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::SetZone { cell: existing, .. } if existing == cell)
                }) {
                    let remove = match &mut self.operations[index] {
                        EditOperation::SetZone {
                            before,
                            after: existing_after,
                            ..
                        } => {
                            *existing_after = *after;
                            *before == *existing_after
                        }
                        _ => false,
                    };
                    if remove {
                        self.operations.remove(index);
                    }
                    return true;
                }
            }
            EditOperation::MoveObject { id, after, .. } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::MoveObject { id: existing, .. } if existing == id)
                }) {
                    let remove = match &mut self.operations[index] {
                        EditOperation::MoveObject {
                            before,
                            after: existing_after,
                            ..
                        } => {
                            *existing_after = *after;
                            *before == *existing_after
                        }
                        _ => false,
                    };
                    if remove {
                        self.operations.remove(index);
                    }
                    return true;
                }
            }
            EditOperation::MoveStamp { id, after, .. } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::MoveStamp { id: existing, .. } if existing == id)
                }) {
                    let remove = match &mut self.operations[index] {
                        EditOperation::MoveStamp {
                            before,
                            after: existing_after,
                            ..
                        } => {
                            *existing_after = *after;
                            *before == *existing_after
                        }
                        _ => false,
                    };
                    if remove {
                        self.operations.remove(index);
                    }
                    return true;
                }
            }
            EditOperation::ResizeTransition { id, after, .. } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::ResizeTransition { id: existing, .. } if existing == id)
                }) {
                    let remove = match &mut self.operations[index] {
                        EditOperation::ResizeTransition {
                            before,
                            after: existing_after,
                            ..
                        } => {
                            *existing_after = *after;
                            *before == *existing_after
                        }
                        _ => false,
                    };
                    if remove {
                        self.operations.remove(index);
                    }
                    return true;
                }
            }
            EditOperation::RemoveObject { object } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::InsertObject { object: inserted } if inserted.id == object.id)
                }) {
                    self.operations.remove(index);
                    return true;
                }
            }
            EditOperation::RemoveStamp { stamp } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::InsertStamp { stamp: inserted } if inserted.id == stamp.id)
                }) {
                    self.operations.remove(index);
                    return true;
                }
            }
            EditOperation::RemoveTransition { transition } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::InsertTransition { transition: inserted } if inserted.id == transition.id)
                }) {
                    self.operations.remove(index);
                    return true;
                }
            }
            EditOperation::UpdateObject { id, after, .. } => {
                if let Some(index) = self.operations.iter().position(|operation| {
                    matches!(operation, EditOperation::UpdateObject { id: existing, .. } if existing == id)
                }) {
                    let remove = match &mut self.operations[index] {
                        EditOperation::UpdateObject {
                            before,
                            after: existing_after,
                            ..
                        } => {
                            *existing_after = *after;
                            *before == *existing_after
                        }
                        _ => false,
                    };
                    if remove {
                        self.operations.remove(index);
                    }
                    return true;
                }
            }
            EditOperation::InsertObject { .. }
            | EditOperation::InsertStamp { .. }
            | EditOperation::InsertTransition { .. }
            | EditOperation::InsertScene { .. }
            | EditOperation::RemoveScene { .. }
            | EditOperation::RenameScene { .. } => {}
        }
        false
    }
}
#[derive(Clone, Copy)]
enum EditDirection {
    Forward,
    Reverse,
}

#[cfg(test)]
#[path = "transactions_tests.rs"]
mod transactions_tests;
