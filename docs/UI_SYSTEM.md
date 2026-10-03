# UI system

The interface is a product system, not a debug overlay. This document defines the design
language and the flows; milestone S7 completes the full customisation layer (docking,
layouts, themes, input remapping).

## Principles

1. **The simulation is authoritative.** The UI reads state and issues commands (`Edit`,
   focus, speed, save). It never mutates the model directly.
2. **Simple and Advanced over the same simulation.** Simple mode shows sliders and
   friendly units; Advanced shows typed values, scientific notation, all units, lock /
   derived status and provenance. Both edit the same quantities.
3. **Honesty.** Values carry provenance (MEASURED / DERIVED / ESTIMATED / PROCEDURAL / USER
   MODIFIED) and the active model and fidelity are visible.
4. **Calm by default.** Overlays are off unless useful; the Cinematic layout hides
   everything.
5. **Graphics settings never change simulation results.**

## Design language

* Dark, near-black panels (`#090C14`), one warm accent (amber `#E8B04B`), muted text
  `#828CA0`, semantic colours: life green, civilization gold, danger red, data blue.
* Uppercase micro-headings with tracking; numbers right-aligned; units in muted text.
* 6 px radii on controls, 10 px on windows, 1 px hairline strokes.
* Motion: 150–250 ms ease-out for panels; cinematic camera flights 1.8 s with logarithmic
  distance interpolation.

## Screens and flows

```
Intro (logo over the sky, ~2 s, click to skip)
  └ HOME
     ├ NEW SANDBOX → template cards → configure (name, description, seed, physics preset,
     │                enabled systems) → Create → Sandbox view
     ├ CONTINUE → most recently modified sandbox (autosave or save, whichever is newer)
     ├ LOAD SANDBOX → cards (thumbnail, name, date, template, modified) with
     │                Open · Duplicate · Rename · Delete · Checkpoints; legacy saves below
     ├ REAL UNIVERSE → read-only reference view of the Solar System (JPL Horizons epoch)
     │                → "Clone to sandbox"
     ├ SCENARIOS → curated starts (Dawn of Humanity, Garden World, Neighbourhood; the
     │             "what if" experiments arrive with S2/S3 and use the same tools)
     ├ OBJECT LAB (milestone S3+) · SETTINGS · CREDITS · QUIT
```

### Sandbox view

* **Top bar**: menu · sandbox name and badge (SANDBOX / REFERENCE · READ-ONLY) · search ·
  undo / redo · tools (Select · Create · Launch) · save · panel toggles.
* **Left panel**: Systems · Chronicle (event feed) · Civilizations.
* **Right panel (inspector)**: tabs per object type — OVERVIEW · ORBIT · PHYSICS ·
  ENVIRONMENT · LIFE · CIVILIZATION · HISTORY · DATA. Editable fields in sandboxes.
* **Bottom bar**: play/pause, speed, date, *CPU-limited* indicator, physics model badge,
  latest events.
* **Create window**: category → preset → basic fields → placement (orbit around a body at
  a distance with circular / custom velocity, or exact state vectors in Advanced) →
  warnings → Add (undoable).
* **Launch tool**: choose a projectile, then either drag in space (press = start point on
  the orbital plane, drag = velocity) or *Aim at target* (distance, approach speed, miss
  distance). The predicted trajectory updates live; Release adds the object.
* **Physics window**: orbit model (Kepler / N-body), preset with a table of what changes,
  relativity, live diagnostics (energy error since activation, substeps, step size).
* **Command palette** (⌘K / Ctrl+K): actions (create, launch, save, toggle overlays,
  physics settings, set speed) and object search; Enter runs, arrow keys select.

### Keyboard

Space pause · , . speed · ⌘Z / ⌘⇧Z undo/redo · ⌘S save · ⌘K palette · F fly to ·
Home system view · Tab hide UI · F1 help · F3 developer · Esc menu / cancel tool.

## Units

Global preferences (Settings → Units): mass (auto, kg, M⊕, M♃, M☉), distance (auto, km,
AU), speed (m/s, km/s), temperature (K, °C). Every editable field also has its own unit
picker. All conversion lives in `cosmogon_sim::units`.

## Customisation roadmap (S7)

Dockable/undockable panels (`egui_dock`), multi-monitor windows (bevy_egui multi-context),
saved layouts (Exploration, Sandbox, Physics, Planet, Civilization, Minimal, Cinematic),
themes and accent colours, transparency, font and UI scaling, key/mouse/controller
remapping, per-profile layout storage, accessibility review (contrast, scalable fonts,
reduced motion).
