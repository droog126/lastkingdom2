//! PvP 角色控制器组件 (跨 server/client 共享的数据结构)
//!
//! 位于 `crates/core/src/controller/mod.rs` 而非 `crates/core/src/controller/components.rs`,
//! 方便外部用 `use lk2_core::controller::PvPController` 一行拿到。
//!
//! 所有状态都是 ECS 组件，天然支持 lightyear 网络同步。
//!
//! **未迁入**（保留在 umbrella binary, 留给 client task 拆）:
//! - `systems.rs` — 移动系统（地面检测 + 输入处理 + 自动爬），client 任务
//! - `ControllerPlugin` — 跟随 systems.rs 一起迁

use bevy::prelude::*;

// ---------------------------------------------------------------------------
// PvP 角色控制器
// ---------------------------------------------------------------------------

/// PvP 角色控制器
///
/// 设计目标：
/// - Minecraft 风格移动（5 m/s 基础速度）
/// - 自动爬 0.6m 高度差（1 格 = 1m，半砖 = 0.5m）
/// - 击退抗性（被击中后短暂减速）
/// - 地面检测（射线投射）
#[derive(Component, Clone, Debug, Reflect)]
#[require(Transform)]
pub struct PvPController {
    // === 移动参数 ===
    /// 水平移动速度（m/s），Minecraft 默认 4.3，疾跑 5.6
    pub speed: f32,
    /// 跳跃冲量（m/s 向上），Minecraft 默认约 8.0
    pub jump_impulse: f32,
    /// 空中控制力度（0.0 = 无控制，1.0 = 完全控制）
    pub air_control: f32,
    /// 重力倍率（1.0 = 正常重力）
    pub gravity_scale: f32,

    // === 自动爬台阶 ===
    /// 最大自动爬升高度（米），0.6 = 1 格体素
    pub step_height: f32,
    /// 自动爬升速度（m/s）
    pub step_speed: f32,

    // === 地面检测 ===
    /// 是否在地面上
    pub is_grounded: bool,
    /// 上次检测到地面的时间（秒）
    pub last_grounded_time: f32,
    /// 地面法线（用于斜坡滑动）
    pub ground_normal: Option<Vec3>,

    // === 击退系统 ===
    /// 击退抗性（0.0 = 完全受击退，1.0 = 免疫击退）
    pub knockback_resistance: f32,
    /// 当前击退速度（逐渐衰减）
    pub knockback_velocity: Vec3,
    /// 击退硬直剩余时间（秒）
    pub knockback_stun: f32,
    /// 击退硬直期间输入削弱比例（0.3 = 只生效 30%）
    pub knockback_input_penalty: f32,

    // === 输入状态 ===
    /// 移动输入方向（归一化）
    pub move_input: Vec2,
    /// 是否请求跳跃
    pub jump_requested: bool,
    /// 是否正在疾跑
    pub is_sprinting: bool,
}

impl Default for PvPController {
    fn default() -> Self {
        Self {
            // 移动参数
            speed: 5.0,        // Minecraft 疾跑速度
            jump_impulse: 8.0, // MC 默认
            air_control: 0.3,  // 空中控制较弱
            gravity_scale: 1.0,

            // 自动爬
            step_height: 0.6, // 自动爬 1 格
            step_speed: 3.0,

            // 地面检测
            is_grounded: false,
            last_grounded_time: 0.0,
            ground_normal: None,

            // 击退
            knockback_resistance: 0.0,
            knockback_velocity: Vec3::ZERO,
            knockback_stun: 0.0,
            knockback_input_penalty: 0.3,

            // 输入
            move_input: Vec2::ZERO,
            jump_requested: false,
            is_sprinting: false,
        }
    }
}

impl PvPController {
    /// 创建新的控制器
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置移动速度
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// 设置跳跃力度
    pub fn with_jump(mut self, impulse: f32) -> Self {
        self.jump_impulse = impulse;
        self
    }

    /// 设置击退抗性
    pub fn with_knockback_resistance(mut self, resistance: f32) -> Self {
        self.knockback_resistance = resistance.clamp(0.0, 1.0);
        self
    }

    /// 应用击退
    pub fn apply_knockback(&mut self, velocity: Vec3) {
        let effective = velocity * (1.0 - self.knockback_resistance);
        self.knockback_velocity += effective;
        self.knockback_stun = 0.5; // 0.5 秒硬直
    }

    /// 是否处于击退硬直中
    pub fn is_stunned(&self) -> bool {
        self.knockback_stun > 0.0
    }

    /// 获取当前有效输入比例（击退硬直期间降低）
    pub fn input_multiplier(&self) -> f32 {
        if self.is_stunned() {
            self.knockback_input_penalty
        } else {
            1.0
        }
    }
}

// ---------------------------------------------------------------------------
// 玩家碰撞体配置
// ---------------------------------------------------------------------------

/// 玩家碰撞体配置
///
/// 胶囊体：半径 0.4m，高度 1.8m（Minecraft 玩家碰撞箱 0.6×1.8）
#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct PlayerCollider {
    /// 胶囊体半径
    pub radius: f32,
    /// 胶囊体半高（不含两端半球）
    pub half_height: f32,
    /// 眼睛高度（从脚底算起）
    pub eye_height: f32,
}

impl Default for PlayerCollider {
    fn default() -> Self {
        Self {
            radius: 0.3,      // MC 玩家宽度 0.6m
            half_height: 0.9, // 总高度 = 0.3 + 0.9*2 + 0.3 = 2.4m（略高，方便碰撞）
            eye_height: 1.62, // MC 标准
        }
    }
}

// ---------------------------------------------------------------------------
// 地面检测射线结果（不是 Component，是 query 时返回的数据结构）
// ---------------------------------------------------------------------------

/// 地面检测射线结果
#[derive(Clone, Debug)]
pub struct GroundHit {
    /// 碰撞点（世界坐标）
    pub point: Vec3,
    /// 碰撞法线
    pub normal: Vec3,
    /// 到碰撞点的距离
    pub distance: f32,
    /// 碰撞的实体（如果有）
    pub entity: Option<Entity>,
}

// ---------------------------------------------------------------------------
// 兼容层：保留 `crate::controller::components::*` 的导入路径
// （原 src/controller/systems.rs 可能用过这个路径）
// ---------------------------------------------------------------------------

pub mod components {
    pub use super::{GroundHit, PlayerCollider, PvPController};
}
