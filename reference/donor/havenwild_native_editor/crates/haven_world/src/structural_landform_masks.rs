//! Small mask utilities for discrete structural-landform generation.
//!
//! Kept separate from the world assembly/orchestration path so the generator
//! stays readable and the broad-shape normalization can be tested in isolation.

pub(crate) fn neighborhood_density_percent(
    mask: &[bool],
    width: usize,
    height: usize,
    index: usize,
    radius: i32,
) -> usize {
    let x = (index % width) as i32;
    let y = (index / width) as i32;
    let mut support = 0usize;
    let mut samples = 0usize;
    for offset_y in -radius..=radius {
        for offset_x in -radius..=radius {
            let nx = x + offset_x;
            let ny = y + offset_y;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            samples += 1;
            support += usize::from(mask[ny as usize * width + nx as usize]);
        }
    }
    if samples == 0 {
        0
    } else {
        support * 100 / samples
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn retain_broad_candidates(
    candidates: &[bool],
    present: &[bool],
    land: &[bool],
    protected: &[bool],
    width: usize,
    height: usize,
    radius: i32,
    minimum_density_percent: usize,
) -> Vec<bool> {
    candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            *candidate
                && present[index]
                && land[index]
                && !protected[index]
                && neighborhood_density_percent(candidates, width, height, index, radius)
                    >= minimum_density_percent
        })
        .collect()
}

fn adjacent_support(
    levels: &[u8],
    width: usize,
    height: usize,
    index: usize,
    minimum: u8,
) -> usize {
    let x = (index % width) as i32;
    let y = (index / width) as i32;
    let mut support = 0usize;
    for offset_y in -1..=1 {
        for offset_x in -1..=1 {
            if offset_x == 0 && offset_y == 0 {
                continue;
            }
            let nx = x + offset_x;
            let ny = y + offset_y;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            support += usize::from(levels[ny as usize * width + nx as usize] >= minimum);
        }
    }
    support
}


fn cardinal_support_at_threshold(
    levels: &[u8],
    width: usize,
    height: usize,
    index: usize,
    threshold: u8,
) -> usize {
    let x = (index % width) as i32;
    let y = (index / width) as i32;
    let mut support = 0_usize;
    for (offset_x, offset_y) in [(-1_i32, 0_i32), (1, 0), (0, -1), (0, 1)] {
        let nx = x + offset_x;
        let ny = y + offset_y;
        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
            continue;
        }
        support += usize::from(levels[ny as usize * width + nx as usize] >= threshold);
    }
    support
}

/// Normalize generated Level 1/2 contours before cliff topology is baked.
///
/// LPC cliff art is designed around broad platform contours. A one-cell spur
/// has three exposed cardinal edges and projects as a narrow rectangular rock
/// post; a one-cell notch produces the inverse visual defect. These shapes are
/// valid for an explicit player-authored construction, but they are undesirable
/// rasterization noise in procedurally generated macro landforms.
///
/// The cleanup is deliberately conservative:
/// * raised cells with zero/one same-threshold cardinal neighbor are lowered;
/// * lower cells surrounded on three/four cardinal sides are raised;
/// * cells with two or more cardinal supports, including 45-degree staircase
///   contours, are preserved;
/// * protected/civic cells are never raised and remain Level 0.
///
/// Run twice so removing a terminal spur can expose one immediately behind it.
/// Level 2 is normalized independently against the `>= 2` mask and therefore
/// falls back to Level 1 rather than punching holes through the whole plateau.
pub(crate) fn normalize_structural_contours(
    levels: &mut [u8],
    land: &[bool],
    protected: &[bool],
    width: usize,
    height: usize,
) {
    debug_assert_eq!(levels.len(), land.len());
    debug_assert_eq!(levels.len(), protected.len());

    for _ in 0..2 {
        for threshold in [1_u8, 2_u8] {
            let source = levels.to_vec();
            let mut next = source.clone();
            for index in 0..source.len() {
                if !land[index] || protected[index] {
                    next[index] = 0;
                    continue;
                }

                let support =
                    cardinal_support_at_threshold(&source, width, height, index, threshold);
                if source[index] >= threshold {
                    if support <= 1 {
                        next[index] = next[index].min(threshold - 1);
                    }
                } else if support >= 3 {
                    next[index] = next[index].max(threshold);
                }
            }
            levels.copy_from_slice(&next);
        }
    }
}

pub(crate) fn fill_small_structural_holes(
    levels: &mut [u8],
    land: &[bool],
    protected: &[bool],
    width: usize,
    height: usize,
) {
    let source = levels.to_vec();
    for (index, level) in levels.iter_mut().enumerate() {
        if !land[index] || protected[index] {
            *level = 0;
            continue;
        }
        if source[index] == 0 && adjacent_support(&source, width, height, index, 1) >= 7 {
            *level = 1;
        } else if source[index] == 1
            && adjacent_support(&source, width, height, index, 2) >= 7
        {
            *level = 2;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_candidate_ribbon_is_rejected_before_becoming_a_cliff() {
        let width = 9;
        let height = 9;
        let mut candidates = vec![false; width * height];
        for y in 0..height {
            candidates[y * width + width / 2] = true;
        }
        let present = vec![true; candidates.len()];
        let land = vec![true; candidates.len()];
        let protected = vec![false; candidates.len()];

        let retained = retain_broad_candidates(
            &candidates,
            &present,
            &land,
            &protected,
            width,
            height,
            2,
            68,
        );
        assert!(!retained.iter().any(|value| *value));
    }

    #[test]
    fn generated_single_cell_south_spur_is_removed() {
        let width = 5;
        let height = 5;
        let mut levels = vec![0_u8; width * height];
        // Broad 3x2 platform plus one-cell south spur at x=2,y=3.
        for y in 1..=2 {
            for x in 1..=3 {
                levels[y * width + x] = 1;
            }
        }
        levels[3 * width + 2] = 1;
        let land = vec![true; levels.len()];
        let protected = vec![false; levels.len()];

        normalize_structural_contours(&mut levels, &land, &protected, width, height);
        assert_eq!(levels[3 * width + 2], 0);
        assert_eq!(levels[2 * width + 2], 1);
    }

    #[test]
    fn generated_single_cell_notch_is_filled() {
        let width = 5;
        let height = 5;
        let mut levels = vec![1_u8; width * height];
        let center = 2 * width + 2;
        levels[center] = 0;
        let land = vec![true; levels.len()];
        let protected = vec![false; levels.len()];

        normalize_structural_contours(&mut levels, &land, &protected, width, height);
        assert_eq!(levels[center], 1);
    }

    #[test]
    fn two_neighbor_diagonal_staircase_corner_is_preserved() {
        let width = 5;
        let height = 5;
        let mut levels = vec![0_u8; width * height];
        // A compact staircase whose center boundary cell has north+east support.
        for (x, y) in [(2, 2), (2, 1), (3, 2), (3, 1), (4, 2), (4, 1)] {
            levels[y * width + x] = 1;
        }
        let land = vec![true; levels.len()];
        let protected = vec![false; levels.len()];

        normalize_structural_contours(&mut levels, &land, &protected, width, height);
        assert_eq!(levels[2 * width + 2], 1);
    }

    #[test]
    fn level_two_terminal_falls_back_to_level_one() {
        let width = 5;
        let height = 5;
        let mut levels = vec![1_u8; width * height];
        for (x, y) in [(2, 1), (2, 2)] {
            levels[y * width + x] = 2;
        }
        let land = vec![true; levels.len()];
        let protected = vec![false; levels.len()];

        normalize_structural_contours(&mut levels, &land, &protected, width, height);
        assert_eq!(levels[2 * width + 2], 1);
    }


    #[test]
    fn generated_cleanup_leaves_no_three_edge_or_isolated_raised_hosts() {
        let width = 9;
        let height = 9;
        let mut levels = vec![0_u8; width * height];
        // Broad platform with deliberately noisy one-cell tips on every side.
        for y in 2..=6 {
            for x in 2..=6 {
                levels[y * width + x] = 1;
            }
        }
        for (x, y) in [(4, 1), (7, 4), (4, 7), (1, 4), (0, 0)] {
            levels[y * width + x] = 1;
        }
        let land = vec![true; levels.len()];
        let protected = vec![false; levels.len()];

        normalize_structural_contours(&mut levels, &land, &protected, width, height);

        for (index, level) in levels.iter().copied().enumerate() {
            if level == 0 {
                continue;
            }
            let support = cardinal_support_at_threshold(&levels, width, height, index, 1);
            assert!(
                support >= 2,
                "fresh PCG retained a raised host with {} exposed edges at index {}",
                4_usize.saturating_sub(support),
                index
            );
        }
    }

}
