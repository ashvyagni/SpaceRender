# Development Workflow — Cosmogon

## Git Branching Strategy

```
main          ← production-ready, tagged releases
  └── develop ← integration branch, all features merge here
        ├── feature/*   ← individual features
        ├── fix/*       ← bug fixes
        ├── docs/*      ← documentation only
        └── release/*   ← release preparation
```

- **main**: Always deployable. Tagged with semver. No direct commits.
- **develop**: Integration branch. All features merge here first. CI must pass.
- **feature/\***: Short-lived branches for specific features. Branch off develop, merge back via PR.
- **fix/\***: Bug fixes. Same flow as features.
- **docs/\***: Documentation-only changes.
- **release/\***: Cut from develop when ready for release. Only bug fixes and docs here.

## Commit Conventions

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

[optional body]

[optional footer]
```

**Types:**
- `feat` — new feature
- `fix` — bug fix
- `docs` — documentation only
- `style` — formatting, no code change
- `refactor` — code change that neither fixes a bug nor adds a feature
- `perf` — performance improvement
- `test` — adding or fixing tests
- `chore` — build process, CI, dependencies

**Examples:**
```
feat(physics): add Keplerian orbital propagation
fix(render): correct atmospheric scattering at sunset
docs(adr): add decision record for ECS architecture
perf(render): implement frustum culling via octree
test(physics): add regression test for two-body problem
```

## Pull Request Template

```markdown
## Description
<!-- What does this PR do? Why? -->

## Related Issue
<!-- Link to issue: Fixes #123, Relates to #456 -->

## Changes
- Change 1
- Change 2

## Testing
- [ ] Unit tests pass (`cargo test`)
- [ ] Clippy passes (`cargo clippy -- -D warnings`)
- [ ] Formatting is correct (`cargo fmt --check`)
- [ ] Manual testing done (if visual change)

## Screenshots
<!-- If visual change, include before/after -->

## Checklist
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] No new clippy warnings
- [ ] Branch is up to date with develop
```

## Issue Templates

### Bug Report
```markdown
**Describe the bug**
A clear description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. ...
2. ...

**Expected behavior**
What you expected to happen.

**Screenshots**
If applicable, add screenshots.

**Environment**
- OS: [e.g., macOS 15.0]
- GPU: [e.g., M3 Pro]
- Rust version: [e.g., 1.80.0]
```

### Feature Request
```markdown
**Is your feature request related to a problem?**
A clear description of the problem. Ex: "I'm always frustrated when..."

**Describe the solution you'd like**
What you want to happen.

**Describe alternatives you've considered**
Other solutions you've thought about.

**Additional context**
Any other context, screenshots, or references.
```

### Performance Issue
```markdown
**Describe the performance issue**
What's slow? When does it happen?

**Benchmark data**
If available, include criterion results or profiler output.

**Environment**
- OS, GPU, Rust version

**Expected performance**
What you expected vs what you got.
```

### Question
```markdown
**Your question**
What do you need help with?

**Context**
What were you working on? What have you tried?
```

## Testing Strategy

### Unit Tests
- Each module has its own tests in `tests/` or inline `#[cfg(test)]`
- Run with `cargo test`
- Aim for coverage on critical paths (physics, math, serialization)

### Integration Tests
- Cross-crate behavior tested in `tests/integration/`
- ECS system interactions tested end-to-end

### Visual Regression Tests
- Screenshot comparison for rendering output
- Baseline images stored in `tests/baselines/`
- Automated comparison with tolerance threshold

### Physics Accuracy Tests
- Compare simulation results against known analytical solutions
- Two-body Kepler problem, N-body benchmark problems
- Validate conservation laws (energy, angular momentum)

### Benchmarks
- criterion.rs for micro-benchmarks
- Track performance over time
- Benchmark critical paths: integration, rendering, ECS iteration

## Benchmarking

### Micro-benchmarks with criterion
```bash
cargo bench
```

Benchmarks live in `benches/` and cover:
- Physics integration performance
- ECS query performance
- Rendering pipeline throughput
- Serialization/deserialization speed

### Frame Time Tracking
- Log frame times per frame
- Track P95, P99 frame times
- Alert on regression beyond threshold

### Memory Usage Tracking
- Track allocation patterns
- Monitor peak memory usage
- Profile for leaks over long simulation runs

### GPU Profiling
- Use wgpu's built-in trace support
- Enable `WGPU_TRACE=1` for API call tracing
- Analyze GPU timeline for bottlenecks

## Profiling

### CPU Profiling
- **puffin** — lightweight, non-intrusive profiler
- **cargo flamegraph** — for hot path analysis
- **Instruments (macOS)** — for platform-specific profiling

### GPU Profiling
- **wgpu trace** — replay and analyze GPU command streams
- **RenderDoc** — frame capture and GPU debugging
- **Metal System Trace (macOS)** — Metal-specific profiling

### Usage
```bash
# Generate flamegraph
cargo flamegraph

# wgpu trace
WGPU_TRACE=1 cargo run
# Trace saved to trace/

# Puffin
# Add puffin to your binary, view at puffin viewer
```

## Documentation

### API Documentation
```bash
cargo doc --open
```
- Doc comments on all public items
- Examples in doc comments where helpful
- Link to relevant ADRs in module docs

### Architecture Documentation
- `docs/adr/` — Architecture Decision Records
- `docs/RISK_REGISTER.md` — Project risks and mitigations
- `docs/DEVELOPMENT_WORKFLOW.md` — This file
- `brain.md` — Living knowledge base, learnings, gotchas

### README.md
- Project overview and vision
- Quick start guide
- Build instructions per platform
- Contributing guidelines

## Code Quality

### Linting
```bash
cargo clippy -- -D warnings
```
- Zero warnings policy on develop
- CI fails on any clippy warning
- New lints addressed before merge

### Formatting
```bash
cargo fmt
```
- Run before every commit
- CI checks formatting

### Dependency Auditing
```bash
cargo deny check
```
- `deny.toml` at workspace root
- Audit for vulnerabilities, licenses, duplicates
- CI runs on every PR

### CI Pipeline
Every PR triggers:
1. `cargo fmt --check`
2. `cargo clippy -- -D warnings`
3. `cargo test`
4. `cargo doc`
5. `cargo deny check`
6. Build on all target platforms

## Release Cadence

### During Active Development
- **Minor releases** every 2-4 weeks (0.1.0, 0.2.0, ...)
- **Patch releases** as needed for critical fixes
- Features merged to develop, released when stable

### Release Process
1. Create `release/X.Y.Z` branch from develop
2. Final testing and bug fixes only
3. Update version in `Cargo.toml`
4. Update CHANGELOG.md
5. Tag release
6. Merge to main
7. Merge back to develop

### Semantic Versioning
- **MAJOR** (X.0.0): Breaking changes, major milestones
- **MINOR** (0.X.0): New features, backward compatible
- **PATCH** (0.0.X): Bug fixes, backward compatible

### Changelog
Maintained in `CHANGELOG.md`. Follow [Keep a Changelog](https://keepachangelog.com/) format. Every release gets an entry.
