use std::{
    env,
    ffi::OsStr,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;
use serde_json::json;

use crate::{Result, args, audit, health as rust_health};

const DEV_DYNAMIC_FEATURES: &[&str] = &["dev-dynamic-linking", "lk2-core/dev-dynamic-linking"];

#[derive(Debug)]
struct LoopArgs {
    seconds: u64,
    max_extra_wait: u64,
    rust_log: String,
    skip_build: bool,
    dynamic: bool,
    online: bool,
    offline: bool,
    no_server: bool,
    server_addr: String,
    first_person: bool,
    audit_pretty_models: bool,
    no_kenney: bool,
    legacy_voxel: bool,
    hold_forward_test: bool,
    refresh_after_fail: bool,
}

impl Default for LoopArgs {
    fn default() -> Self {
        Self {
            seconds: 60,
            max_extra_wait: 60,
            rust_log: env::var("RUST_LOG").unwrap_or_else(|_| {
                "info,lightyear_replication=debug,lightyear_connection=debug,lightyear_send=debug,lightyear_receive=debug".to_string()
            }),
            skip_build: false,
            dynamic: !cfg!(windows),
            online: false,
            offline: false,
            no_server: false,
            server_addr: "127.0.0.1:5000".to_string(),
            first_person: false,
            audit_pretty_models: false,
            no_kenney: false,
            legacy_voxel: false,
            hold_forward_test: false,
            refresh_after_fail: false,
        }
    }
}

pub fn run(root: &Path, raw: &[String]) -> Result<()> {
    if raw.iter().any(|a| a == "--help" || a == "-h" || a == "-?") {
        println!(
            "xtask loop --offline --seconds 12 --skip-build --no-dynamic --first-person --refresh-after-fail"
        );
        return Ok(());
    }
    let parsed = parse_loop(raw);
    fs::create_dir_all(root.join("run-logs")).map_err(|e| e.to_string())?;
    fs::create_dir_all(root.join("screenshots")).map_err(|e| e.to_string())?;
    enforce_decision_gate(root, parsed.refresh_after_fail)?;

    let use_offline = parsed.offline || (!parsed.online && !parsed.no_server);
    let mut envs = runtime_env(root, &parsed.rust_log)?;
    if parsed.no_kenney {
        envs.push(("LK2_DISABLE_KENNEY".to_string(), "1".to_string()));
        println!(">>> Kenney gameplay models OFF <<<");
    }
    stop_processes(&["lk2-client", "lk2-server"]);
    thread::sleep(Duration::from_secs(1));

    let mut features = Vec::new();
    if parsed.dynamic {
        features.extend_from_slice(DEV_DYNAMIC_FEATURES);
        println!(">>> dynamic linking ON <<<");
    }
    if parsed.audit_pretty_models {
        features.push("audit-pretty-models");
        println!(">>> audit pretty models ON <<<");
    }

    let client_exe = exe_path(root, "lk2-client");
    let server_exe = exe_path(root, "lk2-server");
    if !parsed.skip_build || !client_exe.exists() {
        cargo_build_with_fallback(root, "lk2-client", &features, &envs)?;
    }
    if !use_offline && !parsed.no_server && (!parsed.skip_build || !server_exe.exists()) {
        cargo_build_with_fallback(root, "lk2-server", &features, &envs)?;
    }
    if !client_exe.exists() {
        return Err(format!("binary not found: {}", client_exe.display()));
    }
    stage_windows_runtime_files(root)?;

    let before = latest_iter(root).and_then(|p| iter_number(&p)).unwrap_or(0);
    let server_log = root.join("screenshots/loop_server.log");
    let client_log = root.join("screenshots/loop_run.log");
    let (mode, mut client_args) = if use_offline {
        println!(
            ">>> Mode: OFFLINE (no server, client --offline --auto-demo) {}s ...",
            parsed.seconds
        );
        (
            "offline",
            vec!["--offline".to_string(), "--auto-demo".to_string()],
        )
    } else if parsed.no_server {
        println!(
            ">>> Mode: NOSERVER (no lk2-server, client --connect={} will fail) {}s ...",
            parsed.server_addr, parsed.seconds
        );
        (
            "noserver",
            vec![
                format!("--connect={}", parsed.server_addr),
                "--auto-demo".to_string(),
            ],
        )
    } else {
        println!(
            ">>> Mode: ONLINE (server + client --connect={}) {}s ...",
            parsed.server_addr, parsed.seconds
        );
        (
            "online",
            vec![
                format!("--connect={}", parsed.server_addr),
                "--auto-demo".to_string(),
            ],
        )
    };
    if parsed.first_person {
        client_args.push("--first-person".to_string());
    }
    if parsed.legacy_voxel {
        client_args.push("--legacy-voxel".to_string());
    }
    if parsed.hold_forward_test {
        client_args.push("--hold-forward-test".to_string());
    }

    let mut server_proc = None;
    if mode == "online" {
        server_proc = Some(spawn_logged(root, &server_exe, &[], &envs, &server_log)?);
        thread::sleep(Duration::from_secs(3));
    }

    println!(">>> Starting lk2-client ({mode}) ...");
    let mut client_proc = spawn_logged(root, &client_exe, &client_args, &envs, &client_log)?;
    let ready = wait_for_iter(
        root,
        &mut client_proc,
        before,
        parsed.seconds,
        parsed.max_extra_wait,
    );
    if let Some(iter) = &ready {
        println!(
            ">>> Loop capture ready: {}",
            iter.file_name().and_then(OsStr::to_str).unwrap_or("?")
        );
    } else {
        println!(
            ">>> Loop capture did not reach ready state before timeout; stopping for health check"
        );
    }
    stop_child(&mut client_proc);
    if let Some(mut child) = server_proc {
        stop_child(&mut child);
    }
    thread::sleep(Duration::from_secs(1));

    if ready.is_none() {
        let message = runtime_failure_message(&client_log);
        write_loop_diagnosis(
            root,
            &diagnosis_json(
                "runtime",
                "no_new_capture",
                &message,
                "fix_client_startup_or_capture_before_visual_iteration",
            ),
        )?;
        write_auto_decision(root, "runtime", &message)?;
        print_latest(root)?;
        return Err(message);
    }

    print_latest(root)?;
    let health_result = run_latest_health(root);
    write_decision_template(root)?;
    if let Err(err) = health_result {
        write_loop_diagnosis(
            root,
            &diagnosis_json("health", "health_verdict_fail", &err, ""),
        )?;
        write_auto_decision(root, "health", &err)?;
        return Err(err);
    }
    println!(
        "\n>>> Done. AI: read latest health.json first; if PARTIAL/FAIL, read assertions.json before PNG/state."
    );
    Ok(())
}

pub fn health(root: &Path, raw: &[String]) -> Result<()> {
    let iter = if let Some(target) = raw.first() {
        let path = PathBuf::from(target);
        if path.exists() {
            path
        } else {
            root.join("screenshots").join(target)
        }
    } else {
        latest_iter(root).ok_or_else(|| "no iter_* dir found".to_string())?
    };
    let prev = previous_iter(root, &iter);
    let verdict = evaluate_health(root, &iter, prev.as_deref(), false)?;
    if verdict == "FAIL" {
        return Err("health verdict FAIL".to_string());
    }
    Ok(())
}

pub fn scenario(root: &Path, raw: &[String]) -> Result<()> {
    let mut json = "scenarios/*.json".to_string();
    let mut seconds = 60;
    let mut skip_build = false;
    let mut i = 0;
    while i < raw.len() {
        let (name, inline) = args::split_flag(&raw[i]);
        match name.as_deref() {
            Some("json") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    json = value;
                }
            }
            Some("seconds") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    seconds = value.parse().unwrap_or(seconds);
                }
            }
            Some("skipbuild") | Some("skip-build") => skip_build = true,
            _ => {}
        }
        i += 1;
    }
    let envs = runtime_env(
        root,
        &env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
    )?;
    let client_exe = exe_path(root, "lk2-client");
    if !skip_build || !client_exe.exists() {
        cargo_build_with_fallback(root, "lk2-client", DEV_DYNAMIC_FEATURES, &envs)?;
    }
    stage_windows_runtime_files(root)?;
    let files = expand_pattern(root, &json)?;
    if files.is_empty() {
        return Err(format!("no JSON files matched: {json}"));
    }
    for file in files {
        println!(">>> Running scenario: {}", audit::rel(root, &file));
        let mut proc = spawn_logged(
            root,
            &client_exe,
            &["--offline".to_string(), file.display().to_string()],
            &envs,
            &root.join("run-logs/scenario_run.log"),
        )?;
        let deadline = Instant::now() + Duration::from_secs(seconds);
        while Instant::now() < deadline {
            if proc.try_wait().map_err(|e| e.to_string())?.is_some() {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }
        stop_child(&mut proc);
    }
    Ok(())
}

fn parse_loop(raw: &[String]) -> LoopArgs {
    let mut parsed = LoopArgs::default();
    let mut i = 0;
    while i < raw.len() {
        let (name, inline) = args::split_flag(&raw[i]);
        match name.as_deref() {
            Some("seconds") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.seconds = value.parse().unwrap_or(parsed.seconds);
                }
            }
            Some("maxextrawait") | Some("max-extra-wait") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.max_extra_wait = value.parse().unwrap_or(parsed.max_extra_wait);
                }
            }
            Some("rustlog") | Some("rust-log") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.rust_log = value;
                }
            }
            Some("skipbuild") | Some("skip-build") => parsed.skip_build = true,
            Some("dynamic") => parsed.dynamic = !args::matches_false(inline.as_deref()),
            Some("no-dynamic") | Some("nodynamic") => parsed.dynamic = false,
            Some("online") => parsed.online = true,
            Some("offline") => parsed.offline = true,
            Some("noserver") | Some("no-server") => parsed.no_server = true,
            Some("serveraddr") | Some("server-addr") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.server_addr = value;
                }
            }
            Some("firstperson") | Some("first-person") => parsed.first_person = true,
            Some("auditprettymodels") | Some("audit-pretty-models") => {
                parsed.audit_pretty_models = true
            }
            Some("nokenney") | Some("no-kenney") => parsed.no_kenney = true,
            Some("legacyvoxel") | Some("legacy-voxel") => parsed.legacy_voxel = true,
            Some("holdforwardtest") | Some("hold-forward-test") => parsed.hold_forward_test = true,
            Some("refreshafterfail") | Some("refresh-after-fail") => {
                parsed.refresh_after_fail = true
            }
            _ => {}
        }
        i += 1;
    }
    parsed
}

fn runtime_env(root: &Path, rust_log: &str) -> Result<Vec<(String, String)>> {
    let sysroot =
        match Command::new("rustc").args(["--print", "sysroot"]).current_dir(root).output() {
            Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
            Err(e) => {
                println!(
                    ">>> [warn] rustc --print sysroot failed: {}, using RUSTUP_HOME",
                    e
                );
                if let Ok(rustup_home) = env::var("RUSTUP_HOME") {
                    format!("{rustup_home}/toolchains/stable-x86_64-pc-windows-msvc")
                } else {
                    String::new()
                }
            }
        };
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut path = [root.join("target/debug/deps"), root.join("target/debug")]
        .into_iter()
        .filter(|p| p.exists())
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>();
    if !sysroot.is_empty() {
        let sysroot_bin = PathBuf::from(&sysroot).join("bin");
        if sysroot_bin.exists() {
            path.push(sysroot_bin.display().to_string());
        }
    }
    let path_str = path.join(sep);
    println!(
        ">>> PATH (truncated): {}...",
        path_str.chars().take(100).collect::<String>()
    );
    Ok(vec![
        ("BEVY_DISABLE_ACCESSIBILITY".to_string(), "1".to_string()),
        ("BEVY_ASSET_ROOT".to_string(), root.display().to_string()),
        ("RUST_LOG".to_string(), rust_log.to_string()),
        ("CARGO_MANIFEST_DIR".to_string(), root.display().to_string()),
        ("PATH".to_string(), path_str),
        ("WGPU_BACKEND".to_string(), "dx12".to_string()),
    ])
}

#[derive(Debug, Clone)]
struct BuildFailure {
    package: String,
    text: String,
}

fn cargo_build_with_fallback(
    root: &Path,
    package: &str,
    features: &[&str],
    envs: &[(String, String)],
) -> Result<()> {
    match cargo_build_once(root, package, features, envs) {
        Ok(()) => Ok(()),
        Err(first) if !features.is_empty() && is_dynamic_link_failure(&first.text) => {
            println!(
                ">>> dynamic-link build failed for {package}; retrying static build without dynamic features"
            );
            write_loop_diagnosis(root, &build_diagnosis(&first, "retry_static_build"))?;
            match cargo_build_once(root, package, &[], envs) {
                Ok(()) => Ok(()),
                Err(second) => {
                    write_loop_diagnosis(
                        root,
                        &build_diagnosis(&second, "fix_compile_before_loop"),
                    )?;
                    write_auto_decision(root, "build", &second.text)?;
                    Err(format!(
                        ">>> BUILD FAILED for {package}; see run-logs/loop_diagnosis.json"
                    ))
                }
            }
        }
        Err(failure) => {
            write_loop_diagnosis(root, &build_diagnosis(&failure, "fix_compile_before_loop"))?;
            write_auto_decision(root, "build", &failure.text)?;
            Err(format!(
                ">>> BUILD FAILED for {package}; see run-logs/loop_diagnosis.json"
            ))
        }
    }
}

fn cargo_build_once(
    root: &Path,
    package: &str,
    features: &[&str],
    envs: &[(String, String)],
) -> std::result::Result<(), BuildFailure> {
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "-p", package]).current_dir(root);
    let joined;
    if !features.is_empty() {
        joined = features.join(",");
        cmd.args(["--features", &joined]);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    println!(">>> {:?}", cmd);
    let output = cmd
        .output()
        .map_err(|e| BuildFailure { package: package.to_string(), text: e.to_string() })?;
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    let _ = fs::create_dir_all(root.join("run-logs"));
    fs::write(root.join("run-logs/build_loop.log"), &text)
        .map_err(|e| BuildFailure { package: package.to_string(), text: e.to_string() })?;
    for line in text.lines().rev().take(5).collect::<Vec<_>>().into_iter().rev() {
        println!("{line}");
    }
    if !output.status.success() {
        return Err(BuildFailure { package: package.to_string(), text });
    }
    Ok(())
}

fn spawn_logged(
    root: &Path,
    exe: &Path,
    args: &[String],
    envs: &[(String, String)],
    log: &Path,
) -> Result<Child> {
    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let stderr = PathBuf::from(format!("{}.err", log.display()));
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .current_dir(root)
        .stdout(Stdio::from(File::create(log).map_err(|e| e.to_string())?))
        .stderr(Stdio::from(
            File::create(stderr).map_err(|e| e.to_string())?,
        ))
        .stdin(Stdio::null());
    cmd.env_remove("PATH");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    println!(">>> Running: {:?}", cmd);
    cmd.spawn().map_err(|e| format!("failed to start {}: {e}", exe.display()))
}

fn is_dynamic_link_failure(text: &str) -> bool {
    text.contains("LNK1189") || text.contains("bevy_dylib")
}

fn classify_build_failure(text: &str) -> (&'static str, Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut files = Vec::new();
    for line in text.lines() {
        if let Some(pos) = line.find("error[E") {
            let code = line[pos + "error[".len()..].split(']').next().unwrap_or("").to_string();
            if !code.is_empty() && !errors.contains(&code) {
                errors.push(code);
            }
        }
        if line.contains("--> ") {
            if let Some(path) = line.split("-->").nth(1).and_then(|s| s.trim().split(':').next()) {
                let path = path.replace('\\', "/");
                if !path.is_empty() && !files.contains(&path) {
                    files.push(path);
                }
            }
        }
    }
    let kind = if is_dynamic_link_failure(text) {
        "windows_dynamic_link_failure"
    } else if !errors.is_empty() {
        "rust_compile_error"
    } else {
        "build_failed"
    };
    (kind, errors, files)
}

fn build_diagnosis(failure: &BuildFailure, next_action: &str) -> Value {
    let (kind, errors, primary_files) = classify_build_failure(&failure.text);
    json!({
        "stage": "build",
        "kind": kind,
        "package": failure.package,
        "errors": errors,
        "primary_files": primary_files,
        "next_action": next_action,
        "message": failure.text.lines().rev().take(20).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n"),
    })
}

fn diagnosis_json(stage: &str, kind: &str, message: &str, next_action: &str) -> Value {
    json!({
        "stage": stage,
        "kind": kind,
        "message": message,
        "next_action": next_action,
    })
}

fn write_loop_diagnosis(root: &Path, diagnosis: &Value) -> Result<()> {
    let dir = root.join("run-logs");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::write(
        dir.join("loop_diagnosis.json"),
        serde_json::to_string_pretty(diagnosis).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())
}

fn write_auto_decision(root: &Path, stage: &str, message: &str) -> Result<()> {
    let name = latest_iter(root)
        .and_then(|iter| iter.file_name().and_then(OsStr::to_str).map(str::to_string))
        .unwrap_or_else(|| "loop".to_string());
    let clipped = message.lines().rev().take(12).collect::<Vec<_>>();
    let mut clipped = clipped.into_iter().rev().collect::<Vec<_>>().join("\n");
    if clipped.is_empty() {
        clipped = "no additional message".to_string();
    }
    let body = format!(
        "# {name} decision\n\n\
task: closed-loop automation failure\n\
result: fail\n\n\
score:\n\
- sky: blocked - {stage} failed before visual scoring could complete\n\
- player: blocked - {stage} failed before visual scoring could complete\n\
- terrain: blocked - {stage} failed before visual scoring could complete\n\
- decor: blocked - {stage} failed before visual scoring could complete\n\
- hud: blocked - {stage} failed before visual scoring could complete\n\
- gameplay: blocked - {stage} failed before loop completion\n\
- total: 0.0/10\n\n\
vs_prev:\n\
- visual: not compared - loop failed at {stage}\n\
- state: not compared\n\n\
problems:\n\
- loop failed during {stage}\n\
- {clipped}\n\n\
tests:\n\
- xtask loop attempted\n\
- result: failed during {stage}\n\n\
next:\n\
- fix the {stage} failure reported in run-logs/loop_diagnosis.json before visual iteration\n"
    );
    let run_logs = root.join("run-logs");
    fs::create_dir_all(&run_logs).map_err(|e| e.to_string())?;
    fs::write(run_logs.join("loop_decision.md"), &body).map_err(|e| e.to_string())?;

    if let Some(iter) = latest_iter(root) {
        let decision = iter.join("decision.md");
        if !decision.exists() {
            fs::write(decision, body).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn runtime_failure_message(client_log: &Path) -> String {
    let stdout_tail = read_tail(client_log, 40);
    let stderr_tail = read_tail(&PathBuf::from(format!("{}.err", client_log.display())), 80);
    format!(
        "client exited or timed out before producing a ready iter\nstdout tail:\n{stdout_tail}\nstderr tail:\n{stderr_tail}"
    )
}

fn read_tail(path: &Path, lines: usize) -> String {
    fs::read_to_string(path)
        .map(|text| {
            let mut tail = text.lines().rev().take(lines).collect::<Vec<_>>();
            tail.reverse();
            tail.join("\n")
        })
        .unwrap_or_else(|err| format!("{}: {err}", path.display()))
}

fn wait_for_iter(
    root: &Path,
    child: &mut Child,
    before: u32,
    seconds: u64,
    extra: u64,
) -> Option<PathBuf> {
    let started = Instant::now();
    let min = Duration::from_secs(seconds);
    let max = min + Duration::from_secs(extra);
    while started.elapsed() < max {
        thread::sleep(Duration::from_secs(1));
        match child.try_wait() {
            Ok(Some(status)) => {
                println!(">>> client exited before loop capture: {status}");
                break;
            }
            Ok(None) => {}
            Err(err) => {
                println!(">>> failed to query client status: {err}");
                break;
            }
        }
        if started.elapsed() < min {
            continue;
        }
        if let Some(iter) = next_ready_iter(root, before) {
            return Some(iter);
        }
    }
    None
}

fn iter_ready(iter: &Path) -> bool {
    let state = iter.join("final_state.json");
    let Ok(text) = fs::read_to_string(state) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    // iter_210: in online mode SimClock.tick stays at 0 (server runs the
    // authoritative sim), so the tick >= 500 gate would never fire. Accept
    // either an offline tick >= 500 OR an online iter that has run at least
    // 4 wall seconds and produced a >= 30KB screenshot.
    let role = json.get("role").and_then(Value::as_str).unwrap_or("");
    let tick_ok = json.get("tick").and_then(Value::as_i64).unwrap_or(0) >= 500;
    let wall_ok = json.get("wall_secs").and_then(Value::as_f64).unwrap_or(0.0) >= 4.0;
    if json.pointer("/visual/movement_probe/first_player_pos").is_none()
        || json.pointer("/visual/movement_probe/current_player_pos").is_none()
    {
        return false;
    }
    if role == "client_offline" {
        if !tick_ok {
            return false;
        }
    } else if !wall_ok {
        return false;
    }
    fs::read_dir(iter).ok().into_iter().flatten().filter_map(|e| e.ok()).any(|e| {
        let name = e.file_name().to_string_lossy().to_string();
        name.starts_with("iter_")
            && name.ends_with(".png")
            && e.metadata().map(|m| m.len() > 30 * 1024).unwrap_or(false)
    })
}

fn next_ready_iter(root: &Path, before: u32) -> Option<PathBuf> {
    let iter = newest_iter_after(root, before)?;
    iter_ready(&iter).then_some(iter)
}

fn evaluate_health(root: &Path, iter: &Path, prev: Option<&Path>, quiet: bool) -> Result<String> {
    let evaluation = rust_health::evaluate_iter(root, iter, prev)?;
    if !quiet {
        println!("  {}", evaluation.summary);
    }
    Ok(evaluation.verdict)
}

fn run_latest_health(root: &Path) -> Result<()> {
    println!("\n=== Health check ===");
    let Some(iter) = latest_iter(root) else {
        println!("  (no iter_* dir to check)");
        return Ok(());
    };
    let prev = previous_iter(root, &iter);
    let verdict = evaluate_health(root, &iter, prev.as_deref(), false)?;
    if verdict == "FAIL" {
        return Err("health verdict FAIL".to_string());
    }
    Ok(())
}

fn print_latest(root: &Path) -> Result<()> {
    println!("\n=== Latest screenshots ===");
    let mut pngs = audit::find_files(&root.join("screenshots"), |p| {
        p.file_name()
            .and_then(OsStr::to_str)
            .map(|n| n.starts_with("iter_") && n.ends_with(".png"))
            .unwrap_or(false)
    })?;
    pngs.sort_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok());
    for p in pngs.into_iter().rev().take(5) {
        println!("  {}", audit::rel(root, &p));
    }
    Ok(())
}

fn write_decision_template(root: &Path) -> Result<()> {
    let dirs = iter_dirs(root)?;
    let Some(latest) = dirs.last() else {
        println!("\n[warn] no iter_* directory found -- can't write decision.template.md");
        return Ok(());
    };
    let prev =
        dirs.get(dirs.len().saturating_sub(2)).and_then(|p| p.file_name()).and_then(OsStr::to_str);
    let latest_name = latest.file_name().and_then(OsStr::to_str).unwrap_or("iter_NN");
    fs::write(
        latest.join("decision.template.md"),
        audit::decision_template(latest_name, prev),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "\n=== SCORE reminder written: {} ===",
        latest.join("decision.template.md").display()
    );
    Ok(())
}

fn enforce_decision_gate(root: &Path, refresh_after_fail: bool) -> Result<()> {
    let Some(prev) = latest_iter(root) else {
        return Ok(());
    };
    let prev_name = prev.file_name().and_then(OsStr::to_str).unwrap_or("iter");

    // gate 1: decision.md 必须存在
    if !prev.join("decision.md").exists() {
        let message = format!(
            "\n=============================================\n  FAIL: {prev_name}/decision.md MISSING\n=============================================\n Previous loop has no decision.md; refusing next loop.\n Template: {}",
            prev.join("decision.template.md").display()
        );
        write_loop_diagnosis(
            root,
            &diagnosis_json(
                "gate",
                "previous_decision_missing",
                &message,
                "inspect_previous_health_and_fix_before_next_loop",
            ),
        )?;
        write_auto_decision(root, "gate", &message)?;
        return Err(message);
    }
    println!(">>> [OK] previous {prev_name}/decision.md exists -- decision gate green");

    // gate 2: health.json 必须存在, FAIL 阻断, PARTIAL warn, PASS 放行
    let health_path = prev.join("health.json");
    if !health_path.exists() {
        println!(
            ">>> [warn] previous {prev_name}/health.json MISSING -- verdict gate bypassed (legacy iter)"
        );
        return Ok(());
    }
    match read_health_verdict(&health_path).as_deref() {
        Some("PASS") => {
            println!(
                ">>> [OK] previous {prev_name}/health.json verdict = PASS -- verdict gate green"
            );
        }
        Some("PARTIAL") => {
            println!(
                ">>> [warn] previous {prev_name}/health.json verdict = PARTIAL -- not blocking, but address in this iter's decision.md problems:"
            );
        }
        Some("FAIL") | None => {
            if refresh_after_fail {
                println!(
                    ">>> [warn] previous {prev_name}/health.json verdict = FAIL -- --refresh-after-fail set, running a replacement loop"
                );
                return Ok(());
            }
            let message = format!(
                "\n=============================================\n  FAIL: previous {prev_name}/health.json verdict = FAIL\n=============================================\n Previous loop FAILED health check; refusing next loop.\n 1. Read {decision_md} for what failed\n 2. Fix the failures\n 3. Re-run `cargo run -q -p xtask -- health {prev_name}` to confirm PASS\n 4. Then re-run `cargo run -q -p xtask -- loop`",
                decision_md = prev.join("decision.md").display(),
            );
            write_loop_diagnosis(
                root,
                &diagnosis_json(
                    "gate",
                    "previous_health_failed",
                    &message,
                    "fix_previous_health_or_rerun_health_after_rule_change",
                ),
            )?;
            write_auto_decision(root, "gate", &message)?;
            return Err(message);
        }
        Some(other) => {
            println!(
                ">>> [warn] previous {prev_name}/health.json verdict UNKNOWN ({other}) -- bypassing verdict gate"
            );
        }
    }
    Ok(())
}

fn read_health_verdict(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;
    json.get("verdict").and_then(Value::as_str).map(str::to_string)
}

fn latest_iter(root: &Path) -> Option<PathBuf> {
    iter_dirs(root).ok()?.pop()
}

fn newest_iter_after(root: &Path, before: u32) -> Option<PathBuf> {
    iter_dirs(root).ok()?.into_iter().filter(|p| iter_number(p).unwrap_or(0) > before).next_back()
}

fn previous_iter(root: &Path, iter: &Path) -> Option<PathBuf> {
    let n = iter_number(iter)?;
    iter_dirs(root).ok()?.into_iter().filter(|p| iter_number(p).unwrap_or(0) < n).next_back()
}

fn iter_dirs(root: &Path) -> Result<Vec<PathBuf>> {
    let mut dirs = Vec::new();
    let ss = root.join("screenshots");
    if !ss.exists() {
        return Ok(dirs);
    }
    for entry in fs::read_dir(ss).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() && iter_number(&path).is_some() {
            dirs.push(path);
        }
    }
    dirs.sort_by_key(|p| iter_number(p).unwrap_or(0));
    Ok(dirs)
}

fn iter_number(path: &Path) -> Option<u32> {
    path.file_name().and_then(OsStr::to_str)?.strip_prefix("iter_")?.parse().ok()
}

fn exe_path(root: &Path, name: &str) -> PathBuf {
    root.join("target/debug").join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    })
}

fn stop_processes(names: &[&str]) {
    for name in names {
        if cfg!(windows) {
            let _ = Command::new("taskkill")
                .args(["/IM", &format!("{name}.exe"), "/F", "/T"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        } else {
            let _ = Command::new("pkill")
                .args(["-x", name])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
    }
}

fn stop_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub fn stage_windows_runtime_files(root: &Path) -> Result<()> {
    if !cfg!(windows) {
        return Ok(());
    }
    let debug = root.join("target/debug");
    if !debug.exists() {
        return Ok(());
    }

    for dll in find_bevy_dylibs(&debug)? {
        copy_to_dir(&dll, &debug)?;
    }

    for dll in find_rust_std_dylibs(root)? {
        copy_to_dir(&dll, &debug)?;
    }

    copy_bevy_dylib_alias(&debug)?;
    stage_assets_dir(root, &debug)?;
    Ok(())
}

fn find_bevy_dylibs(debug: &Path) -> Result<Vec<PathBuf>> {
    let mut dlls = audit::find_files(&debug, |p| {
        p.file_name()
            .and_then(OsStr::to_str)
            .map(|n| n.starts_with("bevy_dylib-") && n.ends_with(".dll"))
            .unwrap_or(false)
    })?;
    dlls.sort_by_key(|p| fs::metadata(p).and_then(|m| m.modified()).ok());
    Ok(dlls)
}

fn find_rust_std_dylibs(root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("rustc")
        .args(["--print", "sysroot"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("rustc --print sysroot failed: {e}"))?;
    if !output.status.success() {
        return Ok(Vec::new());
    }
    let sysroot = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sysroot.is_empty() {
        return Ok(Vec::new());
    }
    let bin = PathBuf::from(sysroot).join("bin");
    if !bin.exists() {
        return Ok(Vec::new());
    }
    let mut dlls = Vec::new();
    for entry in fs::read_dir(bin).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or("");
        if name.starts_with("std-") && name.ends_with(".dll") {
            dlls.push(path);
        }
    }
    dlls.sort();
    Ok(dlls)
}

fn copy_bevy_dylib_alias(debug: &Path) -> Result<()> {
    let mut dlls = find_bevy_dylibs(debug)?;
    if let Some(latest) = dlls.pop() {
        copy_file_if_different(&latest, &debug.join("bevy_dylib.dll"))?;
    }
    Ok(())
}

fn copy_to_dir(src: &Path, dst_dir: &Path) -> Result<()> {
    let Some(name) = src.file_name() else {
        return Ok(());
    };
    copy_file_if_different(src, &dst_dir.join(name))
}

fn copy_file_if_different(src: &Path, dst: &Path) -> Result<()> {
    if src == dst {
        return Ok(());
    }
    if dst.exists() {
        let src_meta = fs::metadata(src).map_err(|e| e.to_string())?;
        let dst_meta = fs::metadata(dst).map_err(|e| e.to_string())?;
        if src_meta.len() == dst_meta.len() {
            return Ok(());
        }
    }
    fs::copy(src, dst).map_err(|e| e.to_string())?;
    Ok(())
}

fn stage_assets_dir(root: &Path, debug: &Path) -> Result<()> {
    let src = root.join("assets");
    if !src.exists() {
        return Ok(());
    }
    let dst = debug.join("assets");
    if dst.exists() {
        return Ok(());
    }
    if try_junction(&src, &dst).is_ok() {
        return Ok(());
    }
    copy_dir_recursive(&src, &dst)
}

fn try_junction(src: &Path, dst: &Path) -> Result<()> {
    let status = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(dst)
        .arg(src)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("mklink /J failed".to_string())
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            copy_file_if_different(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn expand_pattern(root: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
    let pattern = pattern.replace('\\', "/");
    if !pattern.contains('*') {
        let path = root.join(pattern);
        return Ok(if path.exists() { vec![path] } else { vec![] });
    }
    let slash = pattern.rfind('/').unwrap_or(0);
    let (dir, file_pat) = if slash == 0 {
        (".", pattern.as_str())
    } else {
        (&pattern[..slash], &pattern[slash + 1..])
    };
    let mut out = Vec::new();
    for entry in fs::read_dir(root.join(dir)).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or("");
        if wildcard_match(file_pat, name) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn wildcard_match(pattern: &str, value: &str) -> bool {
    if let Some((prefix, suffix)) = pattern.split_once('*') {
        value.starts_with(prefix) && value.ends_with(suffix)
    } else {
        pattern == value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_legacy_dash_flags() {
        let parsed = parse_loop(&[
            "-Offline".into(),
            "-Seconds".into(),
            "12".into(),
            "-Dynamic:$false".into(),
            "-FirstPerson".into(),
            "--legacy-voxel".into(),
        ]);
        assert!(parsed.offline);
        assert_eq!(parsed.seconds, 12);
        assert!(!parsed.dynamic);
        assert!(parsed.first_person);
        assert!(parsed.legacy_voxel);
    }

    #[test]
    fn dev_dynamic_features_include_core_alignment() {
        assert_eq!(
            DEV_DYNAMIC_FEATURES,
            &["dev-dynamic-linking", "lk2-core/dev-dynamic-linking"]
        );
    }

    #[test]
    fn default_loop_disables_dynamic_on_windows() {
        assert_eq!(LoopArgs::default().dynamic, !cfg!(windows));
    }

    #[test]
    fn classifies_windows_dynamic_link_failure() {
        let (kind, errors, files) = classify_build_failure(
            "LINK : fatal error LNK1189: exceeded library limit\nbevy_dylib",
        );

        assert_eq!(kind, "windows_dynamic_link_failure");
        assert!(errors.is_empty());
        assert!(files.is_empty());
    }

    #[test]
    fn classifies_rust_compile_errors_and_primary_files() {
        let text = "error[E0004]: non-exhaustive patterns\n   --> crates\\client\\src\\pretty\\mod.rs:1295:15\nerror[E0599]: no method named chain\n   --> crates\\client\\src\\main.rs:563:66\n";

        let (kind, errors, files) = classify_build_failure(text);

        assert_eq!(kind, "rust_compile_error");
        assert_eq!(errors, vec!["E0004", "E0599"]);
        assert_eq!(
            files,
            vec![
                "crates/client/src/pretty/mod.rs",
                "crates/client/src/main.rs"
            ]
        );
    }

    #[test]
    fn read_tail_keeps_latest_lines() {
        let root = temp_root("xtask_tail");
        let path = root.join("log.txt");
        fs::write(&path, "a\nb\nc\nd\n").unwrap();

        assert_eq!(read_tail(&path, 2), "c\nd");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn next_ready_iter_waits_when_no_new_iter_exists() {
        let root = temp_root("xtask_no_new_iter");
        fs::create_dir_all(root.join("screenshots")).unwrap();
        assert!(next_ready_iter(&root, 1).is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn next_ready_iter_ignores_incomplete_new_iter() {
        let root = temp_root("xtask_incomplete_iter");
        let iter = root.join("screenshots/iter_2");
        fs::create_dir_all(&iter).unwrap();
        fs::write(iter.join("final_state.json"), r#"{"tick":499}"#).unwrap();
        assert!(next_ready_iter(&root, 1).is_none());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn next_ready_iter_requires_movement_probe_schema() {
        let root = temp_root("xtask_missing_probe_iter");
        let iter = root.join("screenshots/iter_2");
        fs::create_dir_all(&iter).unwrap();
        fs::write(
            iter.join("final_state.json"),
            r#"{"role":"client_offline","tick":500,"wall_secs":5.0}"#,
        )
        .unwrap();
        fs::write(iter.join("iter_2.png"), vec![1u8; 40 * 1024]).unwrap();
        assert!(next_ready_iter(&root, 1).is_none());

        fs::write(
            iter.join("final_state.json"),
            r#"{"role":"client_offline","tick":500,"wall_secs":5.0,"visual":{"movement_probe":{"first_player_pos":[1,2,3],"current_player_pos":[2,2,3]}}}"#,
        )
        .unwrap();
        assert_eq!(next_ready_iter(&root, 1), Some(iter));
        let _ = fs::remove_dir_all(root);
    }

    fn temp_root(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!("{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn seed_prev_iter(root: &Path, decision: bool, health_verdict: Option<&str>) -> PathBuf {
        let iter = root.join("screenshots/iter_10");
        fs::create_dir_all(&iter).unwrap();
        if decision {
            fs::write(
                iter.join("decision.md"),
                "# iter_10 decision\nresult: pass\n",
            )
            .unwrap();
        }
        if let Some(verdict) = health_verdict {
            let payload = serde_json::json!({"verdict": verdict, "score": 0.0}).to_string();
            fs::write(iter.join("health.json"), payload).unwrap();
        }
        iter
    }

    #[test]
    fn gate_rejects_when_prev_decision_md_missing() {
        let root = temp_root("xtask_gate_no_decision");
        seed_prev_iter(&root, false, Some("PASS"));
        let err = enforce_decision_gate(&root, false).unwrap_err();
        assert!(err.contains("decision.md MISSING"), "got: {err}");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gate_rejects_when_prev_health_verdict_is_fail() {
        let root = temp_root("xtask_gate_health_fail");
        seed_prev_iter(&root, true, Some("FAIL"));
        let err = enforce_decision_gate(&root, false).unwrap_err();
        assert!(err.contains("verdict = FAIL"), "got: {err}");
        assert!(
            err.contains("Refusing") || err.contains("FAIL"),
            "expected FAIL gate message, got: {err}"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gate_can_refresh_after_failed_health_when_explicit() {
        let root = temp_root("xtask_gate_health_fail_refresh");
        seed_prev_iter(&root, true, Some("FAIL"));
        assert!(enforce_decision_gate(&root, true).is_ok());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gate_rejects_when_prev_health_verdict_is_unparseable() {
        let root = temp_root("xtask_gate_health_garbage");
        let iter = seed_prev_iter(&root, true, None);
        fs::write(iter.join("health.json"), "not json").unwrap();
        let err = enforce_decision_gate(&root, false).unwrap_err();
        assert!(err.contains("verdict = FAIL"), "got: {err}");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gate_passes_when_prev_health_verdict_is_pass() {
        let root = temp_root("xtask_gate_health_pass");
        seed_prev_iter(&root, true, Some("PASS"));
        assert!(enforce_decision_gate(&root, false).is_ok());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gate_passes_with_warn_when_prev_health_missing() {
        let root = temp_root("xtask_gate_health_missing");
        seed_prev_iter(&root, true, None);
        assert!(enforce_decision_gate(&root, false).is_ok());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn gate_passes_with_warn_when_prev_health_partial() {
        let root = temp_root("xtask_gate_health_partial");
        seed_prev_iter(&root, true, Some("PARTIAL"));
        assert!(enforce_decision_gate(&root, false).is_ok());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn read_health_verdict_extracts_string() {
        let dir = env::temp_dir().join(format!("xtask_rhv_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("health.json");
        fs::write(&path, r#"{"verdict":"PASS","score":10.0}"#).unwrap();
        assert_eq!(read_health_verdict(&path).as_deref(), Some("PASS"));
        fs::write(&path, "garbage").unwrap();
        assert_eq!(read_health_verdict(&path), None);
        let _ = fs::remove_dir_all(dir);
    }
}
