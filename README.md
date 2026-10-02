# Cosmogon

**A real-time digital universe simulator.**

> *Not a game. Not a toy. A universe.*

---

## What is this?

Cosmogon is a scientifically accurate, real-time digital universe simulator built in Rust. It simulates the entire solar system with real orbital mechanics, physically-based rendering, atmospheric scattering, and time acceleration — all running at 60fps.

The goal: make someone open the GitHub and think *"How did one person even build this?"*

## Features

- **Real Solar System** — Sun, 8 planets, major moons, all with real orbital elements
- **Keplerian Orbital Mechanics** — Accurate orbital periods, eccentricities, inclinations
- **Physically Based Rendering** — PBR materials, HDR, bloom, atmospheric scattering
- **Time Acceleration** — 1x to 10,000x speed, pause, step through time
- **Star Background** — 51 real stars with correct spectral colors
- **Procedural Terrain** — Earth, Mars, Moon with terrain generation
- **Atmospheric Scattering** — Rayleigh + Mie for Earth and Venus
- **Object Inspector** — Click any body to see mass, radius, orbital parameters
- **Camera System** — Fly-through, orbit, zoom with scroll wheel

## Architecture

```
cosmogon/
├── cosmogon_core      # Math, constants, coordinate systems
├── cosmogon_ecs       # ECS components, resources, system sets
├── cosmogon_physics   # Gravity, orbital mechanics, integrators
├── cosmogon_render    # wgpu rendering pipeline, shaders
├── cosmogon_scene     # Solar system data, celestial bodies
├── cosmogon_ui        # egui-based UI panels
└── cosmogon_app       # Main application, game loop
```

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust |
| Rendering | wgpu (WebGPU) |
| ECS | bevy_ecs |
| UI | egui |
| Math | glam (f32) + custom f64 |
| Parallelism | rayon |
| Serialization | serde |

## Quick Start

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- A GPU with Vulkan/Metal/WebGPU support
- Git

### Build & Run

```bash
# Clone the repository
git clone https://github.com/yourusername/cosmogon.git
cd cosmogon

# Build in release mode
cargo build --release

# Run the simulator
cargo run --release -p cosmogon_app
```

### Controls

| Key | Action |
|-----|--------|
| `Space` | Pause / Resume time |
| `1-5` | Set time acceleration (1x, 10x, 100x, 1000x, 10000x) |
| `Scroll` | Zoom in/out |
| `Mouse Drag` | Orbit camera |
| `Click` | Select celestial body |

## Documentation

- [Product Requirements](PRD.md) — What we're building and why
- [Architecture](ARCHITECTURE.md) — How it's all structured
- [Roadmap](docs/ROADMAP.md) — Multi-year development plan
- [Feature Backlog](docs/FEATURE_BACKLOG.md) — 300+ planned features
- [Brain](docs/brain.md) — Living knowledge base
- [Learning Roadmap](docs/LEARNING_ROADMAP.md) — What to learn and when
- [Development Workflow](docs/DEVELOPMENT_WORKFLOW.md) — How we work

## License

MIT OR Apache-2.0

## Acknowledgments

- Inspired by [Space Engine](http://spaceengine.org/), [Kerbal Space Program](https://www.kerbalspaceprogram.com/), [Universe Sandbox](https://universesandbox.com/)
- Real orbital data from [JPL Solar System Dynamics](https://ssd.jpl.nasa.gov/)
- Rendering techniques from [Learn OpenGL](https://learnopengl.com/) and [Real Shading in Unreal Engine 4](https://cdn2.unrealengine.com/Resources/files/2013SiggraphPresentationsNotes-26915738.pdf)
