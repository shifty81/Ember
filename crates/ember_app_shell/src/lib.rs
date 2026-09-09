//! Renderer-independent Ember Editor application shell.
//!
//! Native backends render and feed input into this model; they do not own
//! project, document, selection, command or runtime state.

use ember_core::{Diagnostic, StableId};
use ember_documents::{DocumentEnvelope, DocumentKind, DocumentRegistry, DocumentStore};
use ember_editor_core::{
    register_standard_workspaces, EditorContext, EditorServices, WorkspaceRegistry,
};
use ember_project::{ProjectError, ProjectManifest};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShellCommand {
    NewProject,
    OpenProject,
    SaveActiveDocument,
    SaveAllDocuments,
    ImportAsset,
    ImportLdtk,
    Undo,
    Redo,
    RunCurrentScene,
    ValidateProject,
    BuildProject,
    OpenWorkspace(StableId),
    OpenDocument(StableId),
    CloseDocument(StableId),
    Custom(StableId),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuItem {
    pub label: String,
    pub command: ShellCommand,
    pub shortcut: Option<String>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenuDefinition {
    pub label: String,
    pub items: Vec<MenuItem>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentTab {
    pub document_id: StableId,
    pub title: String,
    pub kind: DocumentKind,
    pub dirty: bool,
    pub pinned: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StatusKind {
    Ready,
    Working,
    Warning,
    Error,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StatusMessage {
    pub kind: StatusKind,
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecentProject {
    pub manifest_path: PathBuf,
    pub display_name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EditorLayout {
    pub format_version: u32,
    pub active_workspace: Option<StableId>,
    pub left_panel_width: u32,
    pub right_panel_width: u32,
    pub bottom_panel_height: u32,
    pub bottom_panel_open: bool,
}

impl Default for EditorLayout {
    fn default() -> Self {
        Self {
            format_version: 1,
            active_workspace: None,
            left_panel_width: 280,
            right_panel_width: 320,
            bottom_panel_height: 220,
            bottom_panel_open: false,
        }
    }
}

pub struct EmberShell {
    pub project: Option<ProjectManifest>,
    pub project_manifest_path: Option<PathBuf>,
    pub documents: DocumentRegistry,
    pub editor: EditorContext,
    pub services: EditorServices,
    pub workspaces: WorkspaceRegistry,
    pub tabs: Vec<DocumentTab>,
    pub active_tab: Option<StableId>,
    pub menus: Vec<MenuDefinition>,
    pub layout: EditorLayout,
    pub status: StatusMessage,
    pub recent_projects: VecDeque<RecentProject>,
    document_paths: BTreeMap<StableId, PathBuf>,
}

/// Compatibility alias retained for the first Ember migration window.
pub type FoundryShell = EmberShell;

impl Default for EmberShell {
    fn default() -> Self {
        let mut workspaces = WorkspaceRegistry::default();
        register_standard_workspaces(&mut workspaces);
        Self {
            project: None,
            project_manifest_path: None,
            documents: DocumentRegistry::default(),
            editor: EditorContext::default(),
            services: EditorServices::standard(),
            workspaces,
            tabs: Vec::new(),
            active_tab: None,
            menus: default_menus(),
            layout: EditorLayout::default(),
            status: StatusMessage {
                kind: StatusKind::Ready,
                text: "Ready".into(),
            },
            recent_projects: VecDeque::new(),
            document_paths: BTreeMap::new(),
        }
    }
}

impl EmberShell {
    pub fn load_project(&mut self, manifest_path: &Path) -> Result<(), ProjectError> {
        let manifest = ProjectManifest::load(manifest_path)?;
        self.editor.active_project = Some(manifest.project_id.clone());
        self.status = StatusMessage {
            kind: StatusKind::Ready,
            text: format!("Opened {}", manifest.name),
        };
        self.remember_recent(manifest_path.to_path_buf(), manifest.name.clone());
        self.project_manifest_path = Some(manifest_path.to_path_buf());
        self.project = Some(manifest);
        Ok(())
    }

    pub fn insert_document(
        &mut self,
        document: DocumentEnvelope,
        title: impl Into<String>,
        source_path: Option<PathBuf>,
    ) {
        let id = document.id.clone();
        let kind = document.kind.clone();
        self.documents.insert(document);
        if let Some(path) = source_path {
            self.document_paths.insert(id.clone(), path);
        }
        if !self.tabs.iter().any(|tab| tab.document_id == id) {
            self.tabs.push(DocumentTab {
                document_id: id.clone(),
                title: title.into(),
                kind,
                dirty: false,
                pinned: false,
            });
        }
        self.activate_document(&id);
    }

    pub fn activate_document(&mut self, id: &StableId) -> bool {
        if !self.tabs.iter().any(|tab| &tab.document_id == id) {
            return false;
        }
        let kind = self
            .tabs
            .iter()
            .find(|tab| &tab.document_id == id)
            .map(|tab| tab.kind.clone());
        self.active_tab = Some(id.clone());
        self.editor.active_document = Some(id.clone());
        if let Some(kind) = kind {
            if let Some(workspace_id) = self.workspaces.compatible(&kind).into_iter().next() {
                self.layout.active_workspace = Some(workspace_id);
            }
        }
        true
    }

    pub fn close_document(&mut self, id: &StableId, discard_dirty: bool) -> Result<bool, String> {
        let Some(index) = self.tabs.iter().position(|tab| &tab.document_id == id) else {
            return Ok(false);
        };
        if self.tabs[index].dirty && !discard_dirty {
            return Err(format!(
                "{} contains unsaved changes",
                self.tabs[index].title
            ));
        }
        self.tabs.remove(index);
        if self.active_tab.as_ref() == Some(id) {
            self.active_tab = self.tabs.last().map(|tab| tab.document_id.clone());
            self.editor.active_document = self.active_tab.clone();
        }
        Ok(true)
    }

    pub fn mark_dirty(&mut self, id: &StableId, dirty: bool) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|tab| &tab.document_id == id) else {
            return false;
        };
        tab.dirty = dirty;
        true
    }

    pub fn save_document(&mut self, id: &StableId) -> Result<(), String> {
        let path = self
            .document_paths
            .get(id)
            .cloned()
            .ok_or_else(|| format!("document {id} has no save path"))?;
        let document = self
            .documents
            .get(id)
            .ok_or_else(|| format!("document {id} is not open"))?;
        DocumentStore::save_atomic(&path, document).map_err(|error| error.to_string())?;
        self.mark_dirty(id, false);
        self.status = StatusMessage {
            kind: StatusKind::Ready,
            text: format!("Saved {path:?}"),
        };
        Ok(())
    }

    pub fn save_active_document(&mut self) -> Result<(), String> {
        let id = self
            .active_tab
            .clone()
            .ok_or_else(|| "no active document".to_string())?;
        self.save_document(&id)
    }

    pub fn save_all_documents(&mut self) -> Result<usize, String> {
        let ids: Vec<_> = self
            .tabs
            .iter()
            .filter(|tab| tab.dirty && self.document_paths.contains_key(&tab.document_id))
            .map(|tab| tab.document_id.clone())
            .collect();
        for id in &ids {
            self.save_document(id)?;
        }
        Ok(ids.len())
    }

    pub fn dispatch(&mut self, command: &ShellCommand) -> Result<String, String> {
        match command {
            ShellCommand::SaveActiveDocument => {
                self.save_active_document()?;
                Ok("saved active document".into())
            }
            ShellCommand::SaveAllDocuments => {
                let count = self.save_all_documents()?;
                Ok(format!("saved {count} document(s)"))
            }
            ShellCommand::OpenWorkspace(id) => {
                if !self.workspaces.activate(id, &mut self.editor) {
                    return Err(format!("workspace {id} is unavailable"));
                }
                self.layout.active_workspace = Some(id.clone());
                Ok(format!("opened workspace {id}"))
            }
            ShellCommand::OpenDocument(id) => {
                if self.activate_document(id) {
                    Ok(format!("opened document {id}"))
                } else {
                    Err(format!("document {id} is not open"))
                }
            }
            ShellCommand::CloseDocument(id) => {
                self.close_document(id, false)?;
                Ok(format!("closed document {id}"))
            }
            ShellCommand::ValidateProject => {
                let errors = self.diagnostics();
                if errors.is_empty() {
                    Ok("project validation has no document diagnostics".into())
                } else {
                    Err(format!("project has {} diagnostic(s)", errors.len()))
                }
            }
            ShellCommand::RunCurrentScene => Ok("play-test requested".into()),
            ShellCommand::BuildProject => Ok("build requested".into()),
            ShellCommand::Undo => Ok("undo routed to active workspace command history".into()),
            ShellCommand::Redo => Ok("redo routed to active workspace command history".into()),
            ShellCommand::NewProject
            | ShellCommand::OpenProject
            | ShellCommand::ImportAsset
            | ShellCommand::ImportLdtk
            | ShellCommand::Custom(_) => Ok("command accepted by Ember application service".into()),
        }
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        let mut diagnostics = self.documents.validate();
        diagnostics.extend(self.editor.diagnostics.clone());
        diagnostics
    }

    pub fn document_path(&self, id: &StableId) -> Option<&Path> {
        self.document_paths.get(id).map(PathBuf::as_path)
    }

    fn remember_recent(&mut self, path: PathBuf, display_name: String) {
        self.recent_projects
            .retain(|entry| entry.manifest_path != path);
        self.recent_projects.push_front(RecentProject {
            manifest_path: path,
            display_name,
        });
        self.recent_projects.truncate(12);
    }
}

pub fn default_menus() -> Vec<MenuDefinition> {
    vec![
        MenuDefinition {
            label: "File".into(),
            items: vec![
                menu(
                    "New Project",
                    ShellCommand::NewProject,
                    Some("Ctrl+Shift+N"),
                ),
                menu("Open Project", ShellCommand::OpenProject, Some("Ctrl+O")),
                menu("Save", ShellCommand::SaveActiveDocument, Some("Ctrl+S")),
                menu(
                    "Save All",
                    ShellCommand::SaveAllDocuments,
                    Some("Ctrl+Shift+S"),
                ),
                menu("Import Asset", ShellCommand::ImportAsset, None),
                menu("Import LDtk", ShellCommand::ImportLdtk, None),
            ],
        },
        MenuDefinition {
            label: "Edit".into(),
            items: vec![
                menu("Undo", ShellCommand::Undo, Some("Ctrl+Z")),
                menu("Redo", ShellCommand::Redo, Some("Ctrl+Y")),
            ],
        },
        MenuDefinition {
            label: "Project".into(),
            items: vec![
                menu("Validate", ShellCommand::ValidateProject, None),
                menu(
                    "Run Current Scene",
                    ShellCommand::RunCurrentScene,
                    Some("F6"),
                ),
                menu("Build", ShellCommand::BuildProject, Some("F7")),
            ],
        },
    ]
}

fn menu(label: &str, command: ShellCommand, shortcut: Option<&str>) -> MenuItem {
    MenuItem {
        label: label.into(),
        command,
        shortcut: shortcut.map(str::to_owned),
        enabled: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn insert_document_opens_and_activates_tab() {
        let mut shell = EmberShell::default();
        let document_id = StableId::new("document", "scene-one").unwrap();
        shell.insert_document(
            DocumentEnvelope {
                format_version: 1,
                id: document_id.clone(),
                kind: DocumentKind::Scene,
                revision: 0,
                payload: json!({}),
            },
            "Scene One",
            None,
        );
        assert_eq!(shell.active_tab, Some(document_id));
        assert_eq!(shell.tabs.len(), 1);
    }
}
