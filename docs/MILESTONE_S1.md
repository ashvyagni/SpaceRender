# Milestone S1 — Sandbox Foundation: what works and what is approximated

Version 0.4.0. See [SANDBOX_VISION.md](SANDBOX_VISION.md) for the plan this delivers against.

## Acceptance demonstrations

| Demonstration | Status | How it is verified |
|---|---|---|
| Home → New Sandbox → Solar System Lab (JPL data) → pause → create a 1 M⊕ planet between Earth and Mars → play → bodies respond → inspect → accelerate → save → quit → reopen → Continue → resumes | ✅ | `sandbox::tests::add_planet_perturbs_and_save_resumes` (Mars's orbit changes; save → load → continue is bit-identical); app captures of Create, autosave-on-exit and Continue |
| Create asteroid → launch at Earth → predicted trajectory bends → impact → impact event → environment, biosphere, civilization respond | ✅ | `sandbox::tests::asteroid_impact_on_earth`, `prediction_sees_an_aimed_impact`; app capture of the Launch tool predicting the impact and of the impact toasts |

## Functional now

**Application shell** — cinematic intro (skippable), HOME with New Sandbox, Continue, Load
Sandbox, Real Universe, Scenarios, Settings, Credits, Quit (Object Lab shown as a later
milestone). Seven templates; physics, life and civilization options per sandbox.

**Sandbox documents** — per-sandbox folder with manifest (name, description, dates,
simulated date, seed, template, origin, dataset versions, physics engine version, physics
settings, enabled systems, edit count, checkpoints), explicit save, autosave (timed and on
quit), Save As (branch), Duplicate, Rename, Delete (confirmed), checkpoints with restore,
thumbnails, legacy-save import, local profile directory for future accounts.

**Real data** — JPL Horizons state vectors for the Sun, 8 planets (giants as system
barycentres), the Moon and 6 major moons at 2026-01-01 TDB, fetched by a reproducible
pipeline and cached offline; provenance on every body and quantity
(MEASURED / DERIVED / ESTIMATED / PROCEDURAL / USER MODIFIED) shown in the inspector.
Real Universe opens read-only with *Clone to sandbox*.

**Physics** — Newtonian N-body (4th-order symplectic), automatic encounter substepping,
collisions with inelastic merger, optional 1PN relativity, rails moons promoted on
approach, Kepler ↔ N-body switching, five presets, deterministic and frame-independent,
validated against analytic cases and JPL (see PHYSICS_ENGINE.md).

**Editing** — create planets, ocean/ice worlds, moons, giants, dwarf planets, asteroids and
comet nuclei from real-analogue presets; set mass, radius, gravity (radius follows), day
length, tilt, albedo, pressure, CO₂, water, orbital elements, push (Δv); change the star's
mass, age and metallicity; delete; launch tool aimed at any body; all with units, typed
values in scientific notation, warnings (density, Roche limit, unbound, step limit) and
undo/redo (40 steps). Every edit is journalled in the save.

**Consequences** — orbit → annual-mean insolation → climate → water/ice → habitability →
fertile land → civilization capacity; star changes reach every world; impacts produce a
crater, blast zone, impact winter, extinction and casualties at settlements; destroyed
worlds end their biospheres and civilizations; large insolation changes and impacts are
reported in the chronicle with numbers.

**Views** — osculating orbits for dynamic systems, past trails, predicted trajectories,
velocity vectors, calendar clock, physics badge, command palette (⌘/Ctrl+K), curated
"what if" scenarios built from ordinary edits.

## Approximated (documented, labelled in the UI)

* Climate is zero-dimensional and instantaneous (no thermal inertia, no seasons); impact
  winter cools surface air only; the "ocean" is not frozen by a few-year winter.
* Impact consequences are first-order scaling laws anchored on Chicxulub and Toon et al.;
  no atmospheric entry, airbursts, fragmentation, ejecta or tsunamis yet.
* Collisions always merge; no bounce, fragmentation or debris.
* No tides, oblateness or non-gravitational forces; tidal locking is a flag.
* Rails moons are massless test bodies until promoted.
* The Solar System Lab has no civilization model (present-day humanity is not seeded yet);
  civilizations respond to physics in the Dawn of Humanity and Habitable World templates.
* Thumbnails include the interface.

## Not yet (later milestones)

Drag-in-space launch gesture (the panel aim works), Hill/SOI/Lagrange/apsides overlays,
blueprints, comparison view, dockable layouts and themes, branches UI, rewind/replay,
stars/black holes/compact objects as creatable objects, Gaia/exoplanet data, galaxy layer,
the major graphics and civilization-visualisation upgrade.
