# Save format

Experiments are first-class documents ("sandboxes"). Everything is versioned; nothing
depends on network access.

## On-disk layout

```
<data dir>/Cosmogon/
  profiles/
    local/                         the offline profile (future accounts map onto profiles)
      profile.json                 { id, display_name, created }
      sandboxes/
        <sandbox-id>/
          manifest.json            small: listed without reading any state
          state.cosmo              last explicit save
          autosave.cosmo           written every N minutes and on quit
          thumbnail.png            captured on save (UI hidden)
          checkpoints/
            <unix-time>.cosmo      user checkpoints (label in the manifest)
  saves/                           legacy single-file saves (v0.2–0.3), still listed and loadable
```

`<data dir>` is `~/Library/Application Support` on macOS, `%APPDATA%` on Windows,
`~/.local/share` on Linux. Sandbox ids are `sb-<unix-ms>-<random>` and never reused.

## manifest.json

```json
{
  "format": "cosmogon-sandbox", "version": 1,
  "id": "sb-…", "name": "2× Jupiter experiment", "description": "…",
  "created_unix": 0, "modified_unix": 0,
  "template": "solar_system_lab",
  "origin": { "cloned_from": "dataset:jpl-horizons-sol@1", "parent_sandbox": null, "parent_checkpoint": null },
  "sim_date": "1 Mar 2031", "sim_time": 9.5e8,
  "seed": 2026,
  "app_version": "0.4.0", "physics_engine": "nbody-yoshida4/1",
  "data_versions": { "jpl-horizons-sol": "1", "etopo5-earth": "1" },
  "physics": { "model": "NBody", "preset": "Balanced", "relativity": false },
  "systems_enabled": { "climate": true, "life": true, "civilization": true },
  "checkpoints": [ { "file": "checkpoints/….cosmo", "label": "Before impact", "sim_date": "…", "created_unix": 0 } ],
  "bookmarks": [],
  "owner": "local"
}
```

`origin` makes **branches** possible: a branch is a new sandbox whose `parent_sandbox` and
`parent_checkpoint` point at where it diverged (UI planned; the format already allows it).

## State files (`*.cosmo`)

JSON: `{ "header": {…}, "universe": {…} }`, format `cosmogon-save`.

* **v1** (0.2–0.3): header + universe.
* **v2** (0.4): adds `universe.edits` (the journal of user modifications), per-system
  `dynamics` (N-body state), per-body `provenance`, `class`, `removed`, `impacts`, and
  `settings.systems`. All new fields have defaults, so the v1 → v2 migration is a no-op on
  the JSON; v1 files load unchanged and keep Kepler orbits.

Rules (unchanged): bump `CURRENT_VERSION` and add a migration step for every change;
writes are atomic (temp file + rename); a file newer than the app is refused with a clear
message.

## Reproducibility

Every state file records the app version, physics engine version, dataset versions, seed,
the initial conditions (inside the state) and the journal of user modifications. Saves pin
their data: refreshing a dataset never changes an existing experiment. Because the
simulation is deterministic across platforms, another installation reproduces the same
experiment from the same file.

## Undo / redo

In memory only: a bounded stack (40 entries) of universe snapshots taken immediately before
each edit. *Undo* returns the sandbox to the moment before the edit, including simulation
time. Snapshots are not saved.

## Sharing (planned)

A single-file export bundling manifest + state + thumbnail. The extension will be chosen
after registering document types on each platform (`.cosmogon` is the candidate; macOS
`UTExportedTypeDeclarations`, Windows ProgID via the installer).
