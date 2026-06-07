//! `lk2-core` — sim 逻辑 / 数据层 + 跨 server/client 共享的协议
//!
//! 这是 lastkingdom2 workspace 的 **共享 lib**，被未来的 `lk2-server`
//! (MinimalPlugins headless) 和 `lk2-client` (DefaultPlugins) 同时依赖。
//!
//! 包含（按 client-server-split.md §3 模块去向表）:
//! - `constant`    — 全局常量（tick 节奏 / 资源池 / 世界尺寸）
//! - `world`       — 32³ voxel 世界 + 地形生成（含 `terrain` 子模块）
//! - `resource`    — 资源池 (Wood / Food / Apple / Soul / 元素) + Transfer
//! - `nation`      — 国家注册表 / 旗帜 / 灵魂结算
//! - `monster`     — 怪物生态 (Ecosystem) + 个体生成
//! - `ai`          — TickObserver 不变量 + 简单 AI 决策
//! - `scenario`    — 剧本状态机 (MoveTo / Gather / FoundNation / …)
//! - `creature`    — 动物 / 被动生物（猪羊牛鸡）
//! - `player`      — PlayerState（从原 `render::PlayerState` 迁出）
//! - `pvp`         — 跨 server/client 共享的 PvP 组件 (CombatState / Hitbox / Ping / PositionHistory / DamageEvent / VisualEffectEvent)
//! - `controller`  — 跨 server/client 共享的角色控制器组件 (PvPController / PlayerCollider / GroundHit)
//! - `protocol`    — lightyear 0.26 网络协议 (Messages / Components / PlayerAction / ProtocolPlugin)
//!
//! **不含**（保留在 umbrella binary / 之后的 server / client crate）:
//! - `pvp::los` / `pvp::systems_*` / `pvp::WeaponId` / `pvp::PvPPlugin` — 留到 server/client task 拆
//! - `controller::systems` / `ControllerPlugin` — 留到 client task 拆
//! - `render` / `pretty` / `utils` — 见后续 task

#![allow(dead_code)]
#![allow(unused_imports)]

pub mod clock;
pub mod constant;
pub mod controller;
pub mod creature;
pub mod monster;
pub mod nation;
pub mod player;
pub mod protocol;
pub mod pvp;
pub mod resource;
pub mod scenario;
pub mod world;

// ai 依赖 scenario 类型，scenario 依赖 player — player 必须先
pub mod ai;
