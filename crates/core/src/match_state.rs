//! V2 Match 阶段系统
//!
//! 来源: 《万国余烬_王冠赛季_对战设计案》§3 (单局阶段)
//! + 《技术实现 MVP》§2 MatchService 验收
//!
//! 4 个阶段:
//! - `AshOpening`     0..=300s   灰烬开局(全局保护期)
//! - `Wildland`       300..=1080s 荒野竞逐(普通 PVP 开放)
//! - `SovereignReveal` 1080..=2100s 王权显形(高价值事件)
//! - `Endgame`        2100s+     终局收束(全图 PVP)
//!
//! 设计要点:
//! - `MatchClock` 是 sim 累计 wall-time 秒(权威层),与 `SimClock.tick` 解耦:
//!   `tick` 是节拍计数(可能 1s/2s/5s 一次), `wall_secs` 是连续时间。
//! - `MatchPhase` 切换时发 `MatchPhaseChanged` Message,
//!   客户端/服务端都订阅这个消息,触发 UI / 音乐 / 终局收束。
//! - 阶段阈值都从 `GameTables` 走配置,这里只给默认常量。

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

/// 单局阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatchPhase {
    /// 0-5 min: 全局保护期
    AshOpening,
    /// 5-18 min: 普通 PVP 开放,基础资源争夺
    Wildland,
    /// 18-35 min: 高价值事件、王权火种显形、建国窗口
    SovereignReveal,
    /// 35+ min: 全图 PVP、王座进度
    Endgame,
}

impl MatchPhase {
    /// 阶段显示名(英文,匹配项目惯例;中文显示走 UI 翻译表)
    pub fn label(self) -> &'static str {
        match self {
            Self::AshOpening => "AshOpening",
            Self::Wildland => "Wildland",
            Self::SovereignReveal => "SovereignReveal",
            Self::Endgame => "Endgame",
        }
    }

    /// 阶段中文(给 HUD 直接显示用)
    pub fn label_zh(self) -> &'static str {
        match self {
            Self::AshOpening => "灰烬开局",
            Self::Wildland => "荒野竞逐",
            Self::SovereignReveal => "王权显形",
            Self::Endgame => "终局收束",
        }
    }

    /// 该阶段是否允许 PVP(全局规则;中途加入 5min 个人保护在 `protection` 模块处理)
    pub fn pvp_open(self) -> bool {
        match self {
            Self::AshOpening => false,
            Self::Wildland | Self::SovereignReveal | Self::Endgame => true,
        }
    }

    /// 该阶段是否对所有玩家开全局保护(所有人不能打不能被打)
    pub fn global_protection(self) -> bool {
        match self {
            Self::AshOpening => true,
            Self::Wildland | Self::SovereignReveal | Self::Endgame => false,
        }
    }

    /// 把 wall-time(秒)投影到阶段
    pub fn from_wall_secs(wall_secs: f32) -> Self {
        if wall_secs < 300.0 {
            Self::AshOpening
        } else if wall_secs < 1080.0 {
            Self::Wildland
        } else if wall_secs < 2100.0 {
            Self::SovereignReveal
        } else {
            Self::Endgame
        }
    }
}

/// 单局时钟(权威层)
///
/// `wall_secs` 是 sim 自启动以来的累计 wall-time 秒。
/// `phase` 是当前阶段(由 wall_secs 投影,可手动 override)。
/// `match_duration_secs` 默认 2700 (45min),可被 `GameTables` 覆盖。
#[derive(Resource, Debug, Clone)]
pub struct MatchClock {
    /// 累计 wall-time 秒(权威)
    pub wall_secs: f32,
    /// 当前阶段
    pub phase: MatchPhase,
    /// 单局目标时长(秒)
    pub match_duration_secs: f32,
    /// 单局结束时间戳(用于终局判定)
    pub ends_at_secs: f32,
}

impl Default for MatchClock {
    fn default() -> Self {
        let match_duration_secs = 2700.0; // 45 min
        Self {
            wall_secs: 0.0,
            phase: MatchPhase::AshOpening,
            match_duration_secs,
            ends_at_secs: match_duration_secs,
        }
    }
}

impl MatchClock {
    /// 累积 wall-time(每 FixedUpdate 调一次)
    pub fn advance(&mut self, delta_secs: f32) {
        self.wall_secs += delta_secs;
    }

    /// 重新投影 phase,返回 `Some((from, to))` 如果发生切换,否则 `None`
    pub fn refresh_phase(&mut self) -> Option<(MatchPhase, MatchPhase)> {
        let new_phase = MatchPhase::from_wall_secs(self.wall_secs);
        if new_phase != self.phase {
            let from = self.phase;
            self.phase = new_phase;
            Some((from, new_phase))
        } else {
            None
        }
    }

    /// 距离阶段结束的剩余秒数
    pub fn phase_remaining_secs(&self) -> f32 {
        let end = match self.phase {
            MatchPhase::AshOpening => 300.0,
            MatchPhase::Wildland => 1080.0,
            MatchPhase::SovereignReveal => 2100.0,
            MatchPhase::Endgame => self.match_duration_secs,
        };
        (end - self.wall_secs).max(0.0)
    }
}

/// 阶段切换事件
///
/// 监听方:
/// - 客户端 UI: 切音乐 / 切 HUD 文案 / 切 minimap 警告
/// - 服务端: 触发事件 Service 切换(`Wildland` 时开放肉鸽事件,`Endgame` 时收束)
/// - 客户端 PvP 预测: 全局保护 → 玩家攻击 input 在 `AshOpening` 内被吞
#[derive(Message, Debug, Clone, Copy)]
pub struct MatchPhaseChanged {
    pub from: MatchPhase,
    pub to: MatchPhase,
    pub at_wall_secs: f32,
}

/// 阶段时间表(配置层入口;MVP 默认值)
pub mod config_defaults {
    /// 灰烬开局 0-5 min
    pub const ASH_OPENING_END_SECS: f32 = 300.0;
    /// 荒野竞逐 5-18 min
    pub const WILDLAND_END_SECS: f32 = 1080.0;
    /// 王权显形 18-35 min
    pub const SOVEREIGN_REVEAL_END_SECS: f32 = 2100.0;
    /// 终局 35+ min,推荐对局 45 min
    pub const MATCH_DEFAULT_DURATION_SECS: f32 = 2700.0;
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// 玩法 Plugin: 注册 `MatchClock` 资源 + 阶段推进 + 阶段切换 Message
///
/// 注册方式(`lk2-client` / `lk2-server` / `lk2-core::sim` 任一):
/// ```ignore
/// app.add_plugins(MatchStatePlugin);
/// ```
pub struct MatchStatePlugin;

impl Plugin for MatchStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MatchClock>()
            .add_message::<MatchPhaseChanged>()
            .add_systems(FixedUpdate, advance_match_clock);
    }
}

/// 推进 `MatchClock` 并在阶段切换时发 `MatchPhaseChanged` Message
pub fn advance_match_clock(
    time: Res<Time>,
    mut clock: ResMut<MatchClock>,
    mut phase_events: MessageWriter<MatchPhaseChanged>,
) {
    // 用 fixed timestep 累积(0.1s 一拍的话用 `Time::delta_secs()` 即可,
    // 服务端 30Hz FixedUpdate 一次大概 0.033s)
    let dt = time.delta_secs();
    clock.advance(dt);
    if let Some((from, to)) = clock.refresh_phase() {
        phase_events.write(MatchPhaseChanged {
            from,
            to,
            at_wall_secs: clock.wall_secs,
        });
        info!(
            "[match] phase {} → {} at {:.1}s ({} left)",
            from.label_zh(),
            to.label_zh(),
            clock.wall_secs,
            clock.phase_remaining_secs()
        );
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_from_wall_secs_boundaries() {
        assert_eq!(MatchPhase::from_wall_secs(0.0), MatchPhase::AshOpening);
        assert_eq!(MatchPhase::from_wall_secs(299.9), MatchPhase::AshOpening);
        assert_eq!(MatchPhase::from_wall_secs(300.0), MatchPhase::Wildland);
        assert_eq!(MatchPhase::from_wall_secs(1079.9), MatchPhase::Wildland);
        assert_eq!(MatchPhase::from_wall_secs(1080.0), MatchPhase::SovereignReveal);
        assert_eq!(MatchPhase::from_wall_secs(2099.9), MatchPhase::SovereignReveal);
        assert_eq!(MatchPhase::from_wall_secs(2100.0), MatchPhase::Endgame);
    }

    #[test]
    fn phase_protection_rules() {
        assert!(MatchPhase::AshOpening.global_protection());
        assert!(!MatchPhase::AshOpening.pvp_open());
        assert!(!MatchPhase::Wildland.global_protection());
        assert!(MatchPhase::Wildland.pvp_open());
        assert!(MatchPhase::Endgame.pvp_open());
    }

    #[test]
    fn clock_phase_change_emits() {
        let mut c = MatchClock::default();
        assert_eq!(c.phase, MatchPhase::AshOpening);
        c.wall_secs = 299.5;
        assert!(c.refresh_phase().is_none());
        c.wall_secs = 300.0;
        let changed = c.refresh_phase();
        assert!(changed.is_some());
        let (from, to) = changed.unwrap();
        assert_eq!(from, MatchPhase::AshOpening);
        assert_eq!(to, MatchPhase::Wildland);
        assert_eq!(c.phase, MatchPhase::Wildland);
    }

    #[test]
    fn phase_remaining() {
        let mut c = MatchClock::default();
        c.wall_secs = 100.0;
        assert!((c.phase_remaining_secs() - 200.0).abs() < 0.5);
        c.wall_secs = 1500.0; // SovereignReveal
        assert!((c.phase_remaining_secs() - 600.0).abs() < 0.5);
    }
}
