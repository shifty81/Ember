//! Ember game-engine/runtime layer.

use ember_core::StableId;
use ember_level::LevelDocument;
use ember_nodes::{BehaviorGraph, BehaviorState};
use ember_scene::{CollisionSemantic, EntityInstance, SemanticCell};
use std::collections::BTreeMap;

pub type BehaviorLibrary = BTreeMap<StableId, BehaviorGraph>;

#[derive(Default)]
pub struct RuntimeWorld {
    pub entities: BTreeMap<StableId, RuntimeEntity>,
    pub semantic_cells: Vec<RuntimeSemanticCell>,
}

pub struct RuntimeEntity {
    pub id: StableId,
    pub definition: StableId,
    pub position: [f32; 3],
    pub behaviors: Vec<BehaviorGraph>,
    pub state: BehaviorState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeSemanticCell {
    pub grid: [i32; 3],
    pub collision: Option<CollisionSemantic>,
    pub navigation_cost: Option<f32>,
    pub tags: Vec<StableId>,
}

#[derive(Debug)]
pub struct CompileError(pub String);
impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for CompileError {}

pub fn compile_level(
    level: &LevelDocument,
    behaviors: &BehaviorLibrary,
) -> Result<RuntimeWorld, CompileError> {
    level
        .validate()
        .map_err(|errors| CompileError(errors.join("; ")))?;
    let mut world = RuntimeWorld::default();

    for entity in &level.scene.entities {
        let runtime = compile_entity(entity, behaviors)?;
        if world.entities.insert(runtime.id.clone(), runtime).is_some() {
            return Err(CompileError(format!(
                "duplicate runtime entity {}",
                entity.id
            )));
        }
    }

    for layer in &level.scene.layers {
        for cell in &layer.semantic_cells {
            world.semantic_cells.push(compile_semantic(cell));
        }
    }
    Ok(world)
}

fn compile_entity(
    entity: &EntityInstance,
    behaviors: &BehaviorLibrary,
) -> Result<RuntimeEntity, CompileError> {
    let mut compiled_behaviors = Vec::new();
    for id in &entity.behavior_graphs {
        let graph = behaviors.get(id).ok_or_else(|| {
            CompileError(format!(
                "entity {} references missing behavior {}",
                entity.id, id
            ))
        })?;
        graph
            .validate()
            .map_err(|error| CompileError(format!("behavior {} invalid: {error}", id)))?;
        compiled_behaviors.push(graph.clone());
    }
    Ok(RuntimeEntity {
        id: entity.id.clone(),
        definition: entity.definition.clone(),
        position: entity.transform.position,
        behaviors: compiled_behaviors,
        state: BehaviorState::default(),
    })
}

fn compile_semantic(cell: &SemanticCell) -> RuntimeSemanticCell {
    RuntimeSemanticCell {
        grid: cell.grid,
        collision: cell.semantic.collision.clone(),
        navigation_cost: cell.semantic.navigation_cost,
        tags: cell.semantic.tags.clone(),
    }
}

impl RuntimeWorld {
    pub fn tick_once(&mut self) -> Result<(), String> {
        for entity in self.entities.values_mut() {
            for graph in &entity.behaviors {
                graph
                    .execute(&mut entity.state)
                    .map_err(|error| error.to_string())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ember_level::LevelDocument;

    #[test]
    fn empty_level_compiles() {
        let level = LevelDocument::new(StableId::new("level", "empty").unwrap(), "Empty", 320, 180);
        let runtime = compile_level(&level, &BehaviorLibrary::new()).unwrap();
        assert!(runtime.entities.is_empty());
    }
}
