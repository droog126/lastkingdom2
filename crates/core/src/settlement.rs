use serde::{Deserialize, Serialize};

use crate::resource::{GlobalResourcePool, ResourceKind};

pub const CAMP_WOOD_COST: i64 = 12;
pub const CAMP_STONE_COST: i64 = 6;
pub const CAMP_FOOD_UPKEEP: i64 = 1;
pub const SETTLEMENT_UPKEEP_INTERVAL_TICKS: u32 = 5;
pub const SETTLEMENT_POPULATION_INTERVAL_TICKS: u32 = 10;
pub const SETTLEMENT_GOAL_TICKS: u32 = 30;
pub const SETTLEMENT_MAX_POPULATION: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementPhase {
    Wilderness,
    Camp,
    Established,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementEvent {
    CampBuilt,
    UpkeepPaid { tick: u32, food: i64 },
    SettlerArrived { population: u32 },
    Established,
    Starved { tick: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementError {
    AlreadyStarted,
    MissingWood,
    MissingStone,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementState {
    pub phase: SettlementPhase,
    pub tick: u32,
    pub population: u32,
    pub food_spent: i64,
}

impl Default for SettlementState {
    fn default() -> Self {
        Self {
            phase: SettlementPhase::Wilderness,
            tick: 0,
            population: 0,
            food_spent: 0,
        }
    }
}

impl SettlementState {
    pub fn build_camp(
        &mut self,
        resources: &mut GlobalResourcePool,
    ) -> Result<SettlementEvent, SettlementError> {
        if self.phase != SettlementPhase::Wilderness {
            return Err(SettlementError::AlreadyStarted);
        }
        if resources.get(ResourceKind::Wood) < CAMP_WOOD_COST {
            return Err(SettlementError::MissingWood);
        }
        if resources.get(ResourceKind::Stone) < CAMP_STONE_COST {
            return Err(SettlementError::MissingStone);
        }

        let _ = resources.try_sub(ResourceKind::Wood, CAMP_WOOD_COST);
        let _ = resources.try_sub(ResourceKind::Stone, CAMP_STONE_COST);
        self.phase = SettlementPhase::Camp;
        self.population = 1;
        Ok(SettlementEvent::CampBuilt)
    }

    pub fn advance_tick(&mut self, resources: &mut GlobalResourcePool) -> Vec<SettlementEvent> {
        if self.phase != SettlementPhase::Camp {
            return Vec::new();
        }

        self.tick = self.tick.saturating_add(1);
        let mut events = Vec::with_capacity(3);
        if self.tick % SETTLEMENT_UPKEEP_INTERVAL_TICKS == 0 {
            if resources
                .try_sub(ResourceKind::Food, CAMP_FOOD_UPKEEP)
                .is_err()
            {
                self.phase = SettlementPhase::Failed;
                events.push(SettlementEvent::Starved { tick: self.tick });
                return events;
            }
            self.food_spent += CAMP_FOOD_UPKEEP;
            events.push(SettlementEvent::UpkeepPaid {
                tick: self.tick,
                food: CAMP_FOOD_UPKEEP,
            });
        }

        if self.tick % SETTLEMENT_POPULATION_INTERVAL_TICKS == 0
            && self.population < SETTLEMENT_MAX_POPULATION
        {
            self.population += 1;
            events.push(SettlementEvent::SettlerArrived {
                population: self.population,
            });
        }

        if self.tick >= SETTLEMENT_GOAL_TICKS {
            self.phase = SettlementPhase::Established;
            events.push(SettlementEvent::Established);
        }
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::{GlobalResourcePool, ResourceKind};

    #[test]
    fn camp_build_is_atomic_and_consumes_materials() {
        let mut pool = GlobalResourcePool::new();
        pool.force_add(ResourceKind::Wood, CAMP_WOOD_COST);
        pool.force_add(ResourceKind::Stone, CAMP_STONE_COST);
        let mut settlement = SettlementState::default();

        assert_eq!(
            settlement.build_camp(&mut pool),
            Ok(SettlementEvent::CampBuilt)
        );
        assert_eq!(pool.get(ResourceKind::Wood), 0);
        assert_eq!(pool.get(ResourceKind::Stone), 0);
        assert_eq!(settlement.phase, SettlementPhase::Camp);
        assert_eq!(settlement.population, 1);

        let mut insufficient = GlobalResourcePool::new();
        insufficient.force_add(ResourceKind::Wood, CAMP_WOOD_COST);
        let error = SettlementState::default()
            .build_camp(&mut insufficient)
            .unwrap_err();
        assert_eq!(error, SettlementError::MissingStone);
        assert_eq!(insufficient.get(ResourceKind::Wood), CAMP_WOOD_COST);
    }

    #[test]
    fn camp_upkeep_consumes_food_and_adds_settlers() {
        let mut pool = GlobalResourcePool::new();
        pool.force_add(ResourceKind::Wood, CAMP_WOOD_COST);
        pool.force_add(ResourceKind::Stone, CAMP_STONE_COST);
        pool.force_add(
            ResourceKind::Food,
            CAMP_FOOD_UPKEEP * i64::from(SETTLEMENT_GOAL_TICKS / SETTLEMENT_UPKEEP_INTERVAL_TICKS),
        );
        let mut settlement = SettlementState::default();
        settlement.build_camp(&mut pool).unwrap();

        for _ in 0..SETTLEMENT_GOAL_TICKS {
            settlement.advance_tick(&mut pool);
        }

        assert_eq!(settlement.phase, SettlementPhase::Established);
        assert_eq!(settlement.population, SETTLEMENT_MAX_POPULATION);
        assert_eq!(pool.get(ResourceKind::Food), 0);
        assert_eq!(settlement.food_spent, 6);
    }

    #[test]
    fn camp_fails_when_upkeep_cannot_be_paid() {
        let mut pool = GlobalResourcePool::new();
        pool.force_add(ResourceKind::Wood, CAMP_WOOD_COST);
        pool.force_add(ResourceKind::Stone, CAMP_STONE_COST);
        let mut settlement = SettlementState::default();
        settlement.build_camp(&mut pool).unwrap();

        for _ in 0..SETTLEMENT_UPKEEP_INTERVAL_TICKS {
            settlement.advance_tick(&mut pool);
        }

        assert_eq!(settlement.phase, SettlementPhase::Failed);
        assert_eq!(settlement.tick, SETTLEMENT_UPKEEP_INTERVAL_TICKS);
        assert_eq!(settlement.advance_tick(&mut pool), Vec::new());
    }
}
