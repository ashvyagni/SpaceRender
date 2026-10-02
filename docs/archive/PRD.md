# Cosmogon — Product Requirements Document

**Version:** 1.0
**Last Updated:** 2026-07-20
**Author:** flamexmystix
**Status:** Active Development

---

> "I wanted to build a universe, not a game. Something where you press play and watch
> celestial mechanics unfold like they actually do — no shortcuts, no scripting, just
> math and physics and a whole lot of floating point precision."

---

## Table of Contents

1. [Vision & Inspiration](#1-vision--inspiration)
2. [Core Philosophy](#2-core-philosophy)
3. [Scope](#3-scope)
4. [User Stories](#4-user-stories)
5. [Technical Requirements](#5-technical-requirements)
6. [Milestones](#6-milestones)
7. [Success Metrics](#7-success-metrics)
8. [Risks & Mitigations](#8-risks--mitigations)
9. [Constraints](#9-constraints)
10. [Assumptions](#10-assumptions)

---

## 1. Vision & Inspiration

### What Is Cosmogon?

Cosmogon is a **real-time digital universe simulator**. Not a game. Not a toy.
A serious piece of software that models our solar system — and eventually, a
full cosmos — using real physics, real orbital mechanics, and real rendering
techniques. You sit in the driver's seat of a virtual camera and fly through a
universe that obeys the laws of physics (or your modified version of them).

Think of it as: what if someone built a universe in a box, and you could watch
it run?

### The Inspiration Trifecta

This project stands on the shoulders of three giants:

**Space Engine** — The gold standard for procedural universe exploration.
Cosmogon draws inspiration from its seamless scale transitions (surface to
interstellar in one smooth camera move) and its reverence for scientific
accuracy. Space Engine proves that scientific visualization can be jaw-dropping
beautiful.

**Kerbal Space Program** — KSP taught the world that orbital mechanics can be
fun. The way it makes Keplerian orbits *feel* tangible — the maneuver nodes,
the transfer windows, the "oh no I'm going to crash into the sun" moments —
that tangibility is something Cosmogon wants to capture, but in a pure
simulation context rather than a game.

**Universe Sandbox** — The "what if" machine. What if the Moon were twice as
big? What if Jupiter were a star? Universe Sandbox showed that letting users
poke at universal constants and watch the consequences is incredibly compelling.
Cosmogon wants to bring that same experimental spirit, but with a more rigorous
technical foundation.

### The Goal

The north star for this project is a single reaction:

> **"How did one person even build this?"**

Every decision, every feature, every line of code should contribute to that
moment of disbelief when someone sees Cosmogon running for the first time. The
kind of project where the demo speaks for itself and the source code is its own
documentation.

---

## 2. Core Philosophy

These aren't just feel-good bullet points. They're the engineering principles
that guide every design decision. If a feature violates these, it doesn't ship.

### Simulate, Don't Script

The solar system doesn't run on hardcoded animation curves. Planets don't
follow paths someone drew in Blender. Everything emerges from gravitational
calculations, initial conditions, and time stepping. If a planet's orbit looks
slightly elliptical, it's because its orbital eccentricity is set to the real
value, not because someone added a sine wave to a position.

This means:
- Orbital mechanics come from solving Kepler's equation, not from keyframe
  animation
- Atmospheric scattering comes from physically-based Rayleigh and Mie theory,
  not from hand-tuned gradient maps
- Terrain comes from procedural noise guided by geological constraints, not
  from heightmaps someone painted

### Emergent Behavior

The coolest things in Cosmogon should be things we never explicitly programmed.
If the orbital resonance between Jupiter and Saturn creates visible gaps in an
asteroid belt — that's emergent. If a planet's axial tilt causes seasons that
affect atmospheric color — that's emergent. We build the rules; the universe
provides the spectacle.

### Scientific Accuracy (Where Practical)

We aim for accuracy, not pedantry. The goal is:
- Orbital periods within 1% of real values for all major bodies
- Correct relative sizes and distances (with optional log-scale compression for
  visualization)
- Real spectral classes and colors for stars
- Physically-based atmospheric models

We won't model quantum effects, dark matter distributions, or relativistic
frame-dragging (until much later milestones, at least). The principle is: if a
user who knows astronomy looks at Cosmogon, they should nod approvingably
instead of wincing.

### Beautiful Visuals + High Performance

Scientific accuracy means nothing if the result looks like a spreadsheet.
Cosmogon should be *pretty*. PBR materials on planets. Volumetric atmospheres.
Bloom on stars. Motion blur on fast-moving objects. And it should hit 60fps
while doing it.

Performance is not an afterthought. It's a first-class requirement. The rendering
pipeline, the ECS architecture, the data layout — everything is designed with
cache coherence and GPU utilization in mind.

### Every Feature Emerges from Simulation

This is the hardest principle to follow, but the most important. If someone
says "add clouds to Earth," the answer isn't to paint cloud textures on a
sphere. The answer is to model atmospheric moisture, temperature gradients, and
wind patterns, then render the result. The feature is a *consequence* of the
simulation, not an addition to it.

Obviously, this has limits at every milestone. We can't simulate full
atmospheric fluid dynamics in v0.1. But the *architecture* should always
support the path toward that simulation, even if the early versions use
simplified approximations.

---

## 3. Scope

### In Scope — MVP (v0.1: "First Light")

The MVP is the minimum viable proof that this project is real and worth
continuing. It should make someone go "whoa" within 30 seconds of launching.

#### Solar System Bodies
- **The Sun** — a glowing sphere with a procedural solar surface texture
  (granulation patterns), proper spectral color (G2V), and bloom/glow
- **8 Planets** — Mercury, Venus, Earth, Mars, Jupiter, Saturn, Uranus,
  Neptune — with correct relative sizes, orbital radii, and colors
- **Major Moons** — Earth's Moon, Jupiter's Galilean moons, Titan
- **Asteroid Belt** — a particle system between Mars and Jupiter representing
  ~1000 visible asteroids

#### Orbital Mechanics
- Keplerian orbital mechanics using real orbital elements (semi-major axis,
  eccentricity, inclination, longitude of ascending node, argument of
  perihelion, mean anomaly)
- Kepler's equation solved iteratively for each body each frame
- Correct orbital periods matching real values
- Planets orbit in the correct plane (ecliptic) with correct inclinations

#### Rendering
- PBR (Physically Based Rendering) for all celestial body surfaces
- Procedural terrain generation for Earth, Mars, and Moon using layered
  Perlin/simplex noise with geological constraints
- Atmospheric scattering using Rayleigh + Mie theory for Earth and Venus
- Earth's atmosphere: blue sky from Rayleigh scattering, sunset colors from
  Mie scattering, proper extinction at the horizon
- Venus's atmosphere: thick, yellowish, with the characteristic limb darkening
- Starfield background using real star catalog data with spectral colors
- Basic bloom effect on bright objects (Sun, stars)
- HDR tone mapping (ACES or Reinhard) for natural exposure

#### Time System
- Time acceleration from 1x (real-time) to 10,000x (watch planets orbit)
- Pause/resume
- Variable time step with sub-step accumulation for stability at high time
  scales

#### Camera System
- **Fly-through camera** — WASD + mouse, free movement through space
- **Orbit camera** — focus on any body, orbit around it with mouse drag
- **Planet surface camera** — land on a body, look around from the surface
- Smooth transitions between camera modes
- Speed scaling based on proximity (slow near surfaces, fast in deep space)

#### UI (egui panels)
- **Object Info Panel** — select any body, see its physical properties,
  orbital elements, current position/velocity
- **Time Controls** — play/pause, time scale slider, current simulation time
- **Camera Controls** — current mode, focus target, speed settings
- **Rendering Settings** — bloom intensity, tone mapping mode, atmosphere
  toggle

#### Technical
- Cross-platform: macOS, Linux, Windows
- 7-crate workspace architecture (see Technical Requirements)
- All crates compile and run independently where possible

### In Scope — Post-MVP

These are features for later milestones. They're not in the MVP, but the
architecture should support them.

#### v0.2: "Solar System"
- Complete moon systems for all gas giants
- Ring systems for Saturn, Uranus, Neptune
- Improved asteroid belt with proper size distribution
- Orbital trails/paths visualization
- Lagrange point calculation and visualization

#### v0.3: "Living Worlds"
- Full atmospheric simulation (simplified Navier-Stokes or cell-based)
- Ocean simulation with specular reflections
- Vegetation rendering on Earth and Mars (past/future)
- Day/night cycle with city lights on Earth
- Weather systems (cloud layers, storms)

#### v0.4: "Deep Space"
- Procedural galaxy generation (spiral, elliptical, irregular)
- Nebula rendering with volumetric techniques
- Star clusters and open clusters
- Interstellar medium (dust lanes, HII regions)
- Exoplanet systems with procedural generation

#### v0.5: "N-Body"
- Full gravitational N-body simulation
- Real-time gravitational interactions between all bodies
- Lagrange point stability analysis
- Roche limit calculations
- Tidal force visualization

#### v0.6: "Spacecraft"
- Spacecraft construction (modular parts)
- Thruster physics (chemical, ion, nuclear)
- Hohmann transfers and gravity assists
- Fuel management
- Orbital maneuvering

#### v0.7: "Cosmos"
- Universe-scale streaming with octree LOD
- LOD transitions from surface detail to interplanetary to interstellar
- Level-of-detail for terrain, atmospheres, and orbital mechanics
- Memory-mapped streaming for massive datasets

#### v0.8: "Intelligence"
- AI civilizations that emerge from ecosystem simulation
- Technology trees driven by resource availability
- Communication and conflict between civilizations
- Historical replay of civilization evolution

#### v0.9: "Multiverse"
- Sandbox for changing universal constants (G, c, Planck's constant, etc.)
- Side-by-side comparison of multiple universe configurations
- Export simulation data for analysis
- Parameter sensitivity visualization

#### v1.0: "Cosmogon"
- Production quality code
- Full documentation (API docs, user guide, architecture guide)
- Performance optimization pass
- Accessibility features
- Community contribution guidelines

### Explicitly Out of Scope (Non-Goals)

These are things Cosmogon **will not** do, at least not for the foreseeable
future. This section exists to prevent scope creep and keep the project focused.

- **Multiplayer/Networking** — This is a single-user experience. No
  leaderboards, no shared servers, no real-time collaboration. If that changes,
  it'll be a separate project or a major fork.

- **Game Mechanics** — No scoring, no objectives, no win conditions, no health
  bars. Cosmogon is a simulator, not a game. If you want to play a space game,
  play KSP or Elite Dangerous.

- **VR Support** — VR adds enormous complexity (stereo rendering, motion
  sickness mitigation, input abstraction). It's a "maybe someday" feature, not
  a core requirement.

- **Mobile Platforms** — Mobile GPUs and input methods are fundamentally
  different. Cosmogon targets desktop hardware with real GPUs and keyboard/mouse
  input.

- **Commercial Distribution** — This is a passion project, not a product.
  No licensing, no DRM, no subscription models. It's open source and free.

---

## 4. User Stories

User stories drive features. Each story has a persona, a goal, and acceptance
criteria. The MVP must satisfy all "must have" stories.

### Explorer Persona

**ES-1: Fly Through the Solar System**
> As an explorer, I want to fly my camera from Earth to Mars at light speed so
> that I can experience the scale of the solar system.

*Must Have*
- Fly-through camera with WASD + mouse
- Speed scaling (slow near planets, fast in space)
- Smooth movement at all scales

*Acceptance Criteria:*
- Camera starts at Earth's surface
- Can fly to Mars in < 10 seconds at max speed
- No visual glitches during transit
- FPS stays above 30 during fast movement

**ES-2: Orbit a Planet**
> As an explorer, I want to click on Jupiter and orbit around it so that I can
> see its bands and moons from every angle.

*Must Have*
- Click to select any body
- Orbit camera mode with mouse drag
- Correct relative positions of moons

*Acceptance Criteria:*
- Clicking Jupiter switches to orbit camera centered on Jupiter
- Mouse drag orbits around Jupiter
- Galilean moons visible and orbiting correctly
- Can switch back to fly-through camera

**ES-3: Land on a Planet**
> As an explorer, I want to land on Mars' surface and look around so that I can
> see what it would be like to stand on another world.

*Must Have*
- Surface camera mode
- Procedural terrain visible from surface
- Correct sky color for the planet

*Acceptance Criteria:*
- Can transition from orbit to surface seamlessly
- Terrain has visible features (craters, valleys, mountains)
- Mars sky is butterscotch color (correct)
- Sun appears correct size from Mars surface

**ES-4: Visit the Sun**
> As an explorer, I want to approach the Sun and see its surface churning so
> that I can appreciate the power of a star.

*Must Have*
- Procedural solar surface texture
- Bloom/glow effect
- Correct color (white-yellow, not orange)

*Acceptance Criteria:*
- Sun surface shows granulation patterns
- Bloom creates realistic glow
- Approaching doesn't cause rendering artifacts
- Sun appears correctly dominant in the sky from inner planets

### Student Persona

**ST-1: Learn Planet Properties**
> As a student, I want to click on Saturn and see its mass, diameter, and
> orbital period so that I can learn about it without Googling.

*Must Have*
- Object info panel
- Real physical data for each body
- Clear, readable display

*Acceptance Criteria:*
- Clicking Saturn shows mass (5.683 × 10²⁶ kg)
- Shows diameter, orbital period, rotation period
- Data matches NASA fact sheets
- Panel is readable at various resolutions

**ST-2: Watch Orbital Mechanics**
> As a student, I want to speed up time to 1000x so that I can see planets
> complete orbits and understand Kepler's laws.

*Must Have*
- Time acceleration up to 10,000x
- Planets visibly orbit at high time scales
- Orbital trails visible

*Acceptance Criteria:*
- At 1000x, Mercury completes an orbit in ~2.8 seconds (real period ~88 days)
- Inner planets visibly orbit faster than outer planets (Kepler's 3rd law)
- Orbital paths are visible as trails
- No numerical instability at high time scales

**ST-3: Compare Planet Sizes**
> As a student, I want to see all the planets side by side so that I can
> understand their relative sizes.

*Must Have*
- Ability to view multiple planets
- Correct relative sizes
- Optional log-scale for visualization

*Acceptance Criteria:*
- Jupiter is clearly larger than Earth
- Earth is clearly larger than Mars
- Correct size ratios (Jupiter ~11x Earth diameter)
- Log scale available for extreme comparisons

**ST-4: Observe Atmospheres**
> As a student, I want to see Earth's blue atmosphere and Venus's thick
> atmosphere so that I can understand how atmospheres affect a planet's
> appearance.

*Must Have*
- Atmospheric scattering for Earth
- Atmospheric scattering for Venus
- Visible limb darkening

*Acceptance Criteria:*
- Earth's atmosphere is blue from space
- Venus's atmosphere is yellowish-white
- Sunset colors visible on Earth's limb
- Atmosphere has proper thickness relative to planet

### Scientist Persona

**SC-1: Modify Gravitational Constant**
> As a scientist, I want to change the gravitational constant and observe how
> orbital periods change so that I can experiment with fundamental physics.

*Must Have*
- Editable universal constants in UI
- Simulation updates in real-time
- Orbital elements recalculate correctly

*Acceptance Criteria:*
- Can change G (gravitational constant) via slider
- Orbital periods change proportionally (T ∝ 1/√G)
- No simulation instability for reasonable G changes
- Can reset to real value

**SC-2: Add a New Body**
> As a scientist, I want to add a new planet at a specific orbit so that I can
> simulate hypothetical solar system configurations.

*Must Have*
- UI to add a new body
- Specify mass, orbit, radius
- Body integrates into simulation immediately

*Acceptance Criteria:*
- Can add a planet at Mars's orbit with 2x Earth's mass
- New planet orbits correctly
- Gravitational influence on other bodies visible (post-N-body)
- Can remove the added body

**SC-3: Export Simulation Data**
> As a scientist, I want to export orbital elements and positions over time so
> that I can analyze the data in other tools.

*Must Have*
- CSV/JSON export of simulation state
- Time-series data for orbital elements
- Position/velocity data at each timestep

*Acceptance Criteria:*
- Export includes body name, time, position (x,y,z), velocity (vx,vy,vz)
- Orbital elements exported (a, e, i, Ω, ω, ν)
- Data opens correctly in Python/pandas
- Export doesn't stall the simulation

### Developer Persona

**DE-1: Extend with New Body Types**
> As a developer, I want to add a new celestial body type (e.g., binary star
> system) by implementing a trait so that I can extend Cosmogon without
> modifying core code.

*Must Have*
- Trait-based body definitions
- Clear extension points in the ECS
- Example of adding a custom body type

*Acceptance Criteria:*
- Can create a new body type in a separate crate
- New body type renders and simulates correctly
- Documentation explains the extension process
- Existing bodies are not affected

**DE-2: Custom Rendering Passes**
> As a developer, I want to add a custom rendering pass (e.g., heat
> visualization) by implementing a RenderPass trait so that I can extend the
> rendering pipeline.

*Must Have*
- Trait-based render pass system
- Access to GPU resources
- Integration with existing pipeline

*Acceptance Criteria:*
- Can add a render pass that draws colored overlays on bodies
- Render pass composites correctly with existing passes
- No performance regression when custom pass is disabled
- Documentation explains the render pass interface

**DE-3: Read the Architecture**
> As a developer, I want to read an architecture document that explains how the
> 7 crates fit together so that I can understand the codebase quickly.

*Must Have*
- Architecture document in docs/
- Crate dependency diagram
- Data flow explanation
- Key abstractions documented

*Acceptance Criteria:*
- Document exists and is up-to-date
- Diagram shows all 7 crates and their dependencies
- Explains the ECS data flow
- Explains the rendering pipeline stages

### Viewer Persona

**VW-1: Screenshots and Screen Recording**
> As a viewer, I want to take high-resolution screenshots of beautiful scenes
> so that I can share them on social media.

*Must Have*
- Screenshot capture (button or hotkey)
- Resolution options (1080p, 4K, custom)
- No UI overlay in screenshots (optional)

*Acceptance Criteria:*
- F12 captures a screenshot
- Screenshot saves as PNG
- Resolution matches window or specified size
- UI can be toggled off for clean screenshots

**VW-2: Cinematic Camera Paths**
> As a viewer, I want to set camera waypoints and have the camera automatically
> move between them so that I can create cinematic sequences.

*Must Have*
- Waypoint system (set points in space)
- Smooth interpolation between waypoints
- Play/pause the camera path

*Acceptance Criteria:*
- Can set 3+ waypoints
- Camera smoothly moves between them
- Time continues to advance during camera path
- Can adjust speed of camera movement

**VW-3: Ambient Sound**
> As a viewer, I want ambient space sounds (low hum, wind on planet surfaces)
> so that the experience feels more immersive.

*Must Have*
- Background ambient audio
- Different audio for space vs. planet surfaces
- Volume control

*Acceptance Criteria:*
- Space has a low, eerie hum
- Earth surface has wind and nature sounds
- Volume slider in UI
- Audio doesn't impact FPS

---

## 5. Technical Requirements

### Language & Edition

- **Rust** (edition 2021)
- Minimum supported Rust version (MSRV): 1.75.0
- Stable channel only (no nightly features unless absolutely necessary)

### Workspace Architecture

Cosmogon is organized as a Cargo workspace with 7 crates. Each crate has a
single responsibility and clean public APIs.

```
cosmogon/
├── Cargo.toml              # workspace root
├── crates/
│   ├── cosmogon_core/      # Fundamental types, traits, constants
│   ├── cosmogon_ecs/       # ECS wrapper around bevy_ecs
│   ├── cosmogon_physics/   # Orbital mechanics, gravity, time
│   ├── cosmogon_render/    # wgpu rendering pipeline
│   ├── cosmogon_scene/     # Scene graph, body definitions, loading
│   ├── cosmogon_ui/        # egui interface panels
│   └── cosmogon_app/       # Application entry point, window, event loop
├── assets/
│   ├── textures/           # Planet textures, star catalogs
│   ├── shaders/            # WGSL shader files
│   └── data/               # Orbital element data (NASA JPL)
└── docs/
    ├── ARCHITECTURE.md     # System architecture documentation
    ├── ADRs/               # Architecture Decision Records
    └── brain.md            # Project brain dump, ideas, research
```

#### Crate Responsibilities

**cosmogon_core** — The foundation. Everything else depends on this.
- Fundamental types: `Vec3f64`, `Vec3f32`, `Quaternion`, `Transform`
- Traits: `CelestialBody`, `Renderable`, `Simulatable`
- Constants: G (gravitational constant), speed of light, astronomical unit
- Error types
- No external dependencies beyond `glam`, `serde`, `thiserror`

**cosmogon_ecs** — Thin wrapper around bevy_ecs.
- Re-exports key bevy_ecs types
- Custom component definitions for Cosmogon-specific data
- System scheduling helpers
- Plugin trait for modular system registration

**cosmogon_physics** — All simulation logic.
- Kepler's equation solver
- Orbital element ↔ Cartesian coordinate conversion
- Gravitational force calculations
- Time stepping with sub-step accumulation
- N-body gravitational integration (post-MVP)
- Deterministic math utilities

**cosmogon_render** — The visual layer.
- wgpu device/queue management
- Render pipeline creation and management
- Shader loading and compilation
- PBR material system
- Atmospheric scattering shaders
- Starfield rendering
- Post-processing (bloom, tone mapping)
- Camera implementation

**cosmogon_scene** — The data layer.
- Scene graph (parent-child relationships)
- Body definitions and loading from data files
- Procedural terrain generation
- Texture management
- Asset loading pipeline

**cosmogon_ui** — The interface layer.
- egui integration with wgpu
- Object info panel
- Time controls
- Camera controls
- Settings panel
- Debug overlays

**cosmogon_app** — The glue.
- Window creation and management (winit)
- Event loop
- Input handling
- State management
- Application lifecycle
- Integration of all other crates

#### Dependency Graph

```
cosmogon_app
├── cosmogon_ui
│   ├── cosmogon_scene
│   │   ├── cosmogon_physics
│   │   │   └── cosmogon_core
│   │   └── cosmogon_ecs
│   │       └── cosmogon_core
│   └── cosmogon_core
├── cosmogon_render
│   └── cosmogon_core
└── cosmogon_core
```

Key rules:
- `cosmogon_core` has zero internal dependencies
- `cosmogon_physics` depends only on `cosmogon_core`
- `cosmogon_ecs` depends only on `cosmogon_core`
- No circular dependencies
- Each crate can be tested independently

### Rendering Pipeline

#### wgpu Setup
- WebGPU backend (not Vulkan/Metal/DX12 directly — wgpu abstracts this)
- Surface format: `Bgra8UnormSrgb` (most common across platforms)
- HDR render target: `Rgba16Float` for HDR rendering before tone mapping
- MSAA: 4x for quality (configurable)

#### Render Passes (in order)
1. **Clear pass** — Clear framebuffer
2. **Scene pass** — Render all celestial bodies with PBR materials
3. **Atmosphere pass** — Render atmospheric scattering for applicable bodies
4. **Starfield pass** — Render background starfield
5. **Bloom pass** — Extract bright areas, blur, composite
6. **Tone mapping pass** — HDR to LDR conversion
7. **UI pass** — Render egui panels on top

#### Shader Language
- WGSL (WebGPU Shading Language)
- All shaders in `assets/shaders/`
- Hot-reloading support for development

### Coordinate System

- **Physics coordinates:** f64 precision, origin at solar system barycenter
- **Rendering coordinates:** f32 precision, origin shifts to camera focus
- **Floating origin:** Camera-relative positioning to avoid f32 precision loss
  at large distances
- **Units:** meters for distance, kilograms for mass, seconds for time

The floating origin system is critical. At interplanetary distances, f32
precision is insufficient. The solution:
1. Physics runs in f64 with absolute positions
2. When rendering, subtract camera position to get relative positions
3. Relative positions are converted to f32 for the GPU
4. This gives us ~1mm precision at any distance from the origin

### ECS Architecture

Using `bevy_ecs` as a standalone crate (not the full Bevy engine) gives us:
- Entity Component System pattern
- System scheduling and parallelism
- Query system for efficient iteration
- Event system for decoupled communication

Key Components:
```rust
// Core components (cosmogon_core)
struct Position(Vec3f64);
struct Velocity(Vec3f64);
struct Mass(f64);
struct Radius(f64);

// Orbital components (cosmogon_physics)
struct OrbitalElements {
    semi_major_axis: f64,
    eccentricity: f64,
    inclination: f64,
    longitude_ascending_node: f64,
    argument_perihelion: f64,
    mean_anomaly_epoch: f64,
}

// Rendering components (cosmogon_render)
struct RenderMesh { /* ... */ }
struct PbrMaterial { /* ... */ }
struct Atmosphere { /* ... */ }

// Scene components (cosmogon_scene)
struct CelestialBody {
    name: String,
    body_type: BodyType,
    parent: Option<Entity>,
}
```

### Data Sources

- **Orbital elements:** NASA JPL Solar System Dynamics (SSD)
- **Physical constants:** NIST CODATA
- **Star catalog:** Hipparcos catalog for bright stars
- **Planet textures:** NASA/JPL Photojournal, processed to appropriate resolution

### Testing Strategy

- **Unit tests:** Every public function in `cosmogon_core` and
  `cosmogon_physics` has unit tests
- **Integration tests:** Orbital mechanics tests verify real solar system
  behavior
- **Visual regression tests:** Screenshot comparison for rendering (future)
- **Benchmarks:** Performance benchmarks for hot paths (orbital element
  conversion, Kepler's equation solver)

---

## 6. Milestones

Milestones are the project roadmap. Each milestone is a complete, shippable
version that adds significant capability. The milestone order is deliberate —
each builds on the previous.

### v0.1 — "First Light"

*The proof of concept. A single star with orbiting planets, rendered with PBR.*

**Deliverables:**
- Sun rendered with procedural texture and bloom
- All 8 planets with correct colors and sizes
- Keplerian orbits with correct periods
- PBR materials on all bodies
- Basic fly-through camera
- Basic orbit camera
- egui UI with time controls
- Starfield background

**Exit Criteria:**
- Runs at 60fps on GTX 1060
- All orbital periods within 1% of real values
- Can fly from Earth to Mars
- Can pause/resume time
- Compiles on macOS, Linux, Windows

### v0.2 — "Solar System"

*The complete solar system with all major moons and time controls.*

**Deliverables:**
- All major moons (Moon, Galilean moons, Titan, etc.)
- Saturn's rings
- Orbital trail visualization
- Improved time controls (keyboard shortcuts, time presets)
- Object selection and info display
- Camera focus transitions

**Exit Criteria:**
- All 8 planets + major moons visible and orbiting
- Saturn's rings render correctly
- Can select any body and see its properties
- Orbital trails visible for inner planets

### v0.3 — "Living Worlds"

*Planets come alive with atmospheres, terrain, and oceans.*

**Deliverables:**
- Atmospheric scattering for Earth and Venus
- Procedural terrain for Earth, Mars, Moon
- Ocean rendering on Earth
- Day/night cycle with city lights
- Weather visualization (cloud layers)

**Exit Criteria:**
- Earth's atmosphere is blue from space
- Venus's atmosphere is yellowish
- Can land on Earth and see terrain
- Can land on Mars and see terrain
- Oceans reflect the sky

### v0.4 — "Deep Space"

*Beyond the solar system — galaxies, nebulae, and the interstellar medium.*

**Deliverables:**
- Procedural galaxy generation
- Volumetric nebula rendering
- Star cluster generation
- Interstellar dust rendering
- Improved starfield with proper magnitudes

**Exit Criteria:**
- Can zoom out from solar system to see galaxy
- Nebulae have volumetric appearance
- Star clusters look realistic
- Performance maintained at galactic scale

### v0.5 — "N-Body"

*Full gravitational simulation — bodies influence each other.*

**Deliverables:**
- N-body gravitational integration
- Lagrange point calculation and visualization
- Tidal force effects
- Orbital perturbation visualization
- Stability analysis tools

**Exit Criteria:**
- Three-body problem produces correct chaotic behavior
- Lagrange points are stable/unstable as expected
- Can observe orbital precession from perturbations
- Performance: 100 bodies at 60fps

### v0.6 — "Spacecraft"

*Build, launch, and fly spacecraft.*

**Deliverables:**
- Modular spacecraft construction
- Thruster physics (chemical, ion, nuclear)
- Hohmann transfer calculation
- Gravity assist trajectories
- Fuel management
- Orbital maneuvering interface

**Exit Criteria:**
- Can build a spacecraft from parts
- Can achieve orbit around Earth
- Can perform Hohmann transfer to Mars
- Can land on Mars
- Fuel depletes realistically

### v0.7 — "Cosmos"

*Universe-scale simulation with streaming and LOD.*

**Deliverables:**
- Octree-based spatial partitioning
- Level-of-detail system for all rendering
- Streaming terrain and textures
- Memory-mapped asset loading
- Seamless zoom from surface to interstellar

**Exit Criteria:**
- Can zoom from Earth surface to intergalactic without loading screens
- Memory usage stays bounded
- Performance scales with visible detail level
- LOD transitions are smooth

### v0.8 — "Intelligence"

*AI civilizations emerge from ecosystem simulation.*

**Deliverables:**
- Ecosystem simulation (flora, fauna)
- Resource availability modeling
- Technology development simulation
- Civilization emergence and evolution
- Historical replay system

**Exit Criteria:**
- Civilizations emerge on habitable worlds
- Technology progression follows plausible paths
- Can replay civilization history from start
- Multiple civilizations can interact

### v0.9 — "Multiverse"

*Change the rules of physics and watch the consequences.*

**Deliverables:**
- Universal constants editor (G, c, ℏ, etc.)
- Side-by-side universe comparison
- Parameter sensitivity visualization
- Simulation data export
- Historical comparison (what if G were different?)

**Exit Criteria:**
- Can change G and observe orbital period changes
- Can compare two universe configurations side-by-side
- Exported data is analyzable in standard tools
- No simulation instability for reasonable constant changes

### v1.0 — "Cosmogon"

*Production quality, full documentation, community ready.*

**Deliverables:**
- All public APIs documented with doc comments
- >80% test coverage on core crates
- Architecture documentation complete
- ADRs for all major decisions
- Contributing guidelines
- Performance optimization pass
- Accessibility features (keyboard navigation, screen reader support)

**Exit Criteria:**
- `cargo doc` produces complete documentation
- All tests pass on CI
- Performance meets all success metrics
- Can be built by a new contributor following documentation alone

---

## 7. Success Metrics

These are the measurable goals that define whether Cosmogon is succeeding.
They're organized by category.

### Performance

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Frame rate (full solar system) | ≥ 60 fps | FPS counter in debug mode |
| Frame rate (surface view) | ≥ 60 fps | FPS counter in debug mode |
| Frame time (1% low) | ≤ 20 ms | Frame time profiler |
| Memory usage | ≤ 2 GB | Process memory monitoring |
| GPU memory | ≤ 4 GB | GPU memory profiler |
| Load time (cold start) | ≤ 5 seconds | Stopwatch from launch to render |
| CPU usage (idle simulation) | ≤ 10% | System monitor |

### Scale

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Zoom range | 1m to 100 AU | Camera distance readout |
| Scale transition | Seamless | Visual inspection, no pop-in |
| LOD management | Automatic | Memory stays bounded |

### Accuracy

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Orbital period error | ≤ 1% | Compare simulated vs. NASA values |
| Orbital eccentricity | Matches real values | Compare orbital elements |
| Planet sizes | Within 5% | Visual comparison with reference |
| Star colors | Match spectral classes | Visual comparison |

### Code Quality

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Public API documentation | 100% | `cargo doc` coverage |
| Core crate test coverage | > 80% | `cargo tarpaulin` |
| Clippy warnings | 0 | `cargo clippy -- -D warnings` |
| Compile warnings | 0 | `cargo build 2>&1 | grep warning` |
| Unsafe code | Minimized, audited | `grep -r "unsafe" crates/` |

### Documentation

| Metric | Target | How to Measure |
|--------|--------|----------------|
| Architecture doc | Complete and current | Manual review |
| ADRs | All major decisions documented | Count ADRs vs. decisions |
| brain.md | Updated weekly | Last modified date |
| Code examples | All public APIs have examples | `cargo test --doc` |

### Community

| Metric | Target | How to Measure |
|--------|--------|----------------|
| GitHub stars | 100+ in first year | GitHub analytics |
| Contributors | 5+ in first year | GitHub contributors page |
| Issue response time | < 48 hours | Issue timestamps |
| PR review time | < 1 week | PR timestamps |
| First-time contributor success | > 50% | Contributing guide effectiveness |

---

## 8. Risks & Mitigations

Every project has risks. The goal isn't to avoid them — it's to identify them
early and have plans to deal with them.

### Risk 1: Scope Creep

**Severity:** High
**Likelihood:** High

The biggest risk to Cosmogon is trying to do too much too fast. Every cool idea
feels like it should be in the next milestone. It's not.

**Mitigation:**
- Strict milestone boundaries: if it's not in the milestone spec, it doesn't
  ship
- "parking lot" document for ideas that don't fit current milestone
- Regular scope reviews (monthly) to ensure we're not drifting
- "No" is a complete sentence for feature requests that don't align with the
  current milestone

### Risk 2: Performance at Scale

**Severity:** High
**Likelihood:** Medium

Rendering a full solar system with PBR materials, atmospheres, and post-
processing is demanding. Performance could become a bottleneck as features are
added.

**Mitigation:**
- LOD system designed from the start (even if simple in v0.1)
- Profiling built into the application (not bolted on later)
- Performance budgets per frame (e.g., 2ms for physics, 8ms for rendering)
- GPU compute for expensive operations (atmospheric scattering, particle
  systems)
- Early testing on target hardware (GTX 1060)

### Risk 3: Math Complexity of Orbital Mechanics

**Severity:** Medium
**Likelihood:** Medium

Kepler's equation, coordinate transformations, and orbital element conversions
are mathematically dense. Getting them wrong means incorrect orbits, which
breaks the core experience.

**Mitigation:**
- Well-tested reference implementations (NASA JPL SPICE toolkit algorithms)
- Unit tests against known values (real solar system data)
- Integration tests that verify orbital periods match real values
- Cross-validation with Python/STK implementations
- Clear separation of math code (easy to test in isolation)

### Risk 4: wgpu API Instability

**Severity:** Medium
**Likelihood:** Low

wgpu is actively developed and the API has changed between major versions.
Pinning to a specific version is necessary, but future versions may require
migration work.

**Mitigation:**
- Pin wgpu to a specific version (currently 24.0)
- Abstract wgpu calls behind a rendering trait layer
- Monitor wgpu changelog for breaking changes
- Budget time for wgpu upgrades in major milestones
- Consider contributing upstream if bugs are encountered

### Risk 5: Floating-Point Precision

**Severity:** Medium
**Likelihood:** Medium

At astronomical distances, f32 precision is insufficient. Without proper
handling, rendering will jitter or planets will clip through each other.

**Mitigation:**
- f64 for all physics calculations
- Floating-origin system for rendering (camera-relative positioning)
- f32 used only for GPU-side rendering (after coordinate transformation)
- Precision tests at various distances (1m, 1AU, 100AU)
- Known working solution (Space Engine uses similar approach)

### Risk 6: Procedural Generation Quality

**Severity:** Low
**Likelihood:** Medium

Procedural terrain and textures might look bad or unrealistic, undermining the
visual quality goal.

**Mitigation:**
- Start with real textures where available (NASA imagery)
- Procedural generation as enhancement, not replacement
- User-adjustable parameters for procedural generation
- Reference images for comparison
- Community feedback on visual quality

### Risk 7: Cross-Platform Compatibility

**Severity:** Medium
**Likelihood:** Low

macOS, Linux, and Windows have different graphics driver behaviors, window
management, and system APIs.

**Mitigation:**
- wgpu handles most platform differences
- winit handles window management cross-platform
- CI builds on all three platforms
- Test on actual hardware, not just VMs
- Platform-specific workarounds documented in ADRs

---

## 9. Constraints

Constraints are non-negotiable. They're the boundaries we work within.

### Solo Developer

Cosmogon is primarily a solo project. This means:
- No team meetings, no coordination overhead
- But also no one to review code, no one to bounce ideas off
- Community contributions are welcome but not assumed
- Progress may be slow, and that's okay

### Consumer Hardware Target

Cosmogon must run on consumer hardware, specifically:
- **Minimum GPU:** NVIDIA GTX 1060 / AMD RX 580 / Intel UHD 630
- **Minimum RAM:** 8 GB
- **Recommended GPU:** NVIDIA RTX 3060 / AMD RX 6600
- **Recommended RAM:** 16 GB
- **Storage:** 500 MB for application + assets

This means:
- No requiring RTX features (ray tracing, DLSS)
- No requiring 16GB+ VRAM
- No compute shader features only available on high-end GPUs
- Fallback paths for older hardware where necessary

### Cross-Platform Compilation

Cosmogon must compile and run on:
- **macOS** (ARM64 and x86_64) — Metal backend via wgpu
- **Linux** (x86_64) — Vulkan backend via wgpu
- **Windows** (x86_64) — DX12 or Vulkan backend via wgpu

CI must build and test on all three platforms before any release.

### No Proprietary Dependencies

Cosmogon is open source (MIT/Apache-2.0). All dependencies must be:
- Open source with compatible licenses
- No GPL-incompatible licenses
- No proprietary SDKs or libraries
- No telemetry or phone-home behavior

### Memory Budget

Total memory usage must stay under 2 GB for the application, including:
- ECS world and components
- GPU buffers (vertex, index, uniform)
- Texture memory
- Shader cache
- Audio buffers

This is achievable with proper LOD and streaming, but requires discipline.

---

## 10. Assumptions

These are the things we're taking for granted. If any of these prove false,
the project plan will need adjustment.

### Time Availability

The developer has time for a multi-year project. Cosmogon is a passion project
that will be built over evenings and weekends. Progress will be nonlinear —
some weeks will see major advances, others will see none. This is expected and
accepted.

### Community Interest

There may be community interest in contributing, but the project is designed
to be buildable by a solo developer. Community contributions are a bonus, not
a dependency. The architecture should make it easy for others to contribute,
but should not require it.

### Data Availability

Real astronomical data is freely available from:
- **NASA JPL Solar System Dynamics** — orbital elements, physical constants
- **Hipparcos Catalog** — star positions and magnitudes
- **NASA Photojournal** — planet textures and imagery
- **IAU** — official planet/moon data

This data will be used where available, with procedural generation filling in
gaps.

### Hardware Evolution

Consumer hardware will continue to improve. The GTX 1060 minimum target is
intentionally conservative. As the project matures, the minimum spec can be
raised if necessary, but the initial target ensures broad accessibility.

### Motivation

The developer is motivated by the project itself, not by external deadlines or
commercial pressure. Intrinsic motivation is the most reliable fuel for a
multi-year passion project. If motivation wanes, it's okay to take breaks and
return when inspired.

### Technical Feasibility

The combination of Rust + wgpu + bevy_ecs is proven to be capable of
high-performance real-time rendering. These are not experimental technologies —
they're production-ready tools used by real projects. The technical risk is
low; the effort risk is medium.

---

## Appendix A: Glossary

| Term | Definition |
|------|-----------|
| **Keplerian orbit** | An orbit described by Kepler's laws — elliptical, with the central body at one focus |
| **Orbital elements** | The 6 parameters that uniquely define an orbit (a, e, i, Ω, ω, ν) |
| **PBR** | Physically Based Rendering — lighting model that simulates real light behavior |
| **Rayleigh scattering** | Scattering of light by particles smaller than the wavelength (causes blue sky) |
| **Mie scattering** | Scattering of light by particles comparable to wavelength (causes white haze) |
| **HDR** | High Dynamic Range — rendering with values beyond 0-1 for realistic exposure |
| **Tone mapping** | Converting HDR values to LDR for display |
| **Bloom** | Glow effect around bright objects |
| **ECS** | Entity Component System — architectural pattern for game/simulation engines |
| **LOD** | Level of Detail — reducing complexity for distant objects |
| **Floating origin** | Technique to maintain precision by re-centering coordinates on the camera |
| **N-body** | Simulation where every body gravitationally interacts with every other body |
| **Hohmann transfer** | Minimum-fuel orbital transfer between two circular orbits |
| **Lagrange points** | 5 points in a two-body system where a third body can maintain stable position |
| **Roche limit** | Distance within which a celestial body disintegrates due to tidal forces |

## Appendix B: Reference Data Sources

| Data | Source | URL |
|------|--------|-----|
| Orbital Elements | NASA JPL SSD | https://ssd.jpl.nasa.gov/planets/ |
| Physical Constants | NIST CODATA | https://physics.nist.gov/cuu/Constants/ |
| Star Catalog | Hipparcos | https://www.cosmos.esa.int/web/hipparcos |
| Planet Textures | NASA/JPL Photojournal | https://photojournal.jpl.nasa.gov/ |
| Moon Data | NASA GSFC | https://nssdc.gsfc.nasa.gov/planetary/ |

## Appendix C: Architecture Decision Records

ADRs will be stored in `docs/ADRs/` and follow this naming convention:

```
ADR-001-title.md
ADR-002-title.md
...
```

Each ADR contains:
- **Title** — Short name for the decision
- **Status** — Proposed, Accepted, Deprecated, Superseded
- **Context** — What situation necessitates this decision?
- **Decision** — What was decided?
- **Consequences** — What are the positive and negative outcomes?

Key ADRs to create early:
- ADR-001: Use wgpu instead of raw Vulkan/Metal/DX12
- ADR-002: Use bevy_ecs instead of custom ECS
- ADR-003: f64 for physics, f32 for rendering, floating origin
- ADR-004: 7-crate workspace architecture
- ADR-005: egui for UI instead of custom UI system
- ADR-006: Keplerian mechanics before N-body
- ADR-007: WGSL for shaders instead of SPIR-V

---

*This document is a living artifact. It will be updated as the project evolves,
milestones are completed, and new requirements emerge. Last updated: 2026-07-20.*
