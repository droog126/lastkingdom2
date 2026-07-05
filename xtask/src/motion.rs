use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use serde_json::Value;

use crate::Result;

#[derive(Default)]
struct MotionStats {
    samples: u64,
    move_attempts: u64,
    blocked_moves: u64,
    y_jumps: u64,
    camera_jumps: u64,
    frame_spikes: u64,
    server_corrections: u64,
    max_player_step: f64,
    max_camera_step: f64,
    max_y_delta: f64,
    max_dt: f64,
    max_server_drift: f64,
    max_server_correction: f64,
    sum_player_step: f64,
    sum_camera_step: f64,
}

pub fn analyze(root: &Path, args: &[String]) -> Result<()> {
    let path = args
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("screenshots").join("online_motion_trace.jsonl"));
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    let file = File::open(&path).map_err(|e| format!("failed to open {}: {e}", path.display()))?;

    let mut stats = MotionStats::default();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|e| format!("failed to read {}: {e}", path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line)
            .map_err(|e| format!("bad jsonl in {}: {e}: {line}", path.display()))?;
        stats.samples += 1;

        let dt = num(&value, "dt");
        let player_step = num(&value, "player_step_len");
        let camera_step = num(&value, "camera_step_len");
        let server_drift = num(&value, "server_drift");
        let server_correction = num(&value, "server_correction");
        let player_delta_y = value
            .get("player_delta")
            .and_then(|v| v.as_array())
            .and_then(|a| a.get(1))
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
            .abs();
        let attempted = value.get("local_move_attempted").and_then(Value::as_bool).unwrap_or(false);
        let moved = value.get("local_move_moved").and_then(Value::as_bool).unwrap_or(false);

        stats.max_dt = stats.max_dt.max(dt);
        stats.max_player_step = stats.max_player_step.max(player_step);
        stats.max_camera_step = stats.max_camera_step.max(camera_step);
        stats.max_y_delta = stats.max_y_delta.max(player_delta_y);
        stats.max_server_drift = stats.max_server_drift.max(server_drift);
        stats.max_server_correction = stats.max_server_correction.max(server_correction);
        stats.sum_player_step += player_step;
        stats.sum_camera_step += camera_step;
        if attempted {
            stats.move_attempts += 1;
            if !moved {
                stats.blocked_moves += 1;
            }
        }
        if player_delta_y > 0.05 {
            stats.y_jumps += 1;
        }
        if camera_step > 0.35 {
            stats.camera_jumps += 1;
        }
        if dt > 0.035 {
            stats.frame_spikes += 1;
        }
        if server_correction > 0.0 {
            stats.server_corrections += 1;
        }
    }

    if stats.samples == 0 {
        return Err(format!("{} contained no samples", path.display()));
    }

    let avg_player_step = stats.sum_player_step / stats.samples as f64;
    let avg_camera_step = stats.sum_camera_step / stats.samples as f64;
    println!("motion trace: {}", path.display());
    println!("samples: {}", stats.samples);
    println!(
        "player step avg/max: {:.4} / {:.4}",
        avg_player_step, stats.max_player_step
    );
    println!(
        "camera step avg/max: {:.4} / {:.4}",
        avg_camera_step, stats.max_camera_step
    );
    println!(
        "y jumps: {} (max |dy| {:.4})",
        stats.y_jumps, stats.max_y_delta
    );
    println!(
        "server corrections: {} (max correction {:.4}, max drift {:.4})",
        stats.server_corrections, stats.max_server_correction, stats.max_server_drift
    );
    println!(
        "blocked local moves: {}/{}",
        stats.blocked_moves, stats.move_attempts
    );
    println!(
        "frame spikes: {} (max dt {:.4}s)",
        stats.frame_spikes, stats.max_dt
    );
    println!("likely cause: {}", likely_cause(&stats));
    Ok(())
}

fn num(value: &Value, key: &str) -> f64 {
    value.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn likely_cause(stats: &MotionStats) -> &'static str {
    if stats.server_corrections > 0 && stats.max_server_correction > 0.75 {
        "server reconciliation snap; compare client prediction with authoritative movement"
    } else if stats.y_jumps > stats.samples / 20 && stats.max_y_delta > 0.10 {
        "ground-height or block-position snapping; inspect player_stand_position_at inputs"
    } else if stats.blocked_moves > stats.move_attempts / 10 && stats.move_attempts > 30 {
        "collision/step rejection during movement; inspect volume clearance and step threshold"
    } else if stats.frame_spikes > stats.samples / 20 && stats.max_dt > 0.05 {
        "frame-time spikes; inspect mesh rebuilds, logging, or asset work during movement"
    } else if stats.camera_jumps > stats.samples / 20 {
        "camera-only jitter; inspect camera bob/follow source rather than authoritative movement"
    } else {
        "no dominant jitter signature in this trace; collect a longer trace while reproducing"
    }
}
