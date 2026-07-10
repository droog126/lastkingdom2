//! Machine-checkable causal assertions; no screenshot inference.

use crate::observer::nature::observe_nature;
use serde_json::Value;

pub fn assert_nature_chain(before: &Value, after: &Value) -> Vec<String> {
    let a = observe_nature(before); let b = observe_nature(after);
    let mut failures = Vec::new();
    if !b.finite_and_bounded() { failures.push("nature.values_finite_and_bounded".into()); }
    if b.tick <= a.tick { failures.push("nature.tick_advances".into()); }
    if b.clouds > 0 && b.rainfall <= 0.0 { failures.push("nature.clouds_produce_rain".into()); }
    if b.rainfall > 0.0 && b.soil_moisture < a.soil_moisture { failures.push("nature.rain_increases_soil_moisture".into()); }
    if b.soil_moisture > a.soil_moisture && b.plants < a.plants { failures.push("nature.moisture_supports_plants".into()); }
    failures
}
