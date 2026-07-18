//! Player controls, follow camera, and per-frame scene clock.

use avian3d::prelude::{
    Collider, GravityScale, LinearVelocity, RigidBody, Rotation, SpatialQuery, SpatialQueryFilter,
};
use bevy::ecs::system::SystemParam;
use bevy::input::mouse::{AccumulatedMouseScroll, MouseMotion, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use bevy_hanabi::ParticleEffect;
use bevy_tnua::builtins::{TnuaBuiltinJump, TnuaBuiltinWalk};
use bevy_tnua::prelude::{TnuaConfig, TnuaController, TnuaScheme};

use super::farm_ui::FarmingUiState;
use super::inventory::InventoryUiState;
use super::keybindings::{GameAction, KeyBindings};
use super::offline::OfflineNature;
use super::procedural_motion::{ProceduralAnimationConfig, smooth_follow_alpha};
use super::procedural_rig::{
    HumanoidRig, HumanoidTargets, apply_local_pose, apply_local_segment, local_to_world,
};
use super::state::CodexSceneInput;
use super::state::{
    CameraMode, CaveEntrance, CaveExit, CaveTravelState, CollisionDebugState, CreatureHitSettings,
    DefeatedCreature, DimensionId, DimensionTravelState, DragonKatanaPickup, HeldWeaponVisual,
    HitImpactFx, HitReaction, LivingCameraRig, LivingSceneCamera, LivingSceneState,
    MINE_SURFACE_DEFORMATION_RADIUS, PlayerActor, PlayerGroundShadow, PlayerIkPart,
    PlayerIkPartKind, PlayerJump, PlayerMotion, PlayerSkillState, PlayerSwimState,
    ProceduralTerrainSurface, ReaperScythePickup, SlashFx, TerrainRebuildState,
    TerrainUndergroundState, configure_collision_debug_gizmos,
};
use super::util::{
    PLAYER_COLLIDER_RADIUS, PLAYER_COLLIDER_SEGMENT, PLAYER_CROUCH_COLLIDER_SEGMENT,
    PLAYER_JUMP_SPEED, PLAYER_SPEED, PLAYER_SPRINT_SPEED_MULTIPLIER, player_physics_half_height,
};
use super::vehicle::CartRideState;
use lk2_core::legendary::{LegendaryLoadout, LegendaryWeapon, StatusSet, resolve_reaper_strike};
use lk2_core::pvp::{Health, Hitbox, PvpCombatant, SimpleWeapon, resolve_melee_attack};
use lk2_core::vehicle::{CartDriveInput, cart_steering_from_right_input, step_cart_drive};
use lk2_core::world::{TerrainChunkCoord, mine_target_reachable, top_solid_surface_y};

#[derive(TnuaScheme)]
#[scheme(basis = TnuaBuiltinWalk)]
pub enum PlayerControlScheme {
    Jump(TnuaBuiltinJump),
}

#[derive(SystemParam)]
pub(crate) struct CreatureHitConfig<'w> {
    settings: Option<Res<'w, CreatureHitSettings>>,
    camera_rig: Option<Res<'w, LivingCameraRig>>,
}

const SWIM_SPEED: f32 = 3.8;
const SWIM_VERTICAL_SPEED: f32 = 3.0;
const SWIM_SURFACE_CLEARANCE: f32 = 0.42;
const SWIM_MIN_DEPTH: f32 = 0.85;
const SWIM_EXIT_CLEARANCE: f32 = 0.08;

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
    scroll: Option<Res<AccumulatedMouseScroll>>,
    mut camera_rig: ResMut<LivingCameraRig>,
    bindings: Res<KeyBindings>,
    inventory: Res<InventoryUiState>,
    farming: Option<Res<FarmingUiState>>,
    cameras: Query<&Transform, With<LivingSceneCamera>>,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    if bindings.menu_open || inventory.open || farming_open {
        return;
    }
    if bindings.just_pressed(GameAction::ToggleFreeCamera, &keys, &mouse) {
        if camera_rig.mode == CameraMode::FreeCam {
            camera_rig.mode = camera_rig.free_return_mode;
            camera_rig.yaw = camera_rig.free_yaw;
            camera_rig.pitch = camera_rig.free_pitch;
        } else {
            camera_rig.free_return_mode = camera_rig.mode;
            if let Ok(camera) = cameras.single() {
                camera_rig.free_position = camera.translation;
                let (yaw, pitch) = camera_angles(camera.forward().as_vec3());
                camera_rig.free_yaw = yaw;
                camera_rig.free_pitch = pitch;
            } else {
                camera_rig.free_yaw = camera_rig.yaw;
                camera_rig.free_pitch = camera_rig.pitch;
            }
            camera_rig.mode = CameraMode::FreeCam;
        }
    }
    if bindings.just_pressed(GameAction::ToggleGodView, &keys, &mouse)
        && camera_rig.mode != CameraMode::FreeCam
    {
        if camera_rig.mode == CameraMode::GodView {
            camera_rig.mode = camera_rig.god_return_mode;
        } else {
            camera_rig.god_return_mode = camera_rig.mode;
            camera_rig.mode = CameraMode::GodView;
        }
    }
    if bindings.just_pressed(GameAction::ToggleCamera, &keys, &mouse)
        && !matches!(camera_rig.mode, CameraMode::FreeCam | CameraMode::GodView)
    {
        camera_rig.mode = match camera_rig.mode {
            CameraMode::FirstPerson => CameraMode::ThirdPerson,
            CameraMode::ThirdPerson => CameraMode::FirstPerson,
            CameraMode::FreeCam => unreachable!("free camera is filtered above"),
            CameraMode::GodView => unreachable!("god view is filtered above"),
        };
    }

    let mut delta = Vec2::ZERO;
    for event in mouse_motion.read() {
        delta += event.delta;
    }
    if camera_rig.mode == CameraMode::GodView {
        if mouse.pressed(MouseButton::Right) {
            camera_rig.god_yaw -= delta.x * 0.0032;
            camera_rig.god_pitch = (camera_rig.god_pitch - delta.y * 0.0022).clamp(-1.38, -0.62);
        }
        let turn = if bindings.pressed(GameAction::CameraTurnLeft, &keys, &mouse) {
            1.0
        } else if bindings.pressed(GameAction::CameraTurnRight, &keys, &mouse) {
            -1.0
        } else {
            0.0
        };
        camera_rig.god_yaw += turn * time.delta_secs() * 1.8;

        let forward = yaw_forward(camera_rig.god_yaw);
        let right = Vec3::new(-forward.z, 0.0, forward.x);
        let mut movement = Vec3::ZERO;
        if bindings.pressed(GameAction::MoveForward, &keys, &mouse) {
            movement += forward;
        }
        if bindings.pressed(GameAction::MoveBackward, &keys, &mouse) {
            movement -= forward;
        }
        if bindings.pressed(GameAction::MoveRight, &keys, &mouse) {
            movement += right;
        }
        if bindings.pressed(GameAction::MoveLeft, &keys, &mouse) {
            movement -= right;
        }
        if movement.length_squared() > 0.0 {
            let speed = if bindings.pressed(GameAction::Sprint, &keys, &mouse) {
                34.0
            } else {
                14.0
            };
            camera_rig.god_focus += movement.normalize() * speed * time.delta_secs();
        }
        camera_rig.god_focus.x = camera_rig.god_focus.x.clamp(-64.0, 64.0);
        camera_rig.god_focus.z = camera_rig.god_focus.z.clamp(-64.0, 64.0);

        if let Some(scroll) = scroll {
            let wheel = match scroll.unit {
                MouseScrollUnit::Line => scroll.delta.y,
                MouseScrollUnit::Pixel => scroll.delta.y / 100.0,
            };
            camera_rig.god_distance =
                (camera_rig.god_distance - wheel * 12.0).clamp(72.0, 220.0);
        }
        return;
    }
    if camera_rig.mode == CameraMode::FreeCam {
        camera_rig.free_yaw -= delta.x * 0.0032;
        camera_rig.free_pitch = (camera_rig.free_pitch - delta.y * 0.0027).clamp(-1.50, 1.50);
        camera_rig.yaw = camera_rig.free_yaw;
        camera_rig.pitch = camera_rig.free_pitch;
    } else {
        camera_rig.yaw -= delta.x * 0.0032;
        camera_rig.pitch = (camera_rig.pitch - delta.y * 0.0027).clamp(-1.25, 0.92);
    }

    let turn = if bindings.pressed(GameAction::CameraTurnLeft, &keys, &mouse) {
        1.0
    } else if bindings.pressed(GameAction::CameraTurnRight, &keys, &mouse) {
        -1.0
    } else {
        0.0
    };
    if camera_rig.mode == CameraMode::FreeCam {
        camera_rig.free_yaw += turn * time.delta_secs() * 1.8;
        camera_rig.yaw = camera_rig.free_yaw;
    } else {
        camera_rig.yaw += turn * time.delta_secs() * 1.8;
    }
    if camera_rig.mode == CameraMode::FreeCam {
        update_free_camera_position(&time, &keys, &mouse, &bindings, &mut camera_rig);
    }
}

pub fn player_controls(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    camera_rig: Res<LivingCameraRig>,
    bindings: Res<KeyBindings>,
    terrain: Res<ProceduralTerrainSurface>,
    mut ride: ResMut<CartRideState>,
    mut configs: ResMut<Assets<PlayerControlSchemeConfig>>,
    inventory: Res<InventoryUiState>,
    farming: Option<Res<FarmingUiState>>,
    codex: Option<Res<CodexSceneInput>>,
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &Transform,
            &mut TnuaController<PlayerControlScheme>,
            &TnuaConfig<PlayerControlScheme>,
            &PlayerMotion,
            &mut GravityScale,
            &mut LinearVelocity,
            &RigidBody,
            &PlayerSwimState,
        ),
        With<PlayerActor>,
    >,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    let Ok((
        entity,
        player,
        mut controller,
        config_handle,
        motion,
        mut gravity,
        mut velocity,
        body,
        swim,
    )) = players.single_mut()
    else {
        return;
    };
    if let Some(mut config) = configs.get_mut(&config_handle.0) {
        config.basis.float_height = player_physics_half_height(motion.crouch_amount);
        config.basis.speed = if ride.mounted {
            PLAYER_SPEED * 2.0
        } else {
            PLAYER_SPEED
        };
    }
    gravity.0 = if swim.active { 0.0 } else { 1.0 };
    let desired_body = if swim.active {
        RigidBody::Kinematic
    } else {
        RigidBody::Dynamic
    };
    if *body != desired_body {
        commands.entity(entity).insert(desired_body);
    }
    if bindings.menu_open || inventory.open || farming_open {
        controller.initiate_action_feeding();
        controller.basis = TnuaBuiltinWalk::default();
        return;
    }
    if matches!(camera_rig.mode, CameraMode::FreeCam | CameraMode::GodView) {
        controller.initiate_action_feeding();
        controller.basis = TnuaBuiltinWalk::default();
        velocity.0 = Vec3::ZERO;
        return;
    }

    if swim.active {
        velocity.0 = Vec3::ZERO;
        controller.initiate_action_feeding();
        controller.basis = TnuaBuiltinWalk {
            desired_motion: Vec3::ZERO,
            desired_forward: None,
        };
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
    let keyboard_direction =
        (camera_forward * input_forward + camera_right * input_right).normalize_or_zero();
    let codex_direction = codex.as_ref().map_or(Vec3::ZERO, |input| {
        Vec3::new(input.motion.x, 0.0, input.motion.y)
    });
    let direction = if codex_direction.length_squared() > 0.0 {
        codex_direction.normalize_or_zero()
    } else {
        keyboard_direction
    };
    let desired_motion = if ride.mounted {
        step_cart_drive(
            &mut ride.drive,
            CartDriveInput {
                throttle: input_forward,
                steering: cart_steering_from_right_input(input_right),
            },
            time.delta_secs(),
        );
        let heading = Vec3::new(ride.drive.yaw.sin(), 0.0, ride.drive.yaw.cos());
        let travel_direction = heading * ride.drive.speed.signum();
        if travel_direction.length_squared() > 0.0
            && terrain_cell_is_water(&terrain, player.translation + travel_direction * 0.35)
        {
            ride.drive.speed = 0.0;
            Vec3::ZERO
        } else {
            travel_direction
        }
    } else {
        direction
    };

    let sprinting = !ride.mounted
        && desired_motion.length_squared() > 0.0
        && !bindings.pressed(GameAction::Crouch, &keys, &mouse)
        && bindings.pressed(GameAction::Sprint, &keys, &mouse);
    if let Some(mut config) = configs.get_mut(&config_handle.0) {
        config.basis.speed = if ride.mounted {
            ride.drive.speed.abs()
        } else if sprinting {
            PLAYER_SPEED * PLAYER_SPRINT_SPEED_MULTIPLIER
        } else {
            PLAYER_SPEED
        };
    }

    controller.initiate_action_feeding();
    controller.basis = TnuaBuiltinWalk {
        desired_motion,
        desired_forward: Dir3::new(if ride.mounted {
            // Keep the avatar facing the chassis while reversing; travel
            // direction and visual heading are intentionally separate.
            -Vec3::new(ride.drive.yaw.sin(), 0.0, ride.drive.yaw.cos())
        } else if desired_motion.length_squared() > 0.0 {
            // The avatar mesh faces local +Z, while Tnua/Bevy define forward as -Z.
            -desired_motion
        } else {
            -camera_forward
        })
        .ok(),
    };
    if !ride.mounted
        && (bindings.pressed(GameAction::Jump, &keys, &mouse)
            || codex.as_ref().is_some_and(|input| input.jump))
    {
        controller.action(PlayerControlScheme::Jump(TnuaBuiltinJump::default()));
    }
}

/// Move a player through the water volume after the physics walk controller
/// has run. The water surface is not a collider; the submerged terrain floor
/// and these explicit bounds provide the two meaningful vertical limits.
pub fn swim_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    camera_rig: Res<LivingCameraRig>,
    bindings: Res<KeyBindings>,
    terrain: Res<ProceduralTerrainSurface>,
    cave: Option<Res<CaveTravelState>>,
    inventory: Res<InventoryUiState>,
    farming: Option<Res<FarmingUiState>>,
    mut water_demo: Local<Option<bool>>,
    mut commands: Commands,
    mut players: Query<
        (
            Entity,
            &mut Transform,
            &mut PlayerSwimState,
            &PlayerMotion,
            &RigidBody,
        ),
        With<PlayerActor>,
    >,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    if bindings.menu_open || inventory.open || farming_open {
        return;
    }
    if cave.as_ref().is_some_and(|cave| cave.active) {
        return;
    }
    if matches!(camera_rig.mode, CameraMode::FreeCam | CameraMode::GodView) {
        return;
    }
    let Ok((entity, mut player, mut swim, motion, body)) = players.single_mut() else {
        return;
    };

    let half_height = player_physics_half_height(motion.crouch_amount);
    let current_depth = terrain.water_depth_at(player.translation);
    if !swim.active {
        let can_enter = current_depth.is_some_and(|depth| {
            depth >= SWIM_MIN_DEPTH
                && player.translation.y <= terrain.water_surface_height() + half_height
        });
        if !can_enter {
            return;
        }
        swim.active = true;
        swim.velocity = Vec3::ZERO;
        set_swim_rigid_body(&mut commands, entity, body, true);
    }

    let Some(depth) = terrain.water_depth_at(player.translation) else {
        swim.active = false;
        swim.velocity = Vec3::ZERO;
        set_swim_rigid_body(&mut commands, entity, body, false);
        player.translation.y = terrain.ground_height(player.translation) + half_height;
        return;
    };
    if depth < SWIM_MIN_DEPTH {
        swim.active = false;
        swim.velocity = Vec3::ZERO;
        set_swim_rigid_body(&mut commands, entity, body, false);
        player.translation.y = terrain.water_floor_height(player.translation) + half_height;
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
    let horizontal =
        (camera_forward * input_forward + camera_right * input_right).normalize_or_zero();
    let up = bindings.pressed(GameAction::Jump, &keys, &mouse);
    let down = bindings.pressed(GameAction::Crouch, &keys, &mouse);
    let water_demo =
        *water_demo.get_or_insert_with(|| std::env::args().any(|arg| arg == "--water-demo"));
    let vertical_input = if up == down {
        if water_demo {
            // Keep the deterministic validation camera submerged. Normal
            // gameplay retains the gentle surface buoyancy below.
            0.0
        } else {
            let target_y = terrain.water_surface_height() - SWIM_SURFACE_CLEARANCE;
            (target_y - player.translation.y) * 1.8
        }
    } else if up {
        SWIM_VERTICAL_SPEED
    } else {
        -SWIM_VERTICAL_SPEED
    };
    let desired = horizontal * SWIM_SPEED
        + Vec3::Y * vertical_input.clamp(-SWIM_VERTICAL_SPEED, SWIM_VERTICAL_SPEED);
    let dt = time.delta_secs().min(0.05);
    let candidate = player.translation + desired * dt;
    let candidate_depth = terrain.water_depth_at(candidate);
    if candidate_depth.is_none() {
        if let Some(exit) = shore_exit_position(&terrain, candidate, half_height) {
            swim.active = false;
            swim.velocity = Vec3::ZERO;
            set_swim_rigid_body(&mut commands, entity, body, false);
            player.translation = exit;
            return;
        }
        swim.active = false;
        swim.velocity = Vec3::ZERO;
        set_swim_rigid_body(&mut commands, entity, body, false);
        player.translation = Vec3::new(
            candidate.x,
            terrain.ground_height(candidate) + half_height,
            candidate.z,
        );
        return;
    }
    let floor_y = terrain.water_floor_height(candidate) + half_height + 0.05;
    let surface_y = terrain.water_surface_height() + SWIM_SURFACE_CLEARANCE;
    if floor_y > surface_y {
        swim.active = false;
        swim.velocity = Vec3::ZERO;
        set_swim_rigid_body(&mut commands, entity, body, false);
        player.translation.y = floor_y;
        return;
    }
    player.translation = Vec3::new(
        candidate.x,
        candidate.y.clamp(floor_y, surface_y),
        candidate.z,
    );
    // `swim_player` owns the kinematic root's position. Keep this velocity in
    // swim state for animation/heading only; feeding it to Avian would make
    // the physics schedule move the body again on the next tick.
    swim.velocity = desired;
}

fn set_swim_rigid_body(
    commands: &mut Commands,
    entity: Entity,
    body: &RigidBody,
    swimming: bool,
) {
    if let Some(desired) = swim_rigid_body_for(body, swimming) {
        commands.entity(entity).insert(desired);
    }
}

fn swim_rigid_body_for(body: &RigidBody, swimming: bool) -> Option<RigidBody> {
    let desired = if swimming {
        RigidBody::Kinematic
    } else {
        RigidBody::Dynamic
    };
    (*body != desired).then_some(desired)
}

/// Project a swimming step onto a nearby land sample. Sampling a small ring
/// around the candidate avoids grounding on the bilinear water/land seam,
/// which can leave the capsule intersecting the first shore triangle.
fn shore_exit_position(
    terrain: &ProceduralTerrainSurface,
    candidate: Vec3,
    half_height: f32,
) -> Option<Vec3> {
    const OFFSETS: [Vec2; 9] = [
        Vec2::ZERO,
        Vec2::new(0.22, 0.0),
        Vec2::new(-0.22, 0.0),
        Vec2::new(0.0, 0.22),
        Vec2::new(0.0, -0.22),
        Vec2::new(0.16, 0.16),
        Vec2::new(-0.16, 0.16),
        Vec2::new(0.16, -0.16),
        Vec2::new(-0.16, -0.16),
    ];

    OFFSETS
        .into_iter()
        .map(|offset| Vec3::new(candidate.x + offset.x, 0.0, candidate.z + offset.y))
        .filter(|sample| terrain.water_depth_at(*sample).is_none())
        .min_by(|left, right| {
            let left_distance = left.distance_squared(Vec3::new(candidate.x, 0.0, candidate.z));
            let right_distance = right.distance_squared(Vec3::new(candidate.x, 0.0, candidate.z));
            left_distance.total_cmp(&right_distance)
        })
        .map(|land| {
            Vec3::new(
                land.x,
                terrain.ground_height(land) + half_height + SWIM_EXIT_CLEARANCE,
                land.z,
            )
        })
}

/// Drive the presentation-only crouch blend. The physics capsule remains
/// unchanged; this keeps the feature safe while the pose is being tuned from
/// the runtime settings panel.
pub fn update_player_crouch(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    ride: Res<CartRideState>,
    terrain: Res<ProceduralTerrainSurface>,
    config: Option<Res<ProceduralAnimationConfig>>,
    inventory: Res<InventoryUiState>,
    farming: Option<Res<FarmingUiState>>,
    mut queries: ParamSet<(
        Query<
            (
                &Transform,
                &mut PlayerMotion,
                &mut Collider,
                Option<&PlayerSwimState>,
                Option<&Children>,
            ),
            With<PlayerActor>,
        >,
        Query<
            (&mut Transform, &mut Visibility),
            (With<PlayerGroundShadow>, Without<PlayerActor>),
        >,
    )>,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    let gameplay_active = !bindings.menu_open && !inventory.open && !farming_open;
    let target = if gameplay_active
        && !ride.mounted
        && bindings.pressed(GameAction::Crouch, &keys, &mouse)
    {
        1.0
    } else {
        0.0
    };
    let transition_speed = config.map_or(10.0, |config| config.crouch_transition_speed);
    let blend = smooth_follow_alpha(time.delta_secs(), transition_speed);
    let mut shadow_updates = Vec::new();
    {
        let mut players = queries.p0();
        for (player, mut motion, mut collider, swim, children) in &mut players {
            let swimming = swim.is_some_and(|swim| swim.active);
            motion.crouch_target = target;
            motion.crouch_amount = motion.crouch_amount.lerp(target, blend);
            if (motion.collider_crouch_amount - motion.crouch_amount).abs() > 0.005 {
                let segment = PLAYER_COLLIDER_SEGMENT.lerp(
                    PLAYER_CROUCH_COLLIDER_SEGMENT,
                    motion.crouch_amount.clamp(0.0, 1.0),
                );
                *collider = Collider::capsule(PLAYER_COLLIDER_RADIUS, segment);
                motion.collider_crouch_amount = motion.crouch_amount;
            }
            let shadow_y = -player_physics_half_height(motion.crouch_amount) + 0.015;
            if let Some(children) = children {
                shadow_updates.extend(
                    children
                        .iter()
                        .map(|child| (child, shadow_y, player.translation, swimming)),
                );
            }
        }
    }
    let mut shadows = queries.p1();
    for (child, shadow_y, player_translation, swimming) in shadow_updates {
        if let Ok((mut transform, mut visibility)) = shadows.get_mut(child) {
            // The avatar is rotated onto its swim axis, but this contact decal
            // remains a flat land-space cue attached to the vertical physics
            // capsule. Drawing it at the capsule bottom makes a swimmer look
            // detached from the model and incorrectly grounded on the water floor.
            *visibility = if swimming {
                Visibility::Hidden
            } else {
                Visibility::Visible
            };
            if !swimming {
                transform.translation.y = shadow_y;
                let normal = terrain.collision_surface_normal(Vec3::new(
                    player_translation.x,
                    0.0,
                    player_translation.z,
                ));
                transform.rotation = Quat::from_rotation_arc(Vec3::Y, normal);
            }
        }
    }
}

pub fn mine_surface_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    inventory: Res<InventoryUiState>,
    farming: Option<Res<FarmingUiState>>,
    codex: Option<Res<CodexSceneInput>>,
    cave: Option<Res<CaveTravelState>>,
    mut nature: ResMut<OfflineNature>,
    mut terrain: ResMut<ProceduralTerrainSurface>,
    mut rebuild: ResMut<TerrainRebuildState>,
    mut underground: ResMut<TerrainUndergroundState>,
    players: Query<&Transform, With<PlayerActor>>,
    cameras: Query<&Transform, With<LivingSceneCamera>>,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    if bindings.menu_open || inventory.open || farming_open {
        return;
    }
    if cave.as_ref().is_some_and(|cave| cave.active) {
        return;
    }
    if !bindings.just_pressed(GameAction::Mine, &keys, &mouse)
        && !codex.as_ref().is_some_and(|input| input.mine)
    {
        return;
    }
    let Ok(player) = players.single() else {
        return;
    };

    let target_local = cameras
        .single()
        .ok()
        .and_then(|camera| surface_ray_hit(camera, &terrain))
        .unwrap_or(player.translation);

    let world_x = super::state::PROCEDURAL_TERRAIN_CENTER + target_local.x.round() as i32;
    let world_z = super::state::PROCEDURAL_TERRAIN_CENTER + target_local.z.round() as i32;
    let Some(surface_y) = top_solid_surface_y(&nature.terrain_world, world_x, world_z) else {
        debug!("[terrain] offline column is already empty");
        return;
    };
    let target_y = surface_y - 1;
    let player_world_x =
        super::state::PROCEDURAL_TERRAIN_CENTER + player.translation.x.floor() as i32;
    let player_world_z =
        super::state::PROCEDURAL_TERRAIN_CENTER + player.translation.z.floor() as i32;
    let player_world_y = top_solid_surface_y(&nature.terrain_world, player_world_x, player_world_z)
        .unwrap_or(target_y + 1);
    let target = [world_x, target_y, world_z];
    if !mine_target_reachable([player_world_x, player_world_y, player_world_z], target) {
        debug!(
            "[terrain] offline mine target out of reach: player=({}, {}, {}), target=({}, {}, {})",
            player_world_x, player_world_y, player_world_z, target[0], target[1], target[2]
        );
        return;
    }
    let target_chunk = TerrainChunkCoord::from_block([world_x, target_y, world_z]);
    // Keep the offline path on the same chunk-backed read/write path as the
    // server. The first edit materializes this chunk; subsequent digs in the
    // same area reuse it instead of rebuilding the 16^3 storage.
    nature.terrain_world.materialize_chunk(target_chunk);
    let OfflineNature {
        terrain_world,
        resources,
        ..
    } = &mut *nature;
    let mined = match lk2_core::world::mine_block(
        terrain_world,
        resources,
        world_x,
        target_y,
        world_z,
        0,
    ) {
        Ok(Some((block, drop))) => {
            info!(
                "[terrain] offline mined {:?} at ({world_x}, {target_y}, {world_z})",
                block
            );
            if let Some((kind, amount)) = drop {
                info!("[terrain] offline drop {:?} x{}", kind, amount);
            }
            true
        }
        Ok(None) => {
            debug!("[terrain] offline target was not mineable");
            false
        }
        Err(error) => {
            warn!("[terrain] offline mining rejected: {error}");
            false
        }
    };
    if !mined {
        return;
    }

    rebuild.last_mined = Some([world_x, target_y, world_z]);
    rebuild.feedback_remaining = 2.5;

    // Project the exposed height after the core edit. A mine below an existing
    // cave/water surface updates the underground mesh without inventing a
    // second surface crater.
    let surface_changed = top_solid_surface_y(&nature.terrain_world, world_x, world_z)
        .map(|surface_y| {
            terrain.dig_at_world_column_to_surface(
                world_x,
                world_z,
                surface_y as f32,
                MINE_SURFACE_DEFORMATION_RADIUS,
            )
        })
        .unwrap_or(false);
    rebuild.requested = true;
    underground.requested = true;
    underground.chunk = Some(target_chunk);
    underground.target = Some([world_x, target_y, world_z]);
    if surface_changed {
        info!("[terrain] offline surface deformation added at column ({world_x}, {world_z})");
    } else {
        info!("[terrain] offline underground edit applied at ({world_x}, {target_y}, {world_z})");
    }
}

fn surface_ray_hit(camera: &Transform, terrain: &ProceduralTerrainSurface) -> Option<Vec3> {
    let origin = camera.translation;
    let direction = (camera.rotation * -Vec3::Z).normalize_or_zero();
    if direction.length_squared() <= f32::EPSILON {
        return None;
    }

    for sample in 1..=64 {
        let point = origin + direction * sample as f32 * 0.25;
        if point.x.abs() > super::state::PROCEDURAL_TERRAIN_RADIUS as f32
            || point.z.abs() > super::state::PROCEDURAL_TERRAIN_RADIUS as f32
        {
            continue;
        }
        let surface_y = terrain.ground_height(Vec3::new(point.x, 0.0, point.z));
        if point.y <= surface_y + 0.25 {
            return Some(Vec3::new(point.x, surface_y, point.z));
        }
    }
    None
}

pub fn recover_players_from_void(
    terrain: Res<ProceduralTerrainSurface>,
    dimension: Option<Res<DimensionTravelState>>,
    cave: Option<Res<CaveTravelState>>,
    mut players: Query<
        (
            &mut Transform,
            &mut LinearVelocity,
            Option<&PlayerMotion>,
            Option<&PlayerSwimState>,
        ),
        With<PlayerActor>,
    >,
) {
    const EMBEDDED_PENETRATION: f32 = 0.35;
    const SURFACE_CLEARANCE: f32 = 0.08;
    const SPAWN_POSITION: Vec3 = Vec3::new(-2.0, 0.0, 11.0);
    let in_starfall = dimension
        .as_ref()
        .is_some_and(|dimension| dimension.current == DimensionId::Starfall);
    let in_cave = cave.as_ref().is_some_and(|cave| cave.active);
    let starfall_floor = terrain.ground_height(Vec3::ZERO) + 0.06;
    let cave_floor = starfall_floor;
    let void_threshold = if in_cave {
        cave_floor - 12.0
    } else if in_starfall {
        starfall_floor - 8.0
    } else {
        -12.0
    };

    for (mut transform, mut velocity, motion, swim) in &mut players {
        let half_height =
            player_physics_half_height(motion.map_or(0.0, |motion| motion.crouch_amount));
        let respawn = Vec3::new(
            if in_cave { 0.0 } else { SPAWN_POSITION.x },
            if in_cave {
                cave_floor + half_height + 0.32
            } else if in_starfall {
                starfall_floor + half_height + 0.4
            } else {
                terrain.ground_height(SPAWN_POSITION) + half_height
            },
            if in_cave {
                -8.25
            } else if in_starfall {
                0.0
            } else {
                SPAWN_POSITION.z
            },
        );
        if transform.translation.y < void_threshold {
            transform.translation = respawn;
            velocity.0 = Vec3::ZERO;
            continue;
        }
        if swim.is_some_and(|swim| swim.active) {
            continue;
        }
        if in_cave {
            continue;
        }

        // The visual terrain and the physics mesh are generated from the same
        // height field, but a player can still become embedded after a steep
        // slope correction or a spawn/collider race. The old void-only guard
        // left such a player permanently trapped because it never reached the
        // void threshold. Lift only when the penetration is substantial so
        // normal walking and jumping corrections are left to Tnua/Avian.
        if !in_cave {
            let ground_y = terrain.ground_height(Vec3::new(
                transform.translation.x,
                0.0,
                transform.translation.z,
            ));
            let minimum_center_y = ground_y + half_height - EMBEDDED_PENETRATION;
            if transform.translation.y < minimum_center_y {
                transform.translation.y = ground_y + half_height + SURFACE_CLEARANCE;
                velocity.0 = Vec3::ZERO;
            }
        }
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
    ride: Res<CartRideState>,
    mut players: Query<
        (
            &LinearVelocity,
            &TnuaController<PlayerControlScheme>,
            &mut Rotation,
            &mut PlayerMotion,
            &mut PlayerJump,
            Option<&PlayerSwimState>,
        ),
        With<PlayerActor>,
    >,
) {
    let Ok((velocity, controller, mut rotation, mut motion, mut jump, swim)) = players.single_mut()
    else {
        return;
    };
    let dt = time.delta_secs().max(0.0001);
    let planar_velocity = if swim.is_some_and(|swim| swim.active) {
        swim.map_or(Vec3::ZERO, |swim| {
            Vec3::new(swim.velocity.x, 0.0, swim.velocity.z)
        })
    } else {
        Vec3::new(velocity.0.x, 0.0, velocity.0.z)
    };
    let grounded =
        swim.is_none_or(|swim| !swim.active) && !controller.is_airborne().unwrap_or(true);
    // A mounted player is still moved by the same physics body, but the body
    // is now the rider seat for presentation purposes. Do not feed the cart's
    // travel speed into the humanoid walking cycle.
    motion.planar_velocity = if ride.mounted {
        Vec3::ZERO
    } else {
        planar_velocity
    };
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
    let gait = (motion.smoothed_speed / PLAYER_SPEED).clamp(0.0, 1.35);
    if gait > 0.02 && grounded {
        motion.stride_phase =
            (motion.stride_phase + dt * (5.5 + gait * 5.0)).rem_euclid(std::f32::consts::TAU);
    } else {
        motion.stride_phase = motion.stride_phase.lerp(0.0, 1.0 - (-dt * 5.0).exp());
    }
    jump.vertical_velocity = if swim.is_some_and(|swim| swim.active) {
        swim.map_or(0.0, |swim| swim.velocity.y)
    } else {
        velocity.0.y
    };
    jump.coyote_timer = if grounded { 0.12 } else { 0.0 };
    jump.jump_buffer_timer = 0.0;
    let direction = planar_velocity.normalize_or_zero();
    if camera_rig.mode == CameraMode::FirstPerson {
        // The held weapon and melee attacks follow the view, even while the player is idle.
        rotation.0 = Quat::from_rotation_y(camera_rig.yaw);
    } else if direction.length_squared() > 0.0 {
        rotation.0 = Quat::from_rotation_y(direction.x.atan2(direction.z));
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
    codex: Option<Res<CodexSceneInput>>,
    loadout: Option<Res<LegendaryLoadout>>,
    terrain: Res<ProceduralTerrainSurface>,
    spatial_query: Option<SpatialQuery>,
    hit_config: CreatureHitConfig,
    cameras: Query<&Transform, (With<LivingSceneCamera>, Without<PlayerActor>)>,
    mut players: Query<
        (
            Entity,
            &mut Transform,
            Option<&mut LinearVelocity>,
            Option<&mut Health>,
            &mut PvpCombatant,
            &SimpleWeapon,
            Option<&mut PlayerSkillState>,
            Option<&PlayerMotion>,
        ),
        (With<PlayerActor>, Without<LivingSceneCamera>),
    >,
    mut target_queries: ParamSet<(
        Query<
            (Entity, &Transform, &Health, Option<&Hitbox>),
            (
                Without<PlayerActor>,
                Without<DefeatedCreature>,
                Without<SlashFx>,
                Without<HitImpactFx>,
                With<HitReaction>,
            ),
        >,
        Query<
            (&mut Health, &mut HitReaction),
            (
                Without<PlayerActor>,
                Without<DefeatedCreature>,
                Without<SlashFx>,
                Without<HitImpactFx>,
                With<HitReaction>,
            ),
        >,
    )>,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    if bindings.menu_open || inventory.open || farming_open {
        return;
    }
    let Ok((
        player_entity,
        mut player,
        linear_velocity,
        player_health,
        mut combatant,
        weapon,
        mut skill_state,
        motion,
    )) = players.single_mut()
    else {
        return;
    };
    let camera_rig = hit_config.camera_rig.as_deref();
    if camera_rig.is_some_and(|camera| camera.mode == CameraMode::GodView) {
        return;
    }
    let hit_settings = hit_config.settings.as_deref().copied().unwrap_or_default();
    let equipped_weapon = loadout.as_ref().and_then(|loadout| loadout.equipped);
    let dragon_equipped = equipped_weapon == Some(LegendaryWeapon::DragonKatana);
    let reaper_equipped = equipped_weapon == Some(LegendaryWeapon::ReaperScythe);
    let skill = bindings.just_pressed(GameAction::WeaponSkill, &keys, &mouse);
    let skill_target = if skill {
        if !dragon_equipped && !reaper_equipped {
            return;
        }
        if dragon_equipped {
            dragon_katana_skill_target(
                &player,
                motion,
                cameras.single().ok(),
                &terrain,
                spatial_query.as_ref(),
                player_entity,
            )
        } else {
            None
        }
    } else {
        None
    };
    let attack_weapon = if skill {
        let Some(ref mut skill_state) = skill_state else {
            return;
        };
        if !skill_state.begin(4.0) {
            return;
        }
        if dragon_equipped {
            let Some((destination, direction)) = skill_target else {
                return;
            };
            player.translation = destination;
            player.rotation = Quat::from_rotation_y(direction.x.atan2(direction.z));
            if let Some(mut velocity) = linear_velocity {
                velocity.0 = Vec3::ZERO;
            }
        }
        let mut weapon = *weapon;
        if dragon_equipped {
            weapon.damage *= 3.0;
            weapon.reach *= 1.35;
            weapon.sweep_angle_deg = 110.0;
        } else {
            weapon.reach *= 1.25;
            weapon.sweep_angle_deg = 100.0;
        }
        weapon
    } else {
        // Holding the attack binding repeats at the weapon cooldown. This
        // matches the expected combat rhythm and prevents a missed click
        // during cooldown from making the player manually re-time every hit.
        if !(bindings.pressed(GameAction::Attack, &keys, &mouse)
            || codex.as_ref().is_some_and(|input| input.attack))
            || !combatant.begin_attack(*weapon)
        {
            return;
        }
        *weapon
    };
    let crouch = motion.map_or(0.0, |motion| motion.crouch_amount);
    let player_position = player.translation - Vec3::Y * player_physics_half_height(crouch);
    // Melee follows the aim camera in both views. Previously third-person
    // attacks fell back to the player's last movement heading, so rotating
    // the camera while standing still made the swing visibly miss its aim.
    let attack_direction = camera_rig.map_or(*player.back(), |rig| yaw_forward(rig.yaw));
    state.attack_flash = if skill { 0.52 } else { 0.32 };
    let best_hit = target_queries
        .p0()
        .iter()
        .filter_map(|(entity, target, health, hitbox)| {
            if health.is_dead() {
                return None;
            }
            let victim_position =
                target.translation + hitbox.map_or(Vec3::ZERO, |hitbox| hitbox.center_offset);
            let attack_weapon = hitbox.map_or(attack_weapon, |hitbox| SimpleWeapon {
                reach: attack_weapon.reach + hitbox.radius,
                ..attack_weapon
            });
            let hit = resolve_melee_attack(
                player_position,
                attack_direction,
                victim_position,
                attack_weapon,
            )?;
            Some((
                entity,
                hit,
                victim_position.distance_squared(player_position),
            ))
        })
        .min_by(|left, right| left.2.total_cmp(&right.2));

    if let Some((target_entity, hit, _)) = best_hit {
        if let Ok((mut health, mut reaction)) = target_queries.p1().get_mut(target_entity) {
            let actual_damage = if skill && reaper_equipped {
                let target_statuses = StatusSet::default();
                let mut wielder_statuses = StatusSet::default();
                if let Some(mut player_health) = player_health {
                    resolve_reaper_strike(
                        &mut player_health,
                        &mut wielder_statuses,
                        &mut health,
                        &target_statuses,
                        state.frame as u32,
                    )
                    .damage_dealt
                } else {
                    let drain = health.current * 0.25;
                    health.damage(
                        drain,
                        state.frame as u32,
                        hit_settings.invulnerability_ticks,
                    )
                }
            } else {
                health.damage(
                    hit.damage,
                    state.frame as u32,
                    hit_settings.invulnerability_ticks,
                )
            };
            if actual_damage > 0.0 {
                let defeated = health.is_dead();
                // ReaperStrike resolves through the shared legendary helper,
                // whose generic path has no creature-specific invulnerability
                // parameter. Apply the same gate after a successful strike so
                // both normal and skill hits share one contract.
                health.invuln_until_tick =
                    (state.frame as u32).saturating_add(hit_settings.invulnerability_ticks);
                reaction.trigger_with_settings(hit.knockback, 0.18, hit_settings);
                state.camera_shake = state.camera_shake.max(0.11);
                commands.spawn((
                    ParticleEffect::new(scene_materials.enemy_hit_effect.clone()),
                    Transform::from_translation(hit.hit_pos + Vec3::Y * 0.55)
                        .with_scale(Vec3::splat(0.72)),
                    HitImpactFx {
                        age: 0.0,
                        lifetime: 0.46,
                        scale: 1.0,
                    },
                    Name::new("hit_impact"),
                ));
                if defeated {
                    commands
                        .entity(target_entity)
                        .insert((DefeatedCreature, Visibility::Hidden))
                        .remove::<Collider>()
                        .remove::<RigidBody>()
                        .remove::<LinearVelocity>()
                        .remove::<Rotation>();
                }
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

const DRAGON_KATANA_SKILL_RANGE: f32 = 12.0;
const DRAGON_KATANA_SKILL_CLEARANCE: f32 = 0.75;

fn dragon_katana_skill_target(
    player: &Transform,
    motion: Option<&PlayerMotion>,
    camera: Option<&Transform>,
    terrain: &ProceduralTerrainSurface,
    spatial_query: Option<&SpatialQuery>,
    player_entity: Entity,
) -> Option<(Vec3, Vec3)> {
    let crouch = motion.map_or(0.0, |motion| motion.crouch_amount);
    let half_height = player_physics_half_height(crouch);
    let visual_position = player.translation - Vec3::Y * half_height;
    let origin = first_person_eye(visual_position);
    let direction = camera.map_or(*player.back(), |camera| camera.forward().as_vec3());
    let direction = Dir3::new(direction).ok()?;
    let filter = SpatialQueryFilter::from_excluded_entities([player_entity]);
    let distance = spatial_query
        .and_then(|query| {
            query
                .cast_ray(origin, direction, DRAGON_KATANA_SKILL_RANGE, true, &filter)
                .map(|hit| (hit.distance - DRAGON_KATANA_SKILL_CLEARANCE).max(0.0))
        })
        .unwrap_or(DRAGON_KATANA_SKILL_RANGE);
    let aimed = origin + direction * distance;
    let ground_position = Vec3::new(aimed.x, 0.0, aimed.z);
    if terrain_cell_is_water(terrain, ground_position) {
        return None;
    }
    Some((
        Vec3::new(
            ground_position.x,
            terrain.ground_height(ground_position) + half_height,
            ground_position.z,
        ),
        direction.as_vec3(),
    ))
}

pub fn pickup_legendary_weapon(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    mut commands: Commands,
    mut loadout: ResMut<LegendaryLoadout>,
    mut players: Query<(&Transform, &mut SimpleWeapon), With<PlayerActor>>,
    dragon_pickups: Query<(Entity, &Transform), With<DragonKatanaPickup>>,
    reaper_pickups: Query<(Entity, &Transform), With<ReaperScythePickup>>,
) {
    if bindings.menu_open || !bindings.just_pressed(GameAction::Interact, &keys, &mouse) {
        return;
    }
    let Ok((player, mut weapon)) = players.single_mut() else {
        return;
    };
    let pickup = dragon_pickups
        .iter()
        .filter_map(|(entity, pickup)| {
            (pickup.translation.distance_squared(player.translation) <= 2.5_f32.powi(2))
                .then_some((entity, LegendaryWeapon::DragonKatana))
        })
        .chain(reaper_pickups.iter().filter_map(|(entity, pickup)| {
            (pickup.translation.distance_squared(player.translation) <= 2.5_f32.powi(2))
                .then_some((entity, LegendaryWeapon::ReaperScythe))
        }))
        .next();
    let Some((entity, weapon_kind)) = pickup else {
        return;
    };
    loadout.grant_and_equip(weapon_kind);
    *weapon = legendary_weapon(weapon_kind);
    commands.entity(entity).despawn();
}

pub fn toggle_cave_teleport(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    terrain: Res<ProceduralTerrainSurface>,
    mut travel: ResMut<CaveTravelState>,
    mut players: Query<
        (&mut Transform, &mut LinearVelocity, Option<&mut PlayerSwimState>),
        (
            With<PlayerActor>,
            Without<CaveEntrance>,
            Without<CaveExit>,
        ),
    >,
    entrances: Query<&Transform, (With<CaveEntrance>, Without<PlayerActor>)>,
    exits: Query<&Transform, (With<CaveExit>, Without<PlayerActor>)>,
) {
    if bindings.menu_open || !bindings.just_pressed(GameAction::Interact, &keys, &mouse) {
        return;
    }
    let Ok((mut player, mut velocity, swim)) = players.single_mut() else {
        return;
    };

    if travel.active {
        let Some(exit) = exits
            .iter()
            .min_by(|left, right| {
                left.translation
                    .distance_squared(player.translation)
                    .total_cmp(&right.translation.distance_squared(player.translation))
            })
        else {
            return;
        };
        if exit.translation.distance_squared(player.translation) > 3.4_f32.powi(2) {
            return;
        }
        if let Some(return_position) = travel.return_position.take() {
            player.translation = return_position;
        }
        if let Some(return_rotation) = travel.return_rotation.take() {
            player.rotation = return_rotation;
        }
        velocity.0 = Vec3::ZERO;
        if let Some(mut swim) = swim {
            swim.active = false;
            swim.velocity = Vec3::ZERO;
        }
        travel.active = false;
        info!("[cave] returned to the cave entrance");
        return;
    }

    let Some(cave) = entrances
        .iter()
        .filter(|cave| cave.translation.distance_squared(player.translation) <= 3.0_f32.powi(2))
        .min_by(|left, right| {
            left.translation
                .distance_squared(player.translation)
                .total_cmp(&right.translation.distance_squared(player.translation))
        })
    else {
        return;
    };

    travel.return_position = Some(player.translation);
    travel.return_rotation = Some(player.rotation);
    let cave_floor = terrain.ground_height(Vec3::ZERO) + 0.06;
    player.translation = Vec3::new(
        0.0,
        cave_floor + super::util::PLAYER_PHYSICS_CENTER_HEIGHT + 0.22,
        -8.25,
    );
    velocity.0 = Vec3::ZERO;
    if let Some(mut swim) = swim {
        swim.active = false;
        swim.velocity = Vec3::ZERO;
    }
    travel.active = true;
    let _ = cave;
    info!("[cave] entered the open-pit mine");
}

fn legendary_weapon(weapon: LegendaryWeapon) -> SimpleWeapon {
    match weapon {
        LegendaryWeapon::DragonKatana => SimpleWeapon {
            reach: 3.6,
            damage: 10.0,
            knockback: 5.5,
            cooldown_secs: 0.5,
            sweep_angle_deg: 75.0,
        },
        LegendaryWeapon::ReaperScythe => SimpleWeapon {
            reach: 4.0,
            damage: 8.0,
            knockback: 4.5,
            cooldown_secs: 0.65,
            sweep_angle_deg: 85.0,
        },
    }
}

pub fn update_camera(
    time: Res<Time>,
    state: Res<LivingSceneState>,
    bindings: Res<KeyBindings>,
    inventory: Res<InventoryUiState>,
    farming: Option<Res<FarmingUiState>>,
    camera_rig: Res<LivingCameraRig>,
    terrain: Res<ProceduralTerrainSurface>,
    config: Option<Res<ProceduralAnimationConfig>>,
    spatial_query: SpatialQuery,
    players: Query<
        (
            Entity,
            &Transform,
            Option<&PlayerMotion>,
            Option<&PlayerSwimState>,
        ),
        (With<PlayerActor>, Without<LivingSceneCamera>),
    >,
    mut cameras: Query<
        (&mut Transform, &mut Projection),
        (With<LivingSceneCamera>, Without<PlayerActor>),
    >,
) {
    let farming_open = farming.as_ref().is_some_and(|state| state.open);
    if bindings.menu_open || inventory.open || farming_open {
        return;
    }
    let Ok((mut camera, mut projection)) = cameras.single_mut() else {
        return;
    };
    let is_god_view = camera_rig.mode == CameraMode::GodView;
    if is_god_view {
        if !matches!(*projection, Projection::Orthographic(_)) {
            *projection = Projection::Orthographic(OrthographicProjection::default_3d());
        }
        if let Projection::Orthographic(orthographic) = &mut *projection {
            // Orthographic cameras do not become closer when their transform
            // moves. Use god_distance as the actual map zoom and keep the
            // orbit radius fixed so the scroll wheel visibly changes framing.
            orthographic.scale = (camera_rig.god_distance / 980.0).clamp(0.07, 0.24);
        }
        let target = camera_rig.god_focus;
        let desired = target
            - camera_direction(camera_rig.god_yaw, camera_rig.god_pitch)
                * 140.0;
        camera.translation = camera
            .translation
            .lerp(desired, 1.0 - (-time.delta_secs() * 5.0).exp());
        camera.look_at(target, Vec3::Y);
        return;
    }
    if !matches!(*projection, Projection::Perspective(_)) {
        *projection = Projection::Perspective(PerspectiveProjection::default());
    }
    if camera_rig.mode == CameraMode::FreeCam {
        let forward = camera_direction(camera_rig.free_yaw, camera_rig.free_pitch);
        camera.translation = camera_rig.free_position;
        let look_target = camera.translation + forward;
        camera.look_at(look_target, Vec3::Y);
        return;
    }
    let Ok((player_entity, player, motion, swim)) = players.single() else {
        return;
    };
    let config = config.map_or_else(ProceduralAnimationConfig::default, |config| *config);
    let crouch = motion.map_or(0.0, |motion| motion.crouch_amount);
    let swimming = swim.is_some_and(|swim| swim.active);
    let feet_position = player.translation - Vec3::Y * player_physics_half_height(crouch);
    let visual_position = if swimming {
        player.translation
    } else {
        feet_position
    };
    let crouch_drop = config.crouch_depth * crouch;
    let shake = camera_shake_offset(state.elapsed, state.camera_shake);
    let water_surface = terrain.water_surface_height();
    match camera_rig.mode {
        CameraMode::FirstPerson => {
            let eye = if swimming {
                // Swimming keeps the physics transform at the capsule center,
                // but an upright walking eye would put the camera at or above
                // the sea plane. Keep the underwater eye near the swimmer's
                // upper body and clamp it below the actual water surface.
                let desired_y = player.translation.y + 0.22;
                Vec3::new(
                    player.translation.x,
                    desired_y.min(water_surface - 0.18),
                    player.translation.z,
                )
            } else {
                first_person_eye(visual_position) - Vec3::Y * crouch_drop * 0.9
            };
            let forward = camera_direction(camera_rig.yaw, camera_rig.pitch);
            camera.translation = eye + forward * 0.08 + shake;
            let target = camera.translation + forward + shake * 0.25;
            camera.look_at(target, Vec3::Y);
        }
        CameraMode::ThirdPerson => {
            // Aim at the body's center rather than above the head. This keeps
            // the feet inside the frame while retaining enough look-down angle
            // to read terrain and nearby interactions.
            let target = if swimming {
                // Keep both the target and the orbit camera inside the water
                // volume. The usual walk orbit sits well above the avatar and
                // would leave a third-person swimmer looking at the shoreline.
                visual_position + Vec3::Y * 0.12
            } else {
                visual_position + Vec3::Y * (0.90 - crouch_drop * 0.65)
            };
            let desired = if swimming {
                let horizontal_forward = yaw_forward(camera_rig.yaw);
                let right = Vec3::new(-horizontal_forward.z, 0.0, horizontal_forward.x);
                let camera_y = target.y.min(water_surface - 0.28);
                Vec3::new(
                    target.x - horizontal_forward.x * 5.1 + right.x * 0.9,
                    camera_y,
                    target.z - horizontal_forward.z * 5.1 + right.z * 0.9,
                )
            } else {
                third_person_camera_position(target, camera_rig.yaw, camera_rig.pitch)
            };
            let desired = camera_collision_position(&spatial_query, player_entity, target, desired);
            camera.translation = camera
                .translation
                .lerp(desired, 1.0 - (-time.delta_secs() * 5.5).exp())
                + shake;
            camera.look_at(target + shake * 0.25, Vec3::Y);
        }
        CameraMode::FreeCam => unreachable!("free camera is handled before player lookup"),
        CameraMode::GodView => unreachable!("god view is handled before player lookup"),
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
    time: Res<Time>,
    state: Res<LivingSceneState>,
    camera_rig: Res<LivingCameraRig>,
    ride: Res<CartRideState>,
    terrain: Res<ProceduralTerrainSurface>,
    config: Option<Res<ProceduralAnimationConfig>>,
    loadout: Option<Res<LegendaryLoadout>>,
    mut players: Query<
        (
            &Transform,
            &mut PlayerMotion,
            &PlayerJump,
            Option<&PlayerSwimState>,
        ),
        With<PlayerActor>,
    >,
    mut queries: ParamSet<(
        Query<(&PlayerIkPart, &mut Transform, Option<&mut Visibility>), Without<PlayerActor>>,
        Query<(&HeldWeaponVisual, &mut Transform, &mut Visibility), Without<PlayerActor>>,
    )>,
) {
    let Ok((player, mut motion, jump, swim)) = players.single_mut() else {
        return;
    };
    let config = config.map_or_else(ProceduralAnimationConfig::default, |config| *config);
    let first_person = camera_rig.mode == CameraMode::FirstPerson;
    let swimming = swim.is_some_and(|swim| swim.active);
    let player_rotation = if first_person {
        Quat::from_rotation_y(camera_rig.yaw)
    } else {
        player.rotation
    };
    let ground_slope = if !swimming && motion.grounded && !ride.mounted {
        // The physics root stays upright because its locked axes are part of
        // the movement contract. The visual root must still use the same
        // height field normal as the collision surface, otherwise the feet
        // solve against a slope while the avatar remains world-vertical.
        let normal =
            terrain.collision_surface_normal(Vec3::new(
                player.translation.x,
                0.0,
                player.translation.z,
            ));
        Quat::from_rotation_arc(Vec3::Y, normal)
    } else {
        Quat::IDENTITY
    };
    let visual_rotation = if swimming {
        // The avatar is authored upright (+Y body axis). Rotate the visual
        // rig onto the swim axis while keeping the physics root upright and
        // authoritative.
        player_rotation * Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)
    } else {
        ground_slope * player_rotation
    };
    let visual_player = Transform {
        translation: if swimming {
            // Center the prone avatar on the physics capsule instead of
            // leaving its pelvis at the capsule center.
            player.translation - visual_rotation * Vec3::new(0.0, 0.70, 0.0)
        } else {
            player.translation - Vec3::Y * player_physics_half_height(motion.crouch_amount)
        },
        rotation: visual_rotation,
        ..*player
    };
    let player = &visual_player;
    if !ride.mounted {
        update_foot_targets(player, &mut motion, &terrain, time.delta_secs(), &config);
    } else {
        motion.foot_targets_initialized = false;
        motion.left_foot_lift = 0.0;
        motion.right_foot_lift = 0.0;
    }
    let equipped_weapon = loadout.as_ref().and_then(|loadout| loadout.equipped);
    let dragon_equipped = equipped_weapon == Some(LegendaryWeapon::DragonKatana);
    let mut pose = procedural_player_pose_with_swim(
        state.elapsed,
        state.attack_flash,
        &motion,
        jump,
        first_person,
        camera_rig.pitch,
        swimming,
        &config,
    );
    if ride.mounted {
        // Seat the rider inside the cart: bent knees forward and hands on the
        // reins/handle area. This keeps movement attached to the cart without
        // showing a full humanoid walk cycle over the vehicle.
        pose.left_hand = Vec3::new(-0.38, 0.68, 0.58);
        pose.right_hand = Vec3::new(0.38, 0.68, 0.58);
        pose.left_foot = Vec3::new(-0.24, 0.07, 0.62);
        pose.right_foot = Vec3::new(0.24, 0.07, 0.62);
        pose.left_knee_pole = Vec3::new(-0.29, 0.34, 0.40);
        pose.right_knee_pole = Vec3::new(0.29, 0.34, 0.40);
        pose.left_boot_pitch = 0.18;
        pose.right_boot_pitch = 0.18;
        pose.body_bob = 0.0;
        pose.torso_pitch = 0.08;
        pose.torso_roll = 0.0;
        pose.crouch = 0.0;
    }
    let (left_foot, left_foot_slope) = if ride.mounted {
        terrain_foot_pose(player, pose.left_foot, &terrain, false)
    } else if motion.grounded {
        let local = player.rotation.inverse() * (motion.left_foot_target - player.translation);
        terrain_foot_pose(player, local, &terrain, true)
    } else {
        terrain_foot_pose(player, pose.left_foot, &terrain, false)
    };
    let (right_foot, right_foot_slope) = if ride.mounted {
        terrain_foot_pose(player, pose.right_foot, &terrain, false)
    } else if motion.grounded {
        let local = player.rotation.inverse() * (motion.right_foot_target - player.translation);
        terrain_foot_pose(player, local, &terrain, true)
    } else {
        terrain_foot_pose(player, pose.right_foot, &terrain, false)
    };
    if motion.grounded && !ride.mounted {
        let left_local = player.rotation.inverse() * (left_foot - player.translation);
        let right_local = player.rotation.inverse() * (right_foot - player.translation);
        // Keep the pole in front of the actual contact, not in front of the
        // old sine-wave target. This is what makes the existing two-bone
        // solver follow the new planted/swinging foot correctly.
        pose.left_knee_pole = Vec3::new(
            -0.29,
            0.28 + motion.left_foot_lift * 0.45,
            left_local.z + 0.20,
        );
        pose.right_knee_pole = Vec3::new(
            0.29,
            0.28 + motion.right_foot_lift * 0.45,
            right_local.z + 0.20,
        );
        pose.left_boot_pitch = -motion.left_foot_lift * 0.58;
        pose.right_boot_pitch = -motion.right_foot_lift * 0.58;
    }
    let body_slope = left_foot_slope
        .slerp(right_foot_slope, 0.5)
        .slerp(Quat::IDENTITY, 0.55);

    let mut rig = HumanoidRig::player_avatar();
    let crouch_drop = config.crouch_depth * pose.crouch;
    rig.left_arm.root.y -= crouch_drop * 0.85;
    rig.right_arm.root.y -= crouch_drop * 0.85;
    rig.left_leg.root.y -= crouch_drop * 0.35;
    rig.right_leg.root.y -= crouch_drop * 0.35;
    rig.left_leg.upper_len = config.ik_leg_upper_length;
    rig.right_leg.upper_len = config.ik_leg_upper_length;
    rig.left_leg.lower_len = config.ik_leg_lower_length;
    rig.right_leg.lower_len = config.ik_leg_lower_length;
    let solved = rig.solve(HumanoidTargets {
        left_hand: pose.left_hand,
        right_hand: pose.right_hand,
        left_foot,
        right_foot,
        left_elbow_pole: pose.left_elbow_pole,
        right_elbow_pole: pose.right_elbow_pole,
        left_knee_pole: pose.left_knee_pole,
        right_knee_pole: pose.right_knee_pole,
    });

    for (part, mut transform, visibility) in &mut queries.p0() {
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
                Vec3::new(0.0, 0.76 + pose.body_bob - crouch_drop * 0.68, 0.0),
                body_slope
                    * Quat::from_rotation_x(pose.torso_pitch)
                    * Quat::from_rotation_z(pose.torso_roll),
                Vec3::new(1.0, 1.0, 0.74),
            ),
            PlayerIkPartKind::Pants => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.0, 0.43 + pose.body_bob * 0.65 - crouch_drop * 0.52, 0.03),
                Quat::IDENTITY,
                Vec3::new(1.0, 0.80, 0.78),
            ),
            PlayerIkPartKind::Neck => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.0, 1.12 - crouch_drop * 0.85, -0.01),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::Head => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.0, 1.37 - crouch_drop, -0.03),
                Quat::from_rotation_x(pose.head_pitch),
                Vec3::new(1.0, 0.92, 0.94),
            ),
            PlayerIkPartKind::Hair => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.0, 1.42 - crouch_drop, -0.11),
                Quat::from_rotation_x(pose.head_pitch),
                Vec3::new(1.04, 0.76, 0.88),
            ),
            PlayerIkPartKind::HairSideL => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(-0.18, 1.35 - crouch_drop, -0.12),
                Quat::from_rotation_x(pose.head_pitch),
                Vec3::new(0.84, 0.82, 0.84),
            ),
            PlayerIkPartKind::HairSideR => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.18, 1.35 - crouch_drop, -0.12),
                Quat::from_rotation_x(pose.head_pitch),
                Vec3::new(0.84, 0.82, 0.84),
            ),
            PlayerIkPartKind::EyeL => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(-0.10, 1.37 - crouch_drop, 0.26),
                Quat::IDENTITY,
                Vec3::ONE,
            ),
            PlayerIkPartKind::EyeR => apply_local_pose(
                &mut transform,
                player,
                Vec3::new(0.10, 1.37 - crouch_drop, 0.26),
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
                solved.left_leg.target + Vec3::new(0.0, -0.02, 0.08),
                left_foot_slope * Quat::from_rotation_x(pose.left_boot_pitch),
                Vec3::new(1.0, 1.0, 0.72),
            ),
            PlayerIkPartKind::BootR => apply_local_pose(
                &mut transform,
                player,
                solved.right_leg.target + Vec3::new(0.0, -0.02, 0.08),
                right_foot_slope * Quat::from_rotation_x(pose.right_boot_pitch),
                Vec3::new(1.0, 1.0, 0.72),
            ),
            PlayerIkPartKind::Stick => {
                let (start, end) = held_stick_segment(&pose);
                apply_local_segment(&mut transform, player, start, end);
            }
        }
    }

    for (visual, mut transform, mut visibility) in &mut queries.p1() {
        if equipped_weapon == Some(visual.weapon) {
            *visibility = Visibility::Visible;
            let (translation, rotation, scale) = match visual.weapon {
                LegendaryWeapon::DragonKatana => {
                    let (translation, rotation) = held_dragon_katana_pose(player, &pose);
                    (translation, rotation, Vec3::splat(0.32))
                }
                LegendaryWeapon::ReaperScythe => {
                    let (translation, rotation) = held_reaper_scythe_pose(player, &pose);
                    (translation, rotation, Vec3::splat(0.34))
                }
            };
            transform.translation = translation;
            transform.rotation = rotation;
            transform.scale = scale;
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
    let ground_world_y = terrain.collision_ground_height(foot_world);
    let ground_local = player.rotation.inverse()
        * (Vec3::new(foot_world.x, ground_world_y, foot_world.z) - player.translation);
    let contact_foot = Vec3::new(foot.x, foot.y.max(ground_local.y + 0.03), foot.z);
    let local_normal = player.rotation.inverse() * terrain.collision_surface_normal(foot_world);
    let slope = Quat::from_rotation_arc(Vec3::Y, local_normal);
    (contact_foot, slope)
}

/// Advance the foot contacts that feed the existing two-bone IK.
///
/// The body/physics transform remains authoritative. This is presentation
/// state only: a planted foot stays in world space until the body has moved
/// far enough, then the alternating foot takes one bounded step to a new
/// ground contact. The IK solver still owns the actual knee and ankle pose.
fn update_foot_targets(
    player: &Transform,
    motion: &mut PlayerMotion,
    terrain: &ProceduralTerrainSurface,
    dt: f32,
    config: &ProceduralAnimationConfig,
) {
    if !motion.grounded {
        motion.foot_targets_initialized = false;
        motion.left_foot_lift = 0.0;
        motion.right_foot_lift = 0.0;
        return;
    }

    let gait = (motion.smoothed_speed / PLAYER_SPEED).clamp(0.0, 1.35);
    let spread = 0.14 + gait * 0.02;
    let movement = motion.planar_velocity.normalize_or_zero();
    let facing = player.rotation * Vec3::Z;
    let forward = if movement.length_squared() > 0.001 {
        movement
    } else {
        Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero()
    };
    let lateral_motion = if movement.length_squared() > 0.001 {
        movement.dot(player.rotation * Vec3::X).abs()
    } else {
        0.0
    };
    let left_anchor = grounded_foot_target(player, Vec3::new(-spread, 0.03, 0.03), terrain);
    let right_anchor = grounded_foot_target(player, Vec3::new(spread, 0.03, 0.03), terrain);

    if !motion.foot_targets_initialized {
        motion.left_foot_target = left_anchor;
        motion.right_foot_target = right_anchor;
        motion.step_start = left_anchor;
        motion.step_goal = left_anchor;
        motion.step_progress = 0.0;
        motion.step_duration = 0.12;
        motion.stepping_left = true;
        motion.foot_targets_initialized = true;
        return;
    }

    if motion.step_progress < 1.0 {
        motion.step_progress =
            (motion.step_progress + dt.max(0.0) / motion.step_duration.max(0.001)).min(1.0);
        let t = smooth01(motion.step_progress);
        let mut contact = motion.step_start.lerp(motion.step_goal, t);
        let ground = terrain.collision_ground_height(Vec3::new(contact.x, 0.0, contact.z));
        let lift =
            (std::f32::consts::PI * t).sin().max(0.0) * config.foot_lift * (0.45 + gait * 0.55);
        contact.y = ground + 0.03 + lift;
        if motion.stepping_left {
            motion.left_foot_target = contact;
            motion.left_foot_lift = lift;
            motion.right_foot_lift = 0.0;
        } else {
            motion.right_foot_target = contact;
            motion.right_foot_lift = lift;
            motion.left_foot_lift = 0.0;
        }
        if motion.step_progress >= 1.0 {
            if motion.stepping_left {
                motion.left_foot_target = motion.step_goal;
            } else {
                motion.right_foot_target = motion.step_goal;
            }
            motion.stepping_left = !motion.stepping_left;
        }
        return;
    }

    motion.left_foot_lift = 0.0;
    motion.right_foot_lift = 0.0;
    if gait <= 0.08 {
        return;
    }

    let (current, anchor) = if motion.stepping_left {
        (motion.left_foot_target, left_anchor)
    } else {
        (motion.right_foot_target, right_anchor)
    };
    let horizontal_error = Vec2::new(current.x - anchor.x, current.z - anchor.z).length();
    let step_threshold = 0.08 + gait * 0.05;
    if horizontal_error <= step_threshold {
        return;
    }

    let mut goal = anchor
        + forward * (config.stride_length * (0.42 + gait * 0.58)) * (1.0 - lateral_motion * 0.30);
    goal.y = terrain.collision_ground_height(Vec3::new(goal.x, 0.0, goal.z)) + 0.03;
    motion.step_start = current;
    motion.step_goal = goal;
    motion.step_progress = 0.0;
    motion.step_duration = (0.16 - gait * 0.05 + lateral_motion * 0.03).clamp(0.08, 0.18);
}

fn grounded_foot_target(
    player: &Transform,
    local: Vec3,
    terrain: &ProceduralTerrainSurface,
) -> Vec3 {
    let world = local_to_world(player, local);
    Vec3::new(
        world.x,
        terrain.collision_ground_height(Vec3::new(world.x, 0.0, world.z)) + local.y,
        world.z,
    )
}

fn yaw_forward(yaw: f32) -> Vec3 {
    Vec3::new(yaw.sin(), 0.0, yaw.cos())
}

pub(crate) fn third_person_camera_position(target: Vec3, yaw: f32, pitch: f32) -> Vec3 {
    let horizontal_forward = yaw_forward(yaw);
    let right = Vec3::new(-horizontal_forward.z, 0.0, horizontal_forward.x);
    let orbit_pitch = (pitch - 0.36).clamp(-1.25, 0.85);
    // Give the toy-like scene enough breathing room for terrain landmarks and
    // foreground foliage to remain visible around the player silhouette.
    // Keep the opening composition focused on the nearby ecology instead of
    // spending most of the frame on empty foreground grass.
    target - camera_direction(yaw, orbit_pitch) * 5.8 + right * 0.9
}

fn camera_direction(yaw: f32, pitch: f32) -> Vec3 {
    let flat = pitch.cos();
    Vec3::new(yaw.sin() * flat, pitch.sin(), yaw.cos() * flat).normalize()
}

fn camera_angles(direction: Vec3) -> (f32, f32) {
    let direction = direction.normalize_or_zero();
    if direction.length_squared() <= f32::EPSILON {
        return (std::f32::consts::PI, 0.0);
    }
    (
        direction.x.atan2(direction.z),
        direction.y.asin().clamp(-1.50, 1.50),
    )
}

fn update_free_camera_position(
    time: &Time,
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    bindings: &KeyBindings,
    camera_rig: &mut LivingCameraRig,
) {
    let forward = yaw_forward(camera_rig.free_yaw);
    let right = Vec3::new(-forward.z, 0.0, forward.x);
    let mut movement = Vec3::ZERO;
    if bindings.pressed(GameAction::MoveForward, keys, mouse) {
        movement += forward;
    }
    if bindings.pressed(GameAction::MoveBackward, keys, mouse) {
        movement -= forward;
    }
    if bindings.pressed(GameAction::MoveRight, keys, mouse) {
        movement += right;
    }
    if bindings.pressed(GameAction::MoveLeft, keys, mouse) {
        movement -= right;
    }
    if bindings.pressed(GameAction::Jump, keys, mouse) {
        movement += Vec3::Y;
    }
    if bindings.pressed(GameAction::Crouch, keys, mouse) {
        movement -= Vec3::Y;
    }
    if movement.length_squared() > 0.0 {
        let speed = if bindings.pressed(GameAction::Sprint, keys, mouse) {
            24.0
        } else {
            8.0
        };
        camera_rig.free_position += movement.normalize() * speed * time.delta_secs();
    }
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

pub(crate) fn held_dragon_katana_pose(
    player: &Transform,
    pose: &ProceduralPlayerPose,
) -> (Vec3, Quat) {
    const KATANA_VISUAL_SCALE: f32 = 0.32;
    const KATANA_GRIP_HEIGHT: f32 = 0.34;

    let hand = pose.left_hand + Vec3::new(-0.02, -0.05, 0.04);
    let idle_direction = Vec3::new(-0.34, 0.78, 0.52);
    let attack_direction = Vec3::new(-0.40, 0.34, 0.85);
    let blade_direction = idle_direction
        .lerp(attack_direction, pose.attack)
        .normalize_or_zero();
    // The GLB is exported with Y-up, so the katana extends along local +Y.
    // Its asset origin is at the pommel/base; move that origin back so the hand
    // sits around the middle of the handle instead of holding the pommel.
    let asset_origin = hand - blade_direction * (KATANA_GRIP_HEIGHT * KATANA_VISUAL_SCALE);
    (
        local_to_world(player, asset_origin),
        player.rotation * Quat::from_rotation_arc(Vec3::Y, blade_direction),
    )
}

pub(crate) fn held_reaper_scythe_pose(
    player: &Transform,
    pose: &ProceduralPlayerPose,
) -> (Vec3, Quat) {
    let grip = pose.left_hand + Vec3::new(-0.02, -0.05, 0.04);
    let idle_direction = Vec3::new(-0.62, 0.52, 0.46);
    let attack_direction = Vec3::new(-0.78, 0.28, 0.58);
    let blade_direction = idle_direction
        .lerp(attack_direction, pose.attack)
        .normalize_or_zero();
    (
        local_to_world(player, grip),
        player.rotation * Quat::from_rotation_arc(Vec3::Z, blade_direction),
    )
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct ProceduralPlayerPose {
    pub gait: f32,
    pub stride_phase: f32,
    pub crouch: f32,
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
    procedural_player_pose_with_config(
        elapsed,
        attack_flash,
        motion,
        jump,
        first_person,
        camera_pitch,
        &ProceduralAnimationConfig::default(),
    )
}

pub fn procedural_player_pose_with_config(
    elapsed: f32,
    attack_flash: f32,
    motion: &PlayerMotion,
    jump: &PlayerJump,
    first_person: bool,
    camera_pitch: f32,
    config: &ProceduralAnimationConfig,
) -> ProceduralPlayerPose {
    procedural_player_pose_with_swim(
        elapsed,
        attack_flash,
        motion,
        jump,
        first_person,
        camera_pitch,
        false,
        config,
    )
}

pub(crate) fn procedural_player_pose_with_swim(
    elapsed: f32,
    attack_flash: f32,
    motion: &PlayerMotion,
    jump: &PlayerJump,
    first_person: bool,
    camera_pitch: f32,
    swimming: bool,
    config: &ProceduralAnimationConfig,
) -> ProceduralPlayerPose {
    if swimming {
        return procedural_swimming_player_pose(
            elapsed,
            attack_flash,
            motion,
            first_person,
            camera_pitch,
        );
    }
    let gait = (motion.smoothed_speed / PLAYER_SPEED).clamp(0.0, 1.35);
    let stride_phase = motion.stride_phase;
    let stride_sin = stride_phase.sin();
    let stride_cos = stride_phase.cos();
    let attack = smooth01(attack_flash / 0.32);
    let airborne = if motion.grounded {
        0.0
    } else {
        (0.35 + motion.airborne_time * 3.2).clamp(0.0, 1.0)
    };
    let crouch = motion.crouch_amount.clamp(0.0, 1.0);
    let jump_up = (jump.vertical_velocity / PLAYER_JUMP_SPEED).clamp(0.0, 1.0);
    let fall = (-jump.vertical_velocity / PLAYER_JUMP_SPEED).clamp(0.0, 1.0);
    let breathe = (elapsed * 2.4).sin();
    let body_bob = breathe * 0.012 * (1.0 - gait)
        + stride_sin.abs() * gait * config.body_bob
        + jump_up * 0.020
        - fall * 0.025;
    let torso_pitch = -gait * config.torso_lean + jump_up * 0.10 - fall * 0.12 - attack * 0.10
        + crouch * config.crouch_lean;
    let torso_roll = stride_sin * gait * 0.055;
    let head_pitch = camera_pitch * 0.25 + jump_up * 0.05 - fall * 0.06 + crouch * 0.08;
    let swing = stride_sin * gait;
    let foot_spread = 0.16 + gait * 0.03;
    let ground_y = 0.06;
    let foot_stride = config.stride_length * gait;
    let left_swing = stride_cos.max(0.0);
    let right_swing = (-stride_cos).max(0.0);
    let left_lift = left_swing * gait * config.foot_lift;
    let right_lift = right_swing * gait * config.foot_lift;
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
        procedural_arm_pose(swing, attack, airborne, first_person, crouch, config);

    ProceduralPlayerPose {
        gait,
        stride_phase,
        crouch,
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
            0.28 + left_lift * 0.45 + airborne * 0.10 - crouch * config.crouch_depth * 0.34,
            0.26 + left_swing * 0.14 + crouch * config.crouch_knee_forward,
        ),
        right_knee_pole: Vec3::new(
            0.29,
            0.28 + right_lift * 0.45 + airborne * 0.10 - crouch * config.crouch_depth * 0.34,
            0.26 + right_swing * 0.14 + crouch * config.crouch_knee_forward,
        ),
        left_boot_pitch: gait * (left_swing * -0.58 + right_swing * 0.16) + airborne * 0.25,
        right_boot_pitch: gait * (right_swing * -0.58 + left_swing * 0.16) + airborne * 0.25,
    }
}

fn procedural_swimming_player_pose(
    elapsed: f32,
    attack_flash: f32,
    motion: &PlayerMotion,
    first_person: bool,
    camera_pitch: f32,
) -> ProceduralPlayerPose {
    let arm_phase = elapsed * 5.2;
    let leg_phase = elapsed * 7.4;
    let left_stroke = arm_phase.sin();
    let right_stroke = -left_stroke;
    let left_kick = leg_phase.sin();
    let right_kick = -left_kick;
    let attack = smooth01(attack_flash / 0.32);
    let swim_speed = (motion.smoothed_speed / SWIM_SPEED).clamp(0.0, 1.0);
    let body_bob = (elapsed * 2.1).sin() * 0.035;
    let torso_pitch = (elapsed * 2.1 + 0.6).sin() * 0.045;
    let torso_roll = (elapsed * 2.7).sin() * 0.05;
    let head_pitch = camera_pitch * 0.12;
    let stroke_reach = if first_person { 0.23 } else { 0.27 };

    // In swim-local space +Y is forward after the visual rig's 90-degree
    // pitch. Each stroke alternates between a long forward reach and a short
    // recovery position; the same targets are consumed by the two-bone arm
    // solver used by the walking pose.
    let left_hand = Vec3::new(
        -0.43,
        1.02 + left_stroke * stroke_reach,
        -0.04 + left_stroke * 0.13,
    );
    let right_hand = Vec3::new(
        0.43,
        1.02 + right_stroke * stroke_reach,
        -0.04 + right_stroke * 0.13,
    );
    let left_foot = Vec3::new(-0.16, 0.16 + left_kick * 0.09, 0.07 + left_kick * 0.05);
    let right_foot = Vec3::new(0.16, 0.16 + right_kick * 0.09, 0.07 + right_kick * 0.05);

    ProceduralPlayerPose {
        gait: swim_speed,
        stride_phase: arm_phase,
        crouch: 0.0,
        attack,
        airborne: 0.0,
        body_bob,
        torso_pitch,
        torso_roll,
        head_pitch,
        left_hand,
        right_hand,
        left_foot,
        right_foot,
        left_elbow_pole: Vec3::new(-0.58, 0.84 + left_stroke * 0.10, 0.28),
        right_elbow_pole: Vec3::new(0.58, 0.84 + right_stroke * 0.10, 0.28),
        left_knee_pole: Vec3::new(-0.29, 0.30 + left_kick * 0.06, 0.24),
        right_knee_pole: Vec3::new(0.29, 0.30 + right_kick * 0.06, 0.24),
        left_boot_pitch: left_kick * 0.20,
        right_boot_pitch: right_kick * 0.20,
    }
}

fn procedural_arm_pose(
    swing: f32,
    attack: f32,
    airborne: f32,
    first_person: bool,
    crouch: f32,
    config: &ProceduralAnimationConfig,
) -> (Vec3, Vec3, Vec3, Vec3) {
    let crouch_drop = crouch * (config.crouch_depth * 0.45 + config.crouch_arm_drop);
    if first_person {
        (
            Vec3::new(
                -0.25,
                1.04 - swing * 0.05 + attack * 0.07 + airborne * 0.05 - crouch_drop,
                0.50 - swing * 0.10 + attack * 0.42 + crouch * 0.10,
            ),
            Vec3::new(
                0.25,
                1.02 + swing * 0.05 + airborne * 0.04 - crouch_drop,
                0.54 + swing * 0.10 + crouch * 0.10,
            ),
            Vec3::new(-0.58, 0.90 - crouch_drop * 0.45, 0.36 + attack * 0.12),
            Vec3::new(0.58, 0.90 - crouch_drop * 0.45, 0.36),
        )
    } else {
        (
            Vec3::new(
                -0.40,
                0.55 - swing * config.arm_swing + attack * 0.18 + airborne * 0.10 - crouch_drop,
                0.16 - swing * 0.20 + attack * 0.38 + crouch * 0.12,
            ),
            Vec3::new(
                0.40,
                0.55 + swing * config.arm_swing + airborne * 0.10 - crouch_drop,
                0.16 + swing * 0.20 + crouch * 0.12,
            ),
            Vec3::new(
                -0.58,
                0.80 + airborne * 0.10 - crouch_drop * 0.45,
                0.32 + attack * 0.10,
            ),
            Vec3::new(0.58, 0.80 + airborne * 0.10 - crouch_drop * 0.45, 0.32),
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
            | PlayerIkPartKind::Hair
            | PlayerIkPartKind::HairSideL
            | PlayerIkPartKind::HairSideR
            | PlayerIkPartKind::EyeL
            | PlayerIkPartKind::EyeR
    )
}

#[cfg(test)]
mod tests {
    use super::swim_rigid_body_for;
    use avian3d::prelude::RigidBody;

    #[test]
    fn swim_rigid_body_switch_is_one_way_per_state_change() {
        assert_eq!(
            swim_rigid_body_for(&RigidBody::Dynamic, true),
            Some(RigidBody::Kinematic)
        );
        assert_eq!(swim_rigid_body_for(&RigidBody::Kinematic, true), None);
        assert_eq!(
            swim_rigid_body_for(&RigidBody::Kinematic, false),
            Some(RigidBody::Dynamic)
        );
        assert_eq!(swim_rigid_body_for(&RigidBody::Dynamic, false), None);
    }
}
