# Instructions for AI agents

Read [HANDOFF.md](HANDOFF.md) before changing anything: project state, how to verify changes (tests,
clippy, and headless PNG captures of the app), conventions, known traps and what's next.

Non-negotiable:
* `cargo test --release --workspace` stays green; look at visual changes with `--capture` before claiming
  they work.
* The simulation (`crates/cosmogon_sim`) is deterministic: `DMath` functions, keyed RNG streams, and a
  re-pinned golden fingerprint (noted in the changelog) for deliberate state changes.
* Commits: no `Co-Authored-By` trailers. Push only as GitHub user `ashvyagni` to
  `git@github.com:ashvyagni/SpaceRender.git`; if that isn't possible, stop and let the owner push.
