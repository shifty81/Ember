//! Typed visual-graph contract layered over Ember's executable behavior graph.

use ember_core::StableId;
use ember_nodes::{BehaviorGraph, Node, NodeKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValueType {
    Number,
    Boolean,
    Text,
    Entity,
    Asset,
    Any,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PortKind {
    Execution,
    Data,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PortDefinition {
    pub id: String,
    pub direction: PortDirection,
    pub kind: PortKind,
    pub value_type: Option<ValueType>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum TypedNodeKind {
    EventStart,
    SetNumber { key: String, value: f64 },
    AddNumber { key: String, value: f64 },
    BranchGreater { key: String, threshold: f64 },
    Debug { message: String },
    End,
    Custom(StableId),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TypedNode {
    pub id: StableId,
    pub kind: TypedNodeKind,
    #[serde(default)]
    pub ports: Vec<PortDefinition>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphConnection {
    pub from_node: StableId,
    pub from_port: String,
    pub to_node: StableId,
    pub to_port: String,
    #[serde(default)]
    pub order: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TypedGraph {
    pub id: StableId,
    pub entry: StableId,
    pub nodes: BTreeMap<StableId, TypedNode>,
    #[serde(default)]
    pub connections: Vec<GraphConnection>,
}

impl TypedGraph {
    pub fn validate(&self) -> Result<(), String> {
        if !self.nodes.contains_key(&self.entry) {
            return Err("typed graph entry node is missing".into());
        }
        for node in self.nodes.values() {
            let mut ids = BTreeSet::new();
            for port in &node.ports {
                if port.id.trim().is_empty() {
                    return Err(format!("node {} has an empty port id", node.id));
                }
                if !ids.insert(port.id.clone()) {
                    return Err(format!("node {} has duplicate port {}", node.id, port.id));
                }
                if port.kind == PortKind::Execution && port.value_type.is_some() {
                    return Err(format!(
                        "execution port {} on {} cannot declare a value type",
                        port.id, node.id
                    ));
                }
                if port.kind == PortKind::Data && port.value_type.is_none() {
                    return Err(format!(
                        "data port {} on {} must declare a value type",
                        port.id, node.id
                    ));
                }
            }
        }
        for connection in &self.connections {
            let from = self
                .nodes
                .get(&connection.from_node)
                .ok_or_else(|| format!("missing source node {}", connection.from_node))?;
            let to = self
                .nodes
                .get(&connection.to_node)
                .ok_or_else(|| format!("missing destination node {}", connection.to_node))?;
            let from_port = from
                .ports
                .iter()
                .find(|port| port.id == connection.from_port)
                .ok_or_else(|| {
                    format!(
                        "missing source port {} on {}",
                        connection.from_port, connection.from_node
                    )
                })?;
            let to_port = to
                .ports
                .iter()
                .find(|port| port.id == connection.to_port)
                .ok_or_else(|| {
                    format!(
                        "missing destination port {} on {}",
                        connection.to_port, connection.to_node
                    )
                })?;
            if from_port.direction != PortDirection::Output
                || to_port.direction != PortDirection::Input
            {
                return Err("graph connection direction is invalid".into());
            }
            if from_port.kind != to_port.kind {
                return Err("graph connection mixes execution and data ports".into());
            }
            if from_port.kind == PortKind::Data
                && from_port.value_type != to_port.value_type
                && from_port.value_type != Some(ValueType::Any)
                && to_port.value_type != Some(ValueType::Any)
            {
                return Err("graph data connection has incompatible value types".into());
            }
        }
        Ok(())
    }

    pub fn compile_behavior(&self) -> Result<BehaviorGraph, String> {
        self.validate()?;
        let mut nodes = BTreeMap::new();
        for node in self.nodes.values() {
            let kind = match &node.kind {
                TypedNodeKind::EventStart => NodeKind::EventStart,
                TypedNodeKind::SetNumber { key, value } => NodeKind::SetNumber {
                    key: key.clone(),
                    value: *value,
                },
                TypedNodeKind::AddNumber { key, value } => NodeKind::AddNumber {
                    key: key.clone(),
                    value: *value,
                },
                TypedNodeKind::BranchGreater { key, threshold } => NodeKind::BranchGreater {
                    key: key.clone(),
                    threshold: *threshold,
                },
                TypedNodeKind::Debug { message } => NodeKind::Debug {
                    message: message.clone(),
                },
                TypedNodeKind::End => NodeKind::End,
                TypedNodeKind::Custom(id) => {
                    return Err(format!("custom typed node {id} has no runtime compiler"));
                }
            };
            let mut outgoing = self
                .connections
                .iter()
                .filter(|connection| connection.from_node == node.id)
                .filter_map(|connection| {
                    let port = node
                        .ports
                        .iter()
                        .find(|port| port.id == connection.from_port)?;
                    (port.kind == PortKind::Execution)
                        .then_some((connection.order, connection.to_node.clone()))
                })
                .collect::<Vec<_>>();
            outgoing.sort_by_key(|(order, _)| *order);
            nodes.insert(
                node.id.clone(),
                Node {
                    kind,
                    next: outgoing.into_iter().map(|(_, id)| id).collect(),
                },
            );
        }
        let graph = BehaviorGraph {
            id: self.id.clone(),
            entry: self.entry.clone(),
            nodes,
        };
        graph.validate().map_err(|error| error.to_string())?;
        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exec_port(id: &str, direction: PortDirection) -> PortDefinition {
        PortDefinition {
            id: id.into(),
            direction,
            kind: PortKind::Execution,
            value_type: None,
        }
    }

    #[test]
    fn typed_graph_compiles_to_behavior_graph() {
        let start = StableId::new("node", "start").unwrap();
        let end = StableId::new("node", "end").unwrap();
        let mut nodes = BTreeMap::new();
        nodes.insert(
            start.clone(),
            TypedNode {
                id: start.clone(),
                kind: TypedNodeKind::EventStart,
                ports: vec![exec_port("out", PortDirection::Output)],
            },
        );
        nodes.insert(
            end.clone(),
            TypedNode {
                id: end.clone(),
                kind: TypedNodeKind::End,
                ports: vec![exec_port("in", PortDirection::Input)],
            },
        );
        let graph = TypedGraph {
            id: StableId::new("graph", "test").unwrap(),
            entry: start.clone(),
            nodes,
            connections: vec![GraphConnection {
                from_node: start,
                from_port: "out".into(),
                to_node: end,
                to_port: "in".into(),
                order: 0,
            }],
        };
        assert_eq!(graph.compile_behavior().unwrap().nodes.len(), 2);
    }
}
