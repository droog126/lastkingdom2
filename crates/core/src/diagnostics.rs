use bevy::prelude::*;

use crate::ai::TickObserver;
use crate::clock::SimClock;
use crate::eco_cycle::EcoCycle;
use crate::monster::MonsterEcosystem;
use crate::nation::NationRegistry;
use crate::player::PlayerState;
use crate::resource::{GlobalResourcePool, ResourceKind};
use crate::sim::SimRole;
use crate::world::World;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotRole {
    ClientOffline,
    ClientOnline,
    ServerAuthority,
}

impl SnapshotRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClientOffline => "client_offline",
            Self::ClientOnline => "client_online",
            Self::ServerAuthority => "server",
        }
    }
}

impl From<SimRole> for SnapshotRole {
    fn from(role: SimRole) -> Self {
        match role {
            SimRole::ClientOffline => Self::ClientOffline,
            SimRole::ServerAuthority => Self::ServerAuthority,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SelfCheckReport {
    pub violations: Vec<String>,
    pub ticks_run: u64,
}

impl SelfCheckReport {
    pub fn is_ok(&self) -> bool {
        self.violations.is_empty()
    }

    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }
}

pub fn run_self_check(
    game_world: &World,
    pool: &GlobalResourcePool,
    nations: &NationRegistry,
    monsters: &MonsterEcosystem,
    eco: &EcoCycle,
    obs: &mut TickObserver,
    player_pos: [i32; 3],
    ticks: u64,
) -> SelfCheckReport {
    let mut pool = pool.clone();
    let mut monsters = MonsterEcosystem::clone(monsters);
    let mut eco = EcoCycle::clone(eco);
    let mut violations = Vec::new();

    for tick in 0..ticks {
        obs.begin_tick();
        monsters.tick(&mut pool);
        eco.tick(&mut pool);
        if let Err(errors) = obs.end_tick(
            tick,
            game_world,
            &pool,
            nations,
            &monsters,
            Some(player_pos),
        ) {
            violations.push(format!("tick {}: {}", tick, errors.join("; ")));
        }
    }

    SelfCheckReport { violations, ticks_run: ticks }
}

pub fn total_invariant_violations(obs: &TickObserver) -> u64 {
    obs.invariants.values().map(|inv| inv.total_violations).sum()
}

pub fn build_state_json(
    time: &Time,
    clock: &SimClock,
    player: &PlayerState,
    pool: &GlobalResourcePool,
    nations: &NationRegistry,
    monsters: &MonsterEcosystem,
    eco: &EcoCycle,
    obs: &TickObserver,
    game_world: &World,
    role: SnapshotRole,
) -> serde_json::Value {
    serde_json::json!({
        "frame_tick": clock.frame_tick,
        "tick": clock.tick,
        "wall_secs": time.elapsed_secs(),
        "role": role.as_str(),
        "player": {
            "block_pos": player.block_pos,
            "pos": [player.pos.x, player.pos.y, player.pos.z],
            "nation_id": player.nation_id.map(|n| n.0),
            "monsters_killed": player.monsters_killed,
            "blocks_gathered": player.blocks_gathered,
            "nations_founded": player.nations_founded,
        },
        "pool": {
            "wood": pool.get(ResourceKind::Wood),
            "food": pool.get(ResourceKind::Food),
            "apple": pool.get(ResourceKind::Apple),
            "soul": pool.get(ResourceKind::Soul),
        },
        "nations": {
            "flag_count": nations.flag_count,
            "total_nations": nations.nations.len(),
        },
        "monsters": {
            "current": monsters.current_individuals,
            "kingdoms": monsters.kingdoms.len(),
            "nests": monsters.kingdoms.values().map(|k| k.nests.len() as u32).sum::<u32>(),
        },
        "creatures": {
            "passive_current": eco.rabbit_count() + eco.wildlife_count(),
            "rabbit_count": eco.rabbit_count(),
            "wildlife": eco.wildlife_count(),
            "berry_bushes": eco.berry_count(),
            "plant_nodes": eco.plant_count(),
        },
        "eco_cycle": {
            "rabbits": eco.rabbit_count(),
            "wildlife": eco.wildlife_count(),
            "berry_bushes": eco.berry_count(),
            "plant_nodes": eco.plant_count(),
            "fruit": eco.total_fruit(),
            "co2": eco.co2,
            "fruit_eaten": eco.fruit_eaten,
            "fruit_grown": eco.fruit_grown,
        },
        "observer": {
            "snapshots": obs.snapshots.len(),
            "decisions": obs.decisions.len(),
            "anomalies": obs.anomalies.len(),
            "invariant_violations": total_invariant_violations(obs),
        },
        "world": {
            "size": game_world.size,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use bevy::prelude::Vec2;
    use serde_json::Value;

    fn assert_has_path<'a>(value: &'a Value, path: &[&str]) -> &'a Value {
        let mut current = value;
        for key in path {
            current =
                current.get(*key).unwrap_or_else(|| panic!("missing json path {}", path.join(".")));
        }
        current
    }

    #[test]
    fn build_state_json_includes_closed_loop_contract_fields() {
        let time = Time::default();
        let clock = SimClock {
            frame_tick: 420,
            tick: 42,
            last_sim_step_ran: true,
            last_tick_wall: 0.0,
            slow_tick_accum: 0.0,
            last_hud_wall: 0.0,
            last_screenshot_wall: 0.0,
            screenshot_count: 3,
        };
        let player = PlayerState {
            pos: Vec3::new(12.0, 18.0, 7.0),
            block_pos: [12, 18, 7],
            inventory: Default::default(),
            nation_id: None,
            monsters_killed: 5,
            blocks_gathered: 8,
            nations_founded: 1,
        };
        let mut pool = GlobalResourcePool::new();
        pool.try_add(ResourceKind::Wood, 11).unwrap();
        pool.try_add(ResourceKind::Food, 22).unwrap();
        pool.try_add(ResourceKind::Apple, 3).unwrap();
        pool.try_add(ResourceKind::Soul, 4).unwrap();

        let nations = NationRegistry::default();
        let monsters = MonsterEcosystem::default();
        let eco = EcoCycle::demo_at(Vec2::new(8.0, 8.0));
        let obs = TickObserver::default();
        let world = World::new(32);

        let json = build_state_json(
            &time,
            &clock,
            &player,
            &pool,
            &nations,
            &monsters,
            &eco,
            &obs,
            &world,
            SnapshotRole::ClientOffline,
        );

        assert_eq!(assert_has_path(&json, &["tick"]).as_u64(), Some(42));
        assert_eq!(assert_has_path(&json, &["frame_tick"]).as_u64(), Some(420));
        assert_eq!(
            assert_has_path(&json, &["role"]).as_str(),
            Some("client_offline")
        );
        assert_eq!(
            assert_has_path(&json, &["player", "block_pos"]).as_array().map(|v| v.len()),
            Some(3)
        );
        assert_eq!(assert_has_path(&json, &["pool", "wood"]).as_i64(), Some(11));
        assert!(assert_has_path(&json, &["nations", "total_nations"]).is_number());
        assert!(assert_has_path(&json, &["monsters", "current"]).is_number());
        assert_eq!(
            assert_has_path(&json, &["creatures", "passive_current"]).as_u64(),
            Some(9)
        );
        assert_eq!(assert_has_path(&json, &["creatures", "wildlife"]).as_u64(), Some(4));
        assert!(assert_has_path(&json, &["creatures", "berry_bushes"]).is_number());
        assert!(assert_has_path(&json, &["creatures", "plant_nodes"]).is_number());
        assert!(assert_has_path(&json, &["eco_cycle", "rabbits"]).is_number());
        assert!(assert_has_path(&json, &["eco_cycle", "wildlife"]).is_number());
        assert!(assert_has_path(&json, &["eco_cycle", "plant_nodes"]).is_number());
        assert!(assert_has_path(&json, &["observer", "anomalies"]).is_number());
        assert_eq!(
            assert_has_path(&json, &["world", "size"]).as_i64(),
            Some(32)
        );
    }

    #[test]
    fn total_invariant_violations_sums_all_registered_invariants() {
        let mut obs = TickObserver::default();
        obs.invariants.insert(
            crate::ai::InvariantKind::ResourceConservation,
            crate::ai::Invariant {
                name: "resource".to_string(),
                kind: crate::ai::InvariantKind::ResourceConservation,
                last_violation_tick: Some(3),
                total_violations: 2,
            },
        );
        obs.invariants.insert(
            crate::ai::InvariantKind::FlagCountCap,
            crate::ai::Invariant {
                name: "flags".to_string(),
                kind: crate::ai::InvariantKind::FlagCountCap,
                last_violation_tick: Some(5),
                total_violations: 4,
            },
        );

        assert_eq!(total_invariant_violations(&obs), 6);
    }
}
