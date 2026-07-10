from __future__ import annotations

import argparse
from pathlib import Path

from glb_utils import validate_glb
from model_catalog import category_for, scale_contract_for
from model_style import PRETTY_DIR


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate LK2 GLB structure and style metadata")
    parser.add_argument("directory", nargs="?", type=Path, default=PRETTY_DIR)
    parser.add_argument("--collection", default="pretty")
    args = parser.parse_args()

    paths = sorted(args.directory.glob("*.glb"))
    failures = 0
    warnings = 0
    for path in paths:
        category = category_for(args.collection, path.stem)
        result = validate_glb(path, category, scale_contract_for(args.collection, path.stem))
        if result.stats is None:
            print(f"[FAIL] {path.name:38} {'; '.join(result.errors)}")
        else:
            print(
                f"[{('OK' if result.ok else 'FAIL'):4}] {path.name:38} "
                f"tris={result.stats.triangles:5} meshes={result.stats.meshes:3} "
                f"nodes={result.stats.nodes:3} mats={result.stats.materials:2} "
                f"size={tuple(round(value, 2) for value in result.stats.dimensions)}m"
            )
            for warning in result.warnings:
                print(f"       WARN: {warning}")
                warnings += 1
            for error in result.errors:
                print(f"       ERROR: {error}")
        failures += len(result.errors)

    if not paths:
        print(f"FAIL: no GLBs found in {args.directory}")
        return 1
    print(
        f"\n{'PASS' if failures == 0 else 'FAIL'}: {len(paths)} GLBs, "
        f"{failures} errors, {warnings} recommendations"
    )
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
