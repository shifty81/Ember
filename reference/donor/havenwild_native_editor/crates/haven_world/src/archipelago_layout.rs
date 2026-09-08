use std::collections::BTreeMap;

use crate::scene_rectangles::{SceneRectangleManifest, SceneRectangleSpec};

const GOLDEN_ANGLE: f64 = 2.399_963_1;
const MAX_PLACEMENT_ATTEMPTS: usize = 360;
const FALLBACK_SCAN_STEP_PX: i32 = 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreviewRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl PreviewRect {
    fn right(self) -> i32 {
        self.x + self.width
    }

    fn bottom(self) -> i32 {
        self.y + self.height
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchipelagoPlacement {
    pub landmass_id: i32,
    pub landmass_name: String,
    pub bounds: PreviewRect,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchipelagoLayoutReport {
    pub seed: u64,
    pub island_count: usize,
    pub minimum_gap_px: i32,
    pub placements: Vec<ArchipelagoPlacement>,
}

#[derive(Clone, Debug)]
struct LandmassFootprint {
    id: i32,
    name: String,
    width: i32,
    height: i32,
    cells: Vec<FootprintCell>,
}

#[derive(Clone, Copy, Debug)]
struct FootprintCell {
    rectangle_index: usize,
    local_x: i32,
    local_y: i32,
}

pub fn rerolled_archipelago_seed(current_seed: u64, reroll_index: u32) -> u64 {
    splitmix64(
        current_seed.wrapping_add((reroll_index.max(1) as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)),
    )
}

pub fn generate_archipelago_layout(
    manifest: &mut SceneRectangleManifest,
    seed: u64,
) -> Result<ArchipelagoLayoutReport, String> {
    let footprints = collect_footprints(manifest)?;
    if footprints.is_empty() {
        return Err("archipelago has no exterior landmasses".to_string());
    }

    let generation = &manifest.archipelago_generation;
    let canvas_width = generation.canvas_size_px[0].max(1);
    let canvas_height = generation.canvas_size_px[1].max(1);
    let margin = generation.outer_margin_px.max(0);
    let minimum_gap = generation.minimum_island_gap_px.max(0);
    let placements = place_footprints(
        &footprints,
        seed,
        canvas_width,
        canvas_height,
        margin,
        minimum_gap,
    )?;

    for footprint in &footprints {
        let placement = placements
            .get(&footprint.id)
            .ok_or_else(|| format!("missing generated placement for landmass {}", footprint.id))?;
        for cell in &footprint.cells {
            let rectangle = &mut manifest.scene_rectangles[cell.rectangle_index];
            rectangle.world_rect_preview_px[0] = placement.x + cell.local_x;
            rectangle.world_rect_preview_px[1] = placement.y + cell.local_y;
        }
    }

    manifest.archipelago_generation.seed = seed;
    manifest.archipelago_generation.layout_version = 1;
    manifest.archipelago_generation.placement_mode = "seeded_collision_safe_ring".to_string();

    let report_placements = footprints
        .iter()
        .filter_map(|footprint| {
            placements
                .get(&footprint.id)
                .copied()
                .map(|bounds| ArchipelagoPlacement {
                    landmass_id: footprint.id,
                    landmass_name: footprint.name.clone(),
                    bounds,
                })
        })
        .collect();

    Ok(ArchipelagoLayoutReport {
        seed,
        island_count: footprints.len(),
        minimum_gap_px: minimum_gap,
        placements: report_placements,
    })
}

pub fn validate_archipelago_spacing(manifest: &SceneRectangleManifest) -> Vec<String> {
    let Ok(footprints) = collect_footprints(manifest) else {
        return vec!["archipelago footprints could not be collected".to_string()];
    };
    let gap = manifest.archipelago_generation.minimum_island_gap_px.max(0);
    let mut warnings = Vec::new();
    let bounds: Vec<(i32, PreviewRect)> = footprints
        .iter()
        .map(|footprint| (footprint.id, current_bounds(manifest, footprint)))
        .collect();
    for (index, (left_id, left)) in bounds.iter().enumerate() {
        for (right_id, right) in bounds.iter().skip(index + 1) {
            if overlaps_with_gap(*left, *right, gap) {
                warnings.push(format!(
                    "landmasses {left_id} and {right_id} overlap or violate the {gap}px minimum gap"
                ));
            }
        }
    }
    warnings
}

fn collect_footprints(manifest: &SceneRectangleManifest) -> Result<Vec<LandmassFootprint>, String> {
    let mut grouped: BTreeMap<i32, Vec<(usize, &SceneRectangleSpec)>> = BTreeMap::new();
    for (index, rectangle) in manifest.scene_rectangles.iter().enumerate() {
        if rectangle.grid_x.is_some() && rectangle.grid_y.is_some() {
            grouped
                .entry(rectangle.landmass_id)
                .or_default()
                .push((index, rectangle));
        }
    }

    grouped
        .into_iter()
        .map(|(id, rectangles)| {
            let min_grid_x = rectangles
                .iter()
                .filter_map(|(_, rectangle)| rectangle.grid_x)
                .min()
                .unwrap_or(0);
            let min_grid_y = rectangles
                .iter()
                .filter_map(|(_, rectangle)| rectangle.grid_y)
                .min()
                .unwrap_or(0);
            let cell_width = rectangles
                .iter()
                .map(|(_, rectangle)| rectangle.world_rect_preview_px[2])
                .max()
                .unwrap_or(1)
                .max(1);
            let cell_height = rectangles
                .iter()
                .map(|(_, rectangle)| rectangle.world_rect_preview_px[3])
                .max()
                .unwrap_or(1)
                .max(1);
            let mut width = 1;
            let mut height = 1;
            let mut cells = Vec::with_capacity(rectangles.len());
            for (rectangle_index, rectangle) in &rectangles {
                let grid_x = rectangle
                    .grid_x
                    .ok_or_else(|| format!("{} is missing grid_x", rectangle.scene_id))?;
                let grid_y = rectangle
                    .grid_y
                    .ok_or_else(|| format!("{} is missing grid_y", rectangle.scene_id))?;
                let local_x = (grid_x - min_grid_x) * cell_width;
                let local_y = (grid_y - min_grid_y) * cell_height;
                width = width.max(local_x + rectangle.world_rect_preview_px[2].max(1));
                height = height.max(local_y + rectangle.world_rect_preview_px[3].max(1));
                cells.push(FootprintCell {
                    rectangle_index: *rectangle_index,
                    local_x,
                    local_y,
                });
            }
            Ok(LandmassFootprint {
                id,
                name: rectangles
                    .first()
                    .map(|(_, rectangle)| rectangle.landmass_name.clone())
                    .unwrap_or_else(|| format!("Island {id}")),
                width,
                height,
                cells,
            })
        })
        .collect()
}

fn place_footprints(
    footprints: &[LandmassFootprint],
    seed: u64,
    canvas_width: i32,
    canvas_height: i32,
    margin: i32,
    minimum_gap: i32,
) -> Result<BTreeMap<i32, PreviewRect>, String> {
    let mut placed = BTreeMap::new();
    let main = footprints
        .iter()
        .find(|footprint| footprint.id == 0)
        .unwrap_or(&footprints[0]);
    ensure_fits_canvas(main, canvas_width, canvas_height, margin)?;
    placed.insert(
        main.id,
        PreviewRect {
            x: (canvas_width - main.width) / 2,
            y: (canvas_height - main.height) / 2,
            width: main.width,
            height: main.height,
        },
    );

    let others: Vec<&LandmassFootprint> = footprints
        .iter()
        .filter(|footprint| footprint.id != main.id)
        .collect();
    let mut rng = SeededLayoutRng::new(seed);
    let phase = rng.next_unit() * std::f64::consts::TAU;
    let island_count = others.len().max(1) as f64;

    for (index, footprint) in others.iter().enumerate() {
        ensure_fits_canvas(footprint, canvas_width, canvas_height, margin)?;
        let base_angle = phase
            + std::f64::consts::TAU * index as f64 / island_count
            + (rng.next_unit() - 0.5) * 0.30;
        let mut chosen = None;
        for attempt in 0..MAX_PLACEMENT_ATTEMPTS {
            let ring = attempt / 72;
            let angle = base_angle + (attempt % 72) as f64 * GOLDEN_ANGLE;
            let radial = 0.82 + ring as f64 * 0.04 + (rng.next_unit() - 0.5) * 0.10;
            let radius_x =
                (canvas_width as f64 * 0.5 - margin as f64 - footprint.width as f64 * 0.5) * radial;
            let radius_y =
                (canvas_height as f64 * 0.5 - margin as f64 - footprint.height as f64 * 0.5)
                    * radial;
            let center_x = canvas_width as f64 * 0.5 + angle.cos() * radius_x;
            let center_y = canvas_height as f64 * 0.5 + angle.sin() * radius_y;
            let candidate = clamp_to_canvas(
                PreviewRect {
                    x: (center_x - footprint.width as f64 * 0.5).round() as i32,
                    y: (center_y - footprint.height as f64 * 0.5).round() as i32,
                    width: footprint.width,
                    height: footprint.height,
                },
                canvas_width,
                canvas_height,
                margin,
            );
            if placed
                .values()
                .all(|existing| !overlaps_with_gap(candidate, *existing, minimum_gap))
            {
                chosen = Some(candidate);
                break;
            }
        }
        if chosen.is_none() {
            chosen = fallback_scan(
                footprint,
                &placed,
                canvas_width,
                canvas_height,
                margin,
                minimum_gap,
            );
        }
        let placement = chosen.ok_or_else(|| {
            format!(
                "could not place {} with a {minimum_gap}px minimum gap inside {}x{}",
                footprint.name, canvas_width, canvas_height
            )
        })?;
        placed.insert(footprint.id, placement);
    }
    Ok(placed)
}

fn ensure_fits_canvas(
    footprint: &LandmassFootprint,
    canvas_width: i32,
    canvas_height: i32,
    margin: i32,
) -> Result<(), String> {
    if footprint.width + margin * 2 > canvas_width || footprint.height + margin * 2 > canvas_height
    {
        return Err(format!(
            "{} footprint {}x{} does not fit inside the {}x{} archipelago canvas",
            footprint.name, footprint.width, footprint.height, canvas_width, canvas_height
        ));
    }
    Ok(())
}

fn current_bounds(manifest: &SceneRectangleManifest, footprint: &LandmassFootprint) -> PreviewRect {
    let rectangles: Vec<&SceneRectangleSpec> = footprint
        .cells
        .iter()
        .map(|cell| &manifest.scene_rectangles[cell.rectangle_index])
        .collect();
    let min_x = rectangles
        .iter()
        .map(|rectangle| rectangle.world_rect_preview_px[0])
        .min()
        .unwrap_or(0);
    let min_y = rectangles
        .iter()
        .map(|rectangle| rectangle.world_rect_preview_px[1])
        .min()
        .unwrap_or(0);
    let max_x = rectangles
        .iter()
        .map(|rectangle| rectangle.world_rect_preview_px[0] + rectangle.world_rect_preview_px[2])
        .max()
        .unwrap_or(min_x + 1);
    let max_y = rectangles
        .iter()
        .map(|rectangle| rectangle.world_rect_preview_px[1] + rectangle.world_rect_preview_px[3])
        .max()
        .unwrap_or(min_y + 1);
    PreviewRect {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    }
}

fn clamp_to_canvas(
    mut rect: PreviewRect,
    canvas_width: i32,
    canvas_height: i32,
    margin: i32,
) -> PreviewRect {
    rect.x = rect
        .x
        .clamp(margin, (canvas_width - margin - rect.width).max(margin));
    rect.y = rect
        .y
        .clamp(margin, (canvas_height - margin - rect.height).max(margin));
    rect
}

fn fallback_scan(
    footprint: &LandmassFootprint,
    placed: &BTreeMap<i32, PreviewRect>,
    canvas_width: i32,
    canvas_height: i32,
    margin: i32,
    minimum_gap: i32,
) -> Option<PreviewRect> {
    let max_x = canvas_width - margin - footprint.width;
    let max_y = canvas_height - margin - footprint.height;
    let mut y = margin;
    while y <= max_y {
        let mut x = margin;
        while x <= max_x {
            let candidate = PreviewRect {
                x,
                y,
                width: footprint.width,
                height: footprint.height,
            };
            if placed
                .values()
                .all(|existing| !overlaps_with_gap(candidate, *existing, minimum_gap))
            {
                return Some(candidate);
            }
            x += FALLBACK_SCAN_STEP_PX;
        }
        y += FALLBACK_SCAN_STEP_PX;
    }
    None
}

fn overlaps_with_gap(left: PreviewRect, right: PreviewRect, gap: i32) -> bool {
    !(left.right() + gap <= right.x
        || right.right() + gap <= left.x
        || left.bottom() + gap <= right.y
        || right.bottom() + gap <= left.y)
}

#[derive(Clone, Copy, Debug)]
struct SeededLayoutRng {
    state: u64,
}

impl SeededLayoutRng {
    fn new(seed: u64) -> Self {
        Self {
            state: splitmix64(seed.max(1)),
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = splitmix64(self.state.wrapping_add(0x9e37_79b9_7f4a_7c15));
        self.state
    }

    fn next_unit(&mut self) -> f64 {
        (self.next_u64() & 0xffff_ffff) as f64 / u32::MAX as f64
    }
}

fn splitmix64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene_rectangles::{
        load_scene_rectangle_manifest_from_path, SCENE_RECTANGLE_MANIFEST_PATH,
    };
    use std::path::Path;

    fn manifest() -> SceneRectangleManifest {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .expect("haven_world crate should live under crates/<name>");
        let manifest_path = repo_root.join(SCENE_RECTANGLE_MANIFEST_PATH);
        load_scene_rectangle_manifest_from_path(&manifest_path.to_string_lossy()).expect("manifest")
    }

    #[test]
    fn seeded_layout_is_repeatable_and_collision_safe() {
        let mut first = manifest();
        let mut second = manifest();
        let first_report = generate_archipelago_layout(&mut first, 42).expect("first layout");
        let second_report = generate_archipelago_layout(&mut second, 42).expect("second layout");
        assert_eq!(first_report, second_report);
        assert!(validate_archipelago_spacing(&first).is_empty());
    }

    #[test]
    fn different_seeds_reposition_surrounding_islands() {
        let mut first = manifest();
        let mut second = manifest();
        generate_archipelago_layout(&mut first, 42).expect("first layout");
        generate_archipelago_layout(&mut second, 43).expect("second layout");
        let first_positions: Vec<[i32; 2]> = first
            .scene_rectangles
            .iter()
            .filter(|rectangle| rectangle.landmass_id != 0)
            .map(|rectangle| {
                [
                    rectangle.world_rect_preview_px[0],
                    rectangle.world_rect_preview_px[1],
                ]
            })
            .collect();
        let second_positions: Vec<[i32; 2]> = second
            .scene_rectangles
            .iter()
            .filter(|rectangle| rectangle.landmass_id != 0)
            .map(|rectangle| {
                [
                    rectangle.world_rect_preview_px[0],
                    rectangle.world_rect_preview_px[1],
                ]
            })
            .collect();
        assert_ne!(first_positions, second_positions);
    }
}
