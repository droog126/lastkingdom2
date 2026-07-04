use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatchPhase {
    AshOpening,

    Wildland,

    SovereignReveal,

    Endgame,
}

impl MatchPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::AshOpening => "AshOpening",
            Self::Wildland => "Wildland",
            Self::SovereignReveal => "SovereignReveal",
            Self::Endgame => "Endgame",
        }
    }

    pub fn label_zh(self) -> &'static str {
        match self {
            Self::AshOpening => "灰烬开局",
            Self::Wildland => "荒野竞逐",
            Self::SovereignReveal => "王权显形",
            Self::Endgame => "终局收束",
        }
    }

    pub fn pvp_open(self) -> bool {
        match self {
            Self::AshOpening => false,
            Self::Wildland | Self::SovereignReveal | Self::Endgame => true,
        }
    }

    pub fn global_protection(self) -> bool {
        match self {
            Self::AshOpening => true,
            Self::Wildland | Self::SovereignReveal | Self::Endgame => false,
        }
    }

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

#[derive(Resource, Debug, Clone)]
pub struct MatchClock {
    pub wall_secs: f32,

    pub phase: MatchPhase,

    pub match_duration_secs: f32,

    pub ends_at_secs: f32,
}

impl Default for MatchClock {
    fn default() -> Self {
        let match_duration_secs = 2700.0;
        Self {
            wall_secs: 0.0,
            phase: MatchPhase::AshOpening,
            match_duration_secs,
            ends_at_secs: match_duration_secs,
        }
    }
}

impl MatchClock {
    pub fn advance(&mut self, delta_secs: f32) {
        self.wall_secs += delta_secs;
    }

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

    pub fn phase_remaining_secs(&self) -> f32 {
        let phase = MatchPhase::from_wall_secs(self.wall_secs);
        let end = match phase {
            MatchPhase::AshOpening => 300.0,
            MatchPhase::Wildland => 1080.0,
            MatchPhase::SovereignReveal => 2100.0,
            MatchPhase::Endgame => self.match_duration_secs,
        };
        (end - self.wall_secs).max(0.0)
    }
}

#[derive(Message, Debug, Clone, Copy)]
pub struct MatchPhaseChanged {
    pub from: MatchPhase,
    pub to: MatchPhase,
    pub at_wall_secs: f32,
}

pub mod config_defaults {

    pub const ASH_OPENING_END_SECS: f32 = 300.0;

    pub const WILDLAND_END_SECS: f32 = 1080.0;

    pub const SOVEREIGN_REVEAL_END_SECS: f32 = 2100.0;

    pub const MATCH_DEFAULT_DURATION_SECS: f32 = 2700.0;
}

pub struct MatchStatePlugin;

impl Plugin for MatchStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MatchClock>()
            .add_message::<MatchPhaseChanged>()
            .add_systems(FixedUpdate, advance_match_clock);
    }
}

pub fn advance_match_clock(
    time: Res<Time>,
    mut clock: ResMut<MatchClock>,
    mut phase_events: MessageWriter<MatchPhaseChanged>,
) {
    let dt = time.delta_secs();
    clock.advance(dt);
    if let Some((from, to)) = clock.refresh_phase() {
        phase_events.write(MatchPhaseChanged { from, to, at_wall_secs: clock.wall_secs });
        info!(
            "[match] phase {} → {} at {:.1}s ({} left)",
            from.label_zh(),
            to.label_zh(),
            clock.wall_secs,
            clock.phase_remaining_secs()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_from_wall_secs_boundaries() {
        assert_eq!(MatchPhase::from_wall_secs(0.0), MatchPhase::AshOpening);
        assert_eq!(MatchPhase::from_wall_secs(299.9), MatchPhase::AshOpening);
        assert_eq!(MatchPhase::from_wall_secs(300.0), MatchPhase::Wildland);
        assert_eq!(MatchPhase::from_wall_secs(1079.9), MatchPhase::Wildland);
        assert_eq!(
            MatchPhase::from_wall_secs(1080.0),
            MatchPhase::SovereignReveal
        );
        assert_eq!(
            MatchPhase::from_wall_secs(2099.9),
            MatchPhase::SovereignReveal
        );
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
        c.wall_secs = 1500.0;
        assert!((c.phase_remaining_secs() - 600.0).abs() < 0.5);
    }

    #[test]
    fn phase_remaining_projects_from_wall_secs_even_before_refresh() {
        let mut c = MatchClock::default();
        assert_eq!(c.phase, MatchPhase::AshOpening);
        c.wall_secs = 600.0;
        assert!((c.phase_remaining_secs() - 480.0).abs() < 0.5);
    }
}
