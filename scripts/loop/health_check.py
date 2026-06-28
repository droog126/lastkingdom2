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

def analyze_sim(state_path: Path, prev_state: dict | None) -> dict[str, Any]:
    
    out: dict[str, Any] = {"verdict": "DEAD", "tick": None, "nations": None,
                            "anomalies": None}
    if not state_path.exists():
        return out
    try:
        s = json.loads(state_path.read_text(encoding="utf-8"))
    except Exception as e:
        out["err"] = str(e)[:120]
        return out
    tick = s.get("tick", 0)
    nations = (s.get("nations") or {}).get("total_nations", 0)
    anomalies = (s.get("observer") or {}).get("anomalies", 0)
    out["tick"] = tick
    out["nations"] = nations
    out["anomalies"] = anomalies
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
    return out

def combine(png: dict, sim: dict) -> dict[str, Any]:
    
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
    sim = analyze_sim(state_path, prev_state)
    combo = combine(png, sim)

    out = {
        "iter": iter_dir.name,
        "png": png,
        "sim": sim,
        "score": combo["score"],
        "verdict": combo["verdict"],
        "reasons": combo["reasons"],
    }
    health_path = iter_dir / "health.json"
    health_path.write_text(
        json.dumps(out, separators=(",", ":"), ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(summarize(iter_dir.name, png, sim, combo))
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv))
