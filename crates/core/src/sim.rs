use bevy::prelude::*;

use crate::ai::TickObserver;
use crate::clock::SimClock;
use crate::constant;
use crate::monster::MonsterEcosystem;
use crate::resource::{GlobalResourcePool, ResourceKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimRole {
    ClientOffline,
    ServerAuthority,
}

impl SimRole {
    pub fn tick_log_label(self) -> &'static str {
        match self {
            Self::ClientOffline => "tick",
            Self::ServerAuthority => "server tick",
        }
    }

    pub fn state_role(self) -> &'static str {
        match self {
            Self::ClientOffline => "client_offline",
            Self::ServerAuthority => "server",
        }
    }
}

pub fn advance_demo_tick(
    time: &Time,
    clock: &mut SimClock,
    pool: &mut GlobalResourcePool,
    monsters: &mut MonsterEcosystem,
    obs: &mut TickObserver,
    role: SimRole,
) -> bool {
    let now = time.elapsed_secs();
    if now - clock.last_tick_wall < constant::SLOW_TICK_SECS {
        return false;
    }

    clock.last_tick_wall = now;
    clock.tick += 1;
    let _ = pool.try_add(ResourceKind::Apple, 1);
    let _ = pool.try_add(ResourceKind::Food, 2);
    obs.begin_tick();
    monsters.tick(pool);

    if clock.tick % 10 == 0 {
        info!(
            "⏱ {} {}: monsters={}, food={}",
            role.tick_log_label(),
            clock.tick,
            monsters.current_individuals,
            pool.get(ResourceKind::Food)
        );
    }

    true
}
