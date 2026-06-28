
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(r"F:\rustProject\lastkingdom2")

EXTS = {".md", ".rs", ".html"}

EXCLUDE_DIRS = {
    ".git", "target", "screenshots", "run-logs",
    ".idea", ".vscode", ".opencode", ".mavis", ".cargo", ".harness", ".github",
    "scripts",
}

SCRIPT_NAMES = ("loop", "tdd", "run_audit")

PATTERN = re.compile(r"(?<![/\w])(" + "|".join(SCRIPT_NAMES) + r")\.ps1(?!\w)")
REPL = r"scripts/\1.ps1"

changed_files: list[Path] = []

def should_skip(p: Path) -> bool:
    rel = p.relative_to(ROOT)
    parts = rel.parts

    return any(part in EXCLUDE_DIRS for part in parts)

def process(p: Path) -> None:
    try:
        original = p.read_text(encoding="utf-8")
    except UnicodeDecodeError:

        return
    new = PATTERN.sub(REPL, original)
    if new != original:
        p.write_text(new, encoding="utf-8")
        changed_files.append(p)

def main() -> int:
    for p in ROOT.rglob("*"):
        if not p.is_file():
            continue
        if p.suffix.lower() not in EXTS:
            continue
        if should_skip(p):
            continue
        process(p)
    print(f"changed {len(changed_files)} file(s):")
    for f in changed_files:
        print(f"  {f.relative_to(ROOT)}")
    return 0

if __name__ == "__main__":
    sys.exit(main())
