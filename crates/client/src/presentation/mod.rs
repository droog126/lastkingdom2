//! Pure mapping boundary from authoritative natural-world snapshots to visuals.

pub mod animals;
pub mod plants;
pub mod sky;
pub mod weather;

use crate::synchronization::NaturePresentationInput;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationCounts {
    pub clouds: usize,
    pub plants: usize,
    pub animals: usize,
}

#[must_use]
pub fn counts(input: NaturePresentationInput<'_>) -> PresentationCounts {
    PresentationCounts { clouds: input.clouds.len(), plants: input.plants.len(), animals: input.animals.len() }
}
