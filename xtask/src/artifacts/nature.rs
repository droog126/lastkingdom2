//! Stable artifact types for machine-checking the authoritative natural world.

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const NATURE_ARTIFACT_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NatureMetrics {
    pub tick: Option<u64>,
    pub clouds: Option<u64>,
    pub rainfall: Option<f64>,
    pub soil_moisture: Option<f64>,
    pub plants: Option<u64>,
    pub animals: Option<u64>,
    pub animal_food_available: Option<f64>,
    pub presented_clouds: Option<u64>,
    pub presented_plants: Option<u64>,
    pub presented_animals: Option<u64>,
    /// Number of raw simulation events before labels are normalized.
    pub event_count: Option<u64>,
    /// Tick of the region represented by the reported snapshot.
    pub region_tick: Option<u64>,
    /// Unsimulated world ticks remaining after bounded catch-up.
    pub catch_up_remaining: Option<u64>,
    /// Whether the producer restored persisted natural-world state.
    pub save_restored: Option<bool>,
}

impl NatureMetrics {
    #[must_use]
    pub fn required_fields_present(&self) -> bool {
        self.tick.is_some()
            && self.clouds.is_some()
            && self.rainfall.is_some()
            && self.soil_moisture.is_some()
            && self.plants.is_some()
            && self.animals.is_some()
            && self.animal_food_available.is_some()
    }

    #[must_use]
    pub fn numeric_values_are_valid(&self) -> bool {
        [
            self.rainfall,
            self.soil_moisture,
            self.animal_food_available,
        ]
        .into_iter()
        .flatten()
        .all(|value| value.is_finite() && value >= 0.0)
    }

    #[must_use]
    pub fn runtime_evidence_is_valid(&self, observed_event_count: usize) -> bool {
        let region_is_not_ahead = match (self.tick, self.region_tick) {
            (Some(world_tick), Some(region_tick)) => region_tick <= world_tick,
            _ => true,
        };
        let event_count_covers_observation = self
            .event_count
            .is_none_or(|count| count >= observed_event_count as u64);
        region_is_not_ahead && event_count_covers_observation
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ObservedLogError {
    pub source: String,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NatureObservation {
    pub schema: u32,
    pub source: String,
    pub metrics: NatureMetrics,
    pub events: Vec<String>,
    pub missing_fields: Vec<String>,
    pub stale: bool,
    pub fingerprint: String,
    pub errors: Vec<ObservedLogError>,
}

impl NatureObservation {
    pub fn normalize(&mut self) {
        self.events.sort();
        self.events.dedup();
        self.missing_fields.sort();
        self.missing_fields.dedup();
        self.errors.sort_by(|a, b| {
            a.source
                .cmp(&b.source)
                .then(a.line.cmp(&b.line))
                .then(a.message.cmp(&b.message))
        });
        self.errors.dedup();
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssertionSeverity {
    Fail,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NatureAssertion {
    pub id: String,
    pub ok: bool,
    pub severity: AssertionSeverity,
    pub actual: Value,
    pub op: String,
    pub expected: Value,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NatureArtifact {
    pub schema: u32,
    pub scenario: String,
    pub seed: u64,
    pub before: Option<NatureObservation>,
    pub after: NatureObservation,
    pub deterministic_repeat: Option<NatureObservation>,
    pub assertions: Vec<NatureAssertion>,
    pub verdict: String,
}

impl NatureArtifact {
    #[must_use]
    pub fn new(
        scenario: String,
        seed: u64,
        before: Option<NatureObservation>,
        after: NatureObservation,
        deterministic_repeat: Option<NatureObservation>,
        mut assertions: Vec<NatureAssertion>,
    ) -> Self {
        assertions.sort_by(|a, b| a.id.cmp(&b.id));
        let verdict = if assertions
            .iter()
            .any(|item| !item.ok && item.severity == AssertionSeverity::Fail)
        {
            "FAIL"
        } else if assertions.iter().any(|item| !item.ok) {
            "PARTIAL"
        } else {
            "OK"
        };
        Self {
            schema: NATURE_ARTIFACT_SCHEMA,
            scenario,
            seed,
            before,
            after,
            deterministic_repeat,
            assertions,
            verdict: verdict.to_owned(),
        }
    }

    #[must_use]
    pub fn health_extension(&self) -> Value {
        let failed = self.assertions.iter().filter(|item| !item.ok).count();
        let hard_failed = self
            .assertions
            .iter()
            .filter(|item| !item.ok && item.severity == AssertionSeverity::Fail)
            .count();
        json!({
            "schema": self.schema,
            "scenario": self.scenario,
            "seed": self.seed,
            "verdict": self.verdict,
            "fingerprint": self.after.fingerprint,
            "assertions": {
                "total": self.assertions.len(),
                "failed": failed,
                "hard_failed": hard_failed
            },
            "runtime_evidence": {
                "event_count": self.after.metrics.event_count,
                "region_tick": self.after.metrics.region_tick,
                "catch_up_remaining": self.after.metrics.catch_up_remaining,
                "save_restored": self.after.metrics.save_restored
            }
        })
    }
}

pub fn write_nature_artifact(iter_dir: &Path, artifact: &NatureArtifact) -> Result<(), String> {
    fs::create_dir_all(iter_dir)
        .map_err(|error| format!("create {}: {error}", iter_dir.display()))?;
    let text = serde_json::to_string_pretty(artifact).map_err(|error| error.to_string())? + "\n";
    fs::write(iter_dir.join("nature.json"), text)
        .map_err(|error| format!("write nature.json: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation() -> NatureObservation {
        NatureObservation {
            schema: 1,
            source: "final_state.json".to_owned(),
            metrics: NatureMetrics {
                tick: Some(10),
                event_count: Some(1),
                region_tick: Some(9),
                catch_up_remaining: Some(2),
                save_restored: Some(true),
                ..NatureMetrics::default()
            },
            events: Vec::new(),
            missing_fields: Vec::new(),
            stale: false,
            fingerprint: "fnv1a64:1".to_owned(),
            errors: Vec::new(),
        }
    }

    #[test]
    fn hard_failure_controls_verdict_and_health_extension() {
        let artifact = NatureArtifact::new(
            "test".to_owned(),
            7,
            None,
            observation(),
            None,
            vec![NatureAssertion {
                id: "nature.test".to_owned(),
                ok: false,
                severity: AssertionSeverity::Fail,
                actual: json!(0),
                op: ">".to_owned(),
                expected: json!(0),
                message: "failed".to_owned(),
                path: None,
            }],
        );
        assert_eq!(artifact.verdict, "FAIL");
        let health = artifact.health_extension();
        assert_eq!(health["verdict"], "FAIL");
        assert_eq!(health["assertions"]["failed"], 1);
        assert_eq!(health["assertions"]["hard_failed"], 1);
        assert_eq!(health["runtime_evidence"]["event_count"], 1);
        assert_eq!(health["runtime_evidence"]["region_tick"], 9);
        assert_eq!(health["runtime_evidence"]["catch_up_remaining"], 2);
        assert_eq!(health["runtime_evidence"]["save_restored"], true);
    }
}
