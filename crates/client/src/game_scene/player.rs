//! Player controls, follow camera, and per-frame scene clock.

use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use super::keybindings::{GameAction, KeyBindings};
use super::procedural_rig::{apply_local_pose, apply_local_segment, TwoBoneLimb};
use super::state::{
    BossActor, CameraMode, LivingCameraRig, LivingSceneCamera, LivingSceneState, PlayerActor,
    PlayerIkPart, PlayerIkPartKind, PlayerJump, PlayerMotion, SlashFx,
};
use super::util::{PLAYER_GRAVITY, PLAYER_JUMP_SPEED, PLAYER_SPEED};
use lk2_core::pvp::{resolve_melee_attack, Health, PvpCombatant, SimpleWeapon};

pub fn advance_scene(
    time: Res<Time>,
    mut state: ResMut<LivingSceneState>,
    mut combatants: Query<&mut PvpCombatant>,
) {
    let dt = time.delta_secs();
    state.elapsed += dt;
    state.frame += 1;
    if dt > 0.05 {
        state.frame_dt_over_50ms = state.frame_dt_over_50ms.saturating_add(1);
    }
    state.frame_dt_max_ms = state.frame_dt_max_ms.max(dt * 1000.0);
    state.attack_flash = (state.attack_flash - dt).max(0.0);
    for mut combatant in &mut combatants {
        combatant.tick(dt);
    }
}

pub fn camera_look_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut camera_rig: ResMut<LivingCameraRig>,
    bindings: Res<KeyBindings>,
) {
    if bindings.menu_open {
        return;
    }
    if bindings.just_pressed(GameAction::ToggleCamera, &keys, &mouse) {
        camera_rig.mode = match camera_rig.mode {
            CameraMode::FirstPerson => CameraMode::ThirdPerson,
            CameraMode::ThirdPerson => CameraMode::FirstPerson,
        };
    }

    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }
    camera_rig.yaw -= delta.x * 0.0032;
    camera_rig.pitch = (camera_rig.pitch - delta.y * 0.0027).clamp(-1.25, 0.92);

    let turn = if bindings.pressed(GameAction::CameraTurnLeft, &keys, &mouse) {
        1.0
    } else if bindings.pressed(GameAction::CameraTurnRight, &keys, &mouse) {
        -1.0
    } else {
        0.0
    };
    camera_rig.yaw += turn * time.delta_secs() * 1.8;
}

pub fn player_controls(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    camera_rig: Res<LivingCameraRig>,
    bindings: Res<KeyBindings>,
    mut commands: Commands,
    mut state: ResMut<LivingSceneState>,
    scene_materials: Res<super::state::SceneMaterials>,
    mut players: Query<
        (
            &mut Transform,
            &mut PvpCombatant,
            &SimpleWeapon,
            &mut PlayerJump,
            &mut PlayerMotion,
        ),
        With<PlayerActor>,
    >,
    mut bosses: Query<(&Transform, &mut Health), (With<BossActor>, Without<PlayerActor>)>,
) {
    if bindings.menu_open {
        return;
    }
    let Ok((mut player, mut combatant, weapon, mut jump, mut motion)) = players.single_mut() else {
        return;
    };
    let dt = time.delta_secs().max(0.0001);
    let previous_position = player.translation;
    jump.vertical_velocity += PLAYER_GRAVITY * dt;
    player.translation.y += jump.vertical_velocity * dt;
    let mut grounded = false;
    if player.translation.y <= 0.0 {
        player.translation.y = 0.0;
        jump.vertical_velocity = 0.0;
        grounded = true;
    }

    if bindings.just_pressed(GameAction::Jump, &keys, &mouse)
        && player.translation.y <= f32::EPSILON
    {
        jump.vertical_velocity = PLAYER_JUMP_SPEED;
        grounded = false;
    }

    let mut input_forward = 0.0;
    let mut input_right = 0.0;
    if bindings.pressed(GameAction::MoveForward, &keys, &mouse) {
        input_forward += 1.0;
    }
    if bindings.pressed(GameAction::MoveBackward, &keys, &mouse) {
        input_forward -= 1.0;
    }
    if bindings.pressed(GameAction::MoveLeft, &keys, &mouse) {
        input_right -= 1.0;
    }
    if bindings.pressed(GameAction::MoveRight, &keys, &mouse) {
        input_right += 1.0;
    }
    let camera_forward = yaw_forward(camera_rig.yaw);
    let camera_right = Vec3::new(-camera_forward.z, 0.0, camera_forward.x);
    let mut direction = camera_forward * input_forward + camera_right * input_right;
    if direction.length_squared() > 0.0 {
        direction = direction.normalize();
        player.translation += direction * PLAYER_SPEED * dt;
        player.translation.x = player.translation.x.clamp(-22.0, 22.0);
        player.translation.z = player.translation.z.clamp(-22.0, 22.0);
        player.rotation = match camera_rig.mode {
            CameraMode::FirstPerson => Quat::from_rotation_y(camera_rig.yaw),
            CameraMode::ThirdPerson => Quat::from_rotation_y(direction.x.atan2(direction.z)),
        };
    } else if camera_rig.mode == CameraMode::FirstPerson {
        player.rotation = Quat::from_rotation_y(camera_rig.yaw);
    }
    let planar_delta = Vec3::new(
        player.translation.x - previous_position.x,
        0.0,
        player.translation.z - previous_position.z,
    );
    motion.planar_velocity = planar_delta / dt;
    motion.planar_speed = motion.planar_velocity.length();
    motion.smoothed_speed = motion
        .smoothed_speed
        .lerp(motion.planar_speed, 1.0 - (-dt * 12.0).exp());
    motion.moving = motion.planar_speed > 0.05;
    motion.grounded = grounded;
    motion.airborne_time = if grounded {
        0.0
    } else {
        motion.airborne_time + dt
    };
    let gait = (motion.smoothed_speed / PLAYER_SPEED).clamp(0.0, 1.0);
    if gait > 0.02 && grounded {
        motion.stride_phase =
            (motion.stride_phase + dt * (5.5 + gait * 5.0)).rem_euclid(std::f32::consts::TAU);
    } else {
        motion.stride_phase = motion.stride_phase.lerp(0.0, 1.0 - (-dt * 5.0).exp());
    }

    let attacking = bindings.just_pressed(GameAction::Attack, &keys, &mouse);
    if !attacking || !combatant.begin_attack(*weapon) {
        return;
    }
    state.attack_flash = 0.32;
    if let Ok((boss, mut health)) = bosses.single_mut() {
        if let Some(hit) = resolve_melee_attack(
            player.translation,
            *player.back(),
            boss.translation,
            *weapon,
        ) {
            health.damage(hit.damage, state.frame as u32, 0);
        }
    }
    commands.spawn((
        Mesh3d(scene_materials.slash_mesh.clone()),
        MeshMaterial3d(scene_materials.slash_material.clone()),
        Transform::from_translation(player.translation + Vec3::Y * 0.9)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
            .with_scale(Vec3::new(1.0, 0.35, 1.0)),
        SlashFx,
    ));
}

pub fn update_camera(
    time: Res<Time>,
    camera_rig: Res<LivingCameraRig>,
    players: Query<&Transform, (With<PlayerActor>, Without<LivingSceneCamera>)>,
    mut cameras: Query<&mut Transform, (With<LivingSceneCamera>, Without<PlayerActor>)>,
) {
    let (Ok(player), Ok(mut camera)) = (players.single(), cameras.single_mut()) else {
        return;
    };
    match camera_rig.mode {
        CameraMode::FirstPerson => {
            let eye = first_person_eye(player.translation);
            let forward = camera_direction(camera_rig.yaw, camera_rig.pitch);
            camera.translation = eye + forward * 0.08;
            let target = camera.translation + forward;
            camera.look_at(target, Vec3::Y);
        }
        CameraMode::ThirdPerson => {
            let forward = yaw_forward(camera_rig.yaw);
            let right = Vec3::new(-forward.z, 0.0, forward.x);
            let desired = player.translation - forward * 6.2 + right * 0.9 + Vec3::Y * 3.2;
            camera.translation = camera
                .translation
                .lerp(desired, 1.0 - (-time.delta_secs() * 5.5).exp());
            camera.look_at(player.translation + Vec3::Y * 1.05, Vec3::Y);
        }
    }
}

pub fn update_player_ik(
    state: Res<LivingSceneState>,
    camera_rig: Res<LivingCameraRig>,
    players: Query<(&Transform, &PlayerMotion, &PlayerJump), With<PlayerActor>>,
    mut parts: Query<
        (&PlayerIkPart, &mut Transform, Option<&mut Visibility>),
        Without<PlayerActor>,
    >,
) {
    let Ok((player, motion, jump)) = players.single() else {
        return;
    };
    let first_person = camera_rig.mode == CameraMode::FirstPerson;
    let pose = procedural_player_pose(
        state.elapsed,
        state.attack_flash,
        motion,
        jump,
        first_person,
        camera_rig.pitch,
    );

    let left_arm = TwoBoneLimb {
        root: Vec3::new(-0.31, 1.02, -0.02),
        target: pose.left_hand,
        pole: pose.left_elbow_pole,
        upper_len: 0.35,
        lower_len: 0.34,
    }
    .solve();
    let right_arm = TwoBoneLimb {
        root: Vec3::new(0.31, 1.02, -0.02),
        target: pose.right_hand,
        pole: pose.right_elbow_pole,
        upper_len: 0.35,
        lower_len: 0.34,
    }
    .solve();
    let left_leg = TwoBoneLimb {
        root: Vec3::new(-0.15, 0.50, 0.03),
        target: pose.left_foot,
        pole: pose.left_knee_pole,
        upper_len: 0.38,
        lower_len: 0.44,
    }
    .solve();
    let right_leg = TwoBoneLimb {
        root: Vec3::new(0.15, 0.50, 0.03),
        target: pose.right_foot,
        pole: pose.right_knee_pole,
        upper_len: 0.38,
        lower_len: 0.44,
    }
    .solve();

    for (part, mut transform, visibility) in &mut parts {
        if let Some(mut visibility) = visibility {
            *visibility = if first_person && hides_in_first_person(part.kind) {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
        }

        match part.kind {
            PlayerIkPartKind::Torso => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.0, 0.76 + pose.body_bob, 0.0),
                Quat::from_rotation_x(pose.torso_pitch) * Quat::from_rotation_z(pose.torso_roll),
                Vec3::ONE,
            ),
            PlayerIkPartKind::Pants => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.0, 0.43 + pose.body_bob * 0.65, 0.03),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::Neck => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.0, 1.13, -0.01),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::Head => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.0, 1.46, -0.03),
                Quat::from_rotation_x(pose.head_pitch),
                Vec3::new(1.0, 0.92, 0.94),
            ),
            PlayerIkPartKind::EyeL => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(-0.11, 1.46, -0.32),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::EyeR => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.11, 1.46, -0.32),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::ArmUpperL => {
                apply_local_segment(&mut transform, player, left_arm.root, left_arm.joint)
            }
            PlayerIkPartKind::ArmLowerL => {
                apply_local_segment(&mut transform, player, left_arm.joint, left_arm.target)
            }
            PlayerIkPartKind::ArmUpperR => {
                apply_local_segment(&mut transform, player, right_arm.root, right_arm.joint)
            }
            PlayerIkPartKind::ArmLowerR => {
                apply_local_segment(&mut transform, player, right_arm.joint, right_arm.target)
            }
            PlayerIkPartKind::HandL => apply_local_pose(
                &mut transform,
                player,
                pose.left_hand,
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::HandR => apply_local_pose(
                &mut transform,
                player,
                pose.right_hand,
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::LegUpperL => {
                apply_local_segment(&mut transform, player, left_leg.root, left_leg.joint)
            }
            PlayerIkPartKind::LegLowerL => {
                apply_local_segment(&mut transform, player, left_leg.joint, left_leg.target)
            }
            PlayerIkPartKind::LegUpperR => {
                apply_local_segment(&mut transform, player, right_leg.root, right_leg.joint)
            }
            PlayerIkPartKind::LegLowerR => {
                apply_local_segment(&mut transform, player, right_leg.joint, right_leg.target)
            }
            PlayerIkPartKind::BootL => apply_local_pose(
                &mut transform,
                player,
                pose.left_foot + Vec3::new(0.0, -0.02, -0.08),
                Quat::from_rotation_x(pose.left_boot_pitch),
                Vec3::ONE,
            ),
            PlayerIkPartKind::BootR => apply_local_pose(
                &mut transform,
                player,
                pose.right_foot + Vec3::new(0.0, -0.02, -0.08),
                Quat::from_rotation_x(pose.right_boot_pitch),
                Vec3::ONE,
            ),
            PlayerIkPartKind::Basket => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.0, 0.76, 0.28),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::Stick => apply_local_segment(
                &mut transform,
                player,
                pose.right_hand + Vec3::new(0.02, -0.05, -0.06),
                pose.right_hand + Vec3::new(0.34, -0.18, -0.52 - pose.attack * 0.20),
            ),
        }
    }
}

fn yaw_forward(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

fn camera_direction(yaw: f32, pitch: f32) -> Vec3 {
    let flat = pitch.cos();
    Vec3::new(yaw.sin() * flat, pitch.sin(), yaw.cos() * flat).normalize()
}

pub fn first_person_eye(player_position: Vec3) -> Vec3 {
    player_position + Vec3::new(0.0, 1.54, 0.0)
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct ProceduralPlayerPose {
    pub gait: f32,
    pub stride_phase: f32,
    pub attack: f32,
    pub airborne: f32,
    pub body_bob: f32,
    pub torso_pitch: f32,
    pub torso_roll: f32,
    pub head_pitch: f32,
    pub left_hand: Vec3,
    pub right_hand: Vec3,
    pub left_foot: Vec3,
    pub right_foot: Vec3,
    pub left_elbow_pole: Vec3,
    pub right_elbow_pole: Vec3,
    pub left_knee_pole: Vec3,
    pub right_knee_pole: Vec3,
    pub left_boot_pitch: f32,
    pub right_boot_pitch: f32,
}

pub fn procedural_player_pose(
    elapsed: f32,
    attack_flash: f32,
    motion: &PlayerMotion,
    jump: &PlayerJump,
    first_person: bool,
    camera_pitch: f32,
) -> ProceduralPlayerPose {
    let gait = (motion.smoothed_speed / PLAYER_SPEED).clamp(0.0, 1.0);
    let stride_phase = motion.stride_phase;
    let stride_sin = stride_phase.sin();
    let stride_cos = stride_phase.cos();
    let attack = smooth01(attack_flash / 0.32);
    let airborne = if motion.grounded {
        0.0
    } else {
        (0.35 + motion.airborne_time * 3.2).clamp(0.0, 1.0)
    };
    let jump_up = (jump.vertical_velocity / PLAYER_JUMP_SPEED).clamp(0.0, 1.0);
    let fall = (-jump.vertical_velocity / PLAYER_JUMP_SPEED).clamp(0.0, 1.0);
    let breathe = (elapsed * 2.4).sin();
    let body_bob =
        breathe * 0.012 * (1.0 - gait) + stride_cos.abs() * gait * 0.026 + jump_up * 0.020
            - fall * 0.025;
    let torso_pitch = -gait * 0.08 + jump_up * 0.10 - fall * 0.12 - attack * 0.10;
    let torso_roll = stride_sin * gait * 0.035;
    let head_pitch = camera_pitch * 0.25 + jump_up * 0.05 - fall * 0.06;
    let swing = stride_sin * gait;
    let foot_spread = 0.15 + gait * 0.02;
    let ground_y = 0.06;
    let foot_stride = 0.30 * gait;
    let left_lift = stride_sin.max(0.0) * gait * 0.15;
    let right_lift = (-stride_sin).max(0.0) * gait * 0.15;
    let air_tuck = airborne * (0.10 + jump_up * 0.08);
    let left_foot = Vec3::new(
        -foot_spread,
        ground_y + left_lift + air_tuck,
        -0.03 + stride_sin * foot_stride - airborne * 0.10,
    );
    let right_foot = Vec3::new(
        foot_spread,
        ground_y + right_lift + air_tuck,
        -0.03 - stride_sin * foot_stride - airborne * 0.10,
    );

    let (left_hand, right_hand, left_elbow_pole, right_elbow_pole) =
        procedural_arm_pose(swing, attack, airborne, first_person);

    ProceduralPlayerPose {
        gait,
        stride_phase,
        attack,
        airborne,
        body_bob,
        torso_pitch,
        torso_roll,
        head_pitch,
        left_hand,
        right_hand,
        left_foot,
        right_foot,
        left_elbow_pole,
        right_elbow_pole,
        left_knee_pole: Vec3::new(-0.27, 0.25 + airborne * 0.10, -0.30),
        right_knee_pole: Vec3::new(0.27, 0.25 + airborne * 0.10, -0.30),
        left_boot_pitch: gait * stride_sin.max(0.0) * -0.42 + airborne * 0.25,
        right_boot_pitch: gait * (-stride_sin).max(0.0) * -0.42 + airborne * 0.25,
    }
}

fn procedural_arm_pose(
    swing: f32,
    attack: f32,
    airborne: f32,
    first_person: bool,
) -> (Vec3, Vec3, Vec3, Vec3) {
    if first_person {
        (
            Vec3::new(-0.25, 1.04 + swing * 0.04 + airborne * 0.05, -0.50),
            Vec3::new(
                0.25,
                1.02 - swing * 0.04 + attack * 0.07 + airborne * 0.04,
                -0.54 - attack * 0.42,
            ),
            Vec3::new(-0.58, 0.90, -0.36),
            Vec3::new(0.58, 0.90, -0.36 - attack * 0.12),
        )
    } else {
        (
            Vec3::new(
                -0.40,
                0.58 - swing * 0.08 + airborne * 0.10,
                -0.08 + swing * 0.12,
            ),
            Vec3::new(
                0.40,
                0.58 + swing * 0.08 + attack * 0.18 + airborne * 0.10,
                -0.08 - swing * 0.12 - attack * 0.38,
            ),
            Vec3::new(-0.58, 0.80 + airborne * 0.10, -0.32),
            Vec3::new(0.58, 0.80 + airborne * 0.10, -0.32 - attack * 0.10),
        )
    }
}

fn smooth01(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn hides_in_first_person(kind: PlayerIkPartKind) -> bool {
    matches!(
        kind,
        PlayerIkPartKind::Neck
            | PlayerIkPartKind::Head
            | PlayerIkPartKind::EyeL
            | PlayerIkPartKind::EyeR
    )
}
