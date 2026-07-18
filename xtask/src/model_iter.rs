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
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Instant,
};

use serde::{Deserialize, Serialize};

use crate::{Result, args, health as rust_health};

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
    pub file_size_bytes: u64,
    pub asset_info: Option<serde_json::Value>,
    pub asset_info_error: Option<String>,
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
    let mut only: Vec<String> = Vec::new();
    let mut only_requested = false;
    let mut skip_build = false;
    let mut skip_render = false;
    let mut limit: Option<usize> = None;
    let mut i = 0;
    while i < raw.len() {
        let (name, inline) = args::split_flag(&raw[i]);
        match name.as_deref() {
            Some("only") => {
                only_requested = true;
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    only.extend(parse_model_filters(&value));
                }
            }
            Some("limit") => {
                if let Some(value) = inline.or_else(|| args::take_next(raw, &mut i)) {
                    limit = value.parse().ok();
                }
            }
            Some("skipbuild") | Some("skip-build") => skip_build = true,
            Some("skiprender") | Some("skip-render") | Some("reevaluate") | Some("re-eval") => {
                skip_render = true
            }
            Some("help") | Some("h") | Some("?") => {
                println!(
                    "xtask model-preview-all [--only=<stem[,stem...]>] [--limit=N] [--skip-build] [--skip-render]"
                );
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    fs::create_dir_all(root.join(OUTPUT_DIR)).map_err(|e| e.to_string())?;
    fs::create_dir_all(root.join("run-logs")).map_err(|e| e.to_string())?;
    fs::write(root.join("run-logs/model_preview_all.log"), "").map_err(|e| e.to_string())?;
    fs::write(root.join("run-logs/model_preview_all.log.err"), "").map_err(|e| e.to_string())?;
    let client_exe = exe_path(root, "lk2-client");
    let envs = runtime_env(root)?;
    if !client_exe.exists() {
        // No binary at all → must build. Build with the same features the rest
        // of the project uses; this path is rare on a hot dev loop.
        cargo_build(root, &envs)?;
    } else if !skip_build {
        println!(
            ">>> using existing {} ({} MB); pass --skip-build to skip rebuild",
            client_exe.display(),
            fs::metadata(&client_exe)
                .map(|m| m.len() / 1024 / 1024)
                .unwrap_or(0)
        );
    }
    if !client_exe.exists() {
        return Err(format!("binary not found: {}", client_exe.display()));
    }

    if only_requested && only.is_empty() {
        return Err("--only requires at least one model stem or path".to_string());
    }
    let asset_root = root.join("assets");
    let mut glbs = collect_glbs(&asset_root);
    if !only.is_empty() {
        glbs.retain(|(rel, _)| only.iter().any(|filter| model_path_matches(rel, filter)));
        if glbs.is_empty() {
            return Err(format!(
                "no GLB matched --only='{}' (try stem or assets-relative path)",
                only.join(",")
            ));
        }
    }
    if glbs.is_empty() {
        return Err(format!(
            "no GLB matched under assets/ (filter={})",
            if only.is_empty() {
                "<none>".to_string()
            } else {
                only.join(",")
            }
        ));
    }
    if let Some(n) = limit {
        glbs.truncate(n);
    }
    println!(">>> model-preview-all: {} GLB(s) to iterate", glbs.len());

    let started = Instant::now();
    let mut entries: Vec<ModelEntry> = Vec::with_capacity(glbs.len());
    for (idx, (asset_path, category)) in glbs.iter().enumerate() {
        let stem = asset_path
            .rsplit('/')
            .next()
            .unwrap_or(asset_path)
            .trim_end_matches(".glb")
            .to_string();
        println!("\n[{}/{}] {} -> {}", idx + 1, glbs.len(), asset_path, stem);
        let png_rel = format!("{OUTPUT_DIR}/{stem}.png");
        let png_abs = root.join(&png_rel);
        if !skip_render {
            let _ = fs::remove_file(&png_abs);
        }
        let mut entry = if skip_render {
            evaluate_png(root, &stem, asset_path, category, &png_abs)
        } else {
            match run_one(root, &client_exe, &envs, &stem, &png_abs) {
                Ok(()) => evaluate_png(root, &stem, asset_path, category, &png_abs),
                Err(err) => {
                    eprintln!("  ! run error: {err}");
                    evaluate_png(root, &stem, asset_path, category, &png_abs)
                }
            }
        };
        let (file_size_bytes, asset_info, asset_info_error) =
            read_preview_manifest_entry(root, asset_path);
        entry.file_size_bytes = file_size_bytes;
        entry.asset_info = asset_info;
        entry.asset_info_error = asset_info_error;
        println!(
            "  verdict={} bytes={} problems={:?}",
            entry.verdict, entry.png_bytes, entry.problems
        );
        entries.push(entry);
    }

    let summary = summarize(&entries);
    write_results(root, &entries, &summary)?;
    write_decision(root, &entries, &summary, started.elapsed())?;
    let error_summary = rust_health::archive_error_logs_for_files(
        root,
        &root.join("run-logs"),
        &[
            root.join("run-logs/model_preview_all.log"),
            root.join("run-logs/model_preview_all.log.err"),
        ],
    )?;
    println!(
        "\n>>> model-preview-all done: {}/{} rendered, mean={:.1}/10 in {:.1}s",
        summary.rendered,
        summary.total,
        summary.mean_score,
        started.elapsed().as_secs_f32()
    );
    println!(
        ">>> archived {} error lines from {} log files to run-logs/error_logs.json and run-logs/error_logs.txt",
        error_summary.error_line_count, error_summary.files_scanned
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
    append_run_header(root, stem, &args)?;
    let stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("run-logs/model_preview_all.log"))
        .map_err(|e| e.to_string())?;
    let stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("run-logs/model_preview_all.log.err"))
        .map_err(|e| e.to_string())?;
    let mut cmd = Command::new(client_exe);
    cmd.args(&args)
        .current_dir(root)
        .env_remove("PATH")
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    let deadline = Instant::now() + std::time::Duration::from_secs(PER_MODEL_SECONDS);
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
        if Instant::now() >= deadline {
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

fn append_run_header(root: &Path, stem: &str, args: &[String]) -> Result<()> {
    let mut stdout = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("run-logs/model_preview_all.log"))
        .map_err(|e| e.to_string())?;
    let mut stderr = OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("run-logs/model_preview_all.log.err"))
        .map_err(|e| e.to_string())?;
    writeln!(stdout, "\n=== model-preview {stem} {:?} ===", args).map_err(|e| e.to_string())?;
    writeln!(stderr, "\n=== model-preview {stem} {:?} ===", args).map_err(|e| e.to_string())?;
    Ok(())
}

fn read_preview_manifest_entry(
    root: &Path,
    asset_path: &str,
) -> (u64, Option<serde_json::Value>, Option<String>) {
    let path = root.join(OUTPUT_DIR).join("manifest.json");
    let Ok(body) = fs::read_to_string(path) else {
        return (0, None, Some("model preview manifest missing".to_string()));
    };
    let Ok(document) = serde_json::from_str::<serde_json::Value>(&body) else {
        return (
            0,
            None,
            Some("model preview manifest is not valid JSON".to_string()),
        );
    };
    let models = document
        .get("models")
        .and_then(serde_json::Value::as_array)
        .or_else(|| document.as_array());
    let Some(entry) = models.and_then(|models| {
        models
            .iter()
            .find(|entry| entry.get("path").and_then(serde_json::Value::as_str) == Some(asset_path))
    }) else {
        return (
            0,
            None,
            Some(format!(
                "model preview manifest has no entry for {asset_path}"
            )),
        );
    };
    let asset_info = entry
        .get("asset_info")
        .filter(|value| !value.is_null())
        .cloned();
    let asset_info_error = entry["asset_info_error"]
        .as_str()
        .map(str::to_string)
        .or_else(|| {
            asset_info
                .is_none()
                .then_some("asset metadata missing".to_string())
        });
    (
        entry["file_size_bytes"].as_u64().unwrap_or_default(),
        asset_info,
        asset_info_error,
    )
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
                problems.push(
                    "PNG inspection helper missing; cannot verify materials/orientation"
                        .to_string(),
                );
            }
        }
    }

    ModelEntry {
        stem: stem.to_string(),
        asset_path: asset_path.to_string(),
        category: category.to_string(),
        file_size_bytes: 0,
        asset_info: None,
        asset_info_error: None,
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
    use std::io::BufReader;

    let file = fs::File::open(png_abs).ok()?;
    let reader = BufReader::new(file);
    let decoder = png::Decoder::new(reader);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let bytes = &buf[..];
    let w = info.width;
    let h = info.height;
    let color = info.color_type;

    let rgb: Vec<u8> = match color {
        png::ColorType::Rgb => bytes.to_vec(),
        png::ColorType::Rgba => bytes.chunks(4).flat_map(|c| [c[0], c[1], c[2]]).collect(),
        png::ColorType::Grayscale => bytes.iter().flat_map(|v| [*v, *v, *v]).collect(),
        png::ColorType::GrayscaleAlpha => {
            let mut out = Vec::with_capacity(bytes.len() / 2 * 3);
            for px in bytes.chunks(2) {
                out.extend_from_slice(&[px[0], px[0], px[0]]);
            }
            out
        }
        _ => return None,
    };

    // The model preview always puts the model near the center of the frame
    // on a small grey disc, with the green ground filling the rest. To stop
    // counting the ground + horizon as "the model silhouette", we restrict
    // all inspection to a 60% central crop that contains the model + disc.
    let cx = w as i32 / 2;
    let cy = h as i32 / 2;
    let half_w = (w as i32 * 30) / 100;
    let half_h = (h as i32 * 30) / 100;
    let x0 = (cx - half_w).max(0) as usize;
    let y0 = (cy - half_h).max(0) as usize;
    let x1 = (cx + half_w).min(w as i32) as usize;
    let y1 = (cy + half_h).min(h as i32) as usize;

    let stride = w as usize * 3;
    let mut sat_sum: f32 = 0.0;
    let mut n: u32 = 0;
    // The disc is roughly `Color::srgb(0.52, 0.54, 0.48)` and the ground is
    // `Color::srgb(0.48, 0.55, 0.42)`. They're almost identical, so we use a
    // very strict deviation threshold (vs. local mean) to find model pixels.
    let mut local_r: u32 = 0;
    let mut local_g: u32 = 0;
    let mut local_b: u32 = 0;
    let mut local_n: u32 = 0;
    let mut y = y0;
    while y < y1 {
        let row = &rgb[y * stride..(y + 1) * stride];
        let mut x = x0;
        while x < x1 {
            local_r += row[x * 3] as u32;
            local_g += row[x * 3 + 1] as u32;
            local_b += row[x * 3 + 2] as u32;
            local_n += 1;
            x += 2;
        }
        y += 2;
    }
    if local_n == 0 {
        return None;
    }
    let mean_r = (local_r / local_n) as i32;
    let mean_g = (local_g / local_n) as i32;
    let mean_b = (local_b / local_n) as i32;

    let mut xs: Vec<u32> = Vec::new();
    let mut ys: Vec<u32> = Vec::new();
    let mut model_pixels: u32 = 0;
    y = y0;
    while y < y1 {
        let row = &rgb[y * stride..(y + 1) * stride];
        let mut x = x0;
        while x < x1 {
            let r = row[x * 3] as i32;
            let g = row[x * 3 + 1] as i32;
            let b = row[x * 3 + 2] as i32;
            let d = (r - mean_r).abs() + (g - mean_g).abs() + (b - mean_b).abs();
            if d > 30 {
                model_pixels += 1;
                xs.push(x as u32);
                ys.push(y as u32);
                let mx = r.max(g).max(b) as f32;
                let mn = r.min(g).min(b) as f32;
                let sat = if mx == 0.0 { 0.0 } else { (mx - mn) / mx };
                sat_sum += sat;
                n += 1;
            }
            x += 2;
        }
        y += 2;
    }
    let mean_sat = if n == 0 { 0.0 } else { sat_sum / n as f32 };

    let sw = if xs.is_empty() {
        0
    } else {
        xs.iter().copied().max().unwrap() - xs.iter().copied().min().unwrap()
    };
    let sh = if ys.is_empty() {
        0
    } else {
        ys.iter().copied().max().unwrap() - ys.iter().copied().min().unwrap()
    };

    // "Blank / no materials": we rendered the disc and the green ground but
    // nothing distinct from the local mean. Tuned so that tiny but visible
    // props (a single flower, a fallen stick, a brown bear against a green
    // field) still count as rendered — those warm tones look close to the
    // ground in HSV, so we only flag a model as missing when the crop is
    // essentially uniform AND shows no model-colored pixels at all.
    let crop_pixels = ((x1 - x0) as u32) * ((y1 - y0) as u32);
    let model_ratio = model_pixels as f32 / crop_pixels.max(1) as f32;
    let is_blank = model_pixels < 80 || (model_ratio < 0.0015 && mean_sat < 0.025);
    let low_saturation = mean_sat < 0.015;
    // "Lying down": model silhouette is much wider than tall. Require both a
    // minimum width (so we don't fire on tiny / round sprites) and a real
    // aspect ratio.
    let flat_or_lying = sw >= 80 && sh >= 40 && sw as f32 / sh as f32 > 2.4;

    Some(PngInspection {
        is_blank,
        low_saturation,
        flat_or_lying,
        silhouette_w: sw,
        silhouette_h: sh,
        top_color_ratio: 1.0 - model_ratio,
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
    fs::write(
        &path,
        serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?,
    )
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
    md.push_str(&format!(
        "- rendered: {}/{}\n",
        summary.rendered, summary.total
    ));
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
    md.push_str("- copied per-model GLB metadata from manifest.json (triangles, vertices, nodes, materials, dimensions)\n");
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
        md.push_str(
            "- all models pass; pick next visual improvement (e.g. add variety, props, glow)\n",
        );
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
            if path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("glb"))
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

fn parse_model_filters(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(normalize_model_filter)
        .filter(|filter| !filter.is_empty())
        .collect()
}

fn normalize_model_filter(raw: &str) -> String {
    let normalized = raw
        .trim()
        .trim_matches('"')
        .replace('\\', "/")
        .to_ascii_lowercase();
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    let normalized = normalized.strip_prefix("assets/").unwrap_or(normalized);
    normalized
        .strip_suffix(".glb")
        .unwrap_or(normalized)
        .to_string()
}

fn model_path_matches(asset_path: &str, filter: &str) -> bool {
    let needle = normalize_model_filter(filter);
    if needle.is_empty() {
        return false;
    }
    let normalized_path = normalize_model_filter(asset_path);
    let stem = normalized_path
        .rsplit('/')
        .next()
        .unwrap_or(&normalized_path);
    normalized_path == needle || stem == needle || normalized_path.ends_with(&format!("/{needle}"))
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
    // Keep the inherited PATH so things like `sccache` keep working, and
    // prepend our target/debug helpers so the spawned `lk2-client.exe` finds
    // its dynamic-link deps.
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    let inherited_str = inherited.to_string_lossy().to_string();
    let mut prepend: Vec<String> = [root.join("target/debug/deps"), root.join("target/debug")]
        .into_iter()
        .filter(|p| p.exists())
        .map(|p| p.display().to_string())
        .collect();
    if !sysroot.is_empty() {
        let sysroot_bin = PathBuf::from(&sysroot).join("bin");
        if sysroot_bin.exists() {
            prepend.push(sysroot_bin.display().to_string());
        }
    }
    let path_str = if inherited_str.is_empty() {
        prepend.join(sep)
    } else {
        format!("{}{sep}{inherited_str}", prepend.join(sep))
    };
    Ok(vec![
        ("BEVY_DISABLE_ACCESSIBILITY".to_string(), "1".to_string()),
        ("BEVY_ASSET_ROOT".to_string(), root.display().to_string()),
        ("RUST_LOG".to_string(), "warn,lk2_client=info".to_string()),
        ("CARGO_MANIFEST_DIR".to_string(), root.display().to_string()),
        ("PATH".to_string(), path_str),
        ("WGPU_BACKEND".to_string(), "vulkan".to_string()),
    ])
}

fn cargo_build(root: &Path, envs: &[(String, String)]) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args([
        "build",
        "-p",
        "lk2-client",
        "--features",
        &DEV_DYNAMIC_FEATURES.join(","),
    ])
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

// (no dead-code anchor)

#[cfg(test)]
mod tests {
    use super::{model_path_matches, parse_model_filters};

    #[test]
    fn parses_comma_separated_model_filters() {
        assert_eq!(
            parse_model_filters(" fish, deer.glb, ,animals/fish.glb "),
            vec!["fish", "deer", "animals/fish"]
        );
    }

    #[test]
    fn matches_stems_and_assets_relative_paths() {
        assert!(model_path_matches("animals/fish.glb", "fish"));
        assert!(model_path_matches(
            "animals/fish.glb",
            "assets/animals/fish.glb"
        ));
        assert!(model_path_matches("procedural/pretty/deer.glb", "deer.glb"));
        assert!(!model_path_matches("animals/fish.glb", "fox"));
    }
}
