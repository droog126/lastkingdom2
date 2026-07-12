//! Player controls, follow camera, and per-frame scene clock.

use avian3d::prelude::{LinearVelocity, Rotation, SpatialQuery, SpatialQueryFilter};
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_hanabi::ParticleEffect;
use bevy_tnua::builtins::{TnuaBuiltinJump, TnuaBuiltinWalk};
use bevy_tnua::prelude::{TnuaController, TnuaScheme};

use super::farm_ui::FarmingUiState;
use super::inventory::InventoryUiState;
use super::keybindings::{GameAction, KeyBindings};
use super::procedural_rig::{
    HumanoidRig, HumanoidTargets, apply_local_pose, apply_local_segment, local_to_world,
};
use super::state::{
    BossActor, CameraMode, CollisionDebugState, DragonKatanaPickup, HeldWeaponVisual, HitImpactFx,
    HitReaction, LivingCameraRig, LivingSceneCamera, LivingSceneState, PlayerActor, PlayerIkPart,
    PlayerIkPartKind, PlayerJump, PlayerMotion, PlayerSkillState, ProceduralTerrainSurface,
    SlashFx, configure_collision_debug_gizmos,
};
use super::util::{PLAYER_JUMP_SPEED, PLAYER_PHYSICS_CENTER_HEIGHT, PLAYER_SPEED};
use lk2_core::legendary::{LegendaryLoadout, LegendaryWeapon};
use lk2_core::pvp::{Health, PvpCombatant, SimpleWeapon, resolve_melee_attack};

#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk)]
pub enum PlayerControlScheme {
    Jump(TnuaBuiltinJump),
}

pub fn advance_scene(
    time: Res<Time>,
    mut state: ResMut<LivingSceneState>,
    mut combatants: Query<(&mut PvpCombatant, Option<&mut PlayerSkillState>)>,
) {
    let dt = time.delta_secs();
    state.elapsed += dt;
    state.frame += 1;
    if dt > 0.05 {
        state.frame_dt_over_50ms = state.frame_dt_over_50ms.saturating_add(1);
    }
    state.frame_dt_max_ms = state.frame_dt_max_ms.max(dt * 1000.0);
    state.attack_flash = (state.attack_flash - dt).max(0.0);
    state.camera_shake = (state.camera_shake - dt * 3.6).max(0.0);
    for (mut combatant, mut skill) in &mut combatants {
        combatant.tick(dt);
        if let Some(ref mut skill) = skill {
            skill.tick(dt);
        }
    }
}

pub fn update_cursor_capture(
    bindings: Res<KeyBindings>,
    inventory: Res<InventoryUiState>,
    farming: Option<Res<FarmingUiState>>,
    mut cursor_options: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    let gameplay_active = !bindings.menu_open && !inventory.open && !farming_open;
    let visible = !gameplay_active;
    let grab_mode = if gameplay_active {
        CursorGrabMode::Locked
    } else {
        CursorGrabMode::None
    };

    let Ok(mut cursor_options) = cursor_options.single_mut() else {
        return;
    };
    if cursor_options.visible != visible {
        cursor_options.visible = visible;
    }
    if cursor_options.grab_mode != grab_mode {
        cursor_options.grab_mode = grab_mode;
    }
}

pub fn camera_look_input(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut mouse_motion: MessageReader<MouseMotion>,
    mut camera_rig: ResMut<LivingCameraRig>,
    bindings: Res<KeyBindings>,
    inventory: Res<InventoryUiState>,
    farming: Option<Res<FarmingUiState>>,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    if bindings.menu_open || inventory.open || farming_open {
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
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    camera_rig: Res<LivingCameraRig>,
    bindings: Res<KeyBindings>,
    terrain: Res<ProceduralTerrainSurface>,
    inventory: Res<InventoryUiState>,
    farming: Option<Res<FarmingUiState>>,
    mut players: Query<(&Transform, &mut TnuaController<PlayerControlScheme>), With<PlayerActor>>,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    let Ok((player, mut controller)) = players.single_mut() else {
        return;
    };
    if bindings.menu_open || inventory.open || farming_open {
        controller.initiate_action_feeding();
        controller.basis = TnuaBuiltinWalk::default();
        return;
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
    let direction =
        (camera_forward * input_forward + camera_right * input_right).normalize_or_zero();
    let desired_motion = if direction.length_squared() > 0.0
        && terrain_cell_is_water(&terrain, player.translation + direction * 0.35)
    {
        Vec3::ZERO
    } else {
        direction
    };

    controller.initiate_action_feeding();
    controller.basis = TnuaBuiltinWalk {
        desired_motion,
        desired_forward: Dir3::new(if desired_motion.length_squared() > 0.0 {
            // The avatar mesh faces local +Z, while Tnua/Bevy define forward as -Z.
            -desired_motion
        } else {
            -camera_forward
        })
        .ok(),
    };
    if bindings.pressed(GameAction::Jump, &keys, &mouse) {
        controller.action(PlayerControlScheme::Jump(TnuaBuiltinJump::default()));
    }
}

fn terrain_cell_is_water(terrain: &ProceduralTerrainSurface, position: Vec3) -> bool {
    let x = 48 + position.x.floor() as i32;
    let z = 48 + position.z.floor() as i32;
    let surface = terrain.pipeline.surface_f32(x, z).unwrap_or(13.0);
    terrain.is_water_at(x, z, surface)
}

pub fn sync_player_physics_state(
    time: Res<Time>,
    camera_rig: Res<LivingCameraRig>,
    mut players: Query<
        (
            &LinearVelocity,
            &TnuaController<PlayerControlScheme>,
            &mut Rotation,
            &mut PlayerMotion,
            &mut PlayerJump,
        ),
        With<PlayerActor>,
    >,
) {
    let Ok((velocity, controller, mut rotation, mut motion, mut jump)) = players.single_mut()
    else {
        return;
    };
    let dt = time.delta_secs().max(0.0001);
    let planar_velocity = Vec3::new(velocity.0.x, 0.0, velocity.0.z);
    let grounded = !controller.is_airborne().unwrap_or(true);
    motion.planar_velocity = planar_velocity;
    motion.planar_speed = planar_velocity.length();
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
    jump.vertical_velocity = velocity.0.y;
    jump.coyote_timer = if grounded { 0.12 } else { 0.0 };
    jump.jump_buffer_timer = 0.0;
    let direction = planar_velocity.normalize_or_zero();
    if direction.length_squared() > 0.0 {
        rotation.0 = match camera_rig.mode {
            CameraMode::FirstPerson => Quat::from_rotation_y(camera_rig.yaw),
            CameraMode::ThirdPerson => Quat::from_rotation_y(direction.x.atan2(direction.z)),
        };
    }
}

pub fn player_combat_controls(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    mut commands: Commands,
    mut state: ResMut<LivingSceneState>,
    scene_materials: Res<super::state::SceneMaterials>,
    inventory: Res<InventoryUiState>,
    farming: Option<Res<FarmingUiState>>,
    loadout: Option<Res<LegendaryLoadout>>,
    mut players: Query<
        (
            &Transform,
            &mut PvpCombatant,
            &SimpleWeapon,
            Option<&mut PlayerSkillState>,
        ),
        With<PlayerActor>,
    >,
    mut bosses: Query<
        (&Transform, &mut Health, &mut HitReaction),
        (With<BossActor>, Without<PlayerActor>),
    >,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    if bindings.menu_open || inventory.open || farming_open {
        return;
    }
    let Ok((player, mut combatant, weapon, mut skill_state)) = players.single_mut() else {
        return;
    };
    let dragon_equipped = loadout
        .as_ref()
        .and_then(|loadout| loadout.equipped)
        .is_some_and(|weapon| weapon == LegendaryWeapon::DragonKatana);
    let skill = bindings.just_pressed(GameAction::WeaponSkill, &keys, &mouse);
    let attack_weapon = if skill {
        if !dragon_equipped {
            return;
        }
        let Some(ref mut skill_state) = skill_state else {
            return;
        };
        if !skill_state.begin(4.0) {
            return;
        }
        let mut weapon = *weapon;
        weapon.damage *= 3.0;
        weapon.reach *= 1.35;
        weapon.sweep_angle_deg = 110.0;
        weapon
    } else {
        if !bindings.just_pressed(GameAction::Attack, &keys, &mouse)
            || !combatant.begin_attack(*weapon)
        {
            return;
        }
        *weapon
    };
    let player_position = player.translation - Vec3::Y * PLAYER_PHYSICS_CENTER_HEIGHT;
    state.attack_flash = if skill { 0.52 } else { 0.32 };
    if let Ok((boss, mut health, mut reaction)) = bosses.single_mut() {
        if let Some(hit) = resolve_melee_attack(
            player_position,
            *player.back(),
            boss.translation,
            attack_weapon,
        ) {
            let actual_damage = health.damage(hit.damage, state.frame as u32, 0);
            if actual_damage > 0.0 {
                reaction.trigger(hit.knockback, 0.18);
                state.camera_shake = state.camera_shake.max(0.11);
                commands.spawn((
                    ParticleEffect::new(scene_materials.hit_effect.clone()),
                    Transform::from_translation(hit.hit_pos + Vec3::Y * 0.86)
                        .with_scale(Vec3::splat(0.24)),
                    HitImpactFx {
                        age: 0.0,
                        lifetime: 0.22,
                    },
                    Name::new("hit_impact"),
                ));
            }
        }
    }
    commands.spawn((
        ParticleEffect::new(scene_materials.hit_effect.clone()),
        Transform::from_translation(player_position + Vec3::Y * 0.9).with_scale(if skill {
            Vec3::new(1.6, 0.52, 1.6)
        } else {
            Vec3::new(1.0, 0.35, 1.0)
        }),
        SlashFx,
    ));
}

pub fn pickup_dragon_katana(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    mut commands: Commands,
    mut loadout: ResMut<LegendaryLoadout>,
    mut players: Query<(&Transform, &mut SimpleWeapon), With<PlayerActor>>,
    pickups: Query<(Entity, &Transform), With<DragonKatanaPickup>>,
) {
    if bindings.menu_open || !bindings.just_pressed(GameAction::Interact, &keys, &mouse) {
        return;
    }
    let Ok((player, mut weapon)) = players.single_mut() else {
        return;
    };
    let Some((entity, _)) = pickups.iter().find(|(_, pickup)| {
        pickup.translation.distance_squared(player.translation) <= 2.5_f32.powi(2)
    }) else {
        return;
    };
    loadout.grant_and_equip(LegendaryWeapon::DragonKatana);
    *weapon = dragon_katana_weapon();
    commands.entity(entity).despawn();
}

fn dragon_katana_weapon() -> SimpleWeapon {
    SimpleWeapon {
        reach: 3.6,
        damage: 10.0,
        knockback: 5.5,
        cooldown_secs: 0.5,
        sweep_angle_deg: 75.0,
    }
}

pub fn update_camera(
    time: Res<Time>,
    state: Res<LivingSceneState>,
    bindings: Res<KeyBindings>,
    inventory: Res<InventoryUiState>,
    farming: Option<Res<FarmingUiState>>,
    camera_rig: Res<LivingCameraRig>,
    spatial_query: SpatialQuery,
    players: Query<(Entity, &Transform), (With<PlayerActor>, Without<LivingSceneCamera>)>,
    mut cameras: Query<&mut Transform, (With<LivingSceneCamera>, Without<PlayerActor>)>,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    if bindings.menu_open || inventory.open || farming_open {
        return;
    }
    let (Ok((player_entity, player)), Ok(mut camera)) = (players.single(), cameras.single_mut())
    else {
        return;
    };
    let visual_position = player.translation - Vec3::Y * PLAYER_PHYSICS_CENTER_HEIGHT;
    let shake = camera_shake_offset(state.elapsed, state.camera_shake);
    match camera_rig.mode {
        CameraMode::FirstPerson => {
            let eye = first_person_eye(visual_position);
            let forward = camera_direction(camera_rig.yaw, camera_rig.pitch);
            camera.translation = eye + forward * 0.08 + shake;
            let target = camera.translation + forward + shake * 0.25;
            camera.look_at(target, Vec3::Y);
        }
        CameraMode::ThirdPerson => {
            let target = visual_position + Vec3::Y * 1.05;
            let desired = third_person_camera_position(target, camera_rig.yaw, camera_rig.pitch);
            let desired = camera_collision_position(&spatial_query, player_entity, target, desired);
            camera.translation = camera
                .translation
                .lerp(desired, 1.0 - (-time.delta_secs() * 5.5).exp())
                + shake;
            camera.look_at(target + shake * 0.25, Vec3::Y);
        }
    }
}

fn camera_shake_offset(elapsed: f32, strength: f32) -> Vec3 {
    let phase = elapsed * 92.0;
    Vec3::new(
        phase.sin() * strength,
        (phase * 1.37).cos() * strength * 0.55,
        (phase * 0.83).sin() * strength * 0.35,
    )
}

fn camera_collision_position(
    spatial_query: &SpatialQuery,
    player_entity: Entity,
    target: Vec3,
    desired: Vec3,
) -> Vec3 {
    let offset = desired - target;
    let distance = offset.length();
    let Ok(direction) = Dir3::new(offset / distance.max(0.0001)) else {
        return desired;
    };
    let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);
    let safe_distance = spatial_query
        .cast_ray(target, direction, distance, true, &filter)
        .map_or(distance, |hit| (hit.distance - 0.25).max(0.8));
    target + direction * safe_distance
}

pub fn update_player_ik(
    state: Res<LivingSceneState>,
    camera_rig: Res<LivingCameraRig>,
    terrain: Res<ProceduralTerrainSurface>,
    loadout: Option<Res<LegendaryLoadout>>,
    players: Query<(&Transform, &PlayerMotion, &PlayerJump), With<PlayerActor>>,
    mut parts: Query<
        (&PlayerIkPart, &mut Transform, Option<&mut Visibility>),
        Without<PlayerActor>,
    >,
    mut weapon_visuals: Query<(&mut Transform, &mut Visibility), With<HeldWeaponVisual>>,
) {
    let Ok((player, motion, jump)) = players.single() else {
        return;
    };
    let visual_player = Transform {
        translation: player.translation - Vec3::Y * PLAYER_PHYSICS_CENTER_HEIGHT,
        ..*player
    };
    let player = &visual_player;
    let first_person = camera_rig.mode == CameraMode::FirstPerson;
    let dragon_equipped = loadout
        .as_ref()
        .and_then(|loadout| loadout.equipped)
        .is_some_and(|weapon| weapon == LegendaryWeapon::DragonKatana);
    let pose = procedural_player_pose(
        state.elapsed,
        state.attack_flash,
        motion,
        jump,
        first_person,
        camera_rig.pitch,
    );
    let (left_foot, left_foot_slope) =
        terrain_foot_pose(player, pose.left_foot, &terrain, motion.grounded);
    let (right_foot, right_foot_slope) =
        terrain_foot_pose(player, pose.right_foot, &terrain, motion.grounded);
    let body_slope = left_foot_slope
        .slerp(right_foot_slope, 0.5)
        .slerp(Quat::IDENTITY, 0.55);

    let solved = HumanoidRig::player_avatar().solve(HumanoidTargets {
        left_hand: pose.left_hand,
        right_hand: pose.right_hand,
        left_foot,
        right_foot,
        left_elbow_pole: pose.left_elbow_pole,
        right_elbow_pole: pose.right_elbow_pole,
        left_knee_pole: pose.left_knee_pole,
        right_knee_pole: pose.right_knee_pole,
    });

    for (part, mut transform, visibility) in &mut parts {
        if let Some(mut visibility) = visibility {
            *visibility = if part.kind == PlayerIkPartKind::Stick && dragon_equipped {
                Visibility::Hidden
            } else if first_person && hides_in_first_person(part.kind) {
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
                body_slope
                    * Quat::from_rotation_x(pose.torso_pitch)
                    * Quat::from_rotation_z(pose.torso_roll),
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
                Vec3::new(-0.11, 1.46, 0.32),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::EyeR => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.11, 1.46, 0.32),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::ArmUpperL => apply_local_segment(
                &mut transform,
                player,
                solved.left_arm.root,
                solved.left_arm.joint,
            ),
            PlayerIkPartKind::ArmLowerL => apply_local_segment(
                &mut transform,
                player,
                solved.left_arm.joint,
                solved.left_arm.target,
            ),
            PlayerIkPartKind::ArmUpperR => apply_local_segment(
                &mut transform,
                player,
                solved.right_arm.root,
                solved.right_arm.joint,
            ),
            PlayerIkPartKind::ArmLowerR => apply_local_segment(
                &mut transform,
                player,
                solved.right_arm.joint,
                solved.right_arm.target,
            ),
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
            PlayerIkPartKind::LegUpperL => apply_local_segment(
                &mut transform,
                player,
                solved.left_leg.root,
                solved.left_leg.joint,
            ),
            PlayerIkPartKind::LegLowerL => apply_local_segment(
                &mut transform,
                player,
                solved.left_leg.joint,
                solved.left_leg.target,
            ),
            PlayerIkPartKind::LegUpperR => apply_local_segment(
                &mut transform,
                player,
                solved.right_leg.root,
                solved.right_leg.joint,
            ),
            PlayerIkPartKind::LegLowerR => apply_local_segment(
                &mut transform,
                player,
                solved.right_leg.joint,
                solved.right_leg.target,
            ),
            PlayerIkPartKind::BootL => apply_local_pose(
                &mut transform,
                player,
                left_foot + Vec3::new(0.0, -0.02, 0.08),
                left_foot_slope * Quat::from_rotation_x(pose.left_boot_pitch),
                Vec3::ONE,
            ),
            PlayerIkPartKind::BootR => apply_local_pose(
                &mut transform,
                player,
                right_foot + Vec3::new(0.0, -0.02, 0.08),
                right_foot_slope * Quat::from_rotation_x(pose.right_boot_pitch),
                Vec3::ONE,
            ),
            PlayerIkPartKind::Basket => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.0, 0.76, -0.28),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::Stick => {
                let (start, end) = held_stick_segment(&pose);
                apply_local_segment(&mut transform, player, start, end);
            }
        }
    }

    for (mut transform, mut visibility) in &mut weapon_visuals {
        if dragon_equipped {
            *visibility = Visibility::Visible;
            transform.translation =
                local_to_world(player, pose.left_hand + Vec3::new(-0.02, -0.05, 0.04));
            transform.rotation = player.rotation;
            transform.scale = Vec3::splat(0.32);
        } else {
            *visibility = Visibility::Hidden;
        }
    }
}

pub fn toggle_collision_debug(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<KeyBindings>,
    mut debug: ResMut<CollisionDebugState>,
) {
    if !bindings.menu_open && keys.just_pressed(KeyCode::F3) {
        debug.enabled = !debug.enabled;
    }
}

pub fn update_collision_debug(
    debug: Res<CollisionDebugState>,
    mut gizmos: ResMut<GizmoConfigStore>,
) {
    if !debug.is_changed() {
        return;
    }
    configure_collision_debug_gizmos(&mut gizmos, debug.enabled);
}

fn terrain_foot_pose(
    player: &Transform,
    foot: Vec3,
    terrain: &ProceduralTerrainSurface,
    grounded: bool,
) -> (Vec3, Quat) {
    if !grounded {
        return (foot, Quat::IDENTITY);
    }

    let foot_world = local_to_world(player, Vec3::new(foot.x, 0.0, foot.z));
    let ground_world_y = terrain.ground_height(foot_world);
    let ground_local = player.rotation.inverse()
        * (Vec3::new(foot_world.x, ground_world_y, foot_world.z) - player.translation);
    let contact_foot = Vec3::new(foot.x, foot.y.max(ground_local.y + 0.03), foot.z);
    let local_normal = player.rotation.inverse() * terrain.surface_normal(foot_world);
    let slope = Quat::from_rotation_arc(Vec3::Y, local_normal);
    (contact_foot, slope)
}

fn yaw_forward(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

pub(crate) fn third_person_camera_position(target: Vec3, yaw: f32, pitch: f32) -> Vec3 {
    let horizontal_forward = yaw_forward(yaw);
    let right = Vec3::new(-horizontal_forward.z, 0.0, horizontal_forward.x);
    let orbit_pitch = (pitch - 0.36).clamp(-1.25, 0.85);
    target - camera_direction(yaw, orbit_pitch) * 6.2 + right * 0.9
}

fn camera_direction(yaw: f32, pitch: f32) -> Vec3 {
    let flat = pitch.cos();
    Vec3::new(yaw.sin() * flat, pitch.sin(), yaw.cos() * flat).normalize()
}

pub fn first_person_eye(player_position: Vec3) -> Vec3 {
    player_position + Vec3::new(0.0, 1.54, 0.0)
}

pub(crate) fn held_stick_segment(pose: &ProceduralPlayerPose) -> (Vec3, Vec3) {
    (
        pose.left_hand + Vec3::new(-0.02, -0.05, 0.06),
        pose.left_hand + Vec3::new(-0.34, -0.18, 0.52 + pose.attack * 0.20),
    )
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
        breathe * 0.012 * (1.0 - gait) + stride_sin.abs() * gait * 0.040 + jump_up * 0.020
            - fall * 0.025;
    let torso_pitch = -gait * 0.13 + jump_up * 0.10 - fall * 0.12 - attack * 0.10;
    let torso_roll = stride_sin * gait * 0.055;
    let head_pitch = camera_pitch * 0.25 + jump_up * 0.05 - fall * 0.06;
    let swing = stride_sin * gait;
    let foot_spread = 0.16 + gait * 0.03;
    let ground_y = 0.06;
    let foot_stride = 0.42 * gait;
    let left_swing = stride_cos.max(0.0);
    let right_swing = (-stride_cos).max(0.0);
    let left_lift = left_swing.powf(0.75) * gait * 0.22;
    let right_lift = right_swing.powf(0.75) * gait * 0.22;
    let air_tuck = airborne * (0.10 + jump_up * 0.08);
    let left_foot = Vec3::new(
        -foot_spread,
        ground_y + left_lift + air_tuck,
        0.03 + stride_sin * foot_stride + airborne * 0.10,
    );
    let right_foot = Vec3::new(
        foot_spread,
        ground_y + right_lift + air_tuck,
        0.03 - stride_sin * foot_stride + airborne * 0.10,
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
        left_knee_pole: Vec3::new(
            -0.29,
            0.28 + left_lift * 0.45 + airborne * 0.10,
            0.26 + left_swing * 0.14,
        ),
        right_knee_pole: Vec3::new(
            0.29,
            0.28 + right_lift * 0.45 + airborne * 0.10,
            0.26 + right_swing * 0.14,
        ),
        left_boot_pitch: gait * (left_swing * -0.58 + right_swing * 0.16) + airborne * 0.25,
        right_boot_pitch: gait * (right_swing * -0.58 + left_swing * 0.16) + airborne * 0.25,
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
            Vec3::new(
                -0.25,
                1.04 - swing * 0.05 + attack * 0.07 + airborne * 0.05,
                0.50 - swing * 0.10 + attack * 0.42,
            ),
            Vec3::new(
                0.25,
                1.02 + swing * 0.05 + airborne * 0.04,
                0.54 + swing * 0.10,
            ),
            Vec3::new(-0.58, 0.90, 0.36 + attack * 0.12),
            Vec3::new(0.58, 0.90, 0.36),
        )
    } else {
        (
            Vec3::new(
                -0.40,
                0.55 - swing * 0.08 + attack * 0.18 + airborne * 0.10,
                0.16 - swing * 0.14 + attack * 0.38,
            ),
            Vec3::new(
                0.40,
                0.55 + swing * 0.08 + airborne * 0.10,
                0.16 + swing * 0.14,
            ),
            Vec3::new(-0.58, 0.80 + airborne * 0.10, 0.32 + attack * 0.10),
            Vec3::new(0.58, 0.80 + airborne * 0.10, 0.32),
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
