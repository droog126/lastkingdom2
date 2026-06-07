//! lightyear 网络协议定义
//!
//! 定义所有跨网络的 message 和 replication 组件。
//! protocol! 宏生成 MessageKind / ComponentKind 枚举和序列化代码。

use bevy::prelude::*;
use leafwing_input_manager::Actionlike;
use lightyear::prelude::*;

// ---------------------------------------------------------------------------
// 协议 ID（同一 server 可以服务多个独立"房间" / 游戏实例）
// ---------------------------------------------------------------------------

pub mod protocols {
    use bevy::prelude::*;
    use lightyear::prelude::PeerId;
    use serde::{Deserialize, Serialize};

    // lightyear 0.22 不在 prelude 暴露 `ProtocolId`（要 `lightyear::prelude::Tick` 也没，但本文件里其实只需要
    // 协议唯一标识做占位用）。定义本地 newtype 即可满足 `PROTOCOL_ID` 常量声明。
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct ProtocolId(pub u128);
    impl ProtocolId {
        pub(crate) const fn from_u128(v: u128) -> Self { Self(v) }
        pub(crate) const fn to_u128(self) -> u128 { self.0 }
    }

    pub(crate) const PROTOCOL_ID: ProtocolId = ProtocolId::from_u128(0x_4c_4b_32_50_56_50_0001); // "LKPVP" v1

    // 通道 — lightyear 0.22 移除了 `ChannelId` newtype（现在用 TypeId 化的 `ChannelKind`）。
    // 单机 demo 只需要占位 ID；这里定义本地 newtype 满足常量声明。
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct ChannelId(pub u8);
    impl ChannelId {
        pub(crate) const fn new(v: u8) -> Self { Self(v) }
    }

    pub(crate) const INPUT_CHANNEL: ChannelId = ChannelId::new(0);
    pub(crate) const UNRELIABLE_VISUALS: ChannelId = ChannelId::new(1); // HitConfirm / Knockback
    pub(crate) const RELIABLE_EVENTS: ChannelId = ChannelId::new(2);     // DamageResult

    // ---------------------------------------------------------------------------
    // Message 协议
    // ---------------------------------------------------------------------------

    pub(crate) mod msg {
        use super::*;

        // 客户端 → 服务端：攻击输入
        #[derive(Message, Clone, Debug, PartialEq)]
        pub struct AttackInput {
            pub tick: u32,
            pub input_dir: Vec3, // 攻击方向（玩家朝向）
            pub is_falling: bool,
            pub combo_count: u8,
        }

        // 服务端 → 客户端：命中确认（即时视觉反馈，不影响逻辑）
        #[derive(Message, Clone, Debug)]
        pub struct HitConfirm {
            pub victim_id: PeerId,
            pub damage: f32,
            pub is_critical: bool,
            pub hit_pos: Vec3,
            pub server_tick: u32,
        }

        // 服务端 → 客户端：击退指令（即时覆盖）
        #[derive(Message, Clone, Debug)]
        pub struct KnockbackEvent {
            pub victim_id: PeerId,
            pub velocity: Vec3,
            pub server_tick: u32,
        }

        // 服务端 → 客户端：伤害结果（可靠，血量最终一致）
        #[derive(Message, Clone, Debug)]
        pub struct DamageResult {
            pub victim_id: PeerId,
            pub new_health: f32,
            pub is_dead: bool,
            pub server_tick: u32,
        }

        // 服务端 → 客户端：击杀播报
        #[derive(Message, Clone, Debug)]
        pub struct KillFeedEntry {
            pub killer_name: String,
            pub victim_name: String,
            pub weapon_id: u8,
        }
    }

        // ---------------------------------------------------------------------------
        // Replicate 组件（服务端复制到客户端）
        // ---------------------------------------------------------------------------

        pub(crate) mod components {
            use super::*;

            /// 玩家血量
            // `Component` derive 已经自动实现 `Bundle`，所以不能再 derive `Bundle`（会冲突）。
            #[derive(Component, Message, Clone, Debug, Reflect)]
            #[component(storage = "SparseSet")]
            pub struct Health(pub f32);

        /// 武器属性
        #[derive(Component, Clone, Debug, Reflect, Serialize, Deserialize)]
        pub struct WeaponStatsRaw {
            pub reach: f32,
            pub damage: f32,
            pub knockback: f32,
            pub attack_speed: f32,
            pub sweep_angle_deg: f32,
        }

        /// 武器实例（挂在玩家 entity 上，指向 WeaponStatsRaw）
        #[derive(Component, Clone, Debug, Reflect, Serialize, Deserialize)]
        pub struct EquippedWeapon(pub u8); // weapon_id

        /// 玩家已装备武器的完整属性（从 WeaponRegistry 查表）
        #[derive(Component, Clone, Debug, Reflect, Serialize, Deserialize)]
        pub struct CombatReady {
            pub weapon_id: u8,
            pub reach: f32,
            pub damage: f32,
            pub knockback: f32,
            pub attack_speed: f32,    // 次/秒
            pub sweep_angle_deg: f32,
            pub attack_cooldown: f32, // 剩余冷却秒
        }

        /// 击退免疫（被攻击后短暂无敌，防止连击无限推）
        #[derive(Component, Clone, Debug, Reflect, Serialize, Deserialize)]
        pub struct KnockbackImmunity(pub f32); // remaining seconds
    }
}

// ---------------------------------------------------------------------------
// 客户端输入（用 leafwing-input 采集，本地存储）
// ---------------------------------------------------------------------------

// Actionlike derive 不会自动添加 `Hash` / `FromReflect` / `Typed`，
// 需要用户手动 derive。`ActionState<A>` 要求 `A: Actionlike + Hash`，
// `InputMap<A>` 的注册机制要求 `A: FromReflect + Typed`（来自 bevy_reflect）。
#[derive(Reflect, Actionlike, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    Sprint,
    Attack,
    Block,
}
