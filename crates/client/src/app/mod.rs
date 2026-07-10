//! Client application boundary for converging online and offline inputs.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NatureRunMode { #[default] Offline, Online }

/// Both modes feed the same synchronization/presentation pipeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NatureInputRoute { pub mode: NatureRunMode }
