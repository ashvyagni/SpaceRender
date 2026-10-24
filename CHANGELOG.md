## Unreleased — student→researcher UX completeness (build 60)

- Add a fifth guided tour, "From data to provenance" (`--tour data_provenance`, six steps), that narrates where every on-screen number comes from and opens the Control Panel & Research window on the relevant tab for each step (Controls, then Data, Plots & Export and Experiments) through the existing `research_requested` flow; the tour starts from the home menu alongside the existing four. A shared `provenance_legend` helper is rendered in the research Controls and Data tabs, naming each provenance badge (MEASURED, DERIVED, ESTIMATED, MODELLED, ML INFERENCE, PROCEDURAL, USER MODIFIED, UNKNOWN) with its plain-language meaning.
- Keep the tour narration card readable while the research window is open: steps that open the panel anchor the card at the top-left of the free 3D view instead of the bottom-left, and the card area joins the tool dock on egui's Foreground layer so the window can never cover it; every other step keeps the previous bottom-left position. Verified with fresh release builds, 684 release tests, the established clippy baseline and headless captures `output/qa/b60-tour.png`, `output/qa/b60-tour-normal.png`.

## Unreleased — independent scientific validation campaign (build 59)

- Grow the research validation report (`research::validation`) from three to six cases, adding three independent cross-checks against exact or published references without touching any simulation law, schema or report interface (`schema_version` stays 1, `cosmogon-research-validation`). A new elliptical two-body fixture (a = 1 AU, e = 0.2, started at periapsis with the 1000/2000 Yoshida-4 substep counts already used by the circular cases) is compared against `kepler::orbital_state_vectors` at both a full analytic return period and an exact quarter period (`FRAC_PI_2` mean anomaly): observed return errors 1.40e-8 m / 8.74e-10 m and quarter-period errors 3.62e-9 m / 2.26e-10 m, with Newtonian energy drift 5.38e-15 / 2.15e-15, angular-momentum drift 3.55e-15 / 2.29e-16, semi-major-axis drift re-derived from `orbital_elements::state_vectors_to_elements` at 5.30e-15 m / 2.04e-15 m, and a measured Yoshida-4 convergence-order deviation (observed 1.69e-4 against the nominal order 4). A new integrator cross-check case drives the identical Plummer-softened two-body potential with classical RK4 at 20 000 steps and Yoshida-4 at 2000 steps through `integrators::rk4::RK4`, agreeing to 8.73e-10 m final position. The JPL Horizons case is extended to a **mid-year plus endpoint** evaluation — case id `solar-lab-horizons-181-365-days`, epoch 2026-01-01 TDB sampled at day 181 (new `data/horizons_validation_2026.rs`, `HELIO_2026`) and day 365 (`HELIO_2027`) — yielding 9 + 9 per-body position residuals plus the aggregate maximum over both epochs and Newtonian energy/angular-momentum drift across the span (21 JPL metrics total; 6 report cases overall).
- Retain honest acceptance limits at roughly 10× the observed build-59 values, documented in the report and docs as historical accuracy guards rather than uncertainties: fixture return/quarter guards 1e-7 m (1000-step) and 1e-8 m (2000-step), energy 1e-13 / 1e-13, angular momentum 1e-13 / 1e-14, semi-major-axis drift 1e-13 m, convergence-order deviation 0.01, cross-check 1e-8 m, and per-body `jpl_limit_km` guards (Mercury 1000, Venus 50, Earth 6000, Mars 30, Jupiter 2, Saturn 5, Uranus 5, Neptune 5, Moon 8000 km; default 8000) with an 8000 km aggregate, 1e-7 energy and 1e-8 angular-momentum span guards. Observed JPL residuals (km): mid-epoch Mercury 22.4, Venus 1.65, Earth 370.8, Mars 0.95, Jupiter 0.083, Saturn 0.46, Uranus 0.18, Neptune 0.38, Moon 164.9; endpoint Mercury 86.2, Venus 5.13, Earth 590.9, Mars 3.14, Jupiter 0.20, Saturn 0.24, Uranus 0.39, Neptune 0.13, Moon 786.6; span energy drift 6.94e-10 and angular-momentum drift 9.80e-11. These checks bound solver/numerical accuracy against published ephemeris positions and exact two-body analytic solutions; they do not bound physical uncertainty in the real Solar System.
- Verify 684 release tests plus one existing ignored (build 58 baseline 684: the existing `research::validation` test was extended in place — six cases, elliptical analytic errors, convergence order, cross-check and the full 21-metric JPL set — rather than adding new tests), zero new clippy warnings in touched files, fresh app/CLI release builds, and the golden remaining `0xf2c4140a45ae509b`. [Scientific validation](docs/SCIENTIFIC_VALIDATION.md), [research validation](docs/RESEARCH_VALIDATION.md).

## Unreleased — uncertainty and controlled comparison across research panes (build 58)

- Add an authored input-band framework that turns deterministic sweep/comparison research panes into explicit low/central/high controlled studies, with honest interval arithmetic and no claim of statistical uncertainty. A new `research::uncertainty` module defines `RunBand` (`cosmogon-study-run-band`), `BandRole` (`low`/`central`/`high` with multipliers 0.5/1.0/1.5), `validate_relative_band` (finite fraction strictly in (0, 1)), and the format/method constants `cosmogon-study-uncertainty`, `cosmogon-study-band-comparison`, `authored-input-band` and `authored-input-band-interval-arithmetic`; the module doc states the band is an authored input range, never statistical uncertainty, a confidence interval, covariance or field calibration. `habitat_sweep::Plan` gains `authored_relative_band: Option<f64>` (schema_version 2 for secondary plans): when set, each swept branch runs three sub-branches at `value · (1 − band)`, `value`, `value · (1 + band)` (roles ordered low, central, high), each writing a `band.json` with parameter, relative band, role, value, swept value, `triple_final_metrics` (`at_low`/`at_central`/`at_high` with units) and run metadata, and the root manifest gains an `uncertainty` block; incomplete triples are refused ("Incomplete authored band triple") before export. A `--band` CLI option on `habitat-sweep` (0–1 exclusive) authorizes this path only: paired studies, zero swept values, out-of-range bands and other sweep commands refuse (`--band is not a habitat comparison option`), and band-less plans stay byte-identical (legacy controls). Moisture-airborne-fraction gating applies as before, so the band path reuses the established moisture study semantics.
- Add interval-arithmetic comparison envelopes for banded branch pairs: `habitat_comparison::report_with_bands` / `export_with_bands` (plain wrappers unchanged) evaluate each metric as `at_low = B_low − A_high`, `at_high = B_high − A_low`, `central_b_minus_a` from the two `triple_final_metrics`, and per-metric `max`/`min`/`unit`; when only one side carries authored band metadata the whole comparison reports `{"unavailable": "Only one compared recording carries authored band metadata; interval arithmetic needs both sides"}` rather than a false precision. The native Recordings pane reads `band.json` (sweep) and the comparison's top-level `uncertainty` block (compare) into `loaded_band`/`comparison_band`, surfaces an "Authored input-band envelope (per-run final observations)" label with per-role readouts, shows unavailable envelopes explicitly, and the sweep/compare paths read band metadata **before** moving output paths so a failed band read preserves the previous comparison ("Could not open comparison band metadata: … Previous comparison is preserved.").
- Extend the radiation record surface for band studies: four `Metric::Radiation*` variants (`RadiationExteriorDoseRate`, `RadiationInteriorDoseRate`, `RadiationShelterDoseRate`, `RadiationCumulativeDose`) join `Metric::ALL` (52 → 56), gated by `habitat::uses_radiation()` so radiation-free studies stay at 110 columns; the sweep band triple therefore carries dose metrics through `triple_final_metrics` alongside humidity, power and other final observations. Crop/heat trajectory test plans set `authored_relative_band: None` and `research/mod.rs` registers the new uncertainty module.
- Verify 684 release tests plus one existing ignored (build 57B baseline 678 + 6: `relative_band_bounds_and_run_consistency_are_enforced`, `comparison_interval_arithmetic_is_conservative_and_scoped`, `authored_band_executes_low_central_high_triples_with_manifest_and_run_metadata`, `authored_band_refuses_paired_zero_out_of_range_and_legacy_plans_stay_byte_identical`, `band_comparison_uses_interval_arithmetic_and_mixed_metadata_stays_unavailable`, `radiation_shielding_transmissions_sweep_and_bands_close_the_dose_envelope`), zero new clippy warnings in touched files, fresh app/CLI release builds and an end-to-end `/tmp/b58` loop (moisture-airborne-fraction sweep at 0.2/0.4/0.6 with `--band 0.5 --runs 3` → 9 branches; per value roles ordered low, central, high with run-000 low 0.1 … run-008 high 0.9; every run writes `band.json` with `triple_final_metrics`, root manifest `uncertainty` block; comparison dirs contain only `comparison.json` with top-level `uncertainty` (`format: cosmogon-study-band-comparison`, `method: authored-input-band-interval-arithmetic`, e.g. humidity `at_high` 36.445 %, delivered power `at_high` 169.738 W, electrolysis oxygen unavailable "Triple final values incomplete on one side"), mixed-side comparison unavailable; refusals for paired/zero/out-of-range/non-sweep `--band`; no-band control byte-identical in shape; two inspected native captures — envelope pane and band field pane, `sweep: null` in the band-field capture EXPECTED) plus the band-before-path-move preservation path. [Evidence](docs/HABITAT_COMPARISONS_VALIDATION.md), [guide addendum](docs/HABITAT_COMPARISONS.md).

## Unreleased — breach inward reverse flow (build 57B)

- Add opt-in reverse (inward) orifice flow to the existing pinned cabin-breach model (`cosmogon-cabin-breach/1` deliberately unchanged so every existing breach save reopens; legacy configurations, worlds, recordings and the golden `0xf2c4140a45ae509b` stay byte-identical because the feature defaults off and its two new `Config` fields — `allow_inward_flow` (false) and `inward_temperature_k` (250 K, validated 100–500 K) — are `serde(default)` + skip-when-default). When the receiver pressure exceeds cabin pressure and the flag is set, the same NASA choked/subcritical law runs with the receiver as upstream (`inflow_rate`; rates signed negative = inward) and the same step-doubled RK4/Budget machinery, integrating `dn/dt = +ṁ/M_in` toward receiver equilibrium. Inflow composition is an authored assumption — dry air at 21% O₂ / 79% N₂ by mole, no vapor or CO₂, no reservoir depletion — at the authored receiver temperature; the thermal path uses the constant-gamma imported-enthalpy relation `T(n) = (C·T₀ + Δn·cp·T_in)/(Cs + n·cv)` (monotone, so at most one temperature support bound is reachable: new `InflowTemperatureBoundary` status, plus supported `InflowComplete`; `Equilibrium`/`Closed`/domain statuses reused) and reports imported enthalpy as **negative outflow enthalpy** so the structure/gas/phase/residual identity closes exactly as outward; the lumped cooling debit may be negative (net heating). `Budget` gains skip-when-zero `inflow: Gas`; habitat interval stocks apply `−escaped + inflow` with cumulative ledgers `Totals.inflow_gas`/`Totals.inflow_enthalpy_j` (skip-if-absent/zero), `breach_enthalpy_j` accumulating only the nonnegative outflow part, and the finite mass identity extended to cabin + escaped = original + cumulative inward.
- Evidence: recording closed-mass validation, the pressure-report substep gas derivation and the heat-ledger breach term (index 9, now signed with its monotone check relaxed) all include inward transfers; a `breach_inflow` diagnostics term plus an inward sentence in the breach explanation; report limits drop "inward" from the unmodelled-leak list; updated `breach::source()` and `breach`/`habitat` registry descriptors document the inward assumptions and non-claims; native Breach pane renamed "Cabin breach orifice flow" with staged "Allow inward flow…" checkbox, inward temperature field, direction-aware Overview (↔ header, cumulative inward ledger, signed rate/enthalpy labels, inward dry-air interval row) and an updated Model pane. CSV columns are unchanged at 110 — inward evidence rides retained budgets/totals/manifest — and the model pin stays `cosmogon-cabin-breach/1`. No measured inward composition, reservoir depletion, seal inward path, inflow condensation or variable properties; uncertainty remains unquantified.
- Verify 678 release tests plus one existing ignored (build 57A baseline 674 + 4: three inward breach unit tests and one inward world coupling test), zero new clippy warnings in touched files, and an end-to-end CLI loop (25-sample Mars study, receiver 110 kPa, 110-column CSV, cabin 101.140 → 109.9998 kPa with inward-only transfers, exact 21/79 composition over 53.21 kg cumulative inflow, closed mass/energy identities, inspect/diagnose/pin evidence, flag-off legacy control byte-identical in shape) plus three inspected native captures. [Guide](docs/CABIN_BREACH.md), [design addendum](docs/CABIN_BREACH_DESIGN.md), [verification addendum](docs/CABIN_BREACH_VALIDATION.md).

## Unreleased — radiation environment and cabin shielding dose (build 57A)

- Add an authored exterior-radiation environment and cabin-shielding dose model (`cosmogon-habitat-radiation/1`, optional `habitat::Config.radiation`, default-absent so legacy worlds/saves/recordings and the golden stay byte-identical): the habitat environment sample evaluates one closed-form authored law from existing world inputs — `exterior [mSv/day] = 1.8 × (1 + 340 × p_bar)^(-0.89) × (1 − 0.3 × B_earth) × (1 + 4 × flare_activity)` — whose constants interpolate published order-of-magnitude anchors (MSL/RAD interplanetary ≈1.8, Mars surface ≈0.67, Earth surface GCR component ≈0.01; authored anchor interpolation, not a fit, with clamped input domains and unavailable (never zero) above 1e4 mSv/day). When configured, each completed interval derives `interior = exterior × wall_transmission` (default 0.5), `shelter = interior × storm_shelter_transmission` (default 0.25) and `interval_dose = interior × dt / 86400`, retains them in `Budget.radiation` with an authored hazard band (`nominal` ≤ 50/365.25, `elevated` ≤ 1.0, `above_design` above, `unavailable` without evidence; ICRP-context design categories, not safety determinations), and accumulates `Totals.cumulative_dose_msv` (skip-if-zero, bounded, replay-revalidated). An eighth appended alert (bit 7, existing positions untouched) fires when the interior rate exceeds the authored `alert_dose_rate_msv_day` (default 1.0) with a radiation-specific Chronicle detail. `Config` keeps `deny_unknown_fields`; bad transmissions/thresholds are refused through the ordinary `Config::validate` path.
- Evidence: conditional `cosmogon-habitat-radiation/1` save pin enforced on reopen; six optional `samples.csv` tail columns (`radiation_exterior/interior/shelter_dose_rate_msv_day`, `radiation_interval_dose_msv`, `radiation_cumulative_dose_msv`, `radiation_hazard_band`; empty cells mean unavailable, never zero) taking accepted widths 110 → 116 only when radiation evidence exists; report `radiation_observation`/`radiation_source`/`source.radiation`/`data_states.radiation` (ESTIMATED design inputs) with the `limits` text now listing radiation as optional and separately sourced; a `radiation` diagnostics term group (serde-skipped while empty so old diagnostic exports keep their exact shape) plus explanation and diagnostic-report source; conditional recording manifest keys replay-validated against the live source and the new `habitat-radiation` registry descriptor (with `msl-rad` and `icrp-dose-limits` scientific sources); native Habitat **Radiation shielding** configuration group staging the checkbox, transmissions and threshold through the ordinary draft Apply path. No solar-particle-event forecast, particle transport, activation, material attenuation data, dosimetry or medical/ALARA claim; uncertainty is unquantified.
- Verify 674 release tests plus one existing ignored (build 56 baseline 662 + 12: ten `habitat_radiation` unit tests and two habitat/save integration tests), zero new clippy warnings in touched files, fresh app/CLI release builds and an end-to-end Mars loop (25-sample study with 116-column CSV; exterior 0.6713548408966546 mSv/day reproduced from Mars's actual surface pressure against the ~0.67 anchor, exact ×0.5/×0.25 transmissions, interval/cumulative arithmetic exact over 1440 fixed minutes, elevated band, report/manifest/save-pin/diagnose evidence, wall-transmission 1.5 authoring refusal, 110-column radiation-free control) plus two inspected app captures. [Guide](docs/RADIATION_SHIELDING.md), [verification](docs/RADIATION_SHIELDING_VALIDATION.md), [design](docs/RADIATION_SHIELDING_DESIGN.md).

## Unreleased — life-support calibration and sensitivity envelopes

- Add authored life-support calibration and retained sensitivity bands (`cosmogon-life-support-calibration/1`, optional `habitat::Config.calibration`, skip-if-none so legacy worlds/saves/recordings and the golden stay byte-identical): four `Factor`s — `crew_demand`, `scrubber`, `recycler`, `oxygen_makeup` — each carrying a `central ∈ [0.1, 10]` multiplier applied in place to its published reference expression (crew demand additionally scales the three crew-water planning reservations) and a `relative_band ∈ [0, 1]` around the applied result; the per-interval `Budget.calibration` retains four `Bound { applied, low, high }` blocks (`crew_cycle_person_days`, `oxygen_makeup_kg`, `co2_captured_kg`, `water_processed_kg`) with `low = max(0, applied·(1−band))` and `high = applied·(1+band)`, every bound replay-revalidated against retained configuration. No new `Edit` variant and no new CLI commands; bad central/band values are refused before any step runs; `Config` keeps `deny_unknown_fields`.
- Evidence: the conditional `cosmogon-life-support-calibration/1` save pin, a 12-column additive `calibration_*` `samples.csv` tail (110 → 122 columns present only when calibration is configured), report `calibration_observation` (model + four factors + per-interval applied/low/high) and `calibration_source` (law, authored-bounds limits, ESTIMATED version note) plus `data_states.life_support_calibration` and `uncertainty_scope` stating the bands are not measured variance or confidence intervals; a "calibration" `habitat-diagnose` explanation; native Habitat **Life-support calibration** controls block (Controls and Settlement panes) staging central/band values with the same staged-Apply workflow.
- Verify 662 release tests plus one existing ignored (build 55 baseline 651 + 11: six `habitat_calibration` unit tests and five habitat world/lifecycle tests), zero new clippy warnings in touched files, fresh app/CLI release builds, an end-to-end loop (four-factor study → 122-column CSV with exact band arithmetic at the first nonzero row → report observation/source/data-state/uncertainty keys → `final.cosmo` pin → diagnose explanation → central 15.0 and band 2.0 refusals) and two inspected native captures. [Guide](docs/LIFE_SUPPORT_CALIBRATION.md), [verification](docs/LIFE_SUPPORT_CALIBRATION_VALIDATION.md), [design](docs/LIFE_SUPPORT_CALIBRATION_DESIGN.md).

## Unreleased — finite settlement construction and logistics

- Add continuous cabin construction (`cosmogon-cabin-construction/1`, optional `habitat::Config.construction`, default-absent so legacy worlds/saves/recordings and the golden stay byte-identical): an authored `material_capacity_kg` ∈ (0,1e7] bounds held processed material, `build_rate_kg_s` ∈ (0,1e4] sets consumption at full served power, `power_w` ∈ [0,1e6] requests a tenth/lowest-priority ordinary bus slot, and `volume_per_kg_m3` ∈ (0,1] is the structural bulk ratio. Each completed interval consumes `min(held, build_rate · dt · served)` further bounded so live volume never exceeds the authored 1e6 m³ bound, converts consumed mass into `construction_structure_kg` (held + structure both in `mass_kg()` — total mass conserved exactly), and applies the volume increase to the **live configuration at interval end after the retained budget configuration is captured**, so the next interval's gas pressures reflect the larger cabin at unchanged gas mass while each budget keeps the pre-add volume it ran at. Delivered power is ordinary bus electricity and becomes cabin heat; thermal mass deliberately unchanged. Material arrives only through the journaled `Edit::SupplyHabitatConstructionMaterial` (`habitat-construction-supply`), one bounded charge per invocation that refuses to exceed capacity; capacity-below-held reconfiguration refuses without discarding stock, disabling freezes held material.
- Add habitat-to-habitat stock transfer (`cosmogon-habitat-stock-transfer/1`): `Edit::TransferHabitatStock {system, from, to, requested: supply_vehicle::Cargo}` (`habitat-stock-transfer`) moves the four finite supply stocks between two habitats of the same body — both stepped to the edit clock first, per species `accepted = min(requested, sender held, recipient free)` with exact `held + accepted ≤ capacity` clamping, zero-accepted and cross-body/same-habitat refusals applying nothing. Conservation is exact (sender decrease == receiver increase per species); each side retains a paired `StockTransfer {model, counterparty, world_time_s, moved, outbound}` record re-validated on reopen (`validate_stock_transfers` keys pairing on time and masses), with conditional save pins whenever records or edits exist.
- Evidence: conditional `cosmogon-cabin-construction` / `cosmogon-habitat-stock-transfer` save pins; five optional `samples.csv` construction tail columns (`construction_material_kg`, `construction_requested_w`, `construction_delivered_w`, `construction_volume_added_m3`, `total_constructed_volume_m3`) with accepted widths extended by the new band; a "construction" diagnostics explanation plus report `construction_observation`/`construction_source` and a `source.construction` entry in the diagnostic report; native Habitat **Settlement** tab (index 13, Overview/Settings/Model & sources) with construction readouts, staged construction controls, supply authoring, transfer authoring (from/to combos plus four species amounts) and retained transfer history; CLI `habitat-construction-supply` and `habitat-stock-transfer` with protected before-worlds, action JSONs with before-state fingerprints and ordinary export folders.
- Verify 651 release tests plus one existing ignored, zero new clippy warnings in touched files, fresh app/CLI release builds, an end-to-end loop (two-habitat Earth world → 400 kg supply → 12-day study consuming all 400 kg into 400 kg structure at +4 m³ live volume with a 115-column CSV → mid-construction study with exact 0.03 m³/interval growth → exact 10/30/5/4 kg paired conserved transfer → diagnose explaining construction → three refusals) and two inspected native captures. [Guide](docs/SETTLEMENT_CONSTRUCTION.md), [verification](docs/SETTLEMENT_CONSTRUCTION_VALIDATION.md), [design](docs/SETTLEMENT_CONSTRUCTION_DESIGN.md).

## Unreleased — equipment calibration and scheduled maintenance

- Add scheduled wear, maintenance windows and authored capacity calibration for powered cabin life-support equipment (`cosmogon-cabin-maintenance/1`, optional `habitat::Config.maintenance`, default-absent so legacy worlds/saves/recordings and the golden stay byte-identical). Each configured component (scrubber, recycler, heater, radiator, moisture condenser, electrolysis, Sabatier — canonical order shared by state, budgets and effects; crops keep their own maintained-chamber assumption) carries an authored `wear_per_hour` (condition fraction lost per served operating hour), `repair_per_hour` (restored per maintenance hour, must exceed zero), a repeating cycle of `interval_hours` + `duration_hours` intersected exactly with each simulation interval including wrap-around, a `maintenance_power_w` draw that replaces process demand with ordinary bus electricity during the window, and a `capacity_calibration` multiplier in (0, 1] applied to the nominal rating (values above 1 are refused because rated process ceilings are the modeled maximums). A two-phase step brackets the ordinary power allocation: plan from start-of-interval condition computes per-component scale, operating/maintenance shares and time-prorated maintenance power; settle applies wear proportional to the whole slot's served fraction, restores during the window and advances cycle positions. Component effects scale scrubber capture, recycler streams, moisture plan, heater limit and radiator rejection; a windowed component produces nothing and its maintenance electricity becomes cabin heat through the existing ledger. Component-without-equipment configurations are refused; disabling freezes held condition and cycle position (re-enabling resumes them); budget and state validate re-derive the wear/restore arithmetic against retained configuration.
- Evidence: per-component held `state.maintenance` condition and cycle position, per-interval `budget.maintenance` accounting, cumulative `totals.maintenance_hours`, conditional `cosmogon-cabin-maintenance` save pin, a "maintenance" diagnostics explanation, four optional `samples.csv` tail columns present only when maintenance evidence exists, a report maintenance observation with source limits, and a native Habitat **Maintenance** pane (Overview / Settings / Model & sources) that reads conditions, interval accounting and cumulative hours and stages per-component schedules and rates.
- Verify 639 release tests plus one existing ignored, zero new clippy warnings in touched files, fresh app/CLI release builds, an end-to-end Mars study (derated scrubber wear/restore cycle across maintenance windows with accumulated maintenance hours, 114-column CSV with the maintenance tail, diagnose explaining the mechanism), cycle-split exactness across phase boundaries, wear/restore rates and clamps, calibration halving effective capacity, refusals leaving configuration and state unchanged, save roundtrip with pin tamper/refusal, legacy no-maintenance serialization without any new keys, determinism across identical worlds, and the golden remaining `0xf2c4140a45ae509b`. [Guide](docs/EQUIPMENT_MAINTENANCE.md), [verification](docs/EQUIPMENT_MAINTENANCE_VALIDATION.md), [design](docs/EQUIPMENT_MAINTENANCE_DESIGN.md).

## Unreleased — physical wall condensation and nominal structural leak

- Add two passive/structural physical couplings for the finite cabin moisture system, both optional and default-absent (legacy worlds, saves, recordings and the golden stay byte-identical). `moisture::WallConfig` (`cosmogon-cabin-wall-condensation/1`, optional `habitat::Config.wall_condensation`) models signed vapor↔wall-film exchange as lumped surface mass transfer toward NIST saturation at an explicitly assumed wall temperature `T_wall = T_exterior + coupling·(T_cabin − T_exterior)`, bounded by available vapor, held film mass, a `liquid_capacity_kg` film bound and a finite rate cap; outside the 273.15–303 K saturation domain no exchange occurs (retained constraint note), no ice/fog/drips are modelled and wall liquid has no recycling path. Phase heat `net·44.004 kJ/mol` enters the lumped thermal balance exactly like equipment condensation. `habitat::LeakConfig` (`cosmogon-cabin-structural-leak/1`, optional `Config.leak`) removes `min(rate·dt, total gas)` of start-of-interval cabin gas proportionally to species masses, cools the cabin by the assumed constant cabin-gas specific heat enthalpy, and joins the cumulative `escaped_gas`/`totals.leaked_gas` ledgers before the breach substep so leak and breach compose within one interval. State gains skip-if-zero `wall_liquid_kg`; totals gain signed `wall_condensed_kg` and `leaked_gas`; budgets gain `wall`/`leak` blocks; the vapor identity, wall-film identity, cumulative≥interval checks and conditional save pins are enforced; heat-ledger worlds gain the optional `wall_leak_storage_j` supplement (`wall_condensation_heat`, `leak_cooling` span terms, original eleven-term array preserved).
- Evidence surfaces: `samples.csv` gains optional tail columns (`wall_liquid_kg`, `total_wall_condensed_kg`, `leaked_*`, `total_leaked_gas_kg`) present only when wall/leak evidence exists (old recordings re-derive byte-exactly); `habitat-diagnose`/WHY gains "wall" and "leak" explanations plus wall/leak water and heat terms; the report gains wall/leak observations and `leak_source`; the native humidity pane reads wall film, signed net exchange, wall temperature, cumulative leak and interval enthalpy removal, and stages wall/leak configuration blocks alongside moisture; registry sources document the lumped assumptions and non-claims.
- Verify 629 release tests plus one existing ignored, zero new clippy warnings in touched files, fresh app/CLI release builds, an end-to-end Mars study (leak removing proportional gas with enthalpy cooling; wall exchange correctly domain-refused at 210 K walls with retained constraint; 117-column CSV with wall/leak tails; diagnose explaining both), wall condensation condense/evaporate/domain/cap unit and world tests, leak+breach same-interval composition, heat-span residual closure with both supplements, refusals leaving state unchanged, legacy serialization without any new keys, and the golden remaining `0xf2c4140a45ae509b`. [Guide](docs/WALL_CONDENSATION_LEAK.md), [verification](docs/WALL_CONDENSATION_LEAK_VALIDATION.md), [design](docs/WALL_CONDENSATION_LEAK_DESIGN.md).

## Unreleased — physical greenhouse installation and finite habitat-stock exchange

- Add two ordinary undoable habitat edits closing the last physical gaps between the finite crop process and the cabin. `Edit::InstallHabitatCrop` (`cosmogon-cli habitat-crop-install --initial SAVE --habitat-id ID --definition CROP_CONFIG --nutrient-kg KG --output NEW_DIR`) installs a greenhouse process into an existing **cropless** habitat at runtime, granting exactly one explicit bounded finite nutrient-equivalent charge (0…1e7 kg, journaled); water, captured CO₂, oxygen headroom, power and dry food are never granted. Validation refuses a second install, invalid crop/candidate configurations (moisture accounting required for crop vapor; the crop O₂ product ceiling must fit the shared reserve), an O₂ reserve above the product capacity and out-of-bounds charges; apply completes the current interval, arms heat-ledger crop debits at zero when a ledger exists, and re-runs full configuration checks. Because recordings require a constant original definition, a runtime-installed crop pins to the retained live configuration (source crop and dry chemical basis) plus the crop state's self-pins; `check_config` **refuses disabling** it (the live configuration is the only carrier of its operational parameters), while authored crops keep disable-retains-inventory. The escaped-mass identity adds the runtime crop's initial nutrient charge for habitats whose original definition had no crop.
- `Edit::StoreHabitatCropFood` (`habitat-crop-store --initial SAVE --habitat-id ID [--store-kg KG] --output NEW_DIR`, defaulting to the full held amount) moves exactly `min(request, held harvested edible, food-tank headroom)` kilograms 1:1 into `state.food_dry_kg` — the finite tank the nominal crew cycle already draws at 0.800 kg per reference person-day — after an explicit completed harvest; the rejected remainder stays in the harvested store, refusals change neither side, and inedible dry material never converts. Harvest semantics are unchanged (separate stores, no diet credit, no restart). Crop state gains a cumulative `food_stored_kg` counter (`#[serde(default, skip_serializing_if = zero)]`: legacy saves, recordings and never-stored crop worlds serialize byte-identically); `chemical_energy_j()` counts stored material so heat-ledger debits stay consistent, and validate gains counter bounds plus the harvested-ledger identity (harvested + stored = pinned 30-day curve edible; stored requires the explicit harvest state).
- Installations may span recorded observations: `habitat_crop_trajectory::validate_series` allows leading rows without crop state with the explicit origin at the first crop row (crop never disappears and the origin never changes from there; counters count standing + stored), `crop.csv` gains one optional `food_stored_kg` column present only when a positive conversion exists (old CSVs re-derive byte-exactly), and balance bundles still refuse stock-set-changing ranges. `LIMITS` and the `finite-crop` registry descriptor state installation, the bounded store conversion and the explicit non-claims (no diet adequacy, no metabolizable energy). Native Installed crops overview gains the install control (staged from New crop setup) for cropless habitats, the store control with an explicit amount, and a cabin dry-food tank readout; Settings points cropless habitats at the install control and marks runtime crops non-disableable. CLI copies the before-world, writes an action JSON with state fingerprints and exports an ordinary recording folder for both commands.
- Verify 622 release tests plus one existing ignored, zero new clippy warnings in touched files, fresh app/CLI release builds, a full end-to-end loop (`habitat-crop-install` 2 kg nutrient charge → 30-day growth to standing [0.7036212416499155, 0.03703770526983985] kg edible/inedible → harvest → full store into `food_dry_kg: 75.90362124188393` with crop counter `0.7036212416499155` → exact 0.800 kg/day nominal crew diet draw; grown-balance residual −6.417089082333405e-14 kg over 26 stocks with crop standing +0.7036212416499155 kg and nutrients −0.14368726475222515 kg; double-install, pre-harvest-store, cropless-store and negative-charge refusals; exact undo/redo JSON round trips; inspected native capture) plus regression coverage that legacy saves/recordings stay byte-identical and the golden remains `0xf2c4140a45ae509b`. [Guide](docs/PHYSICAL_GREENHOUSE.md), [verification](docs/PHYSICAL_GREENHOUSE_VALIDATION.md).

## Unreleased — end-to-end supply-mission closeout

- One authored Mars supply mission — surface launch → orbit insertion → deorbit → atmospheric entry → touchdown → supported hold → atomic habitat delivery — now runs as a **single mission-span study** with complete retained evidence. `flight-definition --mission-span` (requires `--launch FILE`) admits raised but still bounded work ceilings (300 M nominal fleet substeps, 3 G gravity-source visits, 256 MB recording) and `--transfer FILE` authors the atomic habitat transfer at an exact sample boundary; admission only, no physics/sampling/determinism change and legacy studies serialize byte-exact. `Study::new_mission`/`mission_span` is a serde-skipped default-false field.
- Add consolidated `manifest["mission"]` (`cosmogon-world-vehicle-mission/1`): per-phase evidence with actual retained values (launch liftoff/burnout/apogee/peak force, orbit insertion summary from the retained osculating elements, entry interface/flux/fluence, landing touchdown speeds, hold window vs `required_window_s`, delivery accepted cargo + habitat receipts) and the single `complete` gate (`liftoff && launch_ok && orbit_ok && entry_ok && touchdown_ok && window_ok && delivered_positive`). Orbit gating is the insertion evidence (elliptical state with periapsis above the entry interface, e < 1); the whole-span minimum periapsis is retained as informational `minimum_periapsis_above_entry_interface` and never gates completion, because a surface launch and a authored entry both cross the interface legitimately. Mission-span recordings retain `orbital.csv`; `flight-inspect` replays every value. The native Recorded flight pane renders the mission evidence chain with per-phase pass/fail rows and completion status (inspected capture).
- Correctness fixes exposed by the full-span flight: foot shear resets to unloaded whenever the normal load vanishes or the foot is separated (free-flight arcs no longer transport contact normals into the 0.02 rad refusal); the supported-hold window uses cargo-ledger-aware `Flight::mass_kg()` instead of the plan-definition mass (an accepted transfer no longer fakes post-transfer ground-relative acceleration); operation sampling tolerates equal representable craft-clock instants (task-tail ulp fragments) while still refusing a backwards clock; `trajectories.csv` reports the current cargo-aware vehicle mass and held cargo after a transfer.
- The mission is authored, not guided: an inertial command sequence from the closed-loop replica (`/tmp/op51e2e/design.py`) whose touchdown aligns with engine truth (~0.3 s of the design stop), with the terrain datum calibrated to `ground0 = 694.18` (`TERRAIN_OFFSET = 2030.58`, `gap@end = ground0 − 693.88 m` engine-vs-replica bias in this profile family), per-foot `sliding_friction = 0` plus `cosmogon-finite-foot-shear/1` for a persistent hold, and the habitat re-sited to the observed ballistic landing footprint (lat −0.000004°, lon 96.101386° E) so delivery is genuinely co-located. No guidance, certification or mission-feasibility claim.
- Verify 618 release tests plus one existing ignored, no new clippy warnings in touched files, fresh app/CLI release builds, a full end-to-end loop (`flight-definition --mission-span --transfer` → `flight-study` with `manifest["mission"].complete == true`: liftoff 0.75 s / apogee 98362.95792136295 m / interface 1437.757812488079 s / touchdown 2383.469531249964 s at 1.3807864117485245 / 0.01820571388295438 m/s / hold 237.6640625 s ≥ 5.0 s / 170.0 kg delivered with 1 receipt → replay-exact `flight-inspect` → byte-identical rerun → inspected native capture) plus regression coverage for the three fixed defects. [Guide](docs/MISSION_CLOSEOUT.md), [verification](docs/MISSION_CLOSEOUT_VALIDATION.md).

## Unreleased — entry heating and terminal landing limits

- Add `Definition.entry` / `Flight.entry_ledger` (`cosmogon-vehicle-entry/1`): opt-in versioned stagnation-point heating limits (interface altitude, nose radius, Sutton-Graves flux coefficient, flux/total-heat bounds) plus retained thermal evidence (sticky interface crossing on the craft propulsion clock, running peak flux, integrated fluence, substeps) integrated on the accepted contact substeps that already resolve the gear against the parent's existing exponential bulk atmosphere. Add `Definition.landing` / `Flight.landing_ledger` (`cosmogon-vehicle-landing/1`): opt-in versioned first-touchdown speed limits (vertical/horizontal ground-relative, liftoff bound above the authored start clearance so pad settling never counts) plus retained touchdown evidence (time, speeds, start clearance, airborne flag, substeps) on the bounded gear solver. Both stop the craft atomically at the last supported macro state with `Phase::Unsupported` carrying both numbers when a bound is exceeded; entry requires gear and an atmosphere-bearing parent, landing requires gear.
- Evaluate both ledgers after every accepted contact substep; `validate_saved` requires each ledger exactly when its limits exist and revalidates on reopen; conditional `cosmogon-vehicle-entry` / `cosmogon-vehicle-landing` save pins; `vehicle-entry`/`vehicle-landing` research-registry descriptors and model/limits study context; conditional `manifest["entry_evidence"]`/`manifest["landing_evidence"]` with exact `flight-inspect` replay. CLI `entry-definition`/`landing-definition` author pad-resolved coast definitions; `flight-definition --entry FILE|--landing FILE` reads the whole definition file and requires contact gear or a launch definition. Native world-vehicle/recorded/files panes gain conditional limits/evidence readouts, disclaimers and geographic carry-over notes above the flight details. Legacy flights serialize byte-exactly and the golden remains `0xf2c4140a45ae509b`.
- Verify 613 release tests plus one existing ignored, no new clippy warnings in touched files, a fresh app/CLI release build and an eleven-case end-to-end loop (entry study with interface crossing 13.234375 s, peak 44117.097866207376 W/m² and total 302775.0217576408 J/m², pad-settling nulls as designed, touchdown at 0.22021484375 s with 0.8176573684913913/0.016955761270384735 m/s, two atomic-limit negatives, four flag negatives, build-48/49 legacy replay regressions, inspected native captures). [Guide](docs/VEHICLE_ENTRY_LANDING.md), [verification](docs/VEHICLE_ENTRY_LANDING_VALIDATION.md).

## Unreleased — osculating orbital evidence and craft rendezvous

- Add derived orbital evidence for free-flight studies: `vehicle_orbit` (`cosmogon-orbital-evidence/1`) computes instantaneous two-body elements (`osculating`: a from −μ/2E, eccentricity vector, inclination via z-clamped acos; hyperbolic apoapsis/period are `None`, never zero) from each retained observation, using the exact `GravitySample` the craft felt plus `system.body_state` at the craft clock — never origin-time reconstruction, with sample/body/radius/μ mismatch refusals. Every free-flight study records a byte-exact-replayable `orbital.csv` plus a conditional `manifest["orbital"]` summary (initial/final elements, minimum periapsis, maximum apoapsis, `closed_at_final`); contact studies omit both and refuse a stray CSV. A `vehicle-orbital-evidence` registry descriptor (sources `horizons`, `solar-facts`) is added.
- Add CLI/native craft-to-craft rendezvous: `world_vehicle_rendezvous` (`cosmogon-vehicle-rendezvous/1`) independently replays two complete flight recordings, requires one origin fingerprint, one system and one exact observation grid, and derives closest-approach evidence (index/time, distance, relative speed and position) over the shared times. `flight-rendezvous --recording A --reference B [--output NEW]` exports a protected report (`definition.json`, `rendezvous.json`, `rendezvous.csv`) or prints the manifest; readers re-verify by recomputation. The native World flight pane gains a live "Osculating orbit about the parent body" readout for craft without contact hardware.
- No universe state, simulation law or save-schema change: derived evidence carries no save pin, legacy recordings read unchanged (missing `orbital.csv` is absent evidence) and the golden remains `0xf2c4140a45ae509b`.
- Verify 596 release tests plus one existing ignored, no new clippy warnings in touched files, a fresh app/CLI release build and a thirteen-case end-to-end loop (circular study with initial `e = 3.12e-13` and honest N-body drift, equatorial geographic study whose inclination reproduces Mars's 25.1886° obliquity and speed matches circular to 5e-9 m/s, rendezvous at 342909.446772 m closest with distance varying 0.34–7.26 Mm, replay-exact inspect, six negative cases, build-48 contact-recording regression, inspected native capture). [Guide](docs/VEHICLE_ORBIT.md), [verification](docs/VEHICLE_ORBIT_VALIDATION.md).

## Unreleased — authored finite surface launch

- Add `Definition.launch` / `Flight.launch_ledger` (`cosmogon-vehicle-launch/1`): opt-in versioned structural limits (sensed specific-force bound, liftoff height above the authored start, surface-start height bound) plus retained trajectory evidence (liftoff/burnout on the propulsion clock, running post-liftoff apogee above the local terrain, peak specific force, consumed propellant, observed substeps). Exceeding the bound stops the craft atomically at the last supported macro state with `Phase::Unsupported`; launch-limited craft structurally require contact landing gear and `Flight::new` refuses non-surface starts using the contact sampler's radial clearance.
- Evaluate the ledger after every accepted contact substep (one extra local surface sample per launch substep); `validate_saved` requires the ledger exactly when limits exist and revalidates it on reopen; conditional `cosmogon-vehicle-launch` save pin; a `vehicle-launch` research-registry descriptor and launch model/limits study context.
- Add CLI `launch-definition` (pad resolved on actual local terrain, fixed inertial zenith burn on the example bench at the authored thrust, authoring-time refusal when the sensed force would exceed the bound) and `flight-definition --launch FILE` (mutually exclusive with geographic/contact/plan options); native launch limits/trajectory-evidence readout. Legacy flights serialize byte-exactly and the golden remains `0xf2c4140a45ae509b`.
- Verify 584 release tests plus one existing ignored, no new clippy warnings in touched files, a fresh CLI release build and a seven-case end-to-end loop (definition → study with liftoff 0.79296875 s, burnout 10.00390625 s, apogee 309.48951319931075 m, peak 12.820512820512821 m/s², 50.98581064899918 kg consumed → replay-exact inspect, three negative cases). [Guide](docs/VEHICLE_LAUNCH.md), [verification](docs/VEHICLE_LAUNCH_VALIDATION.md).

## Unreleased — atomic co-located habitat transfer

- Add `Edit::TransferVehicleCargoToHabitat` (`cosmogon-vehicle-transfer/1`): one atomic undoable edit that moves held cargo from a supported craft into finite habitat storage, or refuses and changes nothing. The plan layers the build-46 handling preflight with a modeled spherical great-circle reach (`reach_m`, default 1e7, bounded) and a window-scaled mass-rate envelope (`mass_rate_kg_s × current_window_s`, end-first deterministic trim; trimmed mass stays on the craft), then ulp-clamps `held + accepted ≤ tank` exactly.
- Commit both sides or neither: the recipient is stepped to the edit time first so plan and receipt see identical post-step stocks; the craft side reuses the co-moving impulse-free `remove_cargo` (velocity unchanged, zero-relative cargo event recorded); the habitat side adds stocks plus one `Receipt`. Retained `Transfer` records carry window, separation, reach, rate envelope, ground-relative velocity, ground-absorbed momentum/kinetic energy and declared power/thermal (evidence and assumptions only). `validate_transfers` re-derives ordered cargo-event matching and receipt agreement on reopen; conditional `cosmogon-vehicle-transfer` save pin.
- Add authored study transfers applied at exact sample boundaries (before the observation), conditional `manifest["transfers"]` and `transfers.csv` with exact `flight-inspect` replay, CLI `habitat-setup` (protected habitat-bearing world) and `flight-definition --handling`, a native co-located transfer section and habitat receipts readout. Legacy worlds serialize byte-exactly and the golden remains `0xf2c4140a45ae509b`.
- Verify 575 release tests plus one existing ignored, no new clippy warnings in touched files, a fresh CLI release build and an eight-case end-to-end loop (habitat-setup → handling definition → full 170 kg transfer with a real 12.38671875 s supported window, replay-exact inspect, legacy stability, six negative/clamp cases). [Guide](docs/VEHICLE_TRANSFER.md), [verification](docs/VEHICLE_TRANSFER_VALIDATION.md).

## Unreleased — finite handling equipment and recipient storage capacity

- Add four optional finite habitat general storage tanks (defaulting to the historical 1e7 kg bound, byte-exact for legacy definitions) that bound initial stock, held stock and reconfiguration per transferable species, with an O₂-tank-above-installed-equipment-capacity coupling and potable-recovery clamping that keeps overflow as brine; add `Habitat::held_cargo`/`storage_capacity_cargo`/`available_cargo` queries.
- Add opt-in authored craft handling equipment (`cosmogon-vehicle-handling/1`) with per-species, total, shared oxygen-interface and required-operation-window limits plus declared-but-unmodeled power/thermal assumptions (structurally requiring cargo ledgers and surface-operation hardware), and a pure request/accept/reject preflight with a deterministic trim order that keeps rejected stock explicit on the craft. Nothing transfers, is received or delivered.
- Add conditional `manifest["handling"]` study evidence with exact `flight-inspect` replay, a native tanks/readout exposure and a conditional `cosmogon-vehicle-handling` save pin. Legacy worlds serialize byte-exactly and the golden remains `0xf2c4140a45ae509b`.
- Verify 562 release tests plus one existing ignored, no new clippy warnings in touched files, a fresh CLI release build and a seven-case end-to-end loop (contact-study handling manifest, geographic landing with a real 9.38671875 s supported window, replay-exact inspect, legacy stability, four negative cases). [Guide](docs/VEHICLE_HANDLING_CAPACITY.md), [verification](docs/VEHICLE_HANDLING_CAPACITY_VALIDATION.md).

## Unreleased — cargo mass in authoritative world flight

- Add opt-in strictly versioned per-flight cargo mass ledgers (`cosmogon-vehicle-cargo-mass-epochs/1`) so held cargo is part of the craft's real mass in every integration step of both the spherical and rigid-contact solvers, with `Flight::mass_kg()` as the single authoritative accessor and epoch-correct thrust/mass-flow/force/inertia substeps.
- Add one shared removal path: interactive undoable `Edit::RemoveVehicleCargo` and authored timed `Definition.cargo_removals` applied at exact represented craft-clock boundaries, with exact linear momentum closure, bounded relative ejection (`MAX_CARGO_EJECTION_SPEED_M_S = 1000`, co-moving removal impulse-free) and the uniform-gyration layout rule (craft angular velocity exactly unchanged; offset layouts not representable). Every removal records a `CargoEvent`; saved worlds re-derive momentum/mass/angular closure, ledger piecewise counters and authored coverage, and tampering refuses reopen.
- Add conditional `manifest["cargo"]` study evidence with exact `flight-inspect` replay, CLI `flight-definition --cargo-ledgers`, a native held-cargo display and removal control, and a conditional `cosmogon-vehicle-cargo-flight` save pin. Legacy flights serialize byte-exactly and the golden remains `0xf2c4140a45ae509b`. Removal does not deliver or transfer cargo; storage/recipient capacity remains future work.
- Verify 548 release tests plus one existing ignored, no new clippy warnings in touched files, a fresh CLI release build and an eight-case end-to-end loop (definition → study → inspect manifest equality, authored mid-study removal 5000 → 4200 kg held with an event at exactly 0.5 s, contact+operation composition, three negative cases). [Guide](docs/VEHICLE_CARGO_FLIGHT.md), [verification](docs/VEHICLE_CARGO_FLIGHT_VALIDATION.md).

## Unreleased — solver-integrated surface-operation window

- Add `cosmogon-vehicle-surface-operation/1`: a continuous supported-operation window evaluated at every accepted contact-solver midpoint and endpoint against the authored footprint-support envelope, retained as a versioned flight ledger with counters, bounded exclusion reasons and whole-macro-step rollback. Conditional save pin for enabled operation hardware; absent legacy flights keep no ledger and the golden remains `0xf2c4140a45ae509b`.
- Add protected `operation.csv`/manifest window evidence to world-flight recordings and exact independent replay in `flight-inspect`; CLI `contact-hardware --operation-window` authors the window. Detailed foot-shear refusal diagnostics now state the failing quantities inside the unchanged refusal prefix.
- Verify 539 release tests plus one existing ignored and no new clippy warnings. End-to-end: millimeter-gap geographic start on real 4ppd MOLA height with a coast plan holds `flying` for 20 s and grows the window to 9.38671875 s ≥ 5 s required (5800 supported samples), with replay-exact inspect and negative flag cases. [Guide](docs/VEHICLE_OPERATION.md), [verification](docs/VEHICLE_OPERATION_VALIDATION.md). The window is a modeled envelope, not attachment, storage, handling or delivery; graphics remain queued.

## Unreleased — actual vehicle footprint and ground-hold evidence

- Add actual per-foot loads/utilization, shared-plane guards, deterministic loaded-site hulls, geometric/force-weighted/applied-wrench margins and conditional ground-following force/torque comparisons. Keep compliant force application separate from projected ground sites; retain source derivative/coordinate indicators and motion exclusions.
- Add a native equal-scale footprint diagram, typed authored limits, frozen original-world reports and protected exact numeric/nested-flight exports, with strict reopening and headless commands. Original flight formats, simulation laws/saves and legacy golden remain unchanged. Instantaneous inclusion does not attach a craft, prove future stability or permit cargo access. Graphics remain queued.
- Independent Mars rail comparison exposes existing source-operator approximation error separately from numerical derivative refinement. The original orbital operator and golden remain unchanged.
- Verify 514 release tests plus one existing ignored, six release/package checks, 45 CLI/native cases, eight inspected captures and a 60.053035-second ordinary run. Correct new style warnings and bring the full diagram/legend ahead of numeric detail; all 173 frozen Rust files match the checked app.

## Unreleased — integrated persistent foot shear

- Add explicit world-flight version3 / terrain-contact version2 with actual per-foot history, objective rotating-ground transport and finite tangential spring/Coulomb response in coupled motion. Keep endpoint constitutive and actual midpoint mechanical balances separate, including numerical/storage/slip/unloading terms and actual residuals.
- Connect native typed controls/graphs, explicit CLI hardware, conditional strict save pins and protected independently replayed shear CSV/nested approach evidence. Old spherical/smooth-contact recordings and absent legacy state retain their semantics; enabled state deliberately changes under separate pins. The golden remains `0xf2c4140a45ae509b`. Stable footprint/attachment, safe landing and cargo transfer remain incomplete.
- Repair checked-build overflow in the existing south-hemisphere rendering key with explicit modulo-u64 arithmetic, preserving released generated values. No new graphics, shaders, assets or dependencies; graphics remain queued. All 505 release tests plus one existing ignored pass; release/package checks, 67 CLI/native cases, six inspected screenshots and a 48.448818-second live run pass. [Verification and numerical limits](docs/VEHICLE_STATIC_SHEAR_VALIDATION.md).

## Unreleased — persistent foot shear foundation

- Add a separately versioned finite tangential spring/slider, objective terrain-frame history transport, retained plastic slip/unloading release/numerical dissipation and structural refusals. Eight component tests cover static force, independent supported-mass equilibrium, changing directions, separation, objective motion and refinement; all 500 release workspace tests plus one existing ignored pass.
- This mechanical component is not yet enabled in world flight. Existing flight/contact/save semantics, absent legacy state and golden remain unchanged. Actual multi-foot coupling, stability, attachment and delivery remain in progress; packaged terrain evidence remains the preceding 492-test checkpoint. Graphics remain queued.

## Unreleased — finite terrain and rigid-body vehicle response

- Add explicit version-2 contact hardware, actual rotating modeled-terrain feet, finite unilateral force/stroke/load/sliding friction and coupled centre/Euler-quaternion motion. Retain authored inertia/exhaust assumptions and actual contact/translation/rotation work and angular-momentum residuals; structural/source/precision/work failures stop at the prior supported state.
- Native typed hardware/geographic controls, actual live/recorded contact readings/curves, separately pinned saves and protected full-world/contact CSV replay. Explicit signed contact starts support below-spherical-base MOLA terrain; original positive geographic starts, spherical impacts and version-1 flight/approach exports retain their semantics.
- Enabled contact state/fingerprints deliberately change under separate capability/model pins. Absent legacy state and golden `0xf2c4140a45ae509b` are unchanged. Static friction, footprint stability/attachment, safe landing, physical cargo transfer, entry/hardware calibration and the full directive remain incomplete. Graphics remain queued; no shader, asset or dependency changes.

## Unreleased — rotating vehicle arrival

- Add rotating spacecraft approach frames, derivative-aware geographic initial conditions and original-branch approach recordings with native/CLI exact evidence, finite mass/energy, historical contact frames and explicit pole/terrain support. World/save/physics/old flight formats and golden are unchanged; terrain touchdown/gear response, delivery and graphics remain incomplete.

## Unreleased — physical finite vehicle flight

- Connect authored finite propellant/cargo to optional actual world spacecraft, fixed deterministic gravity/variable-mass motion, explicit spherical first contacts and retained unsupported-state reasons. Ordinary create/remove edits, undo/redo, saves and model/source pins share the authoritative world.
- Native live controls and independent pinned-world studies, protected complete origins/final worlds/36-column CSV/JSON, strict full-world reopening/replay, stable bytes and request-time worker controls. Preserve exact previous propulsion recordings; first contact is not a safe landing or delivery. Gravity/frame discontinuities currently stop unresolved flight explicitly.
- Retain habitat scheduler slots while later vehicle tasks exist; park terminal flight on the completed task time so fractional caller chunks preserve identical state. Bound study work and estimated payload before allocation. Enabled fleet state/fingerprints deliberately change; absent legacy worlds omit the new state and retain golden `0xf2c4140a45ae509b`. No graphics or dependency change; physical landing/loading/delivery, hardware fidelity and phase 10 remain incomplete.

## Unreleased — finite supply-vehicle design and propulsion

- Shared authored dry/tank/propellant/payload and finite cabin-consumable cargo design, deterministic constant-effective-Isp burns, exact fuel cutoff and actual mass/impulse/vector-capability observations. Add stable deterministic `dln1p` and expose raw numerical residuals.
- Native designer and CLI protected complete plans/24-column CSV/JSON/source exports, strict independent reopening and request-time controls without live world or supply grants. Save laws/catalogs/source constants, graphics and golden `0xf2c4140a45ae509b` remain unchanged; physical flight/delivery and phase 10 remain incomplete.

## Unreleased — frozen habitat study origins

- Explicit original-saved/current-live starting-world selection, cached frozen preflight, exact finite stores/clocks/source pins and immutable request-time context for new single/paired experiments.
- Shared `habitat-rerun` and packaged rerun/export controls, fresh destinations, changed-source/ordinary-edit refusals and successful-result-only view changes. Preserve solver, save/catalog formats, source constants, graphics and golden `0xf2c4140a45ae509b`.

## Unreleased — complete saved single-setting habitat designs

- Reopen all independent common-state branches with strict plan/source/edit/time/catalog checks, actual branch choices, restored fresh-output controls and protected all-branch trajectories/selected figures.
- Independently replay every retained branch and observation through shared native/CLI tools; preserve old paired reports and ordinary figure serialization, solver/saves and golden. Long paired setting descriptions wrap within the pane.

## Unreleased — controlled installed crop studies

- Four typed ordinary-edit controls for ideal chamber assumptions, auxiliary power and finite dry/shared oxygen capacity; exact retained original state, strict boolean/no-discard preflight and plan-dependent actual crop responses.
- Shared native/CLI single and paired studies, explicit crop/thermal source context and restored staged controls. Previous metric catalogs, simulation/save state and golden stay unchanged.

## Unreleased — recorded actual crop trajectories

- Add explicit retained crop progress/stocks/material/electric/chemical curves, signed heat debit selections, aligned-origin/completed-time comparisons and protected source-context research figures through existing native/CLI tools. Keep actual zeros distinct from absent state, disclose ideal chamber/source progress limits and preserve immutable saved catalogs, old analyses, solver/save state and golden `0xf2c4140a45ae509b`. Write optional crop CSV before habitat completion manifest. No graphics/dependency or calibrated biology claim.

## Unreleased — finite installed crop process

- Connect explicit originally installed finite crop state to real model water/CO₂/nutrient feeds, bounded oxygen/dry stores, allocated source lighting and chemical/vapor heat storage. Add native setup/controls/harvest, exact save pins, protected actual crop CSV/reopening and ordinary undoable harvest without crew diet credit or hidden replanting. Ideal maintained conditions and chemical energy remain user assumptions; source progress is not biological age. Absent legacy state and golden `0xf2c4140a45ae509b` stay unchanged; enabled habitat state/fingerprints deliberately change. No shader/graphics/dependency changes or calibrated biology claim.

## Unreleased — cumulative cabin heat accounting

- Retain passive signed heat-path/storage counters for every actual executed habitat interval, excluded coverage and capacity transitions. New definitions explicitly enable model-pinned accounting; absent legacy definitions preserve identical physics/state/replay. Native/CLI recorded Heat ledger and optional protected heat CSV expose exact span differences/residuals. Existing temperature solver and untouched-universe golden `0xf2c4140a45ae509b` stay unchanged; enabled habitat state fingerprints deliberately change. No backfilled history or hardware calibration.

## Unreleased — recorded cabin balances

- Add shared native/CLI exact endpoint stocks, cumulative model counters, mass/electrical identities and temperature references, all four independent paired branches and protected source-context JSON/CSV. Preserve original inputs, simulation laws/saves and golden `0xf2c4140a45ae509b`; never extrapolate sparse minute budgets into heat history.

## Unreleased — recorded research figures

- Add shared native/CLI protected vector figures with original full-precision numbers, complete typed observations/source identity, paired method context and reusable strict graph layouts. Actual samples, unavailable gaps and positive-log omissions remain explicit; resolve small-signal bounds/ticks and capture native export inputs at request time. No smoothing, invented uncertainty, simulation law/save/dependency change; golden `0xf2c4140a45ae509b` remains unchanged.

## Unreleased — controlled cabin geometry and thermal studies

- Add four typed volume/capacity/conductance/radiator studies, illustrative editable levels, fixed-inventory method/source notes, plan-dependent total-pressure catalogs, protected paired trajectories and exact independent replay. Reopened designs restore their actual typed controls with a fresh output field; staged inputs remain separate from completed evidence. Preserve legacy analyses, simulation laws/saves and golden `0xf2c4140a45ae509b`. No physical-expansion, equal-initial-energy or equipment-calibration claim.

## Unreleased — recorded pressure explanations

- Add actual cabin gas components, explicitly ordered inventory/temperature/volume accounting between retained observations, and separately scoped finite breach substep evidence. Native tables, CLI and protected exact-input JSON/CSV exports share one derivation; preserve original studies, simulation laws/saves and golden `0xf2c4140a45ae509b`. Mathematical accounting is distinct from unique physical causation or field calibration.

# Changelog

## Unreleased — finite cabin breach

- Connect optional source-pinned outward open-orifice flow to finite O₂/N₂/CO₂/vapor, retained outside mass history, supported bounded integration and separate gas/structure/phase/enthalpy identities. Native controls, WHY, protected 110-column recordings and four controlled settings share ordinary edits/saves/paired replay. Main heat report explicitly distinguishes temperature-equivalent cooling from outflow enthalpy. Absent legacy states and golden `0xf2c4140a45ae509b` remain unchanged. Constant-property/minute split approximation; no seal/rarefied/inward flow, calibrated geometry or EVA claim.

## Unreleased — recorded graph navigation

- Add actual-time viewport zoom/pan/selection, truthful positive/signed log display and protected reusable layouts to ordinary/comparison/paired habitat graphs. Preserve full raw observations, selected/final analyses, simulation laws, saves and golden `0xf2c4140a45ae509b`. Correct replay diagnostic comparison warnings and retain signed-zero differences.

## Unreleased — saved habitat analysis

- Add selectable stored-time paired response tables/effect curves and protected complete numeric exports, plus explicit independent whole-design replay verification. Reconstruct all four branches, compare every complete typed observation and endpoint/source identity, retain bounded differences, native progress/cancellation and protected reports. Preserve original recordings/final analyses, simulation laws, saves and golden `0xf2c4140a45ae509b`; no empirical calibration claim.

## Unreleased — finite cabin moisture
- Add paired two-setting/two-level habitat designs, four atomic common-state branches, actual conditional/interaction contrasts and protected source-pinned results with shared native/CLI reopening. Preserve single-setting plans, simulation laws, saves and the golden.

- Add six controlled moisture settings to shared native/CLI habitat experiments with exact common inventories, all-branch preflight and actual phase/water comparisons. Clarify completed moisture intervals/disable semantics and expose phase heat terms before WHY aggregates. No simulation law, save schema or golden change.


Optional source-pinned finite vapor/isolated condensate, assumed crew moisture partition, NIST-supported RH, conservative dehumidifier bus allocation, explicit crew phase debit/condensation heat and later powered liquid recycling. Native Overview/Settings/Model views, ordinary undoable edits, exact retained model pins, shared WHY and protected 85-column studies/recording graphs. Legacy configurations and golden `0xf2c4140a45ae509b` remain unchanged. No wall/ice condensation, hardware calibration, purification quality or physical greenhouse exchange claim.

## Unreleased — controlled crop transpiration

Source-pinned optional light-period water/vapor forecasts, explicit maintained humidity at fixed terrestrial reference pressure, gross/light-cycle net carbon and conductance, deterministic quadrature with resolution evidence, native water graphs/comparisons and protected numeric exports. Version-1 definitions preserve identical biomass without fabricated humidity/water; negative source conductance and unknown water versions are refused. No installed greenhouse, finite habitat exchange, field calibration, simulation/save/golden or dependency change.

## Unreleased — authored camera paths
* Native keyboard customization for 24 camera/everyday actions, shared input and cinematic interruption, capture/cancel/release gating, conflict validation, dynamic help, explicit protected versioned files and separate preferences. No Universe/save/golden change.


App-only timed keyframes, normalized orientation/look-at and lens interpolation, native authoring from actual/saved views, explicit Play/Pause/Stop/scrub, protected schema-1 files and capture/performance evidence of evaluated versus actual ground-corrected positions. Simulation laws, save schema and golden `0xf2c4140a45ae509b` unchanged. Video, DOF, spline paths and simulation-rate synchronization remain incomplete.

## Unreleased — native exploration, research and habitat engineering

- Add shared completed-interval habitat WHY diagnostics, native live/recorded power/water/heat/air explanations and protected source/CSV exports. Retain interval configuration, recorded constraints, explicit unavailable dependencies and unit/formula evidence; reuse allocations in equipment audio. Legacy studies remain readable and present diagnostic reports are checked. No physical state, source version, dependency or golden change.

- Add an optional live habitat interior listening preview with five equipment textures driven by actual completed power budgets, separate environment mixing, bounded streaming synthesis and protected eight-second WAV/source exports. Context resets on load/undo/removal; capture/photo/quiet modes suppress playback. Designed sonification, with no calibrated acoustic or physical interior claim. Simulation state and golden remain unchanged.

- Add controlled habitat single-setting experiments through ordinary deterministic configuration edits, preserving common initial inventory and protected per-run evidence. Native worker sweeps and two-recording curves share exact stored-time/interval-supported differences with CLI JSON/CSV. No interpolation, field-validation, global-optimization, save or golden claim.

- Reopen bounded habitat studies in native Recordings with actual metric curves, explicit unavailable gaps and selected rows. Shared CLI inspection checks source/endpoint/fingerprint/budget/report/CSV consistency; asynchronous transactional imports and photo metadata preserve recorded/live scope. No world or golden change.

- Connect optional finite Sabatier CO₂ reduction, isolated methane/raw water, later powered recycling and reference reaction heat to habitat controls, saves and actual source-pinned exports. Ratings are assumptions; no closed-loop or hardware-validation claim. Existing optional electrolysis and golden world are unchanged.

- Add optional finite habitat water electrolysis with separated O₂/H₂ products, crew/power priority, reaction-enthalpy and waste-heat accounting, source/version pins, native controls and actual research exports. Legacy defaults and golden `0xf2c4140a45ae509b` remain unchanged. Equipment is assumed and uncalibrated; physical greenhouse coupling remains incomplete.

- Add source-pinned controlled lettuce crop planning from NASA MEC equations and corrected low-light regimes. Shared native/headless calculations retain source snapshot, analytic cumulative growth/gas exchange, explicit assumed lighting demand, integer-age graphs, protected numeric/comparison exports and exact optional habitat context. Unlimited controlled inputs are exposed; no installation, inventory credit or closed-loop greenhouse claim. No Universe/golden change.

- Add authoritative user-created geographic habitat experiments with finite mass stores, NASA BVAD Rev2 nominal crew exchanges, powered air/water processing, primary-star solar/external supply, battery losses and lumped heat budgets. Ordinary undoable edits, exact save/resume, design-alert entry/recovery, native controls/markers, model/source context and protected snapshot/trajectory exports share the engine. Optional state and monotonic identifiers are omitted from untouched worlds; the golden remains `0xf2c4140a45ae509b`. Phase 9 remains partial, with unquantified predictive uncertainty and no safety/medical certification.
- Restore derived N-body acceleration caches on save loading before between-step position/illumination readers run. Dynamic save/continue and habitat replay fixtures cover the correction without changing serialized initial trajectories.

- Reopen native numeric terrain studies with exact pinned source/sphere/samples and transactional failure recovery. Show source-aware sampled paths and selected rows on Mars; suppress stale, hidden or mismatched traces. Preserve independent study graphs/exports and capture actual study definitions/rows in photo metadata. Add explicit current-world adaptation and bounded pre-render automation checks. No Universe or golden change.

- Add independently pinned NASA MOLA 16ppd relief and coverage, with exact source/label/provider checksums and reproducible conversion. Explicit undoable source selection shares heights across terrain, probes, landing markers, camera travel and numeric studies. Save/version and bookmark/section mismatch checks preserve reproducibility; terrain/bake caches follow source changes. Existing worlds and new defaults stay unchanged, so the golden remains `0xf2c4140a45ae509b`. Finer pixels retain interpolated support and unavailable uncertainty; no local safety/accuracy claim.

- Add shared deterministic MOLA terrain sections between historical/custom endpoints, actual native height-distance graphs and row-to-probe/travel actions. Protect source-supported CSV/definition/JSON exports and headless replay; retain source-cell weights/counts, checksums, unavailable uncertainty and overview/geodesic limits. No Universe or golden change.

- Add four cited historical Mars landing references, native surface selection/markers, shared f64 geographic camera transforms, radial-altitude travel and explicit snapshot/tracked modes. Tracked viewpoint anchors follow spin/translation and restore lens/site identity without rewinding; version-1 libraries still load and unknown versions remain protected. Native/headless probe JSON and photo sidecars retain provenance/registration limits. No Universe schema or golden change.

- New Solar System worlds attach NASA PDS MOLA 4-pixels/degree observed-derived Mars relief with observation-count coverage, source/checksum metadata and native/headless coordinate probes. Mars no longer uses generated elevation; colours/climate/fine shading remain generated/modelled. Existing saves keep their stored terrain identifier. This deliberate new-state change re-pins the deterministic golden fingerprint from `0x579903fd05a28945` to `0xf2c4140a45ae509b`.

* Adopt the owner's master development directive; add current-state audit, evidence/limitations,
  product vision, source/asset inventories and active phased gap analysis while preserving historical plans.
* Add an optional Performance panel (View menu and command palette), frame-time chart and
  bounded average/p50/p95/p99/max recordings, simulation timing and async universe-load timing.
* Export local versioned JSON with scene/camera/seed/dataset context and raw samples;
  `--profile`, `--profile-frames`, `--profile-warmup` enable reproducible native runs.
* Opt-in process/render diagnostics persist as an app preference. Unsupported GPU timings and
  unmeasured VRAM are explicitly unavailable. Export errors are visible; earlier files are protected.
* Add six-degree free flight with f64 position/orientation, adaptive/manual travel speed, roll,
  boost/fine movement, optional inertia, sensitivity and vertical inversion. Preserve orbit/focus
  transitions; nearby solid bodies drive terrain loading in free flight.
* Add Camera & Photo controls for presentation-only field of view, exposure, warmth and saturation.
  Save camera/lens preferences independently of Universe. Pause and hide-interface controls are explicit.
* Add a persistent, versioned viewpoint library with object-relative positions, stable anchor IDs,
  lens restoration and protection against overwriting invalid/newer libraries.
* PNG capture reports actual completion/failure, preserves the original overlay state and optionally
  writes camera/dataset metadata in a separate `.png.json` sidecar. Developer capture failures exit nonzero.
* Reject misleading zero fallback process counters and unsupported Metal GPU timestamp diagnostics.
* Add Realistic/Cinematic/Hybrid audio modes and independent music, cinematic-effects and alert
  levels. Live mute/mode changes apply to active sounds; leaving a universe stops its audio.
* Add a searchable recent notification inbox, category/severity/unread filters and object navigation.
  Per-category popup/inbox/sound/pause routes and priority thresholds persist in app settings.
  Do Not Disturb/photo suppress interruptions; bell bursts are coalesced, pause defaults off.
* Preference writes now replace a temporary file and new controls report failures. Older sound
  preferences retain their released mix. Developer `--sound-mode` and `--quiet` support native QA.
* Match free-flight ground clearance and near-plane/bloom behavior to orbit mode after native captures
  exposed close-range terrain precision artifacts.
* Repair macOS packaging in synced/file-provider folders by assembling/signing on the temporary
  filesystem; keep the launchable app in the local build cache, link it from `dist`, and clean staging
  on exit. The disk image remains standalone.
* Propagate Bevy AppExit from the native entry point so failed developer exports, captures and
  loads return a nonzero process status. Native verification exposed the previously discarded status.
* Add searchable native research controls over 36 shared typed parameters, dependency/formula info,
  locked derived values and undoable input changes. Read-only reference views retain protection.
* Add seven model descriptions with methods, assumptions, fidelity/cost, error limits and primary
  sources; distinguish estimated catalogue inputs from procedural stars and modelled outputs.
* Add bounded native observer plots and protected six-file CSV/initial-state/edit/event/model bundles.
  Paused edits have distinct edit_count rows; rewind stops a recording. Missing uncertainty is null.
* Add CLI model registry export and uniform-simulation-time experiments, plus coordinated native
  developer research/capture/profile exports. Add frozen-state definition replay and bounded independent parameter sweeps; no blanket scientific validation claimed.
* Add parent-relative orbital measurements and bounded dynamic-particle Newtonian diagnostics.
  Charts support nearest-state inspection, zoom/pan, log scale, fractional changes and event markers.
* Add cancellable native background studies with guided questions, saved prepared definitions,
  ordered sweep values and canonical-unit outcome comparison. CLI sweep/replay shares the runner.
  Frozen-state fingerprints reject a changed base; incomplete batches retain completed runs.
* Calculate native/headless analytic circular-orbit and cached JPL endpoint validation reports with actual residuals, initial/reference vectors, units, solver conditions and explicit coverage limits.
* Allow independent changed/measured objects and a persisted primary measurement. Surface completed branches from cancelled/failed studies without reporting success; open computed results automatically.
* Fit research scroll areas within the window, separate per-tab scroll state, add CLI help and graceful argument errors; research creation defaults to Solar Lab.
* Reopen observer bundles through a bounded shared numeric reader and CLI inspect; verify keys, units, finite rows and state ordering. Load up to eight completed study trajectories off the main thread.
* Add shared elapsed-time graph zoom/cursor, nearest-state keyboard inspection, multiple computed branches and explicit scale guards. Add saved graph/time choices, parameter favorites and Explore/Learn/Research guidance, including current formula inputs.
* Preserve invalid/newer workspace files and expose save/load errors; coordinate capture/quit with imported data and completed studies.
* Simulation state and save format unchanged; **golden fingerprint unchanged**.

## 1.0.0 — The whole universe (2026-10-04)
Plan and rationale: docs/PRODUCT_PLAN_1_0.md.
### Added
* **Life and death of stars**: main sequence → red giant (swelling, brightening, swallowing inner
  planets, shedding mass so surviving orbits widen) → planetary nebula and white dwarf, or supernova
  and neutron star / black hole by initial mass; supernova radiation fronts cross interstellar space
  at light speed and sterilise worlds in their path.
* **Compact objects you can create and throw**: black holes with ray-traced gravitational lensing,
  photon ring and Doppler-beamed accretion disks; neutron stars with pulsar beams; white dwarfs;
  extra stars.
* **Star formation**: stellar nurseries turn gas into new star systems until the cloud is spent; the
  real **Orion Nebula** with the Trapezium stars at 1,344 ly; ray-marched emission nebulae.
* **Galaxy scale**: the Milky Way seen from outside at its real orientation (bar, arms, dust, star-
  forming knots), the Local Group (Andromeda, Triangulum, Magellanic Clouds) at catalogued positions,
  and a deep field of galaxies and quasars along a cosmic web; the camera pivots to the Galactic Centre
  as you pull back.
* **Spectacle physics**: catastrophic disruption after Leinhardt & Stewart (2012) with debris, molten
  remnants and boiling oceans; moons inside the Roche limit become rings; impact flashes and shock
  rings; comets with sublimation-driven comae, ion tails and curved dust tails.
* **Throwing, Universe Sandbox style**: Throw (drag to fling, click for a circular orbit, live
  trajectory) and Grab tools, an object shelf, Aim and Create in a tool dock.
* **Civilizations across the stars**: Dyson swarms (Kardashev II), terraforming of colony worlds,
  generation ships that found new branch civilizations at stars scouted by probes.
* **Interface**: Inter + Phosphor icons, icon top bar with search, redesigned universe tree, time bar
  with speed slider, **guided tours** (four), **timeline with rewind**, **photo mode** (P),
  synthesised **sound**, decluttered labels.
* **Experiments**: Halley's Comet, Theia, a second sun, a feeding black hole, the Sun as a red giant
  or a dying supergiant, a Kardashev II humanity, and more.
### Changed
* Golden fingerprint re-pinned (new civilization fields and technologies); old saves load.
* Technologies that build on not-yet-invented ones can no longer leak into present-day Earth.
### Fixed
* Far-distance f32 overflow in direction normalisation (camera and galaxy shader).
* Inter's private-use glyphs shadowing Phosphor icons.

## 0.5.0 — Worlds and civilizations you can see (2026-10-04)
Details: docs/MILESTONE_G1_C1.md.
### Added
* **Appearance model**: every planet, moon and exoplanet gets a physically chosen style (cratered,
  volcanic, ice shell, grooved ice, tiger stripes, haze, cloud deck, lava/magma ocean, Sudarsky-class
  giants with belts, jets and storms); Jupiter's GRS and ovals, Saturn's hexagon, Neptune's dark vortex.
* **Rendering**: atmospheric single scattering, eclipse shadows from moons and planets, ring shadows,
  Saturn's measured ring structure, stellar limb darkening/granulation/spots, lava and thermal glow,
  procedural detail and bump mapping beyond texture resolution, mip-maps.
* **Present-day humanity** in the Solar System Lab: real cities and countries, exploration record,
  missions in flight (BepiColombo, Europa Clipper, JUICE), 11 000 satellites.
* **Space programmes**: missions to every world (flyby → orbiter → lander → crewed), colony ships,
  energy transition away from fossil fuels; Kardashev rating.
* **Visible civilizations**: at-a-glance card, space programme panel, planet badges, satellite shells
  incl. geostationary belt, stations, spacecraft in flight, colony sites and lights, cities and farmland
  on the ground.
* **Interventions**: Inspire, Signal, Hardship, Share knowledge, Teach a technology — with a 25-year
  cooldown and no skipping of prerequisites; undoable sandbox edits.
### Changed
* Only the Garden World template guarantees a habitable planet; hot Jupiters can now form.
* Golden fingerprint re-pinned (energy transition changes long runs).
### Fixed
* Stray terrain quads and one-frame flashes of newly spawned bodies.

## 0.4.0 — Sandbox Foundation (2026-10-03)
Cosmogon becomes a universe sandbox. Details: docs/MILESTONE_S1.md.
### Added
* **Home & sandboxes**: new application shell (intro, New Sandbox with seven templates, Continue, Load,
  Real Universe, Scenarios, Settings, Credits); sandbox documents with manifest, autosave (also on quit),
  Save As, Duplicate, Rename, Delete, checkpoints, thumbnails; legacy-save import; local profile.
* **Real data**: NASA/JPL Horizons pipeline (`tools/fetch_horizons.py`) and cached 2026-01-01 state vectors;
  Solar System Lab starts from them; Real Universe view (read-only) with Clone to sandbox; per-quantity
  provenance (measured / derived / estimated / procedural / user-modified) in the inspector.
* **N-body gravity**: 4th-order symplectic integrator with encounter substepping, collisions and mergers,
  optional 1PN relativity, rails moons, presets (Fast/Balanced/Accurate/Research/Custom), Kepler ↔ N-body;
  one-year integration matches JPL to ≤ 6 km for Venus–Neptune.
* **Editing**: object creator (planets, moons, giants, dwarf planets, asteroids, comets), editable inspector
  with units and scientific notation, star mass/age/metallicity, orbital elements, Δv pushes, delete,
  launch tool with live predicted trajectory and impact warning, undo/redo, warnings for unusual input.
* **Consequences**: orbit → insolation → climate → habitability → civilizations; impacts with crater,
  blast radius, impact winter, extinction and casualties; destroyed worlds end their civilizations.
* Overlays (osculating orbits, trails, predictions, velocity), calendar clock, command palette,
  seven curated "what if" scenarios built from ordinary edits; save format v2.
* Docs: GAP_ANALYSIS, SANDBOX_VISION, PHYSICS_ENGINE, ASTRONOMICAL_DATA, DATA_SOURCES, OBJECT_MODEL,
  SAVE_FORMAT, UI_SYSTEM, MILESTONE_S1.
### Changed
* One set of physical constants (IAU 2012/2015, CODATA 2018); insolation uses the annual-mean distance
  a(1−e²)^¼. Seeds replay slightly differently from 0.3 (golden fingerprint re-pinned); old saves load.
### Fixed
* `state_vectors_to_elements`: swapped arguments for the ascending node; equatorial orbits lost periapsis.

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
- Paired habitat trajectories: native selectable stored-time response/contrast tables, signed discrete effect curves with unavailable gaps, protected all-time JSON/40-column CSV export and explicit CLI observation inspection. Loaded design identity stays separate from editable folder text. Original final analyses, simulation laws, saves and golden fingerprint remain unchanged.

### Recorded cabin balances (development)

* Shared retained endpoint stock/electricity/temperature/counter reports, all four independent paired ledgers, actual source/method context and protected CSV/JSON exports. Native cached views and immutable workers preserve recordings/live state; sparse sampled heat/power budgets are not extrapolated. No simulation law/save/dependency/golden change.

Selected cumulative cabin heat curves add eleven signed energy paths/storage, signed path/residual sums and executed/excluded time; protected native/CLI figures retain origin/coverage and four actual branch responses. Incomplete/different-origin comparisons stay unsupported. Existing recorded catalogs/analyses, physical laws, state/saves and golden unchanged.

Added separate source-pinned finite crop-supply feasibility studies: explicit liquid/CO₂/HNO₃-equivalent/lighting budgets, daily support brackets over unchanged maintained forecasts, retained source rounding residual, shared native/CLI numerical exports and verified frozen-context reopening. No physical crop or habitat stock credits, simulation/save change or golden re-pin.

Added separate conservative CHON crop material studies with pinned CIAAW nominal weights, original recipe/forecast retention, explicit inferred dry composition and oxygen differences, native element tables and strict protected JSON/CSV/source reopening. No simulation/save/golden/dependency/graphics change.
