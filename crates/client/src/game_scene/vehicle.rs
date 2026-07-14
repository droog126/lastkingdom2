//! Lightweight local vehicle interaction for the playable scene.

use bevy::prelude::*;

use super::keybindings::{GameAction, KeyBindings};
use super::state::{PlayerActor, ProceduralTerrainSurface};
use super::util::{CART_PATH, spawn_asset};

#[derive(Component)]
pub struct PlayCart;

#[derive(Resource, Default)]
pub struct CartRideState {
    pub mounted: bool,
}

pub fn setup_cart(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    terrain: Res<ProceduralTerrainSurface>,
) {
    let position = Vec3::new(-2.0, terrain.ground_height(Vec3::new(-2.0, 0.0, 8.0)), 8.0);
    spawn_asset(
        &mut commands,
        &asset_server,
        CART_PATH,
        position,
        1.2,
        std::f32::consts::PI,
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
    let cart_translation = {
        let cart_query = actors.p1();
        let Ok(cart) = cart_query.single() else {
            return;
        };
        cart.translation
    };
    let mut player_query = actors.p0();
    let Ok(mut player) = player_query.single_mut() else {
        return;
    };
    if state.mounted || player.translation.distance(cart_translation) < 3.5 {
        state.mounted = !state.mounted;
        if state.mounted {
            player.translation = cart_translation + Vec3::Y * 1.15;
        } else {
            player.translation = cart_translation + Vec3::new(1.8, 0.9, 0.0);
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
    let planar_delta = Vec3::new(
        player.translation.x - cart.translation.x,
        0.0,
        player.translation.z - cart.translation.z,
    );
    let speed = planar_delta.length() / time.delta_secs().max(0.001);
    let moving = speed > 0.05;
    let phase = time.elapsed_secs() * (3.0 + speed * 1.5);
    let bob = if moving { phase.sin().abs() * 0.025 } else { 0.0 };
    let roll = if moving { phase.sin() * 0.025 } else { 0.0 };
    cart.translation = player.translation - Vec3::Y * 1.15 + Vec3::Y * bob;
    cart.rotation = player.rotation * Quat::from_rotation_z(roll);
}
