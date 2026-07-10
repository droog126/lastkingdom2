//! Plant visual descriptors. No client-side spawning or growth decisions live here.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlantVisual { pub id: u32, pub kind: u8, pub stock: u32 }
