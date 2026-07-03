from __future__ import annotations

import glob
import json
import subprocess
import sys
from pathlib import Path
from typing import Any

from .artifacts import iter_number, latest_iter, newest_after, previous_iter
from .health import evaluate_iter, write_reports
from .manifest import git_sha, utc_now_iso, write_manifest


def run_eval(iter_dir: Path, prev_dir: Path | None = None) -> tuple[int, str]:
    result = evaluate_iter(iter_dir, prev_dir)
    write_reports(result)
    verdict = result.health["verdict"]
    code = 1 if verdict == "FAIL" else 0
    return code, result.summary


def run_loop(
    root: Path,
    seconds: int = 60,
    max_extra_wait: int = 60,
    offline: bool = True,
    skip_build: bool = False,
    dynamic: bool = True,
    first_person: bool = False,
) -> tuple[int, dict[str, Any]]:
    before = latest_iter(root)
    before_number = iter_number(before) if before else 0
    script = root / "scripts" / "loop" / "loop.ps1"
    args = [
        "powershell",
        "-NoProfile",
        "-File",
        str(script),
        "-Seconds",
        str(seconds),
        "-MaxExtraWait",
        str(max_extra_wait),
    ]
    if offline:
        args.append("-Offline")
    if skip_build:
        args.append("-SkipBuild")
    if not dynamic:
        args.append("-Dynamic:$false")
    if first_person:
        args.append("-FirstPerson")

    started_at = utc_now_iso()
    proc = subprocess.run(args, cwd=root, text=True)
    ended_at = utc_now_iso()
    iter_dir = newest_after(root, before_number)
    prev_dir = previous_iter(root, iter_dir) if iter_dir else None

    health_summary = None
    health_code = 2
    if iter_dir is not None:
        health_code, health_summary = run_eval(iter_dir, prev_dir)
        manifest = {
            "schema": "lk2.harness.run.v1",
            "started_at": started_at,
            "ended_at": ended_at,
            "git_sha": git_sha(root),
            "command": args,
            "process_exit_code": proc.returncode,
            "health_exit_code": health_code,
            "health_summary": health_summary,
            "mode": "offline" if offline else "online",
            "seconds": seconds,
            "max_extra_wait": max_extra_wait,
            "skip_build": skip_build,
            "dynamic": dynamic,
            "first_person": first_person,
            "iter": iter_dir.name,
            "previous_iter": prev_dir.name if prev_dir else None,
        }
        write_manifest(iter_dir, manifest)
    else:
        manifest = {
            "schema": "lk2.harness.run.v1",
            "started_at": started_at,
            "ended_at": ended_at,
            "git_sha": git_sha(root),
            "command": args,
            "process_exit_code": proc.returncode,
            "health_exit_code": 2,
            "health_summary": "no iter directory produced",
            "failure": "no_new_iter",
            "latest_before": before.name if before else None,
            "mode": "offline" if offline else "online",
            "seconds": seconds,
            "max_extra_wait": max_extra_wait,
            "skip_build": skip_build,
            "dynamic": dynamic,
            "first_person": first_person,
            "iter": None,
            "previous_iter": None,
        }

    out_dir = root / "run-logs"
    out_dir.mkdir(exist_ok=True)
    (out_dir / "harness_last_run.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )

    final_code = proc.returncode
    if final_code == 0 and health_code == 1:
        final_code = 1
    elif final_code == 0 and health_code == 2:
        final_code = 2
    return final_code, manifest


def run_suite(root: Path, pattern: str, fail_on_partial: bool = False) -> tuple[int, dict[str, Any]]:
    paths = [Path(p) for p in glob.glob(str(root / pattern))]
    if not paths:
        return 2, {"schema": "lk2.harness.suite.v1", "error": f"no matches: {pattern}"}

    results = []
    worst = 0
    for path in sorted(paths):
        iter_dir = path if path.is_dir() else path.parent
        prev_dir = previous_iter(root, iter_dir)
        result = evaluate_iter(iter_dir, prev_dir)
        write_reports(result)
        verdict = result.health["verdict"]
        if verdict == "FAIL":
            worst = max(worst, 1)
        elif verdict == "PARTIAL" and fail_on_partial:
            worst = max(worst, 1)
        results.append(
            {
                "iter": iter_dir.name,
                "path": str(iter_dir),
                "verdict": verdict,
                "score": result.health["score"],
                "summary": result.summary,
            }
        )

    report = {
        "schema": "lk2.harness.suite.v1",
        "generated_at": utc_now_iso(),
        "git_sha": git_sha(root),
        "pattern": pattern,
        "fail_on_partial": fail_on_partial,
        "total": len(results),
        "fail": len([r for r in results if r["verdict"] == "FAIL"]),
        "partial": len([r for r in results if r["verdict"] == "PARTIAL"]),
        "pass": len([r for r in results if r["verdict"] == "PASS"]),
        "results": results,
    }
    out_dir = root / "run-logs"
    out_dir.mkdir(exist_ok=True)
    (out_dir / "harness_suite.json").write_text(
        json.dumps(report, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    return worst, report


def print_json(data: dict[str, Any]) -> None:
    print(json.dumps(data, indent=2, ensure_ascii=False))
