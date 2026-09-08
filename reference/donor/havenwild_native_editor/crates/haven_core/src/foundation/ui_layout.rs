#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl UiAnchor {
    pub fn code(self) -> &'static str {
        match self {
            UiAnchor::TopLeft => "top_left",
            UiAnchor::TopRight => "top_right",
            UiAnchor::BottomLeft => "bottom_left",
            UiAnchor::BottomRight => "bottom_right",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "top_left" => Some(UiAnchor::TopLeft),
            "top_right" => Some(UiAnchor::TopRight),
            "bottom_left" => Some(UiAnchor::BottomLeft),
            "bottom_right" => Some(UiAnchor::BottomRight),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiPanelId {
    Hud,
    Editor,
    Inspector,
    WorldGraph,
    Validation,
}

impl UiPanelId {
    pub const ALL: [UiPanelId; 5] = [
        UiPanelId::Hud,
        UiPanelId::Editor,
        UiPanelId::Inspector,
        UiPanelId::WorldGraph,
        UiPanelId::Validation,
    ];

    pub fn code(self) -> &'static str {
        match self {
            UiPanelId::Hud => "hud",
            UiPanelId::Editor => "editor",
            UiPanelId::Inspector => "inspector",
            UiPanelId::WorldGraph => "world_graph",
            UiPanelId::Validation => "validation",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            UiPanelId::Hud => "HUD",
            UiPanelId::Editor => "Editor Overlay",
            UiPanelId::Inspector => "Inspector",
            UiPanelId::WorldGraph => "World Graph",
            UiPanelId::Validation => "Validation",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "hud" => Some(UiPanelId::Hud),
            "editor" => Some(UiPanelId::Editor),
            "inspector" => Some(UiPanelId::Inspector),
            "world_graph" => Some(UiPanelId::WorldGraph),
            "validation" => Some(UiPanelId::Validation),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct UiPanelLayout {
    pub anchor: UiAnchor,
    pub grid_x: i32,
    pub grid_y: i32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Debug)]
pub struct UiLayoutState {
    pub hud: UiPanelLayout,
    pub editor: UiPanelLayout,
    pub inspector: UiPanelLayout,
    pub world_graph: UiPanelLayout,
    pub validation: UiPanelLayout,
}

impl Default for UiLayoutState {
    fn default() -> Self {
        Self {
            hud: UiPanelLayout {
                anchor: UiAnchor::TopLeft,
                grid_x: 1,
                grid_y: 1,
                width: 430.0,
                height: 122.0,
            },
            editor: UiPanelLayout {
                anchor: UiAnchor::TopRight,
                grid_x: 1,
                grid_y: 1,
                width: 720.0,
                height: 520.0,
            },
            inspector: UiPanelLayout {
                anchor: UiAnchor::TopRight,
                grid_x: 1,
                grid_y: 24,
                width: 370.0,
                height: 190.0,
            },
            world_graph: UiPanelLayout {
                anchor: UiAnchor::BottomLeft,
                grid_x: 1,
                grid_y: 1,
                width: 450.0,
                height: 276.0,
            },
            validation: UiPanelLayout {
                anchor: UiAnchor::TopLeft,
                grid_x: 1,
                grid_y: 10,
                width: 430.0,
                height: 132.0,
            },
        }
    }
}

impl UiLayoutState {
    pub fn panel(&self, panel: UiPanelId) -> UiPanelLayout {
        match panel {
            UiPanelId::Hud => self.hud,
            UiPanelId::Editor => self.editor,
            UiPanelId::Inspector => self.inspector,
            UiPanelId::WorldGraph => self.world_graph,
            UiPanelId::Validation => self.validation,
        }
    }

    pub fn panel_mut(&mut self, panel: UiPanelId) -> &mut UiPanelLayout {
        match panel {
            UiPanelId::Hud => &mut self.hud,
            UiPanelId::Editor => &mut self.editor,
            UiPanelId::Inspector => &mut self.inspector,
            UiPanelId::WorldGraph => &mut self.world_graph,
            UiPanelId::Validation => &mut self.validation,
        }
    }

    pub fn serialize_lines(&self) -> String {
        let mut out = String::from("tavern_layout 1\n");
        for panel in UiPanelId::ALL {
            let entry = self.panel(panel);
            out.push_str(&format!(
                "panel {} {} {} {} {} {}
",
                panel.code(),
                entry.anchor.code(),
                entry.grid_x,
                entry.grid_y,
                entry.width as i32,
                entry.height as i32
            ));
        }
        out
    }

    pub fn deserialize_lines(input: &str) -> Result<Self, String> {
        let mut lines = input.lines();
        let header = lines.next().ok_or("missing layout header")?;
        if header != "tavern_layout 1" {
            return Err(format!("unsupported layout header: {header}"));
        }
        let mut layout = UiLayoutState::default();
        for line in lines {
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() != 7 || parts[0] != "panel" {
                continue;
            }
            let panel = UiPanelId::from_code(parts[1]).ok_or("bad panel id")?;
            let anchor = UiAnchor::from_code(parts[2]).ok_or("bad panel anchor")?;
            let grid_x = parts[3].parse::<i32>().map_err(|_| "bad panel grid x")?;
            let grid_y = parts[4].parse::<i32>().map_err(|_| "bad panel grid y")?;
            let width = parts[5].parse::<f32>().map_err(|_| "bad panel width")?;
            let height = parts[6].parse::<f32>().map_err(|_| "bad panel height")?;
            *layout.panel_mut(panel) = UiPanelLayout {
                anchor,
                grid_x,
                grid_y,
                width,
                height,
            };
        }
        Ok(layout)
    }
}
