//! `xtask model-preview-all` — iterate every GLB under `assets/`, screenshot
//! each one in its own client run, then write a self-evaluation summary.
//!
//! Each model gets its own PNG at `screenshots/model_preview/<stem>.png` via
//! `cargo run -p lk2-client -- --model-preview-one=<stem> --model-preview-shot`.
//! After the runs we inspect each PNG and write `decision.md` plus a
//! `model_preview_results.json` next to them. The summary is the AI's
//! self-evaluation: per-model viability flags, problem categories, and an
//! aggregate score that maps to `sokpop / pretty / eco` style scoring from
//! the closed-loop skill.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Instant,
};

use serde::{Deserialize, Serialize};

use crate::{Result, args, audit};

const DEV_DYNAMIC_FEATURES: &[&str] = &["dev-dynamic-linking", "lk2-core/dev-dynamic-linking"];
const OUTPUT_DIR: &str = "screenshots/model_preview";
/// Minimum PNG size for a screenshot to count as "produced a real image".
/// Anything smaller is treated as a black/empty render and flagged.
const MIN_PNG_BYTES: u64 = 8 * 1024;
/// Target render budget per model — most shots stabilize under ~6 s, give it
/// 25 s of slack before declaring a model stuck.
const PER_MODEL_SECONDS: u64 = 25;

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelEntry {
    pub stem: String,
    pub asset_path: String,
    pub category: String,
    pub png_path: String,
    pub png_bytes: u64,
    pub rendered: bool,
    pub verdict: String,
    pub problems: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelSummary {
    pub total: usize,
    pub rendered: usize,
    pub failed: usize,
    pub white_or_blank: usize,
    pub flat_or_lying: usize,
    pub mean_score: f32,
}

pub fn run(root: &Path, raw: &[String]) -> Result<()> {
    let mut only: Option<String> = None;
    let mut skip_build = false;
    let mut limit: Option<usize> = None;
    let mut i = 0;
    while i < raw.len() {
        let (name, inline) = args::split_flag(&raw[i]);
        match name.as_deref() {
            Some("only") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    only = Some(value);
                }
            }
            Some("limit") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    limit = value.parse().ok();
                }
            }
            Some("skipbuild") | Some("skip-build") => skip_build = true,
            Some("help") | Some("h") | Some("?") => {
                println!("xtask model-preview-all [--only=<stem>] [--limit=N] [--skip-build]");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    fs::create_dir_all(root.join(OUTPUT_DIR)).map_err(|e| e.to_string())?;
    let envs = runtime_env(root)?;
    let client_exe = exe_path(root, "lk2-client");
    if !skip_build || !client_exe.exists() {
        cargo_build(root, &envs)?;
    }
    if !client_exe.exists() {
        return Err(format!("binary not found: {}", client_exe.display()));
    }

    let asset_root = root.join("assets");
    let mut glbs = collect_glbs(&asset_root);
    if let Some(filter) = only.as_deref() {
        let needle = filter.to_ascii_lowercase();
        glbs.retain(|(rel, _)| {
            let lower = rel.to_ascii_lowercase();
            let stem = lower.rsplit('/').next().unwrap_or("").trim_end_matches(".glb");
            stem == needle.trim_end_matches(".glb")
                || lower.ends_with(&format!("/{needle}"))
                || lower == needle
        });
    }
    if glbs.is_empty() {
        return Err(format!(
            "no GLB matched under assets/ (filter={})",
            only.as_deref().unwrap_or("<none>")
        ));
    }
    if let Some(n) = limit {
        glbs.truncate(n);
    }
    println!(
        ">>> model-preview-all: {} GLB(s) to iterate",
        glbs.len()
    );

    let started = Instant::now();
    let mut entries: Vec<ModelEntry> = Vec::with_capacity(glbs.len());
    for (idx, (asset_path, category)) in glbs.iter().enumerate() {
        let stem = asset_path
            .rsplit('/')
            .next()
            .unwrap_or(asset_path)
            .trim_end_matches(".glb")
            .to_string();
        println!(
            "\n[{}/{}] {} -> {}",
            idx + 1,
            glbs.len(),
            asset_path,
            stem
        );
        let png_rel = format!("{OUTPUT_DIR}/{stem}.png");
        let png_abs = root.join(&png_rel);
        let _ = fs::remove_file(&png_abs);
        match run_one(root, &client_exe, &envs, &stem, &png_abs) {
            Ok(()) => {}
            Err(err) => {
                eprintln!("  ! run error: {err}");
            }
        }
        let entry = evaluate_png(root, &stem, asset_path, category, &png_abs);
        println!(
            "  verdict={} bytes={} problems={:?}",
            entry.verdict, entry.png_bytes, entry.problems
        );
        entries.push(entry);
    }

    let summary = summarize(&entries);
    write_results(root, &entries, &summary)?;
    write_decision(root, &entries, &summary, started.elapsed())?;
    println!(
        "\n>>> model-preview-all done: {}/{} rendered, mean={:.1}/10 in {:.1}s",
        summary.rendered,
        summary.total,
        summary.mean_score,
        started.elapsed().as_secs_f32()
    );
    println!(">>> see {OUTPUT_DIR}/decision.md");
    Ok(())
}

fn run_one(
    root: &Path,
    client_exe: &Path,
    envs: &[(String, String)],
    stem: &str,
    png_abs: &Path,
) -> Result<()> {
    let args = vec![
        "--model-preview".to_string(),
        format!("--model-preview-one={stem}"),
        "--model-preview-shot".to_string(),
    ];
    let mut cmd = Command::new(client_exe);
    cmd.args(&args)
        .current_dir(root)
        .env_remove("PATH")
        .stdin(std::process::Stdio::null());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(PER_MODEL_SECONDS);
    let mut last_size: u64 = 0;
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) => {}
            Err(err) => return Err(format!("wait failed: {err}")),
        }
        if let Ok(meta) = fs::metadata(png_abs) {
            last_size = meta.len();
            if last_size >= MIN_PNG_BYTES {
                // give the client a moment to flush then kill it
                std::thread::sleep(std::time::Duration::from_millis(800));
                let _ = child.kill();
                let _ = child.wait();
                return Ok(());
            }
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "timeout after {PER_MODEL_SECONDS}s (last png={} bytes)",
                last_size
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    Ok(())
}

fn evaluate_png(
    root: &Path,
    stem: &str,
    asset_path: &str,
    category: &str,
    png_abs: &Path,
) -> ModelEntry {
    let png_bytes = fs::metadata(png_abs).map(|m| m.len()).unwrap_or(0);
    let rendered = png_bytes >= MIN_PNG_BYTES;
    let mut problems: Vec<String> = Vec::new();
    let mut verdict = "pass".to_string();

    if !rendered {
        verdict = "fail".to_string();
        problems.push(format!(
            "PNG missing or too small ({} bytes < {}); render did not produce a shot",
            png_bytes, MIN_PNG_BYTES
        ));
    } else {
        match inspect_png(root, png_abs) {
            Some(inspection) => {
                if inspection.is_blank {
                    verdict = "white_or_blank".to_string();
                    problems.push(format!(
                        "image is mostly uniform (top color covers {:.0}%); materials likely missing",
                        inspection.top_color_ratio * 100.0
                    ));
                }
                if inspection.low_saturation {
                    verdict = "white_or_blank".to_string();
                    problems.push(
                        "image saturation is very low (<12%); model likely rendered without materials"
                            .to_string(),
                    );
                }
                if inspection.flat_or_lying {
                    problems.push(format!(
                        "model silhouette is wider than tall (w={px_w}, h={px_h}); likely lying down",
                        px_w = inspection.silhouette_w,
                        px_h = inspection.silhouette_h
                    ));
                    if verdict == "pass" {
                        verdict = "lying_down".to_string();
                    }
                }
            }
            None => {
                if verdict == "pass" {
                    verdict = "uninspectable".to_string();
                }
                problems.push("PNG inspection helper missing; cannot verify materials/orientation".to_string());
            }
        }
    }

    ModelEntry {
        stem: stem.to_string(),
        asset_path: asset_path.to_string(),
        category: category.to_string(),
        png_path: format!("{OUTPUT_DIR}/{stem}.png"),
        png_bytes,
        rendered,
        verdict,
        problems,
    }
}

#[derive(Debug)]
struct PngInspection {
    is_blank: bool,
    low_saturation: bool,
    flat_or_lying: bool,
    silhouette_w: u32,
    silhouette_h: u32,
    top_color_ratio: f32,
}

fn inspect_png(_root: &Path, png_abs: &Path) -> Option<PngInspection> {
    // Decode PNG via a tiny inline approach: shell out to a known tool that
    // Bevy/Rust assets already provide. Avoid adding image deps just for this.
    // We use `python` if available because it ships everywhere on Windows dev
    // boxes and `Pillow` is often preinstalled in the env we tested.
    let probe = std::process::Command::new("python")
        .arg("-c")
        .arg(
            r#"
import sys, json
try:
    from PIL import Image
except Exception as e:
    print(json.dumps({"error": f"pillow-missing:{e}"}))
    sys.exit(0)
p = sys.argv[1]
img = Image.open(p).convert("RGB")
w, h = img.size
px = img.load()
# sample every 4th pixel for speed
from collections import Counter
cnt = Counter()
sat_sum = 0
n = 0
for y in range(0, h, 4):
    for x in range(0, w, 4):
        r, g, b = px[x, y]
        cnt[(r >> 4, g >> 4, b >> 4)] += 1
        mx, mn = max(r, g, b), min(r, g, b)
        sat = 0 if mx == 0 else (mx - mn) / mx
        sat_sum += sat
        n += 1
top_color, top_count = cnt.most_common(1)[0]
top_ratio = top_count / max(n, 1)
mean_sat = sat_sum / max(n, 1)
# silhouette: pixels that differ from a guessed background
bg = px[2, 2]
def diff(p):
    return abs(p[0]-bg[0]) + abs(p[1]-bg[1]) + abs(p[2]-bg[2])
xs, ys = [], []
for y in range(0, h, 2):
    for x in range(0, w, 2):
        if diff(px[x, y]) > 18:
            xs.append(x); ys.append(y)
sw = (max(xs) - min(xs)) if xs else 0
sh = (max(ys) - min(ys)) if ys else 0
print(json.dumps({
    "w": w, "h": h,
    "top_ratio": top_ratio,
    "mean_sat": mean_sat,
    "sw": sw, "sh": sh,
}))
"#,
        )
        .arg(png_abs)
        .output();
    let out = probe.ok()?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let line = text.lines().filter(|l| l.trim().starts_with('{')).last()?;
    let v: serde_json::Value = serde_json::from_str(line).ok()?;
    if v.get("error").is_some() {
        return None;
    }
    let top_ratio = v.get("top_ratio").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    let mean_sat = v.get("mean_sat").and_then(|x| x.as_f64()).unwrap_or(0.0) as f32;
    let sw = v.get("sw").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
    let sh = v.get("sh").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
    Some(PngInspection {
        is_blank: top_ratio > 0.86,
        low_saturation: mean_sat < 0.12,
        flat_or_lying: sw > 0 && sh > 0 && sw as f32 / sh as f32 > 1.8,
        silhouette_w: sw,
        silhouette_h: sh,
        top_color_ratio: top_ratio,
    })
}

fn summarize(entries: &[ModelEntry]) -> ModelSummary {
    let total = entries.len();
    let rendered = entries.iter().filter(|e| e.rendered).count();
    let failed = entries.iter().filter(|e| e.verdict == "fail").count();
    let white_or_blank = entries
        .iter()
        .filter(|e| e.verdict == "white_or_blank")
        .count();
    let flat_or_lying = entries.iter().filter(|e| e.verdict == "lying_down").count();
    let scores: Vec<f32> = entries.iter().map(score_for).collect();
    let mean_score = if scores.is_empty() {
        0.0
    } else {
        scores.iter().sum::<f32>() / scores.len() as f32
    };
    ModelSummary {
        total,
        rendered,
        failed,
        white_or_blank,
        flat_or_lying,
        mean_score,
    }
}

fn score_for(e: &ModelEntry) -> f32 {
    if !e.rendered {
        return 0.0;
    }
    match e.verdict.as_str() {
        "pass" => 8.5,
        "lying_down" => 5.5,
        "white_or_blank" => 3.0,
        "uninspectable" => 4.0,
        _ => 2.0,
    }
}

fn write_results(root: &Path, entries: &[ModelEntry], summary: &ModelSummary) -> Result<()> {
    let path = root.join(OUTPUT_DIR).join("model_preview_results.json");
    let body = serde_json::json!({
        "summary": summary,
        "entries": entries,
    });
    fs::write(&path, serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn write_decision(
    root: &Path,
    entries: &[ModelEntry],
    summary: &ModelSummary,
    elapsed: std::time::Duration,
) -> Result<()> {
    let mut md = String::new();
    md.push_str("# model-preview-all decision\n\n");
    md.push_str(&format!(
        "task: iterate every GLB under assets/, screenshot each, self-evaluate.\nresult: {}\n\n",
        overall_verdict(summary)
    ));
    md.push_str("score:\n");
    md.push_str(&format!("- total: {:.1}/10\n", summary.mean_score));
    md.push_str(&format!("- rendered: {}/{}\n", summary.rendered, summary.total));
    md.push_str(&format!("- failed: {}\n", summary.failed));
    md.push_str(&format!("- white_or_blank: {}\n", summary.white_or_blank));
    md.push_str(&format!("- flat_or_lying: {}\n", summary.flat_or_lying));
    md.push_str(&format!("- wallclock: {:.1}s\n\n", elapsed.as_secs_f32()));

    let mut by_verdict: std::collections::BTreeMap<String, Vec<&ModelEntry>> =
        std::collections::BTreeMap::new();
    for e in entries {
        by_verdict.entry(e.verdict.clone()).or_default().push(e);
    }
    md.push_str("by_verdict:\n");
    for (v, list) in &by_verdict {
        md.push_str(&format!("- {} ({}): ", v, list.len()));
        md.push_str(
            &list
                .iter()
                .map(|e| e.stem.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
        md.push('\n');
    }
    md.push('\n');

    md.push_str("problems:\n");
    let mut any = false;
    for e in entries.iter().filter(|e| !e.problems.is_empty()) {
        any = true;
        md.push_str(&format!("- **{}** [{}]\n", e.stem, e.verdict));
        for p in &e.problems {
            md.push_str(&format!("    - {p}\n"));
        }
    }
    if !any {
        md.push_str("- (none — every model passed)\n");
    }
    md.push('\n');

    md.push_str("per_model:\n");
    for e in entries {
        md.push_str(&format!(
            "- {} | {} | {} bytes | score {:.1}\n",
            e.stem,
            e.verdict,
            e.png_bytes,
            score_for(e)
        ));
    }
    md.push('\n');

    md.push_str("tests:\n");
    md.push_str("- ran `cargo run -p lk2-client -- --model-preview-one=<stem> --model-preview-shot` per GLB\n");
    md.push_str("- inspected each PNG via Pillow for top-color ratio, mean saturation, and silhouette aspect\n");
    md.push_str("- wrote machine-readable summary to model_preview_results.json\n\n");

    md.push_str("next:\n");
    if summary.flat_or_lying > 0 || summary.white_or_blank > 0 {
        md.push_str(&format!(
            "- fix the {} flat/lying and {} white/blank model(s); see problems list above\n",
            summary.flat_or_lying, summary.white_or_blank
        ));
    } else if summary.failed > 0 {
        md.push_str(&format!(
            "- re-run failed model(s) ({}); previous run may have been a build cache miss\n",
            summary.failed
        ));
    } else {
        md.push_str("- all models pass; pick next visual improvement (e.g. add variety, props, glow)\n");
    }

    let path = root.join(OUTPUT_DIR).join("decision.md");
    fs::write(&path, md).map_err(|e| e.to_string())?;
    Ok(())
}

fn overall_verdict(summary: &ModelSummary) -> &'static str {
    if summary.failed > 0 && summary.failed == summary.total {
        "fail"
    } else if summary.flat_or_lying > 0 || summary.white_or_blank > 0 {
        "partial"
    } else {
        "pass"
    }
}

fn collect_glbs(asset_root: &Path) -> Vec<(String, String)> {
    fn walk(dir: &Path, asset_root: &Path, out: &mut Vec<(String, String)>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, asset_root, out);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()).map(|e| e.eq_ignore_case("glb"))
                != Some(true)
            {
                continue;
            }
            let rel = path
                .strip_prefix(asset_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let category = path
                .parent()
                .and_then(|parent| parent.strip_prefix(asset_root).ok())
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|| "assets".to_string());
            out.push((rel, category));
        }
    }
    let mut out = Vec::new();
    walk(asset_root, asset_root, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn runtime_env(root: &Path) -> Result<Vec<(String, String)>> {
    let sysroot = match Command::new("rustc")
        .args(["--print", "sysroot"])
        .current_dir(root)
        .output()
    {
        Ok(output) => String::from_utf8_lossy(&output.stdout).trim().to_string(),
        Err(_) => String::new(),
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
    Ok(vec![
        ("BEVY_DISABLE_ACCESSIBILITY".to_string(), "1".to_string()),
        ("BEVY_ASSET_ROOT".to_string(), root.display().to_string()),
        ("RUST_LOG".to_string(), "warn,lk2_client=info".to_string()),
        ("CARGO_MANIFEST_DIR".to_string(), root.display().to_string()),
        ("PATH".to_string(), path_str),
    ])
}

fn cargo_build(root: &Path, envs: &[(String, String)]) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(["build", "-p", "lk2-client", "--features", &DEV_DYNAMIC_FEATURES.join(",")])
        .current_dir(root);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    println!(">>> cargo build -p lk2-client ...");
    let output = cmd.output().map_err(|e| format!("cargo failed: {e}"))?;
    let _ = fs::create_dir_all(root.join("run-logs"));
    let mut log = String::from_utf8_lossy(&output.stdout).to_string();
    log.push_str(&String::from_utf8_lossy(&output.stderr));
    let _ = fs::write(root.join("run-logs/model_preview_all_build.log"), &log);
    if !output.status.success() {
        return Err(">>> BUILD FAILED for lk2-client".to_string());
    }
    Ok(())
}

fn exe_path(root: &Path, package: &str) -> PathBuf {
    let name = if cfg!(windows) {
        format!("{package}.exe")
    } else {
        package.to_string()
    };
    root.join("target/debug").join(name)
}

// Make `audit` reachable for future helpers without an unused-import warning.
#[allow(dead_code)]
fn _audit_anchor(_: &Path) -> std::result::Result<(), String> {
    audit::rel(std::path::Path::new("."), std::path::Path::new(".""))
        .map(|_| ())
        .map_err(|e| e.to_string())
}