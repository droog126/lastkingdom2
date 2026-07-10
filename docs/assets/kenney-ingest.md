<!-- doc-status: reference -->
# Kenney Asset Ingest

> Document status: asset-ingest reference; verify paths and runtime behavior before reuse.

The five imported Kenney packs were digested into `assets/kenney/curated/` on 2026-07-04 and the
source pack directories have been removed from the working tree. Bevy loads curated GLBs by
repo-relative asset paths only.

## Inventory

| Pack | GLBs | Best Use |
| --- | ---: | --- |
| `kenney_cube-pets_1.0` | 24 | passive animals, biome wildlife |
| `kenney_mini-characters` | 26 | villagers, NPCs, player avatar experiments |
| `kenney_pirate-kit` | 72 | coastal POIs, boats, docks, pirate props |
| `kenney_platformer-kit` | 153 | terrain chunks, pickups, doors, fences |
| `kenney_survival-kit` | 80 | camps, tools, resource drops, workstations |

Total: 355 GLBs across the original packs. **27 of those are now promoted into the project** under
`assets/kenney/curated/`, grouped by role (animals, characters, coastal_and_pirate, survival_props,
terrain_and_pickups). The remaining 328 stay in the upstream packs and are not loaded by runtime.

## License

Each pack includes `License.txt` and declares Creative Commons Zero (CC0). Credit to
Kenney / `www.kenney.nl` is appreciated but not required.

## Curated Layout

```
assets/kenney/
├── MANIFEST.json          # canonical index of curated IDs, with original `source` for traceability
└── curated/
    ├── animals/           # 5  passive / neutral creatures
    ├── characters/        # 4  villagers and player avatar experiments
    ├── coastal_and_pirate/# 6  boats, docks, cannons, palm trees, flags
    ├── survival_props/    # 7  camps, tools, resource drops, workstations
    └── terrain_and_pickups/# 5 terrain blocks and pickups (coin, heart, door)
```

Each curated GLB is renamed to its `id` so the file path equals the manifest ID, e.g.
`kenney/curated/animals/kenney_bunny.glb`.

## Runtime Strategy

Load GLBs by repo-relative asset paths. The manifest is the source of truth:

```rust
asset_server.load("kenney/curated/animals/kenney_bunny.glb#Scene0")
```

Start with curated IDs from the manifest, organized by role:

- `kenney_bunny`, `kenney_deer`, `kenney_cow`, `kenney_fox`, `kenney_bear` for passive animal tests.
- `kenney_villager_male_a`, `kenney_villager_female_a`, `kenney_player_male_b`,
  `kenney_player_female_b` for settlement population and player avatars.
- `kenney_campfire_pit`, `kenney_tent`, `kenney_workbench`, `kenney_resource_wood`,
  `kenney_resource_stone`, `kenney_tool_axe`, `kenney_tool_pickaxe` for camps and crafting.
- `kenney_row_boat_small`, `kenney_ship_wreck`, `kenney_pirate_flag`, `kenney_cannon`,
  `kenney_palm_straight`, `kenney_dock_platform` for coastal POIs.
- `kenney_block_grass_large`, `kenney_block_snow_large`, `kenney_coin_gold`, `kenney_heart`,
  `kenney_door_open` for terrain blocks and pickups.

## Audit Preview

`crates/client/src/pretty/audit_pretty.rs` now spawns a curated Kenney audit ring behind the
existing `audit-pretty-models` feature. The first pass includes animals, villagers, camp props,
pirate/coastal props, and pickups so scale and orientation can be judged in one screenshot before
any procedural gameplay visuals are replaced. All paths point at `kenney/curated/`.

## Gameplay Use

The normal client path now uses a first small batch of Kenney models loaded from
`kenney/curated/`:

- `kenney_campfire_pit`, `kenney_tent`, `kenney_workbench` follow near the player as camp props.
- `kenney_row_boat_small` follows near the player as a coastal-style prop.
- `kenney_coin_gold` and `kenney_heart` follow near the player as pickup-style props.

The old procedural ecology meshes are still present for now. Dynamic animal GLBs are intentionally
held back until the fixed landmarks are verified in screenshots.

Validate the preview after compile/run is allowed:

```sh
cargo run -p lk2-client --features audit-pretty-models -- --offline
```

Only promote assets into normal gameplay after they look correct in the audit ring.
