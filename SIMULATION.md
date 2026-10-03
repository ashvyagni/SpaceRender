# Simulation model

All numbers that are assumptions live in [`crates/cosmogon_sim/data/science.toml`](crates/cosmogon_sim/data/science.toml);
physical constants live in `cosmogon_core::constants` and `cosmogon_sim::astro`. This document lists
every major simplification.

## Time and scheduling
* The clock is `f64` seconds from J2000 (resolution ≈ 64 s at 10 Gyr — ample for every system that runs then).
* **Orbits are never stepped**: positions are analytic Kepler solutions at the current time, exact at any speed, past or future.
* Stateful systems are **fixed-period tasks**: biospheres every 10,000 years (climate refreshed every 1 Myr),
  civilizations every year (their home climate every 10 years). Due times are `origin + k·period` from an
  integer counter, so there is no drift.
* Speeds run from real time to 100 Myr/s. If the CPU budget per frame (9 ms) runs out the clock simply
  falls behind — steps are never enlarged — so speed never changes outcomes. The UI reports "CPU-limited".
* With *auto-slow* on, a milestone (importance 5) stops the advance immediately and drops the speed to 100 yr/s.

## Determinism
* RNG streams are keyed: `(universe seed, domain, entity…, step index)` → SplitMix64. No RNG state is
  stored, so saves need none, and adding a new random consumer never perturbs existing ones.
* Tests prove (a) identical universes from identical settings, (b) identical results however time is
  chopped into frames, (c) save → load → continue equals an uninterrupted run (`cosmogon-cli check`).
* All transcendental functions in the simulation go through the pure-Rust `libm` crate (`DMath` trait),
  so a seed produces **bit-identical universes on every OS and CPU**. A golden-fingerprint test runs in CI on
  macOS (ARM), Windows and Linux (x86-64).

## Stars
Piecewise mass–luminosity (`L ∝ M^2.3 / M^4 / M^3.5`) and mass–radius power laws; T from Stefan–Boltzmann;
main-sequence lifetime `10 Gyr · M^-2.5`. Luminosity rises linearly from 72% to 134% of its characteristic value
over the main sequence (calibrated so today's Sun has L = 1.00), then a short giant phase (L × 2000·Δ) and a
white-dwarf remnant. Flare activity is high for M dwarfs and decays with age. Conservative habitable zone
from Kasting-style flux limits (1.1 and 0.53 S⊕). Spectral types are derived from temperature.

## Neighbourhoods and planetary systems
* Star count from the local stellar density (0.004 ly⁻³); masses from a broken power-law IMF (≈73% M dwarfs).
* 22% of systems get a wide companion (S-type: planets orbit only the primary).
* Planet count ~ Poisson; spacing ratios 1.35–2.6; rocky inside the frost line (2.7 AU · √L), giants beyond
  with a probability ∝ 10^(2[Fe/H]); radii from Chen & Kipping-style mass–radius relations.
* Tidal locking for close-in planets (≈0.45 AU · M^⅓ at Gyr ages) and all moons.
* Atmospheres: retention index from escape velocity vs. thermal speed, eroded by flares, helped by a magnetic
  field; hot wet worlds become Venus-like CO₂ hothouses; some super-Earths keep H₂ envelopes; cold icy moons
  may have Titan-like N₂/CH₄. Magnetic dynamos need mass, spin and a warm interior.
* **Garden world** scenario: an Earth-analogue at Earth-equivalent insolation, CO₂ set by the thermostat below.

## Climate (zero-dimensional)
`T_eq = 278.6 K · L^¼ · d^-½ · (1−A)^¼` and `T_s = T_eq · (1 + 0.75 τ)^¼` with optical depth
`τ = 4.24·p_CO₂^0.777 + 0.16·ln(1 + p_CO₂/10⁻⁴)·min(P,1) + τ_H₂O + τ_CH₄ + τ_H₂`. Uncalibrated, this gives Venus
≈ 740 K, Earth ≈ 288 K, Mars ≈ 213 K and ~3 K per CO₂ doubling on Earth (all covered by tests). Real bodies get a
small calibration offset so they match observations exactly while still responding to change.
* Water: liquid between 273 K and the Clausius–Clapeyron boiling point; ocean cover from inventory; ice caps
  and snowball states feed back through albedo.
* **Carbonate–silicate cycle** (geological timescales only): on active wet worlds CO₂ accumulates while frozen
  and is weathered out when hot — this is what rescues planets from the faint-young-star snowball.
* Moist/runaway greenhouse slowly removes water above 340 K.
* **Glacial cycles**: Milankovitch-like periods (20–120 kyr); a large moon stabilises obliquity and lengthens
  interglacials. Real Earth: glacial until 11,700 years ago (start of the Holocene).

## Terrain
Earth uses measured relief (NOAA ETOPO5, bicubic); all other worlds are procedural.
3D domain-warped gradient noise on the unit sphere plus ridged mountain belts; sea level chosen so the
ocean fraction matches the climate model. Biomes from temperature (latitude, elevation lapse, substellar point
for locked worlds) and moisture. **The same functions place settlements and paint the planet**.

## Resources
Relative to Earth = 1. Metals ∝ 10^[Fe/H]; ore concentration (copper, tin, uranium) needs geological activity;
U-235 decays with a ~1 Gyr mean life, so old worlds are poor in fissile uranium. **Coal and oil are produced by
the biosphere** (land vegetation → coal, marine biomass → oil) and consumed by civilizations. Fertile land and
fresh water follow climate and vegetation. Deposits are placed on terrain (copper/tin in mountain belts, oil on
shelves, coal in lowland basins).

## Habitability
A weighted geometric mean of factors in 0..1 — temperature, liquid water, pressure, radiation (magnetic field,
atmosphere, flares), climate regulation (geology), stellar energy, stellar stability, chemistry, tidal locking —
plus the most complex life the environment can support (surface ocean → intelligence on rocky worlds;
subsurface ocean → at most multicellular). It scales rates; it never decides outcomes.

## Life
Stages: sterile → prebiotic → microbial → complex cells → multicellular → complex ecosystems → intelligent.
Each transition is a Poisson hazard (per Gyr, see `science.toml`) × habitability × multipliers, with gates:
complex cells and multicellularity are helped by oxygen; complex ecosystems need O₂ > 5%; intelligence needs
50 Myr of diverse complex ecosystems. Photosynthesis evolves by hazard and oxygenates the air after a sink
delay. Mass extinctions (rate raised around flaring stars) cut biodiversity and can set life back a stage;
rare sterilising events reset it. Dying environments kill biospheres.

**Prehistory:** at creation every biosphere is simulated from its planet's formation to the start epoch with
the same step and model (in parallel), except that intelligence may not arise before observation begins.

**Default rates are deliberately uncertain and conservative**: with "Realistic" settings most universes
contain microbes in subsurface oceans and little else. "Hopeful" and "Teeming" presets multiply the rates.
