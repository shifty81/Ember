use super::{
    resolve_terrain_transitions_from_neighbors, resolve_transition_atlas_requests,
    resolve_transition_inner_corner_requests, CardinalDirection, DiagonalDirection,
    FamilyNeighbors, TerrainFamily, TerrainTransitionRule, TransitionMaterial, TransitionRulePhase,
};

/// Role of one cell in a 3x3 transition preview sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionPreviewCellRole {
    Center,
    SameFamily,
    NeighborCardinal,
    NeighborDiagonal,
}

impl TransitionPreviewCellRole {
    pub fn code(self) -> &'static str {
        match self {
            Self::Center => "center",
            Self::SameFamily => "same",
            Self::NeighborCardinal => "neighbor_cardinal",
            Self::NeighborDiagonal => "neighbor_diagonal",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionPreviewSampleKind {
    Edge,
    InnerCorner,
}

impl TransitionPreviewSampleKind {
    pub fn code(self) -> &'static str {
        match self {
            Self::Edge => "edge_sample",
            Self::InnerCorner => "inner_corner_sample",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Edge => "Edge Sample",
            Self::InnerCorner => "Inner Corner Sample",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionPreviewCell {
    pub grid_x: u8,
    pub grid_y: u8,
    pub family: TerrainFamily,
    pub role: TransitionPreviewCellRole,
    pub material: Option<TransitionMaterial>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransitionPreviewSample {
    pub kind: TransitionPreviewSampleKind,
    pub center: TerrainFamily,
    pub neighbor: TerrainFamily,
    pub cells: Vec<TransitionPreviewCell>,
    pub edge_count: usize,
    pub corner_count: usize,
    pub atlas_summary: String,
}

impl TransitionPreviewSample {
    pub fn summary_line(&self) -> String {
        format!(
            "{}: {} edge(s), {} corner(s), {}",
            self.kind.label(),
            self.edge_count,
            self.corner_count,
            self.atlas_summary
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainTransitionRulePreview {
    pub rule_id: String,
    pub center_selector: String,
    pub neighbor_selector: String,
    pub center_family: TerrainFamily,
    pub neighbor_family: TerrainFamily,
    pub material: TransitionMaterial,
    pub atlas_group: String,
    pub priority: i32,
    pub applies_to: Vec<TransitionRulePhase>,
    pub reason: Option<String>,
    pub samples: Vec<TransitionPreviewSample>,
}

impl TerrainTransitionRulePreview {
    pub fn summary_line(&self) -> String {
        format!(
            "{}: {} -> {} = {} @ {} / priority {}",
            self.rule_id,
            self.center_selector,
            self.neighbor_selector,
            self.material.code(),
            self.atlas_group,
            self.priority
        )
    }
}

pub fn preview_transition_rule(rule: &TerrainTransitionRule) -> TerrainTransitionRulePreview {
    let center_family = rule.center.representative_family();
    let neighbor_family = rule.neighbor.representative_family();
    let mut samples = Vec::new();
    if rule.applies_to_phase(TransitionRulePhase::Edge) {
        samples.push(build_preview_sample(
            TransitionPreviewSampleKind::Edge,
            center_family,
            neighbor_family,
        ));
    }
    if rule.applies_to_phase(TransitionRulePhase::Corner) {
        samples.push(build_preview_sample(
            TransitionPreviewSampleKind::InnerCorner,
            center_family,
            neighbor_family,
        ));
    }

    TerrainTransitionRulePreview {
        rule_id: rule.id.clone(),
        center_selector: rule.center.code().to_string(),
        neighbor_selector: rule.neighbor.code().to_string(),
        center_family,
        neighbor_family,
        material: rule.material,
        atlas_group: rule.atlas_group.clone(),
        priority: rule.priority,
        applies_to: rule.applies_to.clone(),
        reason: rule.reason.clone(),
        samples,
    }
}

pub fn preview_transition_rules() -> Result<Vec<TerrainTransitionRulePreview>, String> {
    let manifest = super::terrain_transition_rule_manifest().map_err(|error| error.to_string())?;
    Ok(manifest.rules.iter().map(preview_transition_rule).collect())
}

fn build_preview_sample(
    kind: TransitionPreviewSampleKind,
    center: TerrainFamily,
    neighbor: TerrainFamily,
) -> TransitionPreviewSample {
    let neighbors = match kind {
        TransitionPreviewSampleKind::Edge => FamilyNeighbors {
            center,
            north: neighbor,
            east: neighbor,
            south: neighbor,
            west: neighbor,
            north_east: center,
            south_east: center,
            south_west: center,
            north_west: center,
        },
        TransitionPreviewSampleKind::InnerCorner => FamilyNeighbors {
            center,
            north: center,
            east: center,
            south: center,
            west: center,
            north_east: neighbor,
            south_east: neighbor,
            south_west: neighbor,
            north_west: neighbor,
        },
    };
    let resolved = resolve_terrain_transitions_from_neighbors(neighbors);
    let atlas_requests = resolve_transition_atlas_requests(&resolved);
    let inner_corner_requests = resolve_transition_inner_corner_requests(&resolved);
    let mut atlas_parts = atlas_requests
        .iter()
        .map(|request| format!("{}:{:02x}", request.atlas_group, request.mask4))
        .collect::<Vec<_>>();
    atlas_parts.extend(
        inner_corner_requests
            .iter()
            .map(|request| format!("{}:inner:{:?}", request.atlas_group, request.direction)),
    );
    let atlas_summary = if atlas_parts.is_empty() {
        "no atlas request".to_string()
    } else {
        atlas_parts.join(",")
    };

    TransitionPreviewSample {
        kind,
        center,
        neighbor,
        cells: build_preview_cells(kind, center, neighbor, &resolved),
        edge_count: resolved.edges.len(),
        corner_count: resolved.corners.len(),
        atlas_summary,
    }
}

fn build_preview_cells(
    kind: TransitionPreviewSampleKind,
    center: TerrainFamily,
    neighbor: TerrainFamily,
    resolved: &super::ResolvedTerrainTransitions,
) -> Vec<TransitionPreviewCell> {
    let mut cells = Vec::with_capacity(9);
    for gy in 0..3u8 {
        for gx in 0..3u8 {
            let role = cell_role(kind, gx, gy);
            let family = match role {
                TransitionPreviewCellRole::Center | TransitionPreviewCellRole::SameFamily => center,
                TransitionPreviewCellRole::NeighborCardinal
                | TransitionPreviewCellRole::NeighborDiagonal => neighbor,
            };
            cells.push(TransitionPreviewCell {
                grid_x: gx,
                grid_y: gy,
                family,
                role,
                material: cell_material(gx, gy, resolved),
            });
        }
    }
    cells
}

fn cell_role(kind: TransitionPreviewSampleKind, gx: u8, gy: u8) -> TransitionPreviewCellRole {
    if gx == 1 && gy == 1 {
        return TransitionPreviewCellRole::Center;
    }
    let cardinal = (gx == 1 && gy != 1) || (gy == 1 && gx != 1);
    match (kind, cardinal) {
        (TransitionPreviewSampleKind::Edge, true) => TransitionPreviewCellRole::NeighborCardinal,
        (TransitionPreviewSampleKind::Edge, false) => TransitionPreviewCellRole::SameFamily,
        (TransitionPreviewSampleKind::InnerCorner, true) => TransitionPreviewCellRole::SameFamily,
        (TransitionPreviewSampleKind::InnerCorner, false) => {
            TransitionPreviewCellRole::NeighborDiagonal
        }
    }
}

fn cell_material(
    gx: u8,
    gy: u8,
    resolved: &super::ResolvedTerrainTransitions,
) -> Option<TransitionMaterial> {
    match (gx, gy) {
        (1, 0) => edge_material_at(resolved, CardinalDirection::North),
        (2, 1) => edge_material_at(resolved, CardinalDirection::East),
        (1, 2) => edge_material_at(resolved, CardinalDirection::South),
        (0, 1) => edge_material_at(resolved, CardinalDirection::West),
        (2, 0) => corner_material_at(resolved, DiagonalDirection::NorthEast),
        (2, 2) => corner_material_at(resolved, DiagonalDirection::SouthEast),
        (0, 2) => corner_material_at(resolved, DiagonalDirection::SouthWest),
        (0, 0) => corner_material_at(resolved, DiagonalDirection::NorthWest),
        _ => None,
    }
}

fn edge_material_at(
    resolved: &super::ResolvedTerrainTransitions,
    direction: CardinalDirection,
) -> Option<TransitionMaterial> {
    resolved
        .edges
        .iter()
        .find(|edge| edge.direction == direction)
        .map(|edge| edge.material)
}

fn corner_material_at(
    resolved: &super::ResolvedTerrainTransitions,
    direction: DiagonalDirection,
) -> Option<TransitionMaterial> {
    resolved
        .corners
        .iter()
        .find(|corner| corner.direction == direction)
        .map(|corner| corner.material)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autotile::terrain_transition_rule_manifest;

    #[test]
    fn preview_rules_generate_3x3_samples() {
        let manifest = terrain_transition_rule_manifest().expect("manifest should load");
        let rule = manifest
            .rules
            .iter()
            .find(|rule| rule.id == "shallow_water_touching_sand_bank")
            .expect("shoreline rule should exist");
        let preview = preview_transition_rule(rule);
        assert!(!preview.samples.is_empty());
        assert!(preview.samples.iter().all(|sample| sample.cells.len() == 9));
        assert!(preview.samples.iter().any(|sample| sample.edge_count > 0));
    }
}
