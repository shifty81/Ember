use std::collections::VecDeque;

const MAX_GENERATED_RAMPS: usize = 32;
const RAMP_CLUSTER_SPACING: i32 = 10;
const RAMP_PATH_PROXIMITY_RADIUS: i32 = 14;

pub(super) fn directional_ramp_corridor_indices(
    upper: usize,
    lower: usize,
    width: usize,
    height: usize,
    rises_right: bool,
) -> Vec<usize> {
    debug_assert_eq!(upper.checked_add(width), Some(lower));
    let upper_x = (upper % width) as i32;
    let upper_y = (upper / width) as i32;
    let side = if rises_right { 1 } else { -1 };
    let offsets = [
        (side, -1),
        (side, 0),
        (0, 0),
        (0, 1),
        (-side, 1),
        (-side, 2),
    ];
    offsets
        .into_iter()
        .filter_map(|(dx, dy)| {
            let x = upper_x + dx;
            let y = upper_y + dy;
            ((0..width as i32).contains(&x) && (0..height as i32).contains(&y))
                .then(|| y as usize * width + x as usize)
        })
        .collect()
}

pub(super) fn directional_ramp_orientation_for_candidate(
    upper: usize,
    lower: usize,
    levels: &[u8],
    land: &[bool],
    protected: &[bool],
    width: usize,
    height: usize,
    seed: u64,
) -> Option<bool> {
    let upper_level = levels[upper];
    let lower_level = levels[lower];
    let viable = |rises_right: bool| {
        let corridor = directional_ramp_corridor_indices(upper, lower, width, height, rises_right);
        if corridor.len() != 6 {
            return false;
        }
        corridor.iter().enumerate().all(|(step, index)| {
            let expected = if step <= 2 { upper_level } else { lower_level };
            land[*index] && !protected[*index] && levels[*index] == expected
        })
    };
    match (viable(true), viable(false)) {
        (false, false) => None,
        (true, false) => Some(true),
        (false, true) => Some(false),
        (true, true) => {
            let x = (upper % width) as i32;
            let y = (upper / width) as i32;
            Some(seeded_edge_score(seed ^ 0x57_37, x, y) & 1 == 0)
        }
    }
}

pub(super) fn choose_south_ramp_edges(
    levels: &[u8],
    land: &[bool],
    protected: &[bool],
    width: usize,
    height: usize,
    seed: u64,
) -> Vec<(usize, usize, bool)> {
    if height < 2 || width == 0 {
        return Vec::new();
    }

    // Accessibility is component-based rather than global-random. Every
    // connected exact-level upland that owns a supported south-facing one-step
    // boundary gets a deterministic gateway before optional extra ramps are
    // considered. This prevents a large plateau from being sealed simply
    // because all eight old global candidates happened to land elsewhere.
    let components = exact_level_component_labels(levels, land, protected, width, height);
    let mut candidates = Vec::new();
    for y in 0..height - 1 {
        for x in 0..width {
            let Some(upper) = y.checked_mul(width).and_then(|row| row.checked_add(x)) else {
                continue;
            };
            let Some(lower) = upper.checked_add(width) else {
                continue;
            };
            if !land[upper]
                || !land[lower]
                || protected[upper]
                || protected[lower]
                || levels[upper] == 0
                || levels[upper] != levels[lower].saturating_add(1)
            {
                continue;
            }
            let component = components[upper];
            if component == usize::MAX {
                continue;
            }
            let Some(rises_right) = directional_ramp_orientation_for_candidate(
                upper, lower, levels, land, protected, width, height, seed,
            ) else {
                continue;
            };
            let path_distance = nearest_protected_distance(
                protected,
                width,
                height,
                x as i32,
                y as i32,
                RAMP_PATH_PROXIMITY_RADIUS,
            );
            let seeded = seeded_edge_score(seed, x as i32, y as i32);
            candidates.push((component, path_distance, seeded, upper, lower, rises_right));
        }
    }
    if candidates.is_empty() {
        return Vec::new();
    }

    candidates.sort_by_key(|entry| (entry.0, entry.1, entry.2));
    let mut selected = Vec::new();
    let mut selected_components = std::collections::BTreeSet::new();

    // First pass: one gateway for each reachable raised component, preferring
    // edges near existing protected roads/civic paths while staying outside the
    // protected no-cliff corridor.
    for &(component, _, _, upper, lower, rises_right) in &candidates {
        if selected.len() >= MAX_GENERATED_RAMPS {
            break;
        }
        if selected_components.insert(component) {
            selected.push((upper, lower, rises_right));
        }
    }

    // Second pass: broad components may receive additional gateways. Keep them
    // spatially separated so a long cliff does not become a picket fence of
    // ramps. The target still scales gently with available perimeter length.
    let target = candidates
        .len()
        .div_ceil(72)
        .max(selected.len())
        .min(MAX_GENERATED_RAMPS);
    if selected.len() < target {
        let mut extras = candidates.clone();
        extras.sort_by_key(|entry| (entry.1, entry.2));
        for &(_, _, _, upper, lower, rises_right) in &extras {
            if selected.len() >= target {
                break;
            }
            if selected.iter().any(|(existing, _, _)| {
                grid_manhattan_distance(*existing, upper, width) < RAMP_CLUSTER_SPACING
            }) {
                continue;
            }
            if !selected.iter().any(|(existing_upper, existing_lower, _)| {
                *existing_upper == upper && *existing_lower == lower
            }) {
                selected.push((upper, lower, rises_right));
            }
        }
    }

    selected
}

fn exact_level_component_labels(
    levels: &[u8],
    land: &[bool],
    protected: &[bool],
    width: usize,
    height: usize,
) -> Vec<usize> {
    let mut labels = vec![usize::MAX; levels.len()];
    let mut next_label = 0usize;
    for index in 0..levels.len() {
        if labels[index] != usize::MAX || levels[index] == 0 || !land[index] || protected[index] {
            continue;
        }
        let level = levels[index];
        let mut queue = VecDeque::from([index]);
        labels[index] = next_label;
        while let Some(current) = queue.pop_front() {
            let x = current % width;
            let y = current / width;
            for neighbor in cardinal_indices(x, y, width, height) {
                if labels[neighbor] != usize::MAX
                    || levels[neighbor] != level
                    || !land[neighbor]
                    || protected[neighbor]
                {
                    continue;
                }
                labels[neighbor] = next_label;
                queue.push_back(neighbor);
            }
        }
        next_label += 1;
    }
    labels
}

fn cardinal_indices(x: usize, y: usize, width: usize, height: usize) -> Vec<usize> {
    let mut result = Vec::with_capacity(4);
    if y > 0 {
        result.push((y - 1) * width + x);
    }
    if x + 1 < width {
        result.push(y * width + x + 1);
    }
    if y + 1 < height {
        result.push((y + 1) * width + x);
    }
    if x > 0 {
        result.push(y * width + x - 1);
    }
    result
}

fn nearest_protected_distance(
    protected: &[bool],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    radius: i32,
) -> i32 {
    let mut best = radius + 1;
    for offset_y in -radius..=radius {
        for offset_x in -radius..=radius {
            let distance = offset_x.abs() + offset_y.abs();
            if distance >= best || distance > radius {
                continue;
            }
            let nx = x + offset_x;
            let ny = y + offset_y;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            if protected[ny as usize * width + nx as usize] {
                best = distance;
            }
        }
    }
    best
}

fn grid_manhattan_distance(left: usize, right: usize, width: usize) -> i32 {
    let left_x = (left % width) as i32;
    let left_y = (left / width) as i32;
    let right_x = (right % width) as i32;
    let right_y = (right / width) as i32;
    (left_x - right_x).abs() + (left_y - right_y).abs()
}

fn seeded_edge_score(seed: u64, x: i32, y: i32) -> u64 {
    let mut value = seed ^ (x as i64 as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value ^= (y as i64 as u64).wrapping_mul(0xc2b2_ae3d_27d4_eb4f);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value.wrapping_mul(0x94d0_49bb_1331_11eb) ^ (value >> 31)
}

