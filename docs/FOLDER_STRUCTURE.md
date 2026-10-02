# Cosmogon — Folder Structure

> The complete repository layout. Every folder and file, explained.

This is the source of truth for where things live. If you're adding a new module, check here first. If you're moving something, update this file.

---

## Root

```
cosmogon/
├── Cargo.toml                          # Workspace root — defines all 7 crates, shared dependencies, workspace-level metadata
├── README.md                           # Project overview, quick start instructions, screenshots, and links to docs
├── LICENSE                             # MIT OR Apache-2.0 dual license — lets users pick whichever fits their use case
├── .gitignore                          # Ignore target/, *.swp, .DS_Store, generated assets, and IDE configs
```

---

## .github/

```
.github/
├── workflows/
│   ├── ci.yml                          # Runs on every push/PR: cargo test, clippy, rustfmt, build on all 3 OS
│   └── release.yml                     # Triggered on tags: builds release binaries, creates GitHub release with artifacts
├── ISSUE_TEMPLATE/
│   ├── bug_report.md                   # Bug report template with repro steps, OS, cargo version, screenshots
│   ├── feature_request.md              # Feature request template with use case, mockups, priority
│   └── performance.md                  # Performance issue template with profiling data, hardware specs, FPS numbers
└── PULL_REQUEST_TEMPLATE.md            # PR template: what changed, why, how tested, screenshots if visual
```

---

## docs/

```
docs/
├── brain.md                            # Living knowledge base — accumulates everything we learn as we build
├── ARCHITECTURE.md                     # Master architecture doc: crate relationships, data flow, render pipeline
├── PRD.md                              # Product requirements: what Cosmogon is, who it's for, what it does
├── ROADMAP.md                          # Multi-year versioned roadmap with concrete deliverables per release
├── FOLDER_STRUCTURE.md                 # This file — the repo layout with per-file descriptions
├── LEARNING_ROADMAP.md                 # Rust, wgpu, ECS learning path for contributors getting up to speed
├── FEATURE_BACKLOG.md                  # 300+ features ranked by priority, effort, and version target
├── RISK_REGISTER.md                    # Technical risks: performance cliffs, platform gotchas, dependency drama
├── DEVELOPMENT_WORKFLOW.md             # Git branching strategy, commit conventions, review process, CI details
├── adr/                                # Architecture Decision Records — why we chose what we chose
│   ├── 001-why-rust.md                 # ADR: Rust for safety, performance, and ecosystem — no GC pauses in the sim
│   ├── 002-why-wgpu.md                 # ADR: wgpu over Vulkan/Metal directly — cross-platform, WebGPU future
│   ├── 003-why-ecs.md                  # ADR: ECS for entity management — clean separation of data and systems
│   ├── 004-why-deterministic-simulation.md  # ADR: Deterministic sim for time reversal and "what if" scenarios
│   ├── 005-why-custom-physics.md       # ADR: Custom physics over Rapier — we need N-body gravity, not rigid bodies
│   └── 006-why-chunk-streaming.md      # ADR: Chunk streaming for universe-scale — can't fit everything in VRAM
├── design/                             # Future design documents — UI mockups, interaction patterns, visual specs
│   └── (TBD)
└── research/                           # Research notes — academic papers, algorithms, reference implementations
    └── (TBD)
```

---

## assets/

```
assets/
├── textures/                           # Planet and celestial body textures
│   ├── earth_day.jpg                   # Earth daytime diffuse map — blue oceans, green/brown landmasses
│   ├── mars.jpg                        # Mars surface texture — rust red, polar caps, cratered highlands
│   └── ...                             # Jupiter bands, Saturn rings, Moon surface, Mercury, Venus clouds
├── shaders/                            # WGSL shader files — the GPU programs that make things glow
│   ├── planet.wgsl                     # Planet rendering shader: texture sampling, lighting, basic PBR
│   ├── star.wgsl                       # Star shader: emissive glow, limb darkening, pulsation animation
│   ├── atmosphere.wgsl                 # Atmospheric scattering: Rayleigh + Mie, view-dependent haze
│   ├── skybox.wgsl                     # Background starfield rendering as a skybox cube map
│   ├── post_process.wgsl               # Post-processing: tone mapping, color grading, vignette
│   └── compute/                        # Compute shaders — GPU-accelerated number crunching
│       └── n_body.wgsl                 # N-body gravity compute shader — parallel force calculation on GPU
└── data/                               # Scientific and astronomical data files
    ├── planets.ron                     # Orbital elements for all planets in RON format — human-readable config
    ├── stars.csv                       # Star catalog (Hipparcos subset) — position, magnitude, color class
    └── spk/                            # SPICE kernels for high-precision ephemeris data (future use)
```

---

## crates/

The workspace is split into 7 crates. Each has a single responsibility. Dependencies flow one way: `app` depends on everything, `core` depends on nothing.

### cosmogon_core

```
crates/cosmogon_core/
├── Cargo.toml                          # No dependencies except math/glam — this crate is foundation only
└── src/
    ├── lib.rs                          # Crate root — re-exports public API, docs
    ├── constants.rs                    # Physical constants: G (6.674e-11), c, AU, solar mass, Planck's constant
    ├── coords.rs                       # Coordinate transforms: ecliptic ↔ equatorial ↔ galactic, rotation matrices
    ├── math.rs                         # f64 math utilities: clamp, lerp, smoothstep, angle wrapping, near-zero checks
    ├── units.rs                        # Unit conversions: AU↔meters, solar masses↔kg, lightyears↔parsecs
    └── precision.rs                    # f64→f32 conversion utilities — simulation runs f64, rendering uses f32
```

**Why it exists:** Shared math and constants that every other crate needs. No business logic, no rendering, just numbers.

### cosmogon_ecs

```
crates/cosmogon_ecs/
├── Cargo.toml                          # Depends on: bevy_ecs, cosmogon_core
└── src/
    ├── lib.rs                          # Crate root — world setup, system registration
    ├── components/
    │   ├── mod.rs                      # Component module re-exports
    │   ├── transform.rs               # Position (f64 3D), Rotation (quaternion), Scale — the "where is it" component
    │   ├── motion.rs                  # Velocity, Acceleration — Newtonian state for physics integration
    │   ├── orbital.rs                 # OrbitalElements: semi-major axis, eccentricity, inclination, longitude of ascending node
    │   ├── physics.rs                 # Mass, Radius, Density, EscapeVelocity — physical properties of a body
    │   ├── renderable.rs              # MeshHandle, MaterialId, Color — what the renderer needs to draw this entity
    │   ├── camera.rs                  # Camera (projection, FOV), CameraTarget (which body we're looking at)
    │   ├── celestial.rs               # CelestialBody, Star, Planet, Moon, AsteroidBelt — classification tags
    │   └── time.rs                    # TimeState (simulation time), TimeAcceleration (1x, 100x, 1000x)
    ├── resources/
    │   ├── mod.rs                     # Resource module re-exports
    │   ├── global_time.rs             # GlobalTime resource — master clock, pause state, time step
    │   ├── render_state.rs            # RenderConfig (HDR, bloom toggle), FrameStats (FPS, draw calls, triangles)
    │   └── input_state.rs             # InputState — mouse position, key states, scroll delta, mouse drag vector
    └── sets.rs                        # SystemSet definitions — ordering constraints: physics before render, input before physics
```

**Why it exists:** All ECS components and systems in one place. Other crates define what data exists; this crate defines what data looks like.

### cosmogon_physics

```
crates/cosmogon_physics/
├── Cargo.toml                          # Depends on: cosmogon_core, cosmogon_ecs, rayon (parallel iteration)
└── src/
    ├── lib.rs                          # Crate root — public API
    ├── gravity.rs                      # N-body gravitational force: F = Gm₁m₂/r², pairwise calculation, softening parameter
    ├── kepler.rs                       # Keplerian orbital propagation: solve Kepler's equation, true anomaly from mean anomaly
    ├── integrators/
    │   ├── mod.rs                      # Integrator trait + factory: choose RK4, Verlet, or adaptive
    │   ├── rk4.rs                     # Runge-Kutta 4th order — gold standard for orbital mechanics accuracy
    │   └── verlet.rs                  # Velocity Verlet — symplectic, energy-conserving, good for large step sizes
    ├── orbital_elements.rs             # Convert between Cartesian state vectors and Keplerian orbital elements
    ├── lagrange.rs                     # Calculate L1-L5 Lagrange points for any two-body system
    └── systems.rs                     # ECS systems: GravitySystem, OrbitalPropagationSystem, IntegrationSystem
```

**Why it exists:** All physics lives here. Gravity, orbits, integrators — the math engine that makes the universe tick.

### cosmogon_render

```
crates/cosmogon_render/
├── Cargo.toml                          # Depends on: cosmogon_core, cosmogon_ecs, wgpu, winit, pollster, bytemuck
└── src/
    ├── lib.rs                          # Crate root — public API
    ├── device.rs                       # wgpu Device + Queue initialization — request adapter, create device with features
    ├── pipeline.rs                     # Render pipeline creation: vertex layouts, bind groups, depth testing, blending
    ├── mesh.rs                         # Mesh generation: UV sphere, icosphere, billboard quad, ring (for Saturn)
    ├── material.rs                     # Material definitions: base color, metallic, roughness, emissive, texture slots
    ├── camera.rs                       # Camera uniform buffer: view-projection matrix, position, near/far planes
    ├── lighting.rs                     # Light uniform buffer: sun position, intensity, color temperature
    ├── renderer.rs                     # Main render loop: begin frame, render pass, end frame, present
    ├── hdr.rs                          # HDR rendering + tone mapping: ACES, Reinhard, or filmic curves
    ├── bloom.rs                        # Bloom post-process: Gaussian blur on bright areas, additive blend
    ├── atmosphere.rs                   # Atmospheric scattering: Rayleigh + Mie, view ray integration
    └── systems.rs                     # ECS render systems: SyncCameraSystem, SubmitDrawSystem, PostProcessSystem
```

**Why it exists:** Everything GPU-related. If it touches the screen, it goes through this crate.

### cosmogon_scene

```
crates/cosmogon_scene/
├── Cargo.toml                          # Depends on: cosmogon_core, cosmogon_ecs, serde, ron
└── src/
    ├── lib.rs                          # Crate root — public API
    ├── solar_system.rs                 # Hardcoded solar system data: planet positions, masses, orbital elements for v0.2
    ├── celestial_body.rs               # CelestialBody definitions: struct for creating new bodies with all components
    ├── procedural.rs                   # Procedural generation: terrain, star systems, galaxy structures
    ├── star_catalog.rs                 # Load and query star catalog data — Hipparcos, spectral class, distance
    └── loader.rs                      # Scene loading infrastructure: future file format for saving/loading simulations
```

**Why it exists:** Where the universe's data comes from. Hardcoded solar system data now, procedural generation and file loading later.

### cosmogon_ui

```
crates/cosmogon_ui/
├── Cargo.toml                          # Depends on: cosmogon_core, cosmogon_ecs, egui, eframe
└── src/
    ├── lib.rs                          # Crate root — egui context setup, panel registration
    ├── inspector.rs                    # Object inspector panel: click a body, see its properties, edit parameters
    ├── time_controls.rs               # Time control panel: play/pause, speed slider, date display, set date
    ├── camera_panel.rs                # Camera settings: FOV, follow mode, look-at target, navigation speed
    ├── debug_overlay.rs               # Debug info overlay: FPS, draw calls, triangle count, active bodies, memory
    └── systems.rs                     # ECS UI systems: SyncUIState, HandleUIInput, UpdateInspector
```

**Why it exists:** All egui panels and UI logic. Separated from rendering because UI is CPU-side and rendering is GPU-side.

### cosmogon_app

```
crates/cosmogon_app/
├── Cargo.toml                          # Depends on: ALL other crates — this is the binary, it wires everything together
└── src/
    ├── main.rs                         # Entry point: initialize logging, create window, run event loop
    ├── app.rs                          # App struct: owns World, manages frame lifecycle, coordinates systems
    ├── input.rs                        # Input handling: map winit events to InputState, keyboard shortcuts, mouse dragging
    ├── window.rs                       # Window configuration: title, size, fullscreen, vsync, MSAA samples
    └── config.rs                       # App configuration: startup settings, saved preferences, CLI args
```

**Why it exists:** The top-level binary. It's the glue — creates the ECS world, registers all systems, starts the event loop, and hands off to the render loop.

---

## tests/

```
tests/
├── physics_tests.rs                    # Physics unit tests: gravity calculations, orbital element conversions, energy conservation
├── orbital_tests.rs                    # Orbital mechanics tests: Kepler equation solver, Lagrange points, transfer orbits
└── rendering_tests.rs                 # Rendering integration tests: mesh generation, pipeline creation, buffer sizes
```

**Why it exists:** Integration tests that verify the crates work together correctly. Run with `cargo test`.

---

## benches/

```
benches/
├── gravity_bench.rs                    # Benchmark: N-body force calculation — how many bodies before we drop below 60fps
├── integrator_bench.rs                # Benchmark: RK4 vs Verlet step time at different body counts and step sizes
└── mesh_gen_bench.rs                  # Benchmark: UV sphere and icosphere generation — subdivision levels vs vertex count
```

**Why it exists:** Criterion benchmarks. We track performance over time to catch regressions before they ship.

---

## tools/

```
tools/
├── generate_planets.rs                 # CLI tool: generate planets.ron from raw data, update orbital elements from JPL ephemeris
└── benchmark_runner.rs                # CLI tool: run all benchmarks, generate markdown report, track trends
```

**Why it exists:** Developer utilities that aren't part of the main app. Run with `cargo run --bin generate_planets`.

---

## Dependency Graph

```
cosmogon_app
├── cosmogon_render
│   ├── cosmogon_core
│   └── cosmogon_ecs
│       └── cosmogon_core
├── cosmogon_scene
│   ├── cosmogon_core
│   └── cosmogon_ecs
├── cosmogon_physics
│   ├── cosmogon_core
│   └── cosmogon_ecs
├── cosmogon_ui
│   ├── cosmogon_core
│   └── cosmogon_ecs
├── cosmogon_ecs
└── cosmogon_core
```

**Rule:** Dependencies flow downward only. `cosmogon_core` has zero internal dependencies. `cosmogon_app` depends on everything. No circular deps. No cross-crate dependencies between render, physics, scene, or ui — they only talk through ECS.
