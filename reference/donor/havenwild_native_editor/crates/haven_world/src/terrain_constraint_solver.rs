//! Deterministic local constraint synthesis for complex terrain assemblies.
//!
//! Havenwild macro geography, structural tiers, hydrology, caves, settlements,
//! and routes are generated before this stage. This solver is deliberately a
//! *local* synthesis tool: a caller supplies the semantic/topological hard
//! constraints and the solver selects a compatible set of visual/recipe
//! modules. It is suitable for cliff bands, cave dressing, shoreline modules,
//! and other locally constrained assemblies; it is not a replacement for the
//! WorldPlan or Landform Feature Graph.
//!
//! The propagation model follows the useful portion of the simple-tiled WFC
//! family: every cell holds a domain of possible modules, constraints reduce
//! those domains, and compatibility is propagated until the grid is resolved.
//! Explicit adjacency rules are supported in addition to matching sockets so
//! non-Wang LPC relationships can be represented without hard-coding special
//! cases into renderers.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const TERRAIN_CONSTRAINT_SOLVER_SCHEMA: &str = "havenwild.terrain_constraint_solver.v0_1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainModuleId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainSocket(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TerrainDirection {
    North,
    East,
    South,
    West,
}

impl TerrainDirection {
    pub const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

    pub const fn index(self) -> usize {
        match self {
            Self::North => 0,
            Self::East => 1,
            Self::South => 2,
            Self::West => 3,
        }
    }

    pub const fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        }
    }

    pub const fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainConstraintModule {
    pub id: TerrainModuleId,
    /// Relative deterministic selection weight. Zero is invalid.
    pub weight: u32,
    /// N/E/S/W sockets. Matching sockets are the default compatibility rule.
    pub sockets: [TerrainSocket; 4],
}

impl TerrainConstraintModule {
    pub fn socket(&self, direction: TerrainDirection) -> TerrainSocket {
        self.sockets[direction.index()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TerrainAdjacencyRule {
    pub from: TerrainModuleId,
    pub direction: TerrainDirection,
    pub to: TerrainModuleId,
}

#[derive(Clone, Debug, Default)]
pub struct TerrainConstraintCatalog {
    pub modules: Vec<TerrainConstraintModule>,
    /// Optional explicit adjacency relationships. When any explicit rule exists
    /// for a `(from, direction)` pair, that pair uses only the explicit rules;
    /// otherwise socket compatibility is used.
    pub adjacency: Vec<TerrainAdjacencyRule>,
}

impl TerrainConstraintCatalog {
    pub fn validate(&self) -> Result<(), TerrainConstraintError> {
        if self.modules.is_empty() {
            return Err(TerrainConstraintError::InvalidCatalog(
                "constraint catalog has no modules".into(),
            ));
        }
        let mut ids = BTreeSet::new();
        for module in &self.modules {
            if !ids.insert(module.id) {
                return Err(TerrainConstraintError::InvalidCatalog(format!(
                    "duplicate terrain module id {}",
                    module.id.0
                )));
            }
            if module.weight == 0 {
                return Err(TerrainConstraintError::InvalidCatalog(format!(
                    "terrain module {} has zero weight",
                    module.id.0
                )));
            }
        }
        for rule in &self.adjacency {
            if !ids.contains(&rule.from) || !ids.contains(&rule.to) {
                return Err(TerrainConstraintError::InvalidCatalog(format!(
                    "adjacency rule references unknown module {} -> {}",
                    rule.from.0, rule.to.0
                )));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainCellConstraint {
    pub x: usize,
    pub y: usize,
    /// The semantic/topology stage can leave several compatible visual modules
    /// available, or provide one module for a fully fixed cell.
    pub allowed: Vec<TerrainModuleId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainConstraintSolution {
    pub width: usize,
    pub height: usize,
    pub cells: Vec<TerrainModuleId>,
}

impl TerrainConstraintSolution {
    pub fn get(&self, x: usize, y: usize) -> Option<TerrainModuleId> {
        (x < self.width && y < self.height).then(|| self.cells[y * self.width + x])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerrainConstraintError {
    InvalidCatalog(String),
    InvalidDimensions,
    InvalidConstraint(String),
    Contradiction { x: usize, y: usize },
}

#[derive(Clone, Debug)]
struct CompiledCatalog<'a> {
    source: &'a TerrainConstraintCatalog,
    index_by_id: BTreeMap<TerrainModuleId, usize>,
    explicit: BTreeMap<(TerrainModuleId, TerrainDirection), BTreeSet<TerrainModuleId>>,
}

impl<'a> CompiledCatalog<'a> {
    fn new(source: &'a TerrainConstraintCatalog) -> Result<Self, TerrainConstraintError> {
        source.validate()?;
        let index_by_id = source
            .modules
            .iter()
            .enumerate()
            .map(|(index, module)| (module.id, index))
            .collect();
        let mut explicit: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
        for rule in &source.adjacency {
            explicit
                .entry((rule.from, rule.direction))
                .or_default()
                .insert(rule.to);
        }
        Ok(Self {
            source,
            index_by_id,
            explicit,
        })
    }

    fn index_of(&self, id: TerrainModuleId) -> Option<usize> {
        self.index_by_id.get(&id).copied()
    }

    fn compatible(&self, from_index: usize, direction: TerrainDirection, to_index: usize) -> bool {
        let from = &self.source.modules[from_index];
        let to = &self.source.modules[to_index];
        if let Some(allowed) = self.explicit.get(&(from.id, direction)) {
            return allowed.contains(&to.id);
        }
        from.socket(direction) == to.socket(direction.opposite())
    }
}

/// Solve a bounded local terrain-module region deterministically.
///
/// The caller owns the large-scale constraints. A cliff caller, for example,
/// constrains cells to rim/wall/foot/join modules derived from a traced Level
/// boundary. The solver only chooses locally compatible module variants.
pub fn solve_terrain_constraints(
    catalog: &TerrainConstraintCatalog,
    width: usize,
    height: usize,
    seed: u64,
    constraints: &[TerrainCellConstraint],
) -> Result<TerrainConstraintSolution, TerrainConstraintError> {
    if width == 0 || height == 0 || width.checked_mul(height).is_none() {
        return Err(TerrainConstraintError::InvalidDimensions);
    }
    let compiled = CompiledCatalog::new(catalog)?;
    let all = (0..catalog.modules.len()).collect::<Vec<_>>();
    let mut domains = vec![all; width * height];

    for constraint in constraints {
        if constraint.x >= width || constraint.y >= height || constraint.allowed.is_empty() {
            return Err(TerrainConstraintError::InvalidConstraint(format!(
                "invalid cell constraint at {},{}",
                constraint.x, constraint.y
            )));
        }
        let mut allowed_indices = BTreeSet::new();
        for id in &constraint.allowed {
            let Some(index) = compiled.index_of(*id) else {
                return Err(TerrainConstraintError::InvalidConstraint(format!(
                    "constraint at {},{} references unknown module {}",
                    constraint.x, constraint.y, id.0
                )));
            };
            allowed_indices.insert(index);
        }
        let cell_index = constraint.y * width + constraint.x;
        domains[cell_index].retain(|candidate| allowed_indices.contains(candidate));
        if domains[cell_index].is_empty() {
            return Err(TerrainConstraintError::Contradiction {
                x: constraint.x,
                y: constraint.y,
            });
        }
    }

    let mut queue = (0..domains.len()).collect::<VecDeque<_>>();
    propagate(&compiled, width, height, &mut domains, &mut queue)?;

    let mut observation = 0_u64;
    while let Some(cell_index) = choose_lowest_entropy_cell(&domains, width, seed, observation) {
        let x = cell_index % width;
        let y = cell_index / width;
        let selected = choose_weighted_module(
            &domains[cell_index],
            &catalog.modules,
            deterministic_hash(seed, x as u64, y as u64, observation),
        );
        domains[cell_index].clear();
        domains[cell_index].push(selected);
        let mut local_queue = VecDeque::new();
        local_queue.push_back(cell_index);
        propagate(
            &compiled,
            width,
            height,
            &mut domains,
            &mut local_queue,
        )?;
        observation = observation.wrapping_add(1);
    }

    let cells = domains
        .into_iter()
        .enumerate()
        .map(|(index, domain)| {
            domain
                .first()
                .copied()
                .map(|module_index| catalog.modules[module_index].id)
                .ok_or(TerrainConstraintError::Contradiction {
                    x: index % width,
                    y: index / width,
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(TerrainConstraintSolution {
        width,
        height,
        cells,
    })
}

fn choose_lowest_entropy_cell(
    domains: &[Vec<usize>],
    width: usize,
    seed: u64,
    observation: u64,
) -> Option<usize> {
    domains
        .iter()
        .enumerate()
        .filter(|(_, domain)| domain.len() > 1)
        .min_by_key(|(index, domain)| {
            let x = *index % width;
            let y = *index / width;
            (
                domain.len(),
                deterministic_hash(seed, x as u64, y as u64, observation),
            )
        })
        .map(|(index, _)| index)
}

fn choose_weighted_module(
    domain: &[usize],
    modules: &[TerrainConstraintModule],
    random: u64,
) -> usize {
    let total = domain
        .iter()
        .map(|index| u64::from(modules[*index].weight))
        .sum::<u64>()
        .max(1);
    let mut ticket = random % total;
    for index in domain {
        let weight = u64::from(modules[*index].weight);
        if ticket < weight {
            return *index;
        }
        ticket -= weight;
    }
    domain[0]
}

fn propagate(
    catalog: &CompiledCatalog<'_>,
    width: usize,
    height: usize,
    domains: &mut [Vec<usize>],
    queue: &mut VecDeque<usize>,
) -> Result<(), TerrainConstraintError> {
    while let Some(cell_index) = queue.pop_front() {
        let x = cell_index % width;
        let y = cell_index / width;
        let source_domain = domains[cell_index].clone();

        for direction in TerrainDirection::ALL {
            let (dx, dy) = direction.delta();
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                continue;
            }
            let neighbor_index = ny as usize * width + nx as usize;
            let before = domains[neighbor_index].len();
            domains[neighbor_index].retain(|neighbor_candidate| {
                source_domain.iter().any(|source_candidate| {
                    catalog.compatible(*source_candidate, direction, *neighbor_candidate)
                })
            });
            if domains[neighbor_index].is_empty() {
                return Err(TerrainConstraintError::Contradiction {
                    x: nx as usize,
                    y: ny as usize,
                });
            }
            if domains[neighbor_index].len() != before {
                queue.push_back(neighbor_index);
            }
        }
    }
    Ok(())
}

fn deterministic_hash(seed: u64, x: u64, y: u64, step: u64) -> u64 {
    let mut z = seed
        ^ x.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ y.wrapping_mul(0xBF58_476D_1CE4_E5B9)
        ^ step.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: TerrainModuleId = TerrainModuleId(1);
    const B: TerrainModuleId = TerrainModuleId(2);

    fn matching_catalog() -> TerrainConstraintCatalog {
        TerrainConstraintCatalog {
            modules: vec![
                TerrainConstraintModule {
                    id: A,
                    weight: 1,
                    sockets: [TerrainSocket(1); 4],
                },
                TerrainConstraintModule {
                    id: B,
                    weight: 1,
                    sockets: [TerrainSocket(2); 4],
                },
            ],
            adjacency: Vec::new(),
        }
    }

    #[test]
    fn fixed_constraint_propagates_matching_socket_family() {
        let solution = solve_terrain_constraints(
            &matching_catalog(),
            4,
            1,
            17,
            &[TerrainCellConstraint {
                x: 0,
                y: 0,
                allowed: vec![A],
            }],
        )
        .expect("solve");
        assert!(solution.cells.iter().all(|cell| *cell == A));
    }

    #[test]
    fn same_seed_and_constraints_are_stable() {
        let catalog = TerrainConstraintCatalog {
            modules: vec![
                TerrainConstraintModule {
                    id: A,
                    weight: 1,
                    sockets: [TerrainSocket(7); 4],
                },
                TerrainConstraintModule {
                    id: B,
                    weight: 3,
                    sockets: [TerrainSocket(7); 4],
                },
            ],
            adjacency: Vec::new(),
        };
        let first = solve_terrain_constraints(&catalog, 8, 8, 99, &[]).expect("first");
        let second = solve_terrain_constraints(&catalog, 8, 8, 99, &[]).expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn explicit_non_wang_adjacency_overrides_socket_match() {
        let catalog = TerrainConstraintCatalog {
            modules: vec![
                TerrainConstraintModule {
                    id: A,
                    weight: 1,
                    sockets: [TerrainSocket(5); 4],
                },
                TerrainConstraintModule {
                    id: B,
                    weight: 1,
                    sockets: [TerrainSocket(5); 4],
                },
            ],
            adjacency: vec![
                TerrainAdjacencyRule {
                    from: A,
                    direction: TerrainDirection::East,
                    to: B,
                },
                TerrainAdjacencyRule {
                    from: B,
                    direction: TerrainDirection::West,
                    to: A,
                },
            ],
        };
        let solution = solve_terrain_constraints(
            &catalog,
            2,
            1,
            1,
            &[TerrainCellConstraint {
                x: 0,
                y: 0,
                allowed: vec![A],
            }],
        )
        .expect("solve");
        assert_eq!(solution.cells, vec![A, B]);
    }

    #[test]
    fn contradictory_constraints_fail_cleanly() {
        let err = solve_terrain_constraints(
            &matching_catalog(),
            2,
            1,
            1,
            &[
                TerrainCellConstraint {
                    x: 0,
                    y: 0,
                    allowed: vec![A],
                },
                TerrainCellConstraint {
                    x: 1,
                    y: 0,
                    allowed: vec![B],
                },
            ],
        )
        .expect_err("contradiction expected");
        assert!(matches!(err, TerrainConstraintError::Contradiction { .. }));
    }
}
