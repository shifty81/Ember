use ember_core::StableId;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BehaviorGraph {
    pub id: StableId,
    pub entry: StableId,
    pub nodes: BTreeMap<StableId, Node>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Node {
    pub kind: NodeKind,
    pub next: Vec<StableId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum NodeKind {
    EventStart,
    SetNumber { key: String, value: f64 },
    AddNumber { key: String, value: f64 },
    BranchGreater { key: String, threshold: f64 },
    Debug { message: String },
    End,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BehaviorState {
    pub numbers: BTreeMap<String, f64>,
    pub trace: Vec<String>,
}
impl BehaviorGraph {
    pub fn validate(&self) -> Result<(), GraphError> {
        if !self.nodes.contains_key(&self.entry) {
            return Err(GraphError("missing entry node".into()));
        }
        for n in self.nodes.values() {
            for id in &n.next {
                if !self.nodes.contains_key(id) {
                    return Err(GraphError(format!("missing node {id}")));
                }
            }
        }
        Ok(())
    }
    pub fn execute(&self, state: &mut BehaviorState) -> Result<(), GraphError> {
        self.validate()?;
        let mut q = VecDeque::from([self.entry.clone()]);
        let mut budget = 4096usize;
        while let Some(id) = q.pop_front() {
            if budget == 0 {
                return Err(GraphError("execution budget exceeded".into()));
            }
            budget -= 1;
            let node = &self.nodes[&id];
            state.trace.push(id.to_string());
            match &node.kind {
                NodeKind::SetNumber { key, value } => {
                    state.numbers.insert(key.clone(), *value);
                }
                NodeKind::AddNumber { key, value } => {
                    *state.numbers.entry(key.clone()).or_default() += value;
                }
                NodeKind::BranchGreater { key, threshold } => {
                    let yes = state.numbers.get(key).copied().unwrap_or_default() > *threshold;
                    if let Some(next) = node.next.get(if yes { 0 } else { 1 }) {
                        q.push_back(next.clone())
                    }
                    continue;
                }
                NodeKind::Debug { message } => state.trace.push(message.clone()),
                NodeKind::End => continue,
                NodeKind::EventStart => {}
            }
            q.extend(node.next.iter().cloned());
        }
        Ok(())
    }
}
#[derive(Debug)]
pub struct GraphError(pub String);
impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for GraphError {}
