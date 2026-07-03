from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .artifacts import project_root_from
from .health import evaluate_iter, write_reports
from .orchestrator import print_json, run_eval, run_loop, run_suite


def legacy_eval(argv: list[str]) -> int:
    if len(argv) < 1:
        print("usage: health_check.py ITER_DIR [PREV_ITER_DIR]", file=sys.stderr)
        return 2
    iter_dir = Path(argv[0]).resolve()
    if not iter_dir.is_dir():
        print(f"FATAL: not a directory: {iter_dir}", file=sys.stderr)
        return 2

    prev_dir = Path(argv[1]) if len(argv) >= 2 else None
    if prev_dir is not None and not prev_dir.exists():
        prev_dir = None

    result = evaluate_iter(iter_dir, prev_dir)
    write_reports(result)
    print(result.summary)
    return 0


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="python -m scripts.loop.harness",
        description="Closed-loop harness runner and evaluator.",
    )
    sub = parser.add_subparsers(dest="command")

    eval_p = sub.add_parser("eval", help="evaluate an existing iter directory")
    eval_p.add_argument("iter_dir", type=Path)
    eval_p.add_argument("prev_dir", type=Path, nargs="?")
    eval_p.add_argument("--json", action="store_true", help="print health.json payload")

    run_p = sub.add_parser("run", help="run the existing loop.ps1 and write manifest.json")
    run_p.add_argument("--seconds", type=int, default=60)
    run_p.add_argument("--max-extra-wait", type=int, default=60)
    run_p.add_argument("--online", action="store_true", help="run online/server mode")
    run_p.add_argument("--skip-build", action="store_true")
    run_p.add_argument("--no-dynamic", action="store_true")
    run_p.add_argument("--first-person", action="store_true")
    run_p.add_argument("--json", action="store_true", help="print manifest JSON")

    suite_p = sub.add_parser("suite", help="batch-evaluate iter directories")
    suite_p.add_argument("pattern", nargs="?", default="screenshots/iter_*")
    suite_p.add_argument("--fail-on-partial", action="store_true")
    suite_p.add_argument("--json", action="store_true", help="print suite report JSON")

    return parser


def main(argv: list[str]) -> int:
    # Backward-compatible health_check.py ITER_DIR [PREV_ITER_DIR] mode.
    if argv and argv[0] not in {"eval", "run", "suite", "-h", "--help"}:
        return legacy_eval(argv)

    parser = build_parser()
    args = parser.parse_args(argv)
    root = project_root_from(Path.cwd())

    if args.command == "eval":
        code, summary = run_eval(args.iter_dir.resolve(), args.prev_dir.resolve() if args.prev_dir else None)
        if args.json:
            health = (args.iter_dir / "health.json").read_text(encoding="utf-8")
            print(health.rstrip())
        else:
            print(summary)
        return code

    if args.command == "run":
        code, manifest = run_loop(
            root,
            seconds=args.seconds,
            max_extra_wait=args.max_extra_wait,
            offline=not args.online,
            skip_build=args.skip_build,
            dynamic=not args.no_dynamic,
            first_person=args.first_person,
        )
        if args.json:
            print_json(manifest)
        else:
            print(manifest.get("health_summary", "no health summary"))
            if manifest.get("iter"):
                print(f"manifest={root / 'screenshots' / manifest['iter'] / 'manifest.json'}")
        return code

    if args.command == "suite":
        code, report = run_suite(root, args.pattern, fail_on_partial=args.fail_on_partial)
        if args.json:
            print_json(report)
        elif "error" in report:
            print(report["error"], file=sys.stderr)
        else:
            print(
                f"suite total={report['total']} pass={report['pass']} "
                f"partial={report['partial']} fail={report['fail']}"
            )
            for row in report["results"]:
                print(row["summary"])
            print(f"report={root / 'run-logs' / 'harness_suite.json'}")
        return code

    parser.print_help()
    return 2
