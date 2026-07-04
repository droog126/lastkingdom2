mod args;
mod audit;
mod health;
mod loop_cmd;
mod tdd;

use std::{env, path::PathBuf, process::ExitCode};

type Result<T> = std::result::Result<T, String>;

fn main() -> ExitCode {
    let root = match project_root() {
        Ok(root) => root,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };

    let mut args: Vec<String> = env::args().skip(1).collect();
    let Some(command) = args.first().cloned() else {
        print_help();
        return ExitCode::from(2);
    };
    args.remove(0);

    let result = match command.to_ascii_lowercase().as_str() {
        "loop" => loop_cmd::run(&root, &args),
        "health" => loop_cmd::health(&root, &args),
        "scenario" | "run-scenario" => loop_cmd::scenario(&root, &args),
        "tdd" => tdd::run(&root, &args),
        "dev" => run_dev(&root, &args),
        "audit-tdd" => audit::tdd(&root),
        "audit-architecture" => audit::architecture(&root),
        "help" | "-h" | "--help" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown xtask command: {other}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn project_root() -> Result<PathBuf> {
    let mut dir = env::current_dir().map_err(|e| format!("current_dir failed: {e}"))?;
    loop {
        if dir.join("Cargo.toml").exists() && dir.join("crates").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err(
                "could not find project root containing Cargo.toml and crates/".to_string(),
            );
        }
    }
}

fn print_help() {
    println!("xtask commands:");
    println!("  loop [--offline] [--seconds N]");
    println!("  tdd --scope core|changed|client|server|workspace|fmt|clippy|audit");
    println!("  dev build|stage-runtime|test|core|clippy|fmt|loop|health|help");
    println!("  health [iter_NN|path]");
    println!("  scenario --json scenarios/*.json");
}

fn run_dev(root: &std::path::Path, args: &[String]) -> Result<()> {
    let Some(command) = args.first().map(|s| s.to_ascii_lowercase()) else {
        print_help();
        return Ok(());
    };
    match command.as_str() {
        "build" => tdd::run_step(
            root,
            "client build",
            &[
                "cargo",
                "build",
                "-p",
                "lk2-client",
                "--features",
                "dev-dynamic-linking,lk2-core/dev-dynamic-linking",
            ],
        )
        .and_then(|_| loop_cmd::stage_windows_runtime_files(root)),
        "stage-runtime" => loop_cmd::stage_windows_runtime_files(root),
        "test" => tdd::run_step(root, "workspace tests", &["cargo", "test", "--workspace"]),
        "core" => tdd::run_step(root, "core tests", &["cargo", "test", "-p", "lk2-core"]),
        "clippy" => tdd::run_step(
            root,
            "clippy",
            &["cargo", "clippy", "--workspace", "--", "-D", "warnings"],
        ),
        "fmt" => tdd::run_step(
            root,
            "format check",
            &["cargo", "fmt", "--all", "--", "--check"],
        ),
        "loop" => loop_cmd::run(root, &args[1..]),
        "health" => loop_cmd::health(root, &args[1..]),
        "help" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown dev command: {other}")),
    }
}
