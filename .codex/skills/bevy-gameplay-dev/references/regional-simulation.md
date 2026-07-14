# Regional Simulation Reference

Use this checklist when splitting a shared simulation into spatial regions, adding LOD/catch-up, or migrating a server from one global authority to regional authority.

## Ownership

- Keep the rules in the shared core entry point (`step_world`/`step_world_elapsed`). Adapters coordinate regions; they do not create a second ecology implementation.
- Each region owns its ecology, resource pool, scheduler, and last simulated tick.
- If legacy Bevy systems still expose a global resource pool, treat it as a compatibility mirror: copy gameplay mutations into the region before stepping, then copy authoritative results back. Do not swap pools and leave the region holding a stale pre-step value.
- Keep the primary region identity explicit and stable. Do not select the first `BTreeMap` key as the primary merely because ordering is convenient.

## Scheduling and consumers

- Convert LOD to an explicit interval, preserve partial intervals, and bound catch-up work. Pass elapsed time through the shared step entry point.
- Merge multiple players' LOD requests by highest required frequency; never let the last player visited overwrite earlier requests.
- Order player movement/respawn/input -> region synchronization -> simulation -> snapshot/projection/observation/persistence. Add explicit `.before(...)`/`.after(...)` edges where separate system groups would otherwise run concurrently.
- Project snapshots from the player's region after simulation. Clients may present replicated state but must not create regional authority locally.

## Persistence

- Persist the complete restorable state: schema, region identity, snapshot, resource pool, LOD, and scheduler tick. Keep saved entries when a region unloads so re-entry can restore them.
- Validate schema, finite snapshot values, resource conservation, and `save.tick == snapshot.tick` before applying a save.
- JSON object maps require string keys; for structured region IDs use a versioned file DTO containing an array of `{ id, save }` records.
- Load saves before world/region initialization and write through a temp file plus backup/atomic commit. Restore all state before the first region sync.

## Evidence

- Add pure core tests for cadence, bounded catch-up, stable primary ownership, resource mirroring, and snapshot filtering; keep Bevy schedule checks at the server seam.
- Validate in batches with the narrowest affected package. If a build times out, inspect the process command line, avoid concurrent Cargo runs, and report implementation status separately from compile evidence. Never convert a timeout into a passing claim.
