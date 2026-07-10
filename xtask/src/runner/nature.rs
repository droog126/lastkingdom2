//! Scenario loading and evidence evaluation for the natural-world loop.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    artifacts::nature::{NatureArtifact, write_nature_artifact},
    assertions::nature::{NatureExpectations, evaluate_nature},
    observer::nature::{mark_stale_against, observe_file},
};

pub const NATURE_SCENARIO_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NatureScenario {
    pub schema: u32,
    pub name: String,
    pub seed: u64,
    pub ticks: u64,
    pub evidence: NatureEvidencePaths,
    #[serde(default)]
    pub expectations: NatureExpectations,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NatureEvidencePaths {
    pub before: String,
    pub after: String,
    pub deterministic_repeat: String,
    #[serde(default)]
    pub error_logs: Vec<String>,
}

impl NatureScenario {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != NATURE_SCENARIO_SCHEMA {
            return Err(format!(
                "unsupported nature scenario schema {}",
                self.schema
            ));
        }
        if self.name.trim().is_empty() {
            return Err("nature scenario name must not be empty".to_owned());
        }
        if self.ticks == 0 {
            return Err("nature scenario ticks must be positive".to_owned());
        }
        for (name, path) in [
            ("before", &self.evidence.before),
            ("after", &self.evidence.after),
            ("deterministic_repeat", &self.evidence.deterministic_repeat),
        ] {
            validate_relative_path(name, path)?;
        }
        for path in &self.evidence.error_logs {
            validate_relative_path("error_logs", path)?;
        }
        Ok(())
    }
}

pub fn load_scenario(path: &Path) -> Result<NatureScenario, String> {
    let text =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let scenario: NatureScenario = serde_json::from_str(&text)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    scenario.validate()?;
    Ok(scenario)
}

pub fn evaluate_evidence(
    scenario: &NatureScenario,
    evidence_root: &Path,
    output_dir: &Path,
) -> Result<NatureArtifact, String> {
    scenario.validate()?;
    let log_paths = scenario
        .evidence
        .error_logs
        .iter()
        .map(|path| evidence_root.join(path))
        .collect::<Vec<_>>();
    let log_refs = log_paths.iter().map(PathBuf::as_path).collect::<Vec<_>>();

    let before = observe_file(&evidence_root.join(&scenario.evidence.before), &log_refs)?;
    let mut after = observe_file(&evidence_root.join(&scenario.evidence.after), &log_refs)?;
    mark_stale_against(&before, &mut after);
    let repeat = observe_file(
        &evidence_root.join(&scenario.evidence.deterministic_repeat),
        &log_refs,
    )?;
    let assertions = evaluate_nature(Some(&before), &after, Some(&repeat), &scenario.expectations);
    let artifact = NatureArtifact::new(
        scenario.name.clone(),
        scenario.seed,
        Some(before),
        after,
        Some(repeat),
        assertions,
    );
    write_nature_artifact(output_dir, &artifact)?;
    Ok(artifact)
}

fn validate_relative_path(name: &str, value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.trim().is_empty() || path.is_absolute() || value.contains("..") {
        return Err(format!("{name} must be a safe relative path"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use super::*;

    #[test]
    fn evaluates_real_files_and_writes_machine_artifact() {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("lk2_nature_runner_{nonce}"));
        fs::create_dir_all(&root).unwrap();
        let before = state(1, 0.1, 1, 0);
        let after = state(2, 0.3, 2, 1);
        fs::write(root.join("before.json"), before.to_string()).unwrap();
        fs::write(root.join("after.json"), after.to_string()).unwrap();
        fs::write(root.join("repeat.json"), after.to_string()).unwrap();
        let scenario = NatureScenario {
            schema: 1,
            name: "test".to_owned(),
            seed: 9,
            ticks: 2,
            evidence: NatureEvidencePaths {
                before: "before.json".to_owned(),
                after: "after.json".to_owned(),
                deterministic_repeat: "repeat.json".to_owned(),
                error_logs: Vec::new(),
            },
            expectations: NatureExpectations::default(),
        };
        let output = root.join("iter_01");
        let artifact = evaluate_evidence(&scenario, &root, &output).unwrap();
        assert_eq!(artifact.verdict, "OK");
        assert!(output.join("nature.json").exists());
        fs::remove_dir_all(root).unwrap();
    }

    fn state(tick: u64, moisture: f64, plants: u64, animals: u64) -> serde_json::Value {
        json!({
            "nature": {
                "tick": tick,
                "cloud_count": 1,
                "rainfall": 0.4,
                "soil_moisture": moisture,
                "plant_count": plants,
                "animal_count": animals,
                "animal_food_available": 2.0
            },
            "presentation": {"nature": {"clouds": 1, "plants": plants, "animals": animals}}
        })
    }
}
