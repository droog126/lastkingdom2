//! Translation boundary between wire messages and validated authority input.

use lk2_core::simulation::WorldInput;

pub fn decode_world_input(
    tick: u64,
    last_accepted_tick: Option<u64>,
) -> Result<WorldInput, &'static str> {
    if last_accepted_tick.is_some_and(|last| tick <= last) {
        return Err("authority tick must increase monotonically");
    }
    Ok(WorldInput { tick })
}
