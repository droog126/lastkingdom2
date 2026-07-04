use std::{
    collections::HashMap,
    ffi::OsStr,
    fs::{self, File},
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use crate::Result;

const LUMA_VAR_HEALTHY: f64 = 100.0;
const LUMA_MEAN_WHITE: f64 = 200.0;
const LUMA_MEAN_BLACK: f64 = 20.0;
const PNG_MIN_SIZE_KB: f64 = 30.0;
const SIM_TICK_EARLY: i64 = 30;
const SIM_TICK_COMPLETE: i64 = 500;
const SAMPLE_SIZE: usize = 64;
const PNG_MIN_W: u32 = 640;
const PNG_MIN_H: u32 = 360;
const TOP_COLOR_BUCKET_WARN_PCT: f64 = 92.0;

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

pub fn evaluate_iter(iter_dir: &Path, prev_dir: Option<&Path>) -> Result<Evaluation> {
    let png_path = primary_png(iter_dir);
    let png = if let Some(path) = png_path {
        analyze_png(&path)
    } else {
        json!({"file_kb":0.0,"verdict":"NA","sub":null})
    };

    let prev_state = prev_dir
        .and_then(|dir| fs::read_to_string(dir.join("final_state.json")).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let (sim, state) = analyze_sim(&iter_dir.join("final_state.json"), prev_state.as_ref());
    let assertions = built_in_assertions(&png, &sim, state.as_ref());
    let combo = combine(&png, &sim, &assertions);
    let name = iter_dir.file_name().and_then(OsStr::to_str).unwrap_or("iter");
    let summary = summarize(name, &png, &sim, &combo);

    let failed = assertions.iter().filter(|a| !a.ok).count();
    let hard_failed = assertions.iter().filter(|a| !a.ok && a.severity == "fail").count();
    let partial_failed = assertions.iter().filter(|a| !a.ok && a.severity != "fail").count();
    let health = json!({
        "iter": name,
        "png": png,
        "sim": sim,
        "assertions": {
            "total": assertions.len(),
            "failed": failed,
            "hard_failed": hard_failed,
            "partial_failed": partial_failed,
        },
        "score": combo["score"],
        "verdict": combo["verdict"],
        "reasons": combo["reasons"],
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
    Ok(Evaluation { verdict: combo["verdict"].as_str().unwrap_or("FAIL").to_string(), summary })
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
    let file_kb =
        fs::metadata(path).map(|m| (m.len() as f64 / 1024.0 * 10.0).round() / 10.0).unwrap_or(0.0);
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
    let nations = path_i64(&state, "nations.total_nations").unwrap_or(0);
    let anomalies = path_i64(&state, "observer.anomalies").unwrap_or(0);
    let invariant_violations = path_i64(&state, "observer.invariant_violations").unwrap_or(0);
    out["tick"] = json!(tick);
    out["nations"] = json!(nations);
    out["anomalies"] = json!(anomalies);
    out["invariant_violations"] = json!(invariant_violations);
    out["verdict"] = if tick >= SIM_TICK_COMPLETE {
        json!("COMPLETE")
    } else if tick < SIM_TICK_EARLY {
        json!("EARLY")
    } else {
        json!("RUNNING")
    };
    if let Some(prev) = prev_state {
        if tick > 0 && path_i64(prev, "tick").unwrap_or(0) == tick {
            out["verdict"] = json!("STUCK");
            out["stuck_at"] = json!(tick);
        }
    }
    (out, Some(state))
}

fn built_in_assertions(png: &Value, sim: &Value, state: Option<&Value>) -> Vec<Assertion> {
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
            &sim["tick"],
            ">=",
            json!(SIM_TICK_EARLY),
            "fail",
            "simulation did not run long enough to prove the app is alive",
            Some("tick"),
        ),
        assertion(
            "sim.complete",
            &sim["tick"],
            ">=",
            json!(SIM_TICK_COMPLETE),
            "partial",
            "simulation stopped before the closed-loop completion tick",
            Some("tick"),
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
    ];
    if let Some(state) = state {
        let player_block = path_value(state, "player.block_pos");
        let world_size = path_i64(state, "world.size");
        let player_in_world = player_block
            .and_then(Value::as_array)
            .zip(world_size)
            .map(|(pos, size)| {
                pos.len() == 3
                    && pos.iter().all(|v| v.as_i64().map(|n| n >= 0 && n < size).unwrap_or(false))
            })
            .unwrap_or(false);
        let activity = path_i64(state, "player.blocks_gathered").unwrap_or(0)
            + path_i64(state, "player.monsters_killed").unwrap_or(0)
            + path_i64(state, "player.nations_founded").unwrap_or(0);
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
    a
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
        ">" => num(actual).zip(num(expected)).map(|(a, e)| a > e).unwrap_or(false),
        ">=" => num(actual).zip(num(expected)).map(|(a, e)| a >= e).unwrap_or(false),
        "<" => num(actual).zip(num(expected)).map(|(a, e)| a < e).unwrap_or(false),
        "<=" => num(actual).zip(num(expected)).map(|(a, e)| a <= e).unwrap_or(false),
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
    let hard = failed.iter().filter(|a| a.severity == "fail").collect::<Vec<_>>();
    let partial = failed.iter().filter(|a| a.severity != "fail").collect::<Vec<_>>();
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
            out.push_str(&reasons.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("; "));
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

fn num(value: &Value) -> Option<f64> {
    value.as_f64()
}

fn round1(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}
