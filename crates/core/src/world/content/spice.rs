use super::{ContentCatalog, ContentId, Direction3};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentVolume {
    pub dimensions: [usize; 3],
    pub seed: u64,
    pub cells: Vec<ContentId>,
}

impl ContentVolume {
    pub fn new(dimensions: [usize; 3], seed: u64, cells: Vec<ContentId>) -> Self {
        Self {
            dimensions,
            seed,
            cells,
        }
    }

    pub fn get(&self, position: [usize; 3]) -> Option<ContentId> {
        volume_index(self.dimensions, position).and_then(|index| self.cells.get(index).copied())
    }

    pub fn set(&mut self, position: [usize; 3], content: ContentId) -> bool {
        let Some(index) = volume_index(self.dimensions, position) else {
            return false;
        };
        let Some(cell) = self.cells.get_mut(index) else {
            return false;
        };
        *cell = content;
        true
    }

    pub fn contains(&self, content: ContentId) -> bool {
        self.cells.contains(&content)
    }

    pub fn count(&self, content: ContentId) -> usize {
        self.cells.iter().filter(|cell| **cell == content).count()
    }

    pub fn all_neighbors_are_compatible(&self, catalog: &ContentCatalog) -> bool {
        self.validate(catalog).is_ok()
    }

    pub fn validate(&self, catalog: &ContentCatalog) -> Result<(), ContentGenerationError> {
        if self.dimensions.contains(&0) {
            return Err(ContentGenerationError::EmptyDimensions);
        }
        for (index, &content) in self.cells.iter().enumerate() {
            if catalog.index_of(content).is_none() {
                return Err(ContentGenerationError::UnknownContent { content });
            }
            let position = position_from_index(self.dimensions, index);
            for direction in Direction3::POSITIVE {
                let Some(neighbor_position) =
                    neighbor_position(self.dimensions, position, direction)
                else {
                    continue;
                };
                let Some(neighbor_index) = volume_index(self.dimensions, neighbor_position) else {
                    continue;
                };
                let neighbor = self.cells[neighbor_index];
                if !catalog.is_compatible(content, direction, neighbor) {
                    return Err(ContentGenerationError::IncompatibleNeighbor {
                        position,
                        direction,
                        content,
                        neighbor_position,
                        neighbor,
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentGenerationError {
    EmptyDimensions,
    UnknownContent {
        content: ContentId,
    },
    IncompatibleNeighbor {
        position: [usize; 3],
        direction: Direction3,
        content: ContentId,
        neighbor_position: [usize; 3],
        neighbor: ContentId,
    },
}

impl fmt::Display for ContentGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDimensions => write!(
                formatter,
                "content volume dimensions must be at least 5x3x5"
            ),
            Self::UnknownContent { content } => {
                write!(formatter, "content references unknown id {}", content.0)
            }
            Self::IncompatibleNeighbor {
                position,
                direction,
                content,
                neighbor_position,
                neighbor,
            } => write!(
                formatter,
                "content id {} at {position:?} is incompatible with id {} at {neighbor_position:?} toward {direction:?}",
                content.0, neighbor.0
            ),
        }
    }
}

impl std::error::Error for ContentGenerationError {}

pub const fn volume_index(dimensions: [usize; 3], position: [usize; 3]) -> Option<usize> {
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
