from __future__ import annotations

from pathlib import Path
from typing import Any

from .config import DEFAULT_CONFIG, HarnessConfig

try:
    from PIL import Image
except ImportError:  # pragma: no cover - exercised by CLI environment, not unit tests.
    Image = None


def analyze_png(png_path: Path, config: HarnessConfig = DEFAULT_CONFIG) -> dict[str, Any]:
    out: dict[str, Any] = {
        "file_kb": round(png_path.stat().st_size / 1024, 1),
        "verdict": "NA",
        "sub": None,
    }
    if Image is None:
        out["verdict"] = "ERR"
        out["err"] = "Pillow not installed. Run: pip install Pillow"
        return out

    try:
        img = Image.open(png_path).convert("RGB")
        out["w"], out["h"] = img.size
        small = img.resize((config.sample_size, config.sample_size), Image.LANCZOS)

        try:
            import numpy as np

            arr = np.asarray(small, dtype=np.float32)
            r, g, b = arr[..., 0], arr[..., 1], arr[..., 2]
            luma = 0.299 * r + 0.587 * g + 0.114 * b
            luma_mean = float(luma.mean())
            luma_var = float(luma.var())

            buckets = (arr.astype(np.uint8) >> 5).reshape(-1, 3)
            keys = (
                (buckets[:, 0].astype(np.int32) << 10)
                | (buckets[:, 1].astype(np.int32) << 5)
                | buckets[:, 2].astype(np.int32)
            )
            _unique, counts = np.unique(keys, return_counts=True)
            top_pct = float(counts.max() / keys.size * 100)
        except ImportError:
            data = small.tobytes()
            n = len(data) // 3
            luma_total = 0.0
            luma_sq_total = 0.0
            bucket_counts: dict[tuple[int, int, int], int] = {}
            for i in range(n):
                r = data[i * 3]
                g = data[i * 3 + 1]
                b = data[i * 3 + 2]
                luma = 0.299 * r + 0.587 * g + 0.114 * b
                luma_total += luma
                luma_sq_total += luma * luma
                key = (r >> 5, g >> 5, b >> 5)
                bucket_counts[key] = bucket_counts.get(key, 0) + 1
            luma_mean = luma_total / n
            luma_var = luma_sq_total / n - luma_mean**2
            top_pct = max(bucket_counts.values()) / n * 100

        out["luma_mean"] = round(luma_mean, 1)
        out["luma_var"] = round(luma_var, 1)
        out["top_pct"] = round(top_pct, 1)

        if luma_var < config.luma_var_healthy:
            out["verdict"] = "UNIFORM"
            if luma_mean > config.luma_mean_white:
                out["sub"] = "WHITE"
            elif luma_mean < config.luma_mean_black:
                out["sub"] = "BLACK"
            else:
                out["sub"] = "GRAY"
        else:
            out["verdict"] = "OK"
        return out
    except Exception as e:
        out["verdict"] = "ERR"
        out["err"] = str(e)[:120]
        return out
