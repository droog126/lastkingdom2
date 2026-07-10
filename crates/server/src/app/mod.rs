//! Server integration seam. Registration is performed by the integrator in `main.rs`.

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

