from __future__ import annotations

import json

from model_catalog import ECO_ASSETS, PRETTY_GROUPS
from model_style import ECO_DIR, PRETTY_DIR, STYLE_NAME, STYLE_VERSION


def _existing_registered(directory, registered: set[str]) -> set[str]:
    stems = {path.stem for path in directory.glob("*.glb")}
    unregistered = stems - registered
    if unregistered:
        raise RuntimeError(f"unregistered GLBs in {directory}: {sorted(unregistered)}")
    return stems


def sync_pretty() -> None:
    registered = {asset for assets in PRETTY_GROUPS.values() for asset in assets}
    stems = _existing_registered(PRETTY_DIR, registered)
    groups = {
        group: [asset for asset in assets if asset in stems]
        for group, assets in PRETTY_GROUPS.items()
    }
    manifest = {
        "version": 9,
        "spec": f"{STYLE_NAME}-v{STYLE_VERSION} canonical generated GLBs",
        "format": "glb",
        "assets": {group: assets for group, assets in groups.items() if assets},
    }
    PRETTY_DIR.joinpath("MANIFEST.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def sync_eco() -> None:
    registered = set(ECO_ASSETS)
    stems = _existing_registered(ECO_DIR, registered)
    manifest = {
        "version": 9,
        "spec": f"{STYLE_NAME}-v{STYLE_VERSION} canonical eco GLBs",
        "format": "glb",
        "assets": {"eco": [asset for asset in ECO_ASSETS if asset in stems]},
    }
    ECO_DIR.joinpath("MANIFEST.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def main() -> int:
    sync_pretty()
    sync_eco()
    print(f"synced {PRETTY_DIR / 'MANIFEST.json'}")
    print(f"synced {ECO_DIR / 'MANIFEST.json'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
