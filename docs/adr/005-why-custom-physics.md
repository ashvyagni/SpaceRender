# ADR 005: Why Custom Physics Engine?

**Date:** 2026-07-20
**Status:** Accepted
**Deciders:** Cosmogon Core Team

## Context

We need a physics engine that handles Keplerian orbital mechanics, N-body gravitational interactions, time acceleration, and precise numerical integration. Existing Rust physics engines (Rapier, nphysics) are designed for games — box collisions, springs, rigid bodies. They don't do orbital elements, don't care about numerical precision over millions of timesteps, and don't support time acceleration.

Do we shoehorn a game physics engine into doing celestial mechanics, or build what we actually need?

## Decision

We're going with a **custom physics module** built specifically for orbital mechanics and N-body simulation.

## Alternatives Considered

| Engine   | Verdict                                                        |
|----------|----------------------------------------------------------------|
| Rapier   | Excellent for games, wrong domain entirely                     |
| nphysics | Deprecated, superseded by Rapier                               |
| Bullet   | C++ with Rust bindings, still game-focused                     |
| Custom   | More work, but the right tool for the job                      |

## Tradeoffs

**Celestial Mechanics vs Game Physics**
Game physics engines optimize for collision detection, constraint solving, and real-time responsiveness. We need Keplerian orbital elements (semi-major axis, eccentricity, inclination), time-varying orbital perturbations, and multi-body gravitational interactions. These are fundamentally different problems. Rapier doesn't know what a Hill sphere is.

**Exact Control vs Free Features**
With a custom engine, we control the integrator (Runge-Kutta, Verlet, or something exotic), the coordinate system (barycentric, heliocentric), and the scaling (AU, meters, or relativistic units). We can make it deterministic by design. We can add time acceleration without breaking physics. The tradeoff is we don't get collision detection, broadphase, or constraint solvers for free.

**Time Acceleration**
This is a hard requirement. Users will watch planetary orbits in fast-forward and slow down for close encounters. Game physics engines assume real-time or near-real-time. Our integrator needs to handle variable timesteps gracefully — something game engines actively avoid.

**The Hard Parts**
We don't get collision detection. If we want spacecraft docking physics later, we'll need to either build basic collision or integrate Rapier for that specific use case. We also need to implement our own broadphase for N-body (Barnes-Hut, FMM) when body counts get high.

## Consequences

**Positive:**
- Full control over integration methods and numerical precision
- Deterministic by design — matches our ADR 004
- Keplerian elements are first-class citizens
- Time acceleration is a core feature, not a hack
- Clean separation between physics and rendering

**Negative:**
- Significant development effort — no free lunch
- No collision detection (needed for spacecraft later)
- Must implement N-body acceleration structures ourselves
- More code to maintain and test

**Mitigations:**
- Start with simple Euler integration, upgrade to higher-order methods
- Use established algorithms (Barnes-Hut) for N-body acceleration
- Plan Rapier integration path for spacecraft physics when needed
- Validate against known analytical solutions (two-body Kepler problem)

## References

- [Keplerian Orbital Elements](https://en.wikipedia.org/wiki/Orbital_elements)
- [N-body Problem](https://en.wikipedia.org/wiki/N-body_problem)
- [Barnes-Hut Algorithm](https://en.wikipedia.org/wiki/Barnes%E2%80%93Hut_simulation)
- [Rapier Physics Engine](https://rapier.rs/)
