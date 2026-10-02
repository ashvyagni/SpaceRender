# ADR 006: Why Chunk Streaming + Octree LOD?

**Date:** 2026-07-20
**Status:** Accepted
**Deciders:** Cosmogon Core Team

## Context

The core vision of Cosmogon is seamless zoom — from standing on a planet's surface, to seeing the solar system, to viewing the galaxy, to the observable universe. This requires rendering at scales ranging from meters to gigaparsecs, with detail levels that vary by many orders of magnitude.

We can't render every star in the galaxy at full detail while also rendering terrain. We need a system that loads detail where the camera is looking and simplifies everything else.

## Decision

We're going with **octree-based spatial partitioning with LOD streaming**.

## Alternatives Considered

| Approach           | Verdict                                                     |
|--------------------|-------------------------------------------------------------|
| Brute force        | Render everything — instant OOM and 2fps                    |
| Quadtree           | 2D only, wrong for 3D space                                 |
| Fixed grid         | Can't adapt to varying density, wasteful                    |
| Octree + LOD       | Natural 3D hierarchy, adaptive detail, streaming-ready      |

## Tradeoffs

**3D Spatial Hierarchy vs Simplicity**
An octree recursively subdivides 3D space into eight children. Dense regions (solar systems) get more subdivisions. Empty regions (interstellar voids) stay coarse. This maps naturally to how the universe is structured — clumpy, not uniform.

**LOD = Detailed Close, Simple Far**
Near the camera, we render full terrain meshes with atmosphere and shadows. Far away, we render simple spheres or points. The octree provides the structure for switching between LOD levels. The tradeoff is managing smooth transitions — nobody wants popping artifacts as LOD levels change.

**Memory Streaming**
The octree nodes can be loaded and unloaded as the camera moves. We only keep the visible subtree in memory. For a universe-scale simulation, this is essential — we can't hold every asteroid mesh in VRAM simultaneously.

**The Hard Parts**
Frustum culling needs to efficiently skip entire octree branches. LOD transitions need to be smooth (geomorphing, cross-fading). Seams between LOD levels need to be managed. And all of this needs to work with our ECS architecture.

## Consequences

**Positive:**
- Enables the core vision of seamless universe-scale rendering
- Memory-efficient — only loads what's visible
- Naturally handles varying spatial density
- Provides structure for frustum culling and streaming

**Negative:**
- Complex implementation — octree, LOD, streaming, culling
- LOD transitions require careful management to avoid artifacts
- Needs integration with ECS for entity management
- Frustum culling on octree can be CPU-intensive if not optimized

**Mitigations:**
- Implement octree first, add LOD progressively
- Use established geomorphing techniques for smooth transitions
- Profile octree traversal and optimize hot paths
- Build LOD testing infrastructure early

## References

- [Octree Wikipedia](https://en.wikipedia.org/wiki/Octree)
- [Level of Detail (Rendering)](https://en.wikipedia.org/wiki/Level_of_detail_in_computer_graphics)
- [Clipmap Terrain](https://developer.nvidia.com/gpugems/gpugems/part-i-natural-effects/chapter-1-terrain-rendering-using-geoclipmapped-meshes)
- [Virtual Texturing](https://en.wikipedia.org/wiki/Virtual_texturing)
