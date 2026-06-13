//! 跨 server/client 共享的网络协议定义
//!
//! ## lightyear 0.26 实际 API（2026-06-09 读 `lightyear-0.26.4` 源码确认）
//!
//! lightyear 0.26 **没有** `protocol!` 宏（0.27+ 才有）。0.26 走 **Plugin 风格**，
//! 在 `App` 上直接调 `register_message::<M>()` / `register_component::<C>()`。
//!
//! ### 实际 API 关键点
//!
//! - `lightyear_messages::Message` 是 **blanket impl**：
//!   `impl<T: Send + Sync + 'static> Message for T {}`。消息类型 **不需要**
//!   derive `Message`（`bevy::prelude::Message` derive 是另一回事 — 只
//!   是把类型注册为 bevy 的本地事件，跟 lightyear 互不干扰）。
//! - `app.register_message::<M>()` 来自 `AppMessageExt` trait
//!   （`lightyear_messages-0.26.4/src/registry.rs:344`），约束为
//!   `M: Message + Serialize + DeserializeOwned`，返回**owned** 的
//!   `MessageRegistration<'_, M>`，上面有 `.add_direction(NetworkDirection::*)`。
//! - `app.register_component::<C>()` 来自 `AppComponentExt` trait
//!   （`lightyear_replication-0.26.4/src/registry/registry.rs:404`），
//!   约束更严：`C: Component<Mutability: GetWriteFns<C>> + Serialize +
//!   DeserializeOwned`。`GetWriteFns<C>` 由
//!   `lightyear_replication-0.26.4/src/registry/replication.rs:253` 提供：
//!   `impl<C: Component<Mutability = Self> + PartialEq> GetWriteFns<C> for
//!   Mutable {}`，所以 `#[derive(Component, PartialEq)]` 默认 `Mutability =
//!   Mutable` 就自动满足。
//! - 可选 chain：`.add_prediction()` (`PredictionRegistrationExt`,
//!   lightyear_prediction-0.26.4/src/registry.rs:312`)、`.add_interpolation()` /
//!   `.add_linear_interpolation()` (`InterpolationRegistrationExt`)。本文件
//!   **不**调用 — 用默认 `ComponentReplicationConfig`（server→client 复制，
//!   不带 prediction / interpolation）。等 task-2 / task-3 真的接 PvP 时
//!   再按需加。
//! - `lightyear::prelude::PeerId` 存在（来自
//!   `lightyear_core-0.26.4/src/lib.rs:47`）。
//! - `lightyear_inputs_leafwing::prelude::InputPlugin::<A>::default()` 是
//!   正确路径（`lightyear_inputs_leafwing-0.26.4/src/lib.rs:54-57`）；也可
//!   走 `lightyear::prelude::input::leafwing::InputPlugin::<A>`。
//! - `leafwing-input-manager` 0.20 的 `Actionlike` trait
//!   （`leafwing-input-manager-0.20.0/src/lib.rs:101-106`）需要
//!   `Debug + Eq + Hash + Send + Sync + Clone + Reflect + Typed + TypePath +
//!   FromReflect + 'static`。`#[derive(Actionlike)]` 宏会**自动**加这些，
//!   不用手写。
//!
//! 完整 drift 报告（对照 0.26 源码逐项核对）：`plans/fix-core-protocol-drift.drift.md`
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
// `Actionlike` derive 宏 (`leafwing-input-manager-0.20.0/src/lib.rs:101-106`)
// 要求 trait bound `Debug + Eq + Hash + Send + Sync + Clone + Reflect + Typed
// + TypePath + FromReflect + 'static`, derive 宏会**自动**加上这些, 所以下面
// derive 列表里**只**写 `Actionlike` 即可 (其他 derive 是给 `Message` /
// serialization / `bevy::prelude` 用的, 跟 lightwing 无关).
//
// lightyear 0.26 在运行时通过 `InputPlugin::<PlayerAction>::default()` 把它
// 接进网络层 (在 ProtocolPlugin.build 里 add_plugins)。lightyear 0.26 **不
// 再有** `#[input]` 宏的 derive 形式 — 那个是 0.20 之前的写法; 现在 macro
// 拆成了 `leafwing-input-manager::Actionlike` (本 enum 已 derive) +
// `lightyear_inputs_leafwing::InputPlugin` (在 ProtocolPlugin 里手动 add)。

/// 玩家输入动作 — 客户端采集、本地预测、传送到服务端
#[derive(Reflect, Actionlike, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerAction {
    MoveForward,
    MoveBackward,
    MoveLeft,
    MoveRight,
    Jump,
    Sprint,
    Attack,
    Block,
    Gather,
    Place,
    Craft,
    FoundNation,
    KillCreature,
}

// ============================================================================
// Messages — 网络层传递的事件（非 Component，瞬时消息）
// ============================================================================
//
// lightyear 0.26 仍然保留了 `lightyear_messages::Message` trait 作为标记
// trait, 但它是 **blanket impl** (`lightyear_messages-0.26.4/src/lib.rs:61-62`):
// `pub trait Message: Send + Sync + 'static {}` + `impl<T: Send + Sync + 'static>
// Message for T {}`. 所以消息 struct derive 时 **不需要** 显式 `Message` —
// 任何 `Send + Sync + 'static` 类型都自动实现。
//
// `bevy::prelude::Message` derive (上面 derive 列表里的那个) 是 **bevy 本地
// event 系统** 的 derive 宏, 跟 lightyear 互不干扰 — 加上它只是顺手把消息
// 类型注册为 bevy 事件 (供本地 reader / observer 用), 跟 lightyear 的
// `Message` blanket impl 是两码事。
//
// `app.register_message::<M>()` 实际约束是 `M: Message + Serialize +
// DeserializeOwned` (`lightyear_messages-0.26.4/src/registry.rs:344`), 我
// 们 derive 全套 (Serialize/Deserialize/Clone/Debug/PartialEq) 满足它。

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

    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
    pub enum BuildRecipe {
        PlankPack,
        Campfire,
    }

    #[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
    pub enum GameplayCommandKind {
        GatherFootBlock,
        PlaceWoodFootBlock,
        Craft(BuildRecipe),
        FoundNation,
        KillNearestCreature,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct GameplayCommand {
        pub tick: u64,
        pub player_block: [i32; 3],
        pub kind: GameplayCommandKind,
    }

    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct GameplayFeedback {
        pub ok: bool,
        pub summary: String,
    }

    // ============================================================================
    // wire-network-and-loop 任务 (2026-06-13): 应用层 PlayerPos sync
    //
    // 绕开 lightyear 0.26 自动 replication (UpdatesMessage) 卡 1% 的那 1% ——
    // server 端用 `MessageSender` 直接发 ServerPosUpdate, client 端 reader 解码
    // 写到 `PlayerNetPos` resource (一个用 Mutex<Vec3> 包起来的位置), client 端
    // 自己的 apply_system 写 Transform。后续做预测/插值时, ServerPosUpdate 可
    // 继续用 — 这是"权威帧"推送。
    //
    // 走 UnorderedReliable (MetadataChannel) 保证不丢包 + 不乱序, 60Hz 推也
    // 不会撑爆 (Vec3 = 12 bytes + 1 byte tick = 13 bytes/packet, 780 B/s)。
    // ============================================================================
    #[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Reflect, bevy::prelude::Message)]
    pub struct ServerPosUpdate {
        pub server_tick: u32,
        pub pos: Vec3,
    }
}

// ============================================================================
// Components — Replicate 组件（服务端复制到客户端）
// ============================================================================
//
// lightyear 0.26 走的是 `AppComponentExt::register_component<C: Component<Mutability:
// GetWriteFns<C>> + Serialize + DeserializeOwned>` (`lightyear_replication-0.26.4/
// src/registry/registry.rs:404`)。这个 `GetWriteFns<C>` 由
// `lightyear_replication-0.26.4/src/registry/replication.rs:253` 提供:
// `impl<C: Component<Mutability = Self> + PartialEq> GetWriteFns<C> for Mutable {}`。
// 我们的 struct 都 derive `Component, PartialEq` 且默认 `Mutability = Mutable`
// (bevy 0.18 `#[derive(Component)]` 默认值), 自动满足。
//
// `Health` 上的 `#[component(storage = "SparseSet")]` 是 bevy 0.18 的 attribute
// 写法, lightyear 0.26 不修改这些 attribute。如果 cargo check 报 unknown
// attribute, 改成 bevy 0.18 的新写法 (一般是 `#[component(sparse_set)]` 或
// 删掉 — sparse set 已是默认)。
//
// 没在这里调 `add_prediction()` / `add_interpolation()` / `add_linear_interpolation()` —
// 默认 `ComponentReplicationConfig::default()` 已经够用 (server→client 复制, 不
// 带 prediction / interpolation)。等 task-2 真接 PvP 客户端预测时, 在
// `PlayerPos` / `PlayerRot` 上按需加。

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
        pub attack_speed: f32, // 次/秒
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

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct GameplayHudState {
        pub tick: u64,
        pub player_block_pos: [i32; 3],
        pub player_pos: [f32; 3],
        pub nation_id: Option<u32>,
        pub monsters_killed: u32,
        pub blocks_gathered: u32,
        pub nations_founded: u32,
        pub inventory_wood: i64,
        pub inventory_food: i64,
        pub inventory_apple: i64,
        pub inventory_soul: i64,
        pub pool_wood: i64,
        pub pool_food: i64,
        pub pool_apple: i64,
        pub pool_soul: i64,
        pub flag_count: u32,
        pub total_nations: u32,
        pub monster_count: u32,
        pub observer_anomalies: u64,
        pub observer_invariant_violations: u64,
        pub status_line: String,
    }

    #[derive(Component, Serialize, Deserialize, Clone, Debug, PartialEq, Reflect)]
    pub struct VoxelDelta {
        pub revision: u64,
        pub x: i32,
        pub y: i32,
        pub z: i32,
        pub block: u8,
    }
}

// ============================================================================
// ProtocolPlugin — Bevy plugin，集中注册所有 message / component / input
// ============================================================================
//
// 这个 plugin 会被 `lk2-server` 和 `lk2-client` 各自 add（双方注册一致才能匹配）。
//
// lightyear 0.26 在 add_plugins(ClientPlugins) / add_plugins(ServerPlugins) 之后才
// 调 add_plugins(ProtocolPlugin)；且必须在 spawn Client/Server entity 之前。
// (见 lightyear-0.26.4/src/lib.rs:96 注释)
//
// 顺序: `add_plugins(InputPlugin)` 先 (只依赖 leafwing), 再
// `register_message::<M>().add_direction(NetworkDirection::X)`, 最后
// `register_component::<C>()` — 跟 lib.rs 文档示例一致。

pub struct ProtocolPlugin;

impl Plugin for ProtocolPlugin {
    fn build(&self, app: &mut App) {
        use lightyear::prelude::*;

        // ----- lightyear 0.26 init order workaround -----
        // 1. `MessagePlugin::finish` 会在 finish 阶段 `remove_resource::<MessageRegistry>().unwrap()`
        //    (lightyear_messages-0.26.4/src/plugin.rs:65-68), 然后 line 162 写回一个内部构造的 registry.
        // 2. 但 `ServerMultiMessageSender<With<Connected>>::metadata` (即 `Res<MessageRegistry>`)
        //    在 `receive_input_message` (lightyear_inputs-0.26.4/src/server.rs:128-131, 147) 第一次
        //    跑时验证. 如果资源不在 → panic "Resource does not exist".
        // 3. fix: 我们手动 init 一个空 MessageRegistry BEFORE register 任何 message, 这样
        //    (a) `MessagePlugin::finish` 的 `remove_resource().unwrap()` 成功（我们提供了 resource）,
        //    (b) `insert_resource(registry)` 写回时用 lightyear 内部构造的完整 registry,
        //    (c) system 第一次跑时 resource 已经存在, validation 通过.
        // Reference: lightyear_messages-0.26.4/src/registry.rs:378-384 (`register_message_custom_serde`
        // 内部 `if !has_resource { init_resource }`), 所以 init 0 个 message 也会让 resource 存在.
        app.init_resource::<MessageRegistry>();

        // ----- Inputs (leafwing) -----
        // 客户端采集 PlayerAction, lightyear 自动把它序列化传到服务端。
        app.add_plugins(lightyear_inputs_leafwing::prelude::InputPlugin::<
            PlayerAction,
        >::default());

        // ----- Messages (瞬时事件) -----
        // 客户端 → 服务端
        app.register_message::<messages::AttackInput>()
            .add_direction(NetworkDirection::ClientToServer);
        app.register_message::<messages::GameplayCommand>()
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
        app.register_message::<messages::GameplayFeedback>()
            .add_direction(NetworkDirection::ServerToClient);

        // 应用层 PlayerPos sync — 走 MetadataChannel (UnorderedReliable)
        // ServerToClient, 60Hz 推 12-byte Vec3, 可靠性 100%, 后续做预测/插值
        // 也走这条
        app.register_message::<messages::ServerPosUpdate>()
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
        app.register_component::<components::GameplayHudState>();
        app.register_component::<components::VoxelDelta>();
    }
}
