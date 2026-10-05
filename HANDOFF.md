# Handoff

For anyone — human or AI agent — picking up Cosmogon. Read this first, then
[ARCHITECTURE.md](ARCHITECTURE.md) and [BUILDING.md](BUILDING.md).

## Where things stand (2026-10-05)
* **Released: 1.0.0** ("The whole universe"), tag `v1.0.0`, published on
  [GitHub Releases](https://github.com/ashvyagni/SpaceRender/releases) with macOS, Windows and Linux builds.
  0.5.0 and 0.3.0 are published too. `main` = `universe-1.0` = the 1.0.0 commit.
* What 1.0 contains and what was deliberately left out: [docs/PRODUCT_PLAN_1_0.md](docs/PRODUCT_PLAN_1_0.md)
  (section "Status at 1.0.0"). Milestone table: [ROADMAP.md](ROADMAP.md). History: [CHANGELOG.md](CHANGELOG.md).
* All 173 tests pass across the workspace; CI is green on macOS, Linux and Windows.

## The shape of the project
* `crates/cosmogon_sim` — the simulation. **No engine dependencies, deterministic.** It owns all state
  (`Universe`), advanced by a fixed-period task scheduler. The app may read it and issue `Edit`s; it never
  mutates the model behind the simulation's back.
* `crates/cosmogon` — the Bevy 0.18 app (bevy_egui 0.39, WGSL shaders, Metal/Vulkan/DX12): camera, rendering,
  UI, persistence, audio. Shaders are embedded; `render/shaders/common.wgsl` is imported as `cosmogon::common`.
* `crates/cosmogon_physics`, `cosmogon_core` — orbits, integrators, f64 math, constants.
  `crates/cosmogon_cli` — headless runs and determinism checks.
* Data is embedded TOML in `crates/cosmogon_sim/data/` (technologies, science parameters, the Solar System,
  2025 cities, JPL Horizons state vectors).

Map of the 1.0 features (start here when changing one):

| Feature | Simulation | App |
|---|---|---|
| Stellar evolution, supernovae, blast fronts | `stellar.rs`, `astro/star.rs` | `render/stellar.rs`, `shaders/{star,black_hole,nebula,beam}.wgsl` |
| Star formation (nurseries, Orion Nebula) | `nursery.rs` | `render/nurseries.rs`, `shaders/emission_nebula.wgsl` |
| Collisions, disruption, Roche rings | `collision.rs`, `sandbox.rs` (`on_contact`) | `render/impacts.rs` (flashes) |
| Comets | (bodies with `ObjectClass::Comet`) | `render/comets.rs`, `shaders/comet.wgsl` |
| Galaxy, Local Group, deep field | — | `render/galaxy.rs`, `shaders/{galaxy,deep_field}.wgsl`; galactic pivot in `camera.rs` |
| Dyson swarms, terraforming, generation ships | `expansion.rs` | `ui/markers.rs`, `ui/inspect.rs` (space programme) |
| Civilizations, technology | `civ/`, `data/technologies.toml`, `present_day.rs` | `ui/inspect.rs` |
| Experiments ("what ifs") | `sandbox.rs` (`WHAT_IFS`, `apply_what_if`) | `ui/home.rs` (Scenarios) |
| Throw / Grab tools | — | `interact.rs`, `sim.rs` (`Tool`), `ui/dock.rs` |
| Guided tours | — | `ui/tour.rs` |
| Timeline and rewind | — | `timeline.rs` |
| Photo mode, dev capture | — | `capture.rs` |
| Sound | — | `audio.rs` (synthesised; no audio files) |

## How to verify a change
1. `cargo test --release --workspace` — must stay green. Run it with output to a file if it seems to hang
   (a test that advances the N-body Solar System lab by millions of years *will* take forever; test the
   function directly instead, see `nursery.rs` tests).
2. `cargo clippy --release --workspace` — no new warnings in the code you touched.
3. **Look at it.** The app renders straight to PNG without any screen-recording permission:
   ```bash
   cargo build --release -p cosmogon
   ./target/release/cosmogon --new lab --focus Saturn --distance 5 --speed 0 --hide-ui \
       --capture /tmp/shot.png --capture-after 240 --quit
   ```
   * `--new lab|sol|garden|system|empty` (anything else = neighbourhood), `--what-if ID` (any `WHAT_IFS` id),
     `--tour ID`, `--focus NAME` (body or star name), `--distance` **in radii of the focused object**,
     `--yaw/--pitch` (rad; pass `--yaw` too or the default lighting angle overrides `--pitch`),
     `--speed INDEX` into `cosmogon_sim::time::SPEEDS` (0 = real time … 11 = 1 Myr/s).
   * Galaxy-scale views: `--focus Sun --distance 2e12` ≈ 150 000 ly.
   * Without `--hide-ui` you see the full interface; with it, panels are hidden (labels and orbits stay).
4. For anything that runs for a while, also launch it normally for 30–60 s and watch the log for panics.

## Rules and conventions
* **Determinism.** The simulation must give bit-identical results on every platform. Use the `DMath`
  functions (`dsin`, `dexp`, `dpowf`, …) from `cosmogon_core::dmath` in simulation code, keyed RNG streams
  (`Rng::stream(seed, domain, keys)`), and never iterate a `HashMap` where order affects results.
* **Golden fingerprint** (`save.rs`, `golden_universe_fingerprint`): any change to the serialised state —
  even a new field — changes it. If the change is deliberate, re-pin the constant and say so in the
  changelog. Old saves must still load: new fields get `#[serde(default)]`.
* **Present-day Earth** gets every technology except those in `present_day::AHEAD` *and anything that
  depends on them*. New future technologies therefore stay in the future automatically.
* **Real vs procedural.** Real-data scenarios contain only catalogued objects (the Solar System, the
  Trapezium stars); every quantity carries a provenance label. Don't invent bodies around real stars.
* **Style.** Match the surrounding code: comments explain *why* (often with the physics and a reference),
  names are plain words, units in names or comments (SI internally). No new dependencies without need.
* **Git.** Commits by `ashvyagni`, **no `Co-Authored-By` trailers**. Push only to
  `git@github.com:ashvyagni/SpaceRender.git` as ashvyagni (check with `ssh -T git@github.com`); if you can't,
  hand back to the owner to push. Branch per milestone (1.0 was `universe-1.0`), fast-forward into `main`.

## Traps we already fell into
* **f32 overflow at galaxy scale.** Render-space vectors reach ~1e21 m and `length²` overflows f32
  (max 3.4e38) → `normalize` returns NaN/zero silently. Divide by a scale (ly, the object's radius) *before*
  normalising in shaders, and normalise in f64 on the CPU (`camera.rs`).
* **Metal: `smoothstep(a, b, x)` with `a ≥ b` is undefined.** Write `1.0 - smoothstep(b, a, x)`.
* **Expensive shaders can time out the GPU** (the first nebula shader did). Keep ray-march steps ~40 and
  noise octaves ≤ 4.
* **Fonts:** Inter's private-use code points shadowed Phosphor icons; they were stripped from the bundled
  Inter (see `crates/cosmogon/assets/fonts/README.md`). Don't swap in a stock Inter file.
* **Scheduler placeholders** use `scheduler::NEVER` (1e290), not `f64::INFINITY` — infinity serialises as
  JSON `null` and breaks loading.
* **Bevy query conflicts:** two queries touching the same component mutably in one system need
  `Without<…>` filters or separate systems.
* **UI layering:** bottom panels stack upwards in system order; floating areas (tool dock, tour card) are
  positioned above the timeline strip — adjust both if the bottom bar grows.
* `--capture` disables keyboard input and sound; photo mode is the in-app path.

## Releasing
1. Bump `version` in the root `Cargo.toml`; update `CHANGELOG.md`, `RELEASE_NOTES.md` (it becomes the
   release text), README images if relevant.
2. Commit, tag `vX.Y.Z`, push the tag. `.github/workflows/release.yml` builds macOS (universal DMG),
   Windows (installer, portable exe and zip) and Linux (tarball) and creates a **draft** release.
3. The owner publishes the draft on GitHub (needs a signed-in browser; the `gh` CLI isn't installed here).
   Builds are unsigned — first-launch instructions are in the release notes.

## What's next (candidates for 1.x)
From the plan's "Not yet" list, roughly by value:
* Real nearby stars (Gaia) and known exoplanets; more real nebulae.
* The Chronicle as a shareable book; challenges ("save Earth from the asteroid"); observatory mode.
* GPU N-body for thousands of particles; galaxy collisions; kilonovae, tidal disruption events, auroras,
  atmospheric entry fireballs.
* Video capture and a cinematic camera; signed/notarised installers; update check; localisation;
  accessibility options; a mod folder for data files.
* Known rough edges: auto-slow caps the clock at milestones, so star formation (tens of kyr per star) needs
  auto-slow off and 1 Myr/s to watch; the black hole's lensed secondary image draws as a large loop below
  the disk; catalogue stars show PROCEDURAL provenance tags in the inspector.
