# Sandbox phase — audit, gap analysis and blockers

Written at the start of the *universe sandbox* phase (October 2026), against the completed
base tagged `base-0.3.0` (= release v0.3.0 + README screenshots).

## 1. Base confirmed healthy

| Check | Result |
|---|---|
| Release workflow v0.3.0 (macOS universal DMG, Windows exe/installer/zip, Linux tarball, draft release) | ✅ completed successfully |
| Test suite (`cargo test --workspace --release`) | ✅ 103 passed, 0 failed |
| Cross-platform determinism (golden fingerprint on macOS ARM, Windows x64, Linux x64 CI) | ✅ |
| App launch: menu, Sol scenario, close-up terrain | ✅ (captures in the session log) |
| Git checkpoint | tag `base-0.3.0`; sandbox work on branch `sandbox-foundation` |

## 2. Feature set of the base (recorded before changes)

**Simulation (`cosmogon_sim`)** — deterministic, data-driven, f64, headless.
- Time from real time to 100 Myr/s on a fixed-period scheduler with a CPU budget.
- Stars: mass–luminosity/radius relations, brightening, giant and white-dwarf phases.
- Orbits: analytic Kepler for every body (pure functions of time); S-type binaries.
- Real Solar System: 8 planets + 7 major moons from hand-entered J2000 elements and NASA
  fact-sheet physical data; Earth relief from NOAA ETOPO5.
- Procedural neighbourhoods (IMF, frost lines, moons, belts, rings).
- Climate: grey greenhouse, carbonate–silicate cycle, water phase, glacial cycles.
- Life: stage hazards, oxygenation, fossil fuels, extinctions.
- Civilizations: population, research over 13 domains, causal tech graph (56 techs),
  settlements, rival nations (war, conquest, union, secession), space age, radio contact,
  civilization LOD.
- Versioned JSON saves with migrations hook; golden fingerprint test.

**App (`cosmogon`)** — Bevy 0.18 + egui.
- Main menu (new universe / continue / load / settings / quit), loading screen.
- f64 camera-relative rendering, astronomical camera (metres to light-years), close-up
  cube-sphere terrain, baked planet surfaces, atmospheres, rings, night lights, markers.
- Read-only inspector (overview, orbit, environment, life, civilization, nations, tech).
- Chronicle, toasts, autosave, graphics presets, capture tool.

## 3. Engine capability assessment (extend, don't rewrite)

| Requirement | Current stack | Verdict |
|---|---|---|
| N-body simulation | Rust f64, rayon available | ✅ Implemented in `cosmogon_physics::nbody`; ~1 µs per step for the Sol active set (measured in tests/benchmarks) |
| Large coordinate ranges | f64 sim, f64 camera, camera-relative f32 render transforms | ✅ already in place |
| Custom shaders | WGSL materials (planet, atmosphere, terrain) | ✅ lensing, comet tails, accretion disks fit the same material path |
| Planetary LOD | Cube-sphere quadtree with async patch builds | ✅ |
| Catalogue streaming | Bevy async task pools, plain-file IO | ✅ (to build: tile index + GPU point buffers) |
| Multithreading | rayon, Bevy task pools; sim currently on main thread | 🟡 sim worker thread with snapshots is a planned change, not a rewrite |
| Desktop packaging | .app/.dmg, .exe/installer, Linux tarball, CI release | ✅ |
| Advanced UI | egui 0.33 via bevy_egui | 🟡 capable (custom painting, multi-window via bevy_egui, docking via `egui_dock`); re-evaluate at the Premium UI milestone |
| macOS / Windows | Metal / DX12-Vulkan via wgpu | ✅ |
| GPU acceleration | wgpu compute | ✅ available when measurements justify it |

No subsystem is fundamentally limiting. **No rewrite or engine migration is justified.**
The weakest part for the product vision is UI polish, which is a design task within egui.

## 4. Gap analysis against the sandbox specification

✅ exists · 🟡 partial · ⬜ missing · → milestone that closes it (S1 = Sandbox Foundation, …)

| Area | Status | Notes |
|---|---|---|
| Home screen (New Sandbox, Continue, Load, Real Universe, Scenarios, Object Lab, Settings, Credits) | 🟡 | Main menu exists with fewer entries → S1 |
| Cinematic intro / loading | 🟡 | Loading screen exists → S1 intro |
| Sandbox documents (name, description, dates, thumbnail, seed, data versions, physics settings, enabled systems) | ⬜ | Saves are single files with a small header → S1 |
| Save / Save As / Duplicate / Rename / Delete / Autosave / Checkpoints | 🟡 | Save + autosave only → S1 |
| Branches | ⬜ | Format designed for it in S1; UI later |
| Real-data layer (download → normalise → validate → cache → version) | ⬜ | Hand-entered values → S1 starts with JPL Horizons; S4/S5 Gaia, exoplanets, small bodies |
| Provenance (measured / derived / estimated / procedural / user-modified) | 🟡 | One `real` flag per body → S1 per-quantity provenance |
| Universal object taxonomy | 🟡 | 4 body kinds + stars → S1 taxonomy & schema; categories implemented incrementally |
| Object creator | ⬜ | → S1 (planets, moons, asteroids, comet nuclei) |
| Editable inspector with units | ⬜ | Read-only → S1 |
| Direct manipulation (move, velocity, impulse, duplicate, delete, launch) | ⬜ | → S1 |
| Trajectory prediction | ⬜ | → S1 |
| N-body gravity | ⬜ | Integrators existed but were unused → S1 |
| Hybrid orbit domains (catalogue / Kepler / N-body / encounter) | 🟡 | Kepler only → S1 Kepler + N-body + encounter substeps |
| Collisions | ⬜ | → S1 detection + merger + impact record; S3 fragmentation/cratering regimes |
| Conservation & validation tests | 🟡 | Kepler tests only → S1 |
| Orbit overlays (trail, prediction, vectors, apsides, nodes, Hill/SOI, Lagrange) | 🟡 | Kepler orbit lines → S1 osculating orbits, trails, prediction, velocity; S3 the rest |
| Tides, Roche, relativity | ⬜ | → S1 1PN option; S3 tides/Roche |
| Stellar evolution with consequences | 🟡 | Coarse model exists → S6/S9 |
| Event pipeline physics → climate → life → civilization | 🟡 | Civilizations already respond to climate; climate is not driven by orbital state → S1 begins, S2 completes |
| Time control respecting stability | ✅ | Scheduler never enlarges steps; N-body speed is CPU-bound by design |
| Rewind / replay | ⬜ | Checkpoints in S1; journal + deterministic replay later |
| Physics presets | ⬜ | → S1 |
| Seamless scale galaxy → surface | 🟡 | Star neighbourhood → surface works; galaxy layer S5 |
| Graphics presets that never change physics | ✅ | |
| Undo / redo | ⬜ | → S1 |
| Command palette, search | 🟡 | Name search exists → S1 basic palette; S7 full |
| Object comparison, blueprints, modding, accounts | ⬜ | Later milestones; local profile directory prepared in S1 |

## 5. Blockers found in the code (and how S1 removes them)

1. **Positions are pure functions of time.** `StarSystem::body_local_position(t)` evaluates
   Kepler orbits; there is no place for state. → Optional per-system `Dynamics` state; the
   query API is unchanged so the renderer, camera and UI keep working.
2. **Bodies are addressed by vector index** (`BodyRef`, `Body.parent`, civilization and
   biosphere back-references). Removing a body from the vector would corrupt references. →
   Bodies are never removed; they get a `removed` record (merged / deleted / destroyed) and
   every iterator skips them. Undo restores them for free.
3. **Visuals are spawned once** on entering the universe. → Incremental visual sync
   (spawn new bodies, hide removed ones, rebuild edited ones).
4. **Saves are monolithic files** and listing them reads every file. → Sandbox directories
   with a small manifest, thumbnail, autosave and checkpoints.
5. **Two inconsistent constant sets** (`cosmogon_core::constants` AU = 1.496e11, M☉ =
   1.989e30; `cosmogon_sim::astro` AU = 1.495978707e11, M☉ = 1.98892e30). A 2×10⁻⁴ error in
   GM☉ shifts Earth by degrees per century in an N-body run. → One set (IAU 2015 nominal +
   CODATA 2018), sim re-exports core.
6. **`state_vectors_to_elements` bug**: ascending node used swapped `atan2` arguments and
   equatorial orbits lost their periapsis. → Rewritten with round-trip tests.
7. **Climate refresh cadence is geological** (biospheres every 1 Myr; civilization worlds
   every 10 yr) and uses fixed semi-major axes. → Insolation from osculating orbits and
   event-driven refresh when it changes.
8. **The inspector is read-only by design** ("UI never mutates the model"). → Kept as a
   principle: the UI issues `Edit` commands; the simulation validates, applies, journals
   and propagates them.

## 6. Decisions

- Stack unchanged (Rust, Bevy 0.18, egui, wgpu).
- N-body integrator written in-house (see [PHYSICS_ENGINE.md](PHYSICS_ENGINE.md)); REBOUND
  (GPL-3.0, C) was evaluated and rejected for licence and cross-compilation reasons.
- Real data enters through offline pipelines (`tools/`) into versioned caches shipped with
  the app; no network access at run time.
- Existing scenarios keep Kepler propagation by default; a system becomes dynamic when the
  user selects N-body or edits it. Old saves load unchanged.
