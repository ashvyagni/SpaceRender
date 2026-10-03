# Roadmap

From the current repository to the target product. Each phase ends with a working, tested,
packaged application — never a broken tree. Phases overlap; the order reflects dependencies.
✅ done · 🟡 partially done in M1 · ⬜ not started.

## Universe sandbox phase (from 0.4.0)
The product direction is now a full universe sandbox — see [docs/SANDBOX_VISION.md](docs/SANDBOX_VISION.md),
the audit [docs/GAP_ANALYSIS.md](docs/GAP_ANALYSIS.md) and the S1 status [docs/MILESTONE_S1.md](docs/MILESTONE_S1.md).

| # | Milestone | Status |
|---|---|---|
| S1 | Sandbox foundation: home & sandbox documents, JPL data, object schema & provenance, creator, editable inspector, N-body, launch, prediction, collisions, undo/redo, physics tests, orbit → climate | ✅ 0.4.0 |
| **G1** | **Graphics upgrade** (requested next): every planet as detailed close up as Earth, Moon and Mars — gas-giant banding and storms, Venus/Titan hazes, icy-moon terrain, Mercury/Io/Callisto surfaces from real global mosaics where licensable; atmospheric scattering, ocean shading, volumetric clouds, rings with shadows, eclipses | ⬜ next |
| **C1** | **Civilization visibility** (requested next): progress & technology at a glance on habitable worlds, visible cities and infrastructure, satellites and spacecraft as objects, colonies on other planets, exploration of the system and beyond; player interventions that respect the simulation | ⬜ next |
| S2 | Physical consequence pipeline: seasons and eccentric insolation, thermal inertia, sea level, migration, economic response; present-day humanity in the Lab | ⬜ |
| S3 | Solar System expansion: JPL small bodies, comet tails, atmospheric entry, fragmentation, tides, Roche disruption, Hill/SOI/Lagrange overlays, drag-to-launch | ⬜ |
| S4 | Real-universe data platform (ingestion, spatial index, streaming, dataset versions UI) | ⬜ |
| S5 | Universe exploration (Gaia stars, known exoplanets, galaxy layer, search) | ⬜ |
| S6 | Exotic objects (white dwarfs, neutron stars, black holes, lensing, stellar evolution) | ⬜ |
| S7 | Premium UI (docking, layouts, themes, comparison, accessibility, input remapping) | ⬜ |
| S8–S10 | Advanced planetary consequences · long-time physics · productization (signing, updates) | ⬜ |

Notes on the requested direction for habitable worlds: a world is only labelled habitable from
its computed environment (temperature, water, atmosphere, star), never by assertion, and real
bodies keep their measured data. Civilizations already launch satellites, settle other bodies,
send probes and detect each other by radio in the model; C1 makes all of that visible.

## Phase 0 — Prototype audit ✅
[docs/AUDIT.md](docs/AUDIT.md), [docs/ENGINE_ASSESSMENT.md](docs/ENGINE_ASSESSMENT.md).

## Phase 1 — Core engine architecture ✅ (M1)
✅ Simulation time and speeds from real time to 100 Myr/s · ✅ multi-rate fixed-period scheduler with
CPU budget · ✅ deterministic RNG streams · ✅ f64 camera-relative rendering · ✅ astronomical camera ·
✅ versioned saves with migration hook · ✅ headless CLI · ✅ packaging (.app/.dmg, CI for .exe/Linux).
Next: ⬜ snapshot-based **rewind** (orbits already reverse analytically; stateful systems need keyframes) ·
⬜ move the simulation onto a worker thread with double-buffered read views.

## Phase 2 — Astronomical simulation 🟡
✅ Main-sequence stars with brightening, giant and white-dwarf phases · ✅ IMF-sampled neighbourhoods,
binaries (S-type), planets around frost lines, moons, belts, rings · ✅ real Solar System.
Next: ⬜ comets · ⬜ circumbinary (P-type) planets · ⬜ optional N-body integration for selected systems
(integrators already exist in `cosmogon_physics`) · ⬜ a galaxy layer (procedural star density, lazy generation
of distant systems — determinism is preserved by per-system RNG streams) · ⬜ real nearby-star catalogue
placement (the prototype's 51-star catalogue is ready to import).

## Phase 3 — High-quality planet rendering ⬜ (next major milestone)
⬜ Cube-sphere quadtree terrain with GPU-generated height/normal tiles, streamed by screen-space error ·
⬜ precomputed atmospheric scattering (Bruneton/Hillaire) · ⬜ ocean shading with Fresnel and waves ·
⬜ real elevation data for Earth/Moon/Mars (public-domain NASA/USGS, licensing recorded in ASSETS.md).
The baked-texture surface of M1 already uses the authoritative terrain function, so the switch is a
change of resolution, not of meaning.

## Phase 4 — Planetary environment 🟡
✅ Zero-dimensional climate with greenhouse, carbonate–silicate cycle, water phase, glacial cycles ·
✅ biomes · ✅ resources and deposits. Next: ⬜ regional climate grid (latitude bands → cells) ·
⬜ plate-tectonic history driving continents and ore belts · ⬜ weather systems for rendering.

## Phase 5 — Life 🟡
✅ Stage hazards, oxygenation, fossil-fuel formation, extinctions, subsurface biospheres.
Next: ⬜ biome-level ecosystems (biomass per region) · ⬜ multiple lineages per world · ⬜ panspermia option.

## Phase 6 — Civilization foundation 🟡
✅ Species from environment · ✅ population, capacity, research, pressures, crises, collapse ·
✅ representative settlements, expansion by reach, Zipf urbanisation, roads/rail/sea lanes · ✅ history.
Next: ⬜ **multiple polities per world** (states, borders, diplomacy, war between them) · ⬜ cultures and
languages · ⬜ regional economies · ⬜ **civilization LOD**: multi-year steps for stable mature societies so
geological speeds stay fast once civilizations exist.

## Phase 7 — Technology & economy 🟡
✅ Data-driven causal technology graph (56 techs, alternative routes, resource/environment gates,
"why not yet" explanations). Next: ⬜ macro-economy (production, consumption, trade between settlements) ·
⬜ materials and supply chains feeding technology prerequisites · ⬜ mod/data folder override for
`technologies.toml` and `science.toml` at runtime.

## Phase 8 — Civilization visualisation 🟡
✅ Night-side lights from settlements and corridors · ✅ settlement and network overlays · ✅ satellites.
Next: ⬜ procedural cities as instanced geometry at close range, styled by era and species ·
⬜ farmland/deforestation tinting of the surface · ⬜ visible infrastructure (ports, dams, spaceports).

## Phase 9 — Space age 🟡
✅ Launch Δv physics (super-Earth trap), satellites, crewed flight, moon landing, colonies, probes,
radio spheres and detection between civilizations. Next: ⬜ visible spacecraft on transfer orbits
(Hohmann/Lambert from `cosmogon_physics`) · ⬜ interplanetary trade · ⬜ colony growth and independence ·
⬜ sub-light interstellar settlement. No faster-than-light travel in the default scientific mode.

## Phase 10 — Graphics upgrade ⬜
Volumetric clouds, eclipses and shadows between bodies, PBR rocky surfaces at close range, lens and
exposure model (auto-exposure), TAA, star rendering by apparent magnitude, galaxy backdrop from the
simulated star field.

## Phase 11 — Productization 🟡
✅ Main menu, new-universe setup, save browser, settings, graphics presets, autosave, icon, .app/.dmg,
CI release workflow (Windows installer + portable zip, Linux tarball). Next: ⬜ signed + notarised macOS
builds (needs an Apple Developer ID) · ⬜ Windows code signing · ⬜ crash reporter (write panic + seed +
save to a report file) · ⬜ AppImage/Flatpak.

## Phase 12 — Optimisation & polish ⬜
Profiling (Tracy), LOD tuning, memory budgets, UI polish, audio, tutorial, cinematic camera paths,
localisation.

## Milestone 2 (done, v0.3.0)
Real Earth relief · rival nations · close-up quadtree terrain · civilization LOD · cross-platform determinism ·
Windows build + GitHub release pipeline.

## Known limitations (honest list)
* See [docs/MILESTONE_S1.md](docs/MILESTONE_S1.md) for what the sandbox approximates.
* Planets other than Earth, Moon and Mars lack close-up surface detail (graphics milestone G1).
* Close-up terrain is vertex-coloured relief without textures, vegetation, cities or volumetric clouds yet;
  at very low altitude it looks smooth-sculpted rather than photographic (Phase 3/10).
* Earth's moisture/biome pattern is procedural; only relief is real (Köppen climate data planned).
* Nations share one knowledge pool; there is no diplomacy UI, trade or per-nation technology yet.
* Unsigned builds: first launch needs an extra click on macOS and Windows (signing needs paid certificates).
* The Windows build is cross-compiled and CI-built but has had no manual QA on Windows hardware yet.
