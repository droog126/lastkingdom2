---
name: bevy-resource-lifecycle
description: Diagnose and fix Bevy runtime asset, entity, render texture, GPU memory, and resource lifecycle bugs in lastkingdom2. Use when logs mention Out of Memory, Validation Error, invalid Texture, Queue::write_texture, Texture::create_view, wgpu resource errors, repeated terrain/render rebuilds, screenshots failing after movement, entity despawn without asset removal, or code creates Mesh/StandardMaterial/Image/ScatteringMedium/Texture resources during runtime systems.
---

# Bevy Resource Lifecycle

## Scope

Use with `$bevy-gameplay-dev` for client/render work and with `$closed-loop-ai-dev` when the fix needs screenshot or loop evidence.

This skill is for lifetime bugs, not ordinary visual tuning. The common pattern is: entities are despawned, but GPU-backed assets or render attachments remain allocated, or an expensive render feature is enabled without a bounded lifetime.

## First Reads

Before editing, inspect:

- `git status --short`
- the failing log tail and any full log file
- the system that creates runtime assets or render features
- the resource/component that remembers spawned entities
- the narrowest caller schedule in `crates/client/src/main.rs`

Search first with:

```sh
rg -n "Assets<|Handle<|meshes.add|materials.add|images.add|ScatteringMedium|TextureDescriptor|Image::|Screenshot|despawn|remove\\(" crates/client/src crates/core/src
rg -n "Out of Memory|Validation Error|write_texture|create_view|Texture with|wgpu" screenshots run-logs .harness/scratch
```

## Diagnosis Checklist

For each runtime allocation path, answer:

- What creates the asset or GPU resource?
- Is it startup-only or can it run repeatedly?
- What entity owns the handle?
- When that entity is despawned, does the asset store also drop or remove the generated asset?
- Are replacement builds atomic, so a failed rebuild keeps the previous visible state?
- Does validation exercise runtime movement/screenshot paths, not only type checking?

Treat these as leak suspects:

- `meshes.add(...)` or `materials.add(...)` inside `Update` systems
- `images.add(...)`, `Image::new_*`, or `TextureDescriptor` with unbounded dimensions
- screenshot capture loops with very short intervals
- repeatedly inserting TAA, SSAO, SSR, atmosphere, volumetric fog, or prepass components
- terrain/chunk rebuild systems that `despawn()` old entities but do not track generated handles

## Fix Pattern

Prefer explicit ownership:

1. Store generated handles next to spawned entity IDs in the owning resource.
2. Build replacement assets first.
3. If the rebuild fails or produces no mesh, keep the old visible entities and old assets.
4. After a successful replacement, despawn old entities and remove old generated assets from `Assets<T>`.
5. Do not remove shared handles loaded from `AssetServer` or handles owned by another system.

For terrain-like rebuilds, the owner resource should track both:

- `Vec<Entity>` for visual/collider entities
- `Vec<Handle<Mesh>>`, `Vec<Handle<StandardMaterial>>`, or other generated handles

Use `Assets::remove(&handle)` for Bevy 0.19 asset removal.

## Render Feature Guardrails

Keep expensive render features opt-in unless the task explicitly requires them:

- TAA can add history textures and prepass requirements.
- SSAO/SSR can add depth/normal/prepass attachments.
- Volumetric fog and atmosphere can allocate lookup or render resources.
- Screenshot systems allocate capture resources; validate interval and readiness gates.

If toggling components at runtime, only insert/remove when settings actually changed. Avoid per-frame reinsertion.

## Validation

Run, in order:

```sh
cargo fmt -p lk2-client
cargo check -p lk2-client
```

Then run the shortest runtime check that exercises the suspected leak path:

```sh
just xtask loop --offline --seconds 15
```

If the loop times out before a ready screenshot, still inspect logs:

```sh
rg -n "Out of Memory|Validation Error|write_texture|create_view|Texture with|wgpu" screenshots run-logs .harness/scratch
```

Do not claim the bug is fully fixed unless the runtime check either passes or runs long enough to exercise the failing path. If only a short run was possible, state that no matching GPU errors were observed in that run.

## Completion

Report:

- the owning resource or system changed
- the asset/resource lifetime rule added
- compile result
- runtime log result
- any remaining non-lifecycle failures, such as auto-demo timeout or unrelated log spam
