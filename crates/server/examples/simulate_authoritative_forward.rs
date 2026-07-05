use bevy::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use lk2_core::constant;
use lk2_core::player::PlayerState;
use lk2_core::protocol::PlayerAction;
use lk2_core::protocol::components::PlayerPos;
use lk2_core::protocol::messages::GameplayCommandKind;
use lk2_core::world::{
    World as GameWorld, install_huge_spawn_platform, player_spawn_position_near,
};

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
    .expect("server authoritative world must provide safe spawn on huge platform");

    let mut player = PlayerState { pos: spawn, block_pos: spawn_block, ..Default::default() };
    let mut transform = Transform::from_translation(spawn);
    let mut player_pos = PlayerPos(spawn);
    let mut actions = ActionState::<PlayerAction>::default();
    actions.press(&PlayerAction::MoveForward);
    let command = GameplayCommandKind::MoveWorld { dx_milli: 0, dz_milli: -1000 };

    let before = transform.translation;
    let before_block = player.block_pos;
    let mut moved_frames = 0;
    for _ in 0..120 {
        let dir = match command {
            GameplayCommandKind::MoveWorld { dx_milli, dz_milli } => {
                Vec2::new(dx_milli as f32 / 1000.0, dz_milli as f32 / 1000.0)
            }
            _ => action_dir(&actions),
        };
        if apply_world_move(
            &mut transform,
            &mut player_pos,
            &mut player,
            dir,
            1.0 / 30.0,
        ) {
            moved_frames += 1;
        }
    }
    let delta = transform.translation - before;
    println!(
        "server_before_pos={:?} before_block={:?} server_after_pos={:?} after_block={:?} delta={:?} moved_frames={}",
        before, before_block, transform.translation, player.block_pos, delta, moved_frames
    );

    assert!(
        moved_frames > 0,
        "server authoritative MoveForward never moved"
    );
    assert!(
        delta.length() > 1.0,
        "server authoritative MoveForward did not advance far enough: delta={:?}",
        delta
    );
}

fn action_dir(actions: &ActionState<PlayerAction>) -> Vec2 {
    let mut local_dir = Vec2::ZERO;
    if actions.pressed(&PlayerAction::MoveForward) {
        local_dir.y -= 1.0;
    }
    if actions.pressed(&PlayerAction::MoveBackward) {
        local_dir.y += 1.0;
    }
    if actions.pressed(&PlayerAction::MoveLeft) {
        local_dir.x -= 1.0;
    }
    if actions.pressed(&PlayerAction::MoveRight) {
        local_dir.x += 1.0;
    }
    if local_dir.length() > 1.0 {
        local_dir = local_dir.normalize();
    }
    local_dir
}

fn apply_world_move(
    transform: &mut Transform,
    player_pos: &mut PlayerPos,
    player: &mut PlayerState,
    dir: Vec2,
    dt: f32,
) -> bool {
    if dir.length() <= 0.01 {
        return false;
    }

    let speed = 4.0;
    let delta = Vec3::new(dir.x, 0.0, dir.y) * speed * dt;
    transform.translation += delta;
    player_pos.0 = transform.translation;
    player.pos = transform.translation;
    player.block_pos = [
        transform.translation.x.floor() as i32,
        transform.translation.y.floor() as i32,
        transform.translation.z.floor() as i32,
    ];
    true
}
