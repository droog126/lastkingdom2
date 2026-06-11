use bevy::prelude::*;

use crate::ai::TickObserver;
use crate::clock::SimClock;
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
    obs: &mut TickObserver,
    player_pos: [i32; 3],
    ticks: u64,
) -> SelfCheckReport {
    let mut pool = pool.clone();
    let mut monsters = MonsterEcosystem::clone(monsters);
    let mut violations = Vec::new();

    for tick in 0..ticks {
        obs.begin_tick();
        monsters.tick(&mut pool);
        let _ = pool.try_add(ResourceKind::Food, 2);
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
    obs: &TickObserver,
    game_world: &World,
    role: SnapshotRole,
) -> serde_json::Value {
    serde_json::json!({
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
