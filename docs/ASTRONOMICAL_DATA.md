# Astronomical data layer

How real observations enter Cosmogon and coexist with procedural and user-created objects.

## Pipeline

```
download  →  normalise  →  validate  →  cache (versioned)  →  load  →  instantiate
tools/*.py   SI units,      epochs,        data/*.toml or        at run    into a scenario
             frames,        plausibility,  user data dir         time      or sandbox
             provenance     uniqueness
```

* **Download** uses the provider's official API with polite rate limits.
* **Normalise** converts to SI, the ecliptic J2000 frame, TDB, and records uncertainty
  columns where the source provides them.
* **Validate** rejects implausible rows (scripts assert distances, epochs, duplicates).
* **Cache** writes a versioned dataset with a `[dataset]` header (provider, version,
  epoch, retrieval time, license, attribution). Small datasets are committed and embedded
  in the binary; large catalogues (Gaia) will be downloaded on demand into
  `<data dir>/Cosmogon/catalogues/<dataset>/<version>/` as spatial tiles.
* **Offline**: once cached, everything works without a network. The app never calls a
  remote API while simulating.

## Real, procedural and user objects together

| Kind | Where it lives | Provenance | Editable |
|---|---|---|---|
| Reference dataset (Real Universe) | instantiated read-only | MEASURED / DERIVED | no — *Clone to sandbox* first |
| Sandbox copy of real data | the sandbox state | MEASURED until edited | yes → USER MODIFIED |
| Procedural objects | generated from seed | PROCEDURAL | yes in a sandbox |
| User-created objects | the sandbox state | USER MODIFIED (source "Created by user") | yes |

**Clone, never mutate.** Opening the Real Universe instantiates the dataset as a reference
view. *Clone to sandbox* copies the current state into a new sandbox document; the dataset
and other sandboxes are never affected. The clone records `origin.cloned_from =
"dataset:<id>@<version>"`.

**Pinned versions.** A sandbox stores its initial conditions, so a dataset update never
changes an existing experiment. New sandboxes use the latest cached dataset.

## Current datasets

* `jpl-horizons-sol@1` — Solar System barycentric state vectors at 2026-01-01 TDB (see
  [DATA_SOURCES.md](DATA_SOURCES.md)). Used by the *Solar System Lab* template and the
  Real Universe view: the N-body simulation starts from these vectors, so planetary
  positions correspond to the real sky at the epoch.
* Physical parameters of the same bodies from `sol.toml` (NASA fact sheets).

## Scale and streaming (design for S4/S5)

* **Spatial index**: HEALPix (nside by depth) × distance shells; one tile file per cell,
  stars sorted by brightness so a tile can be read partially.
* **LOD**: at galactic scale, aggregate tiles into luminosity-weighted point clouds; at
  stellar-neighbourhood scale, stream individual catalogue stars as GPU point sprites
  (instanced, f32 offsets from a tile origin in f64); near or selected stars become full
  `StarSystem`s with physical models.
* **Memory**: bounded LRU of loaded tiles; never more than a few million point sprites.
* **Selected objects** receive the richest available data (astrophysical parameters,
  known planets from the Exoplanet Archive) with uncertainties shown in the inspector.
