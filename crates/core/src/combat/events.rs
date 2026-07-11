//! Combat event and intent types.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::attack::AttackType;
use super::state::StunSource;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CombatIntent {
    Attack(AttackType),

    BlockStart,

    BlockEnd,

    ParryAttempt,
}

#[derive(Message, Debug, Clone)]
pub enum CombatEvent {
    DamageDealt {
        attacker: Entity,
        victim: Entity,
        attack: AttackType,
        damage: f32,
    },

    Blocked {
        attacker: Entity,
        defender: Entity,
    },

    Parried {
        attacker: Entity,
        defender: Entity,
    },

    Stunned {
        entity: Entity,
        source: StunSource,
        duration_secs: f32,
    },

    Knockback {
        victim: Entity,
        magnitude: f32,
    },
}
