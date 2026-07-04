use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::match_state::MatchClock;
use crate::player::PlayerTag;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProtectionSource {
    Opening,

    MidJoin,

    Event,

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
        Self::new(300.0, ProtectionSource::Opening)
    }

    pub fn mid_join() -> Self {
        Self::new(300.0, ProtectionSource::MidJoin)
    }
}

pub mod config_defaults {
    pub const MID_JOIN_PROTECTION_SECS: f32 = 300.0;
    pub const OPENING_PROTECTION_SECS: f32 = 300.0;
}

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

pub fn can_be_attacked(target_protection: Option<&Protection>, match_clock: &MatchClock) -> bool {
    if match_clock.phase.global_protection() {
        return false;
    }
    if target_protection.is_some() {
        return false;
    }
    true
}

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

pub fn ensure_opening_protection(
    match_clock: Res<MatchClock>,
    mut commands: Commands,
    players: Query<Entity, (With<PlayerTag>, Without<Protection>)>,
    mut initialized: Local<bool>,
) {
    if *initialized {
        return;
    }
    if !matches!(
        match_clock.phase,
        crate::match_state::MatchPhase::AshOpening
    ) {
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

pub struct ProtectionPlugin;

impl Plugin for ProtectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (tick_protection, ensure_opening_protection).chain(),
        );
    }
}

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

        assert!(!can_be_attacked(Some(&prot), &c));

        assert!(!can_attack(None, Some(&prot), &c));
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
