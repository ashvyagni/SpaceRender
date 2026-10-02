# Prototype audit (Phase 0)

*Audit of the repository as found on 2026-10-03 (baseline commit `3b764f0`).*

## 1. System architecture as found

A Rust workspace of eight crates, ~8,300 lines of Rust and ~6,700 lines of planning documents.
There were **two independent front-ends** that shared almost nothing:

| Crate | Lines | What it was |
|---|---:|---|
| `cosmogon_core` | ~1,240 | f64 `Vec3d`/`Quatd`/`Mat4d`, constants, units, coordinate frames. Tested. |
| `cosmogon_physics` | ~1,350 | Kepler solver, state vectors ⇄ elements, Hohmann, Lagrange points, N-body gravity, RK4/Verlet integrators, plus `bevy_ecs` 0.15 systems. Tested. |
| `cosmogon_scene` | ~960 | Real Solar System data (JPL-style J2000 elements, 16 bodies), a 51-star catalogue, value noise. |
| `cosmogon_ecs` | ~520 | `bevy_ecs` 0.15 components/resources. |
| `cosmogon_render` | ~2,200 | Hand-written wgpu 24 renderer: planet/star/atmosphere WGSL, HDR + bloom modules, loading screen. |
| `cosmogon_ui` | ~360 | egui 0.30 panels. |
| `cosmogon_app` | ~500 | winit event loop driving the above. |
| `cosmogon_bevy` | ~460 | A second, newer front-end on **Bevy 0.18 + `big_space`**: 8 planets on circular toy orbits, a fake loading bar. |

## 2. What worked
* Both front-ends compiled; 60 unit tests passed (core math and orbital mechanics).
* `cosmogon_core` and `cosmogon_physics` were correct, documented and tested.
* `cosmogon_scene` held valuable real astronomical data.

## 3. What was incomplete or broken
* **wgpu app — nothing visible but the Sun.** Scene positions were in metres but `build_transforms`
  multiplied them by an "AU" scale of 100, placing planets ~1.5×10¹³ units away — beyond the 50,000-unit far plane.
* **Time acceleration applied twice** (`delta = dt·accel`, then `integrate_motion` multiplies by `accel` again).
* **Every moon orbited the Sun's μ** (`GravityConfig::central_mass` used for all bodies).
* **N-body integration ran on top of the analytic Kepler propagator** on the same entities, fighting it.
* HDR, bloom, atmosphere and the egui panels existed as modules but were **never wired into the frame**.
* The Bevy app had circular orbits with a fixed 100-units-per-AU scale and no use of the real data.
* No life, habitability, civilization, technology, history, save or scheduler code existed at all.

## 4. Technical debt
* Two renderers, two ECS versions (bevy_ecs 0.15 standalone + Bevy 0.18) compiled side by side.
* Physics crate coupled to an ECS, so it could not be used headless.
* `Cargo.lock` was git-ignored (wrong for an application); the project was not under version control.
* Planning documents (PRD, 2,900-line architecture doc, 300-item backlog) described features far beyond the code.

## 5. Rendering assessment
The custom renderer was a reasonable learning exercise but ~2,000 lines away from parity with a
modern engine (no materials system, no post-processing in use, no shadows, no TAA, no UI integration,
no asset pipeline, no LOD). Bevy 0.18 already provides HDR, physically based materials, bloom,
tonemapping, MSAA/TAA/SMAA, shadows, volumetrics, compute, a render graph and cross-platform
backends — on the same wgpu the prototype chose deliberately (ADR 002).

## 6. Simulation assessment
The orbital mathematics was sound and was kept. Everything that makes the project unique — living
planets — was absent. There was no scheduler, so nothing supported running different systems at
different rates, and no determinism strategy beyond an ADR.

## 7. Build & distribution assessment
`cargo run` only. No release profile tuning, no app bundle, no icon, no installer, no CI. The Bevy crate
enabled `bevy/dynamic_linking` by default, which produces binaries that cannot be distributed.

## 8. Recommended long-term architecture
An **engine-independent, deterministic simulation core** (`cosmogon_sim`) that the app, a headless CLI
and the tests all drive through one API, and a **Bevy application** that only *views* it. See
[ARCHITECTURE.md](../ARCHITECTURE.md).

## 9. Should the stack remain?
Rust: yes. wgpu: yes (via Bevy). Hand-written renderer: no. `big_space`: replaced by our own f64
camera-relative transform pass (fewer moving parts, the simulation already owns f64 positions).
Details and the alternatives considered (Unreal, Godot, staying custom) are in
[ENGINE_ASSESSMENT.md](ENGINE_ASSESSMENT.md).

## 10. Roadmap
See [ROADMAP.md](../ROADMAP.md).

## 11. First implementation milestone
**M1 — "Living skeleton"**: a thin but complete vertical slice through every layer, shipped as a
double-clickable macOS app — deterministic universe generation, habitability, life, a civilization
with a causal technology graph, history, saves, a usable 3D view with UI, and packaging. Delivered;
see [CHANGELOG.md](../CHANGELOG.md).
