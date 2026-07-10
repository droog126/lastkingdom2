//! Client assembly for the shared natural-world presentation path.

use bevy::prelude::*;
use lk2_core::protocol::components::EcoSnapshot;

use crate::presentation::NaturePresentationPlugin;
use crate::synchronization::{NatureSnapshotBuffer, SnapshotAcceptance};

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NatureRunMode {
    #[default]
    Offline,
    Online,
}

/// Installs one snapshot-to-presentation path for both authority modes.
pub struct NatureClientPlugin {
    pub mode: NatureRunMode,
}

impl Plugin for NatureClientPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(self.mode).add_plugins(NaturePresentationPlugin);
    }
}

/// Transport adapters call this function after decoding either an online replicated snapshot or
/// an offline in-process authority snapshot.
pub fn submit_authoritative_snapshot(
    buffer: &mut NatureSnapshotBuffer,
    snapshot: EcoSnapshot,
) -> SnapshotAcceptance {
    buffer.push(snapshot)
}
