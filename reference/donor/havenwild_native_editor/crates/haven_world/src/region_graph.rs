use haven_core::{ProjectSceneId, RegionNodeId, SceneBiome, SceneId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionNodeKind {
    Hub,
    Connector,
    Farm,
    Forage,
    CaveEntrance,
    Cave,
    FutureHarbor,
    IslandHarbor,
}

impl RegionNodeKind {
    pub fn code(self) -> &'static str {
        match self {
            RegionNodeKind::Hub => "hub",
            RegionNodeKind::Connector => "connector",
            RegionNodeKind::Farm => "farm",
            RegionNodeKind::Forage => "forage",
            RegionNodeKind::CaveEntrance => "cave_entrance",
            RegionNodeKind::Cave => "cave",
            RegionNodeKind::FutureHarbor => "future_harbor",
            RegionNodeKind::IslandHarbor => "island_harbor",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionLinkKind {
    Road,
    FieldPath,
    Trail,
    CaveRoute,
    FutureRoute,
    SeaRoute,
}

impl RegionLinkKind {
    pub fn code(self) -> &'static str {
        match self {
            RegionLinkKind::Road => "road",
            RegionLinkKind::FieldPath => "field_path",
            RegionLinkKind::Trail => "trail",
            RegionLinkKind::CaveRoute => "cave_route",
            RegionLinkKind::FutureRoute => "future_route",
            RegionLinkKind::SeaRoute => "sea_route",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionPoint {
    pub x: f32,
    pub y: f32,
}

impl RegionPoint {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn is_normalized(self) -> bool {
        (0.0..=1.0).contains(&self.x) && (0.0..=1.0).contains(&self.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegionLandmass {
    pub center: RegionPoint,
    pub radius: RegionPoint,
    pub shore_noise: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegionNode {
    pub id: RegionNodeId,
    pub label: String,
    pub scene_id: Option<ProjectSceneId>,
    pub kind: RegionNodeKind,
    pub biome: SceneBiome,
    pub position: RegionPoint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegionLink {
    pub from: RegionNodeId,
    pub to: RegionNodeId,
    pub kind: RegionLinkKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IslandRegionGraph {
    pub id: &'static str,
    pub display_name: &'static str,
    pub role: &'static str,
    pub landmass: RegionLandmass,
    pub nodes: Vec<RegionNode>,
    pub links: Vec<RegionLink>,
}

impl IslandRegionGraph {
    pub fn node(&self, id: &RegionNodeId) -> Option<&RegionNode> {
        self.nodes.iter().find(|node| &node.id == id)
    }

    pub fn node_by_code(&self, id: &str) -> Option<&RegionNode> {
        self.nodes.iter().find(|node| node.id.as_str() == id)
    }
}

pub fn starter_island_region_graph() -> IslandRegionGraph {
    IslandRegionGraph {
        id: "starter_island_region",
        display_name: "Starter Island Region",
        role: "Island-scale landmass and scene placement graph. Scene nodes point to local 96x64 playable SceneMaps.",
        landmass: RegionLandmass {
            center: RegionPoint::new(0.5, 0.52),
            radius: RegionPoint::new(0.39, 0.32),
            shore_noise: 0.12,
        },
        nodes: vec![
            RegionNode {
                id: RegionNodeId::new("north_road"),
                label: "Road/Town".to_string(),
                scene_id: Some(ProjectSceneId::from(SceneId::NorthRoad)),
                kind: RegionNodeKind::Connector,
                biome: SceneBiome::Highlands,
                position: RegionPoint::new(0.18, 0.51),
            },
            RegionNode {
                id: RegionNodeId::new("farmstead"),
                label: "Estate".to_string(),
                scene_id: Some(ProjectSceneId::from(SceneId::Farmstead)),
                kind: RegionNodeKind::Hub,
                biome: SceneBiome::Temperate,
                position: RegionPoint::new(0.48, 0.5),
            },
            RegionNode {
                id: RegionNodeId::new("south_field"),
                label: "Fields".to_string(),
                scene_id: Some(ProjectSceneId::from(SceneId::SouthField)),
                kind: RegionNodeKind::Farm,
                biome: SceneBiome::Coastal,
                position: RegionPoint::new(0.4, 0.75),
            },
            RegionNode {
                id: RegionNodeId::new("east_woods"),
                label: "Woods".to_string(),
                scene_id: Some(ProjectSceneId::from(SceneId::EastWoods)),
                kind: RegionNodeKind::Forage,
                biome: SceneBiome::Temperate,
                position: RegionPoint::new(0.77, 0.52),
            },
            RegionNode {
                id: RegionNodeId::new("cave_mouth"),
                label: "Cave".to_string(),
                scene_id: Some(ProjectSceneId::from(SceneId::CaveMouth)),
                kind: RegionNodeKind::CaveEntrance,
                biome: SceneBiome::Highlands,
                position: RegionPoint::new(0.76, 0.36),
            },
            RegionNode {
                id: RegionNodeId::new("cave_depths"),
                label: "Depths".to_string(),
                scene_id: Some(ProjectSceneId::from(SceneId::CaveDepths)),
                kind: RegionNodeKind::Cave,
                biome: SceneBiome::Cave,
                position: RegionPoint::new(0.86, 0.29),
            },
            RegionNode {
                id: RegionNodeId::new("future_harbor"),
                label: "Future Harbor".to_string(),
                scene_id: None,
                kind: RegionNodeKind::FutureHarbor,
                biome: SceneBiome::Coastal,
                position: RegionPoint::new(0.56, 0.86),
            },
        ],
        links: vec![
            RegionLink {
                from: RegionNodeId::new("north_road"),
                to: RegionNodeId::new("farmstead"),
                kind: RegionLinkKind::Road,
            },
            RegionLink {
                from: RegionNodeId::new("farmstead"),
                to: RegionNodeId::new("south_field"),
                kind: RegionLinkKind::FieldPath,
            },
            RegionLink {
                from: RegionNodeId::new("farmstead"),
                to: RegionNodeId::new("east_woods"),
                kind: RegionLinkKind::Road,
            },
            RegionLink {
                from: RegionNodeId::new("east_woods"),
                to: RegionNodeId::new("cave_mouth"),
                kind: RegionLinkKind::Trail,
            },
            RegionLink {
                from: RegionNodeId::new("cave_mouth"),
                to: RegionNodeId::new("cave_depths"),
                kind: RegionLinkKind::CaveRoute,
            },
            RegionLink {
                from: RegionNodeId::new("farmstead"),
                to: RegionNodeId::new("future_harbor"),
                kind: RegionLinkKind::FutureRoute,
            },
        ],
    }
}

/// Connects a generated landmass to the main harbor in the editor-facing region graph.
/// The graph remains a lightweight travel overview: each generated island owns an
/// editable harbor scene, and the main harbor is the single embarkation hub.
pub fn connect_generated_island_harbor(
    graph: &mut IslandRegionGraph,
    landmass_id: i32,
    landmass_name: &str,
    harbor_scene_id: ProjectSceneId,
    position: RegionPoint,
) {
    let main_harbor_id = RegionNodeId::new("future_harbor");
    if landmass_id == 0 {
        if let Some(node) = graph
            .nodes
            .iter_mut()
            .find(|node| node.id == main_harbor_id)
        {
            node.label = "Main Harbor".to_string();
            node.scene_id = Some(harbor_scene_id);
            node.kind = RegionNodeKind::FutureHarbor;
            node.position = position;
        }
        return;
    }

    let node_id = RegionNodeId::new(format!("island_harbor_{landmass_id}"));
    if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == node_id) {
        node.label = format!("{} Harbor", landmass_name);
        node.scene_id = Some(harbor_scene_id);
        node.kind = RegionNodeKind::IslandHarbor;
        node.biome = SceneBiome::Coastal;
        node.position = position;
    } else {
        graph.nodes.push(RegionNode {
            id: node_id.clone(),
            label: format!("{} Harbor", landmass_name),
            scene_id: Some(harbor_scene_id),
            kind: RegionNodeKind::IslandHarbor,
            biome: SceneBiome::Coastal,
            position,
        });
    }

    if !graph.links.iter().any(|link| {
        link.from == main_harbor_id && link.to == node_id && link.kind == RegionLinkKind::SeaRoute
    }) {
        graph.links.push(RegionLink {
            from: main_harbor_id,
            to: node_id,
            kind: RegionLinkKind::SeaRoute,
        });
    }
}
