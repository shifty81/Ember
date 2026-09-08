use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

pub(crate) const NATIVE_WORKSPACE_LAYOUT_PATH: &str =
    "WORKSPACE/editor/native_workspace_layout_v0_1.json";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WorkspaceResizeDrag {
    LeftPanel,
    RightPanel,
    BottomDock,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BottomDockTab {
    Console,
    #[default]
    Validation,
    Imports,
    Build,
    Tasks,
}

impl BottomDockTab {
    pub(crate) const ALL: [Self; 5] = [
        Self::Console,
        Self::Validation,
        Self::Imports,
        Self::Build,
        Self::Tasks,
    ];

    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Console => "Console",
            Self::Validation => "Validation",
            Self::Imports => "Imports",
            Self::Build => "Build",
            Self::Tasks => "Tasks",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct EditorWorkspaceShellState {
    pub schema: String,
    pub left_panel_visible: bool,
    pub right_panel_visible: bool,
    pub bottom_dock_open: bool,
    pub bottom_dock_tab: BottomDockTab,
    pub left_panel_width: f32,
    pub right_panel_width: f32,
    pub bottom_dock_height: f32,
}

impl Default for EditorWorkspaceShellState {
    fn default() -> Self {
        Self {
            schema: "havenwild.native_editor.workspace_layout.v0_1".to_string(),
            left_panel_visible: true,
            right_panel_visible: true,
            bottom_dock_open: false,
            bottom_dock_tab: BottomDockTab::Validation,
            left_panel_width: 248.0,
            right_panel_width: 360.0,
            bottom_dock_height: 176.0,
        }
    }
}

impl EditorWorkspaceShellState {
    pub(crate) fn load_default() -> Self {
        let path = Path::new(NATIVE_WORKSPACE_LAYOUT_PATH);
        let Ok(text) = fs::read_to_string(path) else {
            return Self::default();
        };
        serde_json::from_str::<Self>(&text)
            .map(Self::normalized)
            .unwrap_or_default()
    }

    pub(crate) fn save_default(&self) -> Result<(), String> {
        let path = Path::new(NATIVE_WORKSPACE_LAYOUT_PATH);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("could not create native editor workspace directory: {error}")
            })?;
        }
        let normalized = self.clone().normalized();
        let text = serde_json::to_string_pretty(&normalized)
            .map_err(|error| format!("could not serialize native editor workspace: {error}"))?;
        fs::write(path, format!("{text}\n"))
            .map_err(|error| format!("could not save native editor workspace: {error}"))
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn normalize_in_place(&mut self) {
        self.schema = "havenwild.native_editor.workspace_layout.v0_1".to_string();
        self.left_panel_width = self.left_panel_width.clamp(220.0, 360.0);
        self.right_panel_width = self.right_panel_width.clamp(300.0, 480.0);
        self.bottom_dock_height = self.bottom_dock_height.clamp(136.0, 320.0);
    }

    pub(crate) fn normalized(mut self) -> Self {
        self.normalize_in_place();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_keeps_panels_inside_supported_ranges() {
        let state = EditorWorkspaceShellState {
            left_panel_width: 12.0,
            right_panel_width: 900.0,
            bottom_dock_height: 12.0,
            ..EditorWorkspaceShellState::default()
        }
        .normalized();
        assert_eq!(state.left_panel_width, 220.0);
        assert_eq!(state.right_panel_width, 480.0);
        assert_eq!(state.bottom_dock_height, 136.0);
    }

    #[test]
    fn bottom_dock_tabs_have_stable_labels() {
        assert_eq!(BottomDockTab::ALL.len(), 5);
        assert_eq!(BottomDockTab::Validation.label(), "Validation");
    }
}
