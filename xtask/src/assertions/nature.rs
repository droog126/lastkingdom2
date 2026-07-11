//! Machine assertions for deterministic natural-world evidence.

use serde_json::{Value, json};

use crate::artifacts::nature::{AssertionSeverity, NatureAssertion, NatureObservation};

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct NatureExpectations {
    #[serde(default = "yes")]
    pub require_determinism: bool,
    #[serde(default = "yes")]
    pub require_causal_progress: bool,
    #[serde(default = "yes")]
    pub require_presentation_consistency: bool,
    #[serde(default)]
    pub maximum_error_lines: usize,
}

impl Default for NatureExpectations {
    fn default() -> Self {
        Self {
            require_determinism: true,
            require_causal_progress: true,
            require_presentation_consistency: true,
            maximum_error_lines: 0,
        }
    }
}

const fn yes() -> bool {
    true
}

#[must_use]
pub fn evaluate_nature(
    before: Option<&NatureObservation>,
    after: &NatureObservation,
    repeat: Option<&NatureObservation>,
    expectations: &NatureExpectations,
) -> Vec<NatureAssertion> {
    let mut out = vec![check(
        "nature.summary_exists",
        after.metrics.required_fields_present(),
        AssertionSeverity::Fail,
        json!(after.missing_fields),
        "==",
        json!([]),
        "authoritative nature summary is incomplete",
        Some("nature"),
    )];

    out.push(check(
        "nature.values_finite_and_non_negative",
        after.metrics.numeric_values_are_valid(),
        AssertionSeverity::Fail,
        serde_json::to_value(&after.metrics).unwrap_or(Value::Null),
        "all",
        json!("finite and non-negative"),
        "natural state contains NaN, infinity, or a negative inventory",
        Some("nature"),
    ));
    out.push(check(
        "nature.snapshot_is_fresh",
        !after.stale,
        AssertionSeverity::Fail,
        json!(after.metrics.tick),
        ">",
        json!(before.and_then(|item| item.metrics.tick)),
        "natural summary did not advance",
        Some("nature.tick"),
    ));

    if expectations.require_determinism {
        out.push(check(
            "nature.deterministic_repeat_matches",
            repeat.is_some_and(|item| item.fingerprint == after.fingerprint),
            AssertionSeverity::Fail,
            json!(repeat.map(|item| &item.fingerprint)),
            "==",
            json!(after.fingerprint),
            "same seed, initial state, inputs, and tick count produced a different summary",
            Some("nature.fingerprint"),
        ));
    }

    if expectations.require_causal_progress {
        append_causal_assertions(&mut out, before, after);
    }
    if expectations.require_presentation_consistency {
        append_presentation_assertions(&mut out, after);
    }
    out.push(check(
        "nature.error_log_budget",
        after.errors.len() <= expectations.maximum_error_lines,
        AssertionSeverity::Fail,
        json!(after.errors.len()),
        "<=",
        json!(expectations.maximum_error_lines),
        "runtime error log budget exceeded",
        Some("error_logs"),
    ));
    out
}

fn append_causal_assertions(
    out: &mut Vec<NatureAssertion>,
    before: Option<&NatureObservation>,
    after: &NatureObservation,
) {
    let previous = before.map(|item| &item.metrics);
    let metrics = &after.metrics;
    out.push(check(
        "nature.clouds_produce_rain",
        metrics.clouds.is_some_and(|value| value > 0)
            && metrics.rainfall.is_some_and(|value| value > 0.0),
        AssertionSeverity::Fail,
        json!({"clouds": metrics.clouds, "rainfall": metrics.rainfall}),
        ">",
        json!(0),
        "cloud evidence did not lead to rainfall",
        Some("nature.clouds|nature.rainfall"),
    ));
    out.push(check(
        "nature.rain_increases_soil_moisture",
        progressed_f64(
            previous.and_then(|item| item.soil_moisture),
            metrics.soil_moisture,
        ),
        AssertionSeverity::Fail,
        json!(metrics.soil_moisture),
        ">",
        json!(previous.and_then(|item| item.soil_moisture)),
        "rainfall did not increase soil moisture",
        Some("nature.soil_moisture"),
    ));
    out.push(check(
        "nature.moisture_advances_plants",
        progressed_u64(previous.and_then(|item| item.plants), metrics.plants),
        AssertionSeverity::Partial,
        json!(metrics.plants),
        ">",
        json!(previous.and_then(|item| item.plants)),
        "higher soil moisture did not advance plant state",
        Some("nature.plants"),
    ));
    out.push(check(
        "nature.plants_feed_animals",
        metrics
            .animal_food_available
            .is_some_and(|value| value > 0.0)
            && (progressed_u64(previous.and_then(|item| item.animals), metrics.animals)
                || metrics.animals.is_some_and(|value| value > 0)),
        AssertionSeverity::Partial,
        json!({
            "food": metrics.animal_food_available,
            "animals": metrics.animals
        }),
        ">",
        json!(0),
        "plant state did not enter the animal food/behavior summary",
        Some("nature.animal_food_available|nature.animals"),
    ));
}

fn append_presentation_assertions(out: &mut Vec<NatureAssertion>, after: &NatureObservation) {
    for (name, presented, authoritative) in [
        (
            "clouds",
            after.metrics.presented_clouds,
            after.metrics.clouds,
        ),
        (
            "plants",
            after.metrics.presented_plants,
            after.metrics.plants,
        ),
        (
            "animals",
            after.metrics.presented_animals,
            after.metrics.animals,
        ),
    ] {
        let present = presented.is_some();
        out.push(check(
            &format!("nature.presentation.{name}_summary_present"),
            present,
            AssertionSeverity::Partial,
            json!(presented),
            "present",
            json!(true),
            "client presentation count is missing",
            Some(&format!("presentation.nature.{name}")),
        ));
        if present {
            out.push(check(
                &format!("nature.presentation.no_ghost_{name}"),
                presented <= authoritative,
                AssertionSeverity::Fail,
                json!(presented),
                "<=",
                json!(authoritative),
                "client created semantic entities absent from the authoritative snapshot",
                Some(&format!("presentation.nature.{name}")),
            ));
        }
    }
}

fn progressed_f64(before: Option<f64>, after: Option<f64>) -> bool {
    matches!((before, after), (Some(before), Some(after)) if after > before)
}

fn progressed_u64(before: Option<u64>, after: Option<u64>) -> bool {
    matches!((before, after), (Some(before), Some(after)) if after > before)
}

fn check(
    id: &str,
    ok: bool,
    severity: AssertionSeverity,
    actual: Value,
    op: &str,
    expected: Value,
    message: &str,
    path: Option<&str>,
) -> NatureAssertion {
    NatureAssertion {
        id: id.to_owned(),
        ok,
        severity,
        actual,
        op: op.to_owned(),
        expected,
        message: message.to_owned(),
        path: path.map(str::to_owned),
    }
}

#[cfg(test)]
mod tests {
    use crate::observer::nature::observe_value;
    use serde_json::json;

    use super::*;

    fn state(tick: u64, moisture: f64, plants: u64, animals: u64) -> Value {
        json!({
            "nature": {
                "tick": tick,
                "cloud_count": 1,
                "rainfall": 0.5,
                "soil_moisture": moisture,
                "plant_count": plants,
                "animal_count": animals,
                "animal_food_available": 2.0
            },
            "presentation": {"nature": {"clouds": 1, "plants": plants, "animals": animals}}
        })
    }

    #[test]
    fn complete_chain_and_repeat_pass() {
        let before = observe_value(&state(1, 0.1, 1, 0), "before", Vec::new());
        let mut after = observe_value(&state(2, 0.4, 2, 1), "after", Vec::new());
        crate::observer::nature::mark_stale_against(&before, &mut after);
        let repeat = after.clone();
        let assertions = evaluate_nature(
            Some(&before),
            &after,
            Some(&repeat),
            &NatureExpectations::default(),
        );
        assert!(assertions.iter().all(|item| item.ok));
    }

    #[test]
    fn ghosts_and_missing_repeat_are_hard_failures() {
        let before = observe_value(&state(1, 0.1, 1, 0), "before", Vec::new());
        let mut value = state(2, 0.4, 2, 1);
        value["presentation"]["nature"]["animals"] = json!(4);
        let after = observe_value(&value, "after", Vec::new());
        let assertions =
            evaluate_nature(Some(&before), &after, None, &NatureExpectations::default());
        assert!(
            assertions
                .iter()
                .any(|item| { item.id == "nature.presentation.no_ghost_animals" && !item.ok })
        );
        assert!(
            assertions
                .iter()
                .any(|item| { item.id == "nature.deterministic_repeat_matches" && !item.ok })
        );
    }
}
