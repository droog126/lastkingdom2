use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
pub enum RuntimeMode {
    #[default]
    Client,
    Server,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    Boot,
    Loading,
    MainMenu,
    InMatch,
    CrownWorld,
}

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
    pub fn next(self) -> Self {
        match self {
            Self::OpeningProtection => Self::WildRace,
            Self::WildRace => Self::SovereignRise,
            Self::SovereignRise => Self::Endgame,
            Self::Endgame => Self::Finished,
            Self::Finished => Self::Finished,
        }
    }

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
