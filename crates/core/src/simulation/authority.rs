use bevy::prelude::*;

use crate::ai::TickObserver;
use crate::clock::SimClock;
use crate::constant;
use crate::ecology::EcoCycle;
use crate::ecology::threats::MonsterEcosystem;
use crate::resource::{GlobalResourcePool, ResourceKind};

use super::{TickReport, WorldInput, step_world_elapsed};

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

    let elapsed_ticks = ((now - clock.last_tick_wall) / constant::SLOW_TICK_SECS)
        .floor()
        .min(constant::MAX_CATCH_UP_TICKS as f32) as u32;
    clock.last_tick_wall += elapsed_ticks as f32 * constant::SLOW_TICK_SECS;
    clock.tick += u64::from(elapsed_ticks);
    for _ in 0..elapsed_ticks {
        obs.begin_tick();
        monsters.tick(pool);
    }
    let _ = step_world_elapsed(WorldInput { tick: clock.tick }, elapsed_ticks, eco, pool);

    if clock.tick % 10 == 0 {
        info!(
            "{} {}: monsters={}, food={}",
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
    advance_fixed_authority_tick_report(delta_secs, clock, pool, monsters, eco, obs, role).is_some()
}

pub fn advance_fixed_authority_tick_report(
    delta_secs: f32,
    clock: &mut SimClock,
    pool: &mut GlobalResourcePool,
    monsters: &mut MonsterEcosystem,
    eco: &mut EcoCycle,
    obs: &mut TickObserver,
    role: SimRole,
) -> Option<TickReport> {
    clock.last_sim_step_ran = false;
    clock.frame_tick += 1;
    clock.slow_tick_accum += delta_secs.max(0.0);

    if clock.slow_tick_accum + f32::EPSILON < constant::SLOW_TICK_SECS {
        return None;
    }

    let elapsed_ticks = (clock.slow_tick_accum / constant::SLOW_TICK_SECS)
        .floor()
        .min(constant::MAX_CATCH_UP_TICKS as f32) as u32;
    clock.slow_tick_accum -= elapsed_ticks as f32 * constant::SLOW_TICK_SECS;
    clock.tick += u64::from(elapsed_ticks);
    clock.last_sim_step_ran = true;
    for _ in 0..elapsed_ticks {
        obs.begin_tick();
        monsters.tick(pool);
    }
    let report = step_world_elapsed(WorldInput { tick: clock.tick }, elapsed_ticks, eco, pool);

    if clock.tick % 10 == 0 {
        info!(
            "{} {}: monsters={}, food={}",
            role.tick_log_label(),
            clock.tick,
            monsters.current_individuals,
            pool.get(ResourceKind::Food)
        );
    }

    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::{NatureEvent, NatureSnapshot};

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

    #[test]
    fn fixed_authority_report_exposes_the_exact_shared_nature_step() {
        let mut clock = SimClock::default();
        let mut pool = GlobalResourcePool::default();
        let mut monsters = MonsterEcosystem::default();
        let mut eco = EcoCycle::seeded_weather_at(Vec2::new(8.0, 8.0));
        let mut obs = TickObserver::default();

        let report = advance_fixed_authority_tick_report(
            constant::SLOW_TICK_SECS,
            &mut clock,
            &mut pool,
            &mut monsters,
            &mut eco,
            &mut obs,
            SimRole::ServerAuthority,
        )
        .expect("one complete fixed interval advances nature");

        assert_eq!(report.tick, clock.tick);
        assert_eq!(
            report.snapshot,
            NatureSnapshot::from_ecology(clock.tick, &eco)
        );
        assert!(
            report
                .events
                .iter()
                .any(|event| matches!(event, NatureEvent::RainFell { .. }))
        );
    }

    #[test]
    fn fixed_authority_catches_up_multiple_elapsed_ticks() {
        let mut clock = SimClock::default();
        let mut pool = GlobalResourcePool::default();
        let mut monsters = MonsterEcosystem::default();
        let mut eco = EcoCycle::seeded_weather_at(Vec2::new(8.0, 8.0));
        let mut obs = TickObserver::default();

        let report = advance_fixed_authority_tick_report(
            constant::SLOW_TICK_SECS * 2.5,
            &mut clock,
            &mut pool,
            &mut monsters,
            &mut eco,
            &mut obs,
            SimRole::ServerAuthority,
        )
        .expect("large frame advances all completed fixed ticks");

        assert_eq!(clock.tick, 2);
        assert_eq!(report.tick, 2);
        assert!((clock.slow_tick_accum - constant::SLOW_TICK_SECS * 0.5).abs() < 0.001);
        assert!(report.snapshot.is_finite());
    }

    #[test]
    fn fixed_authority_limits_catch_up_and_retains_backlog() {
        let mut clock = SimClock::default();
        let mut pool = GlobalResourcePool::default();
        let mut monsters = MonsterEcosystem::default();
        let mut eco = EcoCycle::default();
        let mut obs = TickObserver::default();

        let report = advance_fixed_authority_tick_report(
            constant::SLOW_TICK_SECS * (constant::MAX_CATCH_UP_TICKS as f32 + 3.0),
            &mut clock,
            &mut pool,
            &mut monsters,
            &mut eco,
            &mut obs,
            SimRole::ServerAuthority,
        )
        .expect("large frame still advances a bounded batch");

        assert_eq!(clock.tick, u64::from(constant::MAX_CATCH_UP_TICKS));
        assert_eq!(report.tick, clock.tick);
        assert!(clock.slow_tick_accum >= 2.9);
    }
}
