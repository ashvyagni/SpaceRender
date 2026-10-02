# Cosmogon — Multi-Year Development Roadmap

> Real-time digital universe simulator. From a spinning sphere to a living cosmos.

This roadmap is ambitious. That's the point. Each version ships concrete, testable deliverables — no vague "improve performance" goals. If it's in here, we build it and we ship it.

---

## v0.1 — First Light (Months 1–3)

> The moment a triangle appears on screen and you realize this is real.

**Milestone: Bare-bones rendering pipeline.**

- Cargo workspace with 7 crates scaffolded
- wgpu initialization + window creation
- Basic sphere mesh generation (UV sphere)
- Planet rendering with solid colors
- Sun as emissive sphere
- Basic camera (orbit camera around sun)
- Simple circular orbits (hardcoded)
- egui integration with basic panels
- Time display

**Exit Criteria:** Open the app, see a yellow sun with 8 colored spheres orbiting it. Drag to rotate the camera. Time ticks forward.

---

## v0.2 — Solar System (Months 4–6)

> Real orbits. Real numbers. The solar system as it actually is.

**Milestone: Scientifically accurate orbital mechanics for all major bodies.**

- Real Keplerian orbital elements for all 8 planets
- Accurate orbital periods, eccentricities, inclinations
- Planet textures (basic procedural or simple color)
- Moon orbits (Earth's Moon, Galilean moons)
- Asteroid belt as particle field
- Time acceleration controls (1x to 1000x)
- Camera focus on any body
- Object info panel (name, mass, distance, velocity)
- Starfield background

**Exit Criteria:** Pause at any date, verify Mars is at its real position. Zoom into Jupiter, see the four Galilean moons orbiting.

---

## v0.3 — Living Worlds (Months 7–10)

> Planets stop being billboards and start being places.

**Milestone: Visual realism for solar system bodies.**

- Atmospheric scattering (Rayleigh + Mie) for Earth and Venus
- Procedural terrain on Earth, Mars, Moon
- Ocean rendering on Earth (simple water shader)
- Cloud layer on Earth
- Planet rotation with correct periods
- Axial tilt visualization
- Seasonal color variations
- Shadow rendering (planets cast shadows)
- Eclipse detection and visualization

**Exit Criteria:** Earth has a blue atmosphere halo, visible oceans, and cloud cover. Watch a lunar eclipse happen in real time.

---

## v0.4 — Deep Space (Months 11–14)

> Zoom out far enough and the solar system is a pixel. Now fill the void.

**Milestone: Interstellar and galactic rendering.**

- Procedural galaxy generation (spiral, elliptical, irregular)
- Starfield from real catalog data (Hipparcos subset)
- Nebula rendering (raymarched volumetric)
- Procedural star systems around other stars
- Variable star brightness
- Stellar classification colors (O B A F G K M)
- Cosmic distance scale markers
- Search and navigation to any star

**Exit Criteria:** Navigate from Earth to Proxima Centauri. See it resolve from a dot to a dim red dwarf. Search for Sirius, teleport there.

---

## v0.5 — N-Body (Months 15–18)

> Kepler was an approximation. Newton is the real deal.

**Milestone: Full gravitational simulation with thousands of bodies.**

- N-body gravitational simulation
- RK4 integrator with adaptive step
- Lagrange point calculation and visualization
- Gravitational assist trajectories
- Orbital transfer planning
- Trajectory prediction lines
- Energy conservation verification
- Performance: 1000+ bodies at 60fps

**Exit Criteria:** Simulate Jupiter's moons without Keplerian shortcuts. Watch a spacecraft slingshot around Jupiter. Total system energy drifts less than 0.01% over 1000 orbits.

---

## v0.6 — Spacecraft (Months 19–22)

> If you can't fly there, it's just a pretty picture.

**Milestone: Player-controlled spacecraft with realistic flight.**

- Basic spacecraft entity
- Thrust vectoring
- Orbital insertion burn
- Hohmann transfer execution
- Atmospheric entry heating effect
- Simple rocket staging
- Docking interface
- Navigation HUD
- Mission timeline

**Exit Criteria:** Launch from Earth, circularize orbit, execute a transfer to Mars, enter Mars orbit. See reentry heat on return.

---

## v0.7 — Cosmos (Months 23–26)

> A universe can't fit in RAM. But it can fit on disk and stream in.

**Milestone: Infinite-feeling universe through streaming and LOD.**

- Octree spatial partitioning
- LOD system for terrain and objects
- Chunk streaming from disk
- Seamless zoom: planet surface → interplanetary → interstellar
- Floating-origin coordinate system
- Memory management for universe-scale
- Performance: 100k objects visible

**Exit Criteria:** Zoom from Earth's surface into space, past Mars, past the Oort cloud, to another star — no pops, no stutters. 100k bodies rendered.

---

## v0.8 — Intelligence (Months 27–30)

> The universe is empty. Let's fill it with someone who cares.

**Milestone: Emergent civilization simulation.**

- Civilization emergence simulation
- Technology progression
- Colony placement on habitable worlds
- Resource extraction
- Simple diplomacy (trade, war, alliance)
- Megastructure construction (basic)
- Civilization statistics panel
- Historical event log

**Exit Criteria:** Place 10 civilizations on habitable worlds. Watch one develop spaceflight and colonize a neighbor. See a trade agreement form.

---

## v0.9 — Multiverse (Months 31–34)

> The real question isn't "what is?" — it's "what if?"

**Milestone: Parameter editing and scenario comparison.**

- Universal constants editor (G, c, particle masses)
- Side-by-side simulation comparison
- "What if" scenarios (no Moon, Jupiter mass = 10x)
- Chaos visualization (sensitive dependence)
- Time reversal
- Alternate timeline branching
- Preset scenarios (Great Filter, Rare Earth, etc.)

**Exit Criteria:** Clone a simulation, double Earth's mass, watch plate tectonics break. Compare timelines side by side.

---

## v1.0 — Cosmogon (Months 35–36)

> The name on the box. The version you ship to the world.

**Milestone: Production-ready release.**

- Production-quality code
- Complete documentation
- Full test suite
- Performance benchmarks
- CI/CD pipeline
- Release packaging (macOS, Linux, Windows)
- Community guidelines
- License finalized
- Website/landing page

**Exit Criteria:** `cargo install cosmogon` works. A stranger can clone the repo, build it, and understand it. The docs don't lie.

---

## Versioning Philosophy

Every version ships. No version is "done" — it's "shipped." We iterate in public. If v0.3's atmosphere rendering is garbage, it ships as garbage and we fix it in v0.4. The roadmap is a direction, not a contract.

**Key principle:** Every deliverable is binary — it works or it doesn't. No partial credit.
