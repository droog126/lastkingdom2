use bevy::prelude::*;
use std::collections::VecDeque;

#[derive(Component, Clone, Debug)]
pub struct CombatState {
    pub is_attacking: bool,
    pub attack_cooldown_timer: f32,
    pub combo_count: u8,
    pub last_attack_tick: u32,
}

impl Default for CombatState {
    fn default() -> Self {
        Self {
            is_attacking: false,
            attack_cooldown_timer: 0.0,
            combo_count: 0,
            last_attack_tick: 0,
        }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct WeaponStats {
    pub reach: f32,
    pub damage: f32,
    pub knockback: f32,
    pub attack_speed: f32,
    pub sweep_angle_deg: f32,
    pub sweep_range: f32,
}

impl WeaponStats {
    pub fn cooldown_secs(&self) -> f32 {
        1.0 / self.attack_speed
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Hitbox {
    pub half_extents: Vec3,

    pub offset: Vec3,
}

impl Default for Hitbox {
    fn default() -> Self {
        Self { half_extents: Vec3::new(0.3, 0.9, 0.3), offset: Vec3::new(0.0, 0.9, 0.0) }
    }
}

#[derive(Component, Clone, Copy, Debug)]
pub struct Ping(pub f32);

#[derive(Component, Clone, Debug)]
pub struct PositionHistory {
    pub snapshots: VecDeque<PositionSnapshot>,
    pub max_size: usize,
}

impl Default for PositionHistory {
    fn default() -> Self {
        Self::new(60)
    }
}

impl PositionHistory {
    pub fn new(max_size: usize) -> Self {
        Self { snapshots: VecDeque::with_capacity(max_size), max_size }
    }

    pub fn push(&mut self, snap: PositionSnapshot) {
        if self.snapshots.len() >= self.max_size {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(snap);
    }

    pub fn query(&self, target_tick: u32) -> Option<PositionSnapshot> {
        self.snapshots.iter().rev().find(|s| s.tick <= target_tick).cloned()
    }

    pub fn oldest_tick(&self) -> Option<u32> {
        self.snapshots.front().map(|s| s.tick)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PositionSnapshot {
    pub tick: u32,
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
}

#[derive(Message, Clone, Debug)]
pub struct DamageEvent {
    pub attacker: Entity,
    pub victim: Entity,
    pub damage: f32,
    pub knockback: Vec3,
    pub is_critical: bool,
    pub hit_location: Vec3,
    pub server_tick: u32,
}

#[derive(Message, Clone, Debug)]
pub enum VisualEffectEvent {
    SwingSword,
    Hit {
        target: Entity,
        damage: f32,
        is_critical: bool,
        hit_pos: Vec3,
    },
    CriticalHit {
        target: Entity,
        damage: f32,
        hit_pos: Vec3,
    },
    KnockbackApplied {
        target: Entity,
        velocity: Vec3,
    },
    ScreenShake,
}

#[derive(Resource, Default)]
pub struct FixedTick(pub u32);

pub fn increment_fixed_tick(mut tick: ResMut<FixedTick>) {
    tick.0 = tick.0.wrapping_add(1);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WeaponId {
    Fists = 0,
    WoodenSword = 1,
    StoneSword = 2,
    IronSword = 3,
    DiamondSword = 4,
    GoldSword = 5,
}

impl WeaponId {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Fists),
            1 => Some(Self::WoodenSword),
            2 => Some(Self::StoneSword),
            3 => Some(Self::IronSword),
            4 => Some(Self::DiamondSword),
            5 => Some(Self::GoldSword),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WeaponEntry {
    pub name: &'static str,
    pub damage: f32,
    pub reach: f32,
    pub knockback: f32,
    pub attack_speed: f32,
    pub sweep_deg: f32,
}

const WEAPON_TABLE: &[WeaponEntry] = &[
    WeaponEntry {
        name: "Fists",
        damage: 1.0,
        reach: 2.5,
        knockback: 0.1,
        attack_speed: 1.4,
        sweep_deg: 60.0,
    },
    WeaponEntry {
        name: "Wooden Sword",
        damage: 4.0,
        reach: 3.0,
        knockback: 0.2,
        attack_speed: 1.6,
        sweep_deg: 60.0,
    },
    WeaponEntry {
        name: "Stone Sword",
        damage: 5.0,
        reach: 3.0,
        knockback: 0.25,
        attack_speed: 1.6,
        sweep_deg: 60.0,
    },
    WeaponEntry {
        name: "Iron Sword",
        damage: 6.0,
        reach: 3.2,
        knockback: 0.4,
        attack_speed: 1.6,
        sweep_deg: 60.0,
    },
    WeaponEntry {
        name: "Diamond Sword",
        damage: 7.0,
        reach: 3.2,
        knockback: 0.4,
        attack_speed: 1.6,
        sweep_deg: 60.0,
    },
    WeaponEntry {
        name: "Gold Sword",
        damage: 4.0,
        reach: 3.2,
        knockback: 0.4,
        attack_speed: 2.0,
        sweep_deg: 60.0,
    },
];

impl WeaponId {
    pub fn stats(&self) -> WeaponEntry {
        WEAPON_TABLE[*self as usize].clone()
    }
}

pub mod components {

    pub use super::{
        CombatState, DamageEvent, FixedTick, Hitbox, Ping, PositionHistory, PositionSnapshot,
        VisualEffectEvent, WeaponEntry, WeaponId, WeaponStats,
    };
}
