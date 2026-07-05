from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PROCEDURAL = ROOT / "assets" / "procedural"

PRETTY_GROUPS = {
    "pretty": [
        "player_avatar",
        "monster_snake",
        "monster_frost_elf",
        "monster_sand_wurm",
        "monster_treant",
        "monster_aether_wraith",
        "cloud_puff",
        "sokpop_gatherer",
        "sokpop_tree",
        "tree",
        "fallen_stick",
        "rock_dark",
        "rock_mid",
        "rock_moss",
        "flower_0",
        "flower_1",
        "flower_2",
        "flower_3",
        "flower_4",
        "hill",
        "poi_pillar_red",
        "poi_pillar_cyan",
        "poi_pillar_pink",
        "poi_pillar_gold",
        "ground_disc_outer",
        "ground_disc_inner",
    ],
    "terrain": [
        "mountain_snow",
        "volcano",
        "desert_dune",
        "lake",
        "swamp",
        "cliff",
        "cave_entrance",
        "beach",
        "ground_patch",
    ],
    "buildings": [
        "house_small",
        "watchtower",
        "windmill",
        "bridge_stone",
        "well",
        "barn",
        "fence",
        "shrine",
        "lighthouse",
        "forge",
        "chapel",
        "pier",
        "tavern",
    ],
    "decor": [
        "campfire",
        "lantern_post",
        "crate",
        "barrel",
        "signpost",
        "market_stall",
        "bench",
        "fountain",
        "statue",
        "mushroom_red",
        "mushroom_brown",
        "crystal_blue",
        "crystal_pink",
        "treasure_chest",
        "boat",
        "arch_stone",
        "cart",
        "tombstone",
        "haystack",
        "cauldron",
        "cooking_station",
        "spit_roast",
        "sword",
    ],
    "creatures": ["villager", "wolf", "bear"],
    "test": ["test_cube"],
}

ECO_ASSETS = ["rabbit", "berry_bush", "berry_fruit", "co2_bubble"]


def existing(names: list[str], stems: set[str]) -> list[str]:
    return [name for name in names if name in stems]


def sync_pretty() -> None:
    pretty = PROCEDURAL / "pretty"
    stems = {path.stem for path in pretty.glob("*.glb")}
    grouped = {group: existing(names, stems) for group, names in PRETTY_GROUPS.items()}
    grouped = {group: names for group, names in grouped.items() if names}
    known = {name for names in grouped.values() for name in names}
    extras = sorted(stems - known)
    if extras:
        grouped["uncategorized"] = extras
    manifest = {
        "version": 8,
        "spec": "procedural generated GLB manifest, synced from assets/procedural/pretty",
        "format": "glb",
        "assets": grouped,
    }
    (pretty / "MANIFEST.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def sync_eco() -> None:
    eco = PROCEDURAL / "eco"
    stems = {path.stem for path in eco.glob("*.glb")}
    manifest = {
        "version": 8,
        "spec": "procedural generated GLB manifest, synced from assets/procedural/eco",
        "format": "glb",
        "assets": {
            "eco": existing(ECO_ASSETS, stems),
            "uncategorized": sorted(stems - set(ECO_ASSETS)),
        },
    }
    if not manifest["assets"]["uncategorized"]:
        del manifest["assets"]["uncategorized"]
    (eco / "MANIFEST.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


def main() -> int:
    sync_pretty()
    sync_eco()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
