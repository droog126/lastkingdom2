# Kenney Asset Ingest

The five imported Kenney packs are usable directly through Bevy's `AssetServer` because each
pack includes GLB files.

## Inventory

| Pack | GLBs | Best Use |
| --- | ---: | --- |
| `kenney_cube-pets_1.0` | 24 | passive animals, biome wildlife |
| `kenney_mini-characters` | 26 | villagers, NPCs, player avatar experiments |
| `kenney_pirate-kit` | 72 | coastal POIs, boats, docks, pirate props |
| `kenney_platformer-kit` | 153 | terrain chunks, pickups, doors, fences |
| `kenney_survival-kit` | 80 | camps, tools, resource drops, workstations |

Total: 355 GLBs.

## License

Each pack includes `License.txt` and declares Creative Commons Zero (CC0). Credit to
Kenney / `www.kenney.nl` is appreciated but not required.

## Runtime Strategy

Use `assets/kenney/MANIFEST.json` as the project-facing index. Keep the downloaded source
packs in place and load GLBs by repo-relative asset paths, for example:

```rust
asset_server.load("kenney_cube-pets_1.0/Models/GLB format/animal-bunny.glb#Scene0")
```

Do not wire all assets at once. Start with curated IDs from the manifest:

- `kenney_bunny`, `kenney_deer`, `kenney_cow` for passive animal tests.
- `kenney_villager_male_a`, `kenney_villager_female_a` for settlement population.
- `kenney_campfire_pit`, `kenney_tent`, `kenney_workbench` for camps.
- `kenney_row_boat_small`, `kenney_ship_wreck`, `kenney_pirate_flag` for coastal POIs.
- `kenney_coin_gold`, `kenney_heart` for pickups.

## Audit Preview

`crates/client/src/pretty/audit_pretty.rs` now spawns a curated Kenney audit ring behind the
existing `audit-pretty-models` feature. The first pass includes animals, villagers, camp props,
pirate/coastal props, and pickups so scale and orientation can be judged in one screenshot before
any procedural gameplay visuals are replaced.

## Gameplay Use

The normal client path now uses a first small batch of Kenney models:

- `animal-bunny.glb` follows each ecology rabbit as an additional visible model.
- `campfire-pit.glb`, `tent.glb`, and `workbench.glb` appear near the player spawn as camp props.
- `boat-row-small.glb` appears as a coastal-style prop near the spawn area.
- `coin-gold.glb` and `heart.glb` appear as pickup-style props near the spawn area.

The old procedural ecology meshes are still present for now. This keeps existing readability while
the Kenney scale and orientation are verified in screenshots.

Validate the preview after compile/run is allowed:

```sh
cargo run -p lk2-client --features audit-pretty-models -- --offline
```

Only promote assets into normal gameplay after they look correct in the audit ring.
