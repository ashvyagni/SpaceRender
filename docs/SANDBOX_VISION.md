# Sandbox vision

Cosmogon is becoming a **persistent, interactive universe sandbox**: explore real
astronomical data, create or modify celestial objects, change physical conditions, launch
and collide things, accelerate time, and watch the consequences propagate through
astronomy, planets, climate, life and civilizations.

> Other sandboxes answer *"what happens to Earth's orbit if I add a planet?"*
> Cosmogon's goal is *"what happens to **Earth** if I add a planet?"* — orbit, climate,
> oceans, ecosystems, population, cities, economies, technology and history.

## The causal chain

```
REAL ASTRONOMICAL DATA        datasets with provenance (JPL Horizons, later Gaia, exoplanets…)
        ↓
CELESTIAL MECHANICS           Kepler (analytic) · N-body (numerical) · encounters · collisions
        ↓
STELLAR / PLANETARY PHYSICS   luminosity, mass, radius, rotation, impacts
        ↓
PLANETARY ENVIRONMENT         insolation (from the *current* orbit), atmosphere, water
        ↓
CLIMATE                       temperature, ice, ocean, impact winter
        ↓
BIOSPHERE                     habitability, biomass, extinctions
        ↓
CIVILIZATION                  capacity, food, population, migration, stability, war
        ↓
TECHNOLOGICAL DEVELOPMENT     research pressures and priorities
```

Changes higher in the chain propagate downward through events, never through a single
"health" number:

```
Edit / PhysicsEvent ──► Universe::propagate
   OrbitChanged        → insolation → climate refresh → habitability, fertile land
   Impact              → crater + energy record → impact-winter forcing (climate)
                         → extinction (biosphere) → casualties (settlements)
   Merged / Destroyed  → removed body: biosphere sterilised, civilization extinct
   StarChanged         → luminosity → climate of every world in the system
civilizations then respond through their own model: capacity, food pressure, famine,
migration, instability and research priorities.
```

## Principles

* **Sandbox freedom with honesty**: absurd experiments are allowed; warnings explain why
  they are unusual; provenance shows what is measured and what is invented.
* **Maximum practical physical fidelity**, not "all physics": every system documents its
  model, tier, valid range and error ([PHYSICS_ENGINE.md](PHYSICS_ENGINE.md)).
* **Determinism and reproducibility**: same inputs → same universe on every platform.
* **Offline first, free**: no paid APIs, no required account, data cached locally.
* **Fun**: a beginner can create a planet, throw an asteroid, double Jupiter or change the
  Sun without a manual; experts can type exact values and pick fidelity.

## Milestones

| # | Milestone | Delivers |
|---|---|---|
| S1 | **Sandbox foundation** (this milestone) | Home + sandbox workflow, versioned sandbox documents, Solar System Lab from JPL Horizons, Real Universe → clone, object schema + provenance, creator, editable inspector, N-body gravity, add/delete/move, set state, launch tool, trajectory prediction, collisions, undo/redo, physics tests, orbit → climate link |
| S2 | Physical consequence pipeline | Seasonal/eccentric insolation, thermal inertia, sea level and ice response, migration, economic response; "move Earth" demo end to end |
| S3 | Solar System sandbox expansion | Asteroids & comets from JPL SBDB, comet tails, atmospheric entry, fragmentation, rings, tides, Roche disruption, overlays (Hill/SOI/Lagrange) |
| S4 | Real-universe data platform | Ingestion framework, local databases, spatial index, streaming, dataset versions UI |
| S5 | Universe exploration | Gaia-backed stars, known exoplanets, search, galactic navigation |
| S6 | Exotic objects | White dwarfs, neutron stars, pulsars, black holes, accretion disks, lensing, stellar evolution |
| S7 | Premium UI | Docking, layouts, themes, palette, advanced inspector, comparison, accessibility, input remapping |
| S8 | Advanced planetary consequences | Ejecta, tsunamis, atmospheric loss, biosphere response, extinctions |
| S9 | Large-scale / long-time physics | Stellar evolution consequences, migration, long integrations, supernovae, rogue objects |
| S10 | Productization | Signed installers, update channel, optional privacy-conscious crash reports |

## S1 demonstrations (acceptance)

1. Launch → Home → New Sandbox → Solar System Lab → (clone of the JPL data) → pause →
   Create planet (1 M⊕) between Earth and Mars → assign velocity → play → the new body and
   the existing planets respond gravitationally → inspect orbital changes → accelerate time
   → save → quit → reopen → Continue → the experiment resumes at the same state.
2. Create asteroid → launch it towards Earth → predicted trajectory bends under gravity →
   impact → impact event with energy, crater and location → Earth's environment, biosphere
   and civilizations receive the consequences.
