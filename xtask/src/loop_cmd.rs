use std::{
    env,
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use serde_json::json;

use crate::{Result, args, audit, health as rust_health};

const DEV_DYNAMIC_FEATURES: &[&str] = &["dev-dynamic-linking", "lk2-core/dev-dynamic-linking"];

fn default_gpu_backend() -> &'static str {
    if cfg!(windows) { "dx12" } else { "vulkan" }
}

#[derive(Debug)]
struct LoopArgs {
    seconds: u64,
    max_extra_wait: u64,
    rust_log: String,
    gpu_backend: String,
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

#[derive(Debug)]
struct PlayArgs {
    skip_build: bool,
    rust_log: String,
    gpu_backend: String,
    online: bool,
    server_addr: String,
    client_args: Vec<String>,
}

#[derive(Debug)]
struct FlickerProbeArgs {
    seconds: u64,
    interval: f64,
    period: f64,
    warmup: f64,
    warn_threshold: f64,
    fail_threshold: f64,
    skip_build: bool,
    rust_log: String,
    gpu_backend: String,
}

impl Default for LoopArgs {
    fn default() -> Self {
        Self {
            seconds: 60,
            max_extra_wait: 60,
            rust_log: env::var("RUST_LOG").unwrap_or_else(|_| {
                "info,lightyear_replication=debug,lightyear_connection=debug,lightyear_send=debug,lightyear_receive=debug".to_string()
            }),
            gpu_backend: env::var("WGPU_BACKEND")
                .unwrap_or_else(|_| default_gpu_backend().to_string()),
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

impl Default for PlayArgs {
    fn default() -> Self {
        Self {
            skip_build: false,
            rust_log: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            gpu_backend: env::var("WGPU_BACKEND")
                .unwrap_or_else(|_| default_gpu_backend().to_string()),
            online: false,
            server_addr: "127.0.0.1:5000".to_string(),
            client_args: vec!["--offline".to_string(), "--no-scenario".to_string()],
        }
    }
}

impl Default for FlickerProbeArgs {
    fn default() -> Self {
        Self {
            seconds: 20,
            interval: 0.2,
            period: 8.0,
            warmup: 5.0,
            warn_threshold: 20.0,
            fail_threshold: 35.0,
            skip_build: false,
            rust_log: env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
            gpu_backend: env::var("WGPU_BACKEND")
                .unwrap_or_else(|_| default_gpu_backend().to_string()),
        }
    }
}

pub fn run(root: &Path, raw: &[String]) -> Result<()> {
    if raw.iter().any(|a| a == "--help" || a == "-h" || a == "-?") {
        println!(
            "xtask loop --offline --seconds 12 --skip-build --no-dynamic --first-person --refresh-after-fail --gpu-backend vulkan"
        );
        return Ok(());
    }
    let parsed = parse_loop(raw);
    fs::create_dir_all(root.join("run-logs")).map_err(|e| e.to_string())?;
    fs::create_dir_all(root.join("screenshots")).map_err(|e| e.to_string())?;
    let _lock = LoopLock::acquire(root)?;
    reject_active_workspace_builds(root)?;
    enforce_decision_gate(root, parsed.refresh_after_fail)?;

    let use_offline = parsed.offline || (!parsed.online && !parsed.no_server);
    let target_dir = loop_target_dir(root);
    let mut envs = play_runtime_env(
        runtime_env(root, &target_dir, &parsed.rust_log)?,
        &parsed.gpu_backend,
    );
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

    let client_exe = exe_path(&target_dir, "lk2-client");
    let server_exe = exe_path(&target_dir, "lk2-server");
    if !parsed.skip_build || !client_exe.exists() {
        cargo_build_with_fallback(root, "lk2-client", &features, &envs)?;
    }
    if !use_offline && !parsed.no_server && (!parsed.skip_build || !server_exe.exists()) {
        cargo_build_with_fallback(root, "lk2-server", &features, &envs)?;
    }
    if !client_exe.exists() {
        return Err(format!("binary not found: {}", client_exe.display()));
    }
    ensure_binary_fresh(
        root,
        &client_exe,
        &[
            "crates/client/src/main.rs",
            "crates/client/src/capture.rs",
            "crates/client/src/pretty/mod.rs",
            "crates/client/src/render/mod.rs",
            "crates/core/src/clock.rs",
        ],
    )?;
    stage_windows_runtime_files(root, &target_dir)?;

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
    let health_result = ready
        .as_deref()
        .map(|iter| run_health_for_iter(root, iter))
        .unwrap_or_else(|| run_latest_health(root));
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

pub fn flicker_probe(root: &Path, raw: &[String]) -> Result<()> {
    if raw.iter().any(|a| a == "--help" || a == "-h" || a == "-?") {
        println!(
            "xtask flicker-probe --seconds 20 --interval 0.2 --period 8 --warmup 5 --gpu-backend dx12 --skip-build"
        );
        println!("  captures frame_*.png plus state_*.json under screenshots/flicker_probe/");
        println!("  fails when adjacent sampled luma delta exceeds the fail threshold");
        return Ok(());
    }

    let parsed = parse_flicker_probe(raw);
    fs::create_dir_all(root.join("run-logs")).map_err(|e| e.to_string())?;
    fs::create_dir_all(root.join("screenshots/flicker_probe")).map_err(|e| e.to_string())?;

    let target_dir = root.join("target");
    let mut envs = play_runtime_env(
        runtime_env(root, &target_dir, &parsed.rust_log)?,
        &parsed.gpu_backend,
    );
    let probe_dir = next_flicker_probe_dir(root, &parsed.gpu_backend)?;
    set_env_pair(&mut envs, "LK2_CAPTURE", "1");
    set_env_pair(
        &mut envs,
        "LK2_FLICKER_PROBE_DIR",
        &probe_dir.display().to_string(),
    );
    set_env_pair(
        &mut envs,
        "LK2_SCREENSHOT_INTERVAL",
        &parsed.interval.to_string(),
    );
    set_env_pair(
        &mut envs,
        "LK2_DAY_NIGHT_PERIOD_SECS",
        &parsed.period.to_string(),
    );

    let client_exe = exe_path(&target_dir, "lk2-client");
    if !parsed.skip_build || !client_exe.exists() {
        cargo_build_with_fallback(root, "lk2-client", &[], &envs)?;
    }
    if !client_exe.exists() {
        return Err(format!("binary not found: {}", client_exe.display()));
    }
    ensure_binary_fresh(
        root,
        &client_exe,
        &[
            "crates/client/src/main.rs",
            "crates/client/src/capture.rs",
            "crates/client/src/render/mod.rs",
        ],
    )?;
    stage_windows_runtime_files(root, &target_dir)?;

    stop_processes(&["lk2-client"]);
    thread::sleep(Duration::from_millis(500));

    let log = root.join("run-logs/flicker_probe.log");
    let client_args = vec!["--offline".to_string(), "--no-scenario".to_string()];
    println!(
        ">>> Running flicker probe for {}s; dir={}",
        parsed.seconds,
        audit::rel(root, &probe_dir)
    );
    let mut child = spawn_logged(root, &client_exe, &client_args, &envs, &log)?;
    let started = Instant::now();
    let mut process_status = None;
    while started.elapsed() < Duration::from_secs(parsed.seconds) {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            process_status = Some(status.to_string());
            break;
        }
        thread::sleep(Duration::from_millis(250));
    }
    if process_status.is_none() {
        stop_child(&mut child);
    }
    thread::sleep(Duration::from_millis(500));

    let stderr_path = PathBuf::from(format!("{}.err", log.display()));
    let stderr_tail = read_tail(&stderr_path, 80);
    let summary = analyze_flicker_probe(&probe_dir, &parsed, process_status.as_deref())?;
    write_flicker_probe_summary(&probe_dir, &summary, &stderr_tail)?;

    println!(
        ">>> Flicker probe {}: frames={} max_delta={} worst_pair={}",
        summary.get("verdict").and_then(Value::as_str).unwrap_or("UNKNOWN"),
        summary.get("frames").and_then(Value::as_u64).unwrap_or(0),
        summary.get("max_luma_delta").and_then(Value::as_f64).unwrap_or(0.0),
        summary
            .get("worst_pair")
            .and_then(Value::as_array)
            .map(|pair| { pair.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" -> ") })
            .unwrap_or_else(|| "none".to_string())
    );
    println!(
        ">>> Summary: {}",
        audit::rel(root, &probe_dir.join("summary.json"))
    );

    match summary.get("verdict").and_then(Value::as_str).unwrap_or("UNKNOWN") {
        "OK" | "WARN" => Ok(()),
        verdict => Err(format!(
            "flicker probe verdict {verdict}; see {}",
            audit::rel(root, &probe_dir.join("summary.json"))
        )),
    }
}

pub fn clean_runs(root: &Path) -> Result<()> {
    for rel in ["run-logs", "screenshots"] {
        let path = root.join(rel);
        ensure_clean_target(root, &path)?;
        if path.exists() {
            fs::remove_dir_all(&path)
                .map_err(|e| format!("failed to remove {}: {e}", path.display()))?;
            println!("removed {rel}/");
        } else {
            println!("already clean: {rel}/");
        }
    }
    Ok(())
}

pub fn play(root: &Path, raw: &[String]) -> Result<()> {
    if raw.iter().any(|a| a == "--help" || a == "-h" || a == "-?") {
        println!(
            "xtask play [--skip-build] [--gpu-backend vulkan|dx12] [--online] [--server-addr ADDR] [client flags...]"
        );
        println!("  default client flags: --offline --no-scenario");
        println!("  default gpu backend: {}", default_gpu_backend());
        println!("  logs: run-logs/play.log and run-logs/play.log.err");
        println!(
            "  online logs also include run-logs/play_server.log and run-logs/play_server.log.err"
        );
        return Ok(());
    }
    let parsed = parse_play(raw);
    let run_logs = root.join("run-logs");
    fs::create_dir_all(&run_logs).map_err(|e| e.to_string())?;

    let target_dir = root.join("target");
    let envs = play_runtime_env(
        runtime_env(root, &target_dir, &parsed.rust_log)?,
        &parsed.gpu_backend,
    );
    let client_exe = exe_path(&target_dir, "lk2-client");
    let server_exe = exe_path(&target_dir, "lk2-server");
    if !parsed.skip_build || !client_exe.exists() {
        cargo_build_with_fallback(root, "lk2-client", &[], &envs)?;
    }
    if parsed.online && (!parsed.skip_build || !server_exe.exists()) {
        cargo_build_with_fallback(root, "lk2-server", &[], &envs)?;
    }
    if !client_exe.exists() {
        return Err(format!("binary not found: {}", client_exe.display()));
    }
    if parsed.online && !server_exe.exists() {
        return Err(format!("binary not found: {}", server_exe.display()));
    }
    stage_default_windows_runtime_files(root)?;

    let mut server_proc = None;
    let server_log = run_logs.join("play_server.log");
    if parsed.online {
        println!(
            ">>> Starting play server; stdout={}",
            audit::rel(root, &server_log)
        );
        server_proc = Some(spawn_logged(root, &server_exe, &[], &envs, &server_log)?);
        thread::sleep(Duration::from_secs(2));
    }

    let log = run_logs.join("play.log");
    println!(
        ">>> Starting normal play; stdout={}",
        audit::rel(root, &log)
    );
    println!(
        ">>> Crash/error stderr will be written to {}",
        audit::rel(root, &PathBuf::from(format!("{}.err", log.display())))
    );
    let mut child = spawn_logged(root, &client_exe, &parsed.client_args, &envs, &log)?;
    let status =
        child.wait().map_err(|e| format!("failed to wait for {}: {e}", client_exe.display()))?;
    if let Some(mut child) = server_proc {
        stop_child(&mut child);
    }

    let mut archive_files = vec![log.clone(), PathBuf::from(format!("{}.err", log.display()))];
    if parsed.online {
        archive_files.push(server_log.clone());
        archive_files.push(PathBuf::from(format!("{}.err", server_log.display())));
    }
    let summary = rust_health::archive_error_logs_for_files(root, &run_logs, &archive_files)?;
    println!(
        ">>> Archived {} error lines from {} log files to run-logs/error_logs.json and run-logs/error_logs.txt",
        summary.error_line_count, summary.files_scanned
    );

    if status.success() {
        Ok(())
    } else {
        let stderr_tail = read_tail(&PathBuf::from(format!("{}.err", log.display())), 80);
        Err(format!(
            "lk2-client exited with {status}; see run-logs/play.log.err\nstderr tail:\n{stderr_tail}"
        ))
    }
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
    let target_dir = loop_target_dir(root);
    let envs = runtime_env(
        root,
        &target_dir,
        &env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
    )?;
    let client_exe = exe_path(&target_dir, "lk2-client");
    if !skip_build || !client_exe.exists() {
        cargo_build_with_fallback(root, "lk2-client", &[], &envs)?;
    }
    stage_windows_runtime_files(root, &target_dir)?;
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
            Some("gpubackend") | Some("gpu-backend") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.gpu_backend = value;
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

fn parse_flicker_probe(raw: &[String]) -> FlickerProbeArgs {
    let mut parsed = FlickerProbeArgs::default();
    let mut i = 0;
    while i < raw.len() {
        let (name, inline) = args::split_flag(&raw[i]);
        match name.as_deref() {
            Some("seconds") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.seconds = value.parse().unwrap_or(parsed.seconds);
                }
            }
            Some("interval") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.interval = value.parse().unwrap_or(parsed.interval);
                }
            }
            Some("period") | Some("daynightperiod") | Some("day-night-period") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.period = value.parse().unwrap_or(parsed.period);
                }
            }
            Some("warmup") | Some("warmupsecs") | Some("warmup-secs") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.warmup = value.parse().unwrap_or(parsed.warmup);
                }
            }
            Some("warnthreshold") | Some("warn-threshold") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.warn_threshold = value.parse().unwrap_or(parsed.warn_threshold);
                }
            }
            Some("threshold") | Some("failthreshold") | Some("fail-threshold") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.fail_threshold = value.parse().unwrap_or(parsed.fail_threshold);
                }
            }
            Some("skipbuild") | Some("skip-build") => parsed.skip_build = true,
            Some("rustlog") | Some("rust-log") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.rust_log = value;
                }
            }
            Some("gpubackend") | Some("gpu-backend") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.gpu_backend = value;
                }
            }
            _ => {}
        }
        i += 1;
    }
    parsed.interval = parsed.interval.max(0.05);
    parsed.period = parsed.period.max(0.2);
    parsed.warmup = parsed.warmup.max(0.0);
    parsed.warn_threshold = parsed.warn_threshold.max(0.0);
    parsed.fail_threshold = parsed.fail_threshold.max(parsed.warn_threshold);
    parsed
}

fn parse_play(raw: &[String]) -> PlayArgs {
    let mut parsed = PlayArgs::default();
    let mut client_args = Vec::new();
    let mut passthrough = false;
    let mut i = 0;
    while i < raw.len() {
        if passthrough {
            client_args.push(raw[i].clone());
            i += 1;
            continue;
        }
        if raw[i] == "--" {
            passthrough = true;
            i += 1;
            continue;
        }
        let (name, inline) = args::split_flag(&raw[i]);
        match name.as_deref() {
            Some("skipbuild") | Some("skip-build") => parsed.skip_build = true,
            Some("online") => parsed.online = true,
            Some("serveraddr") | Some("server-addr") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.server_addr = value;
                }
            }
            Some("rustlog") | Some("rust-log") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.rust_log = value;
                }
            }
            Some("gpubackend") | Some("gpu-backend") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    parsed.gpu_backend = value;
                }
            }
            _ => client_args.push(raw[i].clone()),
        }
        i += 1;
    }
    if parsed.online {
        let has_connect = client_args.iter().any(|arg| arg.starts_with("--connect"));
        parsed.client_args = if has_connect {
            client_args
        } else {
            let mut args = vec![format!("--connect={}", parsed.server_addr)];
            args.extend(client_args);
            args
        };
        return parsed;
    }
    if client_args.is_empty() {
        return parsed;
    }
    if client_args.iter().any(|arg| {
        arg == "--offline"
            || arg == "-Offline"
            || arg.starts_with("--connect")
            || arg.starts_with("-Connect")
            || arg == "--model-preview"
            || arg == "--terrain-preview"
    }) {
        parsed.client_args = client_args;
    } else {
        parsed.client_args.extend(client_args);
    }
    parsed
}

fn play_runtime_env(mut envs: Vec<(String, String)>, gpu_backend: &str) -> Vec<(String, String)> {
    set_env_pair(&mut envs, "WGPU_BACKEND", gpu_backend);
    envs
}

fn set_env_pair(envs: &mut Vec<(String, String)>, key: &str, value: &str) {
    if let Some((_, existing)) = envs.iter_mut().find(|(k, _)| k == key) {
        *existing = value.to_string();
    } else {
        envs.push((key.to_string(), value.to_string()));
    }
}

fn ensure_clean_target(root: &Path, path: &Path) -> Result<()> {
    let root_abs =
        root.canonicalize().map_err(|e| format!("failed to resolve {}: {e}", root.display()))?;
    let parent =
        path.parent().ok_or_else(|| format!("invalid cleanup path: {}", path.display()))?;
    let parent_abs = parent
        .canonicalize()
        .map_err(|e| format!("failed to resolve {}: {e}", parent.display()))?;
    if parent_abs != root_abs {
        return Err(format!(
            "refusing to clean outside workspace: {}",
            path.display()
        ));
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("invalid cleanup path: {}", path.display()))?;
    if name != "run-logs" && name != "screenshots" {
        return Err(format!(
            "refusing to clean unexpected path: {}",
            path.display()
        ));
    }
    Ok(())
}

pub fn loop_target_dir(root: &Path) -> PathBuf {
    root.join(".tmp").join("loop-target")
}

fn runtime_env(root: &Path, target_dir: &Path, rust_log: &str) -> Result<Vec<(String, String)>> {
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
    let debug = target_dir.join("debug");
    let mut path = [debug.join("deps"), debug]
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
    if let Ok(existing_path) = env::var("PATH") {
        if !existing_path.is_empty() {
            path.push(existing_path);
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
        (
            "CARGO_TARGET_DIR".to_string(),
            target_dir.display().to_string(),
        ),
        ("PATH".to_string(), path_str),
        (
            "WGPU_BACKEND".to_string(),
            default_gpu_backend().to_string(),
        ),
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
        Err(first) if is_msvc_stale_link_failure(&first.text) => {
            println!(">>> MSVC link failed for {package}; cleaning this package and retrying once");
            write_loop_diagnosis(
                root,
                &build_diagnosis(&first, "clean_package_and_retry_build"),
            )?;
            cargo_clean_package(root, package, envs)?;
            match cargo_build_once(root, package, features, envs) {
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
        Err(first) if is_windows_build_script_access_denied(&first.text) => {
            println!(
                ">>> build script execution was denied for {package}; clearing loop target and retrying once"
            );
            write_loop_diagnosis(
                root,
                &build_diagnosis(&first, "clear_loop_target_and_retry_build"),
            )?;
            clear_loop_target(root, envs)?;
            match cargo_build_once(root, package, features, envs) {
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
    cmd.args(["build", "-p", package, "-j", "1"]).current_dir(root);
    let joined;
    if !features.is_empty() {
        joined = features.join(",");
        cmd.args(["--features", &joined]);
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.env("CARGO_INCREMENTAL", "0");
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

fn ensure_binary_fresh(root: &Path, binary: &Path, sources: &[&str]) -> Result<()> {
    let binary_time = fs::metadata(binary)
        .and_then(|m| m.modified())
        .map_err(|e| format!("failed to stat {}: {e}", binary.display()))?;
    let mut stale_sources = Vec::new();
    for rel in sources {
        let path = root.join(rel);
        let Ok(source_time) = fs::metadata(&path).and_then(|m| m.modified()) else {
            continue;
        };
        if source_time > binary_time {
            stale_sources.push((*rel).to_string());
        }
    }
    if stale_sources.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "refusing to run stale {}; newer sources: {}. Rebuild completed binary before loop capture.",
            binary.display(),
            stale_sources.join(", ")
        ))
    }
}

#[derive(Debug)]
struct LoopLock {
    path: PathBuf,
}

impl LoopLock {
    fn acquire(root: &Path) -> Result<Self> {
        let dir = root.join(".tmp");
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("loop.lock");
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => Ok(Self { path }),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => Err(format!(
                "another xtask loop appears to be running (lock: {}). Stop it or remove the stale lock after verifying no loop/build process is active.",
                path.display()
            )),
            Err(err) => Err(format!(
                "failed to create loop lock {}: {err}",
                path.display()
            )),
        }
    }
}

impl Drop for LoopLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn reject_active_workspace_builds(root: &Path) -> Result<()> {
    #[cfg(not(windows))]
    {
        let _ = root;
        return Ok(());
    }
    #[cfg(windows)]
    {
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Get-CimInstance Win32_Process -Filter \"name = 'cargo.exe' or name = 'rustc.exe' or name = 'lk2-client.exe' or name = 'lk2-server.exe'\" | Select-Object ProcessId,ParentProcessId,Name,CommandLine | ConvertTo-Json -Compress",
            ])
            .output()
            .map_err(|e| format!("failed to query active build processes: {e}"))?;
        let text = String::from_utf8_lossy(&output.stdout);
        if has_active_workspace_process(root, &text) {
            return Err(
                "active cargo/rustc/lk2 client/server process detected for this workspace; wait for it to finish before running xtask loop".to_string(),
            );
        }
        Ok(())
    }
}

#[cfg(windows)]
fn has_active_workspace_process(root: &Path, process_json: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(process_json) else {
        return false;
    };
    let current = std::process::id() as u64;
    let mut rows = match value {
        serde_json::Value::Array(rows) => rows,
        serde_json::Value::Object(_) => vec![value],
        _ => return false,
    };
    let mut ancestor_ids = std::collections::HashSet::new();
    ancestor_ids.insert(current);
    loop {
        let mut changed = false;
        for row in &rows {
            let pid = row.get("ProcessId").and_then(serde_json::Value::as_u64).unwrap_or(0);
            let ppid = row.get("ParentProcessId").and_then(serde_json::Value::as_u64).unwrap_or(0);
            if ancestor_ids.contains(&pid) && ppid != 0 && ancestor_ids.insert(ppid) {
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    let root_text = root.display().to_string().to_ascii_lowercase();
    for row in rows.drain(..) {
        let pid = row.get("ProcessId").and_then(serde_json::Value::as_u64).unwrap_or(0);
        if ancestor_ids.contains(&pid) {
            continue;
        }
        let name =
            row.get("Name").and_then(serde_json::Value::as_str).unwrap_or("").to_ascii_lowercase();
        let command = row
            .get("CommandLine")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if name == "lk2-client.exe" || name == "lk2-server.exe" {
            return true;
        }
        if !command.contains(&root_text) {
            continue;
        }
        if command.contains(" lk2-client")
            || command.contains(" lk2_client")
            || command.contains(" lk2-server")
            || command.contains(" lk2_server")
            || command.contains("-p lk2-client")
            || command.contains("-p lk2-server")
            || command.contains("-p xtask -- loop")
        {
            return true;
        }
    }
    false
}

fn clear_loop_target(root: &Path, envs: &[(String, String)]) -> Result<()> {
    let Some(target_dir) =
        envs.iter().find(|(k, _)| k == "CARGO_TARGET_DIR").map(|(_, v)| PathBuf::from(v))
    else {
        return Err("CARGO_TARGET_DIR missing; refusing to clear unknown target dir".to_string());
    };
    let expected = loop_target_dir(root);
    if target_dir != expected {
        return Err(format!(
            "refusing to clear unexpected target dir {}; expected {}",
            target_dir.display(),
            expected.display()
        ));
    }
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)
            .map_err(|e| format!("failed to clear {}: {e}", target_dir.display()))?;
    }
    Ok(())
}

fn cargo_clean_package(root: &Path, package: &str, envs: &[(String, String)]) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(["clean", "-p", package]).current_dir(root);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let output =
        cmd.output().map_err(|e| format!("failed to run cargo clean -p {package}: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        let mut text = String::from_utf8_lossy(&output.stdout).to_string();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        Err(format!("cargo clean -p {package} failed:\n{text}"))
    }
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

fn is_msvc_stale_link_failure(text: &str) -> bool {
    text.contains("LNK2019")
        && text.contains("LNK1120")
        && (text.contains("bevy_ecs") || text.contains(".rcgu.o"))
}

fn is_windows_build_script_access_denied(text: &str) -> bool {
    text.contains("failed to run custom build command")
        && text.contains("build-script-build")
        && (text.contains("os error 5")
            || text.contains("Access is denied")
            || text.contains("拒绝访问"))
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
    } else if is_msvc_stale_link_failure(text) {
        "windows_msvc_stale_link_failure"
    } else if is_windows_build_script_access_denied(text) {
        "windows_build_script_access_denied"
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
    if let Err(err) = rust_health::archive_error_logs(root, &run_logs) {
        println!(">>> [warn] failed to archive error logs: {err}");
    }
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
    // authoritative sim), so the offline tick gate would never fire. Accept
    // either an offline tick at the health completion threshold OR an online iter that has run at least
    // 4 wall seconds and produced a >= 30KB screenshot.
    let role = json.get("role").and_then(Value::as_str).unwrap_or("");
    let tick_ok = json.get("tick").and_then(Value::as_i64).unwrap_or(0) >= 100;
    let wall_ok = json.get("wall_secs").and_then(Value::as_f64).unwrap_or(0.0) >= 4.0;
    if json.pointer("/visual/movement_probe/first_player_pos").is_none()
        || json.pointer("/visual/movement_probe/current_player_pos").is_none()
    {
        return false;
    }
    if json.pointer("/visual/player_readability/marker_count").is_none() {
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
    run_health_for_iter(root, &iter)
}

fn run_health_for_iter(root: &Path, iter: &Path) -> Result<()> {
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
    iter_dirs(root)
        .ok()?
        .into_iter()
        .filter(|p| iter_number(p).unwrap_or(0) > before)
        .rev()
        .find(|p| iter_ready(p))
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

fn exe_path(target_dir: &Path, name: &str) -> PathBuf {
    target_dir.join("debug").join(if cfg!(windows) {
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

pub fn stage_windows_runtime_files(root: &Path, target_dir: &Path) -> Result<()> {
    if !cfg!(windows) {
        return Ok(());
    }
    let debug = target_dir.join("debug");
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

pub fn stage_default_windows_runtime_files(root: &Path) -> Result<()> {
    stage_windows_runtime_files(root, &root.join("target"))
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

fn next_flicker_probe_dir(root: &Path, backend: &str) -> Result<PathBuf> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
    let backend = sanitize_path_segment(backend);
    let base =
        root.join("screenshots").join("flicker_probe").join(format!("probe_{stamp}_{backend}"));
    if !base.exists() {
        return Ok(base);
    }
    Ok(root
        .join("screenshots")
        .join("flicker_probe")
        .join(format!("probe_{stamp}_{backend}_{}", std::process::id())))
}

fn sanitize_path_segment(value: &str) -> String {
    let cleaned =
        value.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect::<String>();
    if cleaned.is_empty() {
        "default".to_string()
    } else {
        cleaned
    }
}

#[derive(Debug, Clone)]
struct FlickerFrameSample {
    name: String,
    luma_mean: f64,
    state: Option<Value>,
}

#[derive(Debug, Clone)]
struct FlickerSequenceStats {
    max_delta: f64,
    worst_pair: Option<(usize, usize)>,
    verdict: &'static str,
}

fn analyze_flicker_probe(
    probe_dir: &Path,
    args: &FlickerProbeArgs,
    process_status: Option<&str>,
) -> Result<Value> {
    let mut pngs = fs::read_dir(probe_dir)
        .map_err(|e| format!("failed to read {}: {e}", probe_dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|name| name.starts_with("frame_") && name.ends_with(".png"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    pngs.sort();

    let mut samples = Vec::new();
    let mut errors = Vec::new();
    for path in pngs {
        let name = path.file_name().and_then(OsStr::to_str).unwrap_or("frame.png").to_string();
        match decode_sampled_luma_mean(&path, 48) {
            Ok(luma_mean) => {
                let state = flicker_state_for_frame(probe_dir, &name);
                samples.push(FlickerFrameSample { name, luma_mean, state });
            }
            Err(err) => errors.push(json!({
                "frame": name,
                "error": err,
            })),
        }
    }

    let captured_frames = samples.len();
    let analyzed_samples = samples
        .iter()
        .filter(|sample| {
            flicker_sample_wall_secs(sample)
                .map(|wall_secs| wall_secs >= args.warmup)
                .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();
    let mut stats =
        analyze_flicker_sequence(&analyzed_samples, args.warn_threshold, args.fail_threshold);
    if process_status.is_some() && stats.verdict != "FLICKER" {
        stats.verdict = "CRASH";
    }
    if analyzed_samples.len() < 2 && stats.verdict == "OK" {
        stats.verdict = "INSUFFICIENT";
    }

    let (worst_pair, worst_states) = stats
        .worst_pair
        .map(|(from, to)| {
            (
                json!([analyzed_samples[from].name, analyzed_samples[to].name]),
                json!({
                    "from": analyzed_samples[from].state,
                    "to": analyzed_samples[to].state,
                    "from_luma_mean": round1_local(analyzed_samples[from].luma_mean),
                    "to_luma_mean": round1_local(analyzed_samples[to].luma_mean),
                }),
            )
        })
        .unwrap_or_else(|| (json!([]), json!(null)));

    Ok(json!({
        "verdict": stats.verdict,
        "backend": args.gpu_backend,
        "seconds": args.seconds,
        "interval": args.interval,
        "day_night_period_secs": args.period,
        "warmup_secs": args.warmup,
        "captured_frames": captured_frames,
        "frames": analyzed_samples.len(),
        "decode_errors": errors,
        "max_luma_delta": round1_local(stats.max_delta),
        "warn_threshold": args.warn_threshold,
        "fail_threshold": args.fail_threshold,
        "worst_pair": worst_pair,
        "worst_pair_state": worst_states,
        "process_status": process_status,
        "frames_sample": analyzed_samples.iter().take(8).map(|sample| json!({
            "frame": sample.name,
            "luma_mean": round1_local(sample.luma_mean),
            "state": sample.state,
        })).collect::<Vec<_>>(),
    }))
}

fn flicker_sample_wall_secs(sample: &FlickerFrameSample) -> Option<f64> {
    sample.state.as_ref().and_then(|state| state.get("wall_secs")).and_then(Value::as_f64)
}

fn analyze_flicker_sequence(
    samples: &[FlickerFrameSample],
    warn_threshold: f64,
    fail_threshold: f64,
) -> FlickerSequenceStats {
    let mut max_delta = 0.0;
    let mut worst_pair = None;
    for idx in 1..samples.len() {
        let delta = (samples[idx].luma_mean - samples[idx - 1].luma_mean).abs();
        if delta > max_delta {
            max_delta = delta;
            worst_pair = Some((idx - 1, idx));
        }
    }
    let verdict = if max_delta >= fail_threshold {
        "FLICKER"
    } else if max_delta >= warn_threshold {
        "WARN"
    } else {
        "OK"
    };
    FlickerSequenceStats { max_delta, worst_pair, verdict }
}

fn flicker_state_for_frame(probe_dir: &Path, frame_name: &str) -> Option<Value> {
    let id = frame_name.strip_prefix("frame_").and_then(|name| name.strip_suffix(".png"))?;
    let state_path = probe_dir.join(format!("state_{id}.json"));
    let text = fs::read_to_string(state_path).ok()?;
    let state = serde_json::from_str::<Value>(&text).ok()?;
    let lighting = state.get("render").and_then(|render| render.get("lighting")).cloned();
    Some(json!({
        "tick": state.get("tick").cloned(),
        "wall_secs": state.get("wall_secs").cloned(),
        "lighting": lighting,
    }))
}

fn write_flicker_probe_summary(probe_dir: &Path, summary: &Value, stderr_tail: &str) -> Result<()> {
    fs::create_dir_all(probe_dir).map_err(|e| e.to_string())?;
    fs::write(
        probe_dir.join("summary.json"),
        serde_json::to_string_pretty(summary).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    let verdict = summary.get("verdict").and_then(Value::as_str).unwrap_or("UNKNOWN");
    let frames = summary.get("frames").and_then(Value::as_u64).unwrap_or(0);
    let max_delta = summary.get("max_luma_delta").and_then(Value::as_f64).unwrap_or(0.0);
    let worst_pair = summary
        .get("worst_pair")
        .and_then(Value::as_array)
        .map(|pair| pair.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" -> "))
        .unwrap_or_default();
    let md = format!(
        "# flicker probe\n\nresult: {verdict}\nframes: {frames}\nmax_luma_delta: {max_delta}\nworst_pair: {worst_pair}\n\nstderr_tail:\n```text\n{stderr_tail}\n```\n"
    );
    fs::write(probe_dir.join("summary.md"), md).map_err(|e| e.to_string())?;
    Ok(())
}

fn decode_sampled_luma_mean(path: &Path, sample: usize) -> Result<f64> {
    let (w, h, pixels) = decode_png_rgb_local(path)?;
    if w == 0 || h == 0 || pixels.is_empty() {
        return Err("empty PNG".to_string());
    }
    let w = w as usize;
    let h = h as usize;
    let mut sum = 0.0;
    let mut count = 0.0;
    for sy in 0..sample {
        let y = ((sy as f64 + 0.5) * h as f64 / sample as f64).floor() as usize;
        for sx in 0..sample {
            let x = ((sx as f64 + 0.5) * w as f64 / sample as f64).floor() as usize;
            let [r, g, b] = pixels[y.min(h - 1) * w + x.min(w - 1)];
            sum += 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
            count += 1.0;
        }
    }
    Ok(sum / count)
}

fn decode_png_rgb_local(path: &Path) -> Result<(u32, u32, Vec<[u8; 3]>)> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let bytes = &buf[..info.buffer_size()];
    let mut pixels = Vec::with_capacity((info.width * info.height) as usize);
    match info.color_type {
        png::ColorType::Rgb => {
            for chunk in bytes.chunks_exact(3) {
                pixels.push([chunk[0], chunk[1], chunk[2]]);
            }
        }
        png::ColorType::Rgba => {
            for chunk in bytes.chunks_exact(4) {
                pixels.push([chunk[0], chunk[1], chunk[2]]);
            }
        }
        png::ColorType::Grayscale => {
            for v in bytes {
                pixels.push([*v, *v, *v]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for chunk in bytes.chunks_exact(2) {
                pixels.push([chunk[0], chunk[0], chunk[0]]);
            }
        }
        png::ColorType::Indexed => {
            return Err("indexed PNG is not supported by flicker probe".to_string());
        }
    }
    Ok((info.width, info.height, pixels))
}

fn round1_local(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
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
    fn loop_args_accept_gpu_backend_override() {
        let parsed = parse_loop(&["--gpu-backend=vulkan".into()]);

        assert_eq!(parsed.gpu_backend, "vulkan");
    }

    #[test]
    fn play_args_default_to_offline_no_scenario() {
        let parsed = parse_play(&[]);

        assert!(!parsed.skip_build);
        assert_eq!(parsed.gpu_backend, default_gpu_backend());
        assert_eq!(parsed.client_args, vec!["--offline", "--no-scenario"]);
    }

    #[test]
    fn play_args_accept_gpu_backend_override() {
        let parsed = parse_play(&["--gpu-backend=dx12".into()]);

        assert_eq!(parsed.gpu_backend, "dx12");
        assert_eq!(parsed.client_args, vec!["--offline", "--no-scenario"]);
    }

    #[test]
    fn play_args_keep_extra_client_flags_and_skip_build() {
        let parsed = parse_play(&[
            "--skip-build".into(),
            "--first-person".into(),
            "--legacy-voxel".into(),
        ]);

        assert!(parsed.skip_build);
        assert_eq!(
            parsed.client_args,
            vec![
                "--offline",
                "--no-scenario",
                "--first-person",
                "--legacy-voxel"
            ]
        );
    }

    #[test]
    fn play_args_do_not_force_offline_when_connecting() {
        let parsed = parse_play(&["--connect=127.0.0.1:5000".into()]);

        assert_eq!(parsed.client_args, vec!["--connect=127.0.0.1:5000"]);
    }

    #[test]
    fn play_args_online_starts_server_and_connects_client() {
        let parsed = parse_play(&["--online".into(), "--first-person".into()]);

        assert!(parsed.online);
        assert_eq!(
            parsed.client_args,
            vec!["--connect=127.0.0.1:5000", "--first-person"]
        );
    }

    #[test]
    fn play_args_online_accepts_custom_server_addr() {
        let parsed = parse_play(&[
            "--online".into(),
            "--server-addr".into(),
            "127.0.0.1:6000".into(),
        ]);

        assert!(parsed.online);
        assert_eq!(parsed.server_addr, "127.0.0.1:6000");
        assert_eq!(parsed.client_args, vec!["--connect=127.0.0.1:6000"]);
    }

    #[test]
    fn flicker_probe_args_parse_thresholds_and_backend() {
        let parsed = parse_flicker_probe(&[
            "--seconds=12".into(),
            "--interval".into(),
            "0.25".into(),
            "--period=6".into(),
            "--warmup=4.5".into(),
            "--warn-threshold=10".into(),
            "--threshold=30".into(),
            "--gpu-backend=vulkan".into(),
            "--skip-build".into(),
        ]);

        assert_eq!(parsed.seconds, 12);
        assert_eq!(parsed.interval, 0.25);
        assert_eq!(parsed.period, 6.0);
        assert_eq!(parsed.warmup, 4.5);
        assert_eq!(parsed.warn_threshold, 10.0);
        assert_eq!(parsed.fail_threshold, 30.0);
        assert_eq!(parsed.gpu_backend, "vulkan");
        assert!(parsed.skip_build);
    }

    #[test]
    fn flicker_sequence_finds_worst_adjacent_delta() {
        let samples = vec![
            FlickerFrameSample { name: "frame_0001.png".into(), luma_mean: 80.0, state: None },
            FlickerFrameSample { name: "frame_0002.png".into(), luma_mean: 84.0, state: None },
            FlickerFrameSample { name: "frame_0003.png".into(), luma_mean: 121.5, state: None },
        ];

        let stats = analyze_flicker_sequence(&samples, 20.0, 35.0);

        assert_eq!(stats.verdict, "FLICKER");
        assert_eq!(round1_local(stats.max_delta), 37.5);
        assert_eq!(stats.worst_pair, Some((1, 2)));
    }

    #[test]
    fn flicker_sequence_warns_below_fail_threshold() {
        let samples = vec![
            FlickerFrameSample { name: "frame_0001.png".into(), luma_mean: 80.0, state: None },
            FlickerFrameSample { name: "frame_0002.png".into(), luma_mean: 104.0, state: None },
        ];

        let stats = analyze_flicker_sequence(&samples, 20.0, 35.0);

        assert_eq!(stats.verdict, "WARN");
        assert_eq!(round1_local(stats.max_delta), 24.0);
    }

    #[test]
    fn sanitize_path_segment_keeps_probe_dir_portable() {
        assert_eq!(sanitize_path_segment("dx12"), "dx12");
        assert_eq!(sanitize_path_segment("vulkan/dev"), "vulkan_dev");
        assert_eq!(sanitize_path_segment(""), "default");
    }

    #[test]
    fn set_env_pair_replaces_existing_value() {
        let mut envs = vec![("WGPU_BACKEND".to_string(), "vulkan".to_string())];

        set_env_pair(&mut envs, "WGPU_BACKEND", "dx12");

        assert_eq!(envs, vec![("WGPU_BACKEND".to_string(), "dx12".to_string())]);
    }

    #[test]
    fn play_runtime_env_uses_requested_gpu_backend() {
        let envs = play_runtime_env(
            vec![("WGPU_BACKEND".to_string(), "vulkan".to_string())],
            "dx12",
        );

        assert_eq!(
            envs.iter().find(|(k, _)| k == "WGPU_BACKEND").map(|(_, v)| v.as_str()),
            Some("dx12")
        );
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
    fn classifies_msvc_stale_link_failure() {
        let (kind, errors, files) = classify_build_failure(
            "lk2_client.e2lg.rcgu.o : error LNK2019: unresolved external symbol bevy_ecs::entity::clone_entities\nF:\\rustProject\\lastkingdom2\\target\\debug\\deps\\lk2_client.exe : fatal error LNK1120: 441 unresolved externals",
        );

        assert_eq!(kind, "windows_msvc_stale_link_failure");
        assert!(errors.is_empty());
        assert!(files.is_empty());
    }

    #[test]
    fn classifies_windows_build_script_access_denied() {
        let (kind, errors, files) = classify_build_failure(
            "error: failed to run custom build command for `parking_lot_core v0.9.12`\n\nCaused by:\n  could not execute process `F:\\rustProject\\lastkingdom2\\.tmp\\loop-target\\debug\\build\\parking_lot_core\\build-script-build` (never executed)\n\nCaused by:\n  Access is denied. (os error 5)",
        );

        assert_eq!(kind, "windows_build_script_access_denied");
        assert!(errors.is_empty());
        assert!(files.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn active_process_filter_ignores_xtask_launch_parent() {
        let root = PathBuf::from("F:\\rustProject\\lastkingdom2");
        let current = std::process::id();
        let parent = current + 1000;
        let json = serde_json::json!([
            {
                "ProcessId": current,
                "ParentProcessId": parent,
                "Name": "xtask.exe",
                "CommandLine": "xtask.exe loop --offline"
            },
            {
                "ProcessId": parent,
                "ParentProcessId": 1,
                "Name": "cargo.exe",
                "CommandLine": "cargo run -p xtask -- loop --offline"
            }
        ])
        .to_string();

        assert!(!has_active_workspace_process(&root, &json));
    }

    #[cfg(windows)]
    #[test]
    fn active_process_filter_detects_other_lk2_client() {
        let root = PathBuf::from("F:\\rustProject\\lastkingdom2");
        let current = std::process::id();
        let json = serde_json::json!([
            {
                "ProcessId": current,
                "ParentProcessId": 1,
                "Name": "xtask.exe",
                "CommandLine": "xtask.exe loop --offline"
            },
            {
                "ProcessId": current + 2000,
                "ParentProcessId": 1,
                "Name": "lk2-client.exe",
                "CommandLine": "F:\\rustProject\\lastkingdom2\\target\\debug\\lk2-client.exe --offline"
            }
        ])
        .to_string();

        assert!(has_active_workspace_process(&root, &json));
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
    fn clean_runs_removes_only_runtime_outputs() {
        let root = temp_root("xtask_clean_runs");
        fs::create_dir_all(root.join("run-logs")).unwrap();
        fs::create_dir_all(root.join("screenshots/iter_1")).unwrap();
        fs::write(root.join("run-logs/play.log"), "log").unwrap();
        fs::write(root.join("screenshots/iter_1/health.json"), "{}").unwrap();
        fs::write(root.join("keep.txt"), "keep").unwrap();

        clean_runs(&root).unwrap();

        assert!(!root.join("run-logs").exists());
        assert!(!root.join("screenshots").exists());
        assert!(root.join("keep.txt").exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn loop_lock_rejects_concurrent_acquire() {
        let root = temp_root("xtask_loop_lock");
        let first = LoopLock::acquire(&root).expect("first lock should acquire");
        let err = LoopLock::acquire(&root).unwrap_err();
        assert!(
            err.contains("another xtask loop appears to be running"),
            "got: {err}"
        );
        drop(first);
        assert!(LoopLock::acquire(&root).is_ok());
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
        fs::write(
            iter.join("final_state.json"),
            r#"{"role":"client_offline","tick":20,"wall_secs":5.0,"visual":{"movement_probe":{"first_player_pos":[1,2,3],"current_player_pos":[2,2,3]}}}"#,
        )
        .unwrap();
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
            r#"{"role":"client_offline","tick":500,"wall_secs":5.0,"visual":{"movement_probe":{"first_player_pos":[1,2,3],"current_player_pos":[2,2,3]},"player_readability":{"marker_count":2}}}"#,
        )
        .unwrap();
        assert_eq!(next_ready_iter(&root, 1), Some(iter));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn next_ready_iter_uses_newest_ready_iter_not_newest_directory() {
        let root = temp_root("xtask_newest_ready_iter");
        let _ = fs::remove_dir_all(&root);
        let screenshots = root.join("screenshots");
        fs::create_dir_all(&screenshots).unwrap();

        let ready = screenshots.join("iter_2");
        fs::create_dir_all(&ready).unwrap();
        fs::write(
            ready.join("final_state.json"),
            r#"{"role":"client_offline","tick":100,"wall_secs":5.0,"visual":{"movement_probe":{"first_player_pos":[1,2,3],"current_player_pos":[2,2,3]},"player_readability":{"marker_count":2}}}"#,
        )
        .unwrap();
        fs::write(ready.join("iter_2.png"), vec![1u8; 40 * 1024]).unwrap();

        let incomplete = screenshots.join("iter_3");
        fs::create_dir_all(&incomplete).unwrap();
        fs::write(
            incomplete.join("final_state.json"),
            r#"{"role":"client_offline","tick":100,"wall_secs":5.0,"visual":{"movement_probe":{"first_player_pos":[1,2,3],"current_player_pos":[2,2,3]},"player_readability":{"marker_count":2}}}"#,
        )
        .unwrap();
        fs::write(incomplete.join("iter_3.png"), vec![1u8; 72]).unwrap();

        assert_eq!(next_ready_iter(&root, 1), Some(ready));
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
