use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::rules::apply_damage;

/// Default fixed-tick invulnerability window after a confirmed hit.
pub const CREATURE_HIT_INVULNERABILITY_TICKS: u32 = 8;

#[derive(Component, Reflect, Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Health {
    pub current: f32,
    pub max: f32,
    pub invuln_until_tick: u32,
}

impl Default for Health {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
            invuln_until_tick: 0,
        }
    }
}

impl Health {
    pub fn damage(&mut self, amount: f32, current_tick: u32, invuln_ticks: u32) -> f32 {
        if current_tick < self.invuln_until_tick || !amount.is_finite() {
            return 0.0;
        }
        let actual = amount.max(0.0).min(self.current.max(0.0));
        self.current = apply_damage(self.current, actual);
        self.invuln_until_tick = current_tick.saturating_add(invuln_ticks);
        actual
    }

    pub fn heal(&mut self, amount: f32) -> f32 {
        if !amount.is_finite() {
            return 0.0;
        }
        let before = self.current;
        self.current = (self.current + amount.max(0.0)).min(self.max.max(0.0));
        self.current - before
    }

    pub fn is_dead(&self) -> bool {
        self.current <= 0.0
    }

    pub fn ratio(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct PvpCombatant {
    pub cooldown_remaining: f32,
}

impl PvpCombatant {
    pub fn ready(&self) -> bool {
        self.cooldown_remaining <= 0.0
    }

    pub fn begin_attack(&mut self, weapon: SimpleWeapon) -> bool {
        if !self.ready() {
            return false;
        }
        self.cooldown_remaining = weapon.cooldown_secs;
        true
    }

    pub fn tick(&mut self, delta_secs: f32) {
        self.cooldown_remaining = (self.cooldown_remaining - delta_secs.max(0.0)).max(0.0);
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct SimpleWeapon {
    pub reach: f32,
    pub damage: f32,
    pub knockback: f32,
    pub cooldown_secs: f32,
    pub sweep_angle_deg: f32,
}

impl Default for SimpleWeapon {
    fn default() -> Self {
        Self {
            reach: 3.2,
            damage: 6.0,
            knockback: 4.0,
            cooldown_secs: 0.625,
            sweep_angle_deg: 60.0,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Hitbox {
    pub center_offset: Vec3,
    pub radius: f32,
}

impl Default for Hitbox {
    fn default() -> Self {
        Self {
            center_offset: Vec3::new(0.0, 0.9, 0.0),
            radius: 0.45,
        }
    }
}

#[derive(Resource, Default, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedTick(pub u32);

pub fn increment_fixed_tick(mut tick: ResMut<FixedTick>) {
    tick.0 = tick.0.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combatant_cooldown_has_one_simple_state() {
        let weapon = SimpleWeapon::default();
        let mut combatant = PvpCombatant::default();

        assert!(combatant.begin_attack(weapon));
        assert!(!combatant.begin_attack(weapon));
        combatant.tick(weapon.cooldown_secs);
        assert!(combatant.ready());
    }
}
