use std::{
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
    DECISION_TEMPLATE.replace("{latest_iter_name}", latest_iter_name).replace(
        "{prev_suffix}",
        &prev_name.map(|name| format!(" compared with {name}")).unwrap_or_default(),
    )
}

pub fn tdd(root: &Path) -> Result<()> {
    let audit_root = root.join("crates/core/src");
    if !audit_root.exists() {
        return Err("Missing audit root: crates/core/src".to_string());
    }
    let mut rows = Vec::new();
    for file in find_files(&audit_root, |p| {
        p.extension().and_then(OsStr::to_str) == Some("rs")
    })? {
        let text = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        rows.push((rel(root, &file), text.matches("#[test]").count()));
    }
    let total_tests: usize = rows.iter().map(|row| row.1).sum();
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
    println!("unit_tests_found: {total_tests}\n");
    println!("## Modules without direct unit tests");
    for (file, _) in rows.iter().filter(|row| row.1 == 0) {
        println!("- {file}");
    }
    println!("\n## Test counts by module");
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)));
    for (file, tests) in &rows {
        println!("- {file}: {tests}");
    }
    if total_tests == 0 {
        return Err("no unit tests found".to_string());
    }
    Ok(())
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

pub fn visual(root: &Path) -> Result<()> {
    let client_root = root.join("crates/client/src");
    if !client_root.exists() {
        return Err("Missing audit root: crates/client/src".to_string());
    }

    let mut failed = false;
    let mut warnings = Vec::new();
    println!("# Visual ownership audit\n");
    for file in find_files(&client_root, |p| p.extension().and_then(OsStr::to_str) == Some("rs"))? {
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
    path.strip_prefix(root).unwrap_or(path).display().to_string()
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
        assert!(out.contains("carryover:"), "missing carryover section:\n{out}");
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
    fn decision_template_references_known_issues_file() {
        let out = decision_template("iter_201", None);
        assert!(
            out.contains(".harness/KNOWN_ISSUES.md"),
            "missing reference to KNOWN_ISSUES.md:\n{out}"
        );
    }
}
