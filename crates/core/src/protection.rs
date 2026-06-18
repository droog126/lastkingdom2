//! V2 保护期系统
//!
//! 来源: 《万国余烬_王冠赛季_对战设计案》§4 (中途加入保护)
//! + 《技术实现 MVP》§2 ProtectionService 验收
//!
//! 两种保护:
//! - **全局保护** (Global): `MatchPhase::AshOpening` 期间所有玩家不能打不能被打
//! - **个人保护** (Personal): 中途加入的玩家 5 分钟内不能打不能被打
//!
//! 设计要点:
//! - 保护是 **Component**(挂玩家 entity),不是 Resource,
//!   因为每个玩家的保护到期时间不同(中途加入的戳不同)
//! - 全局保护通过 `can_attack` / `can_be_attacked` 谓词统一检查,
//!   谓词里会同时看 `MatchClock.phase` 和玩家的 `Protection` 组件
//! - 系统在 `FixedUpdate` 倒计时,到期自动移除组件(用 `Commands.entity(e).remove::<Protection>()`)

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::match_state::MatchClock;
use crate::player::PlayerTag;

/// 保护来源
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtectionSource {
    /// 开局全局保护(`MatchPhase::AshOpening`)
    Opening,
    /// 中途加入 5 分钟个人保护
    MidJoin,
    /// 事件触发(肉鸽事件局部保护,留给 T2 推)
    Event,
    /// 救援脱困(可破坏地形救援后短暂无敌,留给 T3 推)
    Rescue,
}

impl ProtectionSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Opening => "opening",
            Self::MidJoin => "mid_join",
            Self::Event => "event",
            Self::Rescue => "rescue",
        }
    }
}

/// 玩家保护期组件
///
/// 挂在玩家 entity 上。`remaining_secs <= 0` 时由 `tick_protection` 自动移除。
/// 玩家**只能同时有一个** Protection 组件;中途加入保护被事件保护覆盖时
/// 走 `Commands.entity(e).insert(...)` 替换。
#[derive(Component, Debug, Clone, Copy)]
pub struct Protection {
    pub remaining_secs: f32,
    pub source: ProtectionSource,
}

impl Protection {
    pub fn new(remaining_secs: f32, source: ProtectionSource) -> Self {
        Self { remaining_secs, source }
    }

    pub fn opening() -> Self {
        Self::new(300.0, ProtectionSource::Opening) // 5 min
    }

    pub fn mid_join() -> Self {
        Self::new(300.0, ProtectionSource::MidJoin) // 5 min
    }
}

/// 个人保护期长度配置(秒);MVP 默认 5 分钟
pub mod config_defaults {
    pub const MID_JOIN_PROTECTION_SECS: f32 = 300.0;
    pub const OPENING_PROTECTION_SECS: f32 = 300.0;
}

// ---------------------------------------------------------------------------
// Predicate: 谁可以打 / 谁可以被打
// ---------------------------------------------------------------------------

/// 攻击者能否在 `match_clock.phase` 下打 `target`
///
/// 规则(V2 对战设计案 §3.1, §4):
/// - `AshOpening` 全局保护 → 双方都不行
/// - `target` 有 `Protection` 组件 → 不行
/// - 攻击者自己有 `Protection` 组件 → 不行(中途加入者 5min 内不能主动攻击)
/// - 其余情况放行
pub fn can_attack(
    attacker_protection: Option<&Protection>,
    target_protection: Option<&Protection>,
    match_clock: &MatchClock,
) -> bool {
    if match_clock.phase.global_protection() {
        return false;
    }
    if attacker_protection.is_some() {
        return false;
    }
    if target_protection.is_some() {
        return false;
    }
    true
}

/// 目标能否被攻击
pub fn can_be_attacked(
    target_protection: Option<&Protection>,
    match_clock: &MatchClock,
) -> bool {
    if match_clock.phase.global_protection() {
        return false;
    }
    if target_protection.is_some() {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// 在 `FixedUpdate` 倒计时,到期移除组件
pub fn tick_protection(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Protection)>,
) {
    let dt = time.delta_secs();
    for (e, mut prot) in q.iter_mut() {
        prot.remaining_secs -= dt;
        if prot.remaining_secs <= 0.0 {
            debug!(
                "[protection] expired for {:?} (source={})",
                e,
                prot.source.label()
            );
            commands.entity(e).remove::<Protection>();
        }
    }
}

/// 开局全局保护: 进入 `MatchPhase::AshOpening` 时,给**所有**玩家挂 `Protection::opening()`
///
/// 切换到其他阶段时(在 `MatchClock` 里实现),不主动移除——因为
/// `can_attack` 已经会看 `match_clock.phase.global_protection()`,
/// 全局保护结束后谓词自动放行。
///
/// 此 system 只在阶段**首次进入** `AshOpening` 时触发。
/// 用 `Local<bool>` 守门,保证只跑一次。
pub fn ensure_opening_protection(
    match_clock: Res<MatchClock>,
    mut commands: Commands,
    players: Query<Entity, (With<PlayerTag>, Without<Protection>)>,
    mut initialized: Local<bool>,
) {
    if *initialized {
        return;
    }
    if !matches!(match_clock.phase, crate::match_state::MatchPhase::AshOpening) {
        return;
    }
    let mut count = 0;
    for e in players.iter() {
        commands.entity(e).insert(Protection::opening());
        count += 1;
    }
    *initialized = true;
    info!(
        "[protection] opening protection granted to {} players",
        count
    );
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

/// 玩法 Plugin: 注册 protection systems
///
/// 注意: `MatchClock` 资源在 `MatchStatePlugin` 里注册;本 plugin 假设它已存在。
/// 用法:
/// ```ignore
/// app.add_plugins((MatchStatePlugin, ProtectionPlugin));
/// ```
pub struct ProtectionPlugin;

impl Plugin for ProtectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (tick_protection, ensure_opening_protection).chain(),
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
    fn ash_opening_global_protection_blocks() {
        let c = MatchClock::default();
        assert!(matches!(
            c.phase,
            crate::match_state::MatchPhase::AshOpening
        ));
        assert!(c.phase.global_protection());
        assert!(!can_attack(None, None, &c));
        assert!(!can_be_attacked(None, &c));
    }

    #[test]
    fn wildland_unprotected_can_attack() {
        let mut c = MatchClock::default();
        c.wall_secs = 600.0;
        c.refresh_phase();
        assert!(matches!(c.phase, crate::match_state::MatchPhase::Wildland));
        assert!(can_attack(None, None, &c));
    }

    #[test]
    fn midjoin_protected_player_cannot_attack() {
        let mut c = MatchClock::default();
        c.wall_secs = 600.0;
        c.refresh_phase();
        let prot = Protection::mid_join();
        assert!(!can_attack(Some(&prot), None, &c));
        // 同时也不能被打
        assert!(!can_be_attacked(Some(&prot), &c));
        // 但别人可以打我
        assert!(can_attack(None, Some(&prot), &c));
    }

    #[test]
    fn protection_default_durations() {
        let p = Protection::opening();
        assert_eq!(p.source, ProtectionSource::Opening);
        assert!((p.remaining_secs - 300.0).abs() < 0.1);
        let p = Protection::mid_join();
        assert_eq!(p.source, ProtectionSource::MidJoin);
        assert!((p.remaining_secs - 300.0).abs() < 0.1);
    }
}
