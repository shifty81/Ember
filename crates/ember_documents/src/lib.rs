use ember_core::{Diagnostic, StableId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DocumentEnvelope {
    pub format_version: u32,
    pub id: StableId,
    pub kind: DocumentKind,
    pub revision: u64,
    pub payload: serde_json::Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DocumentKind {
    Level,
    Scene,
    World,
    Entity,
    Prefab,
    PixelImage,
    Animation,
    BehaviorGraph,
    GuiLayout,
    AudioEvent,
    DataTable,
    WorldGenerationGraph,
    BuildProfile,
    Custom(String),
}
#[derive(Default)]
pub struct DocumentRegistry {
    documents: BTreeMap<StableId, DocumentEnvelope>,
}
impl DocumentRegistry {
    pub fn insert(&mut self, d: DocumentEnvelope) -> Option<DocumentEnvelope> {
        self.documents.insert(d.id.clone(), d)
    }
    pub fn get(&self, id: &StableId) -> Option<&DocumentEnvelope> {
        self.documents.get(id)
    }
    pub fn iter(&self) -> impl Iterator<Item = &DocumentEnvelope> {
        self.documents.values()
    }
    pub fn validate(&self) -> Vec<Diagnostic> {
        self.documents
            .values()
            .filter(|d| d.format_version == 0)
            .map(|d| Diagnostic::error("O2D-DOC-001", format!("{} has format version zero", d.id)))
            .collect()
    }
}
