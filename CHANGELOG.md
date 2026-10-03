# Changelog

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
