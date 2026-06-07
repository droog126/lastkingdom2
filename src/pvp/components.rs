//! PvP 核心组件
//!
//! 非网络化组件（仅本地或服务端持有）:
//! - `CombatState`        — 攻击状态机
//! - `WeaponStats`        — 武器属性（不网络复制，只在 entity spawn 时设置）
//! - `Hitbox`             — 命中框（AABB）
//! - `Ping`               — 客户端延迟
//! - `PositionHistory`    — 服务端位置快照缓冲（用于延迟补偿）
//!
//! 网络化组件见 `crate::network::protocols::components`

use bevy::prelude::*;
use std::collections::VecDeque;

// ---------------------------------------------------------------------------
// 攻击状态
// ---------------------------------------------------------------------------

/// 战斗状态机
#[derive(Component, Clone, Debug)]
pub struct CombatState {
    pub is_attacking: bool,
    pub attack_cooldown_timer: f32, // 剩余冷却秒
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

// ---------------------------------------------------------------------------
// 武器属性（服务端 + 客户端各自持有，不跨网络传输）
// ---------------------------------------------------------------------------

/// 武器属性（剑 / 斧 / 锄）
#[derive(Component, Clone, Copy, Debug)]
pub struct WeaponStats {
    pub reach: f32,             // 攻击距离（格）
    pub damage: f32,            // 基础伤害
    pub knockback: f32,         // 水平击退力度
    pub attack_speed: f32,      // 每秒攻击次数（冷却 = 1/speed）
    pub sweep_angle_deg: f32,   // 扇形角度（度）
    pub sweep_range: f32,      // 横扫半径（与 reach 共同决定命中体积）
}

impl WeaponStats {
    pub fn cooldown_secs(&self) -> f32 {
        1.0 / self.attack_speed
    }
}

// ---------------------------------------------------------------------------
// 命中框
// ---------------------------------------------------------------------------

/// 玩家 / 生物的 AABB 命中框
#[derive(Component, Clone, Copy, Debug)]
pub struct Hitbox {
    /// 半尺寸（从中心到表面）
    pub half_extents: Vec3,
    /// 相对于 Transform.translation 的偏移（站立时中心在腰间，所以向上偏移）
    pub offset: Vec3,
}

impl Default for Hitbox {
    fn default() -> Self {
        Self {
            half_extents: Vec3::new(0.3, 0.9, 0.3),
            offset: Vec3::new(0.0, 0.9, 0.0), // 中心在胸口高度
        }
    }
}

// ---------------------------------------------------------------------------
// 网络延迟
// ---------------------------------------------------------------------------

/// 玩家 ping（服务端记录，单位 ms）
#[derive(Component, Clone, Copy, Debug)]
pub struct Ping(pub f32);

// ---------------------------------------------------------------------------
// 位置历史（服务端回滚用）
// ---------------------------------------------------------------------------

/// 服务端保留的位置快照缓冲
#[derive(Component, Clone, Debug)]
pub struct PositionHistory {
    pub snapshots: VecDeque<PositionSnapshot>,
    pub max_size: usize,
}

impl Default for PositionHistory {
    fn default() -> Self {
        Self::new(60) // 默认保留 2 秒 @30TPS
    }
}

impl PositionHistory {
    pub fn new(max_size: usize) -> Self {
        Self {
            snapshots: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn push(&mut self, snap: PositionSnapshot) {
        if self.snapshots.len() >= self.max_size {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(snap);
    }

    /// 找到 <= target_tick 的最近快照（线性搜索，snapshot 数量少所以没问题）
    pub fn query(&self, target_tick: u32) -> Option<PositionSnapshot> {
        self.snapshots.iter().rev().find(|s| s.tick <= target_tick).cloned()
    }

    /// 返回最老快照的 tick（用于丢弃过期数据）
    pub fn oldest_tick(&self) -> Option<u32> {
        self.snapshots.front().map(|s| s.tick)
    }
}

/// 单一时刻的位置快照
#[derive(Clone, Copy, Debug)]
pub struct PositionSnapshot {
    pub tick: u32,
    pub translation: Vec3,
    pub rotation: Quat,
    pub velocity: Vec3,
}

// ---------------------------------------------------------------------------
// 伤害事件（用于系统间通信）
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// 视觉特效事件（客户端专用）
// ---------------------------------------------------------------------------

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
