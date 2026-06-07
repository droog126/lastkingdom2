//! 跨 server/client 共享的网络协议定义
//!
//! ## lightyear 0.26 API 说明
//!
//! TODO(lightyear 0.27+): lightyear 0.26 已经没有 `protocol!` 宏了（之前的 plan
//! §6 和 task 描述是按老版本写的）。0.26 走的是 **Plugin 风格**：
//!
//! ```ignore
//! // 旧版 (lightyear 0.20 之前):
//! protocol! {
//!     #[input]
//!     PlayerActions(actions: PlayerAction),
//!     AttackInput { tick: u32, ... } => Reliable,
//!     PlayerPos(pub Vec3),
//!     ...
//! }
//! ```
//!
//! ```ignore
//! // 0.26 新版:
//! pub struct MyProtocol;
//! impl Plugin for MyProtocol {
//!     fn build(&self, app: &mut App) {
//!         app.register_message::<AttackInput>().add_direction(NetworkDirection::ClientToServer);
//!         app.register_message::<HitConfirm>().add_direction(NetworkDirection::ServerToClient);
//!         app.register_component::<PlayerPos>().add_prediction().add_linear_interpolation();
//!         app.add_plugins(input::leafwing::InputPlugin::<PlayerAction>::default());
//!     }
//! }
//! ```
//!
//! Message 注册需要 `Message + Serialize + DeserializeOwned + Clone + Debug + PartialEq + Reflect`。
//! Component 注册需要 `Component + Serialize + DeserializeOwned + Clone + Debug + PartialEq`。
//!
//! 等 lightyear 出现 `protocol!` 宏后（看 0.27+ 计划），把下面这些 `pub struct` 改成 macro 写法即可。
//!
//! ## 模块组织
//!
//! - `messages`        — Message 定义（AttackInput / HitConfirm / DamageResult / KnockbackEvent / KillFeedEntry）
//! - `components`      — Replicate 组件（Health / WeaponStatsRaw / EquippedWeapon / CombatReady / KnockbackImmunity / PlayerPos / PlayerRot / MonsterKind / MonsterHealth）
//! - `PlayerAction`    — leafwing-input-manager 客户端输入枚举
//! - `ProtocolPlugin`  — Bevy plugin，调用 register_message / register_component / add_plugins(InputPlugin)

// ============================================================================
// 依赖
// ============================================================================

use bevy::prelude::*;
use leafwing_input_manager::Actionlike;
use serde::{Deserialize, Serialize};

// ============================================================================
// PlayerAction — 客户端输入枚举
// ============================================================================
//
// leafwing-input-manager 0.20 的 `Actionlike` derive 自动给 `ActionState<A>` 用的
// trait bound (`A: Actionlike + Hash`)，还需要用户自己 derive `Hash + Eq + Reflect +
// FromReflect + Typed`（来自 bevy_reflect）。
//
// lightyear 0.26 在运行时通过 `InputPlugin::<PlayerAction>::default()` 把它接进网络层
// (在 ProtocolPlugin.build 里 add_plugins)。
//
// 命名说明：lightyear 0.26 不再有 `#[input]` 宏的 derive 形式 — 那个是 0.20 之前的
// 写法；现在 macro 拆成了 `leafwing-input-manager` 的 `Actionlike` (本 enum 已 derive)
// + `lightyear_inputs_leafwing::InputPlugin` (在 ProtocolPlugin 里手动 add)。

/// 玩家输入动作 — 客户端采集、本地预测、传送到服务端
#[derive(
    Reflect, Actionlike, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash,
)]
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

// ============================================================================
// Messages — 网络层传递的事件（非 Component，瞬时消息）
// ============================================================================
//
// 注意: lightyear 0.26 仍然保留了 `lightyear::prelude::Message` trait 作为
// 标记 trait, 但不在 `derive` 列表里 — `Message` 是 blanket impl: `impl<T: Send +
// Sync + 'static> Message for T {}`. 所以消息 derive 时 **不需要** 显式 `Message`。
//
// 但 `app.register_message::<M>()` 在 0.26 要求 `M: Message + Serialize +
// DeserializeOwned + Clone + Debug + PartialEq`。我们 derive 全套。

pub mod messages {
    //! 跨网络瞬时消息

    use super::*;

    // 客户端 → 服务端：攻击输入
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct AttackInput {
        pub tick: u32,
        pub input_dir: Vec3, // 攻击方向（玩家朝向）
        pub is_falling: bool,
        pub combo_count: u8,
    }

    // 服务端 → 客户端：命中确认（即时视觉反馈，不影响逻辑）
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct HitConfirm {
        pub victim_id: lightyear::prelude::PeerId,
        pub damage: f32,
        pub is_critical: bool,
        pub hit_pos: Vec3,
        pub server_tick: u32,
    }

    // 服务端 → 客户端：击退指令（即时覆盖）
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct KnockbackEvent {
        pub victim_id: lightyear::prelude::PeerId,
        pub velocity: Vec3,
        pub server_tick: u32,
    }

    // 服务端 → 客户端：伤害结果（可靠，血量最终一致）
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct DamageResult {
        pub victim_id: lightyear::prelude::PeerId,
        pub new_health: f32,
        pub is_dead: bool,
        pub server_tick: u32,
    }

    // 服务端 → 客户端：击杀播报
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct KillFeedEntry {
        pub killer_name: String,
        pub victim_name: String,
        pub weapon_id: u8,
    }
}

// ============================================================================
// Components — Replicate 组件（服务端复制到客户端）
// ============================================================================
//
// `lightyear::prelude::Component` 是 blanket impl: `impl<T: bevy::prelude::Component>
// Component for T {}`. 同样 **不需要** 显式 derive。
//
// `app.register_component::<C>()` 要求 `C: Component + Serialize + DeserializeOwned
// + Clone + Debug + PartialEq`。derive 全套。
//
// 部分组件带 `#[component(storage = "SparseSet")]` attribute — 这是 bevy 原生
// 0.18 的 attribute, 0.26 lightyear 直接透传, 不冲突。

pub mod components {
    //! 服务端复制到客户端的组件

    use super::*;

    /// 玩家血量
    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    #[component(storage = "SparseSet")]
    pub struct Health(pub f32);

    /// 武器属性
    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct WeaponStatsRaw {
        pub reach: f32,
        pub damage: f32,
        pub knockback: f32,
        pub attack_speed: f32,
        pub sweep_angle_deg: f32,
    }

    /// 武器实例（挂在玩家 entity 上，指向 WeaponStatsRaw）
    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct EquippedWeapon(pub u8); // weapon_id

    /// 玩家已装备武器的完整属性（从 WeaponRegistry 查表）
    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
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
    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct KnockbackImmunity(pub f32); // remaining seconds

    // ----- 新增 protocol 复制组件（task 要求） -----

    /// 玩家位置（server → client 复制）
    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct PlayerPos(pub Vec3);

    /// 玩家偏航角 yaw（server → client 复制）
    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct PlayerRot(pub f32);

    /// 怪物种类 ID（server → client 复制）
    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct MonsterKind(pub u8);

    /// 怪物血量（server → client 复制）
    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct MonsterHealth(pub f32);
}

// ============================================================================
// ProtocolPlugin — Bevy plugin，集中注册所有 message / component / input
// ============================================================================
//
// 这个 plugin 会被 `lk2-server` 和 `lk2-client` 各自 add（双方注册一致才能匹配）。
//
// lightyear 0.26 在 add_plugins(ClientPlugins) / add_plugins(ServerPlugins) 之后才
// 调 add_plugins(ProtocolPlugin)；且必须在 spawn Client/Server entity 之前。

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        use lightyear::prelude::*;

        // ----- Inputs (leafwing) -----
        // 客户端采集 PlayerAction, lightyear 自动把它序列化传到服务端。
        app.add_plugins(
            lightyear_inputs_leafwing::prelude::InputPlugin::<PlayerAction>::default(),
        );

        // ----- Messages (瞬时事件) -----
        // 客户端 → 服务端
        app.register_message::<messages::AttackInput>()
            .add_direction(NetworkDirection::ClientToServer);

        // 服务端 → 客户端
        app.register_message::<messages::HitConfirm>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<messages::KnockbackEvent>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<messages::DamageResult>()
            .add_direction(NetworkDirection::ServerToClient);
        app.register_message::<messages::KillFeedEntry>()
            .add_direction(NetworkDirection::ServerToClient);

        // ----- Components (server → client 复制) -----
        app.register_component::<components::Health>();
        app.register_component::<components::WeaponStatsRaw>();
        app.register_component::<components::EquippedWeapon>();
        app.register_component::<components::CombatReady>();
        app.register_component::<components::KnockbackImmunity>();
        app.register_component::<components::PlayerPos>();
        app.register_component::<components::PlayerRot>();
        app.register_component::<components::MonsterKind>();
        app.register_component::<components::MonsterHealth>();
    }
}
