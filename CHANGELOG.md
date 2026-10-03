# Changelog

## 0.5.0 — Worlds and civilizations you can see (2026-10-04)
Details: docs/MILESTONE_G1_C1.md.
### Added
* **Appearance model**: every planet, moon and exoplanet gets a physically chosen style (cratered,
  volcanic, ice shell, grooved ice, tiger stripes, haze, cloud deck, lava/magma ocean, Sudarsky-class
  giants with belts, jets and storms); Jupiter's GRS and ovals, Saturn's hexagon, Neptune's dark vortex.
* **Rendering**: atmospheric single scattering, eclipse shadows from moons and planets, ring shadows,
  Saturn's measured ring structure, stellar limb darkening/granulation/spots, lava and thermal glow,
  procedural detail and bump mapping beyond texture resolution, mip-maps.
* **Present-day humanity** in the Solar System Lab: real cities and countries, exploration record,
  missions in flight (BepiColombo, Europa Clipper, JUICE), 11 000 satellites.
* **Space programmes**: missions to every world (flyby → orbiter → lander → crewed), colony ships,
  energy transition away from fossil fuels; Kardashev rating.
* **Visible civilizations**: at-a-glance card, space programme panel, planet badges, satellite shells
  incl. geostationary belt, stations, spacecraft in flight, colony sites and lights, cities and farmland
  on the ground.
* **Interventions**: Inspire, Signal, Hardship, Share knowledge, Teach a technology — with a 25-year
  cooldown and no skipping of prerequisites; undoable sandbox edits.
### Changed
* Only the Garden World template guarantees a habitable planet; hot Jupiters can now form.
* Golden fingerprint re-pinned (energy transition changes long runs).
### Fixed
* Stray terrain quads and one-frame flashes of newly spawned bodies.

## 0.4.0 — Sandbox Foundation (2026-10-03)
Cosmogon becomes a universe sandbox. Details: docs/MILESTONE_S1.md.
### Added
* **Home & sandboxes**: new application shell (intro, New Sandbox with seven templates, Continue, Load,
  Real Universe, Scenarios, Settings, Credits); sandbox documents with manifest, autosave (also on quit),
  Save As, Duplicate, Rename, Delete, checkpoints, thumbnails; legacy-save import; local profile.
* **Real data**: NASA/JPL Horizons pipeline (`tools/fetch_horizons.py`) and cached 2026-01-01 state vectors;
  Solar System Lab starts from them; Real Universe view (read-only) with Clone to sandbox; per-quantity
  provenance (measured / derived / estimated / procedural / user-modified) in the inspector.
* **N-body gravity**: 4th-order symplectic integrator with encounter substepping, collisions and mergers,
  optional 1PN relativity, rails moons, presets (Fast/Balanced/Accurate/Research/Custom), Kepler ↔ N-body;
  one-year integration matches JPL to ≤ 6 km for Venus–Neptune.
* **Editing**: object creator (planets, moons, giants, dwarf planets, asteroids, comets), editable inspector
  with units and scientific notation, star mass/age/metallicity, orbital elements, Δv pushes, delete,
  launch tool with live predicted trajectory and impact warning, undo/redo, warnings for unusual input.
* **Consequences**: orbit → insolation → climate → habitability → civilizations; impacts with crater,
  blast radius, impact winter, extinction and casualties; destroyed worlds end their civilizations.
* Overlays (osculating orbits, trails, predictions, velocity), calendar clock, command palette,
  seven curated "what if" scenarios built from ordinary edits; save format v2.
* Docs: GAP_ANALYSIS, SANDBOX_VISION, PHYSICS_ENGINE, ASTRONOMICAL_DATA, DATA_SOURCES, OBJECT_MODEL,
  SAVE_FORMAT, UI_SYSTEM, MILESTONE_S1.
### Changed
* One set of physical constants (IAU 2012/2015, CODATA 2018); insolation uses the annual-mean distance
  a(1−e²)^¼. Seeds replay slightly differently from 0.3 (golden fingerprint re-pinned); old saves load.
### Fixed
* `state_vectors_to_elements`: swapped arguments for the ascending node; equatorial orbits lost periapsis.

## 0.3.0 — Milestone 2 "Living worlds, rival nations" (2026-10-03)
### Added
* **Real Earth geography** (NOAA ETOPO5, public domain) with bicubic sampling; humanity starts in East Africa.
* **Rival nations** (`civ/polity.rs`): polities with territory, capitals, governments, relations, border wars,
  city captures, conquest, unions, secession, revolutions, world government; nuclear war between rivals.
* **Close-up terrain**: cube-sphere quadtree LOD of displaced, vertex-coloured patches (skirts, horizon culling,
  f64 placement), crater fields on airless worlds, path-length haze; terrain-aware camera.
* **Civilization level of detail**: per-civilization step sizes (1/10/100/1000 yr) with exact multi-year
  formulas; the civilization task sleeps when nothing is due.
* **Cross-platform determinism**: all simulation transcendental math via `libm`; golden-fingerprint test in CI.
* Windows cross-compilation from macOS; release workflow publishes a plain `.exe`, installer, portable zip, DMG.
* CLI `profile`; app flags `--latlon`, `--debug`.
### Changed
* Research pace recalibrated for the industrial era; crisis pressures scale with severity.
### Fixed
* Airless worlds (Moon, Mercury, most moons) were never lit and rendered black.
* Dry worlds measured height from a sentinel sea level and were drawn as ice; ice sheets now need water,
  frozen water covers realistic areas; no snow on peaks of waterless worlds.
* UI: opaque panels, markers occluded by planets, captures ignore user input.


## 0.2.0 — Milestone 1 "Living skeleton" (2026-10-03)
First end-to-end vertical slice, shipped as a double-clickable macOS app.

### Added
* `cosmogon_sim`: engine-independent, deterministic universe simulation — keyed RNG streams; multi-rate scheduler
  with CPU budget; stellar evolution; procedural neighbourhoods (IMF, binaries, frost-line architectures, moons,
  belts, rings); real Solar System as data; climate with greenhouse fit, carbonate–silicate cycle and glacial cycles;
  shared terrain and biomes; resources and deposits (fossil fuels made by life); multi-factor habitability; life
  stages with oxygenation and extinctions, simulated through prehistory; species; civilizations with knowledge,
  pressures, crises, collapse, settlements and networks; a 56-node causal technology graph; space age, radio
  detection and probes; history; versioned saves with migration hook.
* `cosmogon_cli`: `run`, `check` (determinism + save round trip), `survey`.
* `cosmogon` app (Bevy 0.18): f64 camera-relative rendering; astronomical camera; planet and atmosphere shaders;
  background texture baking re-run when worlds change; night-side city lights; procedural starfield; orbit lines and
  radio spheres; egui interface (menu with live showcase world, new-universe setup, save browser, settings,
  universe browser, chronicle, inspector, civilization dashboard with "why" explanations, timeline, toasts,
  developer overlay); autosave and quick save; GPU capture tool.
* Packaging: icon generator, `.app` + `.dmg` script, CI test and release workflows (Windows installer/zip, Linux).
* Documentation set.

### Changed
* `cosmogon_physics` no longer depends on an ECS.
* The Bevy prototype became the application crate `crates/cosmogon`; `big_space` replaced by an explicit f64 origin pass.

### Removed (moved to `legacy/`, not built)
* The hand-written wgpu renderer, its ECS and UI crates. See docs/ENGINE_ASSESSMENT.md.

### Fixed (in carried-over logic)
* Moons now orbit their planet's μ instead of the Sun's; time acceleration no longer applied twice;
  analytic orbits no longer fight an N-body integrator.

## 0.1.0 — Prototype (2026-07)
Original prototype; see docs/AUDIT.md.
