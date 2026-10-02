# Cosmogon Brain

> This is the living knowledge base for Cosmogon. If it matters to the project, it goes here.
> This file evolves with the codebase. When you learn something new, add it. When you make a
> decision, document it. This is the project's memory.

---

## Philosophy

- **Simulate, don't script.** Planets move because of gravity, not because of keyframes. Stars
  shine because of fusion models, not because of a "make it glow" toggle. If you can derive it
  from physics, derive it.
- **Emergent behavior over hardcoded behavior.** We don't program asteroid belt gaps — they emerge
  from orbital resonances. We don't program Saturn's rings — they emerge from particle dynamics.
  The best features are ones we didn't explicitly build.
- **Scientific accuracy where practical.** We aim for real orbital elements, real atmospheric
  scattering coefficients, real spectral classes. But we're not building JPL Horizons — if
  something needs to be faked for performance, we fake it honestly and document the approximation.
- **Beautiful AND performant.** We don't sacrifice visuals for speed or vice versa. We find the
  intersection. GPU acceleration, LOD, and smart algorithms let us have both.
- **Every feature should feel like it comes from underlying physics.** If a planet looks right,
  it's because its atmosphere is modeled correctly. If an asteroid field looks right, it's because
  the orbits are dynamically stable. The universe is beautiful because it's physical.

---

## Coding Standards

### Rust Idioms

- Prefer iterators over explicit loops (`map`, `filter`, `fold`, `for_each`)
- Use `Option<T>` and `Result<T, E>` properly — no sentinel values
- Prefer `?` operator over `.unwrap()` or `.expect()` in library code
- Use `#[must_use]` on important return values (computed positions, energy values)
- Prefer `&str` over `String` in function parameters
- Derive common traits: `Debug`, `Clone`, `Copy` (where appropriate, especially for math types)
- Use `From`/`Into` for conversions, not ad-hoc methods
- Use `thiserror` for library error types, `anyhow` for application-level errors

### Naming Conventions

| Category | Convention | Example |
|----------|-----------|---------|
| Crates | `cosmogon_[concern]` | `cosmogon_physics`, `cosmogon_render` |
| Components | PascalCase noun | `Position`, `Velocity`, `Mass`, `OrbitalElements` |
| Systems | `verb_noun` | `update_orbital_positions`, `render_planets` |
| Resources | PascalCase with `State`/`Config` suffix | `TimeState`, `RenderConfig`, `SimulationConfig` |
| Shaders | lowercase snake_case `.wgsl` | `planet.wgsl`, `atmosphere.wgsl`, `star.wgsl` |
| Constants | Standard abbreviations for physical constants | `G`, `C`, `AU`, `SOLAR_MASS` |
| Files | snake_case.rs | `orbital_mechanics.rs`, `atmospheric_scattering.rs` |
| Modules | snake_case | `mod orbital_mechanics;` |
| Enums | PascalCase, variants PascalCase | `CelestialBody::Star`, `CelestialBody::Planet` |

### File Organization

- One concern per file
- Keep files under 500 lines (split when you're scrolling too much)
- Module-level doc comments explain the purpose, not the contents
- Group related functions with `// --- Section ---` comments if needed

### Comments

- Only comment the WHY, never the WHAT
- If the code needs a comment to explain what it does, rewrite the code
- Use doc comments (`///`) on all public items
- Use `//` for implementation notes about non-obvious design decisions
- `// TODO(name):` for future work (include a name and issue number if possible)
- `// HACK:` for temporary workarounds (must have a plan to remove)

### Error Handling

```rust
// Good: library error with thiserror
#[derive(Debug, thiserror::Error)]
pub enum PhysicsError {
    #[error("orbital elements are invalid: {0}")]
    InvalidOrbitalElements(String),
    #[error("integration failed at step {step}: {reason}")]
    IntegrationFailed { step: usize, reason: String },
}

// Good: application-level with anyhow
fn main() -> anyhow::Result<()> {
    let config = load_config()?;  // anyhow propagates automatically
    let universe = build_universe(&config)?;
    run_simulation(universe)?;
    Ok(())
}
```

- No `unwrap()` in library code
- `unwrap()` is okay in examples, tests, and one-off scripts
- Use `expect("descriptive message")` if failure truly is impossible

### Testing

- Unit tests in each module (`#[cfg(test)] mod tests`)
- Integration tests in `tests/` directory
- Test edge cases: zero-mass bodies, coincident positions, very large/small values
- Test with known analytical solutions (Keplerian orbits)
- Use `approx` crate for floating-point comparisons
- Benchmark critical paths with `criterion`

---

## Architectural Principles

### Workspace Structure

```
cosmogon/
├── cosmogon_core/      # Math, coordinate systems, constants, traits
├── cosmogon_ecs/       # ECS framework (bevy_ecs wrapper or custom)
├── cosmogon_physics/   # Orbital mechanics, N-body, gravity
├── cosmogon_render/    # wgpu rendering, shaders, pipelines
├── cosmogon_atmosphere/# Atmospheric scattering models
├── cosmogon_terrain/   # Procedural terrain, noise, generation
└── cosmogon_app/       # Application shell, windowing, main loop
```

### Dependency Direction

```
core → ecs → physics → render → app
                 ↓
              atmosphere
                 ↓
              terrain
```

- `core` has no internal dependencies
- `ecs` depends on `core`
- `physics` depends on `core` and `ecs`
- `render` depends on `core` and `ecs`
- `atmosphere` depends on `core` and `render`
- `terrain` depends on `core`
- `app` depends on everything

**Rule:** Never create circular dependencies. If you need data from a higher-level crate, use traits defined in `core`.

### ECS Principles

- Components are data only. No methods on components. No logic in component definitions.
- Systems are logic only. No stored state (use Resources for that).
- Resources are singletons: `TimeState`, `CameraState`, `RenderConfig`.
- System ordering matters: `Physics → Transform → Render`.
- Use system sets to group related systems (PhysicsSet, RenderSet, UiSet).

### Data Types

- `f64` for physics accuracy (positions, velocities, masses, orbital elements)
- `f32` for rendering (vertex positions, colors, shader math)
- Explicit conversions between precision (`pos_f32 = position as f32`)
- Use `glam::DVec3` for physics, `glam::Vec3` for rendering

### Floating-Origin

- All positions are relative to the camera/focus point
- The "origin" shifts to keep the active object near (0,0,0)
- Prevents floating-point precision loss at astronomical distances
- Convert to absolute coordinates only for external data (SPICE queries)

### Determinism

- Seeded RNG (ChaCha) for any random process
- Fixed timestep for physics (1/60s base, adjustable)
- Same seed + same inputs = same universe, always
- Document any sources of non-determinism (timing, OS calls)

### GPU-Friendly Data Layout

- SoA (Structure of Arrays) for batched rendering
- Packed vertex formats (don't waste GPU memory)
- Uniform buffers aligned to 16-byte boundaries (WebGPU requirement)
- Minimize CPU→GPU transfers per frame (double-buffer)

---

## Performance Rules

1. **Profile before optimizing.** Use Tracy or criterion to find the actual bottleneck.
2. **Avoid per-frame allocations.** Use object pools for particles, temporary vectors, etc.
3. **Batch draw calls by material/shader.** Don't switch pipelines mid-frame.
4. **Use rayon for CPU parallel work.** Parallel iterators are almost free.
5. **GPU compute for large-N simulations.** Barnes-Hut on GPU for 10K+ bodies.
6. **LOD: don't render what you can't see.** Distance-based mesh/detail reduction.
7. **Frustum cull everything.** Don't send invisible objects to the GPU.
8. **Use instancing for repeated geometry.** Asteroids, stars, particles — instanced.
9. **Minimize CPU→GPU transfers per frame.** Double-buffer uniforms, use staging buffers.
10. **Use double-buffered uniforms.** GPU reads buffer A while CPU writes buffer B.

---

## Simulation Rules

### Timestep

- Fixed timestep: 1/60s (16.67ms) base, configurable
- Accumulator pattern for variable frame rates:
  ```
  accumulator += dt;
  while accumulator >= FIXED_DT {
      physics_step(FIXED_DT);
      accumulator -= FIXED_DT;
  }
  ```
- Rendering interpolates between physics steps for smooth visuals

### Determinism

- Seeded RNG: ChaCha8 or ChaCha12, seeded with universe seed
- All random processes use the seeded RNG
- Document any sources of non-determinism

### Gravity

- Gravitational softening: prevent F → ∞ as r → 0
- Softening length ε: `F = G * m1 * m2 / (r² + ε²)`
- Tune ε based on minimum meaningful distance
- Energy conservation checks in long integrations (monitor total energy drift)

### Collision Detection

- Hierarchical bounding volumes (sphere → AABB → mesh)
- Broad phase: spatial hashing or BVH
- Narrow phase: GJK or SAT for precise contact
- For large bodies: treat as point masses (gravity) + visual collision only

### Validation

- Always test with known solutions (Keplerian orbits)
- Compare simulation output to JPL Horizons ephemeris
- Monitor energy conservation over long integrations
- Test edge cases: zero mass, coincident positions, extreme eccentricities

---

## Rendering Rules

### PBR

- PBR everywhere possible (metallic-roughness workflow)
- BRDF: Cook-Torrance with GGX normal distribution
- Energy conservation: diffuse + specular ≤ 1
- Fresnel: Schlick approximation for real-time

### HDR

- HDR rendering pipeline (render to float textures)
- Tone mapping: ACES (default) or AgX (more neutral)
- Bloom on bright objects (stars, explosions, specular highlights)
- Auto-exposure optional (eye adaptation)

### Atmosphere

- Atmospheric scattering for any body with atmosphere
- Precomputed LUTs for performance (Bruneton/Hillaire model)
- Multiple scattering approximation
- Planet-specific parameters (Rayleigh coefficients, Mie coefficients, density profile)

### Shadows

- Shadow maps for directional lights (sun)
- Cascaded shadow maps for large scenes
- Optional: ray-marched shadows for atmospheric bodies

### Depth

- Reverse-Z for better precision near the camera
- Depth pre-pass for complex scenes
- Early-Z where supported

### Instancing

- Instanced rendering for repeated geometry (asteroids, stars, particles)
- GPU-driven instancing (instance buffers updated from compute)

### Batching

- Batch by material/shader, minimize state changes
- Sort opaque front-to-back, transparent back-to-front
- Use indirect drawing where possible

---

## Future Ideas

- [ ] Compute shader N-body (Barnes-Hut on GPU)
- [ ] Sparse voxel octree for terrain
- [ ] Procedural sound generation (gravitational waves → audio)
- [ ] VR support via OpenXR
- [ ] Network multiplayer (lockstep deterministic)
- [ ] Plugin API for community extensions
- [ ] Real-time weather simulation on terraformed bodies
- [ ] Archaeological civilization simulation (empires rise/fall on planets)
- [ ] Relativistic simulation (special/general relativity for extreme environments)
- [ ] Gravitational wave visualization
- [ ] Time dilation visualization near black holes
- [ ] Interstellar medium simulation (gas, dust, magnetic fields)
- [ ] Exoplanet transit detection (light curve generation)
- [ ] Solar flare / coronal mass ejection simulation

---

## Research Notes

### Orbital Mechanics
- Kepler problem: https://en.wikipedia.org/wiki/Kepler_orbit
- Orbital elements: https://en.wikipedia.org/wiki/Orbital_elements
- Two-body problem: https://en.wikipedia.org/wiki/Two-body_problem
- Three-body problem: https://en.wikipedia.org/wiki/Three-body_problem
- Lagrange points: https://en.wikipedia.org/wiki/Lagrange_point

### N-Body Algorithms
- Barnes-Hut 1986: https://doi.org/10.1038/324446a0
- Fast Multipole Method: https://doi.org/10.1016/0021-9991(87)90168-7
- Kustaanheimo-Stiefel regularization: https://en.wikipedia.org/wiki/Kustaanheimo%E2%80%93Stiefel_transformation
- REBOUND N-body code: https://github.com/hannorein/rebound

### Atmospheric Scattering
- Bruneton & Neyret 2008: https://hal.archives-ouvertes.fr/hal-00288782
- Hillaire 2020: https://ebruneton.github.io/precomputed_atmospheric_scattering/
- O'Neil 2004: Real-time atmospheric scattering
- Nishita 1993: Ray marching approach

### Terrain Generation
- Diamond-Square: https://en.wikipedia.org/wiki/Diamond-square_algorithm
- Simplex noise: https://en.wikipedia.org/wiki/Simplex_noise
- Erosion simulation: Hydraulic erosion algorithms
- Perlin noise: https://mrl.nyu.edu/~perlin/doc/...

### PBR Reference
- Real Shading in Unreal Engine 4: Epic Games SIGGRAPH 2013
- PBR Book: https://pbr-book.org/
- Filament: https://google.github.io/filament/Filament.html
- Disney BRDF: https://media.disneyanimation.com/uploads/asset/asset/22/Disney_BRDF.pdf

### Scientific Data
- JPL Horizons: https://ssd.jpl.nasa.gov/horizons/
- JPL Solar System Dynamics: https://ssd.jpl.nasa.gov/
- SPICE toolkit: https://naif.jpl.nasa.gov/naif/toolkit.html
- IAU standards: https://www.iausofa.org/
- NASA planetary fact sheets: https://nssdc.gsfc.nasa.gov/planetary/factsheet/

---

## Glossary

- **N-body**: gravitational simulation of N objects all affecting each other
- **Keplerian orbit**: analytical two-body orbital solution
- **Ephemeris**: table of positions/velocities over time
- **Floating-origin**: camera-relative coordinate system for huge scales
- **SoA vs AoS**: Structure of Arrays vs Array of Structures (GPU prefers SoA)
- **LOD**: Level of Detail — reduce complexity for distant objects
- **PBR**: Physically Based Rendering
- **TAA**: Temporal Anti-Aliasing
- **HDR**: High Dynamic Range
- **ECS**: Entity Component System
- **BRDF**: Bidirectional Reflectance Distribution Function
- **Rayleigh scattering**: wavelength-dependent scattering by small particles (∝ λ⁻⁴)
- **Mie scattering**: wavelength-independent scattering by larger particles
- **Hohmann transfer**: minimum-energy two-impulse orbital transfer
- **Lagrange points**: 5 equilibrium points in a two-body orbital system
- **Barnes-Hut**: O(N log N) tree-based gravitational algorithm
- **Symplectic integrator**: preserves phase space volume (energy stable)
- **Softening length**: prevents gravitational force from diverging at r→0
- **Orbital elements**: 6 parameters defining an orbit (a, e, i, Ω, ω, Mν)
- **AU**: Astronomical Unit (~149.6 million km)
- **SPICE**: NASA's information system for solar system geometry
- **KS regularization**: Kustaanheimo-Stiefel transformation, converts Kepler problem to harmonic oscillator
- **Mean anomaly (M)**: fraction of orbital period elapsed since periapsis
- **True anomaly (ν)**: angle between periapsis and current position
- **Eccentric anomaly (E)**: auxiliary angle for solving Kepler's equation
- **RAAN (Ω)**: Right Ascension of the Ascending Node — orientation of orbital plane
- **Argument of periapsis (ω)**: orientation of orbit within its plane
- **Inclination (i)**: tilt of orbital plane relative to reference plane
- **Semi-major axis (a)**: half the longest diameter of an ellipse
- **Eccentricity (e)**: how elongated an orbit is (0 = circle, 1 = parabola)
- **Vis-viva equation**: v² = GM(2/r - 1/a) — relates speed to position in orbit
- **Tisserand parameter**: conserved quantity in restricted three-body problem
- **Roche limit**: distance within which tidal forces tear a body apart
- **Hill sphere**: region where a body dominates gravitational attraction
- **Sphere of influence**: where one body's gravity dominates over another's
- **Barycenter**: center of mass of a system of bodies
- **Precession**: gradual rotation of orbital orientation over time
- **Libration**: oscillation about a Lagrange point
- **Resonance**: when orbital periods are commensurate (e.g., 2:1 resonance)
- **Perturbation**: deviation from ideal two-body orbit due to other influences
- **Jacobi integral**: conserved quantity in circular restricted three-body problem
- **Lyapunov time**: timescale over which chaotic systems become unpredictable

---

## Decisions Log

> Track architectural and design decisions here. Include the date, decision, rationale,
> and alternatives considered.

| Date | Decision | Rationale |
|------|----------|-----------|
| 2026-07-20 | Use wgpu as rendering backend | Cross-platform, Rust-native, WebAssembly support |
| 2026-07-20 | Use bevy_ecs for ECS | Well-maintained, fast, good ergonomics |
| 2026-07-20 | f64 for physics, f32 for rendering | Physics needs precision, rendering needs performance |
| 2026-07-20 | 7-crate workspace | Clear separation of concerns, compilation parallelism |
| 2026-07-20 | WGSL for shaders | wgpu-native, no GLSL/SPIR-V dependency |

---

> **Last updated:** 2026-07-20
> **Maintained by:** The Cosmogon team
> **Status:** Living document — update as the project evolves
