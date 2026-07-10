from __future__ import annotations

import ast
import re
from dataclasses import dataclass
from pathlib import Path

from model_catalog import (
    ASSET_INDEX,
    ECO_ASSETS,
    GENERATORS,
    PRETTY_GROUPS,
    RETIRED_GENERATORS,
    SCALE_CONTRACTS,
    generator_for_script,
)
from model_style import ANIMALS_DIR, ECO_DIR, PRETTY_DIR, ROOT


TOOLS_DIR = ROOT / "tools"
ABSOLUTE_PATH = re.compile(r"(?:[A-Za-z]:[\\/]|/home/|/Users/)")


@dataclass(frozen=True)
class AuditReport:
    errors: tuple[str, ...]
    warnings: tuple[str, ...]

    @property
    def ok(self) -> bool:
        return not self.errors


def _imports_module(tree: ast.Module, module: str) -> bool:
    for node in tree.body:
        if isinstance(node, ast.Import) and any(alias.name == module for alias in node.names):
            return True
        if isinstance(node, ast.ImportFrom) and node.module == module:
            return True
    return False


def _literal_exports(tree: ast.Module) -> set[str]:
    assets: set[str] = set()
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        name = node.func.id if isinstance(node.func, ast.Name) else None
        if name != "export_glb" or not node.args:
            continue
        first = node.args[0]
        if isinstance(first, ast.Constant) and isinstance(first.value, str):
            assets.add(first.value)
    return assets


def _catalog_assets(collection: str) -> set[str]:
    return {asset for (asset_collection, asset) in ASSET_INDEX if asset_collection == collection}


def audit() -> AuditReport:
    errors: list[str] = []
    warnings: list[str] = []
    registered = {generator.script for generator in GENERATORS}
    discovered = {path.name for path in TOOLS_DIR.glob("build_*.py")}
    discovered.add("create_eco_models.py")
    known = registered | set(RETIRED_GENERATORS)
    for script in sorted(discovered - known):
        errors.append(f"unregistered generator: {script}")
    for script in sorted(known - discovered):
        errors.append(f"catalog references missing generator: {script}")

    for script in sorted(discovered):
        path = TOOLS_DIR / script
        try:
            source = path.read_text(encoding="utf-8")
            tree = ast.parse(source, filename=str(path))
        except (OSError, UnicodeError, SyntaxError) as exc:
            errors.append(f"{script}: cannot parse as UTF-8 Python: {exc}")
            continue

        if script in RETIRED_GENERATORS:
            if "retired_generator" not in source:
                errors.append(f"{script}: retired generator must use model_legacy.retired_generator")
            if "export_glb(" in source or "export_scene.gltf" in source:
                errors.append(f"{script}: retired generator still contains export code")
            continue

        spec = generator_for_script(script)
        if spec is None:
            continue
        if not _imports_module(tree, "models_lib"):
            errors.append(f"{script}: canonical generator must import models_lib")
        if "export_scene.gltf" in source:
            errors.append(f"{script}: call models_lib.export_glb instead of bpy export operator")
        if "MANIFEST.json" in source or ".write_text(" in source:
            errors.append(f"{script}: generators must not write manifests or repository metadata")
        for match in ABSOLUTE_PATH.finditer(source):
            line = source.count("\n", 0, match.start()) + 1
            errors.append(f"{script}:{line}: hard-coded absolute path")
        owned = set(spec.assets)
        for asset in sorted(_literal_exports(tree) - owned):
            owner = next(
                (
                    candidate.script
                    for candidate in GENERATORS
                    if candidate.collection == spec.collection and asset in candidate.assets
                ),
                "unregistered",
            )
            errors.append(f"{script}: exports {asset!r}, owned by {owner}")

    pretty_catalog = _catalog_assets("pretty")
    grouped_pretty = {asset for assets in PRETTY_GROUPS.values() for asset in assets}
    if pretty_catalog != grouped_pretty:
        errors.append(
            "pretty manifest groups and generator catalog differ: "
            f"missing={sorted(pretty_catalog - grouped_pretty)}, extra={sorted(grouped_pretty - pretty_catalog)}"
        )
    if _catalog_assets("eco") != set(ECO_ASSETS):
        errors.append("eco catalog and ECO_ASSETS differ")
    if set(ASSET_INDEX) != set(SCALE_CONTRACTS):
        errors.append(
            "scale contracts and generator catalog differ: "
            f"missing={sorted(set(ASSET_INDEX) - set(SCALE_CONTRACTS))}, "
            f"extra={sorted(set(SCALE_CONTRACTS) - set(ASSET_INDEX))}"
        )

    existing_sets = {
        "pretty": {path.stem for path in PRETTY_DIR.glob("*.glb")},
        "eco": {path.stem for path in ECO_DIR.glob("*.glb")},
        "animals": {path.stem for path in ANIMALS_DIR.glob("*.glb")},
    }
    for collection, existing in existing_sets.items():
        catalog = _catalog_assets(collection)
        if existing - catalog:
            errors.append(f"{collection}: unowned GLBs: {sorted(existing - catalog)}")
        if catalog - existing:
            warnings.append(f"{collection}: registered but not generated: {sorted(catalog - existing)}")

    return AuditReport(tuple(errors), tuple(warnings))


def main() -> int:
    report = audit()
    for warning in report.warnings:
        print(f"WARN: {warning}")
    for error in report.errors:
        print(f"ERROR: {error}")
    print(
        f"{'PASS' if report.ok else 'FAIL'}: model generator audit "
        f"({len(report.errors)} errors, {len(report.warnings)} warnings)"
    )
    return 0 if report.ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
