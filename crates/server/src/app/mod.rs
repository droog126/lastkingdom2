//! Server integration seam. Registration is performed by the integrator in `main.rs`.

use bevy::prelude::*;

use super::authority::NatureAuthorityPlugin;
use super::authority::{LatestNatureReport, NatureAuthoritySet};
use super::observation::NatureObservationPlugin;
use super::persistence::NaturePersistencePlugin;
use super::replication::NatureReplicationPlugin;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ServerModuleRegistration {
    pub authority: bool,
    pub replication: bool,
    pub observation: bool,
    pub persistence: bool,
}

pub const NATURE_SERVER_MODULES: ServerModuleRegistration = ServerModuleRegistration {
    authority: true,
    replication: true,
    observation: true,
    persistence: true,
};

pub struct NatureServerPlugin;

impl Plugin for NatureServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            NatureAuthorityPlugin,
            NatureReplicationPlugin,
            NatureObservationPlugin,
            NaturePersistencePlugin,
        ));
    }
}

/// Projects the server's primary `NatureRegionWorld` report without starting
/// the standalone authority adapter used by focused tests.
pub struct NatureServerProjectionPlugin;

impl Plugin for NatureServerProjectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LatestNatureReport>()
            .configure_sets(
                FixedUpdate,
                (
                    NatureAuthoritySet::StepWorld,
                    NatureAuthoritySet::PublishReport,
                )
                    .chain(),
            )
            .add_plugins((
                NatureReplicationPlugin,
                NatureObservationPlugin,
                NaturePersistencePlugin,
            ));
    }
}
