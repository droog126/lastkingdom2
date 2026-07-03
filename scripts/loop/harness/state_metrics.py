from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .config import DEFAULT_CONFIG, HarnessConfig


def read_state(state_path: Path) -> tuple[dict[str, Any] | None, str | None]:
    if not state_path.exists():
        return None, "missing"
    try:
        return json.loads(state_path.read_text(encoding="utf-8")), None
    except Exception as e:
        return None, str(e)[:120]


def analyze_sim(
    state_path: Path,
    prev_state: dict[str, Any] | None,
    config: HarnessConfig = DEFAULT_CONFIG,
) -> tuple[dict[str, Any], dict[str, Any] | None]:
    out: dict[str, Any] = {
        "verdict": "DEAD",
        "tick": None,
        "nations": None,
        "anomalies": None,
        "invariant_violations": None,
    }
    state, err = read_state(state_path)
    if state is None:
        if err and err != "missing":
            out["err"] = err
        return out, None
    try:
        tick = state.get("tick", 0)
        nations = (state.get("nations") or {}).get("total_nations", 0)
        observer = state.get("observer") or {}
        anomalies = observer.get("anomalies", 0)
        invariant_violations = observer.get("invariant_violations", 0)
    except Exception as e:
        out["err"] = str(e)[:120]
        return out, state

    out["tick"] = tick
    out["nations"] = nations
    out["anomalies"] = anomalies
    out["invariant_violations"] = invariant_violations
    if tick >= config.sim_tick_complete:
        out["verdict"] = "COMPLETE"
    elif tick < config.sim_tick_early:
        out["verdict"] = "EARLY"
    else:
        out["verdict"] = "RUNNING"

    if prev_state is not None and tick > 0:
        prev_tick = prev_state.get("tick", 0)
        if tick == prev_tick and prev_tick > 0:
            out["verdict"] = "STUCK"
            out["stuck_at"] = tick
    return out, state
