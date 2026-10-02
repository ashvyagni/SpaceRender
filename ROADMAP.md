# Roadmap

From the current repository to the target product. Each phase ends with a working, tested,
packaged application — never a broken tree. Phases overlap; the order reflects dependencies.
✅ done · 🟡 partially done in M1 · ⬜ not started.

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

## Known limitations of M1 (honest list)
* Surfaces are baked equirectangular textures (≤ 4096 px): fine from orbit, blurry near the ground.
* Earth's surface in the Sol scenario is procedural, not real geography.
* A civilization is one statistical society; there are no rival nations on the same planet yet.
* With a civilization alive, maximum speed is CPU-bound (years are never skipped).
* Floating-point transcendental functions are not guaranteed bit-identical across CPU architectures,
  so saves continue identically on the same platform but may diverge slowly across platforms
  (fix: `libm`-based math in the sim — tracked).
