//! V2 地形覆盖层系统
//!
//! 来源: 《万国余烬_王冠赛季_地形系统技术方案》§9 地形覆盖层实现
//! + 《表格包》§6.3 改造物 ID 表
//! + 《资源循环与地形改造》(改造 → 资源流)
//!
//! ## 设计原则
//!
//! MVP 不做实时雕刻底层 mesh。所有改造都生成覆盖层实体:
//!
//! | 改造 | 覆盖层形态 | 碰撞 |
//! | --- | --- | --- |
//! | 沟 | 下凹 mesh + 减速区域 | 简化碰撞盒或高度修正 |
//! | 木墙 | 模块化墙段 | 碰撞盒,可破坏 |
//! | 石墙 | 模块化墙段 | 碰撞盒,高耐久 |
//! | 桥 | 模块化桥板 | 可通行碰撞 |
//! | 道路 | 薄 mesh 或 decal | 速度区域,无阻挡 |
//! | 符文槽 | 薄 mesh + 发光线 | 无阻挡,法阵接口 |
//! | 阵眼 | 圆形平台 mesh | 可站立,可破坏 |
//!
//! 坑洞相关覆盖层不直接修改 Base Terrain,坑体使用 `PitVolume` 标记范围和逃生策略,
//! 具体规则见《可破坏地形与坑洞逃生技术方案》。
//!
//! ## MVP 边界
//!
//! - 不接 `TerrainEditIntentMsg` / `TerrainEditCommittedMsg` (那是 T5 玩法接线范围)
//! - 只提供 Component / Resource / Message / Plugin 的"可 spawn / query / cleanup"基础
//! - 覆盖层实体的 mesh / collision 留给 client / server task 拆
//! - 资源消耗 / 预算回收 留给 scenario task
//!
//! ## 与文档的差异
//!
//! - 文档有 `TerrainChunk` / `TerrainEditEntity` / `ProtectedPath` 等,本 MVP 只取
//!   覆盖层本身(`TerrainOverlayEntity` + `TerrainOverlayRegistry`)。Chunk 由 world 模块负责。

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Kinds (对齐《地形系统技术方案》§9 表 + 表格包 §6.3)
// ---------------------------------------------------------------------------

/// 覆盖层类型
///
/// 文档列 7 种改造形态,本枚举直接对应。`id_str` 对齐表格包 ID 规范
/// (snake_case + 改造类别前缀)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OverlayKind {
    /// `ditch` 沟渠 — 下凹 mesh + 减速区域
    Ditch,
    /// `wood_wall` 木墙 — 模块化墙段,低耐久
    WoodWall,
    /// `stone_wall` 石墙 — 模块化墙段,高耐久
    StoneWall,
    /// `bridge` 桥 — 可通行桥板
    Bridge,
    /// `road` 道路 — 薄 mesh / decal,加速区域
    Road,
    /// `rune_slot` 符文槽 — 薄 mesh + 发光线,法阵接口
    RuneSlot,
    /// `altar` 阵眼 — 圆形平台,可破坏
    Altar,
}

impl OverlayKind {
    /// 表格包 ID (snake_case)
    pub fn id_str(self) -> &'static str {
        match self {
            Self::Ditch => "ditch",
            Self::WoodWall => "wood_wall",
            Self::StoneWall => "stone_wall",
            Self::Bridge => "bridge",
            Self::Road => "road",
            Self::RuneSlot => "rune_slot",
            Self::Altar => "altar",
        }
    }

    /// 中文标签
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::Ditch => "沟",
            Self::WoodWall => "木墙",
            Self::StoneWall => "石墙",
            Self::Bridge => "桥",
            Self::Road => "道路",
            Self::RuneSlot => "符文槽",
            Self::Altar => "阵眼",
        }
    }

    /// 默认耐久 (表格包 §6.3: 木墙 100 / 石墙 400 / 桥 150 / 阵眼 80 / 其他无限)
    pub fn default_durability(self) -> f32 {
        match self {
            Self::Ditch => f32::INFINITY,
            Self::WoodWall => 100.0,
            Self::StoneWall => 400.0,
            Self::Bridge => 150.0,
            Self::Road => f32::INFINITY,
            Self::RuneSlot => 200.0,
            Self::Altar => 80.0,
        }
    }

    /// 是否阻挡通行 (true = 实体不能走过;false = 仅视觉/法阵)
    pub fn blocks_movement(self) -> bool {
        match self {
            Self::Ditch => true,
            Self::WoodWall => true,
            Self::StoneWall => true,
            Self::Bridge => false, // 可通行
            Self::Road => false,
            Self::RuneSlot => false,
            Self::Altar => false, // 可站立
        }
    }

    /// 是否可被破坏 (true = 玩家/怪物能 hit + 耐久扣减)
    pub fn is_destructible(self) -> bool {
        match self {
            Self::WoodWall | Self::StoneWall | Self::Bridge | Self::Altar => true,
            Self::Ditch | Self::Road | Self::RuneSlot => false,
        }
    }

    /// 默认占地 (XZ 半径,米)
    pub fn default_footprint_radius(self) -> f32 {
        match self {
            Self::Ditch => 0.6,
            Self::WoodWall => 0.5,
            Self::StoneWall => 0.5,
            Self::Bridge => 1.5,
            Self::Road => 1.0,
            Self::RuneSlot => 0.8,
            Self::Altar => 1.6,
        }
    }
}

// ---------------------------------------------------------------------------
// Components
// ---------------------------------------------------------------------------

/// 覆盖层实体 Component
///
/// 挂在一个 Bevy entity 上,带 kind / 占地 / 耐久 / 所有者。MVP 不挂 mesh,
/// 留给 client 渲染层加 `Mesh3d` + `MeshMaterial3d`。
#[derive(Component, Debug, Clone)]
pub struct TerrainOverlayEntity {
    /// 唯一 ID(网络同步用)
    pub id: u32,
    pub kind: OverlayKind,
    /// 世界坐标 (XZ, y 取地表)
    pub world_pos: [f32; 3],
    /// 朝向弧度 (绕 Y 轴)
    pub yaw_radians: f32,
    /// 占地半径 (米)
    pub footprint_radius: f32,
    /// 当前耐久 (0..=default_durability)
    pub durability: f32,
    /// 所有者 ID (None = 中立 / 世界)
    pub owner: Option<u32>,
}

impl TerrainOverlayEntity {
    /// 构造一个新的覆盖层 (用 kind 默认值)
    pub fn new(id: u32, kind: OverlayKind, world_pos: [f32; 3]) -> Self {
        Self {
            id,
            kind,
            world_pos,
            yaw_radians: 0.0,
            footprint_radius: kind.default_footprint_radius(),
            durability: kind.default_durability(),
            owner: None,
        }
    }

    /// 是否已被破坏
    pub fn is_destroyed(&self) -> bool {
        self.kind.is_destructible() && self.durability <= 0.0
    }
}

// ---------------------------------------------------------------------------
// Resource — 全局注册表
// ---------------------------------------------------------------------------

/// 全局覆盖层注册表 (Resource)
///
/// 维护当前场上所有覆盖层。MVP 用 `HashMap<id, pos>` 索引 + 计数,
/// 不缓存 entity reference(由 bevy `Query` 负责)。
#[derive(Resource, Debug, Default, Clone)]
pub struct TerrainOverlayRegistry {
    /// `id -> world_pos` 索引
    pub entries: std::collections::HashMap<u32, [f32; 3]>,
    /// 当前总数量
    pub total_spawned: u32,
}

impl TerrainOverlayRegistry {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 注册一个新覆盖层 (返回 false 表示 id 已存在)
    pub fn register(&mut self, overlay: &TerrainOverlayEntity) -> bool {
        if self.entries.contains_key(&overlay.id) {
            return false;
        }
        self.entries.insert(overlay.id, overlay.world_pos);
        self.total_spawned += 1;
        true
    }

    /// 注销
    pub fn unregister(&mut self, id: u32) -> bool {
        self.entries.remove(&id).is_some()
    }
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

/// 覆盖层被创建 (服务端权威 → 客户端表现)
#[derive(Message, Debug, Clone)]
pub struct OverlaySpawnedMsg {
    pub id: u32,
    pub kind: OverlayKind,
    pub world_pos: [f32; 3],
    pub yaw_radians: f32,
    pub owner: Option<u32>,
}

/// 覆盖层耐久变化 (被攻击 / 修复)
#[derive(Message, Debug, Clone)]
pub struct OverlayDamagedMsg {
    pub id: u32,
    pub new_durability: f32,
}

/// 覆盖层被销毁 (耐久 <= 0 或被拆除)
#[derive(Message, Debug, Clone)]
pub struct OverlayDestroyedMsg {
    pub id: u32,
    pub kind: OverlayKind,
    /// 销毁原因 (供审计)
    pub reason: OverlayDestroyReason,
}

/// 销毁原因
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayDestroyReason {
    /// 耐久扣到 0
    Damaged,
    /// 玩家拆除(回收)
    Deconstructed,
    /// 国家灭亡 / 所有者失效 → 自然消失
    OwnerLost,
    /// 阶段时间到 (临时道路 / 符文槽过期)
    Expired,
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// 玩法 Plugin: 注册覆盖层资源 + events
///
/// 用法:
/// ```ignore
/// app.add_plugins(TerrainOverlayPlugin);
/// ```
///
/// 注意: 不挂任何 system (MVP 阶段 spawn 由 scenario / worldgen 直接 `commands.spawn`),
/// 只提供基础设施。T5 玩法接线时再加 `tick_overlay_durability` / `auto_cleanup_expired`。
pub struct TerrainOverlayPlugin;

impl Plugin for TerrainOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainOverlayRegistry>()
            .add_message::<OverlaySpawnedMsg>()
            .add_message::<OverlayDamagedMsg>()
            .add_message::<OverlayDestroyedMsg>();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_id_str_matches_doc() {
        // 文档 §9 + 表格包 §6.3 ID 必须一字不差
        assert_eq!(OverlayKind::Ditch.id_str(), "ditch");
        assert_eq!(OverlayKind::WoodWall.id_str(), "wood_wall");
        assert_eq!(OverlayKind::StoneWall.id_str(), "stone_wall");
        assert_eq!(OverlayKind::Bridge.id_str(), "bridge");
        assert_eq!(OverlayKind::Road.id_str(), "road");
        assert_eq!(OverlayKind::RuneSlot.id_str(), "rune_slot");
        assert_eq!(OverlayKind::Altar.id_str(), "altar");
    }

    #[test]
    fn label_zh_covers_all_7_kinds() {
        // 中文标签必须 7 个全覆盖 (防 enum 加了新成员忘了翻译)
        let labels: Vec<&str> = [
            OverlayKind::Ditch,
            OverlayKind::WoodWall,
            OverlayKind::StoneWall,
            OverlayKind::Bridge,
            OverlayKind::Road,
            OverlayKind::RuneSlot,
            OverlayKind::Altar,
        ]
        .iter()
        .map(|k| k.label_zh())
        .collect();
        assert_eq!(labels.len(), 7);
        for l in &labels {
            assert!(!l.is_empty(), "label_zh 不能为空");
            // 中文 / 不应是英文 (避免英文 ID 漏配)
            assert!(
                l.chars().any(|c| c as u32 > 127),
                "label_zh 应是中文: {:?}",
                l
            );
        }
    }

    #[test]
    fn stone_wall_stronger_than_wood() {
        assert!(
            OverlayKind::StoneWall.default_durability()
                > OverlayKind::WoodWall.default_durability(),
            "石墙应比木墙耐久高"
        );
    }

    #[test]
    fn walls_block_but_road_bridge_dont() {
        assert!(OverlayKind::WoodWall.blocks_movement());
        assert!(OverlayKind::StoneWall.blocks_movement());
        assert!(OverlayKind::Ditch.blocks_movement());
        assert!(!OverlayKind::Road.blocks_movement());
        assert!(!OverlayKind::Bridge.blocks_movement());
        assert!(!OverlayKind::RuneSlot.blocks_movement());
    }

    #[test]
    fn only_4_kinds_destructible() {
        let destructible: Vec<OverlayKind> = [
            OverlayKind::Ditch,
            OverlayKind::WoodWall,
            OverlayKind::StoneWall,
            OverlayKind::Bridge,
            OverlayKind::Road,
            OverlayKind::RuneSlot,
            OverlayKind::Altar,
        ]
        .iter()
        .copied()
        .filter(|k| k.is_destructible())
        .collect();
        assert_eq!(destructible.len(), 4);
        assert!(destructible.contains(&OverlayKind::WoodWall));
        assert!(destructible.contains(&OverlayKind::StoneWall));
        assert!(destructible.contains(&OverlayKind::Bridge));
        assert!(destructible.contains(&OverlayKind::Altar));
    }

    #[test]
    fn new_overlay_uses_kind_defaults() {
        let o = TerrainOverlayEntity::new(42, OverlayKind::WoodWall, [10.0, 0.0, 5.0]);
        assert_eq!(o.id, 42);
        assert_eq!(o.kind, OverlayKind::WoodWall);
        assert_eq!(o.world_pos, [10.0, 0.0, 5.0]);
        assert!((o.durability - 100.0).abs() < 0.001);
        assert!(!o.is_destroyed());
    }

    #[test]
    fn destroyed_when_durability_zero() {
        let mut o = TerrainOverlayEntity::new(1, OverlayKind::Altar, [0.0, 0.0, 0.0]);
        o.durability = 0.0;
        assert!(o.is_destroyed());

        // 不可破坏的覆盖层永远不算 destroyed
        let mut r = TerrainOverlayEntity::new(2, OverlayKind::Road, [0.0, 0.0, 0.0]);
        r.durability = 0.0;
        assert!(
            !r.is_destroyed(),
            "Road 是不可破坏覆盖层,durability=0 也不应算 destroyed"
        );
        let _ = r;
    }

    #[test]
    fn registry_register_and_unregister() {
        let mut reg = TerrainOverlayRegistry::default();
        assert!(reg.is_empty());

        let o1 = TerrainOverlayEntity::new(1, OverlayKind::WoodWall, [0.0, 0.0, 0.0]);
        assert!(reg.register(&o1));
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.total_spawned, 1);

        // 重复 ID → 失败
        let o1_dup = TerrainOverlayEntity::new(1, OverlayKind::StoneWall, [5.0, 0.0, 5.0]);
        assert!(!reg.register(&o1_dup));
        assert_eq!(reg.len(), 1); // 没增加

        // 注销
        assert!(reg.unregister(1));
        assert_eq!(reg.len(), 0);

        // 重复注销 → 失败
        assert!(!reg.unregister(1));
    }

    #[test]
    fn registry_total_spawned_monotonic() {
        let mut reg = TerrainOverlayRegistry::default();
        for i in 0..5 {
            let o = TerrainOverlayEntity::new(i, OverlayKind::Road, [i as f32, 0.0, 0.0]);
            reg.register(&o);
        }
        assert_eq!(reg.total_spawned, 5);
        // 即使 unregister,total_spawned 也不减 (审计用)
        reg.unregister(0);
        reg.unregister(1);
        assert_eq!(reg.total_spawned, 5);
        assert_eq!(reg.len(), 3);
    }

    #[test]
    fn all_kinds_have_positive_footprint_radius() {
        // 占地半径必须 > 0,否则 spawn 时 AABB 是 0 没法显示
        for k in [
            OverlayKind::Ditch,
            OverlayKind::WoodWall,
            OverlayKind::StoneWall,
            OverlayKind::Bridge,
            OverlayKind::Road,
            OverlayKind::RuneSlot,
            OverlayKind::Altar,
        ] {
            assert!(
                k.default_footprint_radius() > 0.0,
                "{:?} footprint 必须 > 0",
                k
            );
        }
    }
}
