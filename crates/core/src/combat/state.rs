//! Combat state components attached to combatants.
//!
//! These components track the per-actor state that the combat rules read and
//! mutate: stamina, blocking, parry windows, stun, knockback, and downed. They
//! hold no systems themselves — see [`crate::combat::systems`] for the
//! per-tick systems that drive them.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Component, Debug, Clone, Copy)]
pub struct Stamina {
    pub current: f32,

    pub max: f32,

    pub regen_per_sec: f32,
}

impl Default for Stamina {
    fn default() -> Self {
        Self {
            current: 100.0,
            max: 100.0,
            regen_per_sec: 18.0,
        }
    }
}

impl Stamina {
    pub fn consume(&mut self, amount: f32) -> f32 {
        let before = self.current;
        self.current = (self.current - amount).max(0.0);
        before - self.current
    }

    pub fn regen(&mut self, dt: f32) {
        self.current = (self.current + self.regen_per_sec * dt).min(self.max);
    }

    pub fn has_enough(&self, cost: f32) -> bool {
        self.current >= cost
    }

    pub fn ratio(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct BlockState {
    pub blocking: bool,

    pub drain_per_sec: f32,

    pub damage_reduction: f32,

    pub per_hit_cost: f32,
}

impl Default for BlockState {
    fn default() -> Self {
        Self {
            blocking: false,
            drain_per_sec: 6.0,
            damage_reduction: 0.45,
            per_hit_cost: 4.0,
        }
    }
}

impl BlockState {
    pub fn start(&mut self) {
        self.blocking = true;
    }

    pub fn stop(&mut self) {
        self.blocking = false;
    }

    pub fn drain(&self, sta: &mut Stamina, dt: f32) {
        if self.blocking {
            sta.consume(self.drain_per_sec * dt);
        }
    }

    pub fn apply_hit(&mut self, raw_damage: f32, sta: &mut Stamina) -> f32 {
        if !self.blocking {
            return raw_damage;
        }

        let actual_cost = self.per_hit_cost.min(sta.current);
        sta.consume(actual_cost);
        if sta.current <= 0.0 {
            self.blocking = false;
            return raw_damage;
        }
        raw_damage * (1.0 - self.damage_reduction)
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct ParryWindow {
    pub parry_active: bool,

    pub parry_timer: f32,

    pub parry_window_secs: f32,

    pub riposte_window_secs: f32,
}

impl Default for ParryWindow {
    fn default() -> Self {
        Self {
            parry_active: false,
            parry_timer: 0.0,
            parry_window_secs: 0.16,
            riposte_window_secs: 0.40,
        }
    }
}

impl ParryWindow {
    pub fn begin(&mut self) {
        self.parry_active = true;
        self.parry_timer = self.parry_window_secs;
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.parry_active {
            return false;
        }
        self.parry_timer -= dt;
        if self.parry_timer <= 0.0 {
            self.parry_active = false;
            self.parry_timer = 0.0;
            return true;
        }
        false
    }

    pub fn try_parry(&self) -> bool {
        self.parry_active
    }

    pub fn consume_on_success(&mut self) -> bool {
        if self.parry_active {
            self.parry_active = false;
            self.parry_timer = 0.0;

            true
        } else {
            false
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct StunState {
    pub stunned: bool,
    pub stun_timer: f32,

    pub source: StunSource,
}

impl Default for StunState {
    fn default() -> Self {
        Self {
            stunned: false,
            stun_timer: 0.0,
            source: StunSource::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StunSource {
    None,

    Parried,

    HeavyHit,

    Exhausted,
}

impl StunState {
    pub fn apply(&mut self, secs: f32, source: StunSource) {
        self.stunned = true;
        self.stun_timer = secs;
        self.source = source;
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.stunned {
            return false;
        }
        self.stun_timer -= dt;
        if self.stun_timer <= 0.0 {
            self.stunned = false;
            self.stun_timer = 0.0;
            self.source = StunSource::None;
            return true;
        }
        false
    }

    pub fn can_act(&self) -> bool {
        !self.stunned
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Knockback {
    pub direction: Vec3,

    pub magnitude: f32,

    pub remaining_secs: f32,

    pub total_secs: f32,
}

impl Default for Knockback {
    fn default() -> Self {
        Self {
            direction: Vec3::ZERO,
            magnitude: 0.0,
            remaining_secs: 0.0,
            total_secs: 0.0,
        }
    }
}

impl Knockback {
    pub fn apply(&mut self, direction: Vec3, magnitude: f32, duration_secs: f32) {
        self.direction = direction.normalize_or_zero();
        self.magnitude = magnitude;
        self.remaining_secs = duration_secs;
        self.total_secs = duration_secs;
    }

    pub fn tick(&mut self, dt: f32) -> Vec3 {
        if self.remaining_secs <= 0.0 {
            return Vec3::ZERO;
        }
        self.remaining_secs -= dt;
        if self.remaining_secs < 0.0 {
            self.remaining_secs = 0.0;
            return Vec3::ZERO;
        }
        self.direction * self.magnitude
    }

    pub fn is_active(&self) -> bool {
        self.remaining_secs > 0.0
    }

    pub fn progress(&self) -> f32 {
        if self.total_secs <= 0.0 {
            0.0
        } else {
            (self.remaining_secs / self.total_secs).clamp(0.0, 1.0)
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Downed {
    pub downed: bool,

    pub timer: f32,

    pub total_secs: f32,

    pub revive_hp_ratio: f32,
}

impl Default for Downed {
    fn default() -> Self {
        Self {
            downed: false,
            timer: 0.0,
            total_secs: 8.0,
            revive_hp_ratio: 0.5,
        }
    }
}

impl Downed {
    pub fn knockdown(&mut self, total_secs: f32) {
        self.downed = true;
        self.timer = total_secs;
        self.total_secs = total_secs;
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.downed {
            return false;
        }
        self.timer -= dt;
        if self.timer <= 0.0 {
            self.downed = false;
            self.timer = 0.0;
            return true;
        }
        false
    }

    pub fn progress(&self) -> f32 {
        if !self.downed || self.total_secs <= 0.0 {
            0.0
        } else {
            (self.timer / self.total_secs).clamp(0.0, 1.0)
        }
    }

    pub fn can_act(&self) -> bool {
        !self.downed
    }
}
