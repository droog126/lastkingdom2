# Nature simulation migration ledger

This ledger is temporary integration evidence for the role-1 refactor. Remove it after the old
entry points are fully migrated and the active architecture documentation records the result.

## reused

- `eco_cycle::EcoCycle::tick`: authoritative cloud, rain, plant, rabbit, wildlife, and resource
  transfer behavior.
- `eco_cycle::EcoCycle::{demo_at, seeded_weather_at}`: deterministic ecology initialization.
- `eco_cycle::EcoCycle::to_snapshot` and protocol `EcoSnapshot`: detailed replication state.
- `sim::{advance_demo_tick, advance_fixed_authority_tick}`: existing wall/fixed tick adapters.
- `GlobalResourcePool`: existing resource limits and transfers.

## moved

- No existing rule implementation was physically moved in this slice.

## adapted

- Both existing tick adapters now invoke `simulation::step_world` instead of calling
  `EcoCycle::tick` directly.
- `NatureSnapshot` wraps the existing detailed snapshot with atmosphere, available-water, and
  aggregate ecology summaries.
- `WorldRecipe` delegates to existing ecology constructors and adds only a deterministic seeded
  center offset.

## retired

- Nothing was retired. Deletion requires runtime call tracing and integration evidence.

## still_legacy

- `EcoCycle` still owns atmosphere, unconsumed rain, plants, and animals in one structure.
- Direct `EcoCycle::tick` calls remain in focused tests and diagnostics outside the authority
  adapters.
- `creature` and `monster` remain separate systems; merging them into the natural cycle needs
  explicit lifecycle semantics and regression tests.
- Terrain and world-content generation remain independent from `WorldRecipe`.
- `EcoSnapshot` remains the network contract; server/client adapters should consume
  `NatureSnapshot` incrementally without breaking compatibility.

## public_integration_needed

- Server role: expose or replicate `TickReport::snapshot` and `events` from the authority tick.
- Client role: map `NatureSnapshot` to presentation without creating semantic entities locally.
- Loop role: assert `NatureSnapshot::is_finite` and causal progress from machine-readable state.
- Later core work: decide whether soil moisture needs spatial state before renaming
  `available_water` to soil moisture.

