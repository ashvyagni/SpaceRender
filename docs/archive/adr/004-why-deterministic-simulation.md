# ADR 004: Why Deterministic Simulation?

**Date:** 2026-07-20
**Status:** Accepted
**Deciders:** Cosmogon Core Team

## Context

A universe simulator has unique requirements: bugs need to be reproducible, physics needs to be testable against known solutions, and users should be able to save and restore exact simulation states. Non-deterministic behavior — where the same inputs produce different outputs — makes all of this impossible.

We need to decide: do we embrace determinism and its constraints, or accept non-determinism for simplicity?

## Decision

We're going with **deterministic simulation** where possible: seeded RNG, fixed timestep, ordered integration, deterministic math operations.

## Alternatives Considered

| Approach              | Verdict                                                     |
|-----------------------|-------------------------------------------------------------|
| Non-deterministic     | Simpler, but untestable, unreproducible, unhinged           |
| Fully deterministic   | Impossible on modern hardware (IEEE 754 edge cases)         |
| Deterministic where possible | The sweet spot — discipline without obsession          |

## Tradeoffs

**Determinism = Reproducible Bugs**
When a user reports "Jupiter flew into the sun," we can replay the exact same simulation from the same seed and see it happen. That's debugging power you can't get any other way. Non-deterministic simulations hide bugs behind randomness.

**Testability Against Known Solutions**
We can validate our Keplerian orbital propagation against analytical solutions. We can test N-body gravitational interactions against published benchmarks. Determinism makes physics validation trivial — same inputs, same outputs, every time.

**Save/Load Exact State**
Deterministic simulation means save states are small — just the seed and current timestep. We don't need to serialize every floating-point value. Load the seed, fast-forward, and you're back where you were. Enables "rewind and replay" features.

**Future Networking**
Determinism is the foundation for lockstep networking. If two machines run the same simulation with the same inputs, they get the same results. This is how RTS games handle thousands of units — and it applies to multi-user universe exploration.

**The Hard Parts**
Determinism requires discipline: no float reordering (compiler optimizations can change results), no parallel RNG without careful seeding, and careful handling of platform differences (different CPUs may produce slightly different floating-point results). It's not impossible, but it demands attention.

## Consequences

**Positive:**
- Reproducible bugs, testable physics, save/load, future networking
- Enables rewind and replay features
- Forces clean architecture — ordered systems, explicit state

**Negative:**
- More discipline required in code authoring
- Floating-point determinism across platforms is hard
- Fixed timestep requires careful interpolation for smooth rendering
- Limits some parallelization opportunities

**Mitigations:**
- Use fixed-point math where precision matters
- Seed RNGs explicitly, never use system randomness in simulation
- Profile with different compiler optimization levels
- Document determinism constraints for contributors

## References

- [Deterministic Game Engine Design](https://gamedev.stackexchange.com/questions/tagged/deterministic)
- [IEEE 754 Floating Point](https://en.wikipedia.org/wiki/IEEE_754)
- [Lockstep Networking](https://www.gabrielgambetta.com/client-server-game-architecture.html)
