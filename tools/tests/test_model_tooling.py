from __future__ import annotations

import sys
import unittest
from pathlib import Path


TOOLS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(TOOLS))

from audit_model_generators import audit
from glb_utils import validate_glb
from model_catalog import ASSET_INDEX, PRETTY_GROUPS, SCALE_CONTRACTS, scale_contract_for
from model_style import PRETTY_DIR


class ModelToolingTests(unittest.TestCase):
    def test_pretty_catalog_has_no_duplicate_group_members(self) -> None:
        assets = [asset for group in PRETTY_GROUPS.values() for asset in group]
        self.assertEqual(len(assets), len(set(assets)))
        self.assertEqual(set(assets), {asset for collection, asset in ASSET_INDEX if collection == "pretty"})

    def test_generator_audit_passes(self) -> None:
        report = audit()
        self.assertEqual(report.errors, ())

    def test_every_asset_has_a_scale_contract(self) -> None:
        self.assertEqual(set(ASSET_INDEX), set(SCALE_CONTRACTS))

    def test_hoplite_reference_asset_meets_contract(self) -> None:
        result = validate_glb(
            PRETTY_DIR / "hoplite_dragon_katana.glb",
            "weapons",
            scale_contract_for("pretty", "hoplite_dragon_katana"),
        )
        self.assertTrue(result.ok, result.errors)
        self.assertIsNotNone(result.stats)
        self.assertLessEqual(result.stats.triangles, 3_000)


if __name__ == "__main__":
    unittest.main()
