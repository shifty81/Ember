use ember_core::{Diagnostic, StableId};
use ember_documents::DocumentKind;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default)]
pub struct EditorContext {
    pub active_project: Option<StableId>,
    pub active_document: Option<StableId>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorCommandDescriptor {
    pub id: StableId,
    pub title: String,
    pub shortcut: Option<String>,
    pub mutates: bool,
}

#[derive(Default)]
pub struct CommandRegistry {
    commands: BTreeMap<StableId, EditorCommandDescriptor>,
}

impl CommandRegistry {
    pub fn register(
        &mut self,
        command: EditorCommandDescriptor,
    ) -> Option<EditorCommandDescriptor> {
        self.commands.insert(command.id.clone(), command)
    }

    pub fn get(&self, id: &StableId) -> Option<&EditorCommandDescriptor> {
        self.commands.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &EditorCommandDescriptor> {
        self.commands.values()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectionService {
    primary: Option<StableId>,
    selected: BTreeSet<StableId>,
}

impl SelectionService {
    pub fn set_primary(&mut self, id: StableId) {
        self.selected.insert(id.clone());
        self.primary = Some(id);
    }

    pub fn set_many(&mut self, ids: impl IntoIterator<Item = StableId>) {
        self.selected = ids.into_iter().collect();
        self.primary = self.selected.iter().next().cloned();
    }

    pub fn clear(&mut self) {
        self.primary = None;
        self.selected.clear();
    }

    pub fn primary(&self) -> Option<&StableId> {
        self.primary.as_ref()
    }

    pub fn selected(&self) -> impl Iterator<Item = &StableId> {
        self.selected.iter()
    }
}

#[derive(Default)]
pub struct EditorServices {
    pub commands: CommandRegistry,
    pub selection: SelectionService,
}

impl EditorServices {
    pub fn standard() -> Self {
        let mut services = Self::default();
        for (value, title, shortcut, mutates) in [
            ("save", "Save", Some("Ctrl+S"), true),
            ("save_all", "Save All", Some("Ctrl+Shift+S"), true),
            ("undo", "Undo", Some("Ctrl+Z"), true),
            ("redo", "Redo", Some("Ctrl+Y"), true),
            ("validate", "Validate Project", None, false),
            ("play", "Play Current Scene", Some("F6"), true),
            ("build", "Build Project", Some("F7"), false),
        ] {
            let id = StableId::new("command", value).expect("static command id");
            let _ = services.commands.register(EditorCommandDescriptor {
                id,
                title: title.into(),
                shortcut: shortcut.map(str::to_owned),
                mutates,
            });
        }
        services
    }
}

pub trait EditorWorkspace: Send {
    fn id(&self) -> StableId;
    fn title(&self) -> &str;
    fn supports(&self, kind: &DocumentKind) -> bool;
    fn activate(&mut self, _context: &mut EditorContext) {}
}

#[derive(Default)]
pub struct WorkspaceRegistry {
    workspaces: BTreeMap<StableId, Box<dyn EditorWorkspace>>,
}

impl WorkspaceRegistry {
    pub fn register(
        &mut self,
        workspace: Box<dyn EditorWorkspace>,
    ) -> Option<Box<dyn EditorWorkspace>> {
        self.workspaces.insert(workspace.id(), workspace)
    }

    pub fn ids(&self) -> impl Iterator<Item = &StableId> {
        self.workspaces.keys()
    }

    pub fn compatible(&self, kind: &DocumentKind) -> Vec<StableId> {
        self.workspaces
            .iter()
            .filter(|(_, workspace)| workspace.supports(kind))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub fn activate(&mut self, id: &StableId, context: &mut EditorContext) -> bool {
        let Some(workspace) = self.workspaces.get_mut(id) else {
            return false;
        };
        workspace.activate(context);
        true
    }

    pub fn title(&self, id: &StableId) -> Option<&str> {
        self.workspaces.get(id).map(|workspace| workspace.title())
    }
}

#[derive(Clone, Debug)]
pub struct StandardWorkspace {
    id: StableId,
    title: String,
    supported: Vec<DocumentKind>,
}

impl StandardWorkspace {
    pub fn new(
        id: StableId,
        title: impl Into<String>,
        supported: impl IntoIterator<Item = DocumentKind>,
    ) -> Self {
        Self {
            id,
            title: title.into(),
            supported: supported.into_iter().collect(),
        }
    }
}

impl EditorWorkspace for StandardWorkspace {
    fn id(&self) -> StableId {
        self.id.clone()
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn supports(&self, kind: &DocumentKind) -> bool {
        self.supported.iter().any(|supported| supported == kind)
    }
}

pub fn register_standard_workspaces(registry: &mut WorkspaceRegistry) {
    let definitions = [
        ("project", "Project", vec![DocumentKind::BuildProfile]),
        (
            "world",
            "World",
            vec![DocumentKind::World, DocumentKind::WorldGenerationGraph],
        ),
        (
            "level",
            "Level",
            vec![
                DocumentKind::Level,
                DocumentKind::Scene,
                DocumentKind::Prefab,
            ],
        ),
        (
            "pixel-animation",
            "Pixel & Animation",
            vec![DocumentKind::PixelImage, DocumentKind::Animation],
        ),
        (
            "nodes",
            "Nodes",
            vec![
                DocumentKind::BehaviorGraph,
                DocumentKind::WorldGenerationGraph,
            ],
        ),
        ("gui", "GUI", vec![DocumentKind::GuiLayout]),
        ("audio", "Audio", vec![DocumentKind::AudioEvent]),
        (
            "data",
            "Data",
            vec![DocumentKind::DataTable, DocumentKind::Entity],
        ),
    ];
    for (slug, title, supported) in definitions {
        let id = StableId::new("workspace", slug).expect("static workspace id");
        let _ = registry.register(Box::new(StandardWorkspace::new(id, title, supported)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_primary_is_always_selected() {
        let mut selection = SelectionService::default();
        let id = StableId::new("entity", "one").unwrap();
        selection.set_primary(id.clone());
        assert_eq!(selection.primary(), Some(&id));
        assert_eq!(selection.selected().count(), 1);
    }
}
