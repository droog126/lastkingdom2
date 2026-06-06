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
pub const WORLD_SIZE: i32 = 96;
/// 海平面 Y
pub const SEA_LEVEL: i32 = 12;
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
