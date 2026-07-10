//! Sky visual descriptors sourced from authoritative cloud state.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CloudVisual { pub id: u32, pub rain: f32, pub phase: f32 }
