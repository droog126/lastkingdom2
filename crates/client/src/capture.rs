use std::path::PathBuf;

use bevy::prelude::*;

use lk2_core::ai::TickObserver;
use lk2_core::clock::SimClock;
use lk2_core::eco_cycle::EcoCycle;
use lk2_core::monster::MonsterEcosystem;
use lk2_core::nation::NationRegistry;
use lk2_core::player::PlayerState;
use lk2_core::resource::GlobalResourcePool;
use lk2_core::world::World as GameWorld;

use crate::ui::ClientRunMode;

pub const FIRST_SCREENSHOT_MIN_FRAME: u64 = 500;

#[derive(Resource, Default)]
pub struct TickRecorder {
    pub last_dump_tick: u64,
    pub current_iter: u32,
}

pub fn periodic_screenshot(
    time: Res<Time>,
    mut clock: ResMut<SimClock>,
    mut commands: Commands,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    eco: Res<EcoCycle>,
    obs: Res<TickObserver>,
    game_world: Res<GameWorld>,
    run_mode: Res<ClientRunMode>,
) {
    let now = {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        let start = START.get_or_init(std::time::Instant::now);
        start.elapsed().as_secs_f32()
    };
    let _ = time;

    if clock.frame_tick >= FIRST_SCREENSHOT_MIN_FRAME && now - clock.last_screenshot_wall >= 4.0 {
        eprintln!(
            "[shot] fire frame={} sim_tick={} wall={:.1} last={:.1}",
            clock.frame_tick, clock.tick, now, clock.last_screenshot_wall
        );
    }

    if clock.frame_tick < FIRST_SCREENSHOT_MIN_FRAME {
        return;
    }

    let interval = std::env::var("LK2_SCREENSHOT_INTERVAL")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(4.0);
    if now - clock.last_screenshot_wall < interval {
        return;
    }
    clock.last_screenshot_wall = now;
    clock.screenshot_count += 1;
    let iter_id = clock.screenshot_count;
    let iter_dir = format!("screenshots/iter_{:02}", iter_id);
    let _ = std::fs::create_dir_all(&iter_dir);
    let png_path: PathBuf = format!("{}/iter_{:02}.png", iter_dir, iter_id).into();
    let state_path = format!("{}/final_state.json", iter_dir);
    let diff_path = format!("{}/diff.json", iter_dir);

    info!("📸 截图 #{} → {}", iter_id, png_path.display());
    commands
        .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
        .observe(bevy::render::view::screenshot::save_to_disk(png_path));

    let state = build_state_json(
        &time,
        &clock,
        &player,
        &pool,
        &nations,
        &monsters,
        &eco,
        &obs,
        &game_world,
        *run_mode,
    );
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        if let Err(e) = std::fs::write(&state_path, s) {
            warn!("写 final_state.json 失败: {}", e);
        } else {
            info!("📝 final_state dumped → {}", state_path);
        }
    }
    if let Some(diff) = build_state_diff_for_iter(iter_id, &state) {
        if let Ok(s) = serde_json::to_string_pretty(&diff) {
            if let Err(e) = std::fs::write(&diff_path, s) {
                warn!("write diff.json failed: {}", e);
            } else {
                info!("diff.json dumped -> {}", diff_path);
            }
        }
    }
}

pub fn tick_recorder(
    time: Res<Time>,
    mut rec: ResMut<TickRecorder>,
    clock: Res<SimClock>,
    player: Res<PlayerState>,
    pool: Res<GlobalResourcePool>,
    nations: Res<NationRegistry>,
    monsters: Res<MonsterEcosystem>,
    eco: Res<EcoCycle>,
    obs: Res<TickObserver>,
    game_world: Res<GameWorld>,
    run_mode: Res<ClientRunMode>,
) {
    if clock.tick == 0 || clock.tick % 5 != 0 || clock.tick == rec.last_dump_tick {
        return;
    }
    rec.last_dump_tick = clock.tick;
    rec.current_iter = clock.tick as u32;
    let path = format!("screenshots/state_t{}.json", clock.tick);
    let state = build_state_json(
        &time,
        &clock,
        &player,
        &pool,
        &nations,
        &monsters,
        &eco,
        &obs,
        &game_world,
        *run_mode,
    );
    if let Ok(s) = serde_json::to_string_pretty(&state) {
        let _ = std::fs::write(&path, s);
        info!("📝 tick state dumped → {}", path);
    }
}

fn build_state_json(
    time: &Time,
    clock: &SimClock,
    player: &PlayerState,
    pool: &GlobalResourcePool,
    nations: &NationRegistry,
    monsters: &MonsterEcosystem,
    eco: &EcoCycle,
    obs: &TickObserver,
    game_world: &GameWorld,
    run_mode: ClientRunMode,
) -> serde_json::Value {
    lk2_core::diagnostics::build_state_json(
        time,
        clock,
        player,
        pool,
        nations,
        monsters,
        eco,
        obs,
        game_world,
        run_mode.snapshot_role(),
    )
}

fn build_state_diff_for_iter(iter_id: u32, state: &serde_json::Value) -> Option<serde_json::Value> {
    if iter_id <= 1 {
        return None;
    }
    let prev_path = format!("screenshots/iter_{:02}/final_state.json", iter_id - 1);
    let prev_raw = std::fs::read_to_string(prev_path).ok()?;
    let prev_state: serde_json::Value = serde_json::from_str(&prev_raw).ok()?;
    Some(build_state_diff(&prev_state, state))
}

fn build_state_diff(prev: &serde_json::Value, current: &serde_json::Value) -> serde_json::Value {
    let mut deltas = Vec::new();
    for path in [
        "tick",
        "player.monsters_killed",
        "player.blocks_gathered",
        "player.nations_founded",
        "pool.wood",
        "pool.food",
        "pool.apple",
        "pool.soul",
        "nations.flag_count",
        "nations.total_nations",
        "monsters.current",
        "monsters.kingdoms",
        "monsters.nests",
        "creatures.passive_current",
        "eco_cycle.rabbits",
        "eco_cycle.berry_bushes",
        "eco_cycle.fruit",
        "eco_cycle.fruit_eaten",
        "eco_cycle.fruit_grown",
        "observer.anomalies",
        "observer.invariant_violations",
    ] {
        if let Some(delta) = diff_numeric_path(prev, current, path) {
            deltas.push(delta);
        }
    }

    deltas.sort_by(|a, b| {
        let a_abs = a["delta_abs"].as_f64().unwrap_or(0.0);
        let b_abs = b["delta_abs"].as_f64().unwrap_or(0.0);
        b_abs.partial_cmp(&a_abs).unwrap_or(std::cmp::Ordering::Equal).then_with(|| {
            a["path"].as_str().unwrap_or_default().cmp(b["path"].as_str().unwrap_or_default())
        })
    });

    serde_json::json!({
        "prev_tick": value_at_path(prev, "tick").and_then(|v| v.as_u64()),
        "tick": value_at_path(current, "tick").and_then(|v| v.as_u64()),
        "resource_deltas": deltas,
    })
}

fn diff_numeric_path(
    prev: &serde_json::Value,
    current: &serde_json::Value,
    path: &str,
) -> Option<serde_json::Value> {
    let prev_num = value_at_path(prev, path)?.as_f64()?;
    let current_num = value_at_path(current, path)?.as_f64()?;
    let delta = current_num - prev_num;
    if delta.abs() < f64::EPSILON {
        return None;
    }
    Some(serde_json::json!({
        "path": path,
        "previous": prev_num,
        "current": current_num,
        "delta": delta,
        "delta_abs": delta.abs(),
    }))
}

fn value_at_path<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_diff_reports_sorted_numeric_deltas() {
        let prev = serde_json::json!({
            "tick": 100,
            "player": {
                "monsters_killed": 1,
                "blocks_gathered": 4,
                "nations_founded": 1
            },
            "pool": {
                "wood": 50,
                "food": 90,
                "apple": 70,
                "soul": 60
            },
            "nations": {
                "flag_count": 7,
                "total_nations": 7
            },
            "monsters": {
                "current": 60,
                "kingdoms": 1,
                "nests": 3
            },
            "creatures": {
                "passive_current": 5
            },
            "eco_cycle": {
                "rabbits": 5,
                "berry_bushes": 10,
                "fruit": 1,
                "fruit_eaten": 20,
                "fruit_grown": 11
            },
            "observer": {
                "anomalies": 0,
                "invariant_violations": 0
            }
        });
        let current = serde_json::json!({
            "tick": 125,
            "player": {
                "monsters_killed": 2,
                "blocks_gathered": 6,
                "nations_founded": 1
            },
            "pool": {
                "wood": 45,
                "food": 93,
                "apple": 70,
                "soul": 60
            },
            "nations": {
                "flag_count": 8,
                "total_nations": 8
            },
            "monsters": {
                "current": 58,
                "kingdoms": 1,
                "nests": 3
            },
            "creatures": {
                "passive_current": 4
            },
            "eco_cycle": {
                "rabbits": 5,
                "berry_bushes": 10,
                "fruit": 0,
                "fruit_eaten": 23,
                "fruit_grown": 13
            },
            "observer": {
                "anomalies": 0,
                "invariant_violations": 0
            }
        });

        let diff = build_state_diff(&prev, &current);
        assert_eq!(diff["prev_tick"], 100);
        assert_eq!(diff["tick"], 125);

        let deltas = diff["resource_deltas"].as_array().unwrap();
        let paths = deltas.iter().map(|delta| delta["path"].as_str().unwrap()).collect::<Vec<_>>();
        assert_eq!(
            paths,
            vec![
                "tick",
                "pool.wood",
                "eco_cycle.fruit_eaten",
                "pool.food",
                "eco_cycle.fruit_grown",
                "monsters.current",
                "player.blocks_gathered",
                "creatures.passive_current",
                "eco_cycle.fruit",
                "nations.flag_count",
                "nations.total_nations",
                "player.monsters_killed"
            ]
        );

        let wood = deltas.iter().find(|delta| delta["path"] == "pool.wood").unwrap();
        assert_eq!(wood["previous"], 50.0);
        assert_eq!(wood["current"], 45.0);
        assert_eq!(wood["delta"], -5.0);
    }
}
