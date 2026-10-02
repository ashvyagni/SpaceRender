# Changelog

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
