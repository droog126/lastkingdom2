#!/usr/bin/env python3
from __future__ import annotations

import sys

from harness.assertions import assertion_result, get_path, load_custom_assertions
from harness.cli import main
from harness.config import DEFAULT_CONFIG
from harness.health import built_in_assertions, combine, evaluate_iter, summarize, write_reports
from harness.image_metrics import analyze_png
from harness.state_metrics import analyze_sim, read_state

LUMA_VAR_HEALTHY = DEFAULT_CONFIG.luma_var_healthy
LUMA_MEAN_WHITE = DEFAULT_CONFIG.luma_mean_white
LUMA_MEAN_BLACK = DEFAULT_CONFIG.luma_mean_black
PNG_MIN_SIZE_KB = DEFAULT_CONFIG.png_min_size_kb
SIM_TICK_EARLY = DEFAULT_CONFIG.sim_tick_early
SIM_TICK_COMPLETE = DEFAULT_CONFIG.sim_tick_complete
SAMPLE_SIZE = DEFAULT_CONFIG.sample_size
PNG_MIN_W = DEFAULT_CONFIG.png_min_w
PNG_MIN_H = DEFAULT_CONFIG.png_min_h
TOP_COLOR_BUCKET_WARN_PCT = DEFAULT_CONFIG.top_color_bucket_warn_pct

__all__ = [
    "LUMA_VAR_HEALTHY",
    "LUMA_MEAN_WHITE",
    "LUMA_MEAN_BLACK",
    "PNG_MIN_SIZE_KB",
    "SIM_TICK_EARLY",
    "SIM_TICK_COMPLETE",
    "SAMPLE_SIZE",
    "PNG_MIN_W",
    "PNG_MIN_H",
    "TOP_COLOR_BUCKET_WARN_PCT",
    "analyze_png",
    "analyze_sim",
    "assertion_result",
    "built_in_assertions",
    "combine",
    "evaluate_iter",
    "get_path",
    "load_custom_assertions",
    "main",
    "read_state",
    "summarize",
    "write_reports",
]

if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
