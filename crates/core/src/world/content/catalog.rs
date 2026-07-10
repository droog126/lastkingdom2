use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentId(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction3 {
    NegX,
    PosX,
    NegY,
    PosY,
    NegZ,
    PosZ,
}

impl Direction3 {
    pub const ALL: [Self; 6] = [
        Self::NegX,
        Self::PosX,
        Self::NegY,
        Self::PosY,
        Self::NegZ,
        Self::PosZ,
    ];

    pub const POSITIVE: [Self; 3] = [Self::PosX, Self::PosY, Self::PosZ];

    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::NegX => Self::PosX,
            Self::PosX => Self::NegX,
            Self::NegY => Self::PosY,
            Self::PosY => Self::NegY,
            Self::NegZ => Self::PosZ,
            Self::PosZ => Self::NegZ,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContentArchetype {
    pub id: ContentId,
    pub name: String,
    pub weight: u32,
}

#[derive(Clone, Debug)]
pub struct ContentCatalog {
    archetypes: Vec<ContentArchetype>,
    indices: HashMap<ContentId, usize>,
    allowed: HashSet<(ContentId, Direction3, ContentId)>,
}

impl ContentCatalog {
    pub fn builder() -> ContentCatalogBuilder {
        ContentCatalogBuilder::default()
    }

    pub fn archetypes(&self) -> &[ContentArchetype] {
        &self.archetypes
    }

    pub fn index_of(&self, id: ContentId) -> Option<usize> {
        self.indices.get(&id).copied()
    }

    pub fn is_compatible(
        &self,
        source: ContentId,
        direction: Direction3,
        neighbor: ContentId,
    ) -> bool {
        self.allowed.contains(&(source, direction, neighbor))
    }
}

#[derive(Clone, Debug, Default)]
pub struct ContentCatalogBuilder {
    archetypes: Vec<ContentArchetype>,
    allowed: HashSet<(ContentId, Direction3, ContentId)>,
}

impl ContentCatalogBuilder {
    #[must_use]
    pub fn archetype(mut self, id: ContentId, name: impl Into<String>, weight: u32) -> Self {
        self.archetypes.push(ContentArchetype { id, name: name.into(), weight });
        self
    }

    #[must_use]
    pub fn allow(mut self, source: ContentId, direction: Direction3, neighbor: ContentId) -> Self {
        self.allowed.insert((source, direction, neighbor));
        self
    }

    #[must_use]
    pub fn allow_bidirectional(
        mut self,
        source: ContentId,
        direction: Direction3,
        neighbor: ContentId,
    ) -> Self {
        self.allowed.insert((source, direction, neighbor));
        self.allowed.insert((neighbor, direction.opposite(), source));
        self
    }

    #[must_use]
    pub fn allow_all(mut self, first: ContentId, second: ContentId) -> Self {
        for direction in Direction3::ALL {
            self.allowed.insert((first, direction, second));
            self.allowed.insert((second, direction.opposite(), first));
        }
        self
    }

    #[must_use]
    pub fn allow_same_position_neighbors(self, id: ContentId) -> Self {
        self.allow_all(id, id)
    }

    pub fn build(self) -> Result<ContentCatalog, ContentCatalogError> {
        if self.archetypes.is_empty() {
            return Err(ContentCatalogError::Empty);
        }

        let mut indices = HashMap::with_capacity(self.archetypes.len());
        for (index, archetype) in self.archetypes.iter().enumerate() {
            if archetype.weight == 0 {
                return Err(ContentCatalogError::ZeroWeight(archetype.id));
            }
            if indices.insert(archetype.id, index).is_some() {
                return Err(ContentCatalogError::DuplicateId(archetype.id));
            }
        }

        for &(source, _, neighbor) in &self.allowed {
            if !indices.contains_key(&source) {
                return Err(ContentCatalogError::UnknownId(source));
            }
            if !indices.contains_key(&neighbor) {
                return Err(ContentCatalogError::UnknownId(neighbor));
            }
        }

        Ok(ContentCatalog { archetypes: self.archetypes, indices, allowed: self.allowed })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentCatalogError {
    Empty,
    DuplicateId(ContentId),
    UnknownId(ContentId),
    ZeroWeight(ContentId),
}

impl fmt::Display for ContentCatalogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "content catalog has no archetypes"),
            Self::DuplicateId(id) => write!(formatter, "duplicate content id {}", id.0),
            Self::UnknownId(id) => {
                write!(formatter, "compatibility references unknown id {}", id.0)
            }
            Self::ZeroWeight(id) => write!(formatter, "content id {} has zero weight", id.0),
        }
    }
}

impl std::error::Error for ContentCatalogError {}
