//! Server-owned adapter around the shared deterministic world step.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AuthorityInput {
    pub tick: u64,
    pub rainfall: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AuthorityTick {
    pub tick: u64,
    pub accepted: bool,
}

pub fn validate_input(input: AuthorityInput) -> Result<AuthorityInput, &'static str> {
    if !input.rainfall.is_finite() || input.rainfall < 0.0 {
        return Err("rainfall must be finite and non-negative");
    }
    Ok(AuthorityInput { rainfall: input.rainfall.min(1.0), ..input })
}

/// The concrete shared-core call is intentionally injected by the integrator after Gate 0.
pub trait WorldStepper {
    type Snapshot;
    type Event;
    fn step(&mut self, input: AuthorityInput) -> (Self::Snapshot, Vec<Self::Event>);
}

