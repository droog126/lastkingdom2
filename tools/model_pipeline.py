from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path

from audit_model_generators import audit
from glb_utils import validate_glb
from model_catalog import GENERATORS, category_for, generator_for_script, scale_contract_for
from model_style import COLLECTION_DIRS, ROOT


def find_blender() -> Path:
    configured = os.environ.get("BLENDER")
    candidates = [
        Path(configured) if configured else None,
        Path("F:/BLENDER/blender.exe"),
        Path("F:/BLENDER/blender-launcher.exe"),
    ]
    discovered = shutil.which("blender")
    if discovered:
        candidates.append(Path(discovered))
    for candidate in candidates:
        if candidate and candidate.is_file():
            return candidate
    raise FileNotFoundError("Blender not found; set BLENDER or install it on PATH")


def run_generator(script: str, asset: str | None, output_root: Path | None) -> None:
    blender = find_blender()
    env = os.environ.copy()
    if asset:
        env["LK2_MODEL_ONLY"] = asset
    if output_root:
        env["LK2_MODEL_OUTPUT_ROOT"] = str(output_root.resolve())
    command = [
        str(blender),
        "--background",
        "--python-exit-code",
        "1",
        "--python",
        str(ROOT / "tools" / script),
    ]
    print(f">>> {' '.join(command)}")
    subprocess.run(command, cwd=ROOT, env=env, check=True)


def validate_assets(asset_root: Path | None = None) -> int:
    report = audit()
    failures = list(report.errors)
    for warning in report.warnings:
        print(f"WARN: {warning}")
    for collection, production_directory in COLLECTION_DIRS.items():
        directory = (
            asset_root / production_directory.relative_to(ROOT / "assets")
            if asset_root
            else production_directory
        )
        for path in sorted(directory.glob("*.glb")):
            result = validate_glb(
                path,
                category_for(collection, path.stem),
                scale_contract_for(collection, path.stem),
            )
            if result.stats:
                print(
                    f"[{('OK' if result.ok else 'FAIL'):4}] {collection}/{path.name:35} "
                    f"tris={result.stats.triangles:5} nodes={result.stats.nodes:3} "
                    f"mats={result.stats.materials:2}"
                )
            for warning in result.warnings:
                print(f"  WARN: {warning}")
            failures.extend(f"{path}: {error}" for error in result.errors)
    for failure in failures:
        print(f"ERROR: {failure}")
    print(f"{'PASS' if not failures else 'FAIL'}: model pipeline validation")
    return 0 if not failures else 1


def main() -> int:
    parser = argparse.ArgumentParser(description="Build and validate canonical LK2 model assets")
    subparsers = parser.add_subparsers(dest="command", required=True)
    build = subparsers.add_parser("build")
    selection = build.add_mutually_exclusive_group(required=True)
    selection.add_argument("--asset")
    selection.add_argument("--generator")
    selection.add_argument("--all", action="store_true")
    build.add_argument("--output-root", type=Path)
    validate = subparsers.add_parser("validate")
    validate.add_argument("--root", type=Path, help="staging asset root created by --output-root")
    subparsers.add_parser("audit")
    args = parser.parse_args()

    if args.command == "validate":
        return validate_assets(args.root)
    if args.command == "audit":
        report = audit()
        for warning in report.warnings:
            print(f"WARN: {warning}")
        for error in report.errors:
            print(f"ERROR: {error}")
        return 0 if report.ok else 1

    if args.asset:
        matches = [
            spec for spec in GENERATORS if args.asset in spec.assets
        ]
        if len(matches) != 1:
            parser.error(f"asset {args.asset!r} has {len(matches)} canonical generators")
        run_generator(matches[0].script, args.asset, args.output_root)
    elif args.generator:
        spec = generator_for_script(args.generator)
        if spec is None:
            parser.error(f"unknown canonical generator: {args.generator}")
        run_generator(spec.script, None, args.output_root)
    else:
        for spec in GENERATORS:
            run_generator(spec.script, None, args.output_root)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
