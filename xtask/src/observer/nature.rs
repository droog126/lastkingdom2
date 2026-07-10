//! Read-only normalization of authoritative natural-world state.

use serde_json::Value;
use crate::artifacts::nature::{NatureObservation, NATURE_ARTIFACT_SCHEMA};

fn u64_at(v: &Value, path: &str) -> u64 { v.pointer(path).and_then(Value::as_u64).unwrap_or(0) }
fn f64_at(v: &Value, path: &str) -> f64 { v.pointer(path).and_then(Value::as_f64).unwrap_or(0.0) }

pub fn observe_nature(state: &Value) -> NatureObservation {
    NatureObservation {
        schema: NATURE_ARTIFACT_SCHEMA,
        tick: u64_at(state, "/tick"),
        clouds: u64_at(state, "/nature/clouds").max(u64_at(state, "/eco_cycle/clouds")),
        rainfall: f64_at(state, "/nature/rainfall").max(f64_at(state, "/eco_cycle/rainfall")),
        soil_moisture: f64_at(state, "/nature/soil_moisture"),
        plants: u64_at(state, "/nature/plants").max(u64_at(state, "/eco_cycle/plants_grown")),
        animals: u64_at(state, "/nature/animals").max(u64_at(state, "/eco_cycle/wildlife")),
        events: state.pointer("/nature/events").and_then(Value::as_array)
            .into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect(),
    }
}
