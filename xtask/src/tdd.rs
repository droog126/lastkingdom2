use std::{env, path::Path, process::Command};

use crate::{Result, args, audit, workspace_command};

#[derive(Debug)]
struct TddArgs {
    scope: String,
    test_name: Option<String>,
    base_ref: String,
    no_default_features: bool,
}

impl Default for TddArgs {
    fn default() -> Self {
        Self {
            scope: "core".to_string(),
            test_name: None,
            base_ref: "HEAD".to_string(),
            no_default_features: false,
        }
    }
}

pub fn run(root: &Path, raw: &[String]) -> Result<()> {
    let parsed = parse(raw);
    set_common_env();
    match parsed.scope.as_str() {
        "core" => run_step(root, "core tests", &cargo_test_args("lk2-core", &parsed)),
        "client" => {
            run_step(
                root,
                "client tests",
                &cargo_test_args("lk2-client", &parsed),
            )?;
            run_step(
                root,
                "client build",
                &["cargo", "build", "-p", "lk2-client"],
            )
        }
        "server" => {
            run_step(
                root,
                "server tests",
                &cargo_test_args("lk2-server", &parsed),
            )?;
            run_step(
                root,
                "server build",
                &["cargo", "build", "-p", "lk2-server"],
            )
        }
        "workspace" => {
            let mut cmd = vec!["cargo", "test", "--workspace"];
            if let Some(test) = parsed.test_name.as_deref() {
                cmd.push(test);
            }
            run_step(root, "workspace tests", &cmd)
        }
        "fmt" => run_step(root, "format check", &["cargo", "fmt", "--check"]),
        "clippy" => run_step(
            root,
            "clippy",
            &["cargo", "clippy", "--workspace", "--all-targets"],
        ),
        "audit" => {
            audit::tdd(root)?;
            audit::architecture(root)?;
            crate::doc_audit::run(root)?;
            audit::skills(root)?;
            audit::visual(root)
        }
        "changed" => run_changed(root, &parsed),
        other => Err(format!("unknown TDD scope: {other}")),
    }?;
    println!();
    println!(">>> TDD scope '{}' passed", parsed.scope);
    Ok(())
}

fn parse(raw: &[String]) -> TddArgs {
    let mut parsed = TddArgs::default();
    let mut i = 0;
    while i < raw.len() {
        let arg = &raw[i];
        let (name, inline) = args::split_flag(arg);
        match name.as_deref() {
            Some("scope") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.scope = value.to_ascii_lowercase();
                }
            }
            Some("testname") | Some("test-name") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.test_name = Some(value);
                }
            }
            Some("baseref") | Some("base-ref") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.base_ref = value;
                }
            }
            Some("nodefaultfeatures") | Some("no-default-features") => {
                parsed.no_default_features = true;
            }
            _ if !arg.starts_with('-') && parsed.scope == "core" => {
                parsed.scope = arg.to_ascii_lowercase();
            }
            _ => {}
        }
        i += 1;
    }
    parsed
}

fn cargo_test_args<'a>(package: &'a str, args: &'a TddArgs) -> Vec<&'a str> {
    let mut cmd = vec!["cargo", "test", "-p", package];
    if args.no_default_features {
        cmd.push("--no-default-features");
    }
    if let Some(test) = args.test_name.as_deref() {
        cmd.push(test);
    }
    cmd
}

fn set_common_env() {
    #[allow(unsafe_code)]
    unsafe {
        env::set_var("BEVY_DISABLE_ACCESSIBILITY", "1");
        if env::var_os("RUST_LOG").is_none() {
            env::set_var("RUST_LOG", "warn");
        }
    }
}

fn run_changed(root: &Path, args: &TddArgs) -> Result<()> {
    let mut changed = command_lines(root, "git", &["diff", "--name-only", &args.base_ref])?;
    changed.extend(command_lines(
        root,
        "git",
        &["ls-files", "--others", "--exclude-standard"],
    )?);
    changed.retain(|line| !line.trim().is_empty());
    if changed.is_empty() {
        return run_step(
            root,
            "core tests (no changed files detected)",
            &cargo_test_args("lk2-core", args),
        );
    }
    let touches_cargo = changed.iter().any(|p| {
        p == "Cargo.toml" || p == "Cargo.lock" || wildcard_match("crates/*/Cargo.toml", p)
    });
    let touches_core = changed.iter().any(|p| p.starts_with("crates/core/"));
    let touches_client = changed.iter().any(|p| p.starts_with("crates/client/"));
    let touches_server = changed.iter().any(|p| p.starts_with("crates/server/"));
    let touches_xtask = changed.iter().any(|p| p.starts_with("xtask/"));
    let touches_doc_contract = changed.iter().any(|p| affects_documentation_contract(p));
    let touches_skills = changed.iter().any(|p| affects_skill_contract(p));

    if touches_core || touches_cargo {
        run_step(root, "core tests", &cargo_test_args("lk2-core", args))?;
    }
    if touches_client || touches_cargo {
        run_step(root, "client tests", &cargo_test_args("lk2-client", args))?;
    }
    if touches_server || touches_cargo {
        run_step(root, "server tests", &cargo_test_args("lk2-server", args))?;
    }
    if touches_xtask || touches_cargo {
        run_step(root, "xtask tests", &["cargo", "test", "-p", "xtask"])?;
    }
    if touches_doc_contract {
        crate::doc_audit::run(root)?;
    }
    if touches_skills {
        audit::skills(root)?;
    }
    if !(touches_core
        || touches_client
        || touches_server
        || touches_xtask
        || touches_cargo
        || touches_doc_contract
        || touches_skills)
    {
        run_step(
            root,
            "format check for non-code changes",
            &["cargo", "fmt", "--check"],
        )?;
    }
    Ok(())
}

fn affects_documentation_contract(path: &str) -> bool {
    path == "README.md"
        || path == "AGENTS.md"
        || path == "Cargo.toml"
        || path == "justfile"
        || path.starts_with("docs/")
        || path.starts_with("xtask/")
}

fn affects_skill_contract(path: &str) -> bool {
    path == "AGENTS.md" || path.starts_with(".codex/skills/")
}

fn command_lines(root: &Path, program: &str, args: &[&str]) -> Result<Vec<String>> {
    let output = Command::new(program)
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("{program} failed to start: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).lines().map(|s| s.replace('\\', "/")).collect())
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        value.starts_with(prefix) && value.ends_with(suffix)
    } else {
        pattern == value
    }
}

pub fn run_step(root: &Path, title: &str, cmd: &[&str]) -> Result<()> {
    println!();
    println!(">>> {title}");
    println!("    {}", cmd.join(" "));
    let (program, args) = cmd.split_first().ok_or_else(|| "empty command".to_string())?;
    let status = workspace_command(root, program)
        .args(args)
        .status()
        .map_err(|e| format!("{program} failed to start: {e}"))?;
    if !status.success() {
        return Err(format!(">>> FAILED: {title}"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documentation_contract_paths_trigger_the_docs_audit() {
        for path in [
            "README.md",
            "AGENTS.md",
            "Cargo.toml",
            "justfile",
            "docs/STARTING.md",
            "xtask/src/main.rs",
        ] {
            assert!(affects_documentation_contract(path), "path={path}");
        }
        assert!(!affects_documentation_contract(
            "crates/core/src/world/mod.rs"
        ));
    }

    #[test]
    fn skill_contract_paths_trigger_the_skill_audit() {
        assert!(affects_skill_contract("AGENTS.md"));
        assert!(affects_skill_contract(
            ".codex/skills/docs-governance/SKILL.md"
        ));
        assert!(!affects_skill_contract("docs/README.md"));
    }
}
