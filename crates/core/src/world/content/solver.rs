use super::{ContentCatalog, ContentId, Direction3};
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellConstraint {
    pub position: [usize; 3],
    pub allowed: Vec<ContentId>,
}

impl CellConstraint {
    pub fn only(position: [usize; 3], content: ContentId) -> Self {
        Self { position, allowed: vec![content] }
    }

    pub fn any_of(position: [usize; 3], allowed: impl IntoIterator<Item = ContentId>) -> Self {
        Self { position, allowed: allowed.into_iter().collect() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentSolveConfig {
    pub dimensions: [usize; 3],
    pub seed: u64,
    pub constraints: Vec<CellConstraint>,
}

impl ContentSolveConfig {
    pub const fn new(dimensions: [usize; 3], seed: u64) -> Self {
        Self { dimensions, seed, constraints: Vec::new() }
    }

    #[must_use]
    pub fn with_constraint(mut self, constraint: CellConstraint) -> Self {
        self.constraints.push(constraint);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentVolume {
    pub dimensions: [usize; 3],
    pub seed: u64,
    pub cells: Vec<ContentId>,
}

impl ContentVolume {
    pub fn get(&self, position: [usize; 3]) -> Option<ContentId> {
        volume_index(self.dimensions, position).and_then(|index| self.cells.get(index).copied())
    }

    pub fn contains(&self, content: ContentId) -> bool {
        self.cells.contains(&content)
    }

    pub fn all_neighbors_are_compatible(&self, catalog: &ContentCatalog) -> bool {
        for index in 0..self.cells.len() {
            let position = position_from_index(self.dimensions, index);
            for direction in Direction3::POSITIVE {
                let Some(neighbor_position) =
                    neighbor_position(self.dimensions, position, direction)
                else {
                    continue;
                };
                let Some(neighbor_index) = volume_index(self.dimensions, neighbor_position) else {
                    return false;
                };
                if !catalog.is_compatible(self.cells[index], direction, self.cells[neighbor_index])
                {
                    return false;
                }
            }
        }
        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentSolveError {
    EmptyDimensions,
    ConstraintOutOfBounds { position: [usize; 3] },
    UnknownContent { content: ContentId },
    EmptyConstraint { position: [usize; 3] },
    Contradiction { position: [usize; 3] },
}

impl fmt::Display for ContentSolveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDimensions => {
                write!(formatter, "content volume dimensions must be non-zero")
            }
            Self::ConstraintOutOfBounds { position } => {
                write!(
                    formatter,
                    "content constraint is out of bounds at {position:?}"
                )
            }
            Self::UnknownContent { content } => {
                write!(
                    formatter,
                    "content constraint references unknown id {}",
                    content.0
                )
            }
            Self::EmptyConstraint { position } => {
                write!(
                    formatter,
                    "content constraint has no candidates at {position:?}"
                )
            }
            Self::Contradiction { position } => {
                write!(formatter, "content constraints contradict at {position:?}")
            }
        }
    }
}

impl std::error::Error for ContentSolveError {}

pub fn solve_content_volume(
    catalog: &ContentCatalog,
    config: &ContentSolveConfig,
) -> Result<ContentVolume, ContentSolveError> {
    if config.dimensions.contains(&0) {
        return Err(ContentSolveError::EmptyDimensions);
    }

    let cell_count = config.dimensions[0]
        .checked_mul(config.dimensions[1])
        .and_then(|value| value.checked_mul(config.dimensions[2]))
        .ok_or(ContentSolveError::EmptyDimensions)?;
    let archetype_count = catalog.archetypes().len();
    let mut domains = vec![vec![true; archetype_count]; cell_count];
    let mut queue = VecDeque::new();

    for constraint in &config.constraints {
        let Some(cell_index) = volume_index(config.dimensions, constraint.position) else {
            return Err(ContentSolveError::ConstraintOutOfBounds { position: constraint.position });
        };
        if constraint.allowed.is_empty() {
            return Err(ContentSolveError::EmptyConstraint { position: constraint.position });
        }

        let mut allowed_indices = Vec::with_capacity(constraint.allowed.len());
        for &content in &constraint.allowed {
            let Some(index) = catalog.index_of(content) else {
                return Err(ContentSolveError::UnknownContent { content });
            };
            allowed_indices.push(index);
        }
        for (index, possible) in domains[cell_index].iter_mut().enumerate() {
            *possible &= allowed_indices.contains(&index);
        }
        if !domains[cell_index].iter().any(|possible| *possible) {
            return Err(ContentSolveError::Contradiction { position: constraint.position });
        }
        queue.push_back(cell_index);
    }

    propagate(catalog, config.dimensions, &mut domains, &mut queue)?;

    let mut rng = StdRng::seed_from_u64(config.seed);
    while let Some(cell_index) = select_lowest_entropy_cell(&domains, &mut rng) {
        let selected = select_weighted_archetype(catalog, &domains[cell_index], &mut rng);
        for (index, possible) in domains[cell_index].iter_mut().enumerate() {
            *possible = index == selected;
        }
        queue.push_back(cell_index);
        propagate(catalog, config.dimensions, &mut domains, &mut queue)?;
    }

    let mut cells = Vec::with_capacity(cell_count);
    for (index, domain) in domains.iter().enumerate() {
        let Some(archetype_index) = domain.iter().position(|possible| *possible) else {
            return Err(ContentSolveError::Contradiction {
                position: position_from_index(config.dimensions, index),
            });
        };
        cells.push(catalog.archetypes()[archetype_index].id);
    }

    Ok(ContentVolume { dimensions: config.dimensions, seed: config.seed, cells })
}

fn propagate(
    catalog: &ContentCatalog,
    dimensions: [usize; 3],
    domains: &mut [Vec<bool>],
    queue: &mut VecDeque<usize>,
) -> Result<(), ContentSolveError> {
    while let Some(source_index) = queue.pop_front() {
        let source_position = position_from_index(dimensions, source_index);
        for direction in Direction3::ALL {
            let Some(target_position) = neighbor_position(dimensions, source_position, direction)
            else {
                continue;
            };
            let Some(target_index) = volume_index(dimensions, target_position) else {
                continue;
            };

            let mut changed = false;
            for target_archetype in 0..catalog.archetypes().len() {
                if !domains[target_index][target_archetype] {
                    continue;
                }
                let target_id = catalog.archetypes()[target_archetype].id;
                let supported = domains[source_index]
                    .iter()
                    .enumerate()
                    .filter(|(_, possible)| **possible)
                    .any(|(source_archetype, _)| {
                        let source_id = catalog.archetypes()[source_archetype].id;
                        catalog.is_compatible(source_id, direction, target_id)
                    });
                if !supported {
                    domains[target_index][target_archetype] = false;
                    changed = true;
                }
            }

            if !domains[target_index].iter().any(|possible| *possible) {
                return Err(ContentSolveError::Contradiction { position: target_position });
            }
            if changed {
                queue.push_back(target_index);
            }
        }
    }
    Ok(())
}

fn select_lowest_entropy_cell(domains: &[Vec<bool>], rng: &mut StdRng) -> Option<usize> {
    let minimum = domains
        .iter()
        .map(|domain| domain.iter().filter(|possible| **possible).count())
        .filter(|count| *count > 1)
        .min()?;
    let candidates: Vec<usize> = domains
        .iter()
        .enumerate()
        .filter_map(|(index, domain)| {
            (domain.iter().filter(|possible| **possible).count() == minimum).then_some(index)
        })
        .collect();
    Some(candidates[rng.random_range(0..candidates.len())])
}

fn select_weighted_archetype(catalog: &ContentCatalog, domain: &[bool], rng: &mut StdRng) -> usize {
    let total_weight: u64 = domain
        .iter()
        .enumerate()
        .filter(|(_, possible)| **possible)
        .map(|(index, _)| u64::from(catalog.archetypes()[index].weight))
        .sum();
    let mut roll = rng.random_range(0..total_weight);
    for (index, possible) in domain.iter().enumerate() {
        if !possible {
            continue;
        }
        let weight = u64::from(catalog.archetypes()[index].weight);
        if roll < weight {
            return index;
        }
        roll -= weight;
    }
    domain.iter().position(|possible| *possible).unwrap_or(0)
}

const fn volume_index(dimensions: [usize; 3], position: [usize; 3]) -> Option<usize> {
    if position[0] >= dimensions[0] || position[1] >= dimensions[1] || position[2] >= dimensions[2]
    {
        return None;
    }
    Some((position[1] * dimensions[2] + position[2]) * dimensions[0] + position[0])
}

const fn position_from_index(dimensions: [usize; 3], index: usize) -> [usize; 3] {
    let x = index % dimensions[0];
    let yz = index / dimensions[0];
    let z = yz % dimensions[2];
    let y = yz / dimensions[2];
    [x, y, z]
}

const fn neighbor_position(
    dimensions: [usize; 3],
    position: [usize; 3],
    direction: Direction3,
) -> Option<[usize; 3]> {
    let [x, y, z] = position;
    match direction {
        Direction3::NegX if x > 0 => Some([x - 1, y, z]),
        Direction3::PosX if x + 1 < dimensions[0] => Some([x + 1, y, z]),
        Direction3::NegY if y > 0 => Some([x, y - 1, z]),
        Direction3::PosY if y + 1 < dimensions[1] => Some([x, y + 1, z]),
        Direction3::NegZ if z > 0 => Some([x, y, z - 1]),
        Direction3::PosZ if z + 1 < dimensions[2] => Some([x, y, z + 1]),
        _ => None,
    }
}
