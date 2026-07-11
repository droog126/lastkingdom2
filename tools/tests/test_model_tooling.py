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

    def test_handheld_weapon_scale_is_relative_to_human_height(self) -> None:
        human_height = scale_contract_for("pretty", "sokpop_gatherer").target_meters
        sidearms = ("sword", "hoplite_dragon_katana", "hoplite_midas_sword")

        for asset in sidearms:
            with self.subTest(asset=asset):
                height = scale_contract_for("pretty", asset).target_meters
                self.assertLessEqual(height, human_height * 0.75)

        hammer_height = scale_contract_for("pretty", "hoplite_golem_hammer").target_meters
        self.assertLessEqual(hammer_height, human_height * 0.85)

        scythe_height = scale_contract_for("pretty", "hoplite_reaper_scythe").target_meters
        self.assertGreater(scythe_height, human_height)
        self.assertLessEqual(scythe_height, human_height * 1.20)


if __name__ == "__main__":
    unittest.main()
