# ADR 002: Why wgpu?

**Date:** 2026-07-20
**Status:** Accepted
**Deciders:** Cosmogon Core Team

## Context

We need a graphics API that can handle rendering an entire universe — from the surface of a planet to intergalactic distances — across macOS, Linux, Windows, and potentially the web. The rendering backend is one of the most critical architectural decisions because everything else (scene management, LOD, shaders) builds on top of it.

We need something that's cross-platform by default, has solid Rust bindings, supports compute shaders for future N-body simulation, and won't be obsolete in two years.

## Decision

We're going with **wgpu** (WebGPU API).

## Alternatives Considered

| API           | Verdict                                                    |
|---------------|------------------------------------------------------------|
| Vulkan        | Maximum control, maximum pain, platform-locked             |
| Metal         | Fast on macOS, useless everywhere else                     |
| OpenGL        | Legacy. Cross-platform but deeply limited                  |
| ash            | Raw Vulkan bindings in Rust — you still write Vulkan       |
| gfx-hal       | Deprecated in favor of wgpu                                |

## Tradeoffs

**Cross-Platform vs Peak Performance**
wgpu targets WebGPU, which means it runs on macOS (via Metal), Linux (via Vulkan), Windows (via D3D12/Vulkan), and the web (via WebGPU). We lose maybe 5-10% performance compared to raw Vulkan on any given platform, but we gain write-once-run-everywhere. For a universe simulator, portability matters more than squeezing out the last frame.

**WebGPU as Future Standard vs Vulkan Maturity**
WebGPU is the successor to OpenGL ES for the web, backed by W3C. It's simpler, more predictable, and designed for modern GPU architectures. Vulkan is mature and battle-tested, but it's also incredibly verbose. wgpu gives us WebGPU semantics today, with native backends for performance. We're betting on the future here.

**Rust-Native vs Raw FFI**
wgpu is written in Rust. The API feels Rusty — enums for pipeline states, traits for resources, proper error handling. Using Vulkan from Rust means FFI bindings, unsafe blocks everywhere, and manually managing validation layers. The developer experience difference is night and day.

**Abstraction Ceiling vs Developer Experience**
Raw Vulkan/Metal gives you absolute control — custom memory allocators, explicit synchronization, pipeline compilation tricks. wgpu abstracts some of that away. For 95% of what we need, the abstraction is fine. For the last 5% (if we hit performance walls), we can drop to platform-specific code through wgpu's hal layer.

**Compute Shader Support**
wgpu supports compute shaders natively. This is critical — we want to move N-body gravity simulation to the GPU eventually. Compute pipelines are first-class citizens in wgpu, not an afterthought like in some older APIs.

## Consequences

**Positive:**
- Single rendering codebase across all target platforms
- Modern API design that maps well to Rust's type system
- Active development with frequent releases and breaking changes tracked
- WebGPU standard means long-term relevance
- Compute support enables GPU-accelerated simulation
- Good documentation and growing example library

**Negative:**
- Slight abstraction overhead vs raw APIs (acceptable tradeoff)
- WebGPU spec still evolving, some features pending
- Less control over low-level GPU operations
- Debugging can be harder when wgpu adds its own validation

**Mitigations:**
- Profile early and often; don't wait for performance problems
- Use wgpu's hal layer when we need platform-specific optimizations
- Track WebGPU spec progress and plan around feature availability
- Contribute to wgpu if we hit blockers

## References

- [wgpu Documentation](https://docs.rs/wgpu/)
- [WebGPU Specification](https://www.w3.org/TR/webgpu/)
- [wgpu GitHub](https://github.com/gfx-rs/wgpu)
- [Graphics Programming in Rust](https://ferrisellis.com/tags/graphics/)
