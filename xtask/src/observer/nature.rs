//! Read-only normalization of authoritative natural-world state and error logs.

use std::{fs, path::Path};

use serde_json::Value;

use crate::artifacts::nature::{
    NATURE_ARTIFACT_SCHEMA, NatureMetrics, NatureObservation, ObservedLogError,
};

const REQUIRED_FIELDS: &[&str] = &[
    "tick",
    "clouds",
    "rainfall",
    "soil_moisture",
    "plants",
    "animals",
    "animal_food_available",
];

pub fn observe_file(path: &Path, log_paths: &[&Path]) -> Result<NatureObservation, String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    let source = path.file_name().and_then(|name| name.to_str()).unwrap_or("state");
    Ok(observe_value(&value, source, scan_error_logs(log_paths)))
}

#[must_use]
pub fn observe_value(
    state: &Value,
    source: impl Into<String>,
    errors: Vec<ObservedLogError>,
) -> NatureObservation {
    let metrics = NatureMetrics {
        tick: first_u64(state, &["/nature/tick", "/nature_snapshot/tick", "/tick"]),
        clouds: first_count(
            state,
            &[
                "/nature/cloud_count",
                "/nature/clouds",
                "/nature_snapshot/clouds",
                "/eco_cycle/clouds",
                "/cloud_count",
                "/clouds",
            ],
        ),
        rainfall: first_f64(
            state,
            &[
                "/nature/rainfall",
                "/nature_snapshot/rainfall",
                "/eco_cycle/rainfall",
                "/rainfall",
            ],
        ),
        soil_moisture: first_f64(
            state,
            &[
                "/nature/soil_moisture",
                "/nature_snapshot/soil_moisture",
                "/hydrology/soil_moisture",
                "/soil_moisture",
            ],
        ),
        plants: first_count(
            state,
            &[
                "/nature/plant_count",
                "/nature/plants",
                "/nature_snapshot/plants",
                "/eco_cycle/plants",
                "/eco_cycle/plants_grown",
                "/plant_count",
                "/plants",
            ],
        ),
        animals: first_count(
            state,
            &[
                "/nature/animal_count",
                "/nature/animals",
                "/nature_snapshot/animals",
                "/eco_cycle/animals",
                "/eco_cycle/wildlife",
                "/animal_count",
                "/animals",
            ],
        ),
        animal_food_available: first_f64(
            state,
            &[
                "/nature/animal_food_available",
                "/nature_snapshot/animal_food_available",
                "/ecology/animal_food_available",
                "/animal_food_available",
            ],
        ),
        presented_clouds: first_count(state, &["/presentation/nature/clouds"]),
        presented_plants: first_count(state, &["/presentation/nature/plants"]),
        presented_animals: first_count(state, &["/presentation/nature/animals"]),
    };
    let mut missing_fields = Vec::new();
    let present = [
        metrics.tick.is_some(),
        metrics.clouds.is_some(),
        metrics.rainfall.is_some(),
        metrics.soil_moisture.is_some(),
        metrics.plants.is_some(),
        metrics.animals.is_some(),
        metrics.animal_food_available.is_some(),
    ];
    for (name, is_present) in REQUIRED_FIELDS.iter().zip(present) {
        if !is_present {
            missing_fields.push((*name).to_owned());
        }
    }
    let events = first_value(state, &["/nature/events", "/nature_snapshot/events"])
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(event_label)
        .collect::<Vec<_>>();
    let fingerprint = fingerprint(&metrics, &events);
    let mut observation = NatureObservation {
        schema: NATURE_ARTIFACT_SCHEMA,
        source: source.into(),
        metrics,
        events,
        missing_fields,
        stale: false,
        fingerprint,
        errors,
    };
    observation.normalize();
    observation
}

pub fn mark_stale_against(previous: &NatureObservation, current: &mut NatureObservation) {
    current.stale = match (previous.metrics.tick, current.metrics.tick) {
        (Some(before), Some(after)) => after <= before,
        _ => true,
    };
}

#[must_use]
pub fn scan_error_logs(paths: &[&Path]) -> Vec<ObservedLogError> {
    let mut errors = Vec::new();
    for path in paths {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let source = path.file_name().and_then(|name| name.to_str()).unwrap_or("log");
        for (index, line) in text.lines().enumerate() {
            let lower = line.to_ascii_lowercase();
            if lower.contains("error")
                || lower.contains("panic")
                || lower.contains("nan")
                || lower.contains("invariant violation")
            {
                errors.push(ObservedLogError {
                    source: source.to_owned(),
                    line: index + 1,
                    message: line.trim().to_owned(),
                });
            }
        }
    }
    errors.sort_by(|a, b| a.source.cmp(&b.source).then(a.line.cmp(&b.line)));
    errors
}

fn first_value<'a>(state: &'a Value, paths: &[&str]) -> Option<&'a Value> {
    paths.iter().find_map(|path| state.pointer(path))
}

fn first_u64(state: &Value, paths: &[&str]) -> Option<u64> {
    first_value(state, paths).and_then(|value| {
        value.as_u64().or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
    })
}

fn first_f64(state: &Value, paths: &[&str]) -> Option<f64> {
    first_value(state, paths).and_then(Value::as_f64)
}

fn first_count(state: &Value, paths: &[&str]) -> Option<u64> {
    first_value(state, paths).and_then(|value| {
        value.as_array().map(|items| items.len() as u64).or_else(|| {
            value.as_u64().or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
        })
    })
}

fn event_label(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.get("type").and_then(Value::as_str).map(str::to_owned))
}

fn fingerprint(metrics: &NatureMetrics, events: &[String]) -> String {
    let bytes = serde_json::to_vec(&(metrics, events)).expect("normalized metrics serialize");
    let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn normalizes_server_observation_and_snapshot_arrays() {
        let state = json!({
            "nature": {
                "tick": 12,
                "clouds": [{"id": 1}, {"id": 2}],
                "rainfall": 0.4,
                "soil_moisture": 0.6,
                "plants": [{"id": 3}],
                "animals": 2,
                "animal_food_available": 3.5,
                "events": [{"type": "RainStarted"}, "PlantGrew"]
            }
        });
        let observation = observe_value(&state, "test", Vec::new());
        assert!(observation.metrics.required_fields_present());
        assert_eq!(observation.metrics.clouds, Some(2));
        assert_eq!(observation.events, vec!["PlantGrew", "RainStarted"]);
        assert!(observation.missing_fields.is_empty());
    }

    #[test]
    fn missing_evidence_is_explicit_and_stale_ticks_are_marked() {
        let previous = observe_value(&json!({"tick": 7}), "before", Vec::new());
        let mut current = observe_value(&json!({"tick": 7}), "after", Vec::new());
        mark_stale_against(&previous, &mut current);
        assert!(current.stale);
        assert!(current.missing_fields.contains(&"soil_moisture".to_owned()));
    }

    #[test]
    fn accepts_flat_initial_nature_artifact() {
        let state = json!({
            "tick": 1,
            "cloud_count": 1,
            "rainfall": 0.2,
            "soil_moisture": 0.1,
            "plant_count": 2,
            "animal_count": 1,
            "animal_food_available": 3
        });
        let observation = observe_value(&state, "nature_initial.json", Vec::new());
        assert!(observation.metrics.required_fields_present());
        assert!(observation.missing_fields.is_empty());
    }
}
