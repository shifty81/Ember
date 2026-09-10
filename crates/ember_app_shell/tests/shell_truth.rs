use ember_app_shell::EmberShell;
use ember_core::StableId;
use ember_documents::{DocumentEnvelope, DocumentKind, DocumentStore};
use serde_json::json;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn id(value: &str) -> StableId {
    StableId::new("document", value).expect("valid document id")
}

fn document(document_id: StableId, value: i32) -> DocumentEnvelope {
    DocumentEnvelope {
        format_version: 1,
        id: document_id,
        kind: DocumentKind::Scene,
        revision: 0,
        payload: json!({"value": value}),
    }
}

fn unique_path(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "ember-shell-{label}-{}-{nonce}.ember.json",
        std::process::id()
    ))
}

#[test]
fn path_backed_save_is_real_and_clears_dirty_state() {
    let path = unique_path("save");
    let document_id = id("save-truth");
    let mut shell = EmberShell::default();

    shell.insert_document(
        document(document_id.clone(), 42),
        "Save Truth",
        Some(path.clone()),
    );
    assert!(
        shell.layout.active_workspace.is_some(),
        "opening a Scene should activate a compatible workspace"
    );
    assert!(shell.mark_dirty(&document_id, true));
    shell
        .save_active_document()
        .expect("path-backed save succeeds");

    let tab = shell
        .tabs
        .iter()
        .find(|tab| tab.document_id == document_id)
        .expect("document tab exists");
    assert!(!tab.dirty, "successful save clears dirty state");
    assert_eq!(shell.document_path(&document_id), Some(path.as_path()));

    let loaded = DocumentStore::load(&path).expect("saved document reloads");
    assert_eq!(loaded.id, document_id);
    assert_eq!(loaded.payload["value"], json!(42));

    let _ = std::fs::remove_file(path);
}

#[test]
fn dirty_close_refuses_data_loss_without_explicit_discard() {
    let document_id = id("dirty-close");
    let mut shell = EmberShell::default();
    shell.insert_document(document(document_id.clone(), 1), "Dirty", None);
    assert!(shell.mark_dirty(&document_id, true));

    let error = shell
        .close_document(&document_id, false)
        .expect_err("dirty close must be refused");
    assert!(error.contains("unsaved changes"));
    assert!(shell.tabs.iter().any(|tab| tab.document_id == document_id));

    assert!(shell
        .close_document(&document_id, true)
        .expect("explicit discard closes"));
    assert!(!shell.tabs.iter().any(|tab| tab.document_id == document_id));
}

#[test]
fn save_all_only_counts_dirty_documents_with_real_paths() {
    let path = unique_path("save-all");
    let path_id = id("save-all-path");
    let memory_id = id("save-all-memory");
    let mut shell = EmberShell::default();

    shell.insert_document(
        document(path_id.clone(), 7),
        "Path Backed",
        Some(path.clone()),
    );
    shell.insert_document(document(memory_id.clone(), 8), "Memory Only", None);
    assert!(shell.mark_dirty(&path_id, true));
    assert!(shell.mark_dirty(&memory_id, true));

    assert_eq!(shell.save_all_documents().expect("save-all succeeds"), 1);

    let path_tab = shell
        .tabs
        .iter()
        .find(|tab| tab.document_id == path_id)
        .expect("path tab");
    let memory_tab = shell
        .tabs
        .iter()
        .find(|tab| tab.document_id == memory_id)
        .expect("memory tab");

    assert!(!path_tab.dirty);
    assert!(
        memory_tab.dirty,
        "document without a save path must not be falsely reported saved"
    );
    assert!(path.is_file());

    let _ = std::fs::remove_file(path);
}
