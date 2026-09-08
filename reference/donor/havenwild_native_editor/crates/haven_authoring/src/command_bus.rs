use haven_core::GameWorld;
use serde::{Deserialize, Serialize};

use crate::{EditTransaction, EditTransactionBatch};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorCommandSource {
    MainEditor,
    InGameOverlay,
    WebEditor,
    RuntimeScript,
    Importer,
    ValidatorAutoFix,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorCommandKind {
    PaintTerrain,
    EditAutotile,
    SetHeight,
    SetStructuralLevel,
    PlaceObject,
    PlaceStamp,
    EditStamp,
    EditObjectFootprint,
    AssignRoom,
    CreateTransition,
    EditTransition,
    SetSceneSpawn,
    ResetScene,
    RegenerateScene,
    SaveWorld,
    LoadWorld,
    ValidateWorld,
    ExportWorldgen,
    SceneMutation,
}

impl EditorCommandKind {
    pub fn label(self) -> &'static str {
        match self {
            EditorCommandKind::PaintTerrain => "Paint terrain",
            EditorCommandKind::EditAutotile => "Edit autotile override",
            EditorCommandKind::SetHeight => "Set height",
            EditorCommandKind::SetStructuralLevel => "Set structural level",
            EditorCommandKind::PlaceObject => "Place object",
            EditorCommandKind::PlaceStamp => "Place stamp",
            EditorCommandKind::EditStamp => "Edit stamp",
            EditorCommandKind::EditObjectFootprint => "Edit object footprint",
            EditorCommandKind::AssignRoom => "Assign room/zone",
            EditorCommandKind::CreateTransition => "Create transition",
            EditorCommandKind::EditTransition => "Edit transition",
            EditorCommandKind::SetSceneSpawn => "Set scene spawn",
            EditorCommandKind::ResetScene => "Reset scene",
            EditorCommandKind::RegenerateScene => "Regenerate scene",
            EditorCommandKind::SaveWorld => "Save world",
            EditorCommandKind::LoadWorld => "Load world",
            EditorCommandKind::ValidateWorld => "Validate world",
            EditorCommandKind::ExportWorldgen => "Export worldgen",
            EditorCommandKind::SceneMutation => "Scene mutation",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GridPos {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandTarget {
    pub project_id: String,
    pub scene_id: Option<String>,
    pub asset_id: Option<String>,
    pub grid_cells: Vec<GridPos>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandPayload {
    pub description: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EditorCommand {
    pub command_id: String,
    pub command_type: String,
    pub source: EditorCommandSource,
    pub target: CommandTarget,
    pub payload: CommandPayload,
    pub preview_only: bool,
    pub validation_result: Option<serde_json::Value>,
    pub undo_patch: Option<serde_json::Value>,
}

impl EditorCommand {
    pub fn new(
        kind: EditorCommandKind,
        source: EditorCommandSource,
        project_id: impl Into<String>,
        scene_id: Option<String>,
        asset_id: Option<String>,
        grid_cells: Vec<GridPos>,
        description: impl Into<String>,
    ) -> Self {
        let command_type = kind.label().to_string();
        let description = description.into();
        Self {
            command_id: format!(
                "{}-{}",
                kind.label().to_ascii_lowercase().replace(' ', "_"),
                command_hash_seed(&description)
            ),
            command_type,
            source,
            target: CommandTarget {
                project_id: project_id.into(),
                scene_id,
                asset_id,
                grid_cells,
            },
            payload: CommandPayload {
                description,
                data: None,
            },
            preview_only: false,
            validation_result: None,
            undo_patch: None,
        }
    }

    pub fn label(&self) -> &str {
        &self.command_type
    }
}

/// Legacy snapshot return value retained for the in-game developer overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandUndoStep {
    pub label: String,
    pub snapshot: String,
}

/// Result returned by transaction-aware world undo/redo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandHistoryStep {
    pub label: String,
    pub typed_transaction: bool,
    pub operation_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
enum RecordedEdit {
    Snapshot {
        command: EditorCommand,
        snapshot: String,
    },
    Transaction {
        command: EditorCommand,
        transaction: EditTransaction,
    },
    TransactionBatch {
        command: EditorCommand,
        batch: EditTransactionBatch,
    },
}

impl RecordedEdit {
    fn command(&self) -> &EditorCommand {
        match self {
            RecordedEdit::Snapshot { command, .. }
            | RecordedEdit::Transaction { command, .. }
            | RecordedEdit::TransactionBatch { command, .. } => command,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct ActiveGesture {
    command: EditorCommand,
    batch: EditTransactionBatch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EditorCommandBus {
    undo_limit: usize,
    undo_stack: Vec<RecordedEdit>,
    redo_stack: Vec<RecordedEdit>,
    recent_commands: Vec<EditorCommand>,
    gesture_open: bool,
    active_gesture: Option<ActiveGesture>,
}

impl EditorCommandBus {
    pub fn with_limit(undo_limit: usize) -> Self {
        Self {
            undo_limit,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            recent_commands: Vec::new(),
            gesture_open: false,
            active_gesture: None,
        }
    }

    /// Opens a gesture so subsequent typed edits coalesce into one undo entry.
    pub fn begin_gesture(&mut self) {
        if self.gesture_open {
            return;
        }
        self.gesture_open = true;
        self.active_gesture = None;
    }

    pub fn gesture_is_open(&self) -> bool {
        self.gesture_open
    }

    pub fn active_operation_count(&self) -> usize {
        self.active_gesture
            .as_ref()
            .map_or(0, |gesture| gesture.batch.operation_count())
    }

    /// Records a typed transaction. When a gesture is open, operations from
    /// every touched scene coalesce into one cross-scene undo entry.
    pub fn record_transaction(&mut self, command: EditorCommand, transaction: EditTransaction) {
        if transaction.is_empty() {
            return;
        }
        let mut batch = EditTransactionBatch::new(transaction.label.clone());
        batch.push(transaction);
        self.record_transaction_batch(command, batch);
    }

    pub fn record_transaction_batch(
        &mut self,
        command: EditorCommand,
        batch: EditTransactionBatch,
    ) {
        if batch.is_empty() {
            return;
        }
        if !self.gesture_open {
            self.push_transaction_batch(command, batch);
            return;
        }

        if let Some(active) = self.active_gesture.as_mut() {
            active.batch.extend(batch.transactions);
            merge_command_targets(&mut active.command, &command);
            active.batch.label = command.payload.description.clone();
            active.command.payload.description = active.batch.label.clone();
            return;
        }

        self.active_gesture = Some(ActiveGesture { command, batch });
    }

    pub fn commit_gesture(&mut self) -> Option<CommandHistoryStep> {
        self.gesture_open = false;
        let gesture = self.active_gesture.take()?;
        if gesture.batch.is_empty() {
            return None;
        }
        let step = CommandHistoryStep {
            label: gesture.batch.label.clone(),
            typed_transaction: true,
            operation_count: gesture.batch.operation_count(),
        };
        self.push_transaction_batch(gesture.command, gesture.batch);
        Some(step)
    }

    /// Cancels an in-progress gesture and reverts all uncommitted scene edits atomically.
    pub fn cancel_gesture(&mut self, world: &mut GameWorld) -> Result<(), String> {
        self.gesture_open = false;
        let Some(gesture) = self.active_gesture.take() else {
            return Ok(());
        };
        gesture.batch.revert(world)
    }

    fn push_transaction_batch(&mut self, command: EditorCommand, mut batch: EditTransactionBatch) {
        if batch.transactions.len() == 1 {
            let transaction = batch
                .transactions
                .pop()
                .expect("one transaction batch must contain one transaction");
            self.push_record(RecordedEdit::Transaction {
                command,
                transaction,
            });
        } else {
            self.push_record(RecordedEdit::TransactionBatch { command, batch });
        }
    }

    /// Compatibility fallback for systems that have not migrated to typed edits.
    pub fn capture_undo_snapshot(&mut self, command: EditorCommand, snapshot: String) {
        let _ = self.commit_gesture();
        if self.undo_stack.last().is_some_and(|entry| {
            matches!(entry, RecordedEdit::Snapshot { snapshot: previous, .. } if previous == &snapshot)
        }) {
            return;
        }
        self.push_record(RecordedEdit::Snapshot { command, snapshot });
    }

    pub fn record_event(&mut self, command: EditorCommand) {
        self.push_recent(command);
    }

    /// Legacy snapshot-only undo used by the in-game overlay until it migrates.
    pub fn undo(&mut self, current_snapshot: &str) -> Result<CommandUndoStep, String> {
        let _ = self.commit_gesture();
        let Some(previous) = self.undo_stack.pop() else {
            return Err("Undo stack is empty".to_string());
        };
        match previous {
            RecordedEdit::Snapshot { command, snapshot } => {
                self.redo_stack.push(RecordedEdit::Snapshot {
                    command: command.clone(),
                    snapshot: current_snapshot.to_string(),
                });
                Ok(CommandUndoStep {
                    label: command.label().to_string(),
                    snapshot,
                })
            }
            transaction @ (RecordedEdit::Transaction { .. }
            | RecordedEdit::TransactionBatch { .. }) => {
                self.undo_stack.push(transaction);
                Err("Typed transaction history requires undo_world".to_string())
            }
        }
    }

    /// Legacy snapshot-only redo used by the in-game overlay until it migrates.
    pub fn redo(&mut self, current_snapshot: &str) -> Result<CommandUndoStep, String> {
        let _ = self.commit_gesture();
        let Some(next) = self.redo_stack.pop() else {
            return Err("Redo stack is empty".to_string());
        };
        match next {
            RecordedEdit::Snapshot { command, snapshot } => {
                self.undo_stack.push(RecordedEdit::Snapshot {
                    command: command.clone(),
                    snapshot: current_snapshot.to_string(),
                });
                Ok(CommandUndoStep {
                    label: command.label().to_string(),
                    snapshot,
                })
            }
            transaction @ (RecordedEdit::Transaction { .. }
            | RecordedEdit::TransactionBatch { .. }) => {
                self.redo_stack.push(transaction);
                Err("Typed transaction history requires redo_world".to_string())
            }
        }
    }

    /// Transaction-aware undo used by the native editor. Snapshot entries remain
    /// supported as a migration fallback in the same history.
    pub fn undo_world(&mut self, world: &mut GameWorld) -> Result<CommandHistoryStep, String> {
        let _ = self.commit_gesture();
        let Some(previous) = self.undo_stack.pop() else {
            return Err("Undo stack is empty".to_string());
        };
        match previous {
            RecordedEdit::Transaction {
                command,
                transaction,
            } => {
                if let Err(error) = transaction.revert(world) {
                    self.undo_stack.push(RecordedEdit::Transaction {
                        command,
                        transaction,
                    });
                    return Err(format!("Undo transaction failed: {error}"));
                }
                let step = CommandHistoryStep {
                    label: transaction.label.clone(),
                    typed_transaction: true,
                    operation_count: transaction.operation_count(),
                };
                self.redo_stack.push(RecordedEdit::Transaction {
                    command,
                    transaction,
                });
                Ok(step)
            }
            RecordedEdit::TransactionBatch { command, batch } => {
                if let Err(error) = batch.revert(world) {
                    self.undo_stack
                        .push(RecordedEdit::TransactionBatch { command, batch });
                    return Err(format!("Undo transaction batch failed: {error}"));
                }
                let step = CommandHistoryStep {
                    label: batch.label.clone(),
                    typed_transaction: true,
                    operation_count: batch.operation_count(),
                };
                self.redo_stack
                    .push(RecordedEdit::TransactionBatch { command, batch });
                Ok(step)
            }
            RecordedEdit::Snapshot { command, snapshot } => {
                let current_snapshot = world.serialize_lines();
                let restored = GameWorld::deserialize_lines(&snapshot)
                    .map_err(|error| format!("Undo snapshot restore failed: {error}"));
                match restored {
                    Ok(restored) => {
                        *world = restored;
                        let step = CommandHistoryStep {
                            label: command.label().to_string(),
                            typed_transaction: false,
                            operation_count: 0,
                        };
                        self.redo_stack.push(RecordedEdit::Snapshot {
                            command,
                            snapshot: current_snapshot,
                        });
                        Ok(step)
                    }
                    Err(error) => {
                        self.undo_stack
                            .push(RecordedEdit::Snapshot { command, snapshot });
                        Err(error)
                    }
                }
            }
        }
    }

    pub fn redo_world(&mut self, world: &mut GameWorld) -> Result<CommandHistoryStep, String> {
        let _ = self.commit_gesture();
        let Some(next) = self.redo_stack.pop() else {
            return Err("Redo stack is empty".to_string());
        };
        match next {
            RecordedEdit::Transaction {
                command,
                transaction,
            } => {
                if let Err(error) = transaction.apply(world) {
                    self.redo_stack.push(RecordedEdit::Transaction {
                        command,
                        transaction,
                    });
                    return Err(format!("Redo transaction failed: {error}"));
                }
                let step = CommandHistoryStep {
                    label: transaction.label.clone(),
                    typed_transaction: true,
                    operation_count: transaction.operation_count(),
                };
                self.undo_stack.push(RecordedEdit::Transaction {
                    command,
                    transaction,
                });
                Ok(step)
            }
            RecordedEdit::TransactionBatch { command, batch } => {
                if let Err(error) = batch.apply(world) {
                    self.redo_stack
                        .push(RecordedEdit::TransactionBatch { command, batch });
                    return Err(format!("Redo transaction batch failed: {error}"));
                }
                let step = CommandHistoryStep {
                    label: batch.label.clone(),
                    typed_transaction: true,
                    operation_count: batch.operation_count(),
                };
                self.undo_stack
                    .push(RecordedEdit::TransactionBatch { command, batch });
                Ok(step)
            }
            RecordedEdit::Snapshot { command, snapshot } => {
                let current_snapshot = world.serialize_lines();
                let restored = GameWorld::deserialize_lines(&snapshot)
                    .map_err(|error| format!("Redo snapshot restore failed: {error}"));
                match restored {
                    Ok(restored) => {
                        *world = restored;
                        let step = CommandHistoryStep {
                            label: command.label().to_string(),
                            typed_transaction: false,
                            operation_count: 0,
                        };
                        self.undo_stack.push(RecordedEdit::Snapshot {
                            command,
                            snapshot: current_snapshot,
                        });
                        Ok(step)
                    }
                    Err(error) => {
                        self.redo_stack
                            .push(RecordedEdit::Snapshot { command, snapshot });
                        Err(error)
                    }
                }
            }
        }
    }

    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.active_gesture = None;
        self.gesture_open = false;
    }

    pub fn undo_len(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn typed_undo_len(&self) -> usize {
        self.undo_stack
            .iter()
            .filter(|entry| {
                matches!(
                    entry,
                    RecordedEdit::Transaction { .. } | RecordedEdit::TransactionBatch { .. }
                )
            })
            .count()
    }

    pub fn snapshot_undo_len(&self) -> usize {
        self.undo_stack
            .iter()
            .filter(|entry| matches!(entry, RecordedEdit::Snapshot { .. }))
            .count()
    }

    pub fn recent_commands(&self) -> &[EditorCommand] {
        &self.recent_commands
    }

    fn push_record(&mut self, record: RecordedEdit) {
        let command = record.command().clone();
        self.undo_stack.push(record);
        if self.undo_stack.len() > self.undo_limit {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.push_recent(command);
    }

    fn push_recent(&mut self, command: EditorCommand) {
        self.recent_commands.push(command);
        if self.recent_commands.len() > self.undo_limit {
            self.recent_commands.remove(0);
        }
    }
}

fn merge_command_targets(active: &mut EditorCommand, incoming: &EditorCommand) {
    for cell in &incoming.target.grid_cells {
        if !active.target.grid_cells.contains(cell) {
            active.target.grid_cells.push(*cell);
        }
    }
}

fn command_hash_seed(input: &str) -> u64 {
    input.bytes().fold(0u64, |seed, byte| {
        seed.wrapping_mul(131).wrapping_add(byte as u64)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EditOperation, GridPos};
    use haven_core::{
        GameWorld, ProjectSceneId, SceneBiome, SceneId, SceneKind, SceneMap, TileKind,
    };

    #[test]
    fn gesture_commits_multiple_cells_as_one_history_entry() {
        let mut bus = EditorCommandBus::with_limit(8);
        let scene_id = haven_core::ProjectSceneId::from(SceneId::Farmstead);
        bus.begin_gesture();
        for x in 2..5 {
            let command = EditorCommand::new(
                EditorCommandKind::PaintTerrain,
                EditorCommandSource::MainEditor,
                "test",
                Some(scene_id.code().to_string()),
                Some("road".to_string()),
                vec![GridPos { x, y: 3 }],
                "Paint road",
            );
            let mut transaction = EditTransaction::new("Paint road stroke", scene_id.clone());
            transaction.push(EditOperation::SetTile {
                cell: GridPos { x, y: 3 },
                before: TileKind::Grass,
                after: TileKind::Road,
            });
            bus.record_transaction(command, transaction);
        }
        assert_eq!(bus.undo_len(), 0);
        let step = bus.commit_gesture().expect("gesture should commit");
        assert_eq!(step.operation_count, 3);
        assert_eq!(bus.undo_len(), 1);
        assert_eq!(bus.typed_undo_len(), 1);
    }
    #[test]
    fn gesture_groups_transactions_from_multiple_scenes() {
        let mut bus = EditorCommandBus::with_limit(8);
        let scene_a = haven_core::ProjectSceneId::new("scene_a");
        let scene_b = haven_core::ProjectSceneId::new("scene_b");
        bus.begin_gesture();
        for (scene_id, x) in [(scene_a, 1), (scene_b, 2)] {
            let command = EditorCommand::new(
                EditorCommandKind::PaintTerrain,
                EditorCommandSource::MainEditor,
                "test",
                Some(scene_id.code().to_string()),
                Some("road".to_string()),
                vec![GridPos { x, y: 3 }],
                "Paint global road",
            );
            let mut transaction = EditTransaction::new("Paint global road", scene_id);
            transaction.push(EditOperation::SetTile {
                cell: GridPos { x, y: 3 },
                before: TileKind::Grass,
                after: TileKind::Road,
            });
            bus.record_transaction(command, transaction);
        }
        let step = bus.commit_gesture().expect("global gesture should commit");
        assert_eq!(step.operation_count, 2);
        assert_eq!(bus.undo_len(), 1);
        assert_eq!(bus.typed_undo_len(), 1);
    }

    #[test]
    fn cross_scene_batch_undo_and_redo_are_atomic() {
        let mut world = GameWorld::starter();
        for (id, name) in [("scene_a", "Scene A"), ("scene_b", "Scene B")] {
            world
                .scenes
                .insert(SceneMap::blank(
                    id,
                    name,
                    SceneKind::Exterior,
                    SceneBiome::Temperate,
                ))
                .expect("insert test scene");
        }
        let mut bus = EditorCommandBus::with_limit(8);
        bus.begin_gesture();
        for (scene_code, x) in [("scene_a", 1), ("scene_b", 2)] {
            let scene_id = ProjectSceneId::new(scene_code);
            world
                .scene_mut_by_id(&scene_id)
                .expect("scene")
                .map
                .set(x, 3, TileKind::Road);
            let command = EditorCommand::new(
                EditorCommandKind::PaintTerrain,
                EditorCommandSource::MainEditor,
                "test",
                Some(scene_id.code().to_string()),
                Some("road".to_string()),
                vec![GridPos { x, y: 3 }],
                "Paint cross-scene road",
            );
            let mut transaction = EditTransaction::new("Paint cross-scene road", scene_id);
            transaction.push(EditOperation::SetTile {
                cell: GridPos { x, y: 3 },
                before: TileKind::Grass,
                after: TileKind::Road,
            });
            bus.record_transaction(command, transaction);
        }
        bus.commit_gesture().expect("commit");
        bus.undo_world(&mut world).expect("undo batch");
        assert_eq!(
            world
                .scene_by_id(&ProjectSceneId::new("scene_a"))
                .expect("scene a")
                .map
                .get(1, 3),
            TileKind::Grass
        );
        assert_eq!(
            world
                .scene_by_id(&ProjectSceneId::new("scene_b"))
                .expect("scene b")
                .map
                .get(2, 3),
            TileKind::Grass
        );
        bus.redo_world(&mut world).expect("redo batch");
        assert_eq!(
            world
                .scene_by_id(&ProjectSceneId::new("scene_a"))
                .expect("scene a")
                .map
                .get(1, 3),
            TileKind::Road
        );
        assert_eq!(
            world
                .scene_by_id(&ProjectSceneId::new("scene_b"))
                .expect("scene b")
                .map
                .get(2, 3),
            TileKind::Road
        );
    }
}
