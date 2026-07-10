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

#[derive(Clone, Debug, PartialEq)]
pub struct AuthorityOutput<S, E> {
    pub tick: AuthorityTick,
    pub snapshot: S,
    pub events: Vec<E>,
}

pub struct AuthorityDriver<W> {
    world: W,
    last_tick: Option<u64>,
}

impl<W> AuthorityDriver<W> {
    pub fn new(world: W) -> Self {
        Self { world, last_tick: None }
    }

    pub fn world(&self) -> &W {
        &self.world
    }
}

impl<W: WorldStepper> AuthorityDriver<W> {
    pub fn advance(
        &mut self,
        input: AuthorityInput,
    ) -> Result<AuthorityOutput<W::Snapshot, W::Event>, &'static str> {
        let input = validate_input(input)?;
        if self.last_tick.is_some_and(|last| input.tick <= last) {
            return Err("authority tick must increase monotonically");
        }

        let (snapshot, events) = self.world.step(input);
        self.last_tick = Some(input.tick);
        Ok(AuthorityOutput {
            tick: AuthorityTick { tick: input.tick, accepted: true },
            snapshot,
            events,
        })
    }
}
