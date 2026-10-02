# Risk Register — Cosmogon Project

Last updated: 2026-07-20

## Technical Risks

| ID    | Risk                                                      | Likelihood | Impact | Mitigation Strategy                                              |
|-------|-----------------------------------------------------------|------------|--------|------------------------------------------------------------------|
| T-001 | wgpu API breaking changes mid-development                 | Med        | High   | Pin wgpu version, abstract behind our own trait layer            |
| T-002 | f64 precision loss at extreme scales (galactic distances) | High       | High   | Use relative coordinates, f64 for positions, test precision bounds |
| T-003 | Atmospheric scattering shader too expensive at 60fps      | Med        | Med    | Level-of-detail on atmosphere, precomputed LUTs, fallback mode   |
| T-004 | Shader compilation failures on specific GPU hardware      | Med        | Med    | Test on target hardware matrix, use wgpu's shader validation     |
| T-005 | Memory usage exploding with many celestial bodies         | Med        | High   | Streaming LOD, object pooling, memory budgets per system         |
| T-006 | ECS overhead exceeds benefit for small body counts        | Low        | Low    | Benchmark at various scales, ECS still wins at ~100+ entities    |
| T-007 | Cross-platform rendering differences (Metal vs Vulkan)    | High       | Med    | Abstract platform specifics, test on all targets regularly       |
| T-008 | GPU compute shader limitations on older hardware          | Med        | Med    | Compute fallback to CPU, graceful degradation path               |

## Performance Risks

| ID    | Risk                                                      | Likelihood | Impact | Mitigation Strategy                                              |
|-------|-----------------------------------------------------------|------------|--------|------------------------------------------------------------------|
| P-001 | N-body O(N²) algorithm becomes bottleneck at scale        | High       | High   | Implement Barnes-Hut or FMM early, set performance budgets       |
| P-002 | Draw call count exceeding GPU budget                      | Med        | High   | Instancing, batching, aggressive culling, LOD system             |
| P-003 | Texture memory for procedural planets too high            | Med        | Med    | Virtual texturing, procedural generation at runtime              |
| P-004 | Allocation patterns causing frame time spikes             | Med        | Med    | Pre-allocated pools, avoid runtime allocation in hot paths       |
| P-005 | LOD transitions causing visible popping                   | High       | Med    | Geomorphing, cross-fade, hysteresis on LOD switches              |

## Mathematical Challenges

| ID    | Risk                                                      | Likelihood | Impact | Mitigation Strategy                                              |
|-------|-----------------------------------------------------------|------------|--------|------------------------------------------------------------------|
| M-001 | Orbital element singularities (circular/equatorial orbits)| High       | Med    | Handle edge cases explicitly, use regularized elements           |
| M-002 | Numerical integration drift over long timescales          | High       | High   | Use symplectic integrators, validate against analytical solutions|
| M-003 | Coordinate transform precision loss (heliocentric ↔ galac)| Med        | High   | Use barycentric coordinates, careful transform chains            |
| M-004 | Singularity at r=0 in gravity calculations                | Med        | High   | Softening parameter, collision detection before integration      |
| M-005 | Quaternion gimbal lock edge cases in camera/rotation      | Low        | Low    | Use quaternions consistently, test extreme rotations             |

## Rendering Challenges

| ID    | Risk                                                      | Likelihood | Impact | Mitigation Strategy                                              |
|-------|-----------------------------------------------------------|------------|--------|------------------------------------------------------------------|
| R-001 | Atmospheric scattering quality vs performance tradeoff    | Med        | Med    | Configurable quality levels, precomputed LUTs, screen-space     |
| R-002 | Star rendering at correct apparent magnitudes             | Med        | Low    | HDR rendering pipeline, bloom for bright stars, magnitude LUT   |
| R-003 | Shadow map resolution insufficient for large scenes       | Med        | Med    | Cascaded shadow maps, focus on nearby objects                    |
| R-004 | Anti-aliasing artifacts at extreme zoom levels            | Low        | Low    | MSAA + FXAA fallback, test at various zoom ranges               |

## Scope Creep

| ID    | Risk                                                      | Likelihood | Impact | Mitigation Strategy                                              |
|-------|-----------------------------------------------------------|------------|--------|------------------------------------------------------------------|
| S-001 | Feature scope expanding beyond defined milestones         | High       | High   | Strict milestone discipline, weekly scope review meetings        |
| S-002 | Trying to build everything at once instead of incrementally| Med        | High   | MVP-first approach, one feature at a time, ship early            |
| S-003 | Perfectionism blocking progress on core features          | Med        | Med    | "Good enough" mindset, iterate, ship and refine                 |
| S-004 | Adding "cool features" before MVP is solid                | High       | Med    | Feature freeze during MVP, backlog for post-MVP ideas            |

## Maintenance

| ID    | Risk                                                      | Likelihood | Impact | Mitigation Strategy                                              |
|-------|-----------------------------------------------------------|------------|--------|------------------------------------------------------------------|
| MA-001| Dependency updates breaking existing functionality        | Med        | Med    | Pin versions, update dependencies in isolation, run tests        |
| MA-002| Documentation rot as codebase evolves                     | High       | Low    | Doc comments in code, ADRs, CI enforces doc coverage            |
| MA-003| Technical debt accumulation over time                     | High       | Med    | Regular refactoring sprints, debt tracking in issues             |

---

## Risk Review Cadence

- **Weekly:** Quick scan for new risks, update likelihood/impact
- **Monthly:** Deep review, update mitigation strategies
- **Per release:** Full risk audit, close resolved risks
