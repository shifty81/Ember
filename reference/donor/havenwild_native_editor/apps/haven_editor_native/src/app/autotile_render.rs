use super::*;
use haven_core::TileAutoGroup;
use haven_world::autotile::{
    CardinalDirection, DiagonalDirection, LiveAutotileCache, ResolvedAutotileCell,
    TransitionMaterial, E, N, S, W,
};

pub(crate) fn draw_live_autotile_preview(
    cache: &LiveAutotileCache,
    opacity: f32,
    show_dirty: bool,
    zoom: f32,
) {
    for y in 0..MAP_H as i32 {
        for x in 0..MAP_W as i32 {
            if let Some(cell) = cache.resolved_at(x, y) {
                draw_autotile_cell(cell, opacity, zoom);
            }
        }
    }
    if show_dirty {
        let line = 0.06 / zoom.max(0.20);
        for &(x, y) in cache.last_dirty_cells() {
            draw_rectangle(
                x as f32,
                y as f32,
                1.0,
                1.0,
                Color::new(WARN.r, WARN.g, WARN.b, 0.12),
            );
            draw_rectangle_lines(x as f32, y as f32, 1.0, 1.0, line, WARN);
        }
    }
}

fn draw_autotile_cell(cell: &ResolvedAutotileCell, opacity: f32, zoom: f32) {
    let base = autotile_group_color(cell.group);
    let glyph = Color::new(base.r, base.g, base.b, 0.82 * opacity);
    let cx = cell.x as f32 + 0.5;
    let cy = cell.y as f32 + 0.5;
    let width = 0.14;
    draw_rectangle(cx - width * 0.5, cy - width * 0.5, width, width, glyph);
    if cell.mask & N != 0 {
        draw_rectangle(cx - width * 0.5, cell.y as f32, width, 0.5, glyph);
    }
    if cell.mask & E != 0 {
        draw_rectangle(cx, cy - width * 0.5, 0.5, width, glyph);
    }
    if cell.mask & S != 0 {
        draw_rectangle(cx - width * 0.5, cy, width, 0.5, glyph);
    }
    if cell.mask & W != 0 {
        draw_rectangle(cell.x as f32, cy - width * 0.5, 0.5, width, glyph);
    }

    draw_transition_preview(cell, opacity);
    if cell.manual_override() {
        draw_rectangle_lines(
            cell.x as f32 + 0.08,
            cell.y as f32 + 0.08,
            0.84,
            0.84,
            0.08 / zoom.max(0.20),
            Color::new(0.95, 0.35, 0.88, 0.96),
        );
    }
}

fn draw_transition_preview(cell: &ResolvedAutotileCell, opacity: f32) {
    let band = 0.16;
    for edge in &cell.transitions.edges {
        let mut color = transition_material_color(edge.material);
        color.a *= opacity;
        match edge.direction {
            CardinalDirection::North => {
                draw_rectangle(cell.x as f32, cell.y as f32, 1.0, band, color)
            }
            CardinalDirection::East => {
                draw_rectangle(cell.x as f32 + 1.0 - band, cell.y as f32, band, 1.0, color)
            }
            CardinalDirection::South => {
                draw_rectangle(cell.x as f32, cell.y as f32 + 1.0 - band, 1.0, band, color)
            }
            CardinalDirection::West => {
                draw_rectangle(cell.x as f32, cell.y as f32, band, 1.0, color)
            }
        }
    }
    let corner = 0.24;
    for transition in &cell.transitions.corners {
        let mut color = transition_material_color(transition.material);
        color.a *= opacity * 0.86;
        let (x, y) = match transition.direction {
            DiagonalDirection::NorthEast => (cell.x as f32 + 1.0 - corner, cell.y as f32),
            DiagonalDirection::SouthEast => {
                (cell.x as f32 + 1.0 - corner, cell.y as f32 + 1.0 - corner)
            }
            DiagonalDirection::SouthWest => (cell.x as f32, cell.y as f32 + 1.0 - corner),
            DiagonalDirection::NorthWest => (cell.x as f32, cell.y as f32),
        };
        draw_rectangle(x, y, corner, corner, color);
    }
}

fn autotile_group_color(group: TileAutoGroup) -> Color {
    match group {
        TileAutoGroup::Road => Color::new(0.96, 0.79, 0.42, 1.0),
        TileAutoGroup::WoodFloor => Color::new(0.82, 0.58, 0.32, 1.0),
        TileAutoGroup::StoneFloor => Color::new(0.72, 0.74, 0.75, 1.0),
        TileAutoGroup::Water => Color::new(0.34, 0.76, 0.96, 1.0),
        TileAutoGroup::Wall => Color::new(0.88, 0.72, 0.52, 1.0),
        TileAutoGroup::Cliff => Color::new(0.62, 0.54, 0.46, 1.0),
        TileAutoGroup::CaveWall => Color::new(0.56, 0.48, 0.62, 1.0),
    }
}

fn transition_material_color(material: TransitionMaterial) -> Color {
    match material {
        TransitionMaterial::WetSand => Color::new(0.66, 0.58, 0.39, 0.50),
        TransitionMaterial::Foam => Color::new(0.90, 0.96, 0.96, 0.58),
        TransitionMaterial::ShallowWaterEdge => Color::new(0.34, 0.76, 0.90, 0.48),
        TransitionMaterial::SandBlend => Color::new(0.82, 0.72, 0.48, 0.46),
        TransitionMaterial::GrassFringe => Color::new(0.42, 0.66, 0.34, 0.46),
        TransitionMaterial::DirtBlend => Color::new(0.56, 0.40, 0.28, 0.46),
        TransitionMaterial::RoadShoulder => Color::new(0.76, 0.62, 0.38, 0.50),
        TransitionMaterial::StoneShoulder => Color::new(0.58, 0.60, 0.62, 0.50),
        TransitionMaterial::RockShadow => Color::new(0.20, 0.18, 0.22, 0.54),
    }
}
