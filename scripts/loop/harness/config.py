from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class HarnessConfig:
    luma_var_healthy: float = 100.0
    luma_mean_white: float = 200.0
    luma_mean_black: float = 20.0
    png_min_size_kb: float = 30.0
    sim_tick_early: int = 30
    sim_tick_complete: int = 500
    sample_size: int = 64
    png_min_w: int = 640
    png_min_h: int = 360
    top_color_bucket_warn_pct: float = 92.0


DEFAULT_CONFIG = HarnessConfig()
