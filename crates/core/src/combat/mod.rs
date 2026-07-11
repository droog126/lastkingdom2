//! Combat rules for the shared simulation.
//!
//! This module groups the Soulslike melee combat state machine into a few
//! focused submodules:
//!
//! - [`state`]    — per-actor components (Stamina, BlockState, ParryWindow,
//!   StunState, Knockback, Downed).
//! - [`attack`]   — attack timing, attack state, and bounded input buffer.
//! - [`events`]   — combat intent and event enums.
//! - [`resolve`]  — pure functions that resolve a swing against a defender.
//! - [`systems`]  — Bevy systems scheduled by [`CombatPlugin`].
//!
//! `Health` lives in [`crate::pvp`] because it is shared with simpler combat
//! paths (PvP combatants, wildlife, bosses). It is re-exported from here for
//! compatibility with code that already used `lk2_core::combat::Health`.

use bevy::prelude::*;

mod attack;
mod events;
mod resolve;
mod state;
mod systems;

#[cfg(test)]
mod tests;

pub use attack::{AttackPhase, AttackState, AttackType, InputBuffer};
pub use events::{CombatEvent, CombatIntent};
pub use resolve::{resolve_hit, sweep_hits};
pub use state::{BlockState, Downed, Knockback, ParryWindow, Stamina, StunSource, StunState};
pub use systems::{
    emit_combat_events_system, process_attack_hits_system, process_combat_intents_system,
    regen_stamina_system, tick_attack_state_system, tick_downed_system, tick_knockback_system,
    tick_parry_window_system, tick_stun_system,
};

pub use crate::pvp::Health;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CombatEvent>();

        app.add_systems(
            FixedUpdate,
            (
                regen_stamina_system,
                tick_parry_window_system,
                tick_stun_system,
                tick_knockback_system,
                tick_downed_system,
                tick_attack_state_system,
                process_combat_intents_system,
                process_attack_hits_system,
                emit_combat_events_system,
            )
                .chain(),
        );
    }
}
