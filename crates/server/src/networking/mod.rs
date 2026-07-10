//! Translation boundary between wire messages and validated authority input.

use super::authority::{AuthorityInput, validate_input};

pub fn decode_authority_input(tick: u64, rainfall: f32) -> Result<AuthorityInput, &'static str> {
    validate_input(AuthorityInput { tick, rainfall })
}
