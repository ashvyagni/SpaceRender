# ADR 003: Why ECS (bevy_ecs)?

**Date:** 2026-07-20
**Status:** Accepted
**Deciders:** Cosmogon Core Team

## Context

We need an architecture for managing potentially thousands of celestial bodies — each with position, velocity, mass, orbital elements, visual properties, and more. Traditional OOP scene graphs with deep inheritance hierarchies don't scale well for this. We need something data-oriented, cache-friendly, and that makes the simulation-to-render pipeline explicit and auditable.

The key question: how do we organize state so that physics runs fast, rendering stays efficient, and adding new components (atmospheres, rings, moons) doesn't require rewriting everything?

## Decision

We're going with **bevy_ecs**, used standalone (not the full Bevy engine).

## Alternatives Considered

| Architecture   | Verdict                                                       |
|----------------|---------------------------------------------------------------|
| OOP Scene Graph | Familiar, but inheritance hierarchies become spaghetti fast   |
| specs           | Solid ECS, but maintenance has slowed                        |
| hecs            | Lightweight, but fewer features and smaller community         |
| Custom ECS      | Full control, but massive time sink for something that exists |

## Tradeoffs

**Data-Oriented Performance vs Familiarity**
ECS stores components in contiguous arrays, not scattered across heap-allocated objects. This means iterating over all Position components is a sequential memory walk — cache-friendly and fast. For thousands of bodies, this matters. The tradeoff is learning to think in components and systems instead of objects and methods.

**System Ordering vs Implicit Dependencies**
In an OOP scene graph, the update order is whatever you traverse the tree. In ECS, systems declare what they read and write, and the scheduler can parallelize or order them explicitly. Physics reads Position and Velocity, writes Position. Renderer reads Position and Mesh. That's clear, testable, and debuggable.

**bevy_ecs vs Full Bevy**
Bevy is a fantastic game engine, but it bundles rendering, audio, assets, and more. We're building our own renderer with wgpu. Using just bevy_ecs gives us the architecture without the opinions we don't need. We get the ECS core, system scheduling, and the community's battle-tested patterns.

**Component-Based Design Maps to Celestial Bodies**
A planet is just: `Position + Mass + OrbitalElements + Atmosphere + Mesh`. Want to add rings? Add a `Rings` component. Want moons? Same thing. No inheritance diamond problem, no downcasting, no fragile base class. Components compose naturally.

**Losing Bevy's Ecosystem**
We won't have Bevy's renderer, asset loader, plugin system, or editor. That's intentional — we're building a specialized simulator, not a general game engine. We'll borrow patterns from Bevy where they make sense, but we own the rendering and physics pipeline.

## Consequences

**Positive:**
- Excellent performance characteristics for large entity counts
- Explicit system ordering makes the simulation pipeline auditable
- Component composition makes features additive, not invasive
- Active community with good docs and examples
- Standalone crate means no unnecessary dependencies

**Negative:**
- More boilerplate initially than a simple OOP approach
- ECS thinking is a paradigm shift for developers used to OOP
- Without Bevy, we write our own renderer integration, asset loading, etc.
- Learning curve for system scheduling and world setup

**Mitigations:**
- Create helper macros for common component/system patterns
- Document ECS patterns specific to celestial simulation
- Build small examples early to validate the architecture
- Pair programming sessions to spread ECS knowledge

## References

- [bevy_ecs Documentation](https://docs.rs/bevy_ecs/)
- [Entity Component System Wikipedia](https://en.wikipedia.org/wiki/Entity_component_system)
- [Type ECS in Rust](https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/)
- [Overwatch ECS Talk](https://www.youtube.com/watch?v=p4YkKRCfb2U)
