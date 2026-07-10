from __future__ import annotations

import argparse
from pathlib import Path

from glb_utils import inspect_glb
from model_catalog import category_for
from model_style import HARD_TRIANGLE_LIMIT, PRETTY_DIR, budget_for


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify category-aware LK2 model poly budgets")
    parser.add_argument("directory", nargs="?", type=Path, default=PRETTY_DIR)
    parser.add_argument("--collection", default="pretty")
    parser.add_argument("--budget", type=int, help="override category budgets for diagnostics")
    args = parser.parse_args()

    failures: list[tuple[str, int, int]] = []
    paths = sorted(args.directory.glob("*.glb"))
    for path in paths:
        stats = inspect_glb(path)
        category = category_for(args.collection, path.stem)
        budget = args.budget or min(budget_for(category).triangles, HARD_TRIANGLE_LIMIT)
        status = "OK" if stats.triangles <= budget else "OVER"
        print(
            f"[{status:4}] {path.name:38} category={category:10} "
            f"tris={stats.triangles:5}/{budget:5} verts={stats.vertices:5}"
        )
        if stats.triangles > budget:
            failures.append((path.name, stats.triangles, budget))

    if not paths:
        print(f"FAIL: no GLBs found in {args.directory}")
        return 1
    if failures:
        print(f"\nFAIL: {len(failures)} assets exceed budget")
        for name, triangles, budget in failures:
            print(f"  {name}: {triangles} > {budget}")
        return 1
    print(f"\nPASS: all {len(paths)} GLBs satisfy category budgets")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
