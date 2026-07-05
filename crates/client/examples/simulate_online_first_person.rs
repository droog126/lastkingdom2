use bevy::prelude::*;
use lk2_core::constant;
use lk2_core::player::PlayerState;
use lk2_core::world::{
    World as GameWorld, install_huge_spawn_platform, player_body_clear, player_spawn_position_near,
    player_stand_position_at,
};

const PLAYER_COLLISION_RADIUS: f32 = 0.34;
const GROUND_STEP_THRESHOLD: f32 = 0.85;
const MANUAL_MOVE_SPEED: f32 = 4.5;
const DT: f32 = 1.0 / 60.0;
const FIRST_PERSON_START_PITCH: f32 = -1.05;

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
    let yaw = std::f32::consts::FRAC_PI_2;
    let pitch = FIRST_PERSON_START_PITCH;
    let (sy, cy) = yaw.sin_cos();
    let forward = Vec3::new(sy, 0.0, -cy).normalize_or_zero();
    let first_person_forward = first_person_forward(yaw, pitch);
    let ray_hit = ray_voxel_first_hit(
        &world,
        spawn_pos + Vec3::Y * 1.7,
        first_person_forward,
        120.0,
    )
    .expect("first-person center ray should hit the spawn platform");
    assert_eq!(
        ray_hit.3,
        lk2_core::world::BlockType::Leaves,
        "first-person center ray should hit the visible green spawn platform"
    );
    assert!(
        (1.5..=4.0).contains(&ray_hit.2),
        "first-person center ray should hit ground near the player's feet: {:?}",
        ray_hit
    );
    assert!(
        (Vec3::new(first_person_forward.x, 0.0, first_person_forward.z).normalize_or_zero()
            - forward)
            .length()
            < 0.001,
        "W movement should follow the first-person yaw, independent of pitch: camera={:?} movement={:?}",
        first_person_forward,
        forward
    );

    let mut moved_frames = 0;
    for _ in 0..120 {
        if try_player_move_continuous(
            &mut player,
            &world,
            forward,
            MANUAL_MOVE_SPEED * DT,
            GROUND_STEP_THRESHOLD,
        ) {
            moved_frames += 1;
        }
    }

    let delta = player.pos - before;
    println!(
        "online_first_person before={:?} after={:?} delta={:?} moved_frames={}",
        before, player.pos, delta, moved_frames
    );
    assert!(
        moved_frames > 0,
        "online first-person W input produced no movement"
    );
    assert!(
        delta.length() > 1.0,
        "online first-person local prediction did not move far enough: {delta:?}"
    );
}

fn first_person_forward(yaw: f32, pitch: f32) -> Vec3 {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    Vec3::new(sy * cp, sp, -cy * cp).normalize_or_zero()
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

fn ray_voxel_first_hit(
    world: &GameWorld,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
) -> Option<([i32; 3], Vec3, f32, lk2_core::world::BlockType)> {
    let step = 0.05;
    let steps = (max_dist / step) as i32;
    for i in 1..=steps {
        let t = i as f32 * step;
        let p = origin + dir * t;
        let block = [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32];
        if !world.in_bounds(block[0], block[1], block[2]) {
            continue;
        }
        let kind = world.get(block[0], block[1], block[2]);
        if kind.is_solid() {
            return Some((block, p, t, kind));
        }
    }
    None
}
