from __future__ import annotations

from pathlib import Path


def retired_generator(script: str, replacement: str) -> int:
    name = Path(script).name
    print(f"ERROR: {name} is retired and cannot export production assets.")
    print(f"Use: python tools/{replacement}")
    return 2
