use bevy::prelude::*;
use lk2_core::constant;
use lk2_core::player::PlayerState;
use lk2_core::world::{
    install_huge_spawn_platform, player_body_clear, player_spawn_position_near,
    player_stand_position_at, World as GameWorld,
};

const PLAYER_COLLISION_RADIUS: f32 = 0.34;
const GROUND_STEP_THRESHOLD: f32 = 0.85;
const MANUAL_MOVE_SPEED: f32 = 4.5;
const DT: f32 = 1.0 / 60.0;

fn main() {
    let mut world = GameWorld::with_pipeline(
        constant::WORLD_SIZE,
        lk2_core::world::terrain::presets::by_name("default"),
    );
    install_huge_spawn_platform(&mut world);

    let (spawn_pos, spawn_block) = player_spawn_position_near(
        &world,
        constant::WORLD_SIZE / 2,
        constant::WORLD_SIZE / 2,
        14,
        2,
    )
    .expect("spawn platform should provide a safe standable position");

    let mut player = PlayerState { pos: spawn_pos, block_pos: spawn_block, ..Default::default() };
    let before = player.pos;
    let before_block = player.block_pos;
    let mut moved_frames = 0;
    for _ in 0..120 {
        if try_player_move_continuous(
            &mut player,
            &world,
            Vec3::new(0.0, 0.0, -1.0),
            MANUAL_MOVE_SPEED * DT,
            GROUND_STEP_THRESHOLD,
        ) {
            moved_frames += 1;
        }
    }

    let delta = player.pos - before;
    println!(
        "before_pos={:?} before_block={:?} after_pos={:?} after_block={:?} delta={:?} moved_frames={}",
        before, before_block, player.pos, player.block_pos, delta, moved_frames
    );

    assert!(
        moved_frames > 0,
        "simulated W input never produced a successful movement frame"
    );
    assert!(
        delta.length() > 1.0,
        "player did not advance far enough after simulated W input: delta={:?}",
        delta
    );
}

fn try_player_move_continuous(
    player: &mut PlayerState,
    game_world: &GameWorld,
    dir: Vec3,
    distance: f32,
    threshold: f32,
) -> bool {
    let horizontal = Vec3::new(dir.x, 0.0, dir.z);
    if horizontal.length_squared() < 0.0001 || distance <= 0.0 {
        return false;
    }

    let next = player.pos + horizontal.normalize() * distance;
    let next_x = next.x.floor() as i32;
    let next_z = next.z.floor() as i32;
    let Some((stand_pos, block_pos)) =
        player_stand_position_at(game_world, next_x, next_z, player.pos.y, threshold)
    else {
        return false;
    };

    if stand_pos.y - player.pos.y > threshold {
        return false;
    }
    if !player_volume_clear_at(game_world, Vec3::new(next.x, stand_pos.y, next.z)) {
        return false;
    }

    player.pos = Vec3::new(next.x, stand_pos.y, next.z);
    player.block_pos = block_pos;
    true
}

fn player_volume_clear_at(game_world: &GameWorld, pos: Vec3) -> bool {
    let foot_y = pos.y.floor() as i32;
    if !player_body_clear(
        game_world,
        pos.x.floor() as i32,
        foot_y,
        pos.z.floor() as i32,
    ) {
        return false;
    }
    for y in foot_y..=(foot_y + 1) {
        for ox in [-PLAYER_COLLISION_RADIUS, PLAYER_COLLISION_RADIUS] {
            for oz in [-PLAYER_COLLISION_RADIUS, PLAYER_COLLISION_RADIUS] {
                let x = (pos.x + ox).floor() as i32;
                let z = (pos.z + oz).floor() as i32;
                if game_world.get(x, y, z).is_solid() {
                    return false;
                }
            }
        }
    }
    true
}
