mod args;
mod audit;
mod health;
mod loop_cmd;
mod model_iter;
mod motion;
mod tdd;

use std::{
    env,
    path::PathBuf,
    process::{Command, ExitCode},
};

type Result<T> = std::result::Result<T, String>;

pub(crate) fn workspace_command(root: &std::path::Path, program: &str) -> Command {
    let mut command = Command::new(program);
    command.current_dir(root);
    if program.eq_ignore_ascii_case("cargo") {
        drop_xtask_target_dir(root, &mut command);
    }
    command
}

fn drop_xtask_target_dir(root: &std::path::Path, command: &mut Command) {
    let Ok(target_dir) = env::var("CARGO_TARGET_DIR") else {
        return;
    };
    let target_path = std::path::PathBuf::from(target_dir);
    if target_path == std::path::PathBuf::from(".tmp/xtask-target")
        || target_path == root.join(".tmp/xtask-target")
    {
        command.env_remove("CARGO_TARGET_DIR");
    }
}

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
        "flicker-probe" | "flicker" => loop_cmd::flicker_probe(&root, &args),
        "play" => loop_cmd::play(&root, &args),
        "clean-runs" | "clean-logs" => loop_cmd::clean_runs(&root),
        "motion-analyze" => motion::analyze(&root, &args),
        "scenario" | "run-scenario" => loop_cmd::scenario(&root, &args),
        "tdd" => tdd::run(&root, &args),
        "dev" => run_dev(&root, &args),
        "audit-tdd" => audit::tdd(&root),
        "audit-architecture" => audit::architecture(&root),
        "audit-visual" => audit::visual(&root),
        "model-preview-all" | "model-preview-iterate" => model_iter::run(&root, &args),
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
    println!(
        "  flicker-probe [--seconds N] [--interval S] [--period S] [--warmup S] [--gpu-backend dx12|vulkan]"
    );
    println!("  play [--skip-build] [--online] [client flags...]");
    println!("  clean-runs");
    println!("  tdd --scope core|changed|client|server|workspace|fmt|clippy|audit");
    println!("  dev build|stage-runtime|test|core|clippy|fmt|loop|health|play|help");
    println!("  health [iter_NN|path]");
    println!("  audit-visual");
    println!("  motion-analyze [screenshots/online_motion_trace.jsonl]");
    println!("  scenario --json scenarios/*.json");
    println!("  model-preview-all [--only=<stem>] [--limit=N] [--skip-build]");
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
            &["cargo", "build", "-p", "lk2-client"],
        )
        .and_then(|_| loop_cmd::stage_default_windows_runtime_files(root)),
        "stage-runtime" => loop_cmd::stage_default_windows_runtime_files(root),
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
        "flicker-probe" | "flicker" => loop_cmd::flicker_probe(root, &args[1..]),
        "play" => loop_cmd::play(root, &args[1..]),
        "clean-runs" | "clean-logs" => loop_cmd::clean_runs(root),
        "help" => {
            print_help();
            Ok(())
        }
        other => Err(format!("unknown dev command: {other}")),
    }
}
