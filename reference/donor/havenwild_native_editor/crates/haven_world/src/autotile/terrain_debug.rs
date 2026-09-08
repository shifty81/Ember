use haven_core::{TavernMap, TileKind};

use super::{family_neighbors, resolve_terrain_transitions_from_neighbors, TerrainFamily};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainDebugCell {
    pub x: i32,
    pub y: i32,
    pub tile: TileKind,
    pub family: TerrainFamily,
    pub same_mask: u8,
    pub different_mask: u8,
    pub water_mask: u8,
    pub land_mask: u8,
    pub edge_count: usize,
    pub corner_count: usize,
}

impl TerrainDebugCell {
    pub fn has_transition(&self) -> bool {
        self.edge_count > 0 || self.corner_count > 0
    }

    pub fn summary(&self) -> String {
        format!(
            "{} / {} | same {} diff {} water {} land {} | E{} C{}",
            self.tile.code(),
            self.family.code(),
            mask_code(self.same_mask),
            mask_code(self.different_mask),
            mask_code(self.water_mask),
            mask_code(self.land_mask),
            self.edge_count,
            self.corner_count
        )
    }
}

pub fn terrain_debug_cell(map: &TavernMap, x: i32, y: i32) -> Option<TerrainDebugCell> {
    TavernMap::idx(x, y)?;
    let tile = map.get(x, y);
    let neighbors = family_neighbors(map, x, y);
    let transitions = resolve_terrain_transitions_from_neighbors(neighbors);
    Some(TerrainDebugCell {
        x,
        y,
        tile,
        family: neighbors.center,
        same_mask: neighbors.same_family_cardinal_mask(),
        different_mask: neighbors.different_family_cardinal_mask(),
        water_mask: neighbors.water_cardinal_mask(),
        land_mask: neighbors.land_cardinal_mask(),
        edge_count: transitions.edges.len(),
        corner_count: transitions.corners.len(),
    })
}

pub fn mask_code(mask: u8) -> String {
    let mut parts = Vec::new();
    if mask & super::N != 0 {
        parts.push("N");
    }
    if mask & super::E != 0 {
        parts.push("E");
    }
    if mask & super::S != 0 {
        parts.push("S");
    }
    if mask & super::W != 0 {
        parts.push("W");
    }
    if mask & super::NE != 0 {
        parts.push("NE");
    }
    if mask & super::SE != 0 {
        parts.push("SE");
    }
    if mask & super::SW != 0 {
        parts.push("SW");
    }
    if mask & super::NW != 0 {
        parts.push("NW");
    }
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use haven_core::TileKind;

    #[test]
    fn debug_cell_reports_land_neighbor_on_water_owner_side() {
        let mut map = TavernMap::empty_with(TileKind::ShallowWater);
        map.set(5, 4, TileKind::Grass);

        let debug = terrain_debug_cell(&map, 4, 4).expect("valid debug cell");
        assert!(debug.land_mask & super::super::E != 0);
        assert_eq!(debug.edge_count, 1);
        assert!(debug.has_transition());
        assert!(debug.summary().contains("land"));
    }
}
