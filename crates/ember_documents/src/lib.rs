use ember_core::{Diagnostic, StableId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DocumentEnvelope {
    pub format_version: u32,
    pub id: StableId,
    pub kind: DocumentKind,
    pub revision: u64,
    pub payload: Value,
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

impl DocumentEnvelope {
    pub fn from_typed<T: Serialize>(
        format_version: u32,
        id: StableId,
        kind: DocumentKind,
        revision: u64,
        payload: &T,
    ) -> Result<Self, DocumentIoError> {
        if format_version == 0 {
            return Err(DocumentIoError::Invalid(
                "format version cannot be zero".into(),
            ));
        }
        Ok(Self {
            format_version,
            id,
            kind,
            revision,
            payload: serde_json::to_value(payload)?,
        })
    }

    pub fn decode<T: DeserializeOwned>(&self) -> Result<T, DocumentIoError> {
        serde_json::from_value(self.payload.clone()).map_err(DocumentIoError::Json)
    }
}

#[derive(Default)]
pub struct DocumentRegistry {
    documents: BTreeMap<StableId, DocumentEnvelope>,
}

impl DocumentRegistry {
    pub fn insert(&mut self, document: DocumentEnvelope) -> Option<DocumentEnvelope> {
        self.documents.insert(document.id.clone(), document)
    }

    pub fn get(&self, id: &StableId) -> Option<&DocumentEnvelope> {
        self.documents.get(id)
    }

    pub fn get_mut(&mut self, id: &StableId) -> Option<&mut DocumentEnvelope> {
        self.documents.get_mut(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &DocumentEnvelope> {
        self.documents.values()
    }

    pub fn validate(&self) -> Vec<Diagnostic> {
        self.documents
            .values()
            .filter(|document| document.format_version == 0)
            .map(|document| {
                Diagnostic::error(
                    "EMBER-DOC-001",
                    format!("{} has format version zero", document.id),
                )
            })
            .collect()
    }
}

pub trait DocumentDependencies {
    fn document_dependencies(&self) -> Vec<StableId>;
}

pub type MigrationFn = fn(Value) -> Result<Value, String>;

#[derive(Default)]
pub struct MigrationRegistry {
    migrations: BTreeMap<(DocumentKind, u32), MigrationFn>,
}

impl MigrationRegistry {
    pub fn register(&mut self, kind: DocumentKind, from_version: u32, migration: MigrationFn) {
        let _ = self.migrations.insert((kind, from_version), migration);
    }

    pub fn migrate(
        &self,
        mut envelope: DocumentEnvelope,
        target_version: u32,
    ) -> Result<DocumentEnvelope, DocumentIoError> {
        if envelope.format_version == 0 || target_version == 0 {
            return Err(DocumentIoError::Invalid(
                "document version cannot be zero".into(),
            ));
        }
        if envelope.format_version > target_version {
            return Err(DocumentIoError::Invalid(format!(
                "document {} version {} is newer than supported target {}",
                envelope.id, envelope.format_version, target_version
            )));
        }
        while envelope.format_version < target_version {
            let key = (envelope.kind.clone(), envelope.format_version);
            let migration = self.migrations.get(&key).ok_or_else(|| {
                DocumentIoError::Invalid(format!(
                    "no migration for {:?} version {}",
                    envelope.kind, envelope.format_version
                ))
            })?;
            envelope.payload = migration(envelope.payload).map_err(DocumentIoError::Invalid)?;
            envelope.format_version += 1;
            envelope.revision = envelope.revision.saturating_add(1);
        }
        Ok(envelope)
    }
}

pub struct DocumentStore;

impl DocumentStore {
    pub fn load(path: &Path) -> Result<DocumentEnvelope, DocumentIoError> {
        let bytes = fs::read(path)?;
        let envelope: DocumentEnvelope = serde_json::from_slice(&bytes)?;
        if envelope.format_version == 0 {
            return Err(DocumentIoError::Invalid(
                "document format version is zero".into(),
            ));
        }
        Ok(envelope)
    }

    pub fn save_atomic(path: &Path, envelope: &DocumentEnvelope) -> Result<(), DocumentIoError> {
        if envelope.format_version == 0 {
            return Err(DocumentIoError::Invalid(
                "document format version is zero".into(),
            ));
        }
        let bytes = serde_json::to_vec_pretty(envelope)?;
        atomic_write(path, &bytes)?;
        Ok(())
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = sibling_temp_path(path);
    {
        let mut file = fs::File::create(&temp)?;
        file.write_all(bytes)?;
        file.flush()?;
        let _ = file.sync_all();
    }
    if path.exists() {
        #[cfg(windows)]
        {
            let backup = path.with_extension("ember-save.bak");
            let _ = fs::remove_file(&backup);
            fs::rename(path, &backup)?;
            match fs::rename(&temp, path) {
                Ok(()) => {
                    let _ = fs::remove_file(backup);
                    Ok(())
                }
                Err(error) => {
                    let _ = fs::rename(backup, path);
                    Err(error)
                }
            }
        }
        #[cfg(not(windows))]
        {
            fs::rename(&temp, path)
        }
    } else {
        fs::rename(&temp, path)
    }
}

fn sibling_temp_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document");
    path.with_file_name(format!(".{file_name}.ember-save.tmp"))
}

#[derive(Debug)]
pub enum DocumentIoError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Invalid(String),
}

impl std::fmt::Display for DocumentIoError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "document I/O failed: {error}"),
            Self::Json(error) => write!(formatter, "document JSON failed: {error}"),
            Self::Invalid(error) => formatter.write_str(error),
        }
    }
}

impl std::error::Error for DocumentIoError {}

impl From<std::io::Error> for DocumentIoError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for DocumentIoError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn typed_payload_round_trips() {
        let envelope = DocumentEnvelope::from_typed(
            1,
            StableId::new("document", "typed").unwrap(),
            DocumentKind::DataTable,
            0,
            &vec![1_u32, 2, 3],
        )
        .unwrap();
        let decoded: Vec<u32> = envelope.decode().unwrap();
        assert_eq!(decoded, vec![1, 2, 3]);
    }

    #[test]
    fn migration_advances_version_and_revision() {
        fn migrate(mut value: Value) -> Result<Value, String> {
            value["new"] = json!(true);
            Ok(value)
        }
        let mut registry = MigrationRegistry::default();
        registry.register(DocumentKind::Custom("test".into()), 1, migrate);
        let envelope = DocumentEnvelope {
            format_version: 1,
            id: StableId::new("document", "migration").unwrap(),
            kind: DocumentKind::Custom("test".into()),
            revision: 4,
            payload: json!({"old": true}),
        };
        let migrated = registry.migrate(envelope, 2).unwrap();
        assert_eq!(migrated.format_version, 2);
        assert_eq!(migrated.revision, 5);
        assert_eq!(migrated.payload["new"], json!(true));
    }
}
