# Measured orbit checks — 2026-10-07

This slice computes measurements from the installed deterministic engine. It changes no physical
algorithm, Universe field or golden fingerprint (0x579903fd05a28945). All results below were computed
from fixed cases, not inferred from test names or hardcoded into the interface.

## Conditions and coverage

Two massless circular test-particle fixtures start at 1 AU around GM = G × SOLAR_MASS, relativity
and collisions off, with 1000 or 2000 macro steps over exactly one analytic period. Their return
position errors are 7.93389420e-9 and 4.95852113e-10 of the initial radius. Energy and angular-momentum
changes stay below 1e-11 for these cases. Step doubling reduces this particular return error by
approximately sixteen; it does not establish convergence for every orbit or encounter.

The cached JPL case uses the embedded 2026-01-01 TDB states, the Accurate hybrid N-body/rails preset
(400 steps per shortest orbit, central-body 1PN enabled) and 365 days. It compares heliocentric,
ecliptic-J2000 endpoint positions with nine rounded locally cached 2027 coordinates:

| Quantity | Computed residual |
|---|---|
| Mercury heliocentric position residual | 94.5190195 km |
| Venus heliocentric position residual | 5.52391583 km |
| Earth heliocentric position residual | 590.841412 km |
| Mars heliocentric position residual | 3.13847074 km |
| Jupiter heliocentric position residual | 0.198132628 km |
| Saturn heliocentric position residual | 0.240557842 km |
| Uranus heliocentric position residual | 0.392722351 km |
| Neptune heliocentric position residual | 0.132487845 km |
| Moon heliocentric position residual | 786.781695 km |

The report includes 16 initial body vectors, nine endpoint coordinates, identifiers, source/retrieval
context, epoch, frame, integrator conditions and actual computed metrics. Its 1500 km / Moon 3000 km
limits are historical regression guards, not uncertainties or acceptable research precision.
Earth–Moon error is not independently attributed. J2, tides and small-body perturbations are missing.
No interval-wide bound, covariance propagation or external-integrator comparison is supplied. These
fixtures do not validate climate, stellar, collision or civilization models.

## Workflow evidence

The CLI report reproduced exactly on repeated computation. Existing report files were rejected
without replacement. Malformed numbers/flags and missing values returned status 1 without a panic.
Research creation defaults to the dynamic Solar Lab; explicit scenarios and complete saved bases
still take precedence.

A three-branch Jupiter-mass sweep measured Earth eccentricity at four states separated by ten days.
Earth mass stayed fixed at 5.97217e24 kg. Final eccentricities were 0.01587039997753068,
0.015870006479036327 and 0.015869616373502204 for input masses 9.49e26, 2.3725e27 and 3.796e27 kg.
Replay and native worker output matched every CSV row exactly. This is a short exploratory response
of the existing model, not a validated long-term planetary prediction. The primary measurement is
persisted and native comparisons display enough digits and a first-branch difference to distinguish
small computed effects; displayed digits do not imply observational precision.

Completed branches remain visible after cancellation/failure, while automation returns failure;
partial results never become a successful batch. Prepared definitions refuse a foreign pinned base
rather than silently rebinding it. Older definitions without primary_measurement remain loadable.

The workspace suite passes 212 tests, with one existing ignored test and no failures (app 44, CLI 1, core 32, physics 38, simulation 96, doctest 1). Clippy reports existing warnings only; none were added by this slice.
Artifacts are local and ignored; no public release, notarization or manual accessibility claim is made.

Final native captures show computed model residuals, populated 40-state Earth plots and completed cross-object results; the window ends above the tool dock and numeric rows remain unbroken. Scrolling is still needed for lower charts/reference rows. Direct pointer/keyboard and screen-reader use were not verified.

The final native arm64 macOS package passes ad-hoc signature and disk-image checksum verification. An ordinary packaged run lasted 39.81 seconds and recorded 100 Jupiter states without panic; packaged cross-object CSV also matches the headless study exactly. The final packaged results capture confirms the compact value/difference table. This is a local checkpoint, not a release.

## Build 59 cross-check extensions — 2026-10-23

Build 59 grew the research validation report (`research::validation`, `cosmogon-research-validation`
schema_version 1, case id `solar-lab-horizons-181-365-days`) from three to six cases without changing
any simulation law, serialized field, report interface or golden fingerprint (still
0xf2c4140a45ae509b after re-pinning in an earlier build).

Two analytic cross-checks were added:

* **Elliptical two-body fixture** — a massless particle on e = 0.2, a = 1 AU started at periapsis
  around GM = G × SOLAR_MASS, run 1000 and 2000 Yoshida-4 macro steps over one analytic period.
  It records analytic return error, quarter-period position error against
  `kepler::orbital_state_vectors(..., FRAC_PI_2, gm)`, Newtonian energy and angular-momentum
  change, semi-major-axis drift re-derived through `orbital_elements::state_vectors_to_elements`,
  and the measured convergence order `2 − log₂(err_1000/err_2000)` (observed ≈ 1.69e-4 against a
  0.01 deviation guard — Yoshida-4's empirical order sits below its nominal 4, as expected for a
  method whose leading error constant does not dominate at this step count). Observed: return
  error 1.40e-8 (1000) and 8.74e-10 (2000); quarter-period 3.62e-9 and 2.26e-10; energy drift
  5.38e-15 and 2.15e-15; angular-momentum drift 3.55e-15 and 2.29e-16; semi-major-axis drift
  5.30e-15 and 2.04e-15.
* **Integrator cross-check** — the same Plummer-softened potential integrated with classical RK4
  at 20 000 steps against Yoshida-4 at 2000 steps (equal macro-step count through the substep
  schedule); the two solutions agree to 8.73e-10 of the initial radius, so the two independent
  integrators in the engine agree to solver precision on a shared problem.

The JPL Horizons case now evaluates the nine-body sample at two epochs: the embedded 2026-01-01
TDB states plus a new 181-day mid-year epoch (2026-07-01, `data/horizons_validation_2026.rs`
`HELIO_2026`), both with the Accurate hybrid preset. Mid-year per-body position residuals:
Mercury 22.4 km, Venus 1.65 km, Earth 370.8 km, Mars 0.95 km, Jupiter 0.083 km, Saturn 0.46 km,
Uranus 0.18 km, Neptune 0.38 km, Moon 164.9 km; endpoint (365-day) residuals: Mercury 86.2 km,
Venus 5.13 km, Earth 590.9 km, Mars 3.14 km, Jupiter 0.20 km, Saturn 0.24 km, Uranus 0.39 km,
Neptune 0.13 km, Moon 786.6 km. The case also reports an `interval_max_position_residual_km`
aggregate plus Newtonian energy and angular-momentum change across the 1PN-configured span
(observed 6.94e-10 and 9.80e-11) — 21 metrics in total (9 mid + 9 endpoint + 3 aggregate).

Acceptance guards are build-59 historical thresholds at roughly 10× the observed values, not
uncertainties: `jpl_limit_km` (Mercury 1000, Venus 50, Earth 6000, Mars 30, Jupiter 2, Saturn 5,
Uranus 5, Neptune 5, Moon 8000, default 8000; interval aggregate 8000; energy 1e-7; AM 1e-8) and
fixture guards of 1e-7/1e-8 m return and quarter-period, 1e-13/1e-14 energy, 1e-13
semi-major-axis and 0.01 convergence-order deviation. They bound solver accuracy against
published ephemeris positions and exact two-body solutions — they do not quantify physical-model
error (J2, tides, small bodies are absent), ephemeris accuracy or observational uncertainty, and
the 1PN-configured span means Newtonian energy is not exactly conserved by construction.

Final verification for build 59: `cargo test --release --workspace` passes 684 tests plus one
existing ignored, zero failures (the existing `research::validation` test was extended in place
rather than split); `cargo clippy --workspace --release --all-targets` reports no new warnings;
fresh app/CLI release builds succeed. The report consumers (research UI, CLI `research validate`)
needed no interface change.
