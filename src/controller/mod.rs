//! 体素角色控制器模块
//!
//! 基于 avian3d 的 PvP 角色控制器，专为体素世界设计：
//! - 地面检测（射线投射）
//! - 自动爬 1 格高度
//! - 击退抗性
//! - WASD + 跳跃
//! - 联机同步友好（所有状态都是 ECS 组件）
//!
//! ```
//! src/
//!   controller/
//!     mod.rs         — 模块入口 + 插件
//!     components.rs  — PvPController 组件定义
//!     systems.rs     — 移动系统（地面检测 + 输入处理 + 自动爬）
//! ```

pub mod components;
pub mod systems;

pub use components::*;
pub use systems::*;

use bevy::prelude::*;

/// 角色控制器插件
pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                systems::ground_detection,
                systems::character_movement,
                systems::auto_step_up,
                systems::knockback_decay,
            )
                .chain(),
        );
    }
}