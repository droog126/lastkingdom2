//! Attack state machine.
//!
//! Tracks the currently active attack through Windup → Active → Recovery and
//! the bounded input buffer that hands intents to the combat systems.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::events::CombatIntent;
use super::state::{Downed, Stamina, StunState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackType {
    Light,

    Thrust,

    Heavy,
}

impl AttackType {
    pub fn stamina_cost(self) -> f32 {
        match self {
            Self::Light => 6.0,
            Self::Thrust => 7.0,
            Self::Heavy => 12.0,
        }
    }

    pub fn damage_multiplier(self) -> f32 {
        match self {
            Self::Light => 1.0,
            Self::Thrust => 0.9,
            Self::Heavy => 1.8,
        }
    }

    pub fn knockback_strength(self) -> f32 {
        match self {
            Self::Light => 0.15,
            Self::Thrust => 0.05,
            Self::Heavy => 0.8,
        }
    }

    pub fn cooldown_secs(self) -> f32 {
        match self {
            Self::Light => 0.40,
            Self::Thrust => 0.55,
            Self::Heavy => 1.10,
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Light => "轻击",
            Self::Thrust => "突刺",
            Self::Heavy => "重击",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttackPhase {
    Windup,
    Active,
    Recovery,
}

#[derive(Debug, Clone, Copy)]
pub struct ActiveAttack {
    pub kind: AttackType,
    pub phase: AttackPhase,
    pub phase_timer: f32,

    pub phase_secs: f32,

    pub hit_apps: u8,
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct AttackState {
    pub current: Option<ActiveAttack>,
}

impl AttackState {
    pub fn try_start(
        &mut self,
        kind: AttackType,
        stamina: &Stamina,
        stun: &StunState,
        downed: &Downed,
    ) -> bool {
        if self.current.is_some() {
            return false;
        }
        if !stun.can_act() || !downed.can_act() {
            return false;
        }
        if !stamina.has_enough(kind.stamina_cost()) {
            return false;
        }
        let (phase, phase_secs) = match kind {
            AttackType::Light => (AttackPhase::Windup, 0.06),
            AttackType::Thrust => (AttackPhase::Windup, 0.10),
            AttackType::Heavy => (AttackPhase::Windup, 0.18),
        };
        self.current = Some(ActiveAttack {
            kind,
            phase,
            phase_timer: phase_secs,
            phase_secs,
            hit_apps: 0,
        });
        true
    }

    pub fn tick(&mut self, dt: f32) -> bool {
        let Some(att) = self.current.as_mut() else {
            return false;
        };
        att.phase_timer -= dt;
        if att.phase_timer > 0.0 {
            return false;
        }

        match att.phase {
            AttackPhase::Windup => {
                att.phase = AttackPhase::Active;
                let active_secs = match att.kind {
                    AttackType::Light => 0.10,
                    AttackType::Thrust => 0.18,
                    AttackType::Heavy => 0.22,
                };
                att.phase_timer = active_secs;
                att.phase_secs = active_secs;
                att.hit_apps = 0;
            }
            AttackPhase::Active => {
                att.phase = AttackPhase::Recovery;
                let recovery_secs = match att.kind {
                    AttackType::Light => 0.12,
                    AttackType::Thrust => 0.20,
                    AttackType::Heavy => 0.45,
                };
                att.phase_timer = recovery_secs;
                att.phase_secs = recovery_secs;
            }
            AttackPhase::Recovery => {
                self.current = None;
                return true;
            }
        }
        false
    }

    pub fn is_active(&self) -> bool {
        matches!(self.current, Some(a) if a.phase == AttackPhase::Active)
    }

    pub fn current_kind(&self) -> Option<AttackType> {
        self.current.map(|a| a.kind)
    }

    pub fn phase_progress(&self) -> f32 {
        match self.current {
            Some(a) if a.phase_secs > 0.0 => 1.0 - (a.phase_timer / a.phase_secs).clamp(0.0, 1.0),
            _ => 0.0,
        }
    }
}

#[derive(Component, Debug, Clone, Default)]
pub struct InputBuffer {
    pub queue: Vec<(CombatIntent, u32)>,

    pub window_secs: f32,

    pub window_ticks: u32,
}

impl InputBuffer {
    pub fn new(window_secs: f32, tick_rate: u32) -> Self {
        Self {
            queue: Vec::with_capacity(8),
            window_secs,
            window_ticks: (window_secs * tick_rate as f32) as u32,
        }
    }

    pub fn push(&mut self, intent: CombatIntent, current_tick: u32) {
        if self.queue.len() >= 8 {
            self.queue.remove(0);
        }
        self.queue.push((intent, current_tick));
    }

    pub fn drain_fresh(&mut self, current_tick: u32) -> Vec<CombatIntent> {
        let window = self.window_ticks;
        self.queue
            .retain(|(_, t)| current_tick.saturating_sub(*t) <= window);
        self.queue.drain(..).map(|(i, _)| i).collect()
    }

    pub fn clear(&mut self) {
        self.queue.clear();
    }
}
