use bevy::prelude::*;

use crate::ai::TickObserver;
use crate::clock::SimClock;
use crate::constant;
use crate::eco_cycle::EcoCycle;
use crate::monster::MonsterEcosystem;
use crate::resource::{GlobalResourcePool, ResourceKind};
use crate::simulation::{WorldInput, step_world};

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
    eco: &mut EcoCycle,
    obs: &mut TickObserver,
    role: SimRole,
) -> bool {
    let now = time.elapsed_secs();
    if now - clock.last_tick_wall < constant::SLOW_TICK_SECS {
        return false;
    }

    clock.last_tick_wall = now;
    clock.tick += 1;
    obs.begin_tick();
    monsters.tick(pool);
    let _ = step_world(WorldInput { tick: clock.tick }, eco, pool);

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

pub fn advance_fixed_authority_tick(
    delta_secs: f32,
    clock: &mut SimClock,
    pool: &mut GlobalResourcePool,
    monsters: &mut MonsterEcosystem,
    eco: &mut EcoCycle,
    obs: &mut TickObserver,
    role: SimRole,
) -> bool {
    clock.last_sim_step_ran = false;
    clock.frame_tick += 1;
    clock.slow_tick_accum += delta_secs.max(0.0);

    if clock.slow_tick_accum + f32::EPSILON < constant::SLOW_TICK_SECS {
        return false;
    }

    clock.slow_tick_accum -= constant::SLOW_TICK_SECS;
    clock.tick += 1;
    clock.last_sim_step_ran = true;
    obs.begin_tick();
    monsters.tick(pool);
    let _ = step_world(WorldInput { tick: clock.tick }, eco, pool);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_authority_frame_advances_every_call_but_sim_tick_waits_one_second() {
        let mut clock = SimClock::default();
        let mut pool = GlobalResourcePool::default();
        let mut monsters = MonsterEcosystem::default();
        let mut eco = EcoCycle {
            clouds: Vec::new(),
            rabbits: Vec::new(),
            berries: Vec::new(),
            wildlife: Vec::new(),
            plants: Vec::new(),
            co2: 0.0,
            rain: 0.0,
            rainfall: 0.0,
            fruit_eaten: 0,
            fruit_grown: 0,
            plants_grown: 0,
            rabbits_born: 0,
            wildlife_born: 0,
        };
        let mut obs = TickObserver::default();

        let initial_food = pool.get(ResourceKind::Food);
        for _ in 0..29 {
            let ran_slow = advance_fixed_authority_tick(
                1.0 / 30.0,
                &mut clock,
                &mut pool,
                &mut monsters,
                &mut eco,
                &mut obs,
                SimRole::ServerAuthority,
            );
            assert!(!ran_slow);
        }

        assert_eq!(clock.frame_tick, 29);
        assert_eq!(clock.tick, 0);
        assert_eq!(pool.get(ResourceKind::Food), initial_food);

        let ran_slow = advance_fixed_authority_tick(
            1.0 / 30.0,
            &mut clock,
            &mut pool,
            &mut monsters,
            &mut eco,
            &mut obs,
            SimRole::ServerAuthority,
        );

        assert!(ran_slow);
        assert_eq!(clock.frame_tick, 30);
        assert_eq!(clock.tick, 1);
        assert_eq!(pool.get(ResourceKind::Food), initial_food);
    }
}
