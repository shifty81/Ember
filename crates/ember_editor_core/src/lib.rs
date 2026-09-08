use ember_core::{Diagnostic, StableId};
use ember_documents::DocumentKind;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct EditorContext {
    pub active_project: Option<StableId>,
    pub active_document: Option<StableId>,
    pub diagnostics: Vec<Diagnostic>,
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
        registry.register(Box::new(StandardWorkspace::new(id, title, supported)));
    }
}
