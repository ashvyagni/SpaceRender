# Research workspace verification — 2026-10-07

This checkpoint adds shared parameter/model/source context, native research controls and plots,
observer bundles, frozen definitions/replay/sweeps and guided comparison workflows. It does not
modify Universe serialized state, physical algorithms or the golden fingerprint.

## Automated verification

* cargo clippy --release --workspace: existing warnings only; no new research warnings.
* cargo test --release --workspace: 208 passed, 1 existing ignored, 0 failed.
  App 43, core 32, physics 38, simulation 94, doctest 1.
* Golden guard remains 0x579903fd05a28945.
* Observer tests cover read-only state, sample limits, rewind rejection, paused edit ordering,
  exact initial-save roundtrip, output protection and edit/event context.
* Parameter tests cover safe adapters, unavailable/derived values, exact-field star provenance,
  parent-relative Earth/Moon orbital quantities, hyperbolic period unavailability and no read mutation.
* Definition tests cover independent branches, real gravity consequences, replayed identical CSV,
  invalid inputs/versions/base identity, saved prepared studies and cancellation with retained runs.
* Native wrapper tests cover reference edit protection, undo, selection snapshots and frozen-state
  independence from subsequent live edits/time.

## Actual native and headless workflows

Fresh builds were launched through the native GPU executable and inspected via PNG captures:
Earth controls, current N-body model, Trapezium source states, populated live charts and the native
frozen-study workspace. Earlier captures exposed window translucency; the final window is opaque.
Plots expose a viewport-height/layering rough edge at this display size; fixing that is next work.
No direct mouse/keyboard chart, screen-reader or audio interaction was verified by these captures.

CLI observer: two seed-identical 32-state Earth runs at exact 86400-second spacing produced identical
CSV, edit/event and model files. All numeric rows were finite; existing output and NaN cadence were
refused. Model export contains 36 typed descriptors and seven models after the expansion.

CLI sweep: three Earth radius branches replayed with identical CSV from the exported frozen base and
definition. Doubling radius gave exactly one quarter of surface gravity within floating-point
roundoff. All measurements were finite. A wrong frozen-state fingerprint was rejected before output.

Native coordinated export: 60 finite Earth states (30 supported quantities), screenshot and
performance report all completed before quitting. The native radius batch completed three branches
and matched the headless CSV exactly. Invalid plan identity, existing output and missing save returned
process status 1; earlier batch output was unchanged. Recorded native cadence reflects actual frame/
clock timing, not uniform simulation days. Performance output here checks coordination only: compiler
work overlapped capture verification, so this is not a clean performance benchmark.

## Package and artifacts

macOS packaging produces the native arm64 ad-hoc signed app and standalone disk image. The package
script verifies its signature; additional verification and an ordinary packaged-app stability run are
retained with local logs. An ordinary packaged run completed 100 Jupiter states over 33.33 real-time
simulation seconds without a panic; signature and DMG checksum verification passed. No notarization or public release is claimed. Final checkpoint logs/captures/
experiment bundles are retained locally under ignored dist/research-validation; source validation
notes remain in version control. Review export limitations in [RESEARCH](RESEARCH.md).

The measured analytic/JPL benchmark reports landed as the build-59 cross-check extension: the
report now covers six cases (circular and elliptical two-body fixtures at 1000/2000 steps, an
RK4-vs-Yoshida integrator cross-check, and the JPL Horizons case at 181-day mid-year plus 365-day
endpoints) with 21 JPL metrics and historical acceptance guards. Independent physical/model
uncertainty, covariance propagation and wider regime coverage remain open acceptance work; the
build-59 guards bound solver error only. Details and measured values are in
[SCIENTIFIC_VALIDATION](SCIENTIFIC_VALIDATION.md), section “Build 59 cross-check extensions”.

The subsequent measured-validation/cross-object slice is documented in [SCIENTIFIC_VALIDATION](SCIENTIFIC_VALIDATION.md); the 208-test count above remains the historical first research checkpoint. Build 59 verification stands at 684 release tests plus one existing ignored, zero failures and no new clippy warnings, with the golden fingerprint unchanged.
