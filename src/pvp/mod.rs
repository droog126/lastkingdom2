//! PvP 模块
//!
//! ```
//! src/
//!   pvp/
//!     mod.rs          — 模块入口 + 武器表
//!     components.rs   — 非网络化组件（CombatState / WeaponStats / Hitbox / Ping / PositionHistory）
//!     los.rs          — DDA 体素视线检测
//!     systems_server.rs — 服务端权威系统
//!     systems_client.rs — 客户端预测系统
//! ```

pub mod components;
pub mod los;
pub mod systems_client;
pub mod systems_server;

pub use components::*;
pub use los::*;
pub use systems_client::*;
pub use systems_server::*;

// ---------------------------------------------------------------------------
// 武器注册表（服务端 + 客户端共用）
// ---------------------------------------------------------------------------

/// 武器 ID
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

/// 武器表
#[derive(Clone, Copy)]
pub struct WeaponEntry {
    pub name: &'static str,
    pub damage: f32,
    pub reach: f32,          // 格
    pub knockback: f32,      // m/s 水平速度
    pub attack_speed: f32,   // 次/秒
    pub sweep_deg: f32,     // 扇形角度
}

const WEAPON_TABLE: &[WeaponEntry] = &[
    // id=0: 空手
    WeaponEntry {
        name: "Fists",
        damage: 1.0,
        reach: 2.5,
        knockback: 0.1,
        attack_speed: 1.4,
        sweep_deg: 60.0,
    },
    // id=1: 木剑
    WeaponEntry {
        name: "Wooden Sword",
        damage: 4.0,
        reach: 3.0,
        knockback: 0.2,
        attack_speed: 1.6,
        sweep_deg: 60.0,
    },
    // id=2: 石剑
    WeaponEntry {
        name: "Stone Sword",
        damage: 5.0,
        reach: 3.0,
        knockback: 0.25,
        attack_speed: 1.6,
        sweep_deg: 60.0,
    },
    // id=3: 铁剑（竞技标准）
    WeaponEntry {
        name: "Iron Sword",
        damage: 6.0,
        reach: 3.2,
        knockback: 0.4,
        attack_speed: 1.6,
        sweep_deg: 60.0,
    },
    // id=4: 钻石剑
    WeaponEntry {
        name: "Diamond Sword",
        damage: 7.0,
        reach: 3.2,
        knockback: 0.4,
        attack_speed: 1.6,
        sweep_deg: 60.0,
    },
    // id=5: 金剑
    WeaponEntry {
        name: "Gold Sword",
        damage: 4.0,
        reach: 3.2,
        knockback: 0.4,
        attack_speed: 2.0, // 攻速最快
        sweep_deg: 60.0,
    },
];

impl WeaponId {
    pub fn stats(&self) -> WeaponEntry {
        WEAPON_TABLE[*self as usize].clone()
    }
}

// ---------------------------------------------------------------------------
// 插件
// ---------------------------------------------------------------------------
use bevy::prelude::*;

use lk2_core::protocol::components::CombatReady;
use lk2_core::protocol::messages::{AttackInput, DamageResult, HitConfirm, KnockbackEvent};

/// 当前 fixed-update tick 计数器（每个 FixedUpdate 递增）。
/// 用于跨系统对齐 tick 值（位置历史 / 攻击输入 / 伤害事件 / 预测）
/// bevy 0.18 移除了 `FixedTick` 资源，这里手动维护一份。
#[derive(Resource, Default)]
pub struct FixedTick(pub u32);

fn increment_fixed_tick(mut tick: ResMut<FixedTick>) {
    tick.0 = tick.0.wrapping_add(1);
}

pub struct PvPPlugin;

impl Plugin for PvPPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FixedTick>()
            .add_message::<AttackInput>()
            .add_message::<HitConfirm>()
            .add_message::<KnockbackEvent>()
            .add_message::<DamageResult>()
            .add_message::<DamageEvent>()
            .add_message::<VisualEffectEvent>()
            .add_systems(
                FixedUpdate,
                (
                    increment_fixed_tick,
                    systems_server::record_position_history,
                    systems_server::read_attack_inputs,
                    systems_server::melee_hit_registration,
                    systems_server::apply_damage_and_knockback,
                    systems_server::expire_knockback_immunity,
                    systems_server::tick_combat_cooldowns,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    systems_client::collect_local_input,
                    systems_client::client_attack_predict,
                    systems_client::on_hit_confirm,
                    systems_client::on_knockback_event,
                    systems_client::on_damage_result,
                    systems_client::trigger_visual_effects,
                )
                    .chain(),
            );
    }
}
