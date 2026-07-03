from __future__ import annotations

import re
from pathlib import Path


ITER_RE = re.compile(r"^iter_(\d+)$")


def project_root_from(start: Path) -> Path:
    current = start.resolve()
    for candidate in [current, *current.parents]:
        if (candidate / "Cargo.toml").exists() and (candidate / "scripts" / "loop").exists():
            return candidate
    raise RuntimeError(f"could not locate project root from {start}")


def screenshots_dir(root: Path) -> Path:
    return root / "screenshots"


def iter_number(path: Path) -> int:
    match = ITER_RE.match(path.name)
    if not match:
        return -1
    return int(match.group(1))


def iter_dirs(root: Path) -> list[Path]:
    base = screenshots_dir(root)
    if not base.exists():
        return []
    return sorted(
        [p for p in base.iterdir() if p.is_dir() and iter_number(p) >= 0],
        key=iter_number,
    )


def latest_iter(root: Path) -> Path | None:
    dirs = iter_dirs(root)
    return dirs[-1] if dirs else None


def previous_iter(root: Path, current: Path | None = None) -> Path | None:
    dirs = iter_dirs(root)
    if not dirs:
        return None
    if current is None:
        return dirs[-2] if len(dirs) >= 2 else None
    current_num = iter_number(current)
    candidates = [p for p in dirs if iter_number(p) < current_num]
    return candidates[-1] if candidates else None


def newest_after(root: Path, previous_number: int) -> Path | None:
    candidates = [p for p in iter_dirs(root) if iter_number(p) > previous_number]
    return candidates[-1] if candidates else None
