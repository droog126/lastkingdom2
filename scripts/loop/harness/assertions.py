from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Callable


OPS: dict[str, Callable[[Any, Any], bool]] = {
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


def load_custom_assertions(
    iter_dir: Path,
    state: dict[str, Any] | None,
    png: dict[str, Any],
    sim: dict[str, Any],
) -> list[dict[str, Any]]:
    path = iter_dir / "harness_assertions.json"
    if not path.exists():
        return []
    try:
        raw = json.loads(path.read_text(encoding="utf-8"))
    except Exception as e:
        return [
            assertion_result(
                "custom.parse",
                f"parse error: {str(e)[:120]}",
                "==",
                "ok",
                "fail",
                "harness_assertions.json must parse",
                str(path),
            )
        ]
    specs = raw.get("assertions", raw) if isinstance(raw, dict) else raw
    if not isinstance(specs, list):
        return [
            assertion_result(
                "custom.schema",
                type(specs).__name__,
                "==",
                "list",
                "fail",
                "harness_assertions.json must be a list or an object with assertions",
                str(path),
            )
        ]

    sources = {"state": state or {}, "png": png, "sim": sim}
    results = []
    for i, spec in enumerate(specs):
        if not isinstance(spec, dict):
            results.append(
                assertion_result(
                    f"custom.{i}.schema",
                    type(spec).__name__,
                    "==",
                    "object",
                    "fail",
                    "custom assertion must be an object",
                    str(path),
                )
            )
            continue
        source_name = spec.get("source", "state")
        source = sources.get(source_name, {})
        actual_path = spec.get("path", "")
        op = spec.get("op", "==")
        if op not in OPS:
            results.append(
                assertion_result(
                    spec.get("id", f"custom.{i}.op"),
                    op,
                    "==",
                    "supported op",
                    "fail",
                    f"unsupported assertion op: {op}",
                    actual_path,
                )
            )
            continue
        actual = get_path(source, actual_path)
        results.append(
            assertion_result(
                spec.get("id", f"custom.{i}"),
                actual,
                op,
                spec.get("value"),
                spec.get("severity", "fail"),
                spec.get("message", "custom harness assertion"),
                actual_path,
            )
        )
    return results
