//! Runner contract for a future loop integration; process policy stays here.

use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NatureRunSpec { pub scenario: String, pub timeout: Duration, pub offline: bool }

impl NatureRunSpec {
    pub fn validate(&self) -> Result<(), String> {
        if self.scenario.trim().is_empty() { return Err("scenario must not be empty".into()); }
        if self.timeout.is_zero() { return Err("timeout must be positive".into()); }
        Ok(())
    }
}
