from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from .assertions import assertion_result, get_path, load_custom_assertions
from .config import DEFAULT_CONFIG, HarnessConfig
from .image_metrics import analyze_png
from .state_metrics import analyze_sim


@dataclass
class IterEvaluation:
    iter_dir: Path
    png: dict[str, Any]
    sim: dict[str, Any]
    assertions: list[dict[str, Any]]
    health: dict[str, Any]
    summary: str


def built_in_assertions(
    png: dict[str, Any],
    sim: dict[str, Any],
    state: dict[str, Any] | None,
    config: HarnessConfig = DEFAULT_CONFIG,
) -> list[dict[str, Any]]:
    assertions: list[dict[str, Any]] = []
    assertions.append(
        assertion_result(
            "png.readable",
            png.get("verdict"),
            "==",
            "OK",
            "fail",
            "primary screenshot must be readable and non-uniform",
            "png.verdict",
        )
    )
    assertions.append(
        assertion_result(
            "png.file_size",
            png.get("file_kb", 0.0),
            ">=",
            config.png_min_size_kb,
            "fail",
            "primary screenshot is too small to trust",
            "png.file_kb",
        )
    )
    assertions.append(
        assertion_result(
            "png.width",
            png.get("w", 0),
            ">=",
            config.png_min_w,
            "fail",
            "primary screenshot width is below harness minimum",
            "png.w",
        )
    )
    assertions.append(
        assertion_result(
            "png.height",
            png.get("h", 0),
            ">=",
            config.png_min_h,
            "fail",
            "primary screenshot height is below harness minimum",
            "png.h",
        )
    )
    assertions.append(
        assertion_result(
            "png.color_dominance",
            png.get("top_pct", 100.0),
            "<",
            config.top_color_bucket_warn_pct,
            "partial",
            "one color bucket dominates the screenshot; visual signal may be weak",
            "png.top_pct",
        )
    )
    assertions.append(
        assertion_result(
            "sim.state_exists",
            state is not None,
            "==",
            True,
            "fail",
            "final_state.json must exist and parse",
            "final_state.json",
        )
    )
    assertions.append(
        assertion_result(
            "sim.started",
            sim.get("tick"),
            ">=",
            config.sim_tick_early,
            "fail",
            "simulation did not run long enough to prove the app is alive",
            "tick",
        )
    )
    assertions.append(
        assertion_result(
            "sim.complete",
            sim.get("tick"),
            ">=",
            config.sim_tick_complete,
            "partial",
            "simulation stopped before the closed-loop completion tick",
            "tick",
        )
    )
    assertions.append(
        assertion_result(
            "observer.anomalies",
            sim.get("anomalies"),
            "==",
            0,
            "fail",
            "tick observer reported anomalies",
            "observer.anomalies",
        )
    )
    assertions.append(
        assertion_result(
            "observer.invariant_violations",
            sim.get("invariant_violations"),
            "==",
            0,
            "fail",
            "simulation invariant violations were reported",
            "observer.invariant_violations",
        )
    )

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
        assertions.append(
            assertion_result(
                "player.in_world",
                player_in_world,
                "==",
                True,
                "fail",
                "player block position must stay inside world bounds",
                "player.block_pos",
            )
        )
        assertions.append(
            assertion_result(
                "gameplay.activity",
                activity,
                ">",
                0,
                "partial",
                "auto-demo did not gather, kill, or found a nation",
                "player activity counters",
            )
        )
        assertions.append(
            assertion_result(
                "gameplay.nation_progress",
                get_path(state, "nations.total_nations"),
                ">=",
                1,
                "partial",
                "auto-demo did not create or observe a nation",
                "nations.total_nations",
            )
        )

    return assertions


def combine(
    png: dict[str, Any],
    sim: dict[str, Any],
    assertions: list[dict[str, Any]],
    config: HarnessConfig = DEFAULT_CONFIG,
) -> dict[str, Any]:
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
        reasons.append(f"sim early (tick={sim.get('tick')}, need >={config.sim_tick_early})")
        score = min(score, 5.0)
    elif sim["verdict"] == "STUCK":
        reasons.append(f"sim stuck at tick={sim.get('stuck_at')} (same as prev)")
        score = min(score, 2.0)
    elif sim["verdict"] == "RUNNING":
        reasons.append(
            f"sim incomplete (tick={sim.get('tick')}, need >={config.sim_tick_complete})"
        )
        score = min(score, 6.0)

    failed = [a for a in assertions if not a["ok"]]
    hard_failed = [a for a in failed if a.get("severity") == "fail"]
    partial_failed = [a for a in failed if a.get("severity") != "fail"]
    for a in hard_failed[:5]:
        reasons.append(
            f"{a['id']}: {a['message']} ({a.get('actual')} {a.get('op')} {a.get('expected')})"
        )
    for a in partial_failed[:5]:
        reasons.append(
            f"{a['id']}: {a['message']} ({a.get('actual')} {a.get('op')} {a.get('expected')})"
        )
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


def summarize(name: str, png: dict[str, Any], sim: dict[str, Any], combo: dict[str, Any]) -> str:
    parts = [f"{name}"]
    parts.append(f"{combo['verdict']:<7}")
    parts.append(f"png={png.get('file_kb', '?')}KB")
    png_verdict = png.get("verdict", "?")
    if png_verdict != "OK":
        png_verdict += f"/{png.get('sub', '?')}"
    parts.append(f"pv={png_verdict}")
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


def evaluate_iter(
    iter_dir: Path,
    prev_dir: Path | None = None,
    config: HarnessConfig = DEFAULT_CONFIG,
) -> IterEvaluation:
    pngs = sorted(iter_dir.glob("iter_*.png"))
    png_path = pngs[0] if pngs else None
    state_path = iter_dir / "final_state.json"
    prev_state: dict[str, Any] | None = None
    if prev_dir is not None:
        prev_path = prev_dir / "final_state.json"
        if prev_path.exists():
            try:
                prev_state = json.loads(prev_path.read_text(encoding="utf-8"))
            except Exception:
                prev_state = None

    png = analyze_png(png_path, config) if png_path else {"file_kb": 0.0, "verdict": "NA", "sub": None}
    sim, state = analyze_sim(state_path, prev_state, config)
    assertions = built_in_assertions(png, sim, state, config)
    assertions.extend(load_custom_assertions(iter_dir, state, png, sim))
    combo = combine(png, sim, assertions, config)

    health = {
        "iter": iter_dir.name,
        "png": png,
        "sim": sim,
        "assertions": {
            "total": len(assertions),
            "failed": len([a for a in assertions if not a["ok"]]),
            "hard_failed": len([a for a in assertions if not a["ok"] and a.get("severity") == "fail"]),
            "partial_failed": len(
                [a for a in assertions if not a["ok"] and a.get("severity") != "fail"]
            ),
        },
        "score": combo["score"],
        "verdict": combo["verdict"],
        "reasons": combo["reasons"],
    }
    return IterEvaluation(
        iter_dir=iter_dir,
        png=png,
        sim=sim,
        assertions=assertions,
        health=health,
        summary=summarize(iter_dir.name, png, sim, combo),
    )


def write_reports(result: IterEvaluation) -> None:
    assertions_path = result.iter_dir / "assertions.json"
    assertions_path.write_text(
        json.dumps(
            {"iter": result.iter_dir.name, "assertions": result.assertions},
            indent=2,
            ensure_ascii=False,
        )
        + "\n",
        encoding="utf-8",
    )
    health_path = result.iter_dir / "health.json"
    health_path.write_text(
        json.dumps(result.health, separators=(",", ":"), ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
