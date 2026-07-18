//! Shared, deterministic arcade vehicle movement.

use bevy::prelude::Vec2;

/// Forward speed matches the old mounted-cart top speed (`4.5 * 2.0`) while
/// allowing it to be reached through engine acceleration instead of an
/// instantaneous movement multiplier.
pub const CART_MAX_FORWARD_SPEED: f32 = 9.0;
pub const CART_MAX_REVERSE_SPEED: f32 = 3.5;
pub const CART_ENGINE_ACCELERATION: f32 = 16.0;
pub const CART_COAST_DECELERATION: f32 = 4.5;
pub const CART_BRAKE_DECELERATION: f32 = 22.0;
pub const CART_STEER_RESPONSE: f32 = 7.5;
pub const CART_TURN_RADIUS: f32 = 4.8;

/// Continuous vehicle controls. Values are clamped by [`step_cart_drive`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CartDriveInput {
    pub throttle: f32,
    pub steering: f32,
}

/// The small deterministic state needed to make a cart feel like a vehicle.
/// Position and terrain collision remain owned by the caller.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CartDriveState {
    pub speed: f32,
    pub steer: f32,
    pub yaw: f32,
}

/// Converts the player's screen-relative right input to the shared steering
/// convention. Positive yaw turns from +Z toward +X, which is the player's
/// left side in the camera/world convention used by movement controls.
#[must_use]
pub fn cart_steering_from_right_input(input_right: f32) -> f32 {
    -input_right.clamp(-1.0, 1.0)
}

/// Advances a cart and returns its world-space XZ displacement for this step.
///
/// This deliberately borrows the useful part of STK's control model—engine
/// force, coasting/braking, steering slew, and a speed-dependent turn radius—
/// without introducing rigid-body physics into the voxel world's authoritative
/// grid collision path.
pub fn step_cart_drive(state: &mut CartDriveState, input: CartDriveInput, dt: f32) -> Vec2 {
    let dt = dt.clamp(0.0, 0.25);
    let throttle = input.throttle.clamp(-1.0, 1.0);
    let steering = input.steering.clamp(-1.0, 1.0);
    let target_speed = if throttle > 0.0 {
        throttle * CART_MAX_FORWARD_SPEED
    } else if throttle < 0.0 {
        throttle * CART_MAX_REVERSE_SPEED
    } else {
        0.0
    };

    let accelerating_against_motion = state.speed.abs() > 0.01
        && target_speed.abs() > 0.01
        && state.speed.signum() != target_speed.signum();
    let acceleration = if accelerating_against_motion {
        CART_BRAKE_DECELERATION
    } else if target_speed.abs() <= 0.01 {
        CART_COAST_DECELERATION
    } else {
        CART_ENGINE_ACCELERATION
    };
    state.speed = move_towards(state.speed, target_speed, acceleration * dt);
    if state.speed.abs() < 0.01 && target_speed.abs() <= 0.01 {
        state.speed = 0.0;
    }

    let steer_delta = CART_STEER_RESPONSE * dt;
    state.steer = move_towards(state.steer, steering, steer_delta);

    let speed_fraction = (state.speed.abs() / CART_MAX_FORWARD_SPEED).clamp(0.0, 1.0);
    let turn_rate = state.speed / CART_TURN_RADIUS * (0.70 + speed_fraction * 0.30);
    state.yaw += state.steer * turn_rate * dt;

    Vec2::new(state.yaw.sin(), state.yaw.cos()) * state.speed * dt
}

fn move_towards(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttle_accelerates_smoothly_and_stays_bounded() {
        let mut state = CartDriveState::default();
        let first_step = step_cart_drive(
            &mut state,
            CartDriveInput {
                throttle: 1.0,
                steering: 0.0,
            },
            1.0 / 60.0,
        );

        assert!(state.speed > 0.0);
        assert!(state.speed < CART_MAX_FORWARD_SPEED);
        assert!(first_step.length() > 0.0);

        for _ in 0..240 {
            step_cart_drive(
                &mut state,
                CartDriveInput {
                    throttle: 1.0,
                    steering: 0.0,
                },
                1.0 / 60.0,
            );
        }
        assert!((state.speed - CART_MAX_FORWARD_SPEED).abs() < 0.01);
    }

    #[test]
    fn releasing_throttle_coasts_then_stops() {
        let mut state = CartDriveState {
            speed: CART_MAX_FORWARD_SPEED,
            ..Default::default()
        };

        step_cart_drive(
            &mut state,
            CartDriveInput {
                throttle: 0.0,
                steering: 0.0,
            },
            1.0 / 60.0,
        );
        assert!(state.speed > 0.0);
        assert!(state.speed < CART_MAX_FORWARD_SPEED);

        for _ in 0..240 {
            step_cart_drive(&mut state, CartDriveInput::default(), 1.0 / 60.0);
        }
        assert_eq!(state.speed, 0.0);
    }

    #[test]
    fn steering_is_smoothed_and_turns_less_at_low_speed() {
        let mut stopped = CartDriveState::default();
        step_cart_drive(
            &mut stopped,
            CartDriveInput {
                throttle: 0.0,
                steering: 1.0,
            },
            1.0 / 60.0,
        );
        assert_eq!(stopped.yaw, 0.0);
        assert!(stopped.steer > 0.0 && stopped.steer < 1.0);

        let mut moving = CartDriveState {
            speed: CART_MAX_FORWARD_SPEED,
            ..Default::default()
        };
        step_cart_drive(
            &mut moving,
            CartDriveInput {
                throttle: 0.0,
                steering: 1.0,
            },
            1.0 / 60.0,
        );
        assert!(moving.yaw > 0.0);
        assert!(moving.steer < 1.0);
    }

    #[test]
    fn reversing_reverses_turn_direction() {
        let mut state = CartDriveState {
            speed: -CART_MAX_REVERSE_SPEED,
            ..Default::default()
        };
        step_cart_drive(
            &mut state,
            CartDriveInput {
                throttle: 0.0,
                steering: 1.0,
            },
            1.0 / 60.0,
        );
        assert!(state.yaw < 0.0);
    }

    #[test]
    fn right_input_produces_a_right_turn_from_forward_heading() {
        let mut state = CartDriveState {
            speed: CART_MAX_FORWARD_SPEED,
            ..Default::default()
        };
        step_cart_drive(
            &mut state,
            CartDriveInput {
                throttle: 1.0,
                steering: cart_steering_from_right_input(1.0),
            },
            1.0 / 60.0,
        );

        // yaw 0 faces +Z; a right turn moves toward -X, hence yaw decreases.
        assert!(state.yaw < 0.0);
    }
}
