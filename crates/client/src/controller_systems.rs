use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use lk2_core::controller::components::{GroundHit, PlayerCollider, PvPController};
use lk2_core::world::World as GameWorld;

pub fn ground_detection(
    time: Res<Time>,
    spatial_query: SpatialQuery,
    mut controllers: Query<(&Transform, &PlayerCollider, &mut PvPController)>,

    _voxel_colliders: Query<Entity, (With<Collider>, Without<PvPController>)>,
) {
    let now = time.elapsed_secs();

    for (transform, collider, mut controller) in controllers.iter_mut() {
        let foot_pos = transform.translation - Vec3::Y * collider.half_height;

        let ray_dir = Dir3::NEG_Y;

        let ray_length = controller.step_height + 0.05;

        let hit = spatial_query.cast_ray(
            foot_pos,
            ray_dir,
            ray_length,
            true,
            &SpatialQueryFilter::default(),
        );

        if let Some(hit_result) = hit {
            controller.is_grounded = true;
            controller.last_grounded_time = now;
            controller.ground_normal = Some(hit_result.normal);
        } else {
            if now - controller.last_grounded_time > 0.1 {
                controller.is_grounded = false;
                controller.ground_normal = None;
            }
        }
    }
}

pub fn character_movement(
    mut controllers: Query<(&mut LinearVelocity, &mut PvPController, &Transform)>,
    camera: Query<&Transform, With<Camera3d>>,
    input: Res<ButtonInput<KeyCode>>,
    _time: Res<Time>,
) {
    let cam_tf = camera.single().ok();
    let (forward, right) = if let Some(tf) = cam_tf {
        let f = tf.forward();
        let f_h = Vec3::new(f.x, 0.0, f.z);
        let f_n = if f_h.length() > 0.01 {
            f_h.normalize()
        } else {
            Vec3::X
        };
        let r = Vec3::Y.cross(f_n);
        (f_n, r)
    } else {
        (Vec3::X, Vec3::Z)
    };

    let mut move_dir = Vec3::ZERO;
    if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp) {
        move_dir += forward;
    }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown) {
        move_dir -= forward;
    }
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) {
        move_dir -= right;
    }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) {
        move_dir += right;
    }

    if move_dir.length() > 0.01 {
        move_dir = move_dir.normalize();
    }

    let sprinting = input.pressed(KeyCode::ShiftLeft) || input.pressed(KeyCode::ShiftRight);

    let jump_requested = input.just_pressed(KeyCode::Space);

    for (mut velocity, mut controller, _transform) in controllers.iter_mut() {
        controller.move_input = Vec2::new(move_dir.x, move_dir.z);
        controller.jump_requested = jump_requested;
        controller.is_sprinting = sprinting;

        let speed = if sprinting {
            controller.speed * 1.5
        } else {
            controller.speed
        };

        let input_mult = controller.input_multiplier();

        let target_vel = move_dir * speed * input_mult;

        if controller.is_grounded {
            velocity.x = target_vel.x;
            velocity.z = target_vel.z;

            if jump_requested {
                velocity.y = controller.jump_impulse;
                controller.is_grounded = false;
            }
        } else {
            let lerp_factor = controller.air_control;
            velocity.x = velocity.x.lerp(target_vel.x, lerp_factor);
            velocity.z = velocity.z.lerp(target_vel.z, lerp_factor);
        }

        velocity.x += controller.knockback_velocity.x;
        velocity.z += controller.knockback_velocity.z;
    }
}

pub fn auto_step_up(
    spatial_query: SpatialQuery,
    mut controllers: Query<(
        &mut LinearVelocity,
        &mut PvPController,
        &Transform,
        &PlayerCollider,
    )>,
    _voxel_colliders: Query<Entity, (With<Collider>, Without<PvPController>)>,
) {
    for (mut velocity, controller, transform, collider) in controllers.iter_mut() {
        if !controller.is_grounded || controller.move_input.length() < 0.01 {
            continue;
        }

        let move_dir = Vec3::new(controller.move_input.x, 0.0, controller.move_input.y);
        if move_dir.length() < 0.01 {
            continue;
        }
        let move_dir = move_dir.normalize();

        let foot_pos = transform.translation - Vec3::Y * collider.half_height;
        let check_dist = collider.radius + 0.1;

        let forward_hit = spatial_query.cast_ray(
            foot_pos,
            Dir3::new(move_dir).unwrap_or(Dir3::X),
            check_dist,
            true,
            &SpatialQueryFilter::default(),
        );

        if let Some(hit) = forward_hit {
            let hit_point = foot_pos + move_dir * hit.distance;

            let step_check_pos = hit_point + Vec3::Y * controller.step_height;
            let above_hit = spatial_query.cast_ray(
                step_check_pos,
                Dir3::NEG_Y,
                controller.step_height + 0.1,
                true,
                &SpatialQueryFilter::default(),
            );

            if let Some(above) = above_hit {
                let step_height_actual = above.distance;

                if step_height_actual <= controller.step_height {
                    velocity.y = controller.step_speed;
                }
            }
        }
    }
}

pub fn knockback_decay(time: Res<Time>, mut controllers: Query<&mut PvPController>) {
    let dt = time.delta_secs();

    for mut controller in controllers.iter_mut() {
        controller.knockback_velocity *= 0.5_f32.powf(dt);

        controller.knockback_stun = (controller.knockback_stun - dt).max(0.0);

        if controller.knockback_velocity.length() < 0.01 {
            controller.knockback_velocity = Vec3::ZERO;
        }
    }
}

pub fn collect_input(keys: Res<ButtonInput<KeyCode>>, mut controllers: Query<&mut PvPController>) {
    for mut controller in controllers.iter_mut() {
        let mut input = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyW) {
            input.y += 1.0;
        }
        if keys.pressed(KeyCode::KeyS) {
            input.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyA) {
            input.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) {
            input.x += 1.0;
        }

        if input.length() > 1.0 {
            input = input.normalize();
        }

        controller.move_input = input;
        controller.jump_requested = keys.just_pressed(KeyCode::Space);
        controller.is_sprinting =
            keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    }
}

pub fn spawn_voxel_colliders(
    _commands: Commands,
    _game_world: Res<GameWorld>,
    _player: Query<&Transform, With<PvPController>>,
    _existing_colliders: Query<&Transform, (With<Collider>, Without<PvPController>)>,
) {
}

pub struct ControllerPlugin;

impl Plugin for ControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                ground_detection,
                character_movement,
                auto_step_up,
                knockback_decay,
            )
                .chain(),
        );
    }
}
