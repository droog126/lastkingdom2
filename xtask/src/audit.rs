use std::{
    collections::HashSet,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use crate::Result;

const DECISION_TEMPLATE: &str = r#"# {latest_iter_name} decision

task: [loop goal]
result: pass / partial / fail

score:
- sky: X/10
- player: X/10
- terrain: X/10
- decor: X/10
- hud: X/10
- gameplay: X/10
- total: X.X/10

vs_prev:
- visual: improved / same / worse, with reason
- state: key delta from diff.json{prev_suffix}
- world_space: pass / fail, static world visuals stayed fixed while player moved

ownership:
- player_attached: [expected moving visuals only]
- world_static: [ground/decor/nest/terrain/cloud evidence from final_state visual.static_world]
- debug_visuals: none / listed

claims:
- [claim being validated, e.g. clouds drive rain -> plants -> small animals -> wildlife]
- state_evidence: [health/assertions/final_state/diff paths and values proving or disproving it]
- screenshot_evidence: [PNG observation showing whether the state is visible/readable]
- verdict: proven / disproven / not_observable

carryover:
- source: read .harness/KNOWN_ISSUES.md "Open" section BEFORE writing
- addressed_this_iter:
  - [ISSUE-NNN: short status, or "none"]
- still_open:
  - [ISSUE-NNN: short status, will be picked up next iter, or "same as open"]
- newly_opened:
  - [ISSUE-NNN: new issue you discovered this iter, or "none"]
- sync: after writing this decision, edit .harness/KNOWN_ISSUES.md:
    resolved ISSUEs move from Open to Resolved with commit hash
    new issues get a new ISSUE-NNN + Open entry

problems:
- [concrete problem 1]
- [concrete problem 2]
- [concrete problem 3]

tests:
- [command run]
- [result]

next:
- [single best next action]
"#;

pub fn decision_template(latest_iter_name: &str, prev_name: Option<&str>) -> String {
    DECISION_TEMPLATE
        .replace("{latest_iter_name}", latest_iter_name)
        .replace(
            "{prev_suffix}",
            &prev_name
                .map(|name| format!(" compared with {name}"))
                .unwrap_or_default(),
        )
}

pub fn tdd(root: &Path) -> Result<()> {
    let audit_root = root.join("crates/core/src");
    if !audit_root.exists() {
        return Err("Missing audit root: crates/core/src".to_string());
    }
    let lib_text = fs::read_to_string(audit_root.join("lib.rs")).map_err(|e| e.to_string())?;
    let experimental_modules = feature_gated_modules(&lib_text, "experimental-gameplay");
    let mut rows = Vec::new();
    let mut experimental_rows = Vec::new();
    for file in find_files(&audit_root, |p| {
        p.extension().and_then(OsStr::to_str) == Some("rs")
    })? {
        let text = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let row = (rel(root, &file), count_unit_tests(&text));
        let is_experimental = top_level_module_name(&audit_root, &file)
            .is_some_and(|module| experimental_modules.contains(&module));
        if is_experimental {
            experimental_rows.push(row);
        } else {
            rows.push(row);
        }
    }
    let total_tests: usize = rows.iter().map(|row| row.1).sum();
    let experimental_tests: usize = experimental_rows.iter().map(|row| row.1).sum();
    println!("# TDD audit\n");
    println!("root: crates/core/src");
    println!("files: {}", rows.len());
    println!(
        "files_with_tests: {}",
        rows.iter().filter(|row| row.1 > 0).count()
    );
    println!(
        "files_without_tests: {}",
        rows.iter().filter(|row| row.1 == 0).count()
    );
    println!("unit_tests_found_default: {total_tests}");
    println!("unit_tests_experimental: {experimental_tests}\n");
    println!("## Modules without direct unit tests");
    for (file, _) in rows.iter().filter(|row| row.1 == 0) {
        println!("- {file}");
    }
    println!("\n## Test counts by module");
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
    for (file, tests) in &rows {
        println!("- {file}: {tests}");
    }
    println!("\n## Experimental modules excluded from the default test surface");
    experimental_rows.sort_by(|a, b| a.0.cmp(&b.0));
    if experimental_rows.is_empty() {
        println!("- none");
    } else {
        for (file, tests) in &experimental_rows {
            println!("- {file}: {tests}");
        }
    }

    println!("\n## Top-level modules without runtime references outside themselves");
    let unreferenced =
        top_level_modules_without_runtime_references(root, &audit_root, &experimental_modules)?;
    if unreferenced.is_empty() {
        println!("- none");
    } else {
        for module in unreferenced {
            println!("- {module}");
        }
    }
    if total_tests == 0 {
        return Err("no unit tests found".to_string());
    }
    Ok(())
}

fn count_unit_tests(text: &str) -> usize {
    text.lines().filter(|line| line.trim() == "#[test]").count()
}

fn feature_gated_modules(lib_text: &str, feature: &str) -> HashSet<String> {
    let gate = format!("#[cfg(feature = \"{feature}\")]");
    let mut modules = HashSet::new();
    let mut gated = false;
    for line in lib_text.lines() {
        let trimmed = line.trim();
        if trimmed == gate {
            gated = true;
            continue;
        }
        if gated && trimmed.starts_with("pub mod ") {
            let name = trimmed
                .trim_start_matches("pub mod ")
                .trim_end_matches(';')
                .trim();
            if !name.is_empty() {
                modules.insert(name.to_string());
            }
        }
        if !trimmed.is_empty() && !trimmed.starts_with("#[") {
            gated = false;
        }
    }
    modules
}

fn top_level_module_name(audit_root: &Path, file: &Path) -> Option<String> {
    let relative = file.strip_prefix(audit_root).ok()?;
    let first = relative.components().next()?.as_os_str().to_str()?;
    if first == "lib.rs" {
        return None;
    }
    Some(first.strip_suffix(".rs").unwrap_or(first).to_string())
}

fn production_prefix(text: &str) -> &str {
    text.split_once("#[cfg(test)]")
        .map_or(text, |(production, _)| production)
}

fn runtime_reference_count(module: &str, sources: &[String]) -> usize {
    let patterns = [
        format!("lk2_core::{module}"),
        format!("crate::{module}"),
        format!("super::{module}"),
    ];
    sources
        .iter()
        .map(|source| {
            let production = production_prefix(source);
            patterns
                .iter()
                .map(|pattern| production.matches(pattern).count())
                .sum::<usize>()
        })
        .sum()
}

fn top_level_modules_without_runtime_references(
    root: &Path,
    audit_root: &Path,
    experimental_modules: &HashSet<String>,
) -> Result<Vec<String>> {
    let mut modules = HashSet::new();
    for file in find_files(audit_root, |p| {
        p.extension().and_then(OsStr::to_str) == Some("rs")
    })? {
        if let Some(module) = top_level_module_name(audit_root, &file) {
            if !experimental_modules.contains(&module) {
                modules.insert(module);
            }
        }
    }

    let mut out = Vec::new();
    for module in modules {
        let mut sources = Vec::new();
        for source_root in [
            root.join("crates/core/src"),
            root.join("crates/client/src"),
            root.join("crates/server/src"),
        ] {
            for file in find_files(&source_root, |p| {
                p.extension().and_then(OsStr::to_str) == Some("rs")
            })? {
                if file.starts_with(audit_root)
                    && top_level_module_name(audit_root, &file).as_deref() == Some(module.as_str())
                {
                    continue;
                }
                sources.push(fs::read_to_string(file).map_err(|e| e.to_string())?);
            }
        }
        if runtime_reference_count(&module, &sources) == 0 {
            out.push(module);
        }
    }
    out.sort();
    Ok(out)
}

pub fn architecture(root: &Path) -> Result<()> {
    let mut failed = false;
    println!("# Architecture audit\n");
    let scripts = find_files(&root.join("scripts"), |p| {
        p.extension().and_then(OsStr::to_str) == Some("ps1")
    })?;
    if !scripts.is_empty() {
        failed = true;
        println!("FAIL: PowerShell scripts remain under scripts/:");
        for script in &scripts {
            println!("- {}", rel(root, script));
        }
    }
    let mut hardcoded = Vec::new();
    for file in scripts {
        let text = fs::read_to_string(&file).unwrap_or_default();
        for (idx, line) in text.lines().enumerate() {
            if has_windows_absolute_path(line) {
                hardcoded.push(format!("{}:{}", rel(root, &file), idx + 1));
            }
        }
    }
    if hardcoded.is_empty() {
        println!("OK: scripts are free of hard-coded Windows absolute paths");
    } else {
        failed = true;
        println!(
            "FAIL: scripts contain hard-coded absolute paths: {}",
            hardcoded.join(", ")
        );
    }
    if failed {
        Err("Architecture audit failed".to_string())
    } else {
        Ok(())
    }
}

pub fn skills(root: &Path) -> Result<()> {
    let skills_root = root.join(".codex/skills");
    if !skills_root.exists() {
        return Err("Missing skills root: .codex/skills".to_string());
    }

    let agents_text = fs::read_to_string(root.join("AGENTS.md")).map_err(|e| e.to_string())?;
    let mut issues = Vec::new();
    let mut skill_count = 0_usize;
    for entry in fs::read_dir(&skills_root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        let dir = entry.path();
        let Some(dir_name) = dir.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        let skill_path = dir.join("SKILL.md");
        let metadata_path = dir.join("agents/openai.yaml");
        if !skill_path.exists() || !metadata_path.exists() {
            issues.push(format!(
                ".codex/skills/{dir_name}: SKILL.md and agents/openai.yaml are required"
            ));
            continue;
        }
        skill_count += 1;
        let skill_text = fs::read_to_string(&skill_path).map_err(|e| e.to_string())?;
        let metadata_text = fs::read_to_string(&metadata_path).map_err(|e| e.to_string())?;
        issues.extend(validate_skill_contract(
            dir_name,
            &skill_text,
            &metadata_text,
            &agents_text,
        ));
    }

    println!("# Skill contract audit\n");
    println!("skills: {skill_count}");
    if issues.is_empty() {
        println!("OK: skill frontmatter, metadata, and AGENTS routes are complete");
        Ok(())
    } else {
        for issue in &issues {
            println!("FAIL: {issue}");
        }
        Err(format!(
            "Skill contract audit failed with {} issue(s)",
            issues.len()
        ))
    }
}

fn parse_skill_frontmatter(text: &str) -> Result<(String, String)> {
    let mut lines = text.lines();
    if lines.next() != Some("---") {
        return Err("SKILL.md must start with YAML frontmatter".to_string());
    }

    let mut name = None;
    let mut description = None;
    let mut closed = false;
    for line in lines.by_ref() {
        if line == "---" {
            closed = true;
            break;
        }
        if line.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            return Err(format!("invalid frontmatter line: {line}"));
        };
        let key = key.trim();
        if !matches!(key, "name" | "description") {
            return Err(format!("frontmatter field {key} is not allowed"));
        }
        let value = value.trim();
        if value.is_empty()
            || value.starts_with('[')
            || value.starts_with('{')
            || value.starts_with('|')
            || value.starts_with('>')
            || value.to_ascii_uppercase().contains("TODO")
        {
            return Err(format!(
                "frontmatter {key} must be a completed scalar value"
            ));
        }
        let value = value.trim_matches('"').trim_matches('\'').to_string();
        match key {
            "name" if name.is_none() => name = Some(value),
            "description" if description.is_none() => description = Some(value),
            _ => return Err(format!("frontmatter field {key} is duplicated")),
        }
    }

    if !closed {
        return Err("SKILL.md frontmatter is missing its closing ---".to_string());
    }

    match (name, description) {
        (Some(name), Some(description)) => Ok((name, description)),
        _ => Err("SKILL.md frontmatter requires name and description".to_string()),
    }
}

fn validate_skill_contract(
    dir_name: &str,
    skill_text: &str,
    metadata_text: &str,
    agents_text: &str,
) -> Vec<String> {
    let mut issues = Vec::new();
    match parse_skill_frontmatter(skill_text) {
        Ok((name, _)) if name != dir_name => issues.push(format!(
            ".codex/skills/{dir_name}/SKILL.md name is {name}, expected {dir_name}"
        )),
        Ok(_) => {}
        Err(err) => issues.push(format!(".codex/skills/{dir_name}/SKILL.md: {err}")),
    }
    let upper_skill = skill_text.to_ascii_uppercase();
    if upper_skill.contains("[TODO")
        || upper_skill
            .lines()
            .any(|line| line.trim_start().starts_with("TODO:"))
        || skill_text.contains("Structuring This Skill")
    {
        issues.push(format!(
            ".codex/skills/{dir_name}/SKILL.md contains unfinished TODO scaffold text"
        ));
    }
    if !metadata_text.contains("display_name:")
        || !metadata_text.contains("short_description:")
        || !metadata_text.contains("default_prompt:")
        || !metadata_text.contains(&format!("${dir_name}"))
    {
        issues.push(format!(
            ".codex/skills/{dir_name}/agents/openai.yaml is incomplete or lacks ${dir_name}"
        ));
    }
    if let Some(short_description) = metadata_scalar(metadata_text, "short_description") {
        let len = short_description.chars().count();
        if !(25..=64).contains(&len) {
            issues.push(format!(
                ".codex/skills/{dir_name}/agents/openai.yaml short_description must be 25-64 characters, got {len}"
            ));
        }
    }
    if !agents_text.contains(&format!("${dir_name}")) {
        issues.push(format!(
            "AGENTS.md is missing the ${dir_name} routing entry"
        ));
    }
    let expected_path = format!(".codex/skills/{dir_name}/SKILL.md");
    if !agents_text.contains(&expected_path) {
        issues.push(format!(
            "AGENTS.md is missing the {expected_path} skill path"
        ));
    }
    issues
}

fn metadata_scalar(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (candidate, value) = line.trim().split_once(':')?;
        (candidate == key).then(|| {
            value
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string()
        })
    })
}

pub fn visual(root: &Path) -> Result<()> {
    let client_root = root.join("crates/client/src");
    if !client_root.exists() {
        return Err("Missing audit root: crates/client/src".to_string());
    }

    let mut failed = false;
    let mut warnings = Vec::new();
    println!("# Visual ownership audit\n");
    for file in find_files(&client_root, |p| {
        p.extension().and_then(OsStr::to_str) == Some("rs")
    })? {
        let text = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let mut debug_depth = 0_i32;
        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("show_legacy_debug_props")
                || trimmed.contains("show_monster_cubes")
                || trimmed.contains("audit-pretty-models")
            {
                debug_depth = 64;
            }
            let follows_player = trimmed.contains("player.pos +")
                || trimmed.contains("player.pos.x +")
                || trimmed.contains("player.pos.z +");
            let world_visual_hint = trimmed.contains("Plane3d")
                || trimmed.contains("WorldGroundFallback")
                || trimmed.contains("CloudPuff")
                || trimmed.contains("grounded_world_pos")
                || trimmed.contains("spawn_ground_detail")
                || trimmed.contains("spawn_playable_village");
            let visual_spawn_context = trimmed.contains("Transform::from_translation")
                || trimmed.contains("WorldAssetRoot")
                || trimmed.contains("Mesh3d")
                || trimmed.contains("let pos =")
                || trimmed.contains("let tree_")
                || trimmed.contains("let stick_");
            if follows_player && world_visual_hint {
                failed = true;
                println!(
                    "FAIL: {}:{} world-looking visual depends on player position: {}",
                    rel(root, &file),
                    idx + 1,
                    trimmed
                );
            } else if follows_player && visual_spawn_context && debug_depth <= 0 {
                warnings.push(format!("{}:{} {}", rel(root, &file), idx + 1, trimmed));
            }
            debug_depth -= 1;
        }
    }

    if warnings.is_empty() {
        println!("OK: no unclassified player-relative visual placement found");
    } else {
        println!("WARN: unclassified player-relative visual placement:");
        for warning in warnings.iter().take(24) {
            println!("- {warning}");
        }
    }

    if failed {
        Err("Visual ownership audit failed".to_string())
    } else {
        Ok(())
    }
}

pub fn find_files<F>(start: &Path, predicate: F) -> Result<Vec<PathBuf>>
where
    F: Fn(&Path) -> bool + Copy,
{
    let mut out = Vec::new();
    if !start.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(start).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            out.extend(find_files(&path, predicate)?);
        } else if predicate(&path) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

pub fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn has_windows_absolute_path(line: &str) -> bool {
    line.as_bytes()
        .windows(3)
        .any(|w| w[0].is_ascii_alphabetic() && w[1] == b':' && (w[2] == b'\\' || w[2] == b'/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_template_replaces_iter_name() {
        let out = decision_template("iter_201", None);
        assert!(out.starts_with("# iter_201 decision\n"), "got: {out}");
    }

    #[test]
    fn decision_template_adds_prev_suffix_when_provided() {
        let out = decision_template("iter_201", Some("iter_200"));
        assert!(
            out.contains("key delta from diff.json compared with iter_200"),
            "got: {out}"
        );
    }

    #[test]
    fn decision_template_omits_prev_suffix_when_absent() {
        let out = decision_template("iter_201", None);
        assert!(out.contains("key delta from diff.json\n"), "got: {out}");
    }

    #[test]
    fn decision_template_forces_carryover_section() {
        let out = decision_template("iter_201", None);
        assert!(
            out.contains("carryover:"),
            "missing carryover section:\n{out}"
        );
        assert!(
            out.contains("addressed_this_iter"),
            "missing addressed_this_iter line:\n{out}"
        );
        assert!(
            out.contains("still_open"),
            "missing still_open line:\n{out}"
        );
        assert!(
            out.contains("newly_opened"),
            "missing newly_opened line:\n{out}"
        );
    }

    #[test]
    fn decision_template_forces_claim_evidence_section() {
        let out = decision_template("iter_201", None);
        assert!(out.contains("claims:"), "missing claims section:\n{out}");
        assert!(
            out.contains("state_evidence"),
            "missing state evidence line:\n{out}"
        );
        assert!(
            out.contains("screenshot_evidence"),
            "missing screenshot evidence line:\n{out}"
        );
        assert!(
            out.contains("verdict:"),
            "missing claim verdict line:\n{out}"
        );
    }

    #[test]
    fn decision_template_references_known_issues_file() {
        let out = decision_template("iter_201", None);
        assert!(
            out.contains(".harness/KNOWN_ISSUES.md"),
            "missing reference to KNOWN_ISSUES.md:\n{out}"
        );
    }

    #[test]
    fn unit_test_counter_ignores_string_literals_and_comments() {
        let source = r##"
#[test]
fn real_test() {}
const EXAMPLE: &str = "#[test]";
// #[test]
"##;
        assert_eq!(count_unit_tests(source), 1);
    }

    #[test]
    fn experimental_module_parser_finds_only_immediately_gated_modules() {
        let source = r#"
#[cfg(feature = "experimental-gameplay")]
pub mod equipment;
pub mod match_state;
#[cfg(feature = "different")]
pub mod other;
#[cfg(feature = "experimental-gameplay")]
pub mod mining_site;
"#;
        let modules = feature_gated_modules(source, "experimental-gameplay");
        assert_eq!(
            modules,
            HashSet::from(["equipment".into(), "mining_site".into()])
        );
    }

    #[test]
    fn runtime_reference_counter_ignores_cfg_test_tail() {
        let sources = vec![
            "use lk2_core::resource::GlobalResourcePool;".to_string(),
            "#[cfg(test)]\nmod tests { use lk2_core::resource::ResourceKind; }".to_string(),
        ];
        assert_eq!(runtime_reference_count("resource", &sources), 1);
    }

    #[test]
    fn skill_frontmatter_requires_scalar_name_and_description() {
        let valid = r#"---
name: game-logic-audit
description: Audit game logic without modifying the repository.
---

# Game Logic Audit
"#;
        assert_eq!(
            parse_skill_frontmatter(valid).unwrap(),
            (
                "game-logic-audit".to_string(),
                "Audit game logic without modifying the repository.".to_string()
            )
        );

        let scaffold = r#"---
name: game-logic-audit
description: [TODO: explain this skill]
---
"#;
        assert!(parse_skill_frontmatter(scaffold).is_err());
    }

    #[test]
    fn skill_frontmatter_rejects_extra_fields_and_unclosed_blocks() {
        let extra = r#"---
name: game-logic-audit
description: Audit game logic without modifying the repository.
metadata: not-allowed
---
"#;
        assert!(parse_skill_frontmatter(extra).is_err());

        let unclosed = r#"---
name: game-logic-audit
description: Audit game logic without modifying the repository.
"#;
        assert!(parse_skill_frontmatter(unclosed).is_err());
    }

    #[test]
    fn skill_contract_rejects_scaffolds_and_missing_routes() {
        let skill = r#"---
name: game-logic-audit
description: Audit game logic without modifying the repository.
---

# Game Logic Audit

[TODO: replace this scaffold]
"#;
        let metadata = r#"interface:
  display_name: "Game Logic Audit"
  short_description: "Audit runtime game logic"
  default_prompt: "Use $game-logic-audit to audit game logic."
"#;
        let issues = validate_skill_contract("game-logic-audit", skill, metadata, "# AGENTS.md\n");
        assert!(issues.iter().any(|issue| issue.contains("TODO")));
        assert!(issues.iter().any(|issue| issue.contains("routing entry")));
        assert!(issues.iter().any(|issue| issue.contains("skill path")));
    }
}
