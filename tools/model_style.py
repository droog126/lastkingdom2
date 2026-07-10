from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
ASSET_ROOT = ROOT / "assets"
PRETTY_DIR = ASSET_ROOT / "procedural" / "pretty"
ECO_DIR = ASSET_ROOT / "procedural" / "eco"
ANIMALS_DIR = ASSET_ROOT / "animals"
TEMP_PREVIEW_DIR = ROOT / ".tmp" / "model-previews"

STYLE_NAME = "lk2-sokpop-toy"
STYLE_VERSION = 1
HARD_TRIANGLE_LIMIT = 5_000
MAX_NODE_COUNT = 256
MAX_MATERIAL_COUNT = 32


@dataclass(frozen=True)
class CategoryBudget:
    triangles: int
    recommended_materials: int
    recommended_nodes: int


@dataclass(frozen=True)
class ScaleContract:
    metric: str
    target_meters: float
    grounded: bool = True
    tolerance: float = 0.08
    target_height_meters: float | None = None

    def __post_init__(self) -> None:
        if self.metric not in {"height", "max_extent"}:
            raise ValueError(f"unsupported scale metric: {self.metric}")
        if self.target_meters <= 0.0:
            raise ValueError("target_meters must be positive")
        if self.target_height_meters is not None and self.target_height_meters <= 0.0:
            raise ValueError("target_height_meters must be positive")


CATEGORY_BUDGETS: dict[str, CategoryBudget] = {
    "pretty": CategoryBudget(3_500, 10, 96),
    "terrain": CategoryBudget(5_000, 12, 128),
    "buildings": CategoryBudget(5_000, 16, 160),
    "decor": CategoryBudget(3_500, 12, 128),
    "creatures": CategoryBudget(4_000, 12, 128),
    "weapons": CategoryBudget(3_000, 8, 96),
    "bosses": CategoryBudget(5_000, 12, 160),
    "eco": CategoryBudget(3_500, 10, 128),
    "animals": CategoryBudget(3_500, 10, 128),
}


COLLECTION_DIRS = {
    "pretty": PRETTY_DIR,
    "eco": ECO_DIR,
    "animals": ANIMALS_DIR,
}


def budget_for(category: str) -> CategoryBudget:
    return CATEGORY_BUDGETS.get(category, CategoryBudget(HARD_TRIANGLE_LIMIT, 12, 128))


def validate_rgb(color: tuple[float, float, float], label: str = "color") -> None:
    if len(color) != 3 or any(component < 0.0 or component > 1.0 for component in color):
        raise ValueError(f"{label} must contain three values in [0, 1], got {color!r}")
