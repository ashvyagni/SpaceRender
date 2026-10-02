# Cosmogon Learning Roadmap

> You're building a real-time digital universe simulator. That's not a weekend project — it's a
> journey through some of the deepest CS and physics territory there is. This roadmap gives you
> the sequence, the resources, and the priorities so you're not drowning in irrelevant theory.
> Follow the order. Each stage builds on the last.

---

## 1. Rust Fundamentals

**What to learn:** Ownership, borrowing, lifetimes, traits, generics, error handling (`Result`/`Option`), modules, enums, pattern matching, Cargo workspace structure.

**Why it matters:** Cosmogon is written in Rust. If you don't have a strong grasp of ownership and lifetimes, you'll fight the compiler every step of the way instead of building things. Traits are how you'll define physics components, graphics interfaces, and system behaviors. Cargo workspaces are how you'll structure a multi-crate project.

**Resources:**
- *The Rust Programming Language* (Klabnik & Nichols) — the book, read all of it
- *Rust By Example* — for quick "how do I do X" reference
- *Programming Rust* (Blandy, Orendorff, Tindall) — deeper dive, great chapter on unsafe
- [Rustlings](https://github.com/rust-lang/rustlings) — interactive exercises
- [Exercism Rust Track](https://exercism.org/tracks/rust) — practice problems
- [std docs](https://doc.rust-lang.org/std/) — read it like a novel

**Estimated time:** 2–4 weeks (if coming from another language), 1–2 months (if new to systems programming)

**Priority:** Critical

---

## 2. Linear Algebra

**What to learn:** Vectors (2D/3D/4D), matrix multiplication, transformation matrices (translate, rotate, scale), quaternions for rotation, projections (orthographic, perspective), dot/cross products, vector normalization, homogeneous coordinates.

**Why it matters:** Every position in the universe is a vector. Every orbit is a matrix transformation. Every camera view is a projection. Quaternions are how you'll avoid gimbal lock when rotating objects. You literally cannot build Cosmogon without this.

**Resources:**
- *3D Math Primer for Graphics and Game Development* (Dunn & Parberry) — THE book for game math
- *Linear Algebra and Its Applications* (Strang) — deeper theory if you want it
- [3Blue1Brown Essence of Linear Algebra](https://www.youtube.com/playlist?list=PLZHQObOWTQDPD3MizzM2xVFitgF8hE_ab) — visual intuition, incredible
- [The Matrix and Quaternions FAQ](http://www.euclideanspace.com/maths/geometry/rotations/) — reference
- [glam](https://github.com/bitshifter/glam-rs) — the Rust math lib Cosmogon should use, read its source
- [nalgebra](https://nalgebra.org/) — heavier but more complete, good for learning

**Estimated time:** 3–4 weeks

**Priority:** Critical

---

## 3. Coordinate Systems

**What to learn:** Cartesian (x,y,z), spherical coordinates, ecliptic coordinates, equatorial coordinates, coordinate transformations between systems, rotation matrices vs quaternion rotations, the ecliptic plane, axial tilts, precession.

**Why it matters:** The solar system isn't cartesian. Planets orbit in the ecliptic plane, telescopes use equatorial coordinates, and you'll need to convert between all of these constantly. If your coordinate systems are wrong, nothing will look right — planets will orbit at wrong angles, star positions will be off, and your skybox won't match reality.

**Resources:**
- *Astronomical Algorithms* (Meeus) — the bible of coordinate conversions
- [IAU Standards](https://www.iausofa.org/) — formal definitions
- [SPICE/SOFT](https://naif.jpl.nasa.gov/naif/toolkit.html) — NASA's coordinate system toolkit
- [Astropy](https://www.astropy.org/) — Python reference for understanding transformations
- [SOFA](http://www.iausofa.org/) — IAU's standard of fundamental astronomy (C implementation, reference)

**Estimated time:** 2–3 weeks

**Priority:** Critical

---

## 4. Numerical Methods

**What to learn:** Floating point arithmetic (IEEE 754), precision issues, error accumulation, numerical integration (Euler, Verlet, RK4, symplectic integrators), adaptive timestep, series expansion vs iterative solvers, root finding, interpolation (spline, Hermite).

**Why it matters:** You're simulating physics over millions of years of virtual time. Naive Euler integration will cause orbits to spiral inward or outward. Floating point errors compound. You need to understand *why* your simulation is wrong before you can fix it. Symplectic integrators preserve energy; RK4 gives high accuracy. Your choice of integrator determines whether your universe is stable.

**Resources:**
- *Numerical Recipes* (Press et al.) — classic reference, the C version is fine
- *An Introduction to the Numerical Integration of Ordinary Differential Equations* (Practical series)
- [Wikipedia: Symplectic integrator](https://en.wikipedia.org/wiki/Symplectic_integrator) — great overview
- [Gpg4us N-body notes](https://ui.adsabs.harvard.edu/abs/2008gpgd.book.....A/abstract) — practical guide
- [scipy.integrate](https://docs.scipy.org/doc/scipy/reference/integrate.html) — reference implementations
- [HWClock Blog: Verlet Integration](https://hplgit.github.io/fdm-book/doc/pub/._fdm004_decay002.html) — visual explanation

**Estimated time:** 3–4 weeks

**Priority:** Critical

---

## 5. Orbital Mechanics

**What to learn:** Kepler's laws, orbital elements (semi-major axis, eccentricity, inclination, RAAN, argument of perihelion, true anomaly), two-body problem, Kepler's equation (solve iteratively), orbital propagation, Hohmann transfers, Lagrange points, perturbations.

**Why it matters:** This is the heart of Cosmogon. Every planet, moon, asteroid, and comet follows orbital mechanics. You need to convert orbital elements to positions/velocities, propagate orbits forward in time, and understand multi-body interactions. Kepler's equation requires numerical solving — there's no closed-form solution.

**Resources:**
- *Fundamentals of Astrodynamics* (Bate, Mueller, White) — the standard textbook
- *Orbital Mechanics for Engineering Students* (Curtis) — more modern, great examples
- [JPL Horizons](https://ssd.jpl.nasa.gov/horizons/) — real ephemeris data for validation
- [Kepler Problem Wikipedia](https://en.wikipedia.org/wiki/Kepler_problem) — theory reference
- [astropy.coordinates](https://docs.astropy.org/en/stable/coordinates/) — Python implementation to reference
- [OpenOrb](https://github.com/samuelsmulko/OpenOrb) — open-source orbital propagation library

**Estimated time:** 4–6 weeks

**Priority:** Critical

---

## 6. N-Body Physics

**What to learn:** Gravitational N-body problem, direct summation (O(N²)), Barnes-Hut tree algorithm (O(N log N)), Fast Multipole Method, gravitational regularization (Kustaanheimo-Stiefel, Algorithmic Regularization), symplectic integrators for N-body, chaos sensitivity, energy conservation, chaos in the solar system.

**Why it matters:** Kepler's laws only work for two bodies. Real solar systems are N-body problems. You need algorithms that scale — direct O(N²) is fine for 100 bodies, but not for 10,000 asteroids. Regularization prevents numerical explosions during close encounters. The solar system is chaotic; you need to understand Lyapunov timescales.

**Resources:**
- *Gravitational N-Body Simulations* (Aarseth) — THE reference book
- [Barnes-Hut original paper](https://doi.org/10.1038/324446a0) — foundational
- [REBOUND](https://github.com/hannorein/rebound) — modern N-body code, read the source
- [Swift](https://hwww.astro. Indiana.edu/~swift/) — NASA's N-body integrator
- [Mikkola & Aarseth 2002](https://doi.org/10.1086/342699) — time-transformed leapfrog
- [Chenciner & Montgomery 2000](https://doi.org/10.1006/jfan.2000.3593) — dynamics of the three-body problem

**Estimated time:** 4–6 weeks

**Priority:** Critical

---

## 7. Computer Graphics Fundamentals

**What to learn:** Rasterization pipeline, vertex/fragment shaders, Model-View-Projection matrices, depth buffering, backface culling, texture mapping, UV coordinates, lighting models (Phong, Blinn-Phong), gamma correction, color spaces (sRGB, linear), framebuffers, render passes.

**Why it matters:** You need to see the universe you're simulating. Graphics fundamentals are how you turn mathematical positions into pixels on screen. MVP matrices connect your 3D simulation to 2D output. Lighting models make things look real. Understanding the pipeline prevents mystery bugs where nothing renders.

**Resources:**
- *Fundamentals of Computer Graphics* (Marschner & Shirley) — excellent textbook
- *Real-Time Rendering* (Akenine-Möller et al.) — the reference
- [LearnOpenGL](https://learnopengl.com/) — best free tutorial, concepts transfer to wgpu
- [gpuopen.com](https://gpuopen.com/learnings/) — AMD's graphics knowledge base
- [The Graphics Codex](https://graphicscodex.com/) — reference tool
- [Khronos Vulkan Tutorials](https://vulkan-tutorial.com/) — for understanding low-level concepts

**Estimated time:** 3–4 weeks

**Priority:** Critical

---

## 8. wgpu

**What to learn:** WebGPU API concepts, device/queue, render passes, compute passes, pipeline state objects, vertex/index buffers, uniform buffers, textures/samplers, bind groups, shader modules, swap chain configuration, surface capabilities.

**Why it matters:** wgpu is your rendering backend. It's Rust-native, cross-platform (including WebAssembly), and modern. You need to understand how to create pipelines, bind data, and issue draw calls. wgpu abstracts over Vulkan/Metal/DX12/WebGPU — so you get portability without vendor lock-in.

**Resources:**
- [wgpu official tutorial](https://sotrh.github.io/learn-wgpu/) — THE resource, follow it end-to-end
- [wgpu examples](https://github.com/gfx-rs/wgpu/tree/trunk/examples) — read every example
- [WebGPU specification](https://www.w3.org/TR/webgpu/) — the spec itself
- [gfx-rs wiki](https://github.com/gfx-rs/wgpu/wiki) — internal design docs
- [wgpu-rs docs.rs](https://docs.rs/wgpu/) — API reference
- [Amanjeet's wgpu blog](https://cwbrzeski.com/) — practical tutorials

**Estimated time:** 3–4 weeks

**Priority:** Critical

---

## 9. WGSL Shaders

**What to learn:** WGSL syntax, vertex shaders (entry points, builtin variables), fragment shaders, compute shaders, uniforms/storage buffers, textures and samplers, math functions, struct definitions, workgroup dispatch, barriers, atomic operations.

**Why it matters:** Shaders are where computation meets rendering. Your atmospheric scattering, particle effects, and GPU-accelerated N-body all run as WGSL. Understanding shader math (especially matrix ops and texture sampling) is non-negotiable. Compute shaders are how you'll parallelize heavy simulation work.

**Resources:**
- [WGSL specification](https://www.w3.org/TR/WGSL/) — the spec
- [wgpu shader examples](https://github.com/gfx-rs/wgpu/tree/trunk/examples/src/shader) — practical examples
- [GPU Gems](https://developer.nvidia.com/gpugems/gpugems/contributors) — classic GPU programming chapters
- [The Book of Shaders](https://thebookofshaders.com/) — GLSL-focused but concepts transfer
- [WebGPU Fundamentals](https://webgpufundamentals.org/) — excellent visual tutorial
- [shader playground](https://shader-playground.tinkerersguild.org/) — test snippets

**Estimated time:** 2–3 weeks

**Priority:** Critical

---

## 10. Physically Based Rendering

**What to learn:** BRDF (Cook-Torrance, GGX), metallic-roughness workflow, energy conservation, Fresnel equations, normal mapping, ambient occlusion, HDR rendering, tone mapping (ACES, AgX, Reinhard), bloom, image-based lighting (IBL).

**Why it matters:** Cosmogon should *look* real, not like a 2005 demo. PBR makes materials respond to light like real surfaces. Planets, asteroids, and moons need proper BRDFs. HDR rendering captures the extreme brightness range of space (dim nebula vs. blazing star). Without tone mapping, everything would be washed out or invisible.

**Resources:**
- [Real Shading in Unreal Engine 4](https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentations NOTES-170220998.pdf) — Epic's PBR reference
- [PBR Book](https://pbr-book.org/) — physically based rendering theory (online, free)
- [Filament Material Guide](https://google.github.io/filament/Filament.html) — Google's PBR engine docs
- [HDR Rendering with wgpu](https://medium.com/@otukofurukawa/hdr-rendering-with-webgpu-e8c922dc0a08) — practical tutorial
- [ACES tone mapping](https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/) — reference implementation
- [LearnOpenGL PBR](https://learnopengl.com/PBR/Lighting) — great walkthrough

**Estimated time:** 3–4 weeks

**Priority:** Critical

---

## 11. ECS Architecture

**What to learn:** Entity-Component-System pattern, entities as IDs, components as pure data, systems as pure logic, queries/filters, system ordering, system sets, exclusive systems, resources (singletons), events, system sets for scheduling, archetype-based storage.

**Why it matters:** ECS is how Cosmogon organizes complexity. A planet has Position, Velocity, Mass, RenderMesh, Atmosphere — those are components. Updating positions, rendering, and physics are systems. ECS gives you data locality (cache-friendly), compositional design (no deep inheritance), and natural parallelism (systems on different components don't conflict).

**Resources:**
- [Entity Component System (ecs-rs)](https://specs.rs/) — Rust ECS ecosystem overview
- [Bevy ECS docs](https://bevyengine.org/learn/becs/) — excellent, even if you don't use Bevy
- [flecs](https://github.com/SanderMertens/flecs) — C ECS, great design reference
- [EnTT](https://github.com/skypjack/entt) — C++ ECS, well-documented architecture
- [ECS FAQ](https://github.com/SanderMertens/ecs-faq) — answers common questions
- [Evolve your Hierarchy](https://ajmmertens.medium.com/developing-games-with-ecs-architecture-62d25cb3027a) — why ECS over OOP

**Estimated time:** 2–3 weeks

**Priority:** Critical

---

## 12. bevy_ecs

**What to learn:** `Component` derive, `Resource` derive, `SystemParam`, `Query` (filters, `With`/`Without`/`Changed`), `Res`/`ResMut`, system ordering (`before`/`after`/`in_set`), `Startup`/`Update` schedules, `Commands` for entity spawning, `EventReader`/`EventWriter`, `Local` variables, `App` builder pattern, `States`, `Schedule`.

**Why it matters:** bevy_ecs is your ECS implementation. It's fast, well-maintained, and designed for games. You need to know how to query for "all entities with Position and Velocity but without StaticBody," how to order systems so physics runs before rendering, and how to use resources for global state like `TimeState`.

**Resources:**
- [Bevy ECS cheatsheet](https://github.com/jakobhellermann/bevy-cheatsheet) — quick reference
- [Bevy Cheatbook](https://bevy-cheatbook.github.io/) — comprehensive guide
- [Bevy examples](https://github.com/bevyengine/bevy/tree/main/examples) — read system-related examples
- [Bevy Discord](https://discord.gg/bevy) — active community help
- [bevy_ecs source](https://github.com/bevyengine/bevy/tree/main/crates/bevy_ecs) — read the code
- [Bevy 0.14 release notes](https://bevyengine.org/news/bevy-0-14/) — latest ECS improvements

**Estimated time:** 2–3 weeks

**Priority:** Critical

---

## 13. GPU Programming

**What to learn:** Compute shaders, workgroups, dispatch sizes, storage buffers, read-only buffers, atomic operations, shared memory, thread synchronization (barriers), parallel reduction patterns, GPU vs CPU tradeoffs, memory coalescing, occupancy.

**Why it matters:** N-body gravity with 10,000+ objects needs GPU acceleration. Direct summation on CPU is O(N²) and slow; on GPU, you can parallelize across thousands of cores. Compute shaders let you do physics simulation, not just rendering. Understanding workgroup sizes and memory access patterns is critical for performance.

**Resources:**
- [GPU Gems 2 Chapter 31](https://developer.nvidia.com/gpugems2/part-v-image-processing/chapter-31-parallel-prefix-sum-scan-cuda) — parallel scan on GPU
- [NVIDIA CUDA Programming Guide](https://docs.nvidia.com/cuda/cuda-c-programming-guide/) — CUDA but concepts transfer
- [GPU Performance for Game Artists](https://developer.arm.com/documentation/101897/latest) — memory/coalescing intuition
- [WebGPU Compute Shaders](https://webgpufundamentals.org/webgpu/lessons/webgpu-compute-shaders.html) — practical tutorial
- [wgpu compute examples](https://github.com/gfx-rs/wgpu/tree/trunk/examples/src/compute) — code examples
- [GDC: GPU-Accelerated Physics](https://www.gdcvault.com/) — industry talks

**Estimated time:** 3–4 weeks

**Priority:** Important

---

## 14. Atmospheric Scattering

**What to learn:** Rayleigh scattering (wavelength-dependent, λ⁻⁴), Mie scattering (wavelength-independent, forward-peaked), optical depth, ray marching through atmosphere, precomputed LUTs (lookup tables), Bruneton's model, Hillaire 2020 improved model, multiple scattering approximation, view-transmittance LUT.

**Why it matters:** Planets with atmospheres need to look right. Earth's blue sky and red sunsets come from Rayleigh scattering. Mars has a thin, dusty atmosphere (Mie dominant). Gas giants have deep, layered atmospheres. Without proper scattering, your planets look like billiard balls. This is one of the hardest visual features to get right.

**Resources:**
- [Hillaire 2020: A Scalable and Production Ready Model](https://ebruneton.github.io/precomputed_atmospheric_scattering/) — modern reference
- [Bruneton & Neyret 2008](https://hal.archives-ouvertes.fr/hal-00288782/document) — foundational paper
- [O'Neil 2004: Real-Time Atmospheric Scattering](https://www.gamedevs.org/uploads/real-time-rendering-3.pdf) — practical approach
- [Nishita 1993](https://doi.org/10.1111/1467-8659.1220159) — original ray marching approach
- [Atmospheric Scattering Demo](https://github.com/ebruneton/demo-atmosphere) — reference implementation
- [Scratchapixel: Atmospheric Scattering](https://www.scratchapixel.com/lessons/procedural-generation-virtual-worlds/simulating-sky/sky-lighting-introduction.html) — visual explanation

**Estimated time:** 4–6 weeks

**Priority:** Important

---

## 15. Scientific Visualization

**What to learn:** Data mapping (scalar → color), scientific colormaps (viridis, inferno, turbo), volume rendering (ray marching), isosurface extraction (Marching Cubes), glyph visualization (vectors, tensors), transfer functions, direct vs. indirect volume rendering, data reduction techniques.

**Why it matters:** You need to show simulation data — gravitational fields, temperature maps, orbital energy, particle density. Raw numbers are useless; visualization makes patterns visible. Colormaps must be perceptually uniform. Volume rendering lets you show gas clouds and nebulae. This separates a "demo" from a scientific tool.

**Resources:**
- *Scientific Visualization: A Gentle Introduction* (Laramee & Cosker) — accessible overview
- [Matplotlib colormaps](https://matplotlib.org/stable/gallery/color/colormap_reference.html) — standard scientific colormaps
- [Paraview](https://www.paraview.org/) — reference scientific visualization tool
- [Marching Cubes algorithm](https://en.wikipedia.org/wiki/Marching_cubes) — foundational
- [Volume Rendering Integral](https://developer.nvidia.com/gpugems/gpugems/part-iv-image-processing/chapter-39-volume-rendering-techniques) — GPU implementation
- [CMU 15-462 Computer Graphics](https://www.cs.cmu.edu/~15462/) — university course, free

**Estimated time:** 3–4 weeks

**Priority:** Important

---

## 16. Performance Optimization

**What to learn:** Profiling tools ( Tracy, perf, InstrUMENT), CPU cache behavior (cache lines, prefetching), SIMD (SSE/AVX), data-oriented design (SoA vs AoS), batch rendering, instancing, async Rust, thread pools (rayon), frame pacing, draw call batching, GPU profiling.

**Why it matters:** A universe simulator with millions of objects must be fast. Profiling tells you where the real bottleneck is (it's never where you think). SIMD can 4x your physics calculations. SoA layout makes GPU uploads fast. Batching draw calls reduces API overhead. Without optimization, you'll hit 20 FPS with 1,000 objects.

**Resources:**
- *Optimizing Software in C++* (Agner Fog) — principles transfer to Rust
- [Tracy Profiler](https://github.com/wolfpld/tracy) — excellent Rust integration
- [criterion.rs](https://github.com/bheisler/criterion.rs) — Rust benchmarking
- [rayon docs](https://docs.rs/rayon/) — parallel iterators
- [perf](https://perf.wiki.kernel.org/) — Linux profiler (works on macOS with Instruments)
- [Game Engine Architecture (Gregory)](https://www.gameenginebook.com/) — performance chapters
- [Data-Oriented Design (Acton)](https://.dataorienteddesign.com/) — the philosophy

**Estimated time:** 3–4 weeks

**Priority:** Important

---

## 17. Procedural Generation

**What to learn:** Noise functions (Perlin, Simplex, Worley), fractal Brownian motion (fBm), domain warping, diamond-square algorithm, erosion simulation, L-systems for vegetation, Poisson disk sampling, seeded RNG for determinism, texture synthesis.

**Why it matters:** You can't hand-model every asteroid, moon surface, and nebula. Procedural generation creates infinite variety from compact descriptions. Noise-based terrain for rocky bodies, domain-warped patterns for gas giant cloud bands, L-systems for alien vegetation. Seeded RNG ensures the same universe every time you load it.

**Resources:**
- [Procedural Generation in Game Design](https://www.amazon.com/Procedural-Generation-Game-Design-Tina-Seabrooks/dp/1498799191) — book
- [thebookofshaders.com/05](https://thebookofshaders.com/05/) — noise functions explained visually
- [Stefan Gustavson's noise papers](https://stefan.gustavsson.se/papers/simplex-noise.pdf) — Simplex noise reference
- [LibNoise](https://libnoise.sourceforge.net/) — reference library
- [World Machine](https://www.world-machine.com/) — terrain generation tool (reference)
- [Red Blob Games: Noise](https://www.redblobgames.com/maps/terrain-from-noise/) — excellent visual guide

**Estimated time:** 2–3 weeks

**Priority:** Nice-to-have

---

## 18. Simulation Architecture

**What to learn:** Fixed timestep with accumulator, determinism in simulation, state serialization/deserialization, checkpointing, rollback networking concepts, simulation vs. rendering decoupling, reproducible RNG, state machine design, event sourcing for simulation.

**Why it matters:** A universe simulator must be reliable. Fixed timestep ensures physics doesn't break when frame rate varies. Determinism means the same seed gives the same universe. Serialization lets you save/load simulations. Decoupling physics from rendering means you can fast-forward time without breaking visuals. This is what makes Cosmogon a *tool*, not just a demo.

**Resources:**
- [Fix Your Timestep!](https://gafferongames.com/post/fix_your_timestep/) — Glenn Fiedler's classic
- [Game Programming Patterns (Nystrom)](https://gameprogrammingpatterns.com/) — state, command, update patterns
- [Valve Developer Wiki: Source Multiplayer Networking](https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking) — rollback concepts
- [Entity Component System](https://github.com/SanderMertens/ecs-faq) — ECS + determinism
- [Serde](https://serde.rs/) — Rust serialization
- [Bevy States](https://bevy-cheatbook.github.io/programming/states.html) — state management reference

**Estimated time:** 2–3 weeks

**Priority:** Important

---

## Summary

| # | Topic | Priority | Estimated Time |
|---|-------|----------|----------------|
| 1 | Rust Fundamentals | Critical | 2–4 weeks |
| 2 | Linear Algebra | Critical | 3–4 weeks |
| 3 | Coordinate Systems | Critical | 2–3 weeks |
| 4 | Numerical Methods | Critical | 3–4 weeks |
| 5 | Orbital Mechanics | Critical | 4–6 weeks |
| 6 | N-Body Physics | Critical | 4–6 weeks |
| 7 | Computer Graphics Fundamentals | Critical | 3–4 weeks |
| 8 | wgpu | Critical | 3–4 weeks |
| 9 | WGSL Shaders | Critical | 2–3 weeks |
| 10 | Physically Based Rendering | Critical | 3–4 weeks |
| 11 | ECS Architecture | Critical | 2–3 weeks |
| 12 | bevy_ecs | Critical | 2–3 weeks |
| 13 | GPU Programming | Important | 3–4 weeks |
| 14 | Atmospheric Scattering | Important | 4–6 weeks |
| 15 | Scientific Visualization | Important | 3–4 weeks |
| 16 | Performance Optimization | Important | 3–4 weeks |
| 17 | Procedural Generation | Nice-to-have | 2–3 weeks |
| 18 | Simulation Architecture | Important | 2–3 weeks |

**Total estimated time:** ~8–12 months of focused learning (part-time), ~5–7 months full-time.

---

> **Remember:** You don't need to master everything before you start building. The roadmap is a guide,
> not a gate. Learn just enough to build the next feature, then iterate. The universe won't build
> itself... actually wait, in this case it literally will.
