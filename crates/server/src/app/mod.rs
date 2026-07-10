//! Server integration seam. Registration is performed by the integrator in `main.rs`.

use bevy::prelude::*;

use super::authority::NatureAuthorityPlugin;
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
