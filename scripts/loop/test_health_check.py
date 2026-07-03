import json
import random
import sys
import tempfile
import unittest
from pathlib import Path

from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parent))
import health_check
from harness import evaluate_iter, write_reports
from harness.cli import main as harness_main
from harness.orchestrator import run_loop, run_suite


def write_state(path: Path, tick: int) -> None:
    state = {
        "tick": tick,
        "player": {
            "block_pos": [10, 12, 10],
            "blocks_gathered": 1,
            "monsters_killed": 0,
            "nations_founded": 1,
        },
        "nations": {"total_nations": 1},
        "observer": {"anomalies": 0, "invariant_violations": 0},
        "world": {"size": 96},
    }
    path.write_text(json.dumps(state), encoding="utf-8")


def write_gradient_png(path: Path, size: tuple[int, int] = (1280, 720)) -> None:
    img = Image.new("RGB", size)
    pixels = img.load()
    rng = random.Random(7)
    block = 16
    colors = {}
    for y in range(size[1]):
        for x in range(size[0]):
            key = (x // block, y // block)
            if key not in colors:
                colors[key] = (
                    rng.randrange(40, 230),
                    rng.randrange(40, 230),
                    rng.randrange(40, 230),
                )
            r, g, b = colors[key]
            noise = ((x * 17 + y * 31) % 29) - 14
            pixels[x, y] = (
                max(0, min(255, r + noise)),
                max(0, min(255, g - noise)),
                max(0, min(255, b + noise // 2)),
            )
    img.save(path)


class HealthCheckTests(unittest.TestCase):
    def run_health(self, iter_dir: Path) -> dict:
        result = evaluate_iter(iter_dir)
        write_reports(result)
        return json.loads((iter_dir / "health.json").read_text(encoding="utf-8"))

    def test_complete_state_and_readable_png_pass(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            iter_dir = Path(d)
            write_gradient_png(iter_dir / "iter_001.png")
            write_state(iter_dir / "final_state.json", 500)

            health = self.run_health(iter_dir)

            self.assertEqual(health["verdict"], "PASS")
            self.assertEqual(health["assertions"]["failed"], 0)
            self.assertTrue((iter_dir / "assertions.json").exists())

    def test_incomplete_tick_is_partial_not_pass(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            iter_dir = Path(d)
            write_gradient_png(iter_dir / "iter_001.png")
            write_state(iter_dir / "final_state.json", 90)

            health = self.run_health(iter_dir)

            self.assertEqual(health["verdict"], "PARTIAL")
            self.assertIn("sim incomplete", "; ".join(health["reasons"]))

    def test_black_one_pixel_png_fails_with_actionable_assertions(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            iter_dir = Path(d)
            Image.new("RGB", (1, 1), (0, 0, 0)).save(iter_dir / "iter_001.png")
            write_state(iter_dir / "final_state.json", 90)

            health = self.run_health(iter_dir)
            assertions = json.loads(
                (iter_dir / "assertions.json").read_text(encoding="utf-8")
            )["assertions"]
            failed_ids = {a["id"] for a in assertions if not a["ok"]}

            self.assertEqual(health["verdict"], "FAIL")
            self.assertIn("png.readable", failed_ids)
            self.assertIn("png.width", failed_ids)
            self.assertIn("png.height", failed_ids)

    def test_legacy_health_check_entrypoint_still_writes_reports(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            iter_dir = Path(d)
            write_gradient_png(iter_dir / "iter_001.png")
            write_state(iter_dir / "final_state.json", 500)

            rc = health_check.main([str(iter_dir)])

            self.assertEqual(rc, 0)
            self.assertTrue((iter_dir / "health.json").exists())
            self.assertTrue((iter_dir / "assertions.json").exists())

    def test_harness_eval_subcommand_writes_reports(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            iter_dir = Path(d)
            write_gradient_png(iter_dir / "iter_001.png")
            write_state(iter_dir / "final_state.json", 500)

            rc = harness_main(["eval", str(iter_dir)])

            self.assertEqual(rc, 0)
            self.assertEqual(
                json.loads((iter_dir / "health.json").read_text(encoding="utf-8"))["verdict"],
                "PASS",
            )

    def test_suite_evaluates_iter_directories(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / "scripts" / "loop").mkdir(parents=True)
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            iter_dir = root / "screenshots" / "iter_001"
            iter_dir.mkdir(parents=True)
            write_gradient_png(iter_dir / "iter_001.png")
            write_state(iter_dir / "final_state.json", 500)

            rc, report = run_suite(root, "screenshots/iter_*")

            self.assertEqual(rc, 0)
            self.assertEqual(report["pass"], 1)
            self.assertTrue((root / "run-logs" / "harness_suite.json").exists())

    def test_run_reports_no_new_iter_without_reusing_latest(self) -> None:
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            loop_dir = root / "scripts" / "loop"
            loop_dir.mkdir(parents=True)
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            (loop_dir / "loop.ps1").write_text("exit 0\n", encoding="utf-8")
            existing = root / "screenshots" / "iter_001"
            existing.mkdir(parents=True)
            write_gradient_png(existing / "iter_001.png")
            write_state(existing / "final_state.json", 500)

            rc, manifest = run_loop(root, seconds=1, max_extra_wait=1, skip_build=True)

            self.assertEqual(rc, 2)
            self.assertEqual(manifest["failure"], "no_new_iter")
            self.assertIsNone(manifest["iter"])
            self.assertTrue((root / "run-logs" / "harness_last_run.json").exists())


if __name__ == "__main__":
    unittest.main()
