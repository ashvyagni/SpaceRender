# Physics engine

How Cosmogon moves things. Every model states its fidelity tier, assumptions, valid range and
measured error. Simulation results are never tuned "to look right".

## Fidelity tiers

| Tier | Meaning | Used for |
|---|---|---|
| 0 | Visual approximation only | Background starfield, cloud drift, comet-tail shape (later) |
| 1 | Analytic physical model | Kepler orbits of untouched systems and "rails" moons; stellar relations; zero-dimensional climate |
| 2 | Numerical simulation | N-body gravity of an active sandbox system |
| 3 | High-accuracy local numerical simulation | Automatic encounter substepping; *Accurate* preset |
| 4 | Specialised precision mode | *Research* preset (+ optional 1PN relativity) |

The inspector shows the model and tier for each object (PHYSICS tab).

## Simulation domains (hybrid orbit architecture)

```
GALACTIC SCALE      star systems at fixed/catalogue positions                     Tier 1 (later: catalogue + proper motion)
SYSTEM SCALE        untouched systems: analytic Kepler, hierarchical              Tier 1
ACTIVE SANDBOX      edited / dynamic systems: Newtonian N-body                     Tier 2
  └ rails moons     light, tight moons ride Kepler orbits around their            Tier 1
                    N-body parent (massless in the N-body sense)
CLOSE ENCOUNTER     automatic substepping inside the active system                Tier 3
```

A system switches to N-body when the user selects *Dynamic gravity* or edits it; it can be
switched back to Kepler ("freeze orbits") when every body is bound — each orbit becomes its
current osculating ellipse (mutual perturbations are then neglected; the UI says so).

**Rails moons.** A moon stays on rails while `m_moon / m_parent < 10⁻³` and nobody has
touched it. The Galilean moons, Titan and Enceladus are on rails in the Solar System lab;
the Moon (ratio 0.0123) is fully dynamic. Editing a rails moon, or anything coming within
its parent's Hill sphere, promotes it to a full particle. Rationale: a 1.8-day Io orbit
would force a ~20-minute global step on the whole Solar System for a body whose effect on
anything else is negligible (Ganymede/Jupiter = 7.8 × 10⁻⁵).

## Integrator

**Fourth-order symplectic composition** (Yoshida 1990, *Phys. Lett. A* 150, 262; Forest &
Ruth 1990) of drift–kick–drift leapfrog: 3 force evaluations per step, local error O(h⁵),
bounded energy error with no secular drift at fixed step, time-reversible.

Candidates evaluated:

| Method | Pros | Cons | Decision |
|---|---|---|---|
| Leapfrog (2nd order) | Symplectic, trivial | Needs ~10× more steps than Yoshida-4 for the same accuracy | Building block |
| **Yoshida-4** | Symplectic, cheap, deterministic, handles any mass ratio | Fixed step; encounters need substeps | **Chosen** |
| Wisdom–Holman (WHFast) | Very large steps for near-Keplerian planets | Heliocentric splitting struggles with moons, binaries and close encounters — exactly what users create | Possible later "planets-only" fast path |
| IAS15 (Rein & Spiegel 2015) | Adaptive, machine precision, great for encounters | Complex predictor–corrector; adaptive step makes frame-independent determinism harder | Candidate for the Research preset later |
| RK4 / RK45 / DOP853 | General | Not symplectic: energy drifts secularly over long runs | Rejected for orbits |
| REBOUND (library) | Proven, many integrators | GPL-3.0 (incompatible with MIT/Apache distribution), C FFI complicates Windows cross-compilation and bit-identical determinism | Rejected |

## Time stepping and determinism

* The system advances in fixed **macro steps** of length `dt` on an absolute grid
  `t_k = epoch + k·dt`. Frame boundaries never split a step, so the outcome is identical
  however the work is distributed across frames (tested), on every platform (golden test).
* `dt = η · τ_min`, where `τ_min` is the shortest pair timescale at the moment the system
  became dynamic or was last edited. `η = 2π / steps-per-orbit` from the preset.
* Inside each macro step, `n = ⌈dt / (η · τ)⌉` equal substeps, where `τ` is the shortest
  pair timescale *over the coming step*: free-fall `√(d³/G(m₁+m₂))` and flyby `d/v` at the
  linearly-predicted closest approach `d` (never below the contact distance). Quiet systems
  take one substep; a comet at perihelion or an asteroid grazing Earth automatically gets
  hundreds. Capped at 4096 (reported as *time step insufficient*).
* Positions between grid points are shown by a single extra integration step on a copy
  of the state (display only; the authoritative state is untouched).
* No physics quantity depends on frame rate. Time acceleration never enlarges `dt`: when
  the CPU cannot keep up, the simulation runs slower than requested and the UI says
  *CPU-limited*.

### Presets

| Preset | Steps per tightest orbit | Relativity | Intended use |
|---|---|---|---|
| Fast | 40 | off | Many bodies, high time acceleration |
| Balanced (default) | 120 | off | General sandbox use |
| Accurate | 400 | on | Long or delicate experiments |
| Research | 1500 | on | Small systems where precision matters more than speed |
| Custom | user | user | Expert configuration |

## Collisions

Swept-sphere test for every pair before each substep (exact for straight-line relative
motion over the substep, which is very short during encounters). On contact:

1. The geometry at contact is recorded (relative position and velocity, radii, masses).
2. The bodies merge **perfectly inelastically**: mass, linear momentum and centre of mass
   are conserved exactly; radius from volume conservation; the less massive body gets a
   `removed: merged into …` record.
3. The simulation turns the record into an **impact event** (below).

Bounce, fragmentation, cratering-without-merger and catastrophic disruption regimes are
milestone S3 (Leinhardt & Stewart 2012 scaling laws are the planned basis).

## Relativity (optional)

First post-Newtonian correction for test bodies around the most massive body
(Schwarzschild, harmonic gauge):

`a = GM/(c²r³) · [ (4GM/r − v²) r + 4 (r·v) v ]`

Valid when `GM/(rc²) ≪ 1`. Reproduces Mercury's perihelion advance (test: 43 ± 1.5″/century).
It is not a general-relativity solver: no frame dragging, no radiation reaction, no strong
field. Black holes (milestone S6) will use pseudo-Newtonian potentials plus explicit
labelling.

## Impacts → consequences (first-order models)

All in SI; TNT equivalents for display (1 Mt = 4.184 × 10¹⁵ J).

| Quantity | Model | Reference | Validity / uncertainty |
|---|---|---|---|
| Impact energy | ½ m v² at contact (v includes gravitational acceleration) | — | exact for the simulated state |
| Final crater diameter | π-group scaling: transient `D_tc = 1.161 (ρᵢ/ρₜ)^⅓ L^0.78 v^0.44 g^−0.22 sin^⅓θ`; final `1.25 D_tc` (simple) or `1.17 D_tc^1.13 / D_c^0.13` (complex, `D_c = 3.2 km · g⊕/g`) | Collins, Melosh & Marcus 2005, *MAPS* 40, 817 | rocky targets, L ≪ target radius; ±30 % |
| Severe-blast radius (≈ 5 psi) | `r ≈ 4 km · (Y/1 Mt)^⅓` | Glasstone & Dolan 1977 scaling | surface burst; factor ~1.5 |
| Global climate effect threshold | ≳ 10⁵ Mt | Toon et al. 1997, *Rev. Geophys.* 35, 41 | order of magnitude |
| Impact-winter peak cooling | log-linear between (10⁵ Mt, 0 K) and (10⁸ Mt ≈ Chicxulub, 26 K), capped at 30 K; e-folding recovery 3 yr | Brugger et al. 2017, *GRL* 44, 419; Toon et al. 1997 | factor ~2; anchored, not fitted |
| Biosphere extinction severity | log-linear between (10⁵ Mt, 0) and (10⁸ Mt, 0.75 of species), capped at 0.95 | K–Pg: ~75 % of species (Schulte et al. 2010) | anchored on one event |

Civilizations are affected through existing models: population inside the blast radius of
settlements is lost (50 % within the 5-psi radius, 100 % within the crater), and the
impact winter lowers temperature → fertile land → carrying capacity → famine, migration and
instability through the civilization model itself. There is no separate "damage" number.

Not yet modelled (later milestones): atmospheric entry, airbursts, ablation and
fragmentation (S3), ejecta and tsunamis (S8), impact-generated atmospheric loss.

## Validation tests (measured)

From `cargo test -p cosmogon_physics nbody -- --nocapture`:

| Test | Criterion | Measured |
|---|---|---|
| Two-body circular orbit, 2000 steps/orbit | position after one period | < 10⁻⁹ relative |
| Sun–Earth, 100 years, Balanced | energy / angular momentum drift | < 10⁻⁹ / < 10⁻¹⁰ |
| Sun–Earth–Moon, 10 years | Moon distance stays within 330 000–440 000 km; energy | ✅; < 10⁻⁸ |
| Equal-mass binary, e = 0.5, 100 orbits | energy error, Balanced / Accurate | 2.6 × 10⁻⁵ / 7.0 × 10⁻⁷ |
| Halley-like comet, e = 0.967, 3 orbits | energy error, Balanced / Accurate | 5.1 × 10⁻⁶ / 6.1 × 10⁻⁸ |
| Figure-eight three-body choreography | periodicity after one period | < 2 × 10⁻⁵ |
| Hyperbolic flyby | deflection vs `2 atan(GM / b v∞²)` | within 2 % (finite start distance) |
| Head-on impact at 20 km/s with a 6-hour step | detected; momentum conserved | ✅; < 10⁻⁹ |
| Mercury with 1PN | perihelion advance | 43 ± 1.5″/century |

### Against JPL's own ephemeris

`astro::horizons::tests::one_year_matches_jpl_ephemeris` starts from the JPL Horizons
state of 2026-01-01 (Accurate preset), integrates 365 days and compares heliocentric
positions with Horizons for 2027-01-01:

| Body | Error after one year |
|---|---|
| Venus, Mars, Jupiter, Saturn, Uranus, Neptune | ≤ 6 km |
| Mercury | 95 km |
| Earth | ~590 km |
| Moon | ~790 km |

The Earth–Moon residual is not yet attributed (candidates: the asteroid belt, Earth and
Moon oblateness, tides — none are modelled). Giant planets use their *system barycentres*
because their moons ride rails; using the planet centre instead costs ~50 000 km per year
(the centre wobbles ~1–2 m/s around the barycentre).

Analytic checks elsewhere: Earth orbital period, escape velocity, surface gravity, Hill
sphere, Lagrange points, Roche limit, equilibrium temperature, crater scaling against
Meteor Crater and Chicxulub-class inputs.

## Performance (measured, Apple M-series, release build)

| Case | Cost |
|---|---|
| Solar System Lab (Sun, 8 planets, Moon active; 6 rails moons), Balanced | ~560 simulated years per CPU-second (`cosmogon-cli run --scenario lab --years 100`: 177 ms) |
| Same, inside the app (9 ms per frame budget) | ≈ 300 yr/s before *CPU-limited* |
| One-year trajectory prediction (background thread) | a few ms |

## Known limitations

* Bodies are spheres with uniform density; no oblateness (J2), so nodal precession of
  moons around oblate planets is missing.
* No tides yet (tidal locking is a flag, not a torque) — milestone S3.
* Non-gravitational forces (radiation pressure, drag, outgassing, Yarkovsky) — later.
* N-body runs on the main thread inside the frame budget; very large systems run slower
  than real time requested. A worker thread is planned.
