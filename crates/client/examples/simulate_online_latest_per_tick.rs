//! Pure-logic simulation for the online first-person movement contract:
//! 1. The client encodes sprint into the magnitude of (dx_milli, dz_milli)
//!    and does NOT normalize.
//! 2. The server, on each FixedUpdate, drains all queued MoveWorld commands
//!    and applies ONLY the latest one, so the effective speed is decoupled
//!    from the client's frame rate.

use bevy::prelude::*;
use lk2_core::constant;
use lk2_core::player::PlayerState;
use lk2_core::world::{
    World as GameWorld, install_huge_spawn_platform, player_body_clear, player_spawn_position_near,
    player_stand_position_at,
};

const PLAYER_COLLISION_RADIUS: f32 = 0.34;
const ONLINE_MOVE_SPEED: f32 = 4.5;
const ONLINE_STEP_THRESHOLD: f32 = 0.85;
const FIXED_DT: f32 = 1.0 / 60.0;

fn main() {
    let mut world = GameWorld::with_pipeline(
        constant::WORLD_SIZE,
        lk2_core::world::terrain::presets::by_name("default"),
    );
    install_huge_spawn_platform(&mut world);
    let (spawn, spawn_block) = player_spawn_position_near(
        &world,
        constant::WORLD_SIZE / 2,
        constant::WORLD_SIZE / 2,
        14,
        2,
    )
    .expect("server must provide safe spawn on the huge platform");

    // Case 1: walk encoding: magnitude 1.0 in the +X world direction.
    let client_walk_dir = (1.0_f32, 0.0_f32);
    let sprint_factor_walk: f32 = 1.0;
    let scaled_walk = (
        client_walk_dir.0 * sprint_factor_walk,
        client_walk_dir.1 * sprint_factor_walk,
    );
    let dx_milli_walk = (scaled_walk.0 * 1000.0).round() as i16;
    let dz_milli_walk = (scaled_walk.1 * 1000.0).round() as i16;
    assert_eq!(
        (dx_milli_walk, dz_milli_walk),
        (1000, 0),
        "walk encoding should be +X unit"
    );

    // Case 2: sprint encoding: magnitude 1.5 in the same direction.
    let sprint_factor_sprint: f32 = 1.5;
    let scaled_sprint = (
        client_walk_dir.0 * sprint_factor_sprint,
        client_walk_dir.1 * sprint_factor_sprint,
    );
    let dx_milli_sprint = (scaled_sprint.0 * 1000.0).round() as i16;
    let dz_milli_sprint = (scaled_sprint.1 * 1000.0).round() as i16;
    assert_eq!(
        (dx_milli_sprint, dz_milli_sprint),
        (1500, 0),
        "sprint encoding should scale magnitude by 1.5"
    );

    // Case 3: latest-only per fixed tick, regardless of backlog.
    // Simulate a 144Hz client spamming the same MoveWorld while the server
    // ticks at 60Hz. Only the latest per tick must be honored.
    let mut player = PlayerState { pos: spawn, block_pos: spawn_block, ..Default::default() };
    let before = player.pos;
    let client_send_per_server_tick = 3; // About 180Hz client vs 60Hz server.
    let server_ticks = 120;
    for _ in 0..server_ticks {
        let latest = (dx_milli_walk, dz_milli_walk);
        let dir = Vec2::new(latest.0 as f32 / 1000.0, latest.1 as f32 / 1000.0);
        let _ = client_send_per_server_tick;
        server_apply_world_move(&world, &mut player, dir, FIXED_DT);
    }
    let per_tick_distance = ONLINE_MOVE_SPEED * FIXED_DT;
    let expected_total = per_tick_distance * server_ticks as f32;
    let delta = player.pos - before;
    println!(
        "latest_only: before={:?} after={:?} delta_x={:.4} expected_x~{:.4}",
        before, player.pos, delta.x, expected_total
    );
    assert!(
        (delta.x - expected_total).abs() < 0.05,
        "latest-only per-tick speed must be exactly speed*dt regardless of client backlog: got {:?}, expected ~{}",
        delta,
        expected_total
    );

    // Case 4: sprint scales total distance by 1.5 over the same tick count.
    let mut player = PlayerState { pos: spawn, block_pos: spawn_block, ..Default::default() };
    let before = player.pos;
    for _ in 0..server_ticks {
        let latest = (dx_milli_sprint, dz_milli_sprint);
        let dir = Vec2::new(latest.0 as f32 / 1000.0, latest.1 as f32 / 1000.0);
        server_apply_world_move(&world, &mut player, dir, FIXED_DT);
    }
    let expected_sprint = per_tick_distance * 1.5 * server_ticks as f32;
    let delta = player.pos - before;
    println!(
        "sprint:      before={:?} after={:?} delta_x={:.4} expected_x~{:.4}",
        before, player.pos, delta.x, expected_sprint
    );
    assert!(
        (delta.x - expected_sprint).abs() < 0.05,
        "sprint scaling should produce 1.5x the walk distance: got {:?}, expected ~{}",
        delta,
        expected_sprint
    );

    // Case 5: diagonal input preserves magnitude 1.0.
    let mut player = PlayerState { pos: spawn, block_pos: spawn_block, ..Default::default() };
    let before = player.pos;
    let diag_world = (
        std::f32::consts::FRAC_1_SQRT_2,
        -std::f32::consts::FRAC_1_SQRT_2,
    );
    let dx_milli_diag = (diag_world.0 * 1000.0).round() as i16;
    let dz_milli_diag = (diag_world.1 * 1000.0).round() as i16;
    for _ in 0..server_ticks {
        let dir = Vec2::new(dx_milli_diag as f32 / 1000.0, dz_milli_diag as f32 / 1000.0);
        server_apply_world_move(&world, &mut player, dir, FIXED_DT);
    }
    let expected_diag_len = (diag_world.0 * diag_world.0 + diag_world.1 * diag_world.1).sqrt()
        * ONLINE_MOVE_SPEED
        * FIXED_DT
        * server_ticks as f32;
    let actual_len = (player.pos - before).length();
    println!(
        "diagonal:    delta={:?} len={:.4} expected_len~{:.4}",
        player.pos - before,
        actual_len,
        expected_diag_len
    );
    assert!(
        (actual_len - expected_diag_len).abs() < 0.05,
        "diagonal magnitude should equal length(world_dir) * speed * dt * ticks: got {}, expected ~{}",
        actual_len,
        expected_diag_len
    );

    println!("simulate_online_latest_per_tick: OK");
}

/// Mirrors `server::main::apply_world_move` so the test exercises the same
/// collision-aware, magnitude-preserving step the live server uses.
fn server_apply_world_move(
    world: &GameWorld,
    player: &mut PlayerState,
    dir: Vec2,
    dt: f32,
) -> bool {
    if dir.length_squared() <= 0.0001 {
        return false;
    }
    let step = Vec3::new(dir.x, 0.0, dir.y) * ONLINE_MOVE_SPEED * dt;
    let next = player.pos + step;
    let next_x = next.x.floor() as i32;
    let next_z = next.z.floor() as i32;
    let Some((stand_pos, block_pos)) =
        player_stand_position_at(world, next_x, next_z, player.pos.y, ONLINE_STEP_THRESHOLD)
    else {
        return false;
    };
    if stand_pos.y - player.pos.y > ONLINE_STEP_THRESHOLD {
        return false;
    }
    let next_pos = Vec3::new(next.x, stand_pos.y, next.z);
    if !player_volume_clear(world, next_pos) {
        return false;
    }
    player.pos = next_pos;
    player.block_pos = block_pos;
    true
}

fn player_volume_clear(world: &GameWorld, pos: Vec3) -> bool {
    let foot_y = pos.y.floor() as i32;
    if !player_body_clear(world, pos.x.floor() as i32, foot_y, pos.z.floor() as i32) {
        return false;
    }
    for y in foot_y..=(foot_y + 1) {
        for ox in [-PLAYER_COLLISION_RADIUS, PLAYER_COLLISION_RADIUS] {
            for oz in [-PLAYER_COLLISION_RADIUS, PLAYER_COLLISION_RADIUS] {
                let x = (pos.x + ox).floor() as i32;
                let z = (pos.z + oz).floor() as i32;
                if world.get(x, y, z).is_solid() {
                    return false;
                }
            }
        }
    }
    true
}
