# ADR 001: Why Rust?

**Date:** 2026-07-20
**Status:** Accepted
**Deciders:** Cosmogon Core Team

## Context

We're building a real-time digital universe simulator that needs to handle thousands of celestial bodies, render them with physically-based shading, and do it all at 60fps. The language choice here is foundational — wrong pick and we're fighting the toolchain instead of building the thing.

We need something that gives us raw performance without sacrificing safety, has a growing ecosystem for graphics work, and won't make us want to throw our laptops out the window after a month of development.

## Decision

We're going with **Rust**.

## Alternatives Considered

| Language   | Verdict                                                     |
|------------|-------------------------------------------------------------|
| C++        | The incumbent. Fast, mature, terrifying safety story        |
| Python     | Prototyping king, runtime performance peasant              |
| Zig        | Cool up-and-comer, but ecosystem too young for our needs    |
| C#         | Solid with Unity, but we're not Unity-dependent             |

## Tradeoffs

**Safety vs Learning Curve**
Rust's ownership model eliminates null pointer derefs and data races at compile time. That's huge for a concurrent simulation. The catch? The borrow checker will humble you for the first few weeks. Worth it. We'd rather fight the compiler now than chase segfaults at 3am during a demo.

**Performance vs Compile Times**
Zero-cost abstractions mean we write high-level code that compiles down to the same machine code as hand-tuned C. No garbage collector means no unpredictable pauses during simulation steps. The tradeoff is compile times — they're not great. Incremental compilation helps, and we'll structure the workspace to minimize recompilation.

**Ecosystem vs Maturity**
The Rust graphics and game dev ecosystem has grown massively. wgpu, winit, glam, egui — all solid. Is it as mature as C++'s decades of libraries? No. But the libraries we need exist and are actively maintained. The community is passionate and moving fast.

**Cargo vs CMake**
This isn't even close. Cargo handles dependencies, builds, testing, benchmarks, and documentation generation. CMake is a build system that makes you wish you were using something else. Rust's tooling is a genuine productivity multiplier.

**wgpu First-Class Support**
wgpu is written in Rust, for Rust. We get the best bindings, the best docs, and direct access to the developers. Using wgpu from C++ or Python would be a second-class experience.

## Consequences

**Positive:**
- Memory safety without GC = predictable performance for simulation
- Fearless concurrency for parallel physics integration
- Excellent tooling reduces friction across the entire development lifecycle
- Strong type system catches entire categories of bugs at compile time
- Growing community means more examples, tutorials, and shared knowledge

**Negative:**
- Steep learning curve, especially ownership and lifetimes
- Compile times can slow iteration (mitigated by workspace structure)
- Some graphics libraries less mature than C++ equivalents
- Smaller talent pool if we ever need to hire

**Mitigations:**
- invest in onboarding docs and examples within the project
- use `cargo watch` and incremental builds for fast feedback loops
- contribute upstream to libraries we depend on
- pair programming sessions to spread ownership knowledge

## References

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [Are We Game Yet?](https://arewegameyet.rs/)
- [Rust for Game Development](https://floooh.github.io/2024/09/05/rust-game-dev.html)
