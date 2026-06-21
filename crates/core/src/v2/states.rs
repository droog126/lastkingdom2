//! V2 P0 骨架 — Bevy State / RuntimeMode
//!
//! 这是《万国余烬_王冠赛季_V2》重构设计版的 P0 阶段实现。
//! 不替换现有 V1 代码，**只新增** V2 架构原语。
//!
//! 现有 V1 (`lk2-core::match_state`, `lk2-client` 的 `CameraMode`) 与本模块共存:
//! - V1 用 `ResMut<...>` 字段切换模式（运行时可变）
//! - V2 用 Bevy `State<T>` (类型安全, 系统级 run_if)
//!
//! 当 V2 真正接管时，可以把 V1 的字段迁到 V2 的 State 并删除。
//!
//! 参考 docs\万国余烬_王冠赛季\万国余烬_王冠赛季_Bevy0 18架构实现 382d63af464581039accc17054980fc1.md §6 State 设计

use bevy::prelude::*;

/// 运行时模式：客户端还是服务端
///
/// 服务端 `MinimalPlugins` + headless，客户端 `DefaultPlugins` + 渲染。
/// 同一份玩法代码两边都跑（除非 `if mode == RuntimeMode::Client` 才挂表现层）。
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub enum RuntimeMode {
    #[default]
    Client,
    Server,
}

/// 客户端大流程状态 — Bevy `State<AppState>`
///
/// 用途：UI 大流程 / 场景切换 / 加载屏切换。
///
/// 红线（来自 Bevy 架构文档 §6）：
/// - 专服多房间时不能用全局 AppState 表示每场对局
/// - 每场对局的阶段用 `MatchPhase` 资源（见 `crate::match_flow::MatchPhase`）
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Boot,
    Loading,
    MainMenu,
    InMatch,
    CrownWorld,
}

/// 单局对局阶段 — V2 设计的 5 阶段
///
/// 当前阶段含义：
/// - `OpeningProtection` (0-5min): 全局保护期，不能攻击也不能被攻击
/// - `WildRace` (5-25min): 资源争夺 + 局部 PVP 事件
/// - `SovereignRise` (25-45min): 王权火种争夺 + 建国
/// - `Endgame` (45-54min): 全图 PVP + 全局唯一神器
/// - `Finished`: 单局结束
///
/// 红线：
/// - 单机/单房间可用 Bevy `State<MatchState>`
/// - 专服多房间必须用 `Res<MatchPhase>` 而非 State
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum MatchState {
    #[default]
    OpeningProtection,
    WildRace,
    SovereignRise,
    Endgame,
    Finished,
}

impl MatchState {
    /// 阶段顺序，用于 `next_match_state()` 推进
    pub fn next(self) -> Self {
        match self {
            Self::OpeningProtection => Self::WildRace,
            Self::WildRace => Self::SovereignRise,
            Self::SovereignRise => Self::Endgame,
            Self::Endgame => Self::Finished,
            Self::Finished => Self::Finished,
        }
    }

    /// 是否允许 PVP
    pub fn pvp_enabled(self) -> bool {
        match self {
            Self::OpeningProtection => false,
            Self::WildRace | Self::SovereignRise | Self::Endgame => true,
            Self::Finished => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn match_state_ordering() {
        assert_eq!(MatchState::OpeningProtection.next(), MatchState::WildRace);
        assert_eq!(MatchState::WildRace.next(), MatchState::SovereignRise);
        assert_eq!(MatchState::SovereignRise.next(), MatchState::Endgame);
        assert_eq!(MatchState::Endgame.next(), MatchState::Finished);
        // Finished 不再推进
        assert_eq!(MatchState::Finished.next(), MatchState::Finished);
    }

    #[test]
    fn protection_disables_pvp() {
        assert!(!MatchState::OpeningProtection.pvp_enabled());
        assert!(MatchState::WildRace.pvp_enabled());
        assert!(MatchState::SovereignRise.pvp_enabled());
        assert!(MatchState::Endgame.pvp_enabled());
        assert!(!MatchState::Finished.pvp_enabled());
    }

    #[test]
    fn app_state_default_is_boot() {
        assert_eq!(AppState::default(), AppState::Boot);
    }
}
