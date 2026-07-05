#[path = "../src/render/camera_math.rs"]
mod camera_math;

use bevy::prelude::*;
use camera_math::compute_third_person_camera;
use lk2_core::constant;
use lk2_core::world::{
    BlockType, World as GameWorld, install_huge_spawn_platform, player_spawn_position_near,
};

struct CameraAngles {
    yaw: f32,
    pitch: f32,
}

impl Default for CameraAngles {
    fn default() -> Self {
        Self { yaw: std::f32::consts::FRAC_PI_2, pitch: -1.05 }
    }
}

fn main() {
    let mut world = GameWorld::with_pipeline(
        constant::WORLD_SIZE,
        lk2_core::world::terrain::presets::by_name("default"),
    );
    install_huge_spawn_platform(&mut world);
    let (player_pos, player_block) = player_spawn_position_near(
        &world,
        constant::WORLD_SIZE / 2,
        constant::WORLD_SIZE / 2,
        14,
        2,
    )
    .expect("safe spawn should exist on grass platform");

    let angles = CameraAngles::default();
    let first_person_start_angles =
        CameraAngles { yaw: std::f32::consts::FRAC_PI_2, pitch: angles.pitch };
    let provisional = compute_third_person_camera(player_pos, angles.yaw, angles.pitch, 0.0);
    let ground_at_camera = effective_ground_height(
        &world,
        provisional.translation.x.floor() as i32,
        provisional.translation.z.floor() as i32,
    );
    let camera =
        compute_third_person_camera(player_pos, angles.yaw, angles.pitch, ground_at_camera);
    let grass_y = constant::SEA_LEVEL as f32 + 2.0;
    let hit = ray_plane_y(camera.translation, camera.forward, grass_y)
        .expect("camera center ray should intersect grass platform plane");
    let hit_block = [
        hit.x.floor() as i32,
        grass_y.floor() as i32,
        hit.z.floor() as i32,
    ];
    let below = world.get(hit_block[0], hit_block[1], hit_block[2]);

    println!(
        "player_pos={:?} player_block={:?}",
        player_pos, player_block
    );
    println!(
        "camera_pos={:?} target={:?} forward={:?} pitch={} yaw={}",
        camera.translation, camera.target, camera.forward, camera.pitch, angles.yaw
    );
    println!(
        "center_ray_hit={:?} hit_block={:?} block_at_hit={:?}",
        hit, hit_block, below
    );
    println!(
        "relative_hit_from_player=({:.2}, {:.2}, {:.2})",
        hit.x - player_pos.x,
        hit.y - player_pos.y,
        hit.z - player_pos.z
    );

    assert_eq!(
        below,
        BlockType::Leaves,
        "camera center ray must hit visible grass"
    );
    assert!(
        (hit.x - player_pos.x).abs() <= 90.0 && (hit.z - player_pos.z).abs() <= 90.0,
        "camera center ray should hit inside huge platform: hit={:?}, player={:?}",
        hit,
        player_pos
    );
    assert!(
        camera.forward.y < -0.05,
        "third-person center ray should look down enough to see ground: {:?}",
        camera.forward
    );

    let first_person_eye = player_pos + Vec3::Y * 1.7;
    let first_person_forward = first_person_forward(
        first_person_start_angles.yaw,
        first_person_start_angles.pitch,
    );
    let first_person_hit =
        ray_voxel_first_hit(&world, first_person_eye, first_person_forward, 120.0)
            .expect("first-person center ray should hit the grass platform");
    println!(
        "first_person eye={:?} forward={:?} hit={:?}",
        first_person_eye, first_person_forward, first_person_hit
    );
    assert_eq!(
        first_person_hit.3,
        BlockType::Leaves,
        "first-person center ray must hit visible grass platform"
    );
    assert!(
        (1.5..=4.0).contains(&first_person_hit.2),
        "first-person center ray should hit ground near the player's feet, not the horizon: {:?}",
        first_person_hit
    );
    assert!(
        first_person_hit.0[0] >= constant::WORLD_SIZE / 2 - 90
            && first_person_hit.0[0] <= constant::WORLD_SIZE / 2 + 90
            && first_person_hit.0[2] >= constant::WORLD_SIZE / 2 - 90
            && first_person_hit.0[2] <= constant::WORLD_SIZE / 2 + 90,
        "first-person ray hit outside huge platform: {:?}",
        first_person_hit
    );
}

fn effective_ground_height(world: &GameWorld, x: i32, z: i32) -> f32 {
    for y in (0..world.size).rev() {
        if world.get(x, y, z).is_solid() {
            return y as f32 + 1.0;
        }
    }
    world.size as f32
}

fn ray_plane_y(origin: Vec3, dir: Vec3, y: f32) -> Option<Vec3> {
    if dir.y.abs() < 0.0001 {
        return None;
    }
    let t = (y - origin.y) / dir.y;
    if t < 0.0 {
        return None;
    }
    Some(origin + dir * t)
}

fn first_person_forward(yaw: f32, pitch: f32) -> Vec3 {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    Vec3::new(sy * cp, sp, -cy * cp).normalize_or_zero()
}

fn ray_voxel_first_hit(
    world: &GameWorld,
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
) -> Option<([i32; 3], Vec3, f32, BlockType)> {
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
