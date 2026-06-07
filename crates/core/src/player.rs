//! 玩家状态 — 共享给 core (sim) / client (render) / server (PvP 校验)
//!
//! 原位置: `src/render/mod.rs::PlayerState` (line 798-807)
//! 迁出原因: scenario/creature 在 core 里要引用它，render 又依赖 core，
//! 反向依赖会循环，所以从 render 拆到 core。
//!
//! `Player`（entity marker Component）暂留在 render 那边，不动。

use bevy::prelude::*;
use std::collections::HashMap;

use crate::nation::NationId;
use crate::resource::ResourceKind;

/// 玩家状态（Resource，不是 Component）
///
/// - `pos` / `block_pos` — sim 位置（被 scenario 的 MoveTo / attempt_move 改）
/// - `inventory`         — 玩家背包
/// - `nation_id`         — 当前所属国家
/// - 三个 `*_count`       — UI / tick 录制统计
#[derive(Resource, Default)]
pub struct PlayerState {
    pub pos: Vec3,
    pub block_pos: [i32; 3],
    pub inventory: HashMap<ResourceKind, i64>,
    pub nation_id: Option<NationId>,
    pub monsters_killed: u32,
    pub blocks_gathered: u32,
    pub nations_founded: u32,
}
