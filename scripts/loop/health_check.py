#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

try:
    from PIL import Image
except ImportError:
    print("FATAL: Pillow not installed. Run: pip install Pillow", file=sys.stderr)
    sys.exit(2)

LUMA_VAR_HEALTHY = 100.0
LUMA_MEAN_WHITE = 200.0
LUMA_MEAN_BLACK = 20.0
PNG_MIN_SIZE_KB = 30.0
SIM_TICK_EARLY = 30
SIM_TICK_COMPLETE = 500
SAMPLE_SIZE = 64
PNG_MIN_W = 640
PNG_MIN_H = 360
TOP_COLOR_BUCKET_WARN_PCT = 92.0

OPS = {
    "==": lambda actual, expected: actual == expected,
    "!=": lambda actual, expected: actual != expected,
    ">": lambda actual, expected: actual is not None and actual > expected,
    ">=": lambda actual, expected: actual is not None and actual >= expected,
    "<": lambda actual, expected: actual is not None and actual < expected,
    "<=": lambda actual, expected: actual is not None and actual <= expected,
}

def get_path(value: Any, path: str) -> Any:
    cur = value
    for part in path.split("."):
        if isinstance(cur, dict):
            cur = cur.get(part)
        elif isinstance(cur, list):
            try:
                cur = cur[int(part)]
            except (ValueError, IndexError):
                return None
        else:
            return None
    return cur

def assertion_result(
    assertion_id: str,
    actual: Any,
    op: str,
    expected: Any,
    severity: str,
    message: str,
    path: str | None = None,
) -> dict[str, Any]:
    ok = OPS[op](actual, expected)
    out: dict[str, Any] = {
        "id": assertion_id,
        "ok": bool(ok),
        "severity": severity,
        "actual": actual,
        "op": op,
        "expected": expected,
        "message": message,
    }
    if path is not None:
        out["path"] = path
    return out

def analyze_png(png_path: Path) -> dict[str, Any]:
    
    out: dict[str, Any] = {
        "file_kb": round(png_path.stat().st_size / 1024, 1),
        "verdict": "NA",
        "sub": None,
    }
    try:
        img = Image.open(png_path).convert("RGB")
        out["w"], out["h"] = img.size
        small = img.resize((SAMPLE_SIZE, SAMPLE_SIZE), Image.LANCZOS)

        try:
            import numpy as np
            arr = np.asarray(small, dtype=np.float32)
            r, g, b = arr[..., 0], arr[..., 1], arr[..., 2]
            luma = 0.299 * r + 0.587 * g + 0.114 * b
            luma_mean = float(luma.mean())
            luma_var = float(luma.var())

            buckets = (arr.astype(np.uint8) >> 5).reshape(-1, 3)

            keys = (buckets[:, 0].astype(np.int32) << 10
                    | buckets[:, 1].astype(np.int32) << 5
                    | buckets[:, 2].astype(np.int32))
            unique, counts = np.unique(keys, return_counts=True)
            top_pct = float(counts.max() / keys.size * 100)
        except ImportError:

            data = small.tobytes()
            n = len(data) // 3
            luma_total = 0.0
            luma_sq_total = 0.0
            bucket_counts: dict[tuple[int, int, int], int] = {}
            for i in range(n):
                r = data[i * 3]
                g = data[i * 3 + 1]
                b = data[i * 3 + 2]
                L = 0.299 * r + 0.587 * g + 0.114 * b
                luma_total += L
                luma_sq_total += L * L
                key = (r >> 5, g >> 5, b >> 5)
                bucket_counts[key] = bucket_counts.get(key, 0) + 1
            luma_mean = luma_total / n
            luma_var = luma_sq_total / n - luma_mean ** 2
            top_pct = max(bucket_counts.values()) / n * 100

        out["luma_mean"] = round(luma_mean, 1)
        out["luma_var"] = round(luma_var, 1)
        out["top_pct"] = round(top_pct, 1)

        if luma_var < LUMA_VAR_HEALTHY:
            out["verdict"] = "UNIFORM"
            if luma_mean > LUMA_MEAN_WHITE:
                out["sub"] = "WHITE"
            elif luma_mean < LUMA_MEAN_BLACK:
                out["sub"] = "BLACK"
            else:
                out["sub"] = "GRAY"
        else:
            out["verdict"] = "OK"
        return out
    except Exception as e:
        out["verdict"] = "ERR"
        out["err"] = str(e)[:120]
        return out

def read_state(state_path: Path) -> tuple[dict[str, Any] | None, str | None]:
    if not state_path.exists():
        return None, "missing"
    try:
        return json.loads(state_path.read_text(encoding="utf-8")), None
    except Exception as e:
        return None, str(e)[:120]

def analyze_sim(state_path: Path, prev_state: dict | None) -> tuple[dict[str, Any], dict[str, Any] | None]:
    
    out: dict[str, Any] = {"verdict": "DEAD", "tick": None, "nations": None,
                            "anomalies": None, "invariant_violations": None}
    s, err = read_state(state_path)
    if s is None:
        if err and err != "missing":
            out["err"] = err
        return out, None
    try:
        tick = s.get("tick", 0)
        nations = (s.get("nations") or {}).get("total_nations", 0)
        observer = s.get("observer") or {}
        anomalies = observer.get("anomalies", 0)
        invariant_violations = observer.get("invariant_violations", 0)
    except Exception as e:
        out["err"] = str(e)[:120]
        return out, s
    out["tick"] = tick
    out["nations"] = nations
    out["anomalies"] = anomalies
    out["invariant_violations"] = invariant_violations
    if tick >= SIM_TICK_COMPLETE:
        out["verdict"] = "COMPLETE"
    elif tick < SIM_TICK_EARLY:
        out["verdict"] = "EARLY"
    else:
        out["verdict"] = "RUNNING"

    if prev_state is not None and tick > 0:
        prev_tick = prev_state.get("tick", 0)
        if tick == prev_tick and prev_tick > 0:
            out["verdict"] = "STUCK"
            out["stuck_at"] = tick
    return out, s

def built_in_assertions(png: dict, sim: dict, state: dict[str, Any] | None) -> list[dict[str, Any]]:
    assertions: list[dict[str, Any]] = []
    assertions.append(assertion_result(
        "png.readable",
        png.get("verdict"),
        "==",
        "OK",
        "fail",
        "primary screenshot must be readable and non-uniform",
        "png.verdict",
    ))
    assertions.append(assertion_result(
        "png.file_size",
        png.get("file_kb", 0.0),
        ">=",
        PNG_MIN_SIZE_KB,
        "fail",
        "primary screenshot is too small to trust",
        "png.file_kb",
    ))
    assertions.append(assertion_result(
        "png.width",
        png.get("w", 0),
        ">=",
        PNG_MIN_W,
        "fail",
        "primary screenshot width is below harness minimum",
        "png.w",
    ))
    assertions.append(assertion_result(
        "png.height",
        png.get("h", 0),
        ">=",
        PNG_MIN_H,
        "fail",
        "primary screenshot height is below harness minimum",
        "png.h",
    ))
    assertions.append(assertion_result(
        "png.color_dominance",
        png.get("top_pct", 100.0),
        "<",
        TOP_COLOR_BUCKET_WARN_PCT,
        "partial",
        "one color bucket dominates the screenshot; visual signal may be weak",
        "png.top_pct",
    ))
    assertions.append(assertion_result(
        "sim.state_exists",
        state is not None,
        "==",
        True,
        "fail",
        "final_state.json must exist and parse",
        "final_state.json",
    ))
    assertions.append(assertion_result(
        "sim.started",
        sim.get("tick"),
        ">=",
        SIM_TICK_EARLY,
        "fail",
        "simulation did not run long enough to prove the app is alive",
        "tick",
    ))
    assertions.append(assertion_result(
        "sim.complete",
        sim.get("tick"),
        ">=",
        SIM_TICK_COMPLETE,
        "partial",
        "simulation stopped before the closed-loop completion tick",
        "tick",
    ))
    assertions.append(assertion_result(
        "observer.anomalies",
        sim.get("anomalies"),
        "==",
        0,
        "fail",
        "tick observer reported anomalies",
        "observer.anomalies",
    ))
    assertions.append(assertion_result(
        "observer.invariant_violations",
        sim.get("invariant_violations"),
        "==",
        0,
        "fail",
        "simulation invariant violations were reported",
        "observer.invariant_violations",
    ))

    if state is not None:
        player_block = get_path(state, "player.block_pos")
        world_size = get_path(state, "world.size")
        player_in_world = (
            isinstance(player_block, list)
            and len(player_block) == 3
            and isinstance(world_size, int)
            and all(isinstance(v, int) and 0 <= v < world_size for v in player_block)
        )
        activity = (
            (get_path(state, "player.blocks_gathered") or 0)
            + (get_path(state, "player.monsters_killed") or 0)
            + (get_path(state, "player.nations_founded") or 0)
        )
        assertions.append(assertion_result(
            "player.in_world",
            player_in_world,
            "==",
            True,
            "fail",
            "player block position must stay inside world bounds",
            "player.block_pos",
        ))
        assertions.append(assertion_result(
            "gameplay.activity",
            activity,
            ">",
            0,
            "partial",
            "auto-demo did not gather, kill, or found a nation",
            "player activity counters",
        ))
        assertions.append(assertion_result(
            "gameplay.nation_progress",
            get_path(state, "nations.total_nations"),
            ">=",
            1,
            "partial",
            "auto-demo did not create or observe a nation",
            "nations.total_nations",
        ))

    return assertions

def load_custom_assertions(iter_dir: Path, state: dict[str, Any] | None, png: dict, sim: dict) -> list[dict[str, Any]]:
    path = iter_dir / "harness_assertions.json"
    if not path.exists():
        return []
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:
        return [assertion_result(
            "custom.parse",
            f"parse error: {str(e)[:120]}",
            "==",
            "ok",
            "fail",
            "harness_assertions.json must parse",
            str(path),
        )]
    specs = raw.get("assertions", raw) if isinstance(raw, dict) else raw
    if not isinstance(specs, list):
        return [assertion_result(
            "custom.schema",
            type(specs).__name__,
            "==",
            "list",
            "fail",
            "harness_assertions.json must be a list or an object with assertions",
            str(path),
        )]

    sources = {"state": state or {}, "png": png, "sim": sim}
    results = []
    for i, spec in enumerate(specs):
        if not isinstance(spec, dict):
            results.append(assertion_result(
                f"custom.{i}.schema",
                type(spec).__name__,
                "==",
                "object",
                "fail",
                "custom assertion must be an object",
                str(path),
            ))
            continue
        source_name = spec.get("source", "state")
        source = sources.get(source_name, {})
        actual_path = spec.get("path", "")
        op = spec.get("op", "==")
        if op not in OPS:
            results.append(assertion_result(
                spec.get("id", f"custom.{i}.op"),
                op,
                "==",
                "supported op",
                "fail",
                f"unsupported assertion op: {op}",
                actual_path,
            ))
            continue
        actual = get_path(source, actual_path)
        results.append(assertion_result(
            spec.get("id", f"custom.{i}"),
            actual,
            op,
            spec.get("value"),
            spec.get("severity", "fail"),
            spec.get("message", "custom harness assertion"),
            actual_path,
        ))
    return results

def combine(png: dict, sim: dict, assertions: list[dict[str, Any]]) -> dict[str, Any]:
    
    reasons: list[str] = []
    score = 10.0

    if png["verdict"] == "UNIFORM":
        sub = png.get("sub", "GRAY")
        reasons.append(f"png uniform ({sub}, luma_var={png.get('luma_var', '?')})")
        score = 0.0
    elif png["verdict"] == "ERR":
        reasons.append(f"png read error: {png.get('err', '?')}")
        score = 0.0

    if sim["verdict"] == "DEAD":
        reasons.append("sim state missing")
        score = min(score, 0.0)
    elif sim["verdict"] == "EARLY":
        reasons.append(f"sim early (tick={sim.get('tick')}, need >={SIM_TICK_EARLY})")
        score = min(score, 5.0)
    elif sim["verdict"] == "STUCK":
        reasons.append(f"sim stuck at tick={sim.get('stuck_at')} (same as prev)")
        score = min(score, 2.0)
    elif sim["verdict"] == "RUNNING":
        reasons.append(f"sim incomplete (tick={sim.get('tick')}, need >={SIM_TICK_COMPLETE})")
        score = min(score, 6.0)

    failed = [a for a in assertions if not a["ok"]]
    hard_failed = [a for a in failed if a.get("severity") == "fail"]
    partial_failed = [a for a in failed if a.get("severity") != "fail"]
    for a in hard_failed[:5]:
        reasons.append(f"{a['id']}: {a['message']} ({a.get('actual')} {a.get('op')} {a.get('expected')})")
    for a in partial_failed[:5]:
        reasons.append(f"{a['id']}: {a['message']} ({a.get('actual')} {a.get('op')} {a.get('expected')})")
    if hard_failed:
        score = min(score, 0.0)
    elif partial_failed:
        score = min(score, 6.0)

    if score >= 9.0:
        verdict = "PASS"
    elif score >= 4.0:
        verdict = "PARTIAL"
    else:
        verdict = "FAIL"

    return {"score": score, "verdict": verdict, "reasons": reasons}

def summarize(name: str, png: dict, sim: dict, combo: dict) -> str:
    
    parts = [f"{name}"]
    parts.append(f"{combo['verdict']:<7}")
    parts.append(f"png={png.get('file_kb', '?')}KB")
    pv = png.get("verdict", "?")
    if pv != "OK":
        pv += f"/{png.get('sub', '?')}"
    parts.append(f"pv={pv}")
    parts.append(f"luma={png.get('luma_mean', '?')}/{png.get('luma_var', '?')}")
    parts.append(f"sim={sim.get('verdict', '?')}")
    parts.append(f"tick={sim.get('tick', '?')}")
    parts.append(f"nations={sim.get('nations', '?')}")
    parts.append(f"anomalies={sim.get('anomalies', '?')}")
    parts.append(f"invariants={sim.get('invariant_violations', '?')}")
    parts.append(f"score={combo['score']:.1f}")
    if combo["reasons"]:
        parts.append(f"| {'; '.join(combo['reasons'])}")
    return " ".join(parts)

def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    iter_dir = Path(argv[1]).resolve()
    if not iter_dir.is_dir():
        print(f"FATAL: not a directory: {iter_dir}", file=sys.stderr)
        return 2

    pngs = sorted(iter_dir.glob("iter_*.png"))
    png_path = pngs[0] if pngs else None
    state_path = iter_dir / "final_state.json"
    prev_state: dict | None = None
    if len(argv) >= 3:
        prev_path = Path(argv[2]) / "final_state.json"
        if prev_path.exists():
            try:
                prev_state = json.loads(prev_path.read_text(encoding="utf-8"))
            except Exception:
                prev_state = None

    png = analyze_png(png_path) if png_path else {
        "file_kb": 0.0, "verdict": "NA", "sub": None,
    }
    sim, state = analyze_sim(state_path, prev_state)
    assertions = built_in_assertions(png, sim, state)
    assertions.extend(load_custom_assertions(iter_dir, state, png, sim))
    combo = combine(png, sim, assertions)

    out = {
        "iter": iter_dir.name,
        "png": png,
        "sim": sim,
        "assertions": {
            "total": len(assertions),
            "failed": len([a for a in assertions if not a["ok"]]),
            "hard_failed": len([a for a in assertions if not a["ok"] and a.get("severity") == "fail"]),
            "partial_failed": len([a for a in assertions if not a["ok"] and a.get("severity") != "fail"]),
        },
        "score": combo["score"],
        "verdict": combo["verdict"],
        "reasons": combo["reasons"],
    }
    assertions_path = iter_dir / "assertions.json"
    assertions_path.write_text(
        json.dumps({"iter": iter_dir.name, "assertions": assertions}, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    health_path = iter_dir / "health.json"
    health_path.write_text(
        json.dumps(out, separators=(",", ":"), ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(summarize(iter_dir.name, png, sim, combo))
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))
