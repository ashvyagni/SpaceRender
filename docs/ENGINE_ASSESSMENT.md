# Engine assessment

**Decision: consolidate on Bevy 0.18 (wgpu) with an engine-independent simulation crate.
Retire the hand-written wgpu renderer. Do not migrate to another language or engine.**

This is a consolidation, not a migration: the prototype had already started a Bevy front-end, and
Bevy renders through the same wgpu library chosen in the prototype's ADR 002.

## Requirements vs options

| Requirement | Custom wgpu (as found) | **Bevy 0.18** | Unreal 5 | Godot 4 |
|---|---|---|---|---|
| AAA-quality rendering | Everything must be built: PBR, shadows, TAA, post, volumetrics | PBR, HDR, bloom, AgX/TonyMcMapface, TAA/SMAA/FXAA, cascaded shadows, volumetric fog, SSAO, atmosphere, compute; custom WGSL materials | Best in class (Lumen, Nanite) | Good; Vulkan/Forward+ |
| Planetary LOD | Build it | Build it (custom meshes + compute) — same for every engine at planetary scale | Possible, but Nanite/landscape are not spherical-planet systems | Build it |
| Large coordinate spaces | Own it | Own it (f64 camera-relative, done in M1) | LWC (f64) since 5.0 | Single-precision build required |
| Desktop distribution | Manual | Single static binary; we package .app/.dmg/.exe | Mature but heavy (GBs) | Good |
| Multithreading | Manual | Parallel ECS scheduler, task pools | Yes | Partial |
| Cross-platform / GPU APIs | Metal/Vulkan/DX12 via wgpu | Same (wgpu) | All | All |
| Procedural rendering | Full control | Full control (custom materials, render graph) | Harder to bypass the content pipeline | OK |
| Simulation in the same language | Yes | **Yes — the sim is plain Rust, no engine types** | C++ rewrite of everything | GDScript/C# or FFI |
| Development complexity for 1–3 people | Very high | Moderate | High; huge editor/content workflow | Moderate |

## Why not stay custom
The custom renderer would need years to reach what Bevy provides today, and none of that work is
the project's differentiator. The differentiator is the simulation.

## Why not Unreal
Unreal's rendering is superior, but the simulation would have to be rewritten in C++ (or bridged
through FFI), builds are measured in gigabytes, and the editor-centric content pipeline fights a
fully procedural universe. Its large-world coordinates solve a problem we solved in ~50 lines.

## Why not Godot
A reasonable choice, but it offers less rendering headroom than Bevy for compute-heavy procedural
work, needs a double-precision engine build for astronomical scales, and would still require
FFI to keep the simulation in Rust.

## Risks of Bevy and mitigations
* **API churn** (a breaking release roughly every 3–4 months). *Mitigation:* the simulation crate has
  no Bevy dependency at all; only `crates/cosmogon` must be ported. Upgrade deliberately, one minor
  version at a time; Bevy 0.19 was skipped in M1 because `big_space`/`bevy_egui` compatibility mattered more than novelty at that point (we have since dropped `big_space`).
* **Missing high-end features** (no GPU-driven virtual geometry or ray-traced GI yet). *Mitigation:*
  the render graph accepts custom passes; planetary terrain must be custom in any engine.
* **Metal: no bindless textures yet** (egui warns at startup). Harmless for current scenes.

## What was kept from the prototype
`cosmogon_core` (math, constants, units), `cosmogon_physics` (now ECS-free), the Solar System data
(now data-driven TOML), the idea of a loading/showcase screen, and the deterministic-simulation ADR.

## Precision solution
`big_space` was dropped in favour of an explicit pass: the simulation stores positions in `f64` metres;
each frame the camera's `f64` position becomes the origin and every entity's `Transform` is written as
`(world − camera)` cast to `f32`. Near-camera precision is sub-millimetre at any location; Bevy's
infinite reverse-Z projection handles depth from 0.5 m to 10²² m. See [RENDERING.md](../RENDERING.md).
