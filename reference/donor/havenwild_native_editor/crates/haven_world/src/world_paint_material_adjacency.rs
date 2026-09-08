use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    load_world_paint_material_state_document, WorldPaintMaterialCell,
    WorldPaintMaterialStateDocument,
};

pub const WORLD_PAINT_MATERIAL_ADJACENCY_SCHEMA: &str =
    "havenwild.world_paint_material_adjacency.v0.1";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldPaintNeighborMask {
    pub north: bool,
    pub east: bool,
    pub south: bool,
    pub west: bool,
    pub north_east: bool,
    pub south_east: bool,
    pub south_west: bool,
    pub north_west: bool,
}

impl WorldPaintNeighborMask {
    pub fn cardinal_bits(self) -> u8 {
        (self.north as u8)
            | ((self.east as u8) << 1)
            | ((self.south as u8) << 2)
            | ((self.west as u8) << 3)
    }

    pub fn cardinal_count(self) -> u8 {
        self.north as u8 + self.east as u8 + self.south as u8 + self.west as u8
    }

    pub fn compact_label(self) -> String {
        format!(
            "N{} E{} S{} W{} / NE{} SE{} SW{} NW{}",
            bit(self.north),
            bit(self.east),
            bit(self.south),
            bit(self.west),
            bit(self.north_east),
            bit(self.south_east),
            bit(self.south_west),
            bit(self.north_west)
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldPaintMaterialAdjacencyReport {
    pub schema: String,
    pub scene_id: String,
    pub x: i32,
    pub y: i32,
    pub primary_family: String,
    pub primary_layer: String,
    pub neighbor_mask: WorldPaintNeighborMask,
    pub shoreline_candidate: bool,
    pub cave_edge_candidate: bool,
    pub paved_brick_edge_candidate: bool,
    pub wood_floor_edge_candidate: bool,
    pub protected_edge_candidate: bool,
    pub transition_role_hint: String,
    pub ready_for_autotile: bool,
    pub ready_for_debris: bool,
    pub status: String,
}

impl WorldPaintMaterialAdjacencyReport {
    pub fn empty(scene_id: impl Into<String>, x: i32, y: i32) -> Self {
        let scene_id = scene_id.into();
        Self {
            schema: WORLD_PAINT_MATERIAL_ADJACENCY_SCHEMA.to_string(),
            scene_id: scene_id.clone(),
            x,
            y,
            primary_family: "none".to_string(),
            primary_layer: "none".to_string(),
            neighbor_mask: WorldPaintNeighborMask::default(),
            shoreline_candidate: false,
            cave_edge_candidate: false,
            paved_brick_edge_candidate: false,
            wood_floor_edge_candidate: false,
            protected_edge_candidate: false,
            transition_role_hint: "none".to_string(),
            ready_for_autotile: false,
            ready_for_debris: false,
            status: format!("No material-state cell at {} {},{}", scene_id, x, y),
        }
    }

    pub fn status_line(&self) -> String {
        format!(
            "Adjacency {} {},{}: {} on {} -> {} | mask {} | autotile {}",
            self.scene_id,
            self.x,
            self.y,
            self.primary_family,
            self.primary_layer,
            self.transition_role_hint,
            self.neighbor_mask.compact_label(),
            if self.ready_for_autotile {
                "ready"
            } else {
                "not-ready"
            }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldPaintMaterialAdjacencySceneReport {
    pub schema: String,
    pub scene_id: String,
    pub cell_reports: usize,
    pub shoreline_candidates: usize,
    pub cave_edge_candidates: usize,
    pub paved_brick_edge_candidates: usize,
    pub wood_floor_edge_candidates: usize,
    pub ready_for_autotile: usize,
    pub status: String,
}

impl WorldPaintMaterialAdjacencySceneReport {
    pub fn status_line(&self) -> String {
        format!(
            "Adjacency scene {}: {} cell(s), shore {}, cave {}, brick {}, wood {}, autotile-ready {}",
            self.scene_id,
            self.cell_reports,
            self.shoreline_candidates,
            self.cave_edge_candidates,
            self.paved_brick_edge_candidates,
            self.wood_floor_edge_candidates,
            self.ready_for_autotile
        )
    }
}

pub fn resolve_world_paint_material_adjacency(
    doc: &WorldPaintMaterialStateDocument,
    scene_id: &str,
    x: i32,
    y: i32,
) -> WorldPaintMaterialAdjacencyReport {
    let Some(scene) = doc.scenes.iter().find(|scene| scene.scene_id == scene_id) else {
        return WorldPaintMaterialAdjacencyReport::empty(scene_id.to_string(), x, y);
    };
    let Some(primary) = primary_cell(scene.cells.iter().filter(|cell| cell.x == x && cell.y == y))
    else {
        return WorldPaintMaterialAdjacencyReport::empty(scene_id.to_string(), x, y);
    };

    let family = primary.family.as_str();
    let layer = primary.layer.as_str();
    let mask = WorldPaintNeighborMask {
        north: has_same_family_neighbor(&scene.cells, family, x, y - 1),
        east: has_same_family_neighbor(&scene.cells, family, x + 1, y),
        south: has_same_family_neighbor(&scene.cells, family, x, y + 1),
        west: has_same_family_neighbor(&scene.cells, family, x - 1, y),
        north_east: has_same_family_neighbor(&scene.cells, family, x + 1, y - 1),
        south_east: has_same_family_neighbor(&scene.cells, family, x + 1, y + 1),
        south_west: has_same_family_neighbor(&scene.cells, family, x - 1, y + 1),
        north_west: has_same_family_neighbor(&scene.cells, family, x - 1, y - 1),
    };

    let touches_other_family = has_other_family_neighbor(&scene.cells, family, x, y);
    let touches_water = has_family_neighbor(&scene.cells, "water", x, y) || family == "water";
    let touches_sand = has_family_neighbor(&scene.cells, "sand", x, y) || family == "sand";
    let cardinal_count = mask.cardinal_count();

    let shoreline_candidate =
        (family == "water" && touches_sand) || (family == "sand" && touches_water);
    let cave_edge_candidate = family == "cave" && touches_other_family;
    let paved_brick_edge_candidate = family == "paved_brick" && touches_other_family;
    let wood_floor_edge_candidate = family == "wood_plank" && touches_other_family;
    let protected_edge_candidate = matches!(family, "cave") && layer == "cave_base";
    let ready_for_autotile = shoreline_candidate
        || cave_edge_candidate
        || paved_brick_edge_candidate
        || wood_floor_edge_candidate
        || (cardinal_count > 0 && cardinal_count < 4);
    let ready_for_debris = matches!(family, "sand" | "cave" | "paved_brick" | "wood_plank");
    let transition_role_hint = transition_role_hint(
        family,
        shoreline_candidate,
        cave_edge_candidate,
        paved_brick_edge_candidate,
        wood_floor_edge_candidate,
        cardinal_count,
    )
    .to_string();

    let status = format!(
        "family={} layer={} bits={} role={} shore={} cave={} brick={} wood={} debris={}",
        family,
        layer,
        mask.cardinal_bits(),
        transition_role_hint,
        yes_no(shoreline_candidate),
        yes_no(cave_edge_candidate),
        yes_no(paved_brick_edge_candidate),
        yes_no(wood_floor_edge_candidate),
        yes_no(ready_for_debris)
    );

    WorldPaintMaterialAdjacencyReport {
        schema: WORLD_PAINT_MATERIAL_ADJACENCY_SCHEMA.to_string(),
        scene_id: scene_id.to_string(),
        x,
        y,
        primary_family: primary.family.clone(),
        primary_layer: primary.layer.clone(),
        neighbor_mask: mask,
        shoreline_candidate,
        cave_edge_candidate,
        paved_brick_edge_candidate,
        wood_floor_edge_candidate,
        protected_edge_candidate,
        transition_role_hint,
        ready_for_autotile,
        ready_for_debris,
        status,
    }
}

pub fn resolve_world_paint_material_adjacency_path(
    path: impl AsRef<Path>,
    scene_id: &str,
    x: i32,
    y: i32,
) -> io::Result<WorldPaintMaterialAdjacencyReport> {
    let doc = load_world_paint_material_state_document(path)?;
    Ok(resolve_world_paint_material_adjacency(&doc, scene_id, x, y))
}

pub fn resolve_world_paint_material_scene_adjacency(
    doc: &WorldPaintMaterialStateDocument,
    scene_id: &str,
) -> WorldPaintMaterialAdjacencySceneReport {
    let Some(scene) = doc.scenes.iter().find(|scene| scene.scene_id == scene_id) else {
        return WorldPaintMaterialAdjacencySceneReport {
            schema: WORLD_PAINT_MATERIAL_ADJACENCY_SCHEMA.to_string(),
            scene_id: scene_id.to_string(),
            cell_reports: 0,
            shoreline_candidates: 0,
            cave_edge_candidates: 0,
            paved_brick_edge_candidates: 0,
            wood_floor_edge_candidates: 0,
            ready_for_autotile: 0,
            status: format!("No material-state scene {}", scene_id),
        };
    };

    let mut unique = scene
        .cells
        .iter()
        .map(|cell| (cell.x, cell.y))
        .collect::<Vec<_>>();
    unique.sort_unstable();
    unique.dedup();

    let mut report = WorldPaintMaterialAdjacencySceneReport {
        schema: WORLD_PAINT_MATERIAL_ADJACENCY_SCHEMA.to_string(),
        scene_id: scene_id.to_string(),
        cell_reports: 0,
        shoreline_candidates: 0,
        cave_edge_candidates: 0,
        paved_brick_edge_candidates: 0,
        wood_floor_edge_candidates: 0,
        ready_for_autotile: 0,
        status: String::new(),
    };

    for (x, y) in unique {
        let cell = resolve_world_paint_material_adjacency(doc, scene_id, x, y);
        report.cell_reports += 1;
        report.shoreline_candidates += cell.shoreline_candidate as usize;
        report.cave_edge_candidates += cell.cave_edge_candidate as usize;
        report.paved_brick_edge_candidates += cell.paved_brick_edge_candidate as usize;
        report.wood_floor_edge_candidates += cell.wood_floor_edge_candidate as usize;
        report.ready_for_autotile += cell.ready_for_autotile as usize;
    }
    report.status = report.status_line();
    report
}

pub fn resolve_world_paint_material_scene_adjacency_path(
    path: impl AsRef<Path>,
    scene_id: &str,
) -> io::Result<WorldPaintMaterialAdjacencySceneReport> {
    let doc = load_world_paint_material_state_document(path)?;
    Ok(resolve_world_paint_material_scene_adjacency(&doc, scene_id))
}

fn primary_cell<'a>(
    cells: impl Iterator<Item = &'a WorldPaintMaterialCell>,
) -> Option<&'a WorldPaintMaterialCell> {
    cells.max_by(|a, b| {
        family_priority(a.family.as_str())
            .cmp(&family_priority(b.family.as_str()))
            .then(a.last_sequence.cmp(&b.last_sequence))
            .then(a.layer.cmp(&b.layer))
    })
}

fn has_same_family_neighbor(
    cells: &[WorldPaintMaterialCell],
    family: &str,
    x: i32,
    y: i32,
) -> bool {
    cells
        .iter()
        .any(|cell| cell.x == x && cell.y == y && cell.family == family)
}

fn has_family_neighbor(cells: &[WorldPaintMaterialCell], family: &str, x: i32, y: i32) -> bool {
    neighbor_offsets().iter().any(|[dx, dy]| {
        cells
            .iter()
            .any(|cell| cell.x == x + dx && cell.y == y + dy && cell.family == family)
    })
}

fn has_other_family_neighbor(
    cells: &[WorldPaintMaterialCell],
    family: &str,
    x: i32,
    y: i32,
) -> bool {
    neighbor_offsets().iter().any(|[dx, dy]| {
        cells
            .iter()
            .any(|cell| cell.x == x + dx && cell.y == y + dy && cell.family != family)
    })
}

fn neighbor_offsets() -> [[i32; 2]; 8] {
    [
        [0, -1],
        [1, 0],
        [0, 1],
        [-1, 0],
        [1, -1],
        [1, 1],
        [-1, 1],
        [-1, -1],
    ]
}

fn transition_role_hint(
    family: &str,
    shoreline: bool,
    cave_edge: bool,
    brick_edge: bool,
    wood_edge: bool,
    cardinal_count: u8,
) -> &'static str {
    if shoreline {
        return "shoreline_edge_or_corner";
    }
    if cave_edge {
        return "cave_edge_or_cliff_face";
    }
    if brick_edge {
        return "town_paving_edge_or_corner";
    }
    if wood_edge {
        return "indoor_floor_edge_or_trim";
    }
    match (family, cardinal_count) {
        (_, 0) => "isolated_single_cell",
        (_, 1) => "cap_or_end",
        (_, 2) => "edge_or_corner",
        (_, 3) => "t_junction_or_inner_corner",
        (_, 4) => "filled_center",
        _ => "base_or_variation",
    }
}

fn family_priority(family: &str) -> u8 {
    match family {
        "water" => 5,
        "cave" => 4,
        "paved_brick" => 3,
        "wood_plank" => 2,
        "sand" => 1,
        _ => 0,
    }
}

fn bit(value: bool) -> &'static str {
    if value {
        "1"
    } else {
        "0"
    }
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}
