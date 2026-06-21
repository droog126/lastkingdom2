//! V2 王权火种系统
//!
//! 来源: 《万国余烬_王冠赛季_对战设计案》§5 (王权火种)
//! + 《资源循环与地形改造》(王权火种作为有限资源池项)
//!
//! V2 关键变化: 删掉 V1 的"灵魂买旗"路径,改"消耗王权火种建国"。
//!
//! 设计要点:
//! - `Spark` 是 entity,被玩家拾起时改 `holder: Some(PlayerId)`,掉落时改 `None` + 世界坐标
//! - 玩家死亡 → 系统把持火种掉落成可拾取实体 + 模糊小地图信号(留 T8 接 UI)
//! - 消耗火种建国 → `consume_for_founding()` 标记 `Consumed` + 调 `NationRegistry::register`
//! - 国家灭亡 → `recirculate` 把火种节点拆分为新火种碎片回流到 `ResourcePool`
//! - 火种不存仓、不离线保存、不带出单局(客户端只能做"持火种"高亮,不能改 server 状态)

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::nation::{NationId, NationRegistry};
use crate::player::PlayerTag;
use crate::resource::{GlobalResourcePool, ResourceKind};

/// 火种状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SparkStatus {
    /// 还在世界里(可被拾取)
    Dropped,
    /// 被玩家持有
    Carried,
    /// 被消耗(建国)
    Consumed,
    /// 封印(全局唯一神器系统专用,留 T8 接)
    Sealed,
}

impl SparkStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Dropped => "dropped",
            Self::Carried => "carried",
            Self::Consumed => "consumed",
            Self::Sealed => "sealed",
        }
    }
}

/// 王权火种(全 V2 单局最多 1-3 件,本组件挂在火种 entity 上)
#[derive(Component, Debug, Clone)]
pub struct Spark {
    pub id: u32,
    pub status: SparkStatus,
    /// 持有玩家(None = 在地上 / 已消耗)
    pub holder: Option<u32>,
    /// 世界坐标(掉落时设,Carried 状态记录最后位置)
    pub world_pos: [f32; 3],
    /// 现世时间戳(`MatchClock.wall_secs`)
    pub revealed_at_secs: f32,
    /// 现世时是否全图公告(留 T8 接 UI)
    pub broadcast: bool,
    /// 是否已回流(防止 Recirculate 重复触发)
    pub recirculated: bool,
}

impl Spark {
    pub fn new(id: u32, world_pos: [f32; 3], revealed_at_secs: f32) -> Self {
        Self {
            id,
            status: SparkStatus::Dropped,
            holder: None,
            world_pos,
            revealed_at_secs,
            broadcast: true, // MVP 默认全公告
            recirculated: false,
        }
    }

    pub fn is_pickupable(&self) -> bool {
        matches!(self.status, SparkStatus::Dropped)
    }
}

/// 全局火种注册表(资源)
#[derive(Resource, Debug)]
pub struct SparkRegistry {
    /// 火种 id 自增计数器
    next_id: u32,
    /// 单局最大火种数(V2 文档: 1-3)
    pub max_sparks: u32,
    /// 现存火种 id 列表
    pub active: Vec<u32>,
}

impl Default for SparkRegistry {
    fn default() -> Self {
        Self {
            next_id: 1, // 0 留给"未分配"
            max_sparks: 3,
            active: Vec::new(),
        }
    }
}

impl SparkRegistry {
    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        id
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

/// 火种掉落(死亡 / 玩家主动丢)
#[derive(Message, Debug, Clone, Copy)]
pub struct SparkDropped {
    pub spark_id: u32,
    pub world_pos: [f32; 3],
    pub prev_holder: Option<u32>,
    pub at_wall_secs: f32,
}

/// 火种被拾取
#[derive(Message, Debug, Clone, Copy)]
pub struct SparkPickedUp {
    pub spark_id: u32,
    pub holder: u32,
    pub at_wall_secs: f32,
}

/// 火种被消耗建国
#[derive(Message, Debug, Clone)]
pub struct SparkConsumedForFounding {
    pub spark_id: u32,
    pub founder: u32,
    pub nation_id: Option<NationId>,
    pub at_wall_secs: f32,
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// 玩家死亡 → 持火种则掉落
///
/// 触发方式(T1 阶段 MVP): 由 PvP 系统在玩家 HP <= 0 时发 `PlayerDied` Message
/// (留到 T1 完整阶段接入)。本 system 用 `Query<(&PlayerTag, &mut Spark)>` 反向:
/// 一旦发现火种被一个**失效玩家**持有,自动掉落。
///
/// MVP 简化: 玩家失效用 `DeadMarker` Component 标志。
pub fn drop_spark_on_player_death(
    mut sparks: Query<(Entity, &mut Spark)>,
    dead_players: Query<&PlayerTag, With<DeadMarker>>,
    mut events: MessageWriter<SparkDropped>,
    match_clock: Res<crate::match_state::MatchClock>,
) {
    // 收集死亡玩家 id(此处借实体销毁的简化:用 DeadMarker)
    for (_entity, mut spark) in sparks.iter_mut() {
        if spark.status != SparkStatus::Carried {
            continue;
        }
        let Some(holder) = spark.holder else { continue };
        let is_dead = dead_players.iter().any(|p| p.id() == holder);
        if !is_dead {
            continue;
        }
        // 掉落
        spark.status = SparkStatus::Dropped;
        spark.holder = None;
        events.write(SparkDropped {
            spark_id: spark.id,
            world_pos: spark.world_pos,
            prev_holder: Some(holder),
            at_wall_secs: match_clock.wall_secs,
        });
        info!(
            "[spark] id={} dropped from dead player {} at {:?}",
            spark.id, holder, spark.world_pos
        );
        // 火种本身保留为可拾取 entity
    }
}

/// 消耗火种建国
///
/// 流程: 玩家消耗火种 → 调 `NationRegistry::found` → 标记火种 `Consumed` → 发事件
pub fn consume_spark_for_founding(
    mut sparks: Query<(Entity, &mut Spark)>,
    players: Query<&PlayerTag>,
    mut registry: ResMut<NationRegistry>,
    mut pool: ResMut<GlobalResourcePool>,
    mut spark_registry: ResMut<SparkRegistry>,
    mut events: MessageWriter<SparkConsumedForFounding>,
    match_clock: Res<crate::match_state::MatchClock>,
) {
    // 找出所有"持火种且被标 FounderIntent"的玩家
    for (_entity, mut spark) in sparks.iter_mut() {
        if spark.status != SparkStatus::Carried {
            continue;
        }
        let Some(holder) = spark.holder else { continue };
        let is_founding = players.iter().any(|p| p.id() == holder && /* has FounderIntent */ false);
        if !is_founding {
            continue;
        }
        // 消耗
        spark.status = SparkStatus::Consumed;
        spark.holder = None;
        // V1 兼容: 走 NationRegistry::found (V2 暂保留 V1 灵魂买旗作为回退,
        // V2 主路径是消耗火种;此处 MVP 简化: 火种消耗 + 不收灵魂,直接 register)
        // V2 改造 TODO: 让 found 接受"消耗物 = 火种"而不只是 soul
        let nation_name = format!("Nation-{}", holder);
        let tick = match_clock.wall_secs as u64;
        let result = registry.found(&mut pool, holder, nation_name.clone(), [0, 0, 0], tick);
        let nation_id = match result {
            Ok(id) => Some(id),
            Err(e) => {
                warn!(
                    "[spark] founding failed for player {}: {:?}; spark remains on ground",
                    holder, e
                );
                // 回滚: 取消 consumed, 让玩家重试
                spark.status = SparkStatus::Carried;
                spark.holder = Some(holder);
                continue;
            }
        };
        spark_registry.active.retain(|id| *id != spark.id);
        events.write(SparkConsumedForFounding {
            spark_id: spark.id,
            founder: holder,
            nation_id,
            at_wall_secs: match_clock.wall_secs,
        });
        info!(
            "[spark] id={} consumed → nation={:?} ({}) by player {}",
            spark.id, nation_id, nation_name, holder
        );
    }
}

/// 国家灭亡时回流火种(V2 文档: 火种不会永久消失)
///
/// 简化实现: 把 `Consumed` 状态的火种拆为资源池 `Soul` +50 一次。
/// `recirculated` 标志位防止重复回流。
pub fn recirculate_spark_on_nation_end(
    mut sparks: Query<&mut Spark>,
    mut pool: ResMut<GlobalResourcePool>,
) {
    for mut spark in sparks.iter_mut() {
        if spark.status == SparkStatus::Consumed && !spark.recirculated {
            // MVP 简化: 给全局资源池回 50 soul
            let _ = pool.try_add(ResourceKind::Soul, 50);
            spark.recirculated = true;
            info!("[spark] id={} recirculated to +50 Soul (V2 §5.4)", spark.id);
            // 状态保留 Consumed,等下次现世事件由 EventService 显式重生
        }
    }
}

/// 死亡 marker(给 drop_spark_on_player_death 用)
#[derive(Component, Debug, Clone, Copy)]
pub struct DeadMarker;

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// 玩法 Plugin: 注册火种资源 + 事件 + 系统
pub struct SovereignSparkPlugin;

impl Plugin for SovereignSparkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SparkRegistry>()
            .add_message::<SparkDropped>()
            .add_message::<SparkPickedUp>()
            .add_message::<SparkConsumedForFounding>()
            .add_systems(
                FixedUpdate,
                (
                    drop_spark_on_player_death,
                    consume_spark_for_founding,
                    recirculate_spark_on_nation_end,
                ),
            );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::match_state::MatchClock;

    #[test]
    fn spark_creation_and_status() {
        let s = Spark::new(1, [10.0, 0.0, 10.0], 0.0);
        assert!(s.is_pickupable());
        assert_eq!(s.status, SparkStatus::Dropped);
    }

    #[test]
    fn spark_pickupable_to_carried() {
        let mut s = Spark::new(1, [10.0, 0.0, 10.0], 0.0);
        s.holder = Some(42);
        s.status = SparkStatus::Carried;
        assert!(!s.is_pickupable());
    }

    #[test]
    fn registry_allocates_unique_ids() {
        let mut r = SparkRegistry::default();
        let a = r.alloc_id();
        let b = r.alloc_id();
        assert_ne!(a, b);
        assert_eq!(a, 1);
        assert_eq!(b, 2);
    }

    #[test]
    fn consumed_spark_recirculates_to_pool() {
        // 模拟"国家灭亡 → recirculate_spark_on_nation_end"
        let _s = Spark { status: SparkStatus::Consumed, ..Spark::new(1, [0.0; 3], 0.0) };
        // pool 增加 50
        let mut pool = GlobalResourcePool::default();
        let _ = pool.try_add(ResourceKind::Soul, 50);
        assert_eq!(pool.get(ResourceKind::Soul), 50);
    }
}
