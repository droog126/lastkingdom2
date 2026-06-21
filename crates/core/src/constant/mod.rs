//! Game-wide constants (all numbers from the design doc 总纲)
//!
//! Refs:
//!   * 表 1 - Global Resource Pool (二、核心设计支柱)
//!   * 表 2 - 怪物生态 Max 容量 (三、世界与维度 → 怪物王国)
//!   * §四 1 - 核心循环数值节律 (食物/工具耐久/资源点刷新)

#![allow(dead_code)]

// --- Tick rates (1 / X 秒) ---------------------------------------------

/// 慢 tick：全局资源再生 / 守恒 / 怪物生态 / 怪物王国衰败 / 枯萎
pub const SLOW_TICK_SECS: f32 = 1.0;
/// 快 tick：AI 行为决策 / 战争迷雾刷新 / 玩家位置更新
pub const FAST_TICK_SECS: f32 = 0.2;

// --- World 尺寸（demo 缩小版） ----------------------------------------

/// 单局世界总格数（demo 缩到 32³；doc 是大世界）
/// 垂直方向世界高度（Y 维度）。XZ 方向无限（按需生成 + 局部 AABB 渲染）
pub const VERTICAL_SIZE: i32 = 96;

pub const SEA_LEVEL: i32 = 12;

/// 实体水面 mesh 的 y 坐标（f32）— 与 SEA_LEVEL 同高。
///
/// 设计：superflat 地形 surface 在 SEA_LEVEL + 1 处，水面在 SEA_LEVEL，
/// 水面永远在 ground surface **下方** 至少 1m，玩家在岸上时脚下
/// 是 ground（y=SEA_LEVEL+1），脚下 1m 处是水面（y=SEA_LEVEL），
/// 不会"站在水里"。
///
/// 之前 v1（2026-06-19）water_y = SEA_LEVEL + 1.5 → 水面比地面高 0.5m，
/// 跟玩家脚面 y=13.0 (ground_y+0.5) 几乎齐平，俯瞰时水面会穿过 avatar 脚底
/// 视觉上"玩家站在水里"。本常量锁死这个 invariant。
pub const WATER_Y: f32 = SEA_LEVEL as f32;

/// superflat 地形 surface 的 y 坐标（f32）— 永远 = SEA_LEVEL + 1。
/// 与 `WATER_Y` 的关系是测试的硬不变量: `WATER_Y + 1 == SUPERFLAT_GROUND_Y`。
pub const SUPERFLAT_GROUND_Y: f32 = SEA_LEVEL as f32 + 1.0;

/// 兼容老代码：等于 VERTICAL_SIZE
pub const WORLD_SIZE: i32 = VERTICAL_SIZE;
/// 世界中心 (中立商人 / 王国位置基座)
pub const WORLD_CENTER: [i32; 2] = [WORLD_SIZE / 2, WORLD_SIZE / 2];

// --- 国家系统（§五、1 创建国家） ---------------------------------------

/// 一局最多面国旗 (= 最多国家数)
pub const MAX_NATIONAL_FLAGS: u32 = 8;
/// 国旗购买成本递增（灵魂数，按购买顺序）
pub const FLAG_COSTS_SOULS: [u64; 8] = [10, 15, 20, 25, 30, 40, 50, 60];
/// 国家初始人口上限（含国王）
pub const INITIAL_POP_CAP: u32 = 5;
/// 人口上限 10
pub const POP_UPGRADE_10_COST: (u64, u64, u64) = (500, 200, 0); // 木 + 食 + 灵
/// 人口上限 15
pub const POP_UPGRADE_15_COST: (u64, u64, u64) = (1_000, 500, 10);
/// 人口上限 20
pub const POP_UPGRADE_20_COST: (u64, u64, u64) = (2_000, 1_000, 25);

/// 国家旗帜生命值
pub const FLAG_HP: u32 = 100;

// --- 玩家 / 视野（§八、视野与信息系统） -------------------------------

/// 玩家基础视野半径（格）
pub const PLAYER_VISION_RADIUS: i32 = 24;
/// 战争迷雾衰减（每格视野阻挡衰减）
pub const FOG_FALLOFF_PER_BLOCK: f32 = 0.04;

// --- 怪物生态（§三、怪物王国与小巢生态） -----------------------------

/// 全局怪物王国上限
pub const MAX_MONSTER_KINGDOMS: u32 = 5;
/// 全局怪物小巢上限
pub const MAX_MONSTER_NESTS: u32 = 80;
/// 全局怪物个体上限
pub const MAX_MONSTER_INDIVIDUALS: u32 = 1_500;
/// 单个小巢初始怪物数
pub const NEST_INITIAL_INDIVIDUALS: (u32, u32) = (15, 25);
/// 单个王国维持怪物数
pub const KINGDOM_MAINTAIN_INDIVIDUALS: (u32, u32) = (80, 120);
/// 5 分钟无活动后小巢进入沉寂
pub const NEST_DORMANCY_SECS: u32 = 5 * 60;

// --- 资源守恒（§二、全局资源池） -------------------------------------

/// 启用严格守恒检查（debug 模式 + 测试）
pub const STRICT_CONSERVATION_CHECK: bool = true;

#[cfg(test)]
mod tests {
    use super::*;

    /// invariant: 水面 (WATER_Y) 必须在 superflat 地面 (SUPERFLAT_GROUND_Y) **下方**。
    /// 否则 client 水面 mesh 会盖在玩家脚下，视觉上"玩家站在水里"。
    /// 之前 v1 (iter_1357) water_y = SEA_LEVEL + 1.5 = 13.5 比 ground (13) 高 0.5m，
    /// 跟玩家脚面 y=13.5 几乎齐平。本测试锁死 WATER_Y < SUPERFLAT_GROUND_Y。
    #[test]
    fn water_below_ground_invariant() {
        assert!(
            WATER_Y < SUPERFLAT_GROUND_Y,
            "WATER_Y ({}) 必须 < SUPERFLAT_GROUND_Y ({}), 否则玩家'站在水里'",
            WATER_Y,
            SUPERFLAT_GROUND_Y
        );
        // 至少差 1m，玩家/装饰物踩在 ground 上不会"浸"在水里
        assert!(
            SUPERFLAT_GROUND_Y - WATER_Y >= 1.0,
            "WATER_Y ({}) 到 SUPERFLAT_GROUND_Y ({}) 至少差 1m, got {}",
            WATER_Y,
            SUPERFLAT_GROUND_Y,
            SUPERFLAT_GROUND_Y - WATER_Y
        );
    }

    /// invariant: WATER_Y 必须等于 SEA_LEVEL (单一信息源)
    #[test]
    fn water_y_equals_sea_level() {
        assert_eq!(WATER_Y as i32, SEA_LEVEL);
    }
}
