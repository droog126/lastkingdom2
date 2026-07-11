#![allow(unexpected_cfgs)]

pub mod clock;
pub mod combat;
pub mod constant;
pub mod diagnostics;
pub mod ecology;
#[cfg(feature = "experimental-gameplay")]
pub mod equipment;
pub mod legendary;
pub mod match_state;
#[cfg(feature = "experimental-gameplay")]
pub mod mining_site;
pub mod nation;
pub mod objectives;
pub mod player;
pub mod protection;
pub mod protocol;
pub mod pvp;
pub mod resource;
pub mod scenario;
#[cfg(feature = "experimental-gameplay")]
pub mod sovereign_spark;
pub mod status;
#[cfg(feature = "experimental-gameplay")]
pub mod terrain_overlay;
pub mod transport;
pub mod world;

pub mod ai;
pub mod atmosphere;
pub mod hydrology;
pub mod simulation;
