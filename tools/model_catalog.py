from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from model_style import ScaleContract


@dataclass(frozen=True)
class GeneratorSpec:
    script: str
    collection: str
    category: str
    assets: tuple[str, ...]


GENERATORS: tuple[GeneratorSpec, ...] = (
    GeneratorSpec(
        "build_all_models.py",
        "pretty",
        "pretty",
        (
            "monster_snake",
            "monster_frost_elf",
            "monster_sand_wurm",
            "monster_treant",
            "monster_aether_wraith",
            "cloud_puff",
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
        ),
    ),
    GeneratorSpec(
        "build_sokpop_gathering_models.py",
        "pretty",
        "pretty",
        ("sokpop_gatherer", "fallen_stick", "sokpop_tree"),
    ),
    GeneratorSpec(
        "build_granular_flora.py",
        "pretty",
        "pretty",
        (
            "granular_round_tree",
            "granular_pine_tree",
            "forest_stone_spire",
            "granular_birch_tree",
            "granular_autumn_tree",
            "granular_willow_tree",
            "granular_wildflowers",
            "granular_reed_bank",
        ),
    ),
    GeneratorSpec(
        "build_models_v3.py",
        "pretty",
        "decor",
        (
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
            "palm",
            "resource_wood",
            "resource_stone",
        ),
    ),
    GeneratorSpec(
        "build_models_v4.py",
        "pretty",
        "decor",
        ("cauldron", "cooking_station", "spit_roast"),
    ),
    GeneratorSpec(
        "build_more_models.py",
        "pretty",
        "decor",
        (
            "campfire",
            "lantern_post",
            "crate",
            "barrel",
            "signpost",
            "market_stall",
            "bench",
            "fountain",
            "statue",
            "forge",
            "chapel",
            "pier",
            "tavern",
            "wolf",
            "bear",
        ),
    ),
    GeneratorSpec(
        "build_sokpop_style_pass.py",
        "pretty",
        "pretty",
        ("villager", "ground_patch", "lake", "swamp", "beach"),
    ),
    GeneratorSpec(
        "build_terrain_buildings.py",
        "pretty",
        "terrain",
        (
            "mountain_snow",
            "volcano",
            "desert_dune",
            "cliff",
            "cave_entrance",
            "house_small",
            "watchtower",
            "windmill",
            "bridge_stone",
            "well",
            "barn",
            "fence",
            "shrine",
            "lighthouse",
        ),
    ),
    GeneratorSpec(
        "build_hoplite_legendaries.py",
        "pretty",
        "weapons",
        (
            "sword",
            "hoplite_reaper_scythe",
            "hoplite_dragon_katana",
            "hoplite_golem_hammer",
            "hoplite_midas_sword",
        ),
    ),
    GeneratorSpec(
        "build_hoplite_dragon.py",
        "pretty",
        "bosses",
        ("hoplite_ender_dragon",),
    ),
    GeneratorSpec(
        "create_eco_models.py",
        "eco",
        "eco",
        ("rabbit", "berry_bush", "berry_fruit", "co2_bubble"),
    ),
    GeneratorSpec(
        "build_animals_blender.py",
        "animals",
        "animals",
        (
            "pig",
            "sheep",
            "cow",
            "chicken",
            "rabbit",
            "rabbit_brown",
            "deer",
            "deer_fawn",
            "fox",
            "fox_silver",
        ),
    ),
)


RETIRED_GENERATORS: dict[str, str] = {
    "build_all_flat.py": "build_all_models.py",
    "build_player_v2.py": "build_hoplite_legendaries.py",
    "build_player_v3.py": "build_sokpop_gathering_models.py",
    "build_v5_cute.py": "model_pipeline.py build --all",
    "build_v6_balanced.py": "model_pipeline.py build --all",
}


PRETTY_GROUPS: dict[str, tuple[str, ...]] = {
    "pretty": (
        "monster_snake",
        "monster_frost_elf",
        "monster_sand_wurm",
        "monster_treant",
        "monster_aether_wraith",
        "cloud_puff",
        "sokpop_gatherer",
        "sokpop_tree",
        "granular_round_tree",
        "granular_pine_tree",
        "forest_stone_spire",
        "granular_birch_tree",
        "granular_autumn_tree",
        "granular_willow_tree",
        "granular_wildflowers",
        "granular_reed_bank",
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
    ),
    "terrain": (
        "mountain_snow",
        "volcano",
        "desert_dune",
        "lake",
        "swamp",
        "cliff",
        "cave_entrance",
        "beach",
        "ground_patch",
    ),
    "buildings": (
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
    ),
    "decor": (
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
        "palm",
        "resource_wood",
        "resource_stone",
    ),
    "weapons": (
        "sword",
        "hoplite_reaper_scythe",
        "hoplite_dragon_katana",
        "hoplite_golem_hammer",
        "hoplite_midas_sword",
    ),
    "bosses": ("hoplite_ender_dragon",),
    "creatures": ("villager", "wolf", "bear"),
}

ECO_ASSETS = ("rabbit", "berry_bush", "berry_fruit", "co2_bubble")


SCALE_CONTRACTS: dict[tuple[str, str], ScaleContract] = {}


def _scale(
    collection: str,
    metric: str,
    target_meters: float,
    *assets: str,
    grounded: bool = True,
    height_meters: float | None = None,
) -> None:
    contract = ScaleContract(metric, target_meters, grounded, target_height_meters=height_meters)
    for asset in assets:
        key = (collection, asset)
        if key in SCALE_CONTRACTS:
            raise RuntimeError(f"duplicate scale contract: {key}")
        SCALE_CONTRACTS[key] = contract


_scale("pretty", "max_extent", 1.5, "monster_snake", height_meters=0.8)
_scale("pretty", "height", 1.8, "monster_frost_elf")
_scale("pretty", "max_extent", 2.2, "monster_sand_wurm", height_meters=1.2)
_scale("pretty", "height", 3.2, "monster_treant")
_scale("pretty", "height", 2.0, "monster_aether_wraith", grounded=False)
_scale("pretty", "max_extent", 4.0, "cloud_puff", grounded=False, height_meters=2.0)
_scale("pretty", "height", 1.7, "sokpop_gatherer", "villager")
_scale("pretty", "height", 5.0, "sokpop_tree")
_scale("pretty", "height", 5.5, "granular_round_tree")
_scale("pretty", "height", 6.5, "granular_pine_tree")
_scale("pretty", "height", 7.0, "forest_stone_spire")
_scale("pretty", "height", 5.4, "granular_birch_tree")
_scale("pretty", "height", 5.2, "granular_autumn_tree")
_scale("pretty", "height", 5.0, "granular_willow_tree")
_scale("pretty", "height", 0.65, "granular_wildflowers")
_scale("pretty", "height", 1.4, "granular_reed_bank")
_scale("pretty", "max_extent", 1.2, "fallen_stick", height_meters=0.25)
_scale("pretty", "max_extent", 0.9, "rock_dark", height_meters=0.5)
_scale("pretty", "max_extent", 0.75, "rock_mid", height_meters=0.45)
_scale("pretty", "max_extent", 0.65, "rock_moss", height_meters=0.4)
_scale("pretty", "height", 0.45, "flower_0", "flower_1", "flower_2", "flower_3", "flower_4")
_scale("pretty", "max_extent", 8.0, "hill", height_meters=2.5)
_scale("pretty", "height", 3.8, "poi_pillar_red")
_scale("pretty", "height", 2.8, "poi_pillar_cyan")
_scale("pretty", "height", 2.5, "poi_pillar_pink")
_scale("pretty", "height", 2.4, "poi_pillar_gold")
_scale("pretty", "max_extent", 12.0, "ground_disc_outer", height_meters=0.2)
_scale("pretty", "max_extent", 5.0, "ground_disc_inner", height_meters=0.2)

_scale("pretty", "max_extent", 12.0, "mountain_snow", height_meters=6.5)
_scale("pretty", "max_extent", 10.0, "volcano", height_meters=6.5)
_scale("pretty", "max_extent", 8.0, "desert_dune", height_meters=2.0)
_scale("pretty", "max_extent", 8.0, "lake", height_meters=0.8)
_scale("pretty", "max_extent", 8.0, "beach", height_meters=2.5)
_scale("pretty", "max_extent", 7.0, "swamp", height_meters=1.0)
_scale("pretty", "max_extent", 8.0, "cliff", height_meters=6.0)
_scale("pretty", "height", 5.0, "cave_entrance")
_scale("pretty", "max_extent", 6.0, "ground_patch", height_meters=0.5)

_scale("pretty", "height", 4.0, "house_small")
_scale("pretty", "height", 6.0, "watchtower", "windmill")
_scale("pretty", "max_extent", 8.0, "bridge_stone", height_meters=3.0)
_scale("pretty", "max_extent", 8.0, "pier", height_meters=2.5)
_scale("pretty", "height", 2.2, "well")
_scale("pretty", "height", 4.5, "barn", "forge")
_scale("pretty", "height", 1.5, "fence")
_scale("pretty", "height", 3.0, "shrine")
_scale("pretty", "height", 8.0, "lighthouse")
_scale("pretty", "height", 5.0, "chapel", "tavern")

_scale("pretty", "height", 1.2, "campfire", "barrel", "bench", "cauldron")
_scale("pretty", "height", 3.0, "lantern_post", "market_stall", "statue")
_scale("pretty", "height", 2.4, "fountain")
_scale("pretty", "max_extent", 1.0, "crate")
_scale("pretty", "height", 2.3, "signpost")
_scale("pretty", "height", 0.8, "mushroom_red")
_scale("pretty", "height", 0.6, "mushroom_brown")
_scale("pretty", "height", 1.2, "crystal_blue")
_scale("pretty", "height", 1.0, "crystal_pink", "treasure_chest")
_scale("pretty", "max_extent", 5.0, "boat", height_meters=2.5)
_scale("pretty", "height", 3.5, "arch_stone")
_scale("pretty", "max_extent", 3.0, "cart", height_meters=1.8)
_scale("pretty", "height", 1.5, "tombstone")
_scale("pretty", "height", 2.0, "haystack", "cooking_station")
_scale("pretty", "height", 1.8, "spit_roast")
_scale("pretty", "height", 4.5, "palm")
_scale("pretty", "max_extent", 1.0, "resource_wood")
_scale("pretty", "max_extent", 0.8, "resource_stone")
_scale("pretty", "max_extent", 2.2, "wolf", height_meters=1.2)
_scale("pretty", "height", 1.7, "bear")

_scale("pretty", "height", 1.10, "sword")
_scale("pretty", "height", 1.95, "hoplite_reaper_scythe")
_scale("pretty", "height", 1.15, "hoplite_dragon_katana")
_scale("pretty", "height", 1.35, "hoplite_golem_hammer")
_scale("pretty", "height", 1.20, "hoplite_midas_sword")
_scale("pretty", "max_extent", 6.0, "hoplite_ender_dragon", height_meters=2.6)

_scale("eco", "height", 0.65, "rabbit")
_scale("eco", "height", 1.1, "berry_bush")
_scale("eco", "max_extent", 0.25, "berry_fruit", height_meters=0.25)
_scale("eco", "max_extent", 0.8, "co2_bubble", grounded=False, height_meters=0.8)

_scale("animals", "height", 0.8, "pig")
_scale("animals", "height", 1.0, "sheep")
_scale("animals", "height", 1.5, "cow")
_scale("animals", "height", 0.7, "chicken")
_scale("animals", "height", 0.6, "rabbit")
_scale("animals", "height", 0.6, "rabbit_brown")
_scale("animals", "height", 1.4, "deer")
_scale("animals", "height", 1.15, "deer_fawn")
_scale("animals", "height", 0.9, "fox")
_scale("animals", "height", 0.9, "fox_silver")


def _asset_index() -> dict[tuple[str, str], GeneratorSpec]:
    index: dict[tuple[str, str], GeneratorSpec] = {}
    for generator in GENERATORS:
        for asset in generator.assets:
            key = (generator.collection, asset)
            if key in index:
                raise RuntimeError(f"duplicate generator ownership for {key}: {index[key]} and {generator}")
            index[key] = generator
    return index


ASSET_INDEX = _asset_index()


def owner_for(collection: str, asset: str) -> GeneratorSpec | None:
    return ASSET_INDEX.get((collection, asset))


def generator_for_script(script: str | Path) -> GeneratorSpec | None:
    name = Path(script).name
    return next((generator for generator in GENERATORS if generator.script == name), None)


def category_for(collection: str, asset: str) -> str:
    if collection == "pretty":
        for category, assets in PRETTY_GROUPS.items():
            if asset in assets:
                return category
    owner = owner_for(collection, asset)
    return owner.category if owner else collection


def scale_contract_for(collection: str, asset: str) -> ScaleContract | None:
    return SCALE_CONTRACTS.get((collection, asset))
