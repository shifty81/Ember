use haven_editor::{GridPos, GridRect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorViewportMode {
    RegionGraph,
    SceneRectangles,
    SceneBank,
    SceneMap,
    PixelStudio,
    AnimationStudio,
    CharacterStudio,
}

impl EditorViewportMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            EditorViewportMode::RegionGraph => "World Routes",
            EditorViewportMode::SceneRectangles => "World Editor",
            EditorViewportMode::SceneBank => "Scene Bank",
            EditorViewportMode::SceneMap => "Scene Editor",
            EditorViewportMode::PixelStudio => "Pixel Studio",
            EditorViewportMode::AnimationStudio => "Animation Studio",
            EditorViewportMode::CharacterStudio => "Character Studio",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SceneEditTool {
    Select,
    Paint,
    Rectangle,
    Fill,
    Replace,
    Eyedropper,
    Place,
    Erase,
    Pan,
}

impl SceneEditTool {
    pub(crate) fn label(self) -> &'static str {
        match self {
            SceneEditTool::Select => "Select",
            SceneEditTool::Paint => "Paint",
            SceneEditTool::Rectangle => "Rectangle",
            SceneEditTool::Fill => "Fill",
            SceneEditTool::Replace => "Replace",
            SceneEditTool::Eyedropper => "Pick",
            SceneEditTool::Place => "Place",
            SceneEditTool::Erase => "Erase",
            SceneEditTool::Pan => "Pan",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SceneDockTab {
    Tools,
    Assets,
    Intake,
    Inspector,
    Library,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FootprintEditTarget {
    Visual,
    Collision,
    Interaction,
}

impl FootprintEditTarget {
    pub(crate) fn label(self) -> &'static str {
        match self {
            FootprintEditTarget::Visual => "Visual",
            FootprintEditTarget::Collision => "Collision",
            FootprintEditTarget::Interaction => "Interaction",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditorTextFocus {
    None,
    SceneName,
    ObjectFilter,
    AssetFilter,
    AssetLibraryFilter,
    PixelLibraryFilter,
    CharacterAssetFilter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SceneNameEditMode {
    Create,
    Rename,
}

#[derive(Clone, Debug)]
pub(crate) struct SceneNameEditState {
    pub mode: SceneNameEditMode,
    pub buffer: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SceneLayerMode {
    Terrain,
    Objects,
    Zones,
    Transitions,
}

impl SceneLayerMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            SceneLayerMode::Terrain => "Terrain",
            SceneLayerMode::Objects => "Objects",
            SceneLayerMode::Zones => "Zones",
            SceneLayerMode::Transitions => "Transitions",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SceneLayerState {
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
}

impl Default for SceneLayerState {
    fn default() -> Self {
        Self {
            visible: true,
            locked: false,
            opacity: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SceneCanvasDragKind {
    Marquee,
    MoveSelection,
    Rectangle,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SceneCanvasDrag {
    pub kind: SceneCanvasDragKind,
    pub start: GridPos,
    pub current: GridPos,
    pub additive: bool,
}

impl SceneCanvasDrag {
    pub fn rect(self) -> GridRect {
        GridRect::from_points(self.start, self.current)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum WorldEditTool {
    #[default]
    Select,
    Paint,
    Rectangle,
    Fill,
    Replace,
    Eyedropper,
    Place,
    Erase,
    Pan,
}

impl WorldEditTool {
    pub(crate) const ALL: [WorldEditTool; 9] = [
        WorldEditTool::Select,
        WorldEditTool::Paint,
        WorldEditTool::Rectangle,
        WorldEditTool::Fill,
        WorldEditTool::Replace,
        WorldEditTool::Eyedropper,
        WorldEditTool::Place,
        WorldEditTool::Erase,
        WorldEditTool::Pan,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            WorldEditTool::Select => "Select",
            WorldEditTool::Paint => "Paint",
            WorldEditTool::Rectangle => "Rectangle",
            WorldEditTool::Fill => "Fill",
            WorldEditTool::Replace => "Replace",
            WorldEditTool::Eyedropper => "Pick",
            WorldEditTool::Place => "Place",
            WorldEditTool::Erase => "Erase",
            WorldEditTool::Pan => "Pan",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum WorldLayerMode {
    #[default]
    Terrain,
    Objects,
    Zones,
    StructuralLevels,
}

impl WorldLayerMode {
    pub(crate) const ALL: [WorldLayerMode; 4] = [
        WorldLayerMode::Terrain,
        WorldLayerMode::Objects,
        WorldLayerMode::Zones,
        WorldLayerMode::StructuralLevels,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            WorldLayerMode::Terrain => "Terrain",
            WorldLayerMode::Objects => "Objects/Stamps",
            WorldLayerMode::Zones => "Zones/Lots",
            WorldLayerMode::StructuralLevels => "Levels & Cliffs",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorldCanvasDragKind {
    Rectangle,
    Marquee,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WorldCanvasDrag {
    pub kind: WorldCanvasDragKind,
    pub start: GridPos,
    pub current: GridPos,
}

impl WorldCanvasDrag {
    pub(crate) fn rect(self) -> GridRect {
        GridRect::from_points(self.start, self.current)
    }
}
