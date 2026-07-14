use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{self, File},
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use crate::{
    Result,
    artifacts::nature::{AssertionSeverity, NatureArtifact, write_nature_artifact},
    assertions::nature::{NatureExpectations, evaluate_nature},
    observer::nature::{mark_stale_against, observe_value, scan_error_logs},
};

const LUMA_VAR_HEALTHY: f64 = 100.0;
const LUMA_MEAN_WHITE: f64 = 200.0;
const LUMA_MEAN_BLACK: f64 = 20.0;
const PNG_MIN_SIZE_KB: f64 = 30.0;
const SIM_TICK_EARLY: i64 = 30;
const SIM_TICK_COMPLETE: i64 = 100;
const SAMPLE_SIZE: usize = 64;
const PNG_MIN_W: u32 = 640;
const PNG_MIN_H: u32 = 360;
const TOP_COLOR_BUCKET_WARN_PCT: f64 = 92.0;
const LUMA_MEAN_FLICKER_WARN_DELTA: f64 = 35.0;
const FRAME_DT_OVER_50MS_WARN: i64 = 10;
const SMOOTH_MESH_MAX_WARN_MS: f64 = 60.0;
const SMOOTH_MESH_BUILDS_WARN: i64 = 3;
const TERRAIN_DESPAWNS_WARN: i64 = 6;
const FIRST_PERSON_EYE_MAX_Y: f64 = 80.0;
const RESOURCE_DELTAS_MIN_NONTICK: usize = 1;
const MONSTERS_DROP_PARTIAL_RATIO: f64 = 0.50;
const PLAYER_READABILITY_MIN_MARKERS: i64 = 2;
const PLAYER_READABILITY_MAX_MARKER_DISTANCE: f64 = 3.0;
const TOP_DOWN_CENTER_LEAVES_MIN_DISTANCE: f64 = 10.0;

const STDERR_KW_DESERIALIZE_INVALID: &str = "Attempting to deserialize an invalid entity";
const STDERR_KW_OUT_OF_BOUNDS: &str = "OUT OF BOUNDS";
const STDERR_KW_VOXEL_OVERFLOW: &str = "体素过多";
const STDERR_DESER_INVALID_FAIL_THRESHOLD: i64 = 1;
const STDERR_OUT_OF_BOUNDS_FAIL_THRESHOLD: i64 = 5;
const STDERR_VOXEL_OVERFLOW_FAIL_THRESHOLD: i64 = 5;

pub struct Evaluation {
    pub verdict: String,
    pub summary: String,
}

#[derive(Clone)]
struct Assertion {
    id: String,
    ok: bool,
    severity: String,
    actual: Value,
    op: String,
    expected: Value,
    message: String,
    path: Option<String>,
}

impl Assertion {
    fn json(&self) -> Value {
        let mut out = json!({
            "id": self.id,
            "ok": self.ok,
            "severity": self.severity,
            "actual": self.actual,
            "op": self.op,
            "expected": self.expected,
            "message": self.message,
        });
        if let Some(path) = &self.path {
            out["path"] = json!(path);
        }
        out
    }
}

pub fn evaluate_iter(root: &Path, iter_dir: &Path, prev_dir: Option<&Path>) -> Result<Evaluation> {
    let png_path = primary_png(iter_dir);
    let png = if let Some(path) = png_path {
        analyze_png(&path)
    } else {
        json!({"file_kb":0.0,"verdict":"NA","sub":null})
    };
    let prev_png = prev_dir.and_then(|dir| primary_png(dir).map(|path| analyze_png(&path)));

    let prev_state = prev_dir
        .and_then(|dir| fs::read_to_string(dir.join("final_state.json")).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let (sim, state) = analyze_sim(&iter_dir.join("final_state.json"), prev_state.as_ref());
    let client_tick = state
        .as_ref()
        .and_then(|value| path_value(value, "tick"))
        .and_then(Value::as_u64);
    let server_state = latest_server_state(root, client_tick);
    if let Some(server_state) = &server_state {
        fs::write(
            iter_dir.join("server_state.json"),
            serde_json::to_string_pretty(server_state).map_err(|error| error.to_string())? + "\n",
        )
        .map_err(|error| format!("write server_state.json: {error}"))?;
    }
    let stderr = scan_stderr_logs(root);
    let error_logs = archive_error_logs(root, iter_dir)?;
    let mut assertions = built_in_assertions(
        &png,
        prev_png.as_ref(),
        &sim,
        state.as_ref(),
        prev_state.as_ref(),
        &stderr,
        iter_dir,
        server_state.as_ref(),
    );
    let nature = state
        .as_ref()
        .map(|state| build_nature_artifact(root, iter_dir, state, prev_state.as_ref()));
    if let Some(artifact) = &nature {
        assertions.extend(artifact.assertions.iter().map(|item| Assertion {
            id: item.id.clone(),
            ok: item.ok,
            severity: match item.severity {
                AssertionSeverity::Fail => "fail".to_owned(),
                AssertionSeverity::Partial => "partial".to_owned(),
            },
            actual: item.actual.clone(),
            op: item.op.clone(),
            expected: item.expected.clone(),
            message: item.message.clone(),
            path: item.path.clone(),
        }));
        write_nature_artifact(iter_dir, artifact)?;
    }
    let combo = combine(&png, &sim, &assertions);
    let name = iter_dir
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("iter");
    let summary = summarize(name, &png, &sim, &combo);

    let failed = assertions.iter().filter(|a| !a.ok).count();
    let hard_failed = assertions
        .iter()
        .filter(|a| !a.ok && a.severity == "fail")
        .count();
    let partial_failed = assertions
        .iter()
        .filter(|a| !a.ok && a.severity != "fail")
        .count();
    let health = json!({
        "iter": name,
        "png": png,
        "prev_png": prev_png,
        "sim": sim,
        "stderr": {
            "deserialize_invalid_count": stderr.deserialize_invalid_count,
            "out_of_bounds_count": stderr.out_of_bounds_count,
            "voxel_overflow_count": stderr.voxel_overflow_count,
            "files_scanned": stderr.files_scanned,
        },
        "error_logs": {
            "error_line_count": error_logs.error_line_count,
            "files_scanned": error_logs.files_scanned,
            "json": "error_logs.json",
            "text": "error_logs.txt",
        },
        "assertions": {
            "total": assertions.len(),
            "failed": failed,
            "hard_failed": hard_failed,
            "partial_failed": partial_failed,
        },
        "score": combo["score"],
        "verdict": combo["verdict"],
        "reasons": combo["reasons"],
        "nature": nature.as_ref().map(NatureArtifact::health_extension),
        "server_state": server_state_summary(server_state.as_ref(), state.as_ref()),
    });
    fs::write(
        iter_dir.join("assertions.json"),
        serde_json::to_string_pretty(&json!({
            "iter": name,
            "assertions": assertions.iter().map(Assertion::json).collect::<Vec<_>>(),
        }))
        .map_err(|e| e.to_string())?
            + "\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        iter_dir.join("health.json"),
        serde_json::to_string(&health).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    fs::write(iter_dir.join("health.txt"), summary.as_bytes()).map_err(|e| e.to_string())?;

    write_regression_json(iter_dir, prev_dir, &assertions)?;

    Ok(Evaluation {
        verdict: combo["verdict"].as_str().unwrap_or("FAIL").to_string(),
        summary,
    })
}

fn build_nature_artifact(
    root: &Path,
    iter_dir: &Path,
    state: &Value,
    prev_state: Option<&Value>,
) -> NatureArtifact {
    let client_errors = root.join("screenshots/loop_run.log.err");
    let server_errors = root.join("screenshots/loop_server.log.err");
    let errors = scan_error_logs(&[client_errors.as_path(), server_errors.as_path()]);
    let mut after = observe_value(state, "final_state.json", errors);
    let initial = fs::read_to_string(iter_dir.join("nature_initial.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let before_value = initial.as_ref().or(prev_state);
    let before = before_value.map(|value| observe_value(value, "nature_initial", Vec::new()));
    if let Some(previous) = before.as_ref() {
        mark_stale_against(previous, &mut after);
    }
    let expectations = NatureExpectations {
        require_determinism: false,
        require_causal_progress: before.is_some(),
        require_presentation_consistency: true,
        maximum_error_lines: 0,
    };
    let assertions = evaluate_nature(before.as_ref(), &after, None, &expectations);
    NatureArtifact::new(
        "runtime_nature_loop".to_owned(),
        0,
        before,
        after,
        None,
        assertions,
    )
}

#[derive(Default, Clone, Debug)]
struct StderrScan {
    deserialize_invalid_count: i64,
    out_of_bounds_count: i64,
    voxel_overflow_count: i64,
    files_scanned: usize,
}

#[derive(Default, Clone, Debug)]
pub(crate) struct ErrorLogSummary {
    pub(crate) error_line_count: usize,
    pub(crate) files_scanned: usize,
}

#[derive(Clone, Debug)]
struct ErrorLogEntry {
    source: String,
    line: usize,
    text: String,
}

pub(crate) fn archive_error_logs(root: &Path, iter_dir: &Path) -> Result<ErrorLogSummary> {
    fs::create_dir_all(iter_dir).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    let mut files_scanned = 0;
    let mut dirs: Vec<PathBuf> = Vec::new();
    for rel in ["screenshots", "run-logs"] {
        let p = root.join(rel);
        if p.exists() {
            dirs.push(p);
        }
    }
    for dir in dirs {
        collect_error_logs_dir(root, &dir, &mut entries, &mut files_scanned);
    }
    write_error_log_archive(iter_dir, entries, files_scanned)
}

pub(crate) fn archive_error_logs_for_files(
    root: &Path,
    out_dir: &Path,
    files: &[PathBuf],
) -> Result<ErrorLogSummary> {
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    let mut files_scanned = 0;
    for path in files {
        collect_error_log_file(root, path, &mut entries, &mut files_scanned);
    }
    write_error_log_archive(out_dir, entries, files_scanned)
}

fn write_error_log_archive(
    out_dir: &Path,
    mut entries: Vec<ErrorLogEntry>,
    files_scanned: usize,
) -> Result<ErrorLogSummary> {
    entries.sort_by(|a, b| a.source.cmp(&b.source).then(a.line.cmp(&b.line)));

    let json_entries = entries
        .iter()
        .map(|entry| {
            json!({
                "source": entry.source,
                "line": entry.line,
                "text": entry.text,
            })
        })
        .collect::<Vec<_>>();
    let payload = json!({
        "error_line_count": entries.len(),
        "files_scanned": files_scanned,
        "entries": json_entries,
    });
    fs::write(
        out_dir.join("error_logs.json"),
        serde_json::to_string_pretty(&payload).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;

    let mut text = String::new();
    text.push_str(&format!(
        "error_line_count={} files_scanned={}\n",
        entries.len(),
        files_scanned
    ));
    for entry in &entries {
        text.push_str(&format!(
            "{}:{}: {}\n",
            entry.source, entry.line, entry.text
        ));
    }
    fs::write(out_dir.join("error_logs.txt"), text).map_err(|e| e.to_string())?;

    Ok(ErrorLogSummary {
        error_line_count: entries.len(),
        files_scanned,
    })
}

fn collect_error_logs_dir(
    root: &Path,
    dir: &Path,
    entries: &mut Vec<ErrorLogEntry>,
    files_scanned: &mut usize,
) {
    let Ok(read_dir) = fs::read_dir(dir) else {
        return;
    };
    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_error_logs_dir(root, &path, entries, files_scanned);
            continue;
        }
        if !is_log_file(&path) {
            continue;
        }
        collect_error_log_file(root, &path, entries, files_scanned);
    }
}

fn collect_error_log_file(
    root: &Path,
    path: &Path,
    entries: &mut Vec<ErrorLogEntry>,
    files_scanned: &mut usize,
) {
    if !is_log_file(path) {
        return;
    }
    let Ok(text) = fs::read_to_string(path) else {
        return;
    };
    *files_scanned += 1;
    let source = rel_path(root, path);
    for (idx, line) in text.lines().enumerate() {
        if is_error_log_line(line) {
            entries.push(ErrorLogEntry {
                source: source.clone(),
                line: idx + 1,
                text: line.to_string(),
            });
        }
    }
}

fn is_log_file(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(OsStr::to_str) else {
        return false;
    };
    name.ends_with(".log")
        || name.ends_with(".err")
        || name.ends_with(".log.err")
        || name.ends_with(".err.log")
}

fn is_error_log_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("error")
        || lower.contains("panic")
        || lower.contains("panicked")
        || lower.contains("fatal")
}

fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn scan_stderr_logs(root: &Path) -> StderrScan {
    let mut scan = StderrScan::default();
    let mut dirs: Vec<PathBuf> = Vec::new();
    for rel in ["screenshots", "run-logs", ".harness/scratch"] {
        let p = root.join(rel);
        if p.exists() {
            dirs.push(p);
        }
    }
    for dir in dirs {
        scan_stderr_dir(&dir, &mut scan);
    }
    scan
}

fn scan_stderr_dir(dir: &Path, scan: &mut StderrScan) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_stderr_dir(&path, scan);
            continue;
        }
        let Some(name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        let is_err_log = name.ends_with(".err.log") || name.ends_with(".log.err");
        if !is_err_log {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        scan.files_scanned += 1;
        for line in text.lines() {
            if line.contains(STDERR_KW_DESERIALIZE_INVALID) {
                scan.deserialize_invalid_count += 1;
            }
            if line.contains(STDERR_KW_OUT_OF_BOUNDS) {
                scan.out_of_bounds_count += 1;
            }
            if line.contains(STDERR_KW_VOXEL_OVERFLOW) {
                scan.voxel_overflow_count += 1;
            }
        }
    }
}

fn write_regression_json(
    iter_dir: &Path,
    prev_dir: Option<&Path>,
    current: &[Assertion],
) -> Result<()> {
    let Some(prev_dir) = prev_dir else {
        return Ok(());
    };
    let prev_path = prev_dir.join("assertions.json");
    let Ok(text) = fs::read_to_string(&prev_path) else {
        return Ok(());
    };
    let Ok(prev_json) = serde_json::from_str::<Value>(&text) else {
        return Ok(());
    };
    let Some(prev_assertions) = prev_json.get("assertions").and_then(Value::as_array) else {
        return Ok(());
    };
    let prev_failed: std::collections::HashSet<String> = prev_assertions
        .iter()
        .filter_map(|a| {
            let id = a.get("id").and_then(Value::as_str)?;
            let ok = a.get("ok").and_then(Value::as_bool).unwrap_or(true);
            if !ok { Some(id.to_string()) } else { None }
        })
        .collect();
    let mut newly_failed: Vec<&Assertion> = Vec::new();
    let mut newly_passed: Vec<&Assertion> = Vec::new();
    let mut still_failing: Vec<&Assertion> = Vec::new();
    for a in current {
        if !a.ok {
            if prev_failed.contains(&a.id) {
                still_failing.push(a);
            } else {
                newly_failed.push(a);
            }
        } else if prev_failed.contains(&a.id) {
            newly_passed.push(a);
        }
    }
    let regression = json!({
        "iter": iter_dir.file_name().and_then(OsStr::to_str).unwrap_or("iter"),
        "newly_failed": newly_failed.iter().map(|a| json!({"id": a.id, "severity": a.severity, "message": a.message})).collect::<Vec<_>>(),
        "still_failing": still_failing.iter().map(|a| json!({"id": a.id, "severity": a.severity, "message": a.message})).collect::<Vec<_>>(),
        "newly_passed": newly_passed.iter().map(|a| json!({"id": a.id})).collect::<Vec<_>>(),
    });
    fs::write(
        iter_dir.join("regression.json"),
        serde_json::to_string_pretty(&regression).map_err(|e| e.to_string())? + "\n",
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn primary_png(iter_dir: &Path) -> Option<PathBuf> {
    let mut pngs = fs::read_dir(iter_dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|name| name.starts_with("iter_") && name.ends_with(".png"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    pngs.sort();
    pngs.into_iter().next()
}

fn analyze_png(path: &Path) -> Value {
    let file_kb = fs::metadata(path)
        .map(|m| (m.len() as f64 / 1024.0 * 10.0).round() / 10.0)
        .unwrap_or(0.0);
    let mut out = json!({"file_kb": file_kb, "verdict": "NA", "sub": null});
    match decode_png_rgb(path) {
        Ok((w, h, pixels)) => {
            let sampled = sample_pixels(&pixels, w as usize, h as usize, SAMPLE_SIZE);
            let (luma_mean, luma_var, top_pct) = image_stats(&sampled);
            out["w"] = json!(w);
            out["h"] = json!(h);
            out["luma_mean"] = json!(round1(luma_mean));
            out["luma_var"] = json!(round1(luma_var));
            out["top_pct"] = json!(round1(top_pct));
            if luma_var < LUMA_VAR_HEALTHY {
                out["verdict"] = json!("UNIFORM");
                out["sub"] = if luma_mean > LUMA_MEAN_WHITE {
                    json!("WHITE")
                } else if luma_mean < LUMA_MEAN_BLACK {
                    json!("BLACK")
                } else {
                    json!("GRAY")
                };
            } else {
                out["verdict"] = json!("OK");
            }
        }
        Err(err) => {
            out["verdict"] = json!("ERR");
            out["err"] = json!(err.chars().take(120).collect::<String>());
        }
    }
    out
}

fn decode_png_rgb(path: &Path) -> Result<(u32, u32, Vec<[u8; 3]>)> {
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
            return Err("indexed PNG is not supported by xtask health".to_string());
        }
    }
    Ok((info.width, info.height, pixels))
}

fn sample_pixels(pixels: &[[u8; 3]], w: usize, h: usize, sample: usize) -> Vec<[u8; 3]> {
    let mut out = Vec::with_capacity(sample * sample);
    for sy in 0..sample {
        let y = ((sy as f64 + 0.5) * h as f64 / sample as f64).floor() as usize;
        for sx in 0..sample {
            let x = ((sx as f64 + 0.5) * w as f64 / sample as f64).floor() as usize;
            out.push(pixels[y.min(h - 1) * w + x.min(w - 1)]);
        }
    }
    out
}

fn image_stats(pixels: &[[u8; 3]]) -> (f64, f64, f64) {
    let mut luma_sum = 0.0;
    let mut luma_sq_sum = 0.0;
    let mut buckets: HashMap<u16, usize> = HashMap::new();
    for [r, g, b] in pixels {
        let luma = 0.299 * *r as f64 + 0.587 * *g as f64 + 0.114 * *b as f64;
        luma_sum += luma;
        luma_sq_sum += luma * luma;
        let key = ((*r as u16 >> 5) << 10) | ((*g as u16 >> 5) << 5) | (*b as u16 >> 5);
        *buckets.entry(key).or_insert(0) += 1;
    }
    let n = pixels.len() as f64;
    let mean = luma_sum / n;
    let var = luma_sq_sum / n - mean * mean;
    let top_pct = buckets.values().copied().max().unwrap_or(0) as f64 / n * 100.0;
    (mean, var, top_pct)
}

fn analyze_sim(state_path: &Path, prev_state: Option<&Value>) -> (Value, Option<Value>) {
    let mut out = json!({
        "verdict": "DEAD",
        "tick": null,
        "nations": null,
        "anomalies": null,
        "invariant_violations": null,
    });
    let Ok(text) = fs::read_to_string(state_path) else {
        return (out, None);
    };
    let Ok(state) = serde_json::from_str::<Value>(&text) else {
        out["err"] = json!("final_state.json parse error");
        return (out, None);
    };
    let tick = path_i64(&state, "tick").unwrap_or(0);
    let role = state.get("role").and_then(Value::as_str).unwrap_or("");
    let wall_secs = path_value(&state, "wall_secs")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let nations = path_i64(&state, "nations.total_nations").unwrap_or(0);
    let anomalies = path_i64(&state, "observer.anomalies").unwrap_or(0);
    let invariant_violations = path_i64(&state, "observer.invariant_violations").unwrap_or(0);
    out["tick"] = json!(tick);
    out["nations"] = json!(nations);
    out["anomalies"] = json!(anomalies);
    out["invariant_violations"] = json!(invariant_violations);
    out["verdict"] = if role == "client_online" && wall_secs >= 30.0 {
        json!("COMPLETE")
    } else if tick >= SIM_TICK_COMPLETE {
        json!("COMPLETE")
    } else if tick < SIM_TICK_EARLY {
        json!("EARLY")
    } else {
        json!("RUNNING")
    };
    if role != "client_online"
        && let Some(prev) = prev_state
    {
        if tick > 0 && tick < SIM_TICK_COMPLETE && path_i64(prev, "tick").unwrap_or(0) == tick {
            out["verdict"] = json!("STUCK");
            out["stuck_at"] = json!(tick);
        }
    }
    (out, Some(state))
}

fn built_in_assertions(
    png: &Value,
    prev_png: Option<&Value>,
    sim: &Value,
    state: Option<&Value>,
    prev_state: Option<&Value>,
    stderr: &StderrScan,
    iter_dir: &Path,
    server_state: Option<&Value>,
) -> Vec<Assertion> {
    let role = state
        .and_then(|s| s.get("role"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let online_client = role == "client_online";
    let progress_actual = if online_client {
        state
            .and_then(|s| path_value(s, "wall_secs"))
            .cloned()
            .unwrap_or(json!(0.0))
    } else {
        sim["tick"].clone()
    };
    let progress_path = if online_client { "wall_secs" } else { "tick" };
    let started_expected = if online_client {
        json!(30.0)
    } else {
        json!(SIM_TICK_EARLY)
    };
    let complete_expected = if online_client {
        json!(30.0)
    } else {
        json!(SIM_TICK_COMPLETE)
    };
    let complete_message = if online_client {
        "online client stopped before enough wall-clock runtime to prove networking is alive"
    } else {
        "simulation stopped before the closed-loop completion tick"
    };
    let mut a = vec![
        assertion(
            "png.readable",
            &png["verdict"],
            "==",
            json!("OK"),
            "fail",
            "primary screenshot must be readable and non-uniform",
            Some("png.verdict"),
        ),
        assertion(
            "png.file_size",
            &png["file_kb"],
            ">=",
            json!(PNG_MIN_SIZE_KB),
            "fail",
            "primary screenshot is too small to trust",
            Some("png.file_kb"),
        ),
        assertion(
            "png.width",
            &png["w"],
            ">=",
            json!(PNG_MIN_W),
            "fail",
            "primary screenshot width is below harness minimum",
            Some("png.w"),
        ),
        assertion(
            "png.height",
            &png["h"],
            ">=",
            json!(PNG_MIN_H),
            "fail",
            "primary screenshot height is below harness minimum",
            Some("png.h"),
        ),
        assertion(
            "png.color_dominance",
            &png["top_pct"],
            "<",
            json!(TOP_COLOR_BUCKET_WARN_PCT),
            "partial",
            "one color bucket dominates the screenshot; visual signal may be weak",
            Some("png.top_pct"),
        ),
        assertion(
            "sim.state_exists",
            &json!(state.is_some()),
            "==",
            json!(true),
            "fail",
            "final_state.json must exist and parse",
            Some("final_state.json"),
        ),
        assertion(
            "sim.started",
            &progress_actual,
            ">=",
            started_expected,
            "fail",
            "simulation did not run long enough to prove the app is alive",
            Some(progress_path),
        ),
        assertion(
            "sim.complete",
            &progress_actual,
            ">=",
            complete_expected,
            "partial",
            complete_message,
            Some(progress_path),
        ),
        assertion(
            "observer.anomalies",
            &sim["anomalies"],
            "==",
            json!(0),
            "fail",
            "tick observer reported anomalies",
            Some("observer.anomalies"),
        ),
        assertion(
            "observer.invariant_violations",
            &sim["invariant_violations"],
            "==",
            json!(0),
            "fail",
            "simulation invariant violations were reported",
            Some("observer.invariant_violations"),
        ),
        assertion(
            "stderr.deserialize_invalid",
            &json!(stderr.deserialize_invalid_count),
            "<",
            json!(STDERR_DESER_INVALID_FAIL_THRESHOLD),
            "fail",
            "stderr reported invalid entity deserialization",
            Some("run logs"),
        ),
        assertion(
            "stderr.out_of_bounds",
            &json!(stderr.out_of_bounds_count),
            "<",
            json!(STDERR_OUT_OF_BOUNDS_FAIL_THRESHOLD),
            "fail",
            "stderr reported repeated out-of-bounds events",
            Some("run logs"),
        ),
        assertion(
            "stderr.voxel_overflow",
            &json!(stderr.voxel_overflow_count),
            "<",
            json!(STDERR_VOXEL_OVERFLOW_FAIL_THRESHOLD),
            "fail",
            "stderr reported repeated voxel overflow events",
            Some("run logs"),
        ),
    ];
    if let Some(prev_png) = prev_png {
        a.extend(visual_luma_flicker_assertions(
            png, prev_png, state, prev_state,
        ));
    }
    if stderr.files_scanned == 0 {
        a.push(assertion(
            "stderr.logs_scanned",
            &json!(0),
            ">",
            json!(0),
            "partial",
            "no .err.log files found under screenshots/, run-logs/, or .harness/scratch/; cannot detect runtime errors. Run loop or check log redirect.",
            Some("stderr"),
        ));
    }
    if let Some(state) = state {
        let player_block = path_value(state, "player.block_pos");
        let world_size = path_i64(state, "world.size");
        let player_in_world = player_block
            .and_then(Value::as_array)
            .zip(world_size)
            .map(|(pos, size)| {
                pos.len() == 3
                    && pos
                        .iter()
                        .all(|v| v.as_i64().map(|n| n >= 0 && n < size).unwrap_or(false))
            })
            .unwrap_or(false);
        let activity = path_i64(state, "player.blocks_gathered").unwrap_or(0)
            + path_i64(state, "player.monsters_killed").unwrap_or(0)
            + path_i64(state, "player.nations_founded").unwrap_or(0)
            + path_i64(state, "eco_cycle.fruit_eaten").unwrap_or(0)
            + path_i64(state, "eco_cycle.fruit_grown").unwrap_or(0)
            + path_i64(state, "eco_cycle.plants_grown").unwrap_or(0);
        a.push(assertion(
            "player.in_world",
            &json!(player_in_world),
            "==",
            json!(true),
            "fail",
            "player block position must stay inside world bounds",
            Some("player.block_pos"),
        ));
        a.push(assertion(
            "gameplay.activity",
            &json!(activity),
            ">",
            json!(0),
            "partial",
            "auto-demo did not gather, kill, or found a nation",
            Some("player activity counters"),
        ));
        if !online_client {
            a.push(assertion(
                "gameplay.nation_progress",
                &json!(path_i64(state, "nations.total_nations")),
                ">=",
                json!(1),
                "partial",
                "auto-demo did not create or observe a nation",
                Some("nations.total_nations"),
            ));
        }
        a.extend(network_command_assertions(state));
        a.extend(network_connection_assertions(state));
        a.extend(camera_assertions(state));
        a.extend(player_readability_assertions(state));
        a.extend(resource_deltas_assertions(iter_dir));
        a.extend(eco_cycle_assertions(state));
        if let Some(prev) = prev_state {
            a.extend(regression_assertions(state, prev));
            let drift = static_world_visual_follow_drift(prev, state);
            a.push(assertion(
                "visual.static_world_not_player_following",
                &json!(drift.ok),
                "==",
                json!(true),
                drift.severity,
                &drift.message,
                Some("visual.static_world"),
            ));
        }
        let probe = movement_probe_drift(state);
        a.push(assertion(
            "visual.movement_probe_static_world",
            &json!(probe.ok),
            "==",
            json!(true),
            probe.severity,
            &probe.message,
            Some("visual.movement_probe"),
        ));
        a.extend(render_telemetry_assertions(state));
    }
    a.extend(server_state_assertions(state, server_state));
    a
}

fn latest_server_state(root: &Path, maximum_tick: Option<u64>) -> Option<Value> {
    let dir = root.join("screenshots");
    let mut paths = fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|name| name.starts_with("server_state_t") && name.ends_with(".json"))
                .unwrap_or(false)
        })
        .filter(|path| {
            let Some(maximum_tick) = maximum_tick else {
                return true;
            };
            path.file_name()
                .and_then(OsStr::to_str)
                .and_then(|name| name.strip_prefix("server_state_t"))
                .and_then(|name| name.strip_suffix(".json"))
                .and_then(|tick| tick.parse::<u64>().ok())
                .is_some_and(|tick| tick <= maximum_tick)
        })
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| {
        path.file_name()
            .and_then(OsStr::to_str)
            .and_then(|name| name.strip_prefix("server_state_t"))
            .and_then(|name| name.strip_suffix(".json"))
            .and_then(|tick| tick.parse::<u64>().ok())
            .unwrap_or(0)
    });
    paths.pop().and_then(|path| {
        fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
    })
}

fn server_state_summary(server: Option<&Value>, client: Option<&Value>) -> Value {
    let Some(server) = server else {
        return json!({"present": false});
    };
    let server_tick = path_value(server, "tick").and_then(Value::as_u64);
    let client_tick = client
        .and_then(|state| path_value(state, "tick"))
        .and_then(Value::as_u64);
    json!({
        "present": true,
        "tick": server_tick,
        "client_tick": client_tick,
        "authority_role": path_value(server, "role"),
        "tick_matches_client": tick_alignment(server_tick, client_tick),
        "world_present": server.get("world").is_some(),
        "persistence_present": server.get("persistence").is_some(),
        "persistence_valid": path_value(server, "persistence.valid"),
        "voxel_chunk": path_value(server, "voxel_chunk"),
    })
}

fn server_state_assertions(client: Option<&Value>, server: Option<&Value>) -> Vec<Assertion> {
    let Some(server) = server else {
        return if client
            .and_then(|state| state.get("role"))
            .and_then(Value::as_str)
            == Some("client_online")
        {
            vec![assertion(
                "server.snapshot_present",
                &json!(false),
                "==",
                json!(true),
                "fail",
                "online loop did not produce a server_state snapshot",
                Some("server_state"),
            )]
        } else {
            Vec::new()
        };
    };
    let server_tick = path_value(server, "tick").and_then(Value::as_u64);
    let client_tick = client
        .and_then(|state| path_value(state, "tick"))
        .and_then(Value::as_u64);
    let tick_ok = tick_alignment(server_tick, client_tick);
    let world_ok = server.get("world").is_some();
    let persistence = server.get("persistence");
    let chunk = server.get("voxel_chunk");
    vec![
        assertion(
            "server.snapshot_tick_matches_client",
            &json!(tick_ok),
            "==",
            json!(true),
            "fail",
            "server and client snapshots do not describe the same tick",
            Some("server_state.tick"),
        ),
        assertion(
            "server.world_snapshot_present",
            &json!(world_ok),
            "==",
            json!(true),
            "fail",
            "server snapshot is missing authoritative world data",
            Some("server_state.world"),
        ),
        assertion(
            "server.persistence_valid",
            &json!(
                persistence
                    .and_then(|value| value.get("valid"))
                    .and_then(Value::as_bool)
                    == Some(true)
            ),
            "==",
            json!(true),
            "fail",
            "server persistence snapshot is missing or invalid",
            Some("server_state.persistence.valid"),
        ),
        assertion(
            "server.persistence_tick_matches",
            &json!(
                persistence
                    .and_then(|value| value.get("tick"))
                    .and_then(Value::as_u64)
                    .zip(server_tick)
                    .is_some_and(|(save_tick, tick)| save_tick == tick)
            ),
            "==",
            json!(true),
            "fail",
            "persistence snapshot does not describe the current authoritative tick",
            Some("server_state.persistence.tick"),
        ),
        assertion(
            "server.voxel_chunk_valid",
            &json!(chunk.is_some_and(|value| {
                let block_count = value.get("block_count").and_then(Value::as_u64);
                let valid_count = value.get("valid_block_count").and_then(Value::as_u64);
                block_count.is_some()
                    && block_count == valid_count
                    && value.get("revision").and_then(Value::as_u64).is_some()
            })),
            "==",
            json!(true),
            "fail",
            "server voxel chunk snapshot is missing, malformed, or contains invalid block codes",
            Some("server_state.voxel_chunk"),
        ),
    ]
}

fn tick_alignment(server: Option<u64>, client: Option<u64>) -> bool {
    match (server, client) {
        (Some(server), Some(client)) => server <= client && client - server <= 10,
        _ => false,
    }
}

fn visual_luma_flicker_assertions(
    png: &Value,
    prev_png: &Value,
    state: Option<&Value>,
    prev_state: Option<&Value>,
) -> Vec<Assertion> {
    let Some(current_luma) = path_f64(png, "luma_mean") else {
        return Vec::new();
    };
    let Some(prev_luma) = path_f64(prev_png, "luma_mean") else {
        return Vec::new();
    };
    let delta = (current_luma - prev_luma).abs();
    let dayness_delta = state
        .zip(prev_state)
        .and_then(|(current, prev)| {
            Some(
                (path_f64(current, "render.lighting.dayness")?
                    - path_f64(prev, "render.lighting.dayness")?)
                .abs(),
            )
        })
        .unwrap_or(0.0);
    vec![assertion(
        "visual.luma_flicker_between_iters",
        &json!(round1(delta)),
        "<=",
        json!(LUMA_MEAN_FLICKER_WARN_DELTA),
        "partial",
        &format!(
            "mean screenshot brightness jumped by {delta:.1} between adjacent iters (prev {prev_luma:.1}, current {current_luma:.1}, dayness_delta {dayness_delta:.3}); inspect day_night_cycle, exposure, atmosphere, bloom, and volumetric fog"
        ),
        Some("png.luma_mean"),
    )]
}

fn player_readability_assertions(state: &Value) -> Vec<Assertion> {
    let mut out = Vec::new();
    let mode = path_value(state, "camera.mode")
        .and_then(Value::as_str)
        .unwrap_or("");
    if mode != "TopDown" {
        return out;
    }
    let Some(marker_count) = path_i64(state, "visual.player_readability.marker_count") else {
        out.push(assertion(
            "visual.player_readability_schema_present",
            &json!(false),
            "==",
            json!(true),
            "partial",
            "final_state.json missing visual.player_readability; rerun the client so top-down player readability can be evaluated",
            Some("visual.player_readability"),
        ));
        return out;
    };
    out.push(assertion(
        "visual.player_readability_marker_present",
        &json!(marker_count),
        ">=",
        json!(PLAYER_READABILITY_MIN_MARKERS),
        "partial",
        "top-down screenshot needs an explicit player-attached readability marker so the player is identifiable at village scale",
        Some("visual.player_readability.marker_count"),
    ));
    if let Some(distance) = path_f64(state, "visual.player_readability.marker_max_distance") {
        out.push(assertion(
            "visual.player_readability_marker_near_player",
            &json!(round1(distance)),
            "<=",
            json!(PLAYER_READABILITY_MAX_MARKER_DISTANCE),
            "partial",
            "player readability marker drifted away from the player",
            Some("visual.player_readability.marker_max_distance"),
        ));
    }
    out
}

fn network_command_assertions(state: &Value) -> Vec<Assertion> {
    let mut out = Vec::new();
    let role = state.get("role").and_then(Value::as_str).unwrap_or("");
    let move_sent = path_i64(state, "network_command.move_world_sent").unwrap_or(0);
    if role == "client_online" {
        out.push(assertion(
            "network.move_world_sent_advances",
            &json!(move_sent),
            ">",
            json!(0),
            "fail",
            "online mode running but move_world_sent stayed at 0; WASD pipeline is dead (iter_200 Root Cause B regression)",
            Some("network_command.move_world_sent"),
        ));
    } else if move_sent > 0 {
        out.push(assertion(
            "network.move_world_sent_offline_leak",
            &json!(move_sent),
            "==",
            json!(0),
            "partial",
            "offline mode recorded move_world_sent > 0; network state is leaking into offline iter",
            Some("network_command.move_world_sent"),
        ));
    }
    out
}

fn network_connection_assertions(state: &Value) -> Vec<Assertion> {
    if state.get("role").and_then(Value::as_str) != Some("client_online") {
        return Vec::new();
    }
    let transport = path_value(state, "network_connection.transport")
        .and_then(Value::as_str)
        .unwrap_or("");
    let ping = path_f64(state, "network_connection.ping_ms");
    let pong_age = path_f64(state, "network_connection.pong_age_secs");
    vec![
        assertion(
            "network.transport_connected",
            &json!(transport),
            "==",
            json!("Connected"),
            "fail",
            "online transport is not connected",
            Some("network_connection.transport"),
        ),
        assertion(
            "network.ping_available",
            &json!(ping),
            ">=",
            json!(0.0),
            "partial",
            "online connection has no finite ping sample",
            Some("network_connection.ping_ms"),
        ),
        assertion(
            "network.pong_recent",
            &json!(pong_age),
            "<=",
            json!(5.0),
            "fail",
            "online connection has not received a recent pong",
            Some("network_connection.pong_age_secs"),
        ),
    ]
}

fn camera_assertions(state: &Value) -> Vec<Assertion> {
    let mut out = Vec::new();
    let mode = state
        .get("camera")
        .and_then(|c| c.get("mode"))
        .and_then(Value::as_str)
        .map(str::to_string);
    match mode.as_deref() {
        Some(m) if !m.is_empty() => {
            out.push(assertion(
                "camera.mode_present",
                &json!(m),
                "!=",
                json!(""),
                "fail",
                "camera.mode must be a non-empty string (ThirdPerson / FirstPerson)",
                Some("camera.mode"),
            ));
        }
        Some(_) | None => {
            out.push(assertion(
                "camera.mode_present",
                &json!(false),
                "==",
                json!(true),
                "fail",
                "final_state.json missing camera.mode; harness cannot verify first-person task alignment",
                Some("camera.mode"),
            ));
        }
    }
    if let Some(eye_y) = state
        .get("camera")
        .and_then(|c| c.get("first_person_eye"))
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.get(1))
        .and_then(Value::as_f64)
    {
        out.push(assertion(
            "camera.first_person_eye_y_in_range",
            &json!(round1(eye_y)),
            "<=",
            json!(FIRST_PERSON_EYE_MAX_Y),
            "fail",
            "first_person_eye.y is suspiciously high; pretty landmarks may have spawned at the wrong Y (iter_200 Root Cause C regression)",
            Some("camera.first_person_eye"),
        ));
    }
    if mode.as_deref() == Some("TopDown")
        && path_value(state, "visual.player_readability.marker_count").is_some()
    {
        let center_block = path_value(state, "camera.center_ray_hit.block")
            .and_then(Value::as_str)
            .unwrap_or("");
        if center_block == "Leaves" {
            let distance = path_f64(state, "camera.center_ray_hit.distance").unwrap_or(0.0);
            out.push(assertion(
                "camera.top_down_center_not_leaf_blocked",
                &json!(round1(distance)),
                ">=",
                json!(TOP_DOWN_CENTER_LEAVES_MIN_DISTANCE),
                "partial",
                "top-down camera center is hitting nearby Leaves voxels; foliage or leaf-surface placement is obscuring the player/scene scale read",
                Some("camera.center_ray_hit"),
            ));
        }
    }
    out
}

fn resource_deltas_assertions(iter_dir: &Path) -> Vec<Assertion> {
    let mut out = Vec::new();
    let path = iter_dir.join("diff.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return out;
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return out;
    };
    let Some(deltas) = json.get("resource_deltas").and_then(Value::as_array) else {
        return out;
    };
    let nontick_count = deltas
        .iter()
        .filter(|d| {
            let p = d.get("path").and_then(Value::as_str).unwrap_or("");
            p != "tick"
        })
        .count();
    let final_state = fs::read_to_string(iter_dir.join("final_state.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let fallback_motion = final_state.as_ref().map(gameplay_motion_count).unwrap_or(0);
    let proof_count = nontick_count.max(fallback_motion);
    out.push(assertion(
        "state.resource_deltas_have_nontick_motion",
        &json!(proof_count),
        ">=",
        json!(RESOURCE_DELTAS_MIN_NONTICK),
        "partial",
        "diff.json resource_deltas only contain tick and final_state has no gameplay motion counters",
        Some("diff.json"),
    ));
    out
}

fn gameplay_motion_count(state: &Value) -> usize {
    [
        "player.blocks_gathered",
        "player.monsters_killed",
        "player.nations_founded",
        "eco_cycle.fruit_eaten",
        "eco_cycle.fruit_grown",
        "eco_cycle.plants_grown",
        "nations.total_nations",
    ]
    .iter()
    .filter(|path| path_i64(state, path).unwrap_or(0) > 0)
    .count()
}

fn regression_assertions(state: &Value, prev: &Value) -> Vec<Assertion> {
    if !same_regression_context(state, prev) {
        return Vec::new();
    }
    let mut out = Vec::new();
    let monsters_curr = path_f64(state, "monsters.current").unwrap_or(0.0);
    let monsters_prev = path_f64(prev, "monsters.current").unwrap_or(0.0);
    if monsters_prev > 0.0 {
        let ratio = monsters_curr / monsters_prev;
        out.push(assertion(
            "regression.monsters_not_collapsed",
            &json!(round1(ratio * 100.0) / 100.0),
            ">=",
            json!(MONSTERS_DROP_PARTIAL_RATIO),
            "partial",
            &format!(
                "monsters.current dropped from {monsters_prev:.0} to {monsters_curr:.0} (ratio {ratio:.2}); over-kill or despawn bug"
            ),
            Some("monsters.current"),
        ));
    }
    let nations_curr = path_i64(state, "nations.total_nations").unwrap_or(0);
    let nations_prev = path_i64(prev, "nations.total_nations").unwrap_or(0);
    if nations_prev > 0 {
        out.push(assertion(
            "regression.nations_total_non_decreasing",
            &json!(nations_curr),
            ">=",
            json!(nations_prev),
            "fail",
            &format!(
                "nations.total_nations dropped from {nations_prev} to {nations_curr}; nations must not be destroyed across iters"
            ),
            Some("nations.total_nations"),
        ));
    }
    out
}

fn same_regression_context(state: &Value, prev: &Value) -> bool {
    ["role", "camera.mode"].iter().all(|path| {
        let current = path_value(state, path);
        let previous = path_value(prev, path);
        current == previous || (current.is_none() && previous.is_none())
    })
}

struct StaticWorldDrift {
    ok: bool,
    severity: &'static str,
    message: String,
}

fn static_world_visual_follow_drift(prev: &Value, current: &Value) -> StaticWorldDrift {
    let Some(prev_player) = path_vec3(prev, "player.pos") else {
        return StaticWorldDrift {
            ok: true,
            severity: "partial",
            message: "previous player position missing; static-world drift check skipped"
                .to_string(),
        };
    };
    let Some(current_player) = path_vec3(current, "player.pos") else {
        return StaticWorldDrift {
            ok: true,
            severity: "partial",
            message: "current player position missing; static-world drift check skipped"
                .to_string(),
        };
    };
    let prev_visuals = static_world_visual_positions(prev);
    let current_visuals = static_world_visual_positions(current);
    let mut drift =
        static_world_drift_from_maps(prev_player, current_player, &prev_visuals, &current_visuals);
    if !drift.ok {
        drift.severity = "partial";
    }
    drift
}

fn movement_probe_drift(state: &Value) -> StaticWorldDrift {
    let Some(first_player) = path_vec3(state, "visual.movement_probe.first_player_pos") else {
        return StaticWorldDrift {
            ok: false,
            severity: "partial",
            message: "movement probe missing first player position; closed-loop cannot prove static world visuals are stable".to_string(),
        };
    };
    let Some(current_player) = path_vec3(state, "visual.movement_probe.current_player_pos") else {
        return StaticWorldDrift {
            ok: false,
            severity: "partial",
            message: "movement probe missing current player position; closed-loop cannot prove static world visuals are stable".to_string(),
        };
    };
    let first_visuals =
        static_world_positions_at(state, "visual.movement_probe.first_static_world");
    let current_visuals =
        static_world_positions_at(state, "visual.movement_probe.current_static_world");
    if first_visuals.is_empty() {
        return StaticWorldDrift {
            ok: false,
            severity: "partial",
            message: "movement probe captured no first static-world anchors; register fixed terrain/marker anchors before scoring stability".to_string(),
        };
    }
    if current_visuals.is_empty() {
        return StaticWorldDrift {
            ok: false,
            severity: "partial",
            message: "movement probe captured no current static-world anchors; register fixed terrain/marker anchors before scoring stability".to_string(),
        };
    }
    static_world_drift_from_maps(
        first_player,
        current_player,
        &first_visuals,
        &current_visuals,
    )
}

fn static_world_visual_positions(value: &Value) -> HashMap<String, [f64; 3]> {
    static_world_positions_at(value, "visual.static_world")
}

fn static_world_positions_at(value: &Value, path: &str) -> HashMap<String, [f64; 3]> {
    let mut out = HashMap::new();
    let Some(obj) = path_value(value, path).and_then(Value::as_object) else {
        return out;
    };
    for (name, pos_value) in obj {
        if let Some(pos) = vec3_value(pos_value) {
            out.insert(name.clone(), pos);
        }
    }
    out
}

fn static_world_drift_from_maps(
    prev_player: [f64; 3],
    current_player: [f64; 3],
    prev_visuals: &HashMap<String, [f64; 3]>,
    current_visuals: &HashMap<String, [f64; 3]>,
) -> StaticWorldDrift {
    const PLAYER_MOVE_MIN: f64 = 1.0;
    const STATIC_MOVE_MAX: f64 = 0.25;
    const FOLLOW_RATIO_MIN: f64 = 0.70;

    let player_delta = vec3_sub(current_player, prev_player);
    let player_move = vec3_len_xz(player_delta);
    if player_move < PLAYER_MOVE_MIN {
        return StaticWorldDrift {
            ok: true,
            severity: "partial",
            message: format!(
                "player moved {player_move:.2}m; below {PLAYER_MOVE_MIN:.2}m threshold"
            ),
        };
    }

    let mut worst: Option<(String, f64, f64)> = None;
    for (name, prev_pos) in prev_visuals {
        let Some(current_pos) = current_visuals.get(name).copied() else {
            continue;
        };
        let visual_delta = vec3_sub(current_pos, *prev_pos);
        let visual_move = vec3_len_xz(visual_delta);
        let follow_alignment = if player_move > 0.0 {
            dot_xz(visual_delta, player_delta) / (visual_move.max(0.0001) * player_move)
        } else {
            0.0
        };
        if visual_move > STATIC_MOVE_MAX && follow_alignment >= FOLLOW_RATIO_MIN {
            let replace = worst
                .as_ref()
                .map(|(_, worst_move, _)| visual_move > *worst_move)
                .unwrap_or(true);
            if replace {
                worst = Some((name.clone(), visual_move, follow_alignment));
            }
        }
    }

    if let Some((name, visual_move, follow_alignment)) = worst {
        StaticWorldDrift {
            ok: false,
            severity: "fail",
            message: format!(
                "{name} moved {visual_move:.2}m with player movement {player_move:.2}m (alignment {follow_alignment:.2}); static world visuals must not follow the player"
            ),
        }
    } else {
        StaticWorldDrift {
            ok: true,
            severity: "partial",
            message: format!(
                "static world visuals stayed fixed while player moved {player_move:.2}m"
            ),
        }
    }
}

fn render_telemetry_assertions(state: &Value) -> Vec<Assertion> {
    let mut out = Vec::new();
    if let Some(value) = path_i64(state, "render.frame.dt_over_50ms") {
        out.push(assertion(
            "render.frame_spikes",
            &json!(value),
            "<=",
            json!(FRAME_DT_OVER_50MS_WARN),
            "partial",
            "too many frames exceeded 50ms",
            Some("render.frame.dt_over_50ms"),
        ));
    }
    if let Some(value) = path_i64(state, "render.terrain.smooth_mesh_builds") {
        out.push(assertion(
            "render.smooth_mesh_builds",
            &json!(value),
            "<=",
            json!(SMOOTH_MESH_BUILDS_WARN),
            "partial",
            "smooth terrain mesh rebuilt too often during loop",
            Some("render.terrain.smooth_mesh_builds"),
        ));
    }
    if let Some(value) = path_f64(state, "render.terrain.smooth_mesh_max_ms") {
        out.push(assertion(
            "render.smooth_mesh_max_ms",
            &json!(value),
            "<=",
            json!(SMOOTH_MESH_MAX_WARN_MS),
            "partial",
            "smooth terrain mesh rebuild was slow enough to be player-visible",
            Some("render.terrain.smooth_mesh_max_ms"),
        ));
    }
    if let Some(value) = path_i64(state, "render.terrain.terrain_despawns") {
        out.push(assertion(
            "render.terrain_despawns",
            &json!(value),
            "<=",
            json!(TERRAIN_DESPAWNS_WARN),
            "partial",
            "terrain entities were replaced too often during loop",
            Some("render.terrain.terrain_despawns"),
        ));
    }
    out
}

fn eco_cycle_assertions(state: &Value) -> Vec<Assertion> {
    let mut out = Vec::new();
    push_optional_i64_assertion(
        &mut out,
        state,
        "eco.clouds_exist",
        "eco_cycle.clouds",
        ">",
        json!(0),
        "fail",
        "eco cycle has no authoritative clouds",
    );
    push_optional_f64_assertion(
        &mut out,
        state,
        "eco.clouds_produce_rain",
        "eco_cycle.rainfall",
        ">",
        json!(0.0),
        "fail",
        "clouds did not produce rainfall",
    );
    push_optional_i64_assertion(
        &mut out,
        state,
        "eco.rain_grows_plants",
        "eco_cycle.plants_grown",
        ">",
        json!(0),
        "partial",
        "rain did not grow grass or flowers",
    );
    push_optional_i64_min_of_paths_assertion(
        &mut out,
        state,
        "eco.plants_support_small_animals",
        &["eco_cycle.rabbits_born", "eco_cycle.rabbits"],
        ">",
        json!(0),
        "partial",
        "plants did not support new small animals",
    );
    push_optional_i64_min_of_paths_assertion(
        &mut out,
        state,
        "eco.small_animals_support_wildlife",
        &["eco_cycle.wildlife_born", "eco_cycle.wildlife"],
        ">",
        json!(0),
        "partial",
        "small animals did not support larger wildlife",
    );
    out
}

fn push_optional_i64_min_of_paths_assertion(
    out: &mut Vec<Assertion>,
    state: &Value,
    id: &str,
    paths: &[&str],
    op: &str,
    expected: Value,
    severity: &str,
    message: &str,
) {
    let value = paths.iter().filter_map(|path| path_i64(state, path)).max();
    if let Some(value) = value {
        let path_label = paths.join("|");
        out.push(assertion(
            id,
            &json!(value),
            op,
            expected,
            severity,
            message,
            Some(&path_label),
        ));
    }
}

fn push_optional_i64_assertion(
    out: &mut Vec<Assertion>,
    state: &Value,
    id: &str,
    path: &str,
    op: &str,
    expected: Value,
    severity: &str,
    message: &str,
) {
    if let Some(value) = path_i64(state, path) {
        out.push(assertion(
            id,
            &json!(value),
            op,
            expected,
            severity,
            message,
            Some(path),
        ));
    } else {
        out.push(assertion(
            &format!("{id}.metric_present"),
            &json!(false),
            "==",
            json!(true),
            "partial",
            &format!("missing metric {path}; cannot evaluate: {message}"),
            Some(path),
        ));
    }
}

fn push_optional_f64_assertion(
    out: &mut Vec<Assertion>,
    state: &Value,
    id: &str,
    path: &str,
    op: &str,
    expected: Value,
    severity: &str,
    message: &str,
) {
    if let Some(value) = path_f64(state, path) {
        out.push(assertion(
            id,
            &json!(value),
            op,
            expected,
            severity,
            message,
            Some(path),
        ));
    } else {
        out.push(assertion(
            &format!("{id}.metric_present"),
            &json!(false),
            "==",
            json!(true),
            "partial",
            &format!("missing metric {path}; cannot evaluate: {message}"),
            Some(path),
        ));
    }
}

fn assertion(
    id: &str,
    actual: &Value,
    op: &str,
    expected: Value,
    severity: &str,
    message: &str,
    path: Option<&str>,
) -> Assertion {
    Assertion {
        id: id.to_string(),
        ok: compare(actual, op, &expected),
        severity: severity.to_string(),
        actual: actual.clone(),
        op: op.to_string(),
        expected,
        message: message.to_string(),
        path: path.map(str::to_string),
    }
}

fn compare(actual: &Value, op: &str, expected: &Value) -> bool {
    match op {
        "==" => actual == expected,
        "!=" => actual != expected,
        ">" => num(actual)
            .zip(num(expected))
            .map(|(a, e)| a > e)
            .unwrap_or(false),
        ">=" => num(actual)
            .zip(num(expected))
            .map(|(a, e)| a >= e)
            .unwrap_or(false),
        "<" => num(actual)
            .zip(num(expected))
            .map(|(a, e)| a < e)
            .unwrap_or(false),
        "<=" => num(actual)
            .zip(num(expected))
            .map(|(a, e)| a <= e)
            .unwrap_or(false),
        _ => false,
    }
}

fn combine(png: &Value, sim: &Value, assertions: &[Assertion]) -> Value {
    let mut reasons = Vec::new();
    let mut score = 10.0;
    if png["verdict"] == "UNIFORM" {
        reasons.push(format!(
            "png uniform ({}, luma_var={})",
            png["sub"].as_str().unwrap_or("GRAY"),
            png["luma_var"]
        ));
        score = 0.0;
    } else if png["verdict"] == "ERR" {
        reasons.push(format!(
            "png read error: {}",
            png["err"].as_str().unwrap_or("?")
        ));
        score = 0.0;
    }
    match sim["verdict"].as_str().unwrap_or("DEAD") {
        "DEAD" => {
            reasons.push("sim state missing".to_string());
            score = f64::min(score, 0.0);
        }
        "EARLY" => {
            reasons.push(format!(
                "sim early (tick={}, need >={SIM_TICK_EARLY})",
                sim["tick"]
            ));
            score = f64::min(score, 5.0);
        }
        "STUCK" => {
            reasons.push(format!(
                "sim stuck at tick={} (same as prev)",
                sim["stuck_at"]
            ));
            score = f64::min(score, 2.0);
        }
        "RUNNING" => {
            reasons.push(format!(
                "sim incomplete (tick={}, need >={SIM_TICK_COMPLETE})",
                sim["tick"]
            ));
            score = f64::min(score, 6.0);
        }
        _ => {}
    }
    let failed = assertions.iter().filter(|a| !a.ok).collect::<Vec<_>>();
    let hard = failed
        .iter()
        .filter(|a| a.severity == "fail")
        .collect::<Vec<_>>();
    let partial = failed
        .iter()
        .filter(|a| a.severity != "fail")
        .collect::<Vec<_>>();
    for a in hard.iter().chain(partial.iter()).take(10) {
        reasons.push(format!(
            "{}: {} ({} {} {})",
            a.id, a.message, a.actual, a.op, a.expected
        ));
    }
    if !hard.is_empty() {
        score = f64::min(score, 0.0);
    } else if !partial.is_empty() {
        score = f64::min(score, 6.0);
    }
    let verdict = if score >= 9.0 {
        "PASS"
    } else if score >= 4.0 {
        "PARTIAL"
    } else {
        "FAIL"
    };
    json!({"score": score, "verdict": verdict, "reasons": reasons})
}

fn summarize(name: &str, png: &Value, sim: &Value, combo: &Value) -> String {
    let mut out = format!(
        "{name} {:<7} png={}KB pv={} luma={}/{} sim={} tick={} nations={} anomalies={} invariants={} score={:.1}",
        combo["verdict"].as_str().unwrap_or("?"),
        png["file_kb"],
        png["verdict"].as_str().unwrap_or("?"),
        png["luma_mean"],
        png["luma_var"],
        sim["verdict"].as_str().unwrap_or("?"),
        sim["tick"],
        sim["nations"],
        sim["anomalies"],
        sim["invariant_violations"],
        combo["score"].as_f64().unwrap_or(0.0),
    );
    if let Some(reasons) = combo["reasons"].as_array() {
        if !reasons.is_empty() {
            out.push_str(" | ");
            out.push_str(
                &reasons
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("; "),
            );
        }
    }
    out
}

fn path_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = value;
    for part in path.split('.') {
        cur = cur.get(part)?;
    }
    Some(cur)
}

fn path_i64(value: &Value, path: &str) -> Option<i64> {
    path_value(value, path).and_then(Value::as_i64)
}

fn path_f64(value: &Value, path: &str) -> Option<f64> {
    path_value(value, path).and_then(Value::as_f64)
}

fn path_vec3(value: &Value, path: &str) -> Option<[f64; 3]> {
    path_value(value, path).and_then(vec3_value)
}

fn vec3_value(value: &Value) -> Option<[f64; 3]> {
    let arr = value.as_array()?;
    if arr.len() != 3 {
        return None;
    }
    Some([arr[0].as_f64()?, arr[1].as_f64()?, arr[2].as_f64()?])
}

fn vec3_sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn vec3_len_xz(v: [f64; 3]) -> f64 {
    (v[0] * v[0] + v[2] * v[2]).sqrt()
}

fn dot_xz(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[2] * b[2]
}

fn num(value: &Value) -> Option<f64> {
    value.as_f64()
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(player_x: f64, ground_x: f64) -> Value {
        json!({
            "player": { "pos": [player_x, 15.0, 40.0] },
            "visual": {
                "static_world": {
                    "ground": [ground_x, 0.0, 48.0],
                    "nest_marker": [60.0, 20.0, 60.0]
                }
            }
        })
    }

    fn probed_state(player_x: f64, current_player_x: f64, ground_x: f64) -> Value {
        json!({
            "visual": {
                "movement_probe": {
                    "first_player_pos": [player_x, 15.0, 40.0],
                    "current_player_pos": [current_player_x, 15.0, 40.0],
                    "first_static_world": {
                        "ground": [48.0, 0.0, 48.0]
                    },
                    "current_static_world": {
                        "ground": [ground_x, 0.0, 48.0]
                    }
                }
            }
        })
    }

    #[test]
    fn analyze_sim_keeps_repeated_complete_tick_complete() {
        let dir = temp_root("xtask_complete_tick_not_stuck");
        let state_path = dir.join("final_state.json");
        fs::write(
            &state_path,
            r#"{"tick":100,"nations":{"total_nations":8},"observer":{"anomalies":0,"invariant_violations":0}}"#,
        )
        .unwrap();
        let prev = json!({"tick": 100});

        let (sim, _) = analyze_sim(&state_path, Some(&prev));

        assert_eq!(sim["verdict"], json!("COMPLETE"));
        assert!(sim.get("stuck_at").is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn analyze_sim_marks_repeated_incomplete_tick_stuck() {
        let dir = temp_root("xtask_incomplete_tick_stuck");
        let state_path = dir.join("final_state.json");
        fs::write(
            &state_path,
            r#"{"tick":50,"nations":{"total_nations":8},"observer":{"anomalies":0,"invariant_violations":0}}"#,
        )
        .unwrap();
        let prev = json!({"tick": 50});

        let (sim, _) = analyze_sim(&state_path, Some(&prev));

        assert_eq!(sim["verdict"], json!("STUCK"));
        assert_eq!(sim["stuck_at"], json!(50));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn analyze_sim_online_client_uses_wall_secs_for_completion() {
        let dir = temp_root("xtask_online_tick_not_stuck");
        let state_path = dir.join("final_state.json");
        fs::write(
            &state_path,
            r#"{"role":"client_online","wall_secs":45.0,"tick":6,"nations":{"total_nations":0},"observer":{"anomalies":0,"invariant_violations":0}}"#,
        )
        .unwrap();
        let prev = json!({"tick": 6});

        let (sim, _) = analyze_sim(&state_path, Some(&prev));

        assert_eq!(sim["verdict"], json!("COMPLETE"));
        assert!(sim.get("stuck_at").is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn static_world_drift_is_partial_when_visual_differs_across_iters() {
        let prev = state(10.0, 48.0);
        let current = state(14.0, 52.0);

        let drift = static_world_visual_follow_drift(&prev, &current);

        assert!(!drift.ok, "{}", drift.message);
        assert_eq!(drift.severity, "partial");
        assert!(drift.message.contains("ground"));
    }

    #[test]
    fn static_world_drift_passes_when_visual_stays_fixed() {
        let prev = state(10.0, 48.0);
        let current = state(14.0, 48.0);

        let drift = static_world_visual_follow_drift(&prev, &current);

        assert!(drift.ok, "{}", drift.message);
    }

    #[test]
    fn movement_probe_fails_when_static_visual_moves_with_player() {
        let state = probed_state(10.0, 14.0, 52.0);

        let drift = movement_probe_drift(&state);

        assert!(!drift.ok, "{}", drift.message);
        assert_eq!(drift.severity, "fail");
        assert!(drift.message.contains("ground"));
    }

    #[test]
    fn movement_probe_fails_when_probe_is_missing() {
        let drift = movement_probe_drift(&json!({}));

        assert!(!drift.ok);
        assert_eq!(drift.severity, "partial");
        assert!(drift.message.contains("missing first player position"));
    }

    #[test]
    fn render_telemetry_flags_mesh_and_frame_spikes() {
        let state = json!({
            "render": {
                "frame": {
                    "dt_max_ms": 180.0,
                    "dt_over_50ms": 14
                },
                "terrain": {
                    "smooth_mesh_builds": 5,
                    "smooth_mesh_max_ms": 88.0,
                    "terrain_despawns": 8
                }
            }
        });

        let assertions = render_telemetry_assertions(&state);
        let failed = assertions
            .iter()
            .filter(|a| !a.ok)
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>();

        assert!(failed.contains(&"render.frame_spikes"));
        assert!(failed.contains(&"render.smooth_mesh_builds"));
        assert!(failed.contains(&"render.smooth_mesh_max_ms"));
        assert!(failed.contains(&"render.terrain_despawns"));
    }

    #[test]
    fn top_down_requires_player_readability_marker() {
        let state = json!({
            "camera": { "mode": "TopDown" },
            "visual": { "player_readability": { "marker_count": 0 } }
        });

        let assertions = player_readability_assertions(&state);

        let marker = assertions
            .iter()
            .find(|a| a.id == "visual.player_readability_marker_present")
            .expect("marker assertion");
        assert!(!marker.ok);
        assert_eq!(marker.severity, "partial");
    }

    #[test]
    fn top_down_accepts_near_player_readability_marker() {
        let state = json!({
            "camera": { "mode": "TopDown" },
            "visual": { "player_readability": { "marker_count": 2, "marker_max_distance": 2.1 } }
        });

        let assertions = player_readability_assertions(&state);

        assert!(
            assertions.iter().all(|a| a.ok),
            "{:?}",
            assertions
                .iter()
                .map(|a| (&a.id, a.ok, &a.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn third_person_does_not_require_player_readability_marker() {
        let state = json!({
            "camera": { "mode": "ThirdPerson" },
            "visual": { "player_readability": { "marker_count": 0 } }
        });

        let assertions = player_readability_assertions(&state);

        assert!(assertions.is_empty());
    }

    #[test]
    fn eco_cycle_missing_metrics_are_partial_not_hard_failures() {
        let state = json!({"eco_cycle": {"rabbits": 5}});

        let assertions = eco_cycle_assertions(&state);

        assert!(
            assertions
                .iter()
                .any(|a| a.id == "eco.clouds_exist.metric_present")
        );
        assert!(
            assertions
                .iter()
                .filter(|a| !a.ok)
                .all(|a| a.severity == "partial"),
            "missing metrics should not be hard failures"
        );
    }

    #[test]
    fn eco_cycle_zero_clouds_and_rain_are_hard_failures_when_metrics_exist() {
        let state = json!({
            "eco_cycle": {
                "clouds": 0,
                "rainfall": 0.0,
                "plants_grown": 0,
                "rabbits_born": 0,
                "wildlife_born": 0
            }
        });

        let assertions = eco_cycle_assertions(&state);
        let hard_failed = assertions
            .iter()
            .filter(|a| !a.ok && a.severity == "fail")
            .map(|a| a.id.as_str())
            .collect::<Vec<_>>();

        assert!(hard_failed.contains(&"eco.clouds_exist"));
        assert!(hard_failed.contains(&"eco.clouds_produce_rain"));
    }

    #[test]
    fn network_command_fails_when_online_mode_sends_zero_moves() {
        let state = json!({"role": "client_online", "network_command": {"move_world_sent": 0}});
        let assertions = network_command_assertions(&state);
        let failed: Vec<&Assertion> = assertions.iter().filter(|a| !a.ok).collect();
        assert!(
            failed
                .iter()
                .any(|a| a.id == "network.move_world_sent_advances")
        );
        assert!(failed.iter().all(|a| a.severity == "fail"));
    }

    #[test]
    fn network_connection_requires_connected_transport_and_recent_pong() {
        let stale = json!({
            "role": "client_online",
            "network_connection": {
                "transport": "Connected",
                "ping_ms": 37.0,
                "pong_age_secs": 11.6
            }
        });
        let assertions = network_connection_assertions(&stale);
        assert!(
            assertions
                .iter()
                .any(|a| a.id == "network.transport_connected" && a.ok)
        );
        assert!(
            assertions
                .iter()
                .any(|a| a.id == "network.ping_available" && a.ok)
        );
        assert!(
            assertions
                .iter()
                .any(|a| a.id == "network.pong_recent" && !a.ok)
        );

        let healthy = json!({
            "role": "client_online",
            "network_connection": {
                "transport": "Connected",
                "ping_ms": 24.0,
                "pong_age_secs": 0.4
            }
        });
        assert!(network_connection_assertions(&healthy).iter().all(|a| a.ok));
    }

    #[test]
    fn online_client_progress_uses_wall_secs_not_offline_tick() {
        let png = json!({"verdict":"OK","file_kb":100.0,"w":1280,"h":720,"top_pct":50.0});
        let sim = json!({"tick":4,"anomalies":0,"invariant_violations":0});
        let state = json!({
            "role": "client_online",
            "wall_secs": 46.0,
            "player": {"block_pos": [48, 15, 48]},
            "world": {"size": 96},
            "network_command": {"move_world_sent": 100},
            "camera": {"mode": "TopDown", "first_person_eye": [48.0, 16.7, 48.0]},
            "visual": {
                "player_readability": {"marker_count": 2, "marker_max_distance": 1.0},
                "movement_probe": {
                    "first_player_pos": [48.0, 15.0, 48.0],
                    "current_player_pos": [50.0, 15.0, 48.0],
                    "first_static_world": {"ground": [0.0, 0.0, 0.0]},
                    "current_static_world": {"ground": [0.0, 0.0, 0.0]}
                }
            },
            "render": {
                "frame": {"dt_over_50ms": 0},
                "terrain": {"smooth_mesh_builds": 0, "smooth_mesh_max_ms": 0.0, "terrain_despawns": 0}
            },
            "eco_cycle": {"clouds": 1, "rainfall": 1.0, "plants_grown": 1, "rabbits_born": 1, "wildlife_born": 1}
        });
        let dir = temp_root("xtask_online_wall_secs_progress");
        fs::write(
            dir.join("diff.json"),
            r#"{"resource_deltas":[{"path":"network_command.move_world_sent"}]}"#,
        )
        .unwrap();

        let assertions = built_in_assertions(
            &png,
            None,
            &sim,
            Some(&state),
            None,
            &StderrScan {
                files_scanned: 1,
                ..Default::default()
            },
            &dir,
            None,
        );
        let started = assertions.iter().find(|a| a.id == "sim.started").unwrap();
        let complete = assertions.iter().find(|a| a.id == "sim.complete").unwrap();

        assert!(started.ok, "{}: {}", started.id, started.message);
        assert!(complete.ok, "{}: {}", complete.id, complete.message);
        assert!(
            !assertions
                .iter()
                .any(|a| a.id == "gameplay.nation_progress")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn luma_flicker_assertion_flags_large_adjacent_brightness_jump() {
        let prev_png = json!({"verdict":"OK","luma_mean":72.0});
        let png = json!({"verdict":"OK","luma_mean":128.5});
        let prev_state = json!({"render": {"lighting": {"dayness": 0.74}}});
        let state = json!({"render": {"lighting": {"dayness": 0.77}}});

        let assertions =
            visual_luma_flicker_assertions(&png, &prev_png, Some(&state), Some(&prev_state));

        let flicker = assertions
            .iter()
            .find(|a| a.id == "visual.luma_flicker_between_iters")
            .unwrap();
        assert!(!flicker.ok);
        assert_eq!(flicker.severity, "partial");
        assert_eq!(flicker.actual, json!(56.5));
    }

    #[test]
    fn network_command_passes_when_offline_and_silence() {
        let state = json!({"role": "client_offline", "network_command": {"move_world_sent": 0}});
        let assertions = network_command_assertions(&state);
        assert!(assertions.iter().all(|a| a.ok));
    }

    #[test]
    fn network_command_warns_when_offline_records_moves() {
        let state = json!({"role": "client_offline", "network_command": {"move_world_sent": 12}});
        let assertions = network_command_assertions(&state);
        let offline_leak = assertions
            .iter()
            .find(|a| a.id == "network.move_world_sent_offline_leak")
            .expect("offline leak assertion missing");
        assert!(!offline_leak.ok);
        assert_eq!(offline_leak.severity, "partial");
    }

    #[test]
    fn camera_eye_too_high_is_hard_failure() {
        let state = json!({
            "camera": {
                "mode": "FirstPerson",
                "first_person_eye": [48.0, 95.0, 48.0]
            }
        });
        let assertions = camera_assertions(&state);
        let high_eye = assertions
            .iter()
            .find(|a| a.id == "camera.first_person_eye_y_in_range")
            .expect("eye y assertion missing");
        assert!(
            !high_eye.ok,
            "eye_y=95 should exceed FIRST_PERSON_EYE_MAX_Y={}; actual={} op={} expected={}",
            FIRST_PERSON_EYE_MAX_Y, high_eye.actual, high_eye.op, high_eye.expected
        );
        assert_eq!(high_eye.severity, "fail");
        assert_eq!(high_eye.actual, json!(95.0));
    }

    #[test]
    fn camera_eye_in_range_passes() {
        let state = json!({
            "camera": {
                "mode": "FirstPerson",
                "first_person_eye": [48.0, 16.7, 48.0]
            }
        });
        let assertions = camera_assertions(&state);
        let eye = assertions
            .iter()
            .find(|a| a.id == "camera.first_person_eye_y_in_range")
            .expect("eye y assertion missing");
        assert!(eye.ok, "eye_y=16.7 should pass");
    }

    #[test]
    fn camera_missing_mode_is_hard_failure() {
        let state = json!({"camera": {}});
        let assertions = camera_assertions(&state);
        let mode = assertions
            .iter()
            .find(|a| a.id == "camera.mode_present")
            .expect("mode assertion missing");
        assert!(!mode.ok);
        assert_eq!(mode.severity, "fail");
    }

    #[test]
    fn resource_deltas_flags_only_tick_motion_as_partial() {
        let dir = temp_root("xtask_deltas_only_tick");
        let deltas = json!({
            "tick": 308,
            "resource_deltas": [
                {"path": "tick", "current": 308.0, "previous": 295.0, "delta": 13.0, "delta_abs": 13.0}
            ]
        });
        fs::write(dir.join("diff.json"), deltas.to_string()).unwrap();
        let assertions = resource_deltas_assertions(&dir);
        let a = assertions
            .iter()
            .find(|a| a.id == "state.resource_deltas_have_nontick_motion")
            .expect("resource_deltas assertion missing");
        assert!(!a.ok);
        assert_eq!(a.severity, "partial");
        assert_eq!(a.actual, json!(0));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resource_deltas_passes_when_final_state_proves_gameplay_motion() {
        let dir = temp_root("xtask_deltas_final_state_motion");
        let deltas = json!({
            "tick": 100,
            "resource_deltas": []
        });
        let final_state = json!({
            "player": {"nations_founded": 1, "blocks_gathered": 0, "monsters_killed": 0},
            "eco_cycle": {"fruit_eaten": 12, "fruit_grown": 4, "plants_grown": 3},
            "nations": {"total_nations": 2}
        });
        fs::write(dir.join("diff.json"), deltas.to_string()).unwrap();
        fs::write(dir.join("final_state.json"), final_state.to_string()).unwrap();
        let assertions = resource_deltas_assertions(&dir);
        let a = assertions
            .iter()
            .find(|a| a.id == "state.resource_deltas_have_nontick_motion")
            .expect("resource_deltas assertion missing");
        assert!(a.ok);
        assert!(a.actual.as_u64().unwrap_or(0) >= 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn resource_deltas_passes_with_nontick_motion() {
        let dir = temp_root("xtask_deltas_with_nontick");
        let deltas = json!({
            "tick": 308,
            "resource_deltas": [
                {"path": "tick", "current": 308.0, "previous": 295.0, "delta": 13.0, "delta_abs": 13.0},
                {"path": "pool.food", "current": 50.0, "previous": 45.0, "delta": 5.0, "delta_abs": 5.0}
            ]
        });
        fs::write(dir.join("diff.json"), deltas.to_string()).unwrap();
        let assertions = resource_deltas_assertions(&dir);
        let a = assertions
            .iter()
            .find(|a| a.id == "state.resource_deltas_have_nontick_motion")
            .expect("resource_deltas assertion missing");
        assert!(a.ok);
        assert_eq!(a.actual, json!(1));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn eco_support_assertions_accept_existing_animals() {
        let state = json!({
            "eco_cycle": {
                "clouds": 3,
                "rainfall": 10.0,
                "plants_grown": 5,
                "rabbits_born": 0,
                "rabbits": 4,
                "wildlife_born": 0,
                "wildlife": 2
            }
        });
        let assertions = eco_cycle_assertions(&state);
        for id in [
            "eco.plants_support_small_animals",
            "eco.small_animals_support_wildlife",
        ] {
            let a = assertions
                .iter()
                .find(|a| a.id == id)
                .expect("assertion missing");
            assert!(a.ok, "{id} should pass with existing animals");
        }
    }

    #[test]
    fn eco_support_assertions_still_fail_without_animals() {
        let state = json!({
            "eco_cycle": {
                "clouds": 3,
                "rainfall": 10.0,
                "plants_grown": 5,
                "rabbits_born": 0,
                "rabbits": 0,
                "wildlife_born": 0,
                "wildlife": 0
            }
        });
        let assertions = eco_cycle_assertions(&state);
        for id in [
            "eco.plants_support_small_animals",
            "eco.small_animals_support_wildlife",
        ] {
            let a = assertions
                .iter()
                .find(|a| a.id == id)
                .expect("assertion missing");
            assert!(!a.ok, "{id} should fail without born or current animals");
            assert_eq!(a.severity, "partial");
        }
    }

    #[test]
    fn regression_monsters_drop_below_ratio_is_partial() {
        let prev = json!({"monsters": {"current": 60}});
        let curr = json!({"monsters": {"current": 20}});
        let assertions = regression_assertions(&curr, &prev);
        let monsters = assertions
            .iter()
            .find(|a| a.id == "regression.monsters_not_collapsed")
            .expect("monsters regression assertion missing");
        assert!(!monsters.ok);
        assert_eq!(monsters.severity, "partial");
    }

    #[test]
    fn regression_nations_drop_is_hard_failure() {
        let prev = json!({"nations": {"total_nations": 3}});
        let curr = json!({"nations": {"total_nations": 2}});
        let assertions = regression_assertions(&curr, &prev);
        let nations = assertions
            .iter()
            .find(|a| a.id == "regression.nations_total_non_decreasing")
            .expect("nations regression assertion missing");
        assert!(!nations.ok);
        assert_eq!(nations.severity, "fail");
    }

    #[test]
    fn regression_nations_growth_or_steady_is_ok() {
        let prev = json!({"nations": {"total_nations": 3}});
        let curr = json!({"nations": {"total_nations": 4}});
        let assertions = regression_assertions(&curr, &prev);
        let nations = assertions
            .iter()
            .find(|a| a.id == "regression.nations_total_non_decreasing")
            .expect("nations regression assertion missing");
        assert!(nations.ok);
    }

    #[test]
    fn regression_skips_independent_camera_contexts() {
        let prev = json!({
            "role": "client_offline",
            "camera": {"mode": "TopDown"},
            "nations": {"total_nations": 3}
        });
        let curr = json!({
            "role": "client_offline",
            "camera": {"mode": "ThirdPerson"},
            "nations": {"total_nations": 1}
        });
        assert!(regression_assertions(&curr, &prev).is_empty());
    }

    #[test]
    fn stderr_scan_counts_deserialize_invalid_entity() {
        let dir = temp_root("xtask_stderr_deser");
        fs::write(
            dir.join("loop.err.log"),
            "INFO start\nERROR Attempting to deserialize an invalid entity.\nERROR Attempting to deserialize an invalid entity.\nINFO done\n",
        )
        .unwrap();
        let mut scan = StderrScan::default();
        scan_stderr_dir(&dir, &mut scan);
        assert_eq!(scan.deserialize_invalid_count, 2);
        assert_eq!(scan.files_scanned, 1);
        assert_eq!(scan.out_of_bounds_count, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stderr_scan_counts_out_of_bounds_and_voxel_overflow() {
        let dir = temp_root("xtask_stderr_oob");
        fs::write(
            dir.join("loop.err.log"),
            "INFO a\nWARN OUT OF BOUNDS pos=1\nINFO b\n体素过多 (3100)\nINFO c\nOUT OF BOUNDS pos=2\n",
        )
        .unwrap();
        let mut scan = StderrScan::default();
        scan_stderr_dir(&dir, &mut scan);
        assert_eq!(scan.out_of_bounds_count, 2);
        assert_eq!(scan.voxel_overflow_count, 1);
        assert_eq!(scan.deserialize_invalid_count, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stderr_scan_ignores_non_err_log_files() {
        let dir = temp_root("xtask_stderr_filter");
        fs::write(
            dir.join("loop.log"),
            "Attempting to deserialize an invalid entity.\n",
        )
        .unwrap();
        let mut scan = StderrScan::default();
        scan_stderr_dir(&dir, &mut scan);
        assert_eq!(scan.files_scanned, 0);
        assert_eq!(scan.deserialize_invalid_count, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn archive_error_logs_writes_all_error_lines_with_sources() {
        let root = temp_root("xtask_error_archive");
        fs::create_dir_all(root.join("screenshots")).unwrap();
        fs::create_dir_all(root.join("run-logs")).unwrap();
        fs::write(
            root.join("screenshots/loop_run.log.err"),
            "INFO start\nERROR runtime failed\nwarn only\n",
        )
        .unwrap();
        fs::write(
            root.join("run-logs/build_loop.log"),
            "Compiling crate\nerror[E0599]: missing method\nfatal error LNK1120\n",
        )
        .unwrap();
        let out = root.join("screenshots/iter_1");
        fs::create_dir_all(&out).unwrap();

        let summary = archive_error_logs(&root, &out).unwrap();

        assert_eq!(summary.error_line_count, 3);
        let text = fs::read_to_string(out.join("error_logs.txt")).unwrap();
        assert!(text.contains("screenshots/loop_run.log.err:2"));
        assert!(text.contains("ERROR runtime failed"));
        assert!(!text.contains("INFO start"));
        assert!(!text.contains("warn only"));
        assert!(text.contains("run-logs/build_loop.log:2"));
        assert!(text.contains("error[E0599]: missing method"));
        assert!(text.contains("run-logs/build_loop.log:3"));
        assert!(text.contains("fatal error LNK1120"));

        let json: Value =
            serde_json::from_str(&fs::read_to_string(out.join("error_logs.json")).unwrap())
                .unwrap();
        assert_eq!(json["error_line_count"], json!(3));
        assert_eq!(json["entries"][0]["line"], json!(2));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn archive_error_logs_for_files_ignores_unlisted_old_logs() {
        let root = temp_root("xtask_error_archive_files");
        fs::create_dir_all(root.join("run-logs")).unwrap();
        fs::create_dir_all(root.join(".harness/scratch")).unwrap();
        let play = root.join("run-logs/play.log.err");
        fs::write(&play, "ERROR fresh crash\n").unwrap();
        fs::write(
            root.join(".harness/scratch/old.log"),
            "ERROR stale harness noise\n",
        )
        .unwrap();

        let summary = archive_error_logs_for_files(&root, &root.join("run-logs"), &[play]).unwrap();

        assert_eq!(summary.error_line_count, 1);
        let text = fs::read_to_string(root.join("run-logs/error_logs.txt")).unwrap();
        assert!(text.contains("ERROR fresh crash"));
        assert!(!text.contains("stale harness noise"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn nature_artifact_ignores_aggregate_archived_error_log() {
        let root = temp_root("nature_current_errors");
        let iter_dir = root.join("screenshots/iter_01");
        fs::create_dir_all(&iter_dir).unwrap();
        fs::write(
            iter_dir.join("error_logs.txt"),
            "old-preview.log: ERROR stale failure\n",
        )
        .unwrap();
        let state = json!({
            "nature": {
                "tick": 2,
                "cloud_count": 1,
                "rainfall": 1.0,
                "soil_moisture": 1.0,
                "plant_count": 1,
                "animal_count": 1,
                "animal_food_available": 1
            }
        });

        let artifact = build_nature_artifact(&root, &iter_dir, &state, None);

        assert!(artifact.after.errors.is_empty());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn server_tick_alignment_allows_sampling_lag_but_not_lead_or_stale_data() {
        assert!(tick_alignment(Some(100), Some(100)));
        assert!(tick_alignment(Some(95), Some(100)));
        assert!(tick_alignment(Some(90), Some(100)));
        assert!(!tick_alignment(Some(89), Some(100)));
        assert!(!tick_alignment(Some(101), Some(100)));
        assert!(!tick_alignment(None, Some(100)));
    }

    fn temp_root(name: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("{name}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
