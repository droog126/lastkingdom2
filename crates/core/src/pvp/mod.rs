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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combat_state_default_is_idle() {
        let c = CombatState::default();
        assert!(!c.is_attacking);
        assert_eq!(c.attack_cooldown_timer, 0.0);
        assert_eq!(c.combo_count, 0);
        assert_eq!(c.last_attack_tick, 0);
    }

    #[test]
    fn weapon_stats_cooldown_matches_speed() {
        let w = WeaponStats {
            reach: 3.0,
            damage: 5.0,
            knockback: 0.3,
            attack_speed: 2.0,
            sweep_angle_deg: 60.0,
            sweep_range: 3.0,
        };
        assert!((w.cooldown_secs() - 0.5).abs() < 0.001);

        let w2 = WeaponStats { attack_speed: 1.0, ..w };
        assert!((w2.cooldown_secs() - 1.0).abs() < 0.001);
    }

    #[test]
    fn hitbox_default_has_reasonable_size() {
        let h = Hitbox::default();
        assert_eq!(h.half_extents.x, 0.3);
        assert_eq!(h.half_extents.y, 0.9);
        assert_eq!(h.half_extents.z, 0.3);
        assert_eq!(h.offset.y, 0.9);
    }

    #[test]
    fn position_history_default_size_60() {
        let ph = PositionHistory::default();
        assert_eq!(ph.max_size, 60);
        assert_eq!(ph.snapshots.len(), 0);
    }

    #[test]
    fn position_history_push_and_query() {
        let mut ph = PositionHistory::new(5);

        for i in 0..3 {
            ph.push(PositionSnapshot {
                tick: (i + 1) * 10,
                translation: Vec3::new(i as f32, 0.0, 0.0),
                rotation: Quat::IDENTITY,
                velocity: Vec3::ZERO,
            });
        }

        assert_eq!(ph.snapshots.len(), 3);

        let snap = ph.query(15).unwrap();
        assert_eq!(snap.tick, 10);
        assert!((snap.translation.x - 0.0).abs() < 0.001);

        assert!(ph.query(5).is_none());

        let latest = ph.query(100).unwrap();
        assert_eq!(latest.tick, 30);
    }

    #[test]
    fn position_history_evicts_old_when_full() {
        let mut ph = PositionHistory::new(3);

        for i in 0..5 {
            ph.push(PositionSnapshot {
                tick: i,
                translation: Vec3::ZERO,
                rotation: Quat::IDENTITY,
                velocity: Vec3::ZERO,
            });
        }

        assert_eq!(ph.snapshots.len(), 3);
        assert_eq!(ph.oldest_tick(), Some(2));
        assert_eq!(ph.snapshots.back().unwrap().tick, 4);
    }

    #[test]
    fn position_history_oldest_tick() {
        let mut ph = PositionHistory::new(10);
        assert_eq!(ph.oldest_tick(), None);

        ph.push(PositionSnapshot {
            tick: 42,
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            velocity: Vec3::ZERO,
        });
        assert_eq!(ph.oldest_tick(), Some(42));
    }

    #[test]
    fn weapon_id_from_u8_known_values() {
        assert_eq!(WeaponId::from_u8(0), Some(WeaponId::Fists));
        assert_eq!(WeaponId::from_u8(1), Some(WeaponId::WoodenSword));
        assert_eq!(WeaponId::from_u8(2), Some(WeaponId::StoneSword));
        assert_eq!(WeaponId::from_u8(3), Some(WeaponId::IronSword));
        assert_eq!(WeaponId::from_u8(4), Some(WeaponId::DiamondSword));
        assert_eq!(WeaponId::from_u8(5), Some(WeaponId::GoldSword));
    }

    #[test]
    fn weapon_id_from_u8_out_of_range() {
        assert_eq!(WeaponId::from_u8(6), None);
        assert_eq!(WeaponId::from_u8(255), None);
    }

    #[test]
    fn weapon_stats_match_weapon_table() {
        let fists = WeaponId::Fists.stats();
        assert_eq!(fists.name, "Fists");
        assert!((fists.damage - 1.0).abs() < 0.001);
        assert!((fists.reach - 2.5).abs() < 0.001);

        let iron = WeaponId::IronSword.stats();
        assert_eq!(iron.name, "Iron Sword");
        assert!((iron.damage - 6.0).abs() < 0.001);
        assert!((iron.attack_speed - 1.6).abs() < 0.001);

        let gold = WeaponId::GoldSword.stats();
        assert!((gold.attack_speed - 2.0).abs() < 0.001);
    }

    #[test]
    fn fixed_tick_increment_wraps() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.insert_resource(FixedTick(0));
        app.add_systems(Update, increment_fixed_tick);
        app.update();
        assert_eq!(app.world().resource::<FixedTick>().0, 1);
    }

    #[test]
    fn fixed_tick_wraps_at_max() {
        use bevy::prelude::*;
        let mut app = App::new();
        app.insert_resource(FixedTick(u32::MAX));
        app.add_systems(Update, increment_fixed_tick);
        app.update();
        assert_eq!(app.world().resource::<FixedTick>().0, 0);
    }

    #[test]
    fn ping_wraps_value() {
        let p = Ping(42.5);
        assert_eq!(p.0, 42.5);
    }

    #[test]
    fn all_six_weapons_have_stats() {
        for i in 0..6u8 {
            let wid = WeaponId::from_u8(i).expect(&format!("weapon {} should exist", i));
            let stats = wid.stats();
            assert!(
                stats.damage > 0.0,
                "weapon {} should have positive damage",
                i
            );
            assert!(stats.reach > 0.0, "weapon {} should have positive reach", i);
            assert!(
                stats.attack_speed > 0.0,
                "weapon {} should have positive attack speed",
                i
            );
        }
    }
}
