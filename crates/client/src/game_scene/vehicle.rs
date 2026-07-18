//! Lightweight local vehicle interaction for the playable scene.

use bevy::prelude::*;

use super::keybindings::{GameAction, KeyBindings};
use super::state::{CameraMode, LivingCameraRig, PlayerActor, ProceduralTerrainSurface};
use super::util::{CART_FRONT_YAW_OFFSET, CART_PATH, spawn_asset};
use lk2_core::vehicle::CartDriveState;

#[derive(Component)]
pub struct PlayCart;

#[derive(Resource, Default)]
pub struct CartRideState {
    pub mounted: bool,
    pub drive: CartDriveState,
}

pub fn setup_cart(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    terrain: Res<ProceduralTerrainSurface>,
) {
    let position = Vec3::new(-6.0, terrain.ground_height(Vec3::new(-6.0, 0.0, 10.0)), 10.0);
    spawn_asset(
        &mut commands,
        &asset_server,
        CART_PATH,
        position,
        0.72,
        std::f32::consts::PI + CART_FRONT_YAW_OFFSET,
        "play_cart",
    )
    .insert(PlayCart);
}

pub fn toggle_cart_ride(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    bindings: Res<KeyBindings>,
    mut state: ResMut<CartRideState>,
    mut actors: ParamSet<(
        Query<&mut Transform, (With<PlayerActor>, Without<PlayCart>)>,
        Query<&Transform, (With<PlayCart>, Without<PlayerActor>)>,
    )>,
) {
    if bindings.menu_open || !bindings.just_pressed(GameAction::RideCart, &keys, &mouse) {
        return;
    }
    let (cart_translation, cart_rotation) = {
        let cart_query = actors.p1();
        let Ok(cart) = cart_query.single() else {
            return;
        };
        (cart.translation, cart.rotation)
    };
    let mut player_query = actors.p0();
    let Ok(mut player) = player_query.single_mut() else {
        return;
    };
    if state.mounted || player.translation.distance(cart_translation) < 3.5 {
        state.mounted = !state.mounted;
        if state.mounted {
            player.translation = cart_translation + Vec3::Y * 1.15;
            // The cart mesh faces local +X; the avatar faces local +Z.
            player.rotation = cart_rotation * Quat::from_rotation_y(-CART_FRONT_YAW_OFFSET);
            state.drive = CartDriveState {
                yaw: player.rotation.to_euler(EulerRot::YXZ).0,
                ..default()
            };
        } else {
            player.translation = cart_translation + cart_rotation * Vec3::X * 1.8 + Vec3::Y * 0.9;
            state.drive.speed = 0.0;
            state.drive.steer = 0.0;
        }
    }
}

pub fn sync_cart_to_rider(
    time: Res<Time>,
    state: Res<CartRideState>,
    player: Query<&Transform, (With<PlayerActor>, Without<PlayCart>)>,
    mut cart: Query<&mut Transform, (With<PlayCart>, Without<PlayerActor>)>,
) {
    if !state.mounted {
        return;
    }
    let (Ok(player), Ok(mut cart)) = (player.single(), cart.single_mut()) else {
        return;
    };
    let speed = state.drive.speed.abs();
    let moving = speed > 0.05;
    let phase = time.elapsed_secs() * (3.0 + speed * 1.5);
    let bob = if moving {
        phase.sin().abs() * 0.025
    } else {
        0.0
    };
    let roll = if moving { phase.sin() * 0.025 } else { 0.0 };
    cart.translation = player.translation - Vec3::Y * 1.15 + Vec3::Y * bob;
    cart.rotation = Quat::from_rotation_y(state.drive.yaw + CART_FRONT_YAW_OFFSET)
        * Quat::from_rotation_z(roll);
}

/// The first-person camera starts close to the rider and can look through the
/// cart body. Hide only the presentation root in first-person mode; the cart
/// remains present for physics, interaction, third-person, and free-camera views.
pub fn sync_cart_visibility(
    camera_rig: Res<LivingCameraRig>,
    mut carts: Query<&mut Visibility, With<PlayCart>>,
) {
    let visible = camera_rig.mode != CameraMode::FirstPerson;
    for mut visibility in &mut carts {
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
