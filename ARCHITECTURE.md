# Cosmogon — Master Architecture Document

> *"We're building a universe from scratch. No pressure."*

**Version:** 0.1.0  
**Status:** Early development — core systems taking shape  
**Rust Edition:** 2021  
**License:** TBD

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Crate Architecture](#2-crate-architecture)
3. [Rendering Architecture](#3-rendering-architecture)
4. [Physics Architecture](#4-physics-architecture)
5. [ECS Architecture](#5-ecs-architecture)
6. [Coordinate Systems](#6-coordinate-systems)
7. [Threading Model](#7-threading-model)
8. [Resource Management](#8-resource-management)
9. [Module Dependencies](#9-module-dependencies)
10. [Data Flow](#10-data-flow)
11. [Future Extensibility](#11-future-extensibility)

---

## 1. System Overview

### 1.1 What Is Cosmogon?

Cosmogon is a real-time digital universe simulator. Think of it as a sandbox where you can zoom from a galactic overview down to the surface of a planet, watch moons orbit gas giants, and see light bend around stars — all running at 60fps on consumer hardware.

We're using Rust because we want zero-cost abstractions, fearless concurrency, and内存安全 without a garbage collector. The rendering stack is wgpu (WebGPU), the ECS is bevy_ecs (standalone, not the full engine), and we're keeping things modular with a 7-crate workspace.

### 1.2 High-Level Block Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          COSMOGON UNIVERSE                             │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐              │
│  │  cosmogon_ui │◄───│ cosmogon_app │───►│ cosmogon_scene│              │
│  │   (egui)     │    │ (entry point)│    │  (solar sys) │              │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘              │
│         │                   │                   │                       │
│         │                   ▼                   │                       │
│         │          ┌────────────────┐           │                       │
│         │          │ cosmogon_ecs   │◄──────────┘                       │
│         │          │ (components +  │                                   │
│         │          │  systems)      │                                   │
│         │          └───────┬────────┘                                   │
│         │                  │                                            │
│         │      ┌───────────┼───────────┐                               │
│         │      ▼           ▼           ▼                               │
│         │  ┌────────┐ ┌────────┐ ┌────────┐                            │
│         │  │physics │ │render  │ │  core  │                            │
│         │  │(N-body)│ │(wgpu)  │ │(math)  │                            │
│         │  └────────┘ └────────┘ └────────┘                            │
│         │                                                              │
│         └──────────────────────────────────────────────────────────────│
│                                                                         │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      DATA FLOW                                  │   │
│  │                                                                 │   │
│  │  Input → Time Step → Physics → ECS Flush → Camera → Uniforms   │   │
│  │    → Render Passes → UI Overlay → Submit → Present              │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.3 7-Crate Workspace Architecture

```
cosmogon/
├── Cargo.toml              # Workspace root
├── crates/
│   ├── cosmogon_core/      # Math, constants, coordinate transforms
│   ├── cosmogon_ecs/       # ECS components, systems, resources
│   ├── cosmogon_physics/   # N-body gravity, orbital mechanics, integrators
│   ├── cosmogon_render/    # wgpu rendering pipeline, shaders, meshes
│   ├── cosmogon_scene/     # Solar system data, scene graph, generation
│   ├── cosmogon_ui/        # egui panels, inspector, debug overlay
│   └── cosmogon_app/       # Entry point, winit loop, system orchestration
├── shaders/                # WGSL shader files
├── assets/                 # Textures, fonts, config files
└── ARCHITECTURE.md         # You are here
```

### 1.4 Data Flow: Physics → ECS → Renderer → Screen

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────┐
│   Physics    │────►│     ECS      │────►│   Renderer   │────►│  Screen  │
│              │     │              │     │              │     │          │
│ N-body sim   │     │ Components   │     │ wgpu passes  │     │ Display  │
│ Kepler prop  │     │ Systems      │     │ Uniforms     │     │          │
│ Integrators  │     │ Queries      │     │ Meshes       │     │          │
│ Time mgmt    │     │ Resources    │     │ Shaders      │     │          │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────┘
       │                    │                    │                    │
       │   Position,       │   GPU buffers      │   Frame buffer    │
       │   Velocity,       │   written per      │   presented       │
       │   Mass updates    │   frame            │   to surface      │
       └───────────────────┴────────────────────┴────────────────────┘
```

**The pipeline in plain English:**

1. **Physics** computes gravitational forces, updates positions/velocities
2. **ECS** flushes all component changes (added/removed/mutated)
3. **Renderer** reads Position components, writes transform uniforms to GPU
4. **Render passes** execute: opaque geometry → atmosphere → post-processing → UI
5. **Present** flips the frame to the display

The key insight: the ECS is the **single source of truth**. Physics writes to it, rendering reads from it, UI inspects it. No direct data passing between systems — everything goes through components and resources.

### 1.5 Crate Dependency Graph

```
cosmogon_core          (no internal dependencies — leaf crate)
    │
    ├── cosmogon_ecs   (depends on: core)
    │       │
    │       ├── cosmogon_physics   (depends on: core, ecs)
    │       ├── cosmogon_scene     (depends on: core, ecs)
    │       ├── cosmogon_render    (depends on: core, ecs)
    │       └── cosmogon_ui        (depends on: core, ecs)
    │
    └── cosmogon_app   (depends on: ALL of the above)
```

`cosmogon_core` is the foundation. Everything depends on it. `cosmogon_app` is the top-level orchestrator that wires everything together.

---

## 2. Crate Architecture

### 2.1 cosmogon_core — The Foundation

**Purpose:** Math types, physical constants, coordinate transforms, and utility traits. This crate has zero dependencies on other Cosmogon crates — it's pure math and constants.

#### 2.1.1 Math Types

```rust
// Double-precision for physics (positions in AU, velocities in AU/yr)
pub type Vec3d = glam::DVec3;
pub type Mat4d = glam::DMat4;
pub type Quatd = glam::DQuat;

// Single-precision for rendering (GPU wants f32)
pub type Vec3f = glam::Vec3;
pub type Mat4f = glam::Mat4;
pub type Quatf = glam::Quat;

/// Heliocentric ecliptic coordinates (primary reference frame)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CelestialCoords {
    /// Position in AU from the Sun
    pub position: Vec3d,
    /// Velocity in AU/year
    pub velocity: Vec3d,
    /// Time of measurement (Julian date)
    pub epoch: f64,
}

/// Orbital elements in canonical form
#[derive(Debug, Clone, Copy)]
pub struct OrbitalElements {
    pub semi_major_axis: f64,    // AU
    pub eccentricity: f64,       // dimensionless
    pub inclination: f64,        // radians
    pub longitude_ascending: f64, // radians (Ω)
    pub argument_perihelion: f64, // radians (ω)
    pub mean_anomaly: f64,       // radians (M at epoch)
    pub epoch: f64,              // Julian date
}
```

#### 2.1.2 Physical Constants

```rust
pub mod constants {
    /// Gravitational constant (m³ kg⁻¹ s⁻²)
    pub const G: f64 = 6.67430e-11;

    /// Speed of light (m/s)
    pub const C: f64 = 299_792_458.0;

    /// Astronomical Unit (meters)
    pub const AU: f64 = 1.496e11;

    /// Light-year (meters)
    pub const LIGHT_YEAR: f64 = 9.461e15;

    /// Earth mass (kg)
    pub const EARTH_MASS: f64 = 5.972e24;

    /// Sun mass (kg)
    pub const SUN_MASS: f64 = 1.989e30;

    /// Jupiter mass (kg)
    pub const JUPITER_MASS: f64 = 1.898e27;

    /// Earth radius (meters)
    pub const EARTH_RADIUS: f64 = 6.371e6;

    /// Sun radius (meters)
    pub const SUN_RADIUS: f64 = 6.957e8;

    /// Julian year (seconds)
    pub const JULIAN_YEAR: f64 = 365.25 * 24.0 * 3600.0;

    /// Gravitational parameter of Sun (m³/s²)
    pub const MU_SUN: f64 = 1.327e20;

    /// Gravitational parameter of Earth (m³/s²)
    pub const MU_EARTH: f64 = 3.986e14;
}
```

#### 2.1.3 Coordinate Transforms

```rust
impl CelestialCoords {
    /// Convert from AU/m to meters (for internal physics)
    pub fn to_meters(&self) -> Vec3d {
        self.position * constants::AU
    }

    /// Convert from orbital elements to Cartesian
    pub fn from_orbital_elements(elements: &OrbitalElements) -> Self {
        // Kepler → Cartesian conversion
        // ... (see Physics section for full algorithm)
    }
}

/// Convert from heliocentric ecliptic to equatorial coordinates
pub fn ecliptic_to_equatorial(pos: Vec3d, obliquity: f64) -> Vec3d {
    let (sin_e, cos_e) = obliquity.sin_cos();
    Vec3d::new(
        pos.x,
        pos.y * cos_e + pos.z * sin_e,
        -pos.y * sin_e + pos.z * cos_e,
    )
}
```

#### 2.1.4 Utility Traits

```rust
/// Trait for types that can be converted to GPU-friendly f32
pub trait ToGpu {
    type GpuType;
    fn to_gpu(&self) -> Self::GpuType;
}

/// Trait for types with a natural "up" vector
pub trait HasUp {
    fn up() -> Vec3d;
}

/// Trait for astronomical objects with mass
pub trait Massive {
    fn mass(&self) -> f64;
    fn gravitational_parameter(&self) -> f64 {
        self.mass() * constants::G
    }
}
```

---

### 2.2 cosmogon_ecs — The Nervous System

**Purpose:** ECS components, system sets, system ordering, and resource definitions. This crate defines the **data model** that all other crates use.

#### 2.2.1 Core Components

```rust
use bevy_ecs::prelude::*;

/// Position in heliocentric ecliptic coordinates (AU)
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Position {
    pub coords: Vec3d,
}

/// Velocity in AU/year
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Velocity {
    pub linear: Vec3d,
    pub angular: Vec3d, // rad/year
}

/// Mass in kg
#[derive(Component, Debug, Clone, Copy)]
pub struct Mass(pub f64);

/// Radius in meters (for rendering and collision)
#[derive(Component, Debug, Clone, Copy)]
pub struct Radius(pub f64);

/// Orbital elements (for analytical Keplerian propagation)
#[derive(Component, Debug, Clone, Copy)]
pub struct OrbitalElements {
    pub elements: cosmogon_core::OrbitalElements,
}

/// Marks a body as renderable with a specific visual type
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum Renderable {
    Planet { has_atmosphere: bool },
    Star { temperature: f64 },      // Kelvin, for spectral color
    Moon,
    Asteroid,
    Comet { tail_length: f64 },
}

/// Camera component — attached to the active camera entity
#[derive(Component, Debug, Clone)]
pub struct Camera {
    pub fov: f32,           // degrees
    pub near: f64,          // AU
    pub far: f64,           // AU
    pub target: Option<Entity>,
    pub distance: f64,      // AU from target
    pub pitch: f64,         // radians
    pub yaw: f64,           // radians
}

/// Rotation state (for tidally locked bodies, etc.)
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Rotation {
    pub axis: Vec3d,
    pub period: f64,  // seconds
    pub phase: f64,   // radians at epoch
}

/// Visual properties (color, albedo, emissive)
#[derive(Component, Debug, Clone, Copy)]
pub struct VisualProperties {
    pub base_color: [f32; 3],
    pub albedo: f32,
    pub emissive_strength: f32,
    pub roughness: f32,
}
```

#### 2.2.2 System Sets

```rust
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PhysicsSet {
    GravityCalculation,
    OrbitalPropagation,
    Integration,
    CollisionDetection,
    TimeManagement,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenderSet {
    UniformPreparation,
    MeshGeneration,
    RenderPassRecording,
    PostProcessing,
    UIRendering,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum UISet {
    InputHandling,
    PanelDrawing,
    DebugOverlay,
}
```

#### 2.2.3 System Ordering

```
PhysicsSet::TimeManagement
    → PhysicsSet::GravityCalculation
        → PhysicsSet::OrbitalPropagation
            → PhysicsSet::Integration
                → PhysicsSet::CollisionDetection

RenderSet::UniformPreparation
    → RenderSet::MeshGeneration
        → RenderSet::RenderPassRecording
            → RenderSet::PostProcessing
                → RenderSet::UIRendering

UISet::InputHandling
    → UISet::PanelDrawing
        → UISet::DebugOverlay
```

#### 2.2.4 Resources

```rust
/// Global simulation time state
#[derive(Resource, Debug, Clone)]
pub struct TimeState {
    /// Current simulation time (Julian date)
    pub current_time: f64,
    /// Time acceleration multiplier (1.0 = real-time)
    pub time_scale: f64,
    /// Whether simulation is paused
    pub paused: bool,
    /// Fixed timestep for physics (seconds)
    pub physics_dt: f64,
    /// Accumulated time for fixed timestep
    pub accumulator: f64,
    /// Wall-clock time since last frame
    pub frame_delta: f64,
}

/// Camera state (read by renderer, written by UI/input)
#[derive(Resource, Debug, Clone)]
pub struct CameraState {
    /// View matrix (computed from camera component)
    pub view: Mat4d,
    /// Projection matrix
    pub projection: Mat4d,
    /// Combined view-projection
    pub view_projection: Mat4d,
    /// Camera position in world space
    pub position: Vec3d,
    /// Current target entity
    pub target: Option<Entity>,
}

/// Rendering state (device info, frame count, etc.)
#[derive(Resource, Debug, Clone)]
pub struct RenderState {
    pub frame_count: u64,
    pub resolution: [u32; 2],
    pub hdr_enabled: bool,
    pub bloom_enabled: bool,
    pub msaa_samples: u32,
}

/// Input state (keyboard, mouse, scroll)
#[derive(Resource, Debug, Clone, Default)]
pub struct InputState {
    pub mouse_delta: [f32; 2],
    pub scroll_delta: f32,
    pub keys_pressed: Vec<Key>,
    pub mouse_buttons: Vec<MouseButton>,
}
```

---

### 2.3 cosmogon_physics — The Engine Room

**Purpose:** Gravitational N-body simulation, Keplerian orbital mechanics, numerical integrators, and time management.

#### 2.3.1 Gravity Solver

```rust
pub struct NBodySolver {
    /// Softening parameter (AU) to prevent singularities
    pub softening: f64,
    /// Direct summation for small N, Barnes-Hut for large N
    pub strategy: GravityStrategy,
}

pub enum GravityStrategy {
    DirectSummation,
    BarnesHut { theta: f64 }, // opening angle
}

impl NBodySolver {
    /// Compute gravitational accelerations for all bodies
    pub fn compute_accelerations(
        &self,
        positions: &[Vec3d],
        masses: &[f64],
        accelerations: &mut [Vec3d],
    ) {
        accelerations.fill(Vec3d::ZERO);

        match self.strategy {
            GravityStrategy::DirectSummation => {
                // O(N²) — fine for < 10,000 bodies
                for i in 0..positions.len() {
                    for j in (i + 1)..positions.len() {
                        let r = positions[j] - positions[i];
                        let dist_sq = r.length_squared() + self.softening * self.softening;
                        let dist = dist_sq.sqrt();
                        let force = constants::G * masses[i] * masses[j] / dist_sq;
                        let accel = force * r / (dist * masses[i]);

                        accelerations[i] += accel;
                        accelerations[j] -= accel * (masses[i] / masses[j]);
                    }
                }
            }
            GravityStrategy::BarnesHut { theta } => {
                // O(N log N) — for large N
                let tree = BarnesHutTree::build(positions, masses);
                for i in 0..positions.len() {
                    accelerations[i] = tree.compute_force(
                        positions[i], masses[i], self.softening, theta
                    );
                }
            }
        }
    }
}
```

#### 2.3.2 Keplerian Orbital Mechanics

```rust
/// Solve Kepler's equation: M = E - e*sin(E)
/// Uses Newton-Raphson iteration
pub fn solve_kepler(mean_anomaly: f64, eccentricity: f64, tol: f64) -> f64 {
    let mut e = mean_anomaly; // initial guess
    for _ in 0..100 {  // max iterations
        let f = e - eccentricity * e.sin() - mean_anomaly;
        let f_prime = 1.0 - eccentricity * e.cos();
        let delta = f / f_prime;
        e -= delta;
        if delta.abs() < tol {
            break;
        }
    }
    e
}

/// Convert mean anomaly → eccentric anomaly → true anomaly
pub fn anomalies(mean: f64, e: f64) -> (f64, f64) {
    let eccentric_anomaly = solve_kepler(mean, e, 1e-12);
    let true_anomaly = 2.0 * ((1.0 + e).sqrt() * (eccentric_anomaly / 2.0).sin())
        .atan2((1.0 - e).sqrt() * (eccentric_anomaly / 2.0).cos());
    (eccentric_anomaly, true_anomaly)
}

/// Position from orbital elements
pub fn orbital_position(elements: &OrbitalElements) -> (Vec3d, Vec3d) {
    let (e_anom, v_anom) = anomalies(elements.mean_anomaly, elements.eccentricity);

    let r = elements.semi_major_axis * (1.0 - elements.eccentricity * e_anom.cos());

    // Position in orbital plane
    let x_orb = r * v_anom.cos();
    let y_orb = r * v_anom.sin();

    // Rotation matrices for orbital elements
    let cos_omega = elements.longitude_ascending.cos();
    let sin_omega = elements.longitude_ascending.sin();
    let cos_w = elements.argument_perihelion.cos();
    let sin_w = elements.argument_perihelion.sin();
    let cos_i = elements.inclination.cos();
    let sin_i = elements.inclination.sin();

    // Transform to ecliptic coordinates
    let x = x_orb * (cos_omega * cos_w - sin_omega * sin_w * cos_i)
          - y_orb * (cos_omega * sin_w + sin_omega * cos_w * cos_i);
    let y = x_orb * (sin_omega * cos_w + cos_omega * sin_w * cos_i)
          - y_orb * (sin_omega * sin_w - cos_omega * cos_w * cos_i);
    let z = x_orb * (sin_w * sin_i) + y_orb * (cos_w * sin_i);

    let position = Vec3d::new(x, y, z);

    // Velocity (vis-viva + angular momentum)
    let mu = constants::MU_SUN;
    let h = (mu * elements.semi_major_axis * (1.0 - elements.eccentricity * elements.eccentricity)).sqrt();
    let vx = ...; // (computed from orbital velocity vector)
    let vy = ...;
    let vz = ...;
    let velocity = Vec3d::new(vx, vy, vz);

    (position, velocity)
}
```

#### 2.3.3 Numerical Integrators

```rust
pub trait Integrator {
    fn integrate(
        &self,
        positions: &[Vec3d],
        velocities: &[Vec3d],
        masses: &[f64],
        dt: f64,
        gravity: &NBodySolver,
    ) -> (Vec<Vec3d>, Vec<Vec3d>);
}

/// Runge-Kutta 4th order (primary integrator)
pub struct RK4Integrator;

impl Integrator for RK4Integrator {
    fn integrate(
        &self,
        positions: &[Vec3d],
        velocities: &[Vec3d],
        masses: &[f64],
        dt: f64,
        gravity: &NBodySolver,
    ) -> (Vec<Vec3d>, Vec<Vec3d>) {
        let n = positions.len();
        let mut new_pos = vec![Vec3d::ZERO; n];
        let mut new_vel = vec![Vec3d::ZERO; n];

        // k1
        let mut k1_acc = vec![Vec3d::ZERO; n];
        gravity.compute_accelerations(positions, masses, &mut k1_acc);

        // k2
        let mid_pos: Vec<_> = positions.iter().zip(velocities.iter())
            .map(|(p, v)| *p + *v * (dt * 0.5))
            .collect();
        let mid_vel: Vec<_> = velocities.iter().zip(k1_acc.iter())
            .map(|(v, a)| *v + *a * (dt * 0.5))
            .collect();
        let mut k2_acc = vec![Vec3d::ZERO; n];
        gravity.compute_accelerations(&mid_pos, masses, &mut k2_acc);

        // k3 (similar to k2 but using k2 results)
        // k4 (using full step with k3)

        // Combine: result = (k1 + 2*k2 + 2*k3 + k4) / 6
        for i in 0..n {
            new_pos[i] = positions[i] + (velocities[i] * dt);
            new_vel[i] = velocities[i] + (k1_acc[i] * dt);
        }

        (new_pos, new_vel)
    }
}

/// Velocity Verlet (symplectic, energy-preserving)
pub struct VerletIntegrator;

impl Integrator for VerletIntegrator {
    fn integrate(
        &self,
        positions: &[Vec3d],
        velocities: &[Vec3d],
        masses: &[f64],
        dt: f64,
        gravity: &NBodySolver,
    ) -> (Vec<Vec3d>, Vec<Vec3d>) {
        let n = positions.len();
        let mut acc = vec![Vec3d::ZERO; n];
        gravity.compute_accelerations(positions, masses, &mut acc);

        let mut new_pos = vec![Vec3d::ZERO; n];
        let mut new_vel = vec![Vec3d::ZERO; n];

        // Position update
        for i in 0..n {
            new_pos[i] = positions[i] + velocities[i] * dt + acc[i] * 0.5 * dt * dt;
        }

        // Compute new accelerations
        let mut new_acc = vec![Vec3d::ZERO; n];
        gravity.compute_accelerations(&new_pos, masses, &mut new_acc);

        // Velocity update (average of old and new acceleration)
        for i in 0..n {
            new_vel[i] = velocities[i] + (acc[i] + new_acc[i]) * 0.5 * dt;
        }

        (new_pos, new_vel)
    }
}
```

#### 2.3.4 Time Management

```rust
impl TimeState {
    /// Advance simulation time by frame delta
    pub fn advance(&mut self, frame_delta: f64) {
        self.frame_delta = frame_delta;

        if self.paused {
            return;
        }

        let scaled_dt = frame_delta * self.time_scale;
        self.accumulator += scaled_dt;

        // Fixed timestep for physics
        while self.accumulator >= self.physics_dt {
            self.current_time += self.physics_dt / constants::JULIAN_YEAR;
            self.accumulator -= self.physics_dt;
        }
    }

    /// Get interpolation factor for smooth rendering
    pub fn interpolation_factor(&self) -> f64 {
        self.accumulator / self.physics_dt
    }
}
```

---

### 2.4 cosmogon_render — The Visual Cortex

**Purpose:** wgpu device management, render pipelines, shader management, mesh generation, and post-processing.

#### 2.4.1 Device Initialization

```rust
pub struct RenderContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub depth_texture: wgpu::Texture,
    pub depth_view: wgpu::TextureView,
}

impl RenderContext {
    pub async fn new(window: &Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Cosmogon Device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ).await.unwrap();

        // Surface configuration
        let size = window.inner_size();
        let surface_config = surface.get_capabilities(&adapter).formats[0];
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_config,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // Depth buffer
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: size.width,
                height: size.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                 | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        Self {
            device, queue, surface,
            surface_config: config,
            depth_texture,
            depth_view: depth_texture.create_view(&Default::default()),
        }
    }
}
```

#### 2.4.2 Render Pass Structure

```
Frame Render Passes:
─────────────────────

1. Opaque Pass (depth-tested, back-to-front)
   ├── Sphere meshes (planets, moons)
   ├── Billboard meshes (stars)
   └── Uses: depth buffer, camera uniforms, light uniforms

2. Atmosphere Pass (additive blending)
   ├── Atmospheric scattering for planets with atmosphere
   ├── Rayleigh + Mie scattering in fragment shader
   └── Uses: depth buffer (read-only), camera uniforms

3. Post-Processing Pass
   ├── HDR tone mapping (ACES)
   ├── Bloom (downsample → blur → upsample → composite)
   └── Uses: HDR render target

4. UI Pass
   ├── egui panels (inspector, time controls, debug)
   ├── Rendered directly to swapchain
   └── Uses: egui-wgpu integration

5. Final composite → Present
```

#### 2.4.3 Uniform Buffer Management

```rust
/// Camera uniforms — updated every frame
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    pub view: [[f32; 4]; 4],
    pub projection: [[f32; 4]; 4],
    pub view_position: [f32; 4],  // xyz + padding
    pub time: f32,
    pub padding: [f32; 3],
}

/// Per-object uniforms — updated per visible object
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ObjectUniforms {
    pub model: [[f32; 4]; 4],
    pub base_color: [f32; 4],
    pub emissive: [f32; 4],  // xyz + strength in w
    pub roughness_metallic: [f32; 4],  // x=roughness, y=metallic, z=w
    pub flags: u32,         // bit 0: has_atmosphere, bit 1: is_star
    pub _pad: [f32; 3],
}

/// Light uniforms — sun + any other light sources
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniforms {
    pub positions: [[f32; 4]; MAX_LIGHTS],
    pub colors: [[f32; 4]; MAX_LIGHTS],
    pub counts: u32,
    pub _pad: [f32; 3],
}
```

#### 2.4.4 Mesh Generation

```rust
/// UV Sphere — for planets and moons
pub fn uv_sphere(
    radius: f32,
    sectors: u32,
    stacks: u32,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    for stack in 0..=stacks {
        let phi = std::f32::consts::PI * stack as f32 / stacks as f32;
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();

        for sector in 0..=sectors {
            let theta = 2.0 * std::f32::consts::PI * sector as f32 / sectors as f32;
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();

            let x = cos_theta * sin_phi;
            let y = cos_phi;
            let z = sin_theta * sin_phi;

            vertices.push(Vertex {
                position: [x * radius, y * radius, z * radius],
                normal: [x, y, z],
                uv: [
                    sector as f32 / sectors as f32,
                    stack as f32 / stacks as f32,
                ],
            });
        }
    }

    for stack in 0..stacks {
        for sector in 0..sectors {
            let first = stack * (sectors + 1) + sector;
            let second = first + sectors + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    (vertices, indices)
}

/// Billboard Quad — for stars and particles
pub fn billboard_quad(size: f32) -> (Vec<Vertex>, Vec<u32>) {
    let half = size * 0.5;
    let vertices = vec![
        Vertex { position: [-half, -half, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 0.0] },
        Vertex { position: [ half, -half, 0.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 0.0] },
        Vertex { position: [ half,  half, 0.0], normal: [0.0, 0.0, 1.0], uv: [1.0, 1.0] },
        Vertex { position: [-half,  half, 0.0], normal: [0.0, 0.0, 1.0], uv: [0.0, 1.0] },
    ];
    let indices = vec![0, 1, 2, 0, 2, 3];
    (vertices, indices)
}
```

#### 2.4.5 Shader Organization

```
shaders/
├── planet.wgsl          # Planet rendering (PBR-like)
├── star.wgsl            # Star billboard with spectral color
├── atmosphere.wgsl      # Atmospheric scattering
├── bloom_downsample.wgsl
├── bloom_blur.wgsl
├── bloom_upsample.wgsl
├── tone_map.wgsl        # HDR → LDR tone mapping
└── ui.wgsl              # UI overlay (via egui)
```

---

### 2.5 cosmogon_scene — The Universe Blueprint

**Purpose:** Solar system data, celestial body definitions, scene graph, and procedural generation.

#### 2.5.1 Solar System Data (Real Values)

```rust
pub fn create_solar_system() -> Vec<CelestialBody> {
    vec![
        CelestialBody {
            name: "Sun".to_string(),
            mass: 1.989e30,
            radius: 6.957e8,
            position: Vec3d::ZERO,
            velocity: Vec3d::ZERO,
            renderable: Renderable::Star { temperature: 5778.0 },
            visual: VisualProperties {
                base_color: [1.0, 0.9, 0.8],
                emissive_strength: 10.0,
                ..default()
            },
            ..default()
        },
        CelestialBody {
            name: "Mercury".to_string(),
            mass: 3.301e23,
            radius: 2.4397e6,
            orbital: Some(OrbitalElements {
                semi_major_axis: 0.387,
                eccentricity: 0.2056,
                inclination: deg_to_rad(7.0),
                longitude_ascending: deg_to_rad(48.33),
                argument_perihelion: deg_to_rad(29.12),
                mean_anomaly: deg_to_rad(174.796),
                epoch: 2451545.0,
            }),
            renderable: Renderable::Planet { has_atmosphere: false },
            visual: VisualProperties {
                base_color: [0.6, 0.6, 0.6],
                albedo: 0.142,
                ..default()
            },
            ..default()
        },
        CelestialBody {
            name: "Venus".to_string(),
            mass: 4.867e24,
            radius: 6.0518e6,
            orbital: Some(OrbitalElements {
                semi_major_axis: 0.723,
                eccentricity: 0.0068,
                inclination: deg_to_rad(3.39),
                longitude_ascending: deg_to_rad(76.68),
                argument_perihelion: deg_to_rad(54.88),
                mean_anomaly: deg_to_rad(50.115),
                epoch: 2451545.0,
            }),
            renderable: Renderable::Planet { has_atmosphere: true },
            visual: VisualProperties {
                base_color: [0.8, 0.7, 0.5],
                albedo: 0.77,
                ..default()
            },
            ..default()
        },
        // ... Earth, Mars, Jupiter, Saturn, Uranus, Neptune
        // Moon (Earth's satellite)
    ]
}
```

#### 2.5.2 Scene Graph

```rust
pub struct SceneGraph {
    /// Root entity (usually the Sun)
    pub root: Entity,
    /// Mapping from body name to entity
    pub body_entities: HashMap<String, Entity>,
    /// Parent-child relationships for orbit visualization
    pub hierarchy: HashMap<Entity, Vec<Entity>>,
}

impl SceneGraph {
    pub fn build(bodies: Vec<CelestialBody>, world: &mut World) -> Self {
        let mut graph = SceneGraph {
            root: Entity::PLACEHOLDER,
            body_entities: HashMap::new(),
            hierarchy: HashMap::new(),
        };

        for body in bodies {
            let entity = world.spawn((
                Position { coords: body.position },
                Velocity { linear: body.velocity, angular: Vec3d::ZERO },
                Mass(body.mass),
                Radius(body.radius),
                body.renderable,
                body.visual,
            )).id();

            graph.body_entities.insert(body.name.clone(), entity);

            if body.name == "Sun" {
                graph.root = entity;
            }
        }

        graph
    }
}
```

---

### 2.6 cosmogon_ui — The Control Panel

**Purpose:** egui-based UI panels for inspecting objects, controlling time, adjusting camera, and debugging.

#### 2.6.1 Panel Architecture

```rust
pub struct UIPanels {
    pub inspector: InspectorPanel,
    pub time_control: TimeControlPanel,
    pub camera_control: CameraControlPanel,
    pub debug_overlay: DebugOverlay,
}

pub struct InspectorPanel {
    pub selected_entity: Option<Entity>,
    pub scroll_offset: f32,
}

impl InspectorPanel {
    pub fn draw(&mut self, ui: &mut egui::Ui, world: &World) {
        if let Some(entity) = self.selected_entity {
            if let Some(name) = world.get::<Name>(entity) {
                ui.heading(&name.0);
            }

            if let Some(pos) = world.get::<Position>(entity) {
                ui.label(format!("Position: {:.3} AU", pos.coords.length()));
            }

            if let Some(vel) = world.get::<Velocity>(entity) {
                ui.label(format!("Velocity: {:.3} AU/yr", vel.linear.length()));
            }

            if let Some(mass) = world.get::<Mass>(entity) {
                ui.label(format!("Mass: {:.3e} kg", mass.0));
            }

            if let Some(orbital) = world.get::<OrbitalElements>(entity) {
                ui.separator();
                ui.label("Orbital Elements:");
                ui.label(format!("  a = {:.3} AU", orbital.elements.semi_major_axis));
                ui.label(format!("  e = {:.4}", orbital.elements.eccentricity));
                ui.label(format!("  i = {:.2}°", orbital.elements.inclination.to_degrees()));
            }
        }
    }
}

pub struct TimeControlPanel {
    pub time_scale: f64,
    pub paused: bool,
}

impl TimeControlPanel {
    pub fn draw(&mut self, ui: &mut egui::Ui, time_state: &mut TimeState) {
        ui.heading("Time Control");

        ui.horizontal(|ui| {
            if ui.button("⏸").clicked() {
                time_state.paused = !time_state.paused;
            }
            if ui.button("◀◀").clicked() {
                time_state.time_scale = (time_state.time_scale * 0.5).max(0.001);
            }
            if ui.button("▶▶").clicked() {
                time_state.time_scale = (time_state.time_scale * 2.0).min(1000.0);
            }
        });

        ui.add(egui::Slider::new(&mut self.time_scale, 0.001..=1000.0)
            .logarithmic(true)
            .text("Time Scale"));

        ui.label(format!("Time: JD {:.2}", time_state.current_time));
    }
}

pub struct DebugOverlay {
    pub fps: f32,
    pub entity_count: usize,
    pub physics_steps: u64,
}

impl DebugOverlay {
    pub fn draw(&self, ui: &mut egui::Ui) {
        ui.window("Debug").show(ui.ctx(), |ui| {
            ui.label(format!("FPS: {:.1}", self.fps));
            ui.label(format!("Entities: {}", self.entity_count));
            ui.label(format!("Physics Steps: {}", self.physics_steps));
        });
    }
}
```

---

### 2.7 cosmogon_app — The Conductor

**Purpose:** Application entry point, winit event loop, window creation, input handling, main game loop, and system orchestration.

#### 2.7.1 Application Structure

```rust
pub struct CosmogonApp {
    pub window: Arc<Window>,
    pub render_context: RenderContext,
    pub ecs_world: World,
    pub ecs_schedule: Schedule,
    pub ui_state: UIPanels,
    pub input_state: InputState,
}

impl CosmogonApp {
    pub async fn new() -> Self {
        let event_loop = EventLoop::new().unwrap();
        let window = Arc::new(
            WindowBuilder::new()
                .with_title("Cosmogon — Digital Universe")
                .with_inner_size(LogicalSize::new(1920, 1080))
                .build(&event_loop)
                .unwrap()
        );

        let render_context = RenderContext::new(&window).await;

        // Initialize ECS
        let mut world = World::new();
        world.insert_resource(TimeState::default());
        world.insert_resource(CameraState::default());
        world.insert_resource(RenderState::default());
        world.insert_resource(InputState::default());

        // Build solar system
        let scene = SceneGraph::build(
            cosmogon_scene::create_solar_system(),
            &mut world,
        );

        // Build schedule with system ordering
        let schedule = Schedule::default()
            .with_system_set(PhysicsSet::configure())
            .with_system_set(RenderSet::configure())
            .with_system_set(UISet::configure())
            .add_systems(PhysicsSet, (
                time_management,
                gravity_calculation,
                orbital_propagation,
                integration,
                collision_detection,
            ).chain())
            .add_systems(RenderSet, (
                prepare_uniforms,
                generate_meshes,
                record_render_passes,
                post_processing,
                render_ui,
            ).chain())
            .add_systems(UISet, (
                handle_input,
                draw_panels,
                draw_debug_overlay,
            ).chain());

        Self {
            window,
            render_context,
            ecs_world: world,
            ecs_schedule: schedule,
            ui_state: UIPanels::new(),
            input_state: InputState::default(),
        }
    }

    pub fn run(self) {
        let mut app = self;
        event_loop.run(move |event, _, control_flow| {
            match event {
                Event::WindowEvent { event, .. } => {
                    match event {
                        WindowEvent::CloseRequested => {
                            control_flow.set_exit();
                        }
                        WindowEvent::Resized(size) => {
                            app.render_context.resize(size);
                        }
                        WindowEvent::KeyboardInput { input, .. } => {
                            app.input_state.update_keyboard(input);
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            app.input_state.update_mouse(button, state);
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            app.input_state.update_scroll(delta);
                        }
                        _ => {}
                    }
                }
                Event::AboutToWait => {
                    app.update();
                    app.window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    app.render();
                }
                _ => {}
            }
        }).unwrap();
    }

    fn update(&mut self) {
        // 1. Advance time
        self.ecs_world.resource_mut::<TimeState>().advance(
            self.render_context.frame_delta()
        );

        // 2. Run ECS schedule (physics + other systems)
        self.ecs_schedule.run(&mut self.ecs_world);

        // 3. Update camera
        update_camera(&mut self.ecs_world);

        // 4. Prepare UI state
        self.ui_state.debug_overlay.fps = self.render_context.fps();
        self.ui_state.debug_overlay.entity_count = self.ecs_world.entities().len();
    }

    fn render(&mut self) {
        let output = self.render_context.surface.get_current_texture().unwrap();
        let view = output.texture.create_view(&Default::default());

        let mut encoder = self.render_context.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Main Encoder"),
            }
        );

        // 1. Opaque pass
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Opaque Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.render_context.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            // Draw all visible objects
            self.draw_opaque_pass(&mut pass);
        }

        // 2. Atmosphere pass
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Atmosphere Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.render_context.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });

            self.draw_atmosphere_pass(&mut pass);
        }

        // 3. UI pass (via egui)
        self.ui_state.draw(&mut encoder, &view, &self.ecs_world);

        // 4. Submit and present
        self.render_context.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
```

---

## 3. Rendering Architecture

### 3.1 Device Initialization and Surface Configuration

The rendering pipeline starts with wgpu device initialization. We request a high-performance adapter (discrete GPU preferred), create a logical device and queue, and configure the surface for the window.

**Key decisions:**
- **Surface format:** Prefer `Bgra8UnormSrgb` for sRGB output, fall back to first available
- **Present mode:** `AutoVsync` for smooth display, option for `Mailbox` for benchmarking
- **MSAA:** Configurable, default 4x for quality, 1x for performance
- **HDR:** Optional HDR render target for bloom pipeline

### 3.2 Render Pass Structure

```
┌─────────────────────────────────────────────────────────┐
│                    FRAME RENDERING                       │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  1. OPAQUE PASS (depth-tested)                          │
│     ├── Clear: color=BLACK, depth=1.0                   │
│     ├── Draw: planet meshes (UV sphere)                 │
│     ├── Draw: moon meshes (UV sphere)                   │
│     ├── Draw: asteroid meshes (low-poly)                │
│     └── Uses: depth buffer, camera + light uniforms     │
│                                                          │
│  2. ATMOSPHERE PASS (additive, depth read-only)         │
│     ├── Clear: none (load previous)                     │
│     ├── Draw: atmosphere shells for planets             │
│     ├── Blend: additive (SRC_ALPHA, ONE)                │
│     └── Uses: depth buffer (read-only), camera uniforms │
│                                                          │
│  3. STAR PASS (billboard, additive)                     │
│     ├── Clear: none                                     │
│     ├── Draw: star billboards with point sprites        │
│     ├── Blend: additive                                 │
│     └── Uses: camera uniforms, spectral LUT             │
│                                                          │
│  4. POST-PROCESSING                                     │
│     ├── Downsample HDR → mip chain                      │
│     ├── Gaussian blur (horizontal + vertical)           │
│     ├── Upsample + composite                            │
│     └── Tone mapping (ACES filmic)                      │
│                                                          │
│  5. UI PASS                                             │
│     ├── egui panels (inspector, time, camera, debug)    │
│     ├── Rendered to swapchain directly                  │
│     └── Uses: egui-wgpu integration                     │
│                                                          │
│  6. PRESENT                                             │
│     └── Submit command buffer → present to surface       │
│                                                          │
└─────────────────────────────────────────────────────────┘
```

### 3.3 Uniform Buffer Management

**Per-frame updates (every frame):**
- Camera uniforms: view/projection matrices, position, time
- Light uniforms: sun position/color

**Per-object updates (when object changes):**
- Object uniforms: model matrix, color, emissive, roughness

**Strategy:** Double-buffered uniform buffers. Write to back buffer this frame, swap on next frame. Prevents GPU stalls when updating uniforms mid-frame.

```rust
struct DoubleBufferedUniform<T: bytemuck::Pod> {
    buffers: [wgpu::Buffer; 2],
    current: usize,
    bind_group: wgpu::BindGroup,
}

impl<T: bytemuck::Pod> DoubleBufferedUniform<T> {
    fn update(&mut self, data: &T, queue: &wgpu::Queue) {
        self.current ^= 1;
        queue.write_buffer(&self.buffers[self.current], 0, bytemuck::bytes_of(data));
    }
}
```

### 3.4 Mesh Generation Strategies

| Object Type | Mesh Strategy | Resolution | Notes |
|------------|---------------|------------|-------|
| Planets | UV Sphere | 64×32 | Smooth for close viewing |
| Moons | UV Sphere | 32×16 | Lower detail |
| Stars | Billboard quad | 4 verts | Always faces camera |
| Asteroids | Icosphere | Subdiv 2 | Low-poly for performance |
| Rings | Instanced quads | 1000+ | Particle-like rendering |
| Atmosphere | Shell mesh | 32×16 | Slightly larger than planet |

### 3.5 HDR + Tone Mapping Pipeline

```
HDR Rendering:
  1. Render scene to Rgba16Float texture (HDR)
  2. Apply bloom (see below)
  3. Tone map HDR → LDR using ACES filmic curve

ACES Tone Mapping (simplified):
  color = (color * (2.51 * color + 0.03)) / (color * (2.43 * color + 0.59) + 0.14)
```

### 3.6 Bloom Implementation

```
Bloom Pipeline:
───────────────

Input: HDR render target (Rgba16Float)

1. DOWNSAMPLE (mip chain)
   ├── Full res → 1/2 → 1/4 → 1/8 → 1/16
   └── Each level: box filter downsample

2. GAUSSIAN BLUR (per mip level)
   ├── Horizontal pass (compute shader)
   ├── Vertical pass (compute shader)
   └── Kernel size: 13 taps (adjustable)

3. UPSAMPLE + COMPOSITE
   ├── Start from smallest mip
   ├── Upsample + add to next level
   ├── Repeat until full resolution
   └── Composite: scene + bloom * intensity

Parameters:
  bloom_threshold: 1.0    (HDR values above this bloom)
  bloom_intensity: 0.3    (strength of bloom effect)
  bloom_filter_size: 7    (blur kernel radius)
```

### 3.7 Atmospheric Scattering

The atmosphere shader implements Rayleigh + Mie scattering for realistic sky rendering:

```wgsl
// Simplified atmosphere fragment shader
fn fragment_shader(in: VertexOutput) -> vec4<f32> {
    let ray_origin = camera_position;
    let ray_dir = normalize(in.world_position - camera_position);

    // Sphere intersection for atmosphere
    let atmosphere_radius = planet_radius + atmosphere_height;
    let hit = ray_sphere_intersect(ray_origin, ray_dir, planet_center, atmosphere_radius);

    if hit.t < 0.0 {
        discard;  // Ray misses atmosphere
    }

    // March through atmosphere
    let num_samples = 32;
    let step_size = hit.t / f32(num_samples);

    var optical_depth_rayleigh = 0.0;
    var optical_depth_mie = 0.0;
    var scattered_light = vec3(0.0);

    for (var i = 0; i < num_samples; i++) {
        let sample_pos = ray_origin + ray_dir * (hit.t * (f32(i) + 0.5) / f32(num_samples));
        let height = length(sample_pos - planet_center) - planet_radius;

        // Rayleigh scattering coefficient
        let rayleigh_coeff = rayleigh_scattering * exp(-height / rayleigh_scale);
        // Mie scattering coefficient
        let mie_coeff = mie_scattering * exp(-height / mie_scale);

        optical_depth_rayleigh += rayleigh_coeff * step_size;
        optical_depth_mie += mie_coeff * step_size;

        // In-scattering (sun light scattered toward camera)
        let sun_dir = normalize(sun_position - sample_pos);
        let rayleigh_phase = rayleigh_phase_function(dot(ray_dir, sun_dir));
        let mie_phase = mie_phase_function(dot(ray_dir, sun_dir), mie_g);

        scattered_light += sun_color * (rayleigh_coeff * rayleigh_phase + mie_coeff * mie_phase) * step_size;
    }

    // Transmittance (light lost through atmosphere)
    let transmittance = exp(-(rayleigh_absorption * optical_depth_rayleigh + mie_absorption * optical_depth_mie));

    return vec4<f32>(scattered_light * transmittance, 1.0 - transmittance.x);
}
```

### 3.8 Star Rendering

Stars are rendered as billboard quads with a spectral color lookup:

```wgsl
// Star fragment shader
fn fragment_shader(in: VertexOutput) -> vec4<f32> {
    // Distance from center (for point sprite)
    let dist = length(in.uv - vec2(0.5)) * 2.0;

    // Star profile (Gaussian-like falloff)
    let brightness = exp(-dist * dist * 8.0);

    // Spectral color from temperature
    let color = temperature_to_color(star_temperature);

    // Bloom effect (soft glow)
    let glow = exp(-dist * dist * 2.0) * 0.5;

    return vec4<f32>(color * (brightness + glow), 1.0);
}

// Temperature → RGB (simplified blackbody)
fn temperature_to_color(temp: f32) -> vec3<f32> {
    // Attempt to approximate Planckian locus
    let t = temp / 100.0;
    var r: f32;
    var g: f32;
    var b: f32;

    if (t <= 66.0) {
        r = 1.0;
        g = 99.4708025861 * log(t) - 161.1195681661;
        if (t <= 19.0) {
            b = 0.0;
        } else {
            b = 138.5177312231 * log(t - 10.0) - 305.0447927307;
        }
    } else {
        r = 329.698727446 * pow(t - 60.0, -0.1332047592);
        g = 288.1221695283 * pow(t - 60.0, -0.0755148492);
        b = 1.0;
    }

    return vec3<f32>(clamp(r / 255.0, 0.0, 1.0), clamp(g / 255.0, 0.0, 1.0), clamp(b / 255.0, 0.0, 1.0));
}
```

### 3.9 Depth Buffer Management

- **Format:** `Depth32Float` — 32-bit float depth
- **Usage:** `RENDER_ATTACHMENT | TEXTURE_BINDING` — for rendering and reading in atmosphere pass
- **Clear value:** 1.0 (far plane)
- **Comparison:** `Less` for opaque pass, `Always` for atmosphere pass (read-only)

### 3.10 Frame Timing

```rust
pub struct FrameTiming {
    pub last_frame_time: Instant,
    pub delta_time: f32,
    pub frame_count: u64,
    pub fps: f32,
    pub fps_smooth: f32,  // smoothed FPS for display
}

impl FrameTiming {
    pub fn update(&mut self) {
        let now = Instant::now();
        self.delta_time = (now - self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        self.frame_count += 1;

        // Exponential moving average for FPS display
        let instant_fps = 1.0 / self.delta_time;
        self.fps = instant_fps;
        self.fps_smooth = self.fps_smooth * 0.95 + instant_fps * 0.05;
    }
}
```

---

## 4. Physics Architecture

### 4.1 Gravitational N-Body Simulation

#### 4.1.1 Direct Summation (Small N)

For N < 10,000 bodies, we use direct O(N²) pairwise force calculation:

```rust
/// Compute gravitational force between all pairs
/// F = G * m1 * m2 / r²
/// a1 = F / m1, a2 = F / m2
fn direct_summation(
    positions: &[Vec3d],
    masses: &[f64],
    accelerations: &mut [Vec3d],
    softening: f64,
) {
    let n = positions.len();
    accelerations.fill(Vec3d::ZERO);

    for i in 0..n {
        for j in (i + 1)..n {
            let r_vec = positions[j] - positions[i];
            let dist_sq = r_vec.length_squared() + softening * softening;
            let inv_dist = 1.0 / dist_sq.sqrt();
            let inv_dist3 = inv_dist * inv_dist * inv_dist;

            let force = constants::G * masses[i] * masses[j] * inv_dist3;

            accelerations[i] += force * r_vec / masses[i];
            accelerations[j] -= force * r_vec / masses[j];
        }
    }
}
```

#### 4.1.2 Barnes-Hut Tree (Large N, Future)

For N > 10,000, we'll implement Barnes-Hut with O(N log N) complexity:

```
Barnes-Hut Algorithm:
1. Build octree from all particles
2. For each particle, traverse tree:
   - If node is far enough (s/d < θ), treat as single body
   - Otherwise, recurse into children
3. θ (opening angle) controls accuracy vs speed tradeoff
```

#### 4.1.3 Softening Parameter

The softening length ε prevents the force from diverging as r → 0:

```
F = G * m1 * m2 / (r² + ε²)
```

Typical value: ε = 1e-6 AU (roughly 150 km). This prevents numerical singularities during close encounters while being small enough not to affect large-scale dynamics.

### 4.2 Keplerian Orbital Mechanics

#### 4.2.1 Orbital Elements

| Element | Symbol | Description |
|---------|--------|-------------|
| Semi-major axis | a | Average radius of orbit (AU) |
| Eccentricity | e | Shape of orbit (0=circle, 0<e<1=ellipse) |
| Inclination | i | Tilt relative to ecliptic (radians) |
| Longitude of ascending node | Ω | Where orbit crosses ecliptic going north |
| Argument of perihelion | ω | Angle from ascending node to perihelion |
| Mean anomaly at epoch | M | Starting position in orbit |

#### 4.2.2 Mean → Eccentric → True Anomaly

```
Kepler's Equation: M = E - e·sin(E)

Solve iteratively (Newton-Raphson):
  E₀ = M
  Eₙ₊₁ = Eₙ - (Eₙ - e·sin(Eₙ) - M) / (1 - e·cos(Eₙ))

True anomaly from eccentric anomaly:
  tan(ν/2) = √((1+e)/(1-e)) · tan(E/2)
```

#### 4.2.3 Position and Velocity

```
Position in orbital plane:
  x = a·(cos(E) - e)
  y = a·√(1-e²)·sin(E)

Rotate by (ω, i, Ω) to get ecliptic coordinates:
  [x']   [cos Ω·cos ω - sin Ω·sin ω·cos i   -cos Ω·sin ω - sin Ω·cos ω·cos i   sin Ω·sin i]   [x]
  [y'] = [sin Ω·cos ω + cos Ω·sin ω·cos i   -sin Ω·sin ω + cos Ω·cos ω·cos i  -cos Ω·sin i] · [y]
  [z']   [sin ω·sin i                          cos ω·sin i                          cos i      ]   [0]
```

### 4.3 Numerical Integrators

#### 4.3.1 RK4 (Primary)

- **Order:** 4th order (error ∝ dt⁵)
- **Type:** Non-symplectic (can drift energy)
- **Use case:** General-purpose, good accuracy
- **Cost:** 4 force evaluations per step

```
k1 = f(t, y)
k2 = f(t + dt/2, y + dt/2 · k1)
k3 = f(t + dt/2, y + dt/2 · k2)
k4 = f(t + dt, y + dt · k3)

y_new = y + dt/6 · (k1 + 2·k2 + 2·k3 + k4)
```

#### 4.3.2 Velocity Verlet (Symplectic)

- **Order:** 2nd order (error ∝ dt³)
- **Type:** Symplectic (energy-preserving over long times)
- **Use case:** Long-term orbital stability
- **Cost:** 2 force evaluations per step

```
x_new = x + v·dt + a·dt²/2
a_new = F(x_new) / m
v_new = v + (a + a_new)/2 · dt
```

#### 4.3.3 Adaptive Step Size Control

```rust
pub struct AdaptiveStepController {
    pub min_dt: f64,
    pub max_dt: f64,
    pub target_error: f64,
    pub safety_factor: f64,  // typically 0.9
}

impl AdaptiveStepController {
    pub fn compute_dt(&self, error: f64, current_dt: f64) -> f64 {
        if error == 0.0 {
            return current_dt * 2.0;  // double step if no error
        }

        let scale = self.safety_factor * (self.target_error / error).powf(0.2);
        let new_dt = current_dt * scale.clamp(0.5, 2.0);

        new_dt.clamp(self.min_dt, self.max_dt)
    }
}
```

### 4.4 Time Management

```rust
impl TimeState {
    /// Fixed timestep accumulator pattern
    pub fn advance(&mut self, frame_delta: f64) {
        if self.paused {
            return;
        }

        let scaled_dt = frame_delta * self.time_scale;
        self.accumulator += scaled_dt;

        // Fixed physics steps
        while self.accumulator >= self.physics_dt {
            self.current_time += self.physics_dt / constants::JULIAN_YEAR;
            self.accumulator -= self.physics_dt;
        }
    }

    /// Interpolation factor for smooth rendering between physics steps
    pub fn interpolation_factor(&self) -> f64 {
        self.accumulator / self.physics_dt
    }

    /// Set time scale (1.0 = real-time, 365.25 = 1 year/sec)
    pub fn set_time_scale(&mut self, scale: f64) {
        self.time_scale = scale.clamp(0.001, 10000.0);
    }
}
```

**Time scales in Cosmogon:**
- `time_scale = 1.0`: Real-time (1 second = 1 second)
- `time_scale = 3600.0`: 1 hour per second
- `time_scale = 86400.0`: 1 day per second
- `time_scale = 31557600.0`: 1 year per second

---

## 5. ECS Architecture

### 5.1 Component Definitions

All components use `#[derive(Component)]` from bevy_ecs. They're kept simple and data-oriented — no logic in components, only data.

**Core components (Position, Velocity, Mass, Radius):** Every celestial body has these.

**Optional components (OrbitalElements, Rotation, VisualProperties):** Added only when relevant.

**Marker components (Star, Planet, Moon):** For quick filtering in queries.

### 5.2 System Sets and Ordering

We use bevy_ecs `SystemSet` to enforce ordering between system groups:

```
PhysicsSet::TimeManagement
    → PhysicsSet::GravityCalculation
        → PhysicsSet::OrbitalPropagation
            → PhysicsSet::Integration
                → PhysicsSet::CollisionDetection

RenderSet::UniformPreparation
    → RenderSet::MeshGeneration
        → RenderSet::RenderPassRecording
            → RenderSet::PostProcessing
                → RenderSet::UIRendering

UISet::InputHandling
    → UISet::PanelDrawing
        → UISet::DebugOverlay
```

**Cross-set ordering:**
- `PhysicsSet` runs before `RenderSet` (physics updates positions before rendering)
- `UISet` runs after `RenderSet` (UI overlays on top of rendered scene)

### 5.3 Query Patterns

```rust
// All bodies with positions (for physics)
fn physics_query(query: Query<(&Position, &Velocity, &Mass)>) {}

// All renderable bodies (for rendering)
fn render_query(query: Query<(&Position, &Radius, &Renderable, &VisualProperties)>) {}

// Bodies with orbital elements (for Kepler propagation)
fn orbital_query(query: Query<(&mut Position, &mut Velocity, &OrbitalElements)>) {}

// Camera entity
fn camera_query(query: Query<(&Camera, &Position), With<ActiveCamera>>) {}

// Selected entity (for inspector)
fn selected_query(query: Query<Entity, With<Selected>>) {}
```

### 5.4 Resource Definitions

**TimeState:** Global simulation time, time scale, physics timestep.

**CameraState:** View/projection matrices, camera position, target entity.

**RenderState:** Frame count, resolution, feature flags.

**InputState:** Keyboard/mouse state from winit events.

### 5.5 Physics → Render Data Flow

```
Physics Systems:
  1. time_management: advance TimeState
  2. gravity_calculation: compute accelerations
  3. orbital_propagation: update Keplerian orbits
  4. integration: apply accelerations to velocities/positions
  5. collision_detection: detect/closest approach

  ↓ (Position components updated)

Render Systems:
  1. prepare_uniforms: read Position → write ObjectUniforms
  2. generate_meshes: create/update GPU meshes
  3. record_render_passes: draw calls with uniforms
  4. post_processing: bloom, tone mapping
  5. render_ui: egui overlay
```

### 5.6 UI → ECS Interaction

```
UI Systems:
  1. handle_input: read InputState → update Camera components
  2. draw_panels: read component data → draw egui widgets
  3. draw_debug_overlay: read resources → display stats

  ↓ (Camera component updated, Selected entity updated)

Physics/Render Systems:
  - Read updated Camera component
  - Read Selected entity for highlighting
```

---

## 6. Coordinate Systems

### 6.1 Heliocentric Ecliptic Coordinates (Primary)

The **primary reference frame** for Cosmogon is heliocentric ecliptic:
- **Origin:** Sun's center
- **X-axis:** Vernal equinox direction
- **Z-axis:** North ecliptic pole
- **Y-axis:** Completes right-handed system

**Units:** AU for distance, years for time, solar masses for mass.

All positions and velocities are stored in this frame. Conversions to other frames happen at the boundaries (rendering, UI display).

### 6.2 Equatorial Coordinates

Used for:
- Planet rotation (axial tilt relative to ecliptic)
- Star positions (right ascension / declination)
- Camera orientation

**Conversion from ecliptic:**
```
Obliquity of ecliptic: ε = 23.44°

[x_eq]   [1    0       0   ]   [x_ecl]
[y_eq] = [0    cos ε   sin ε] · [y_ecl]
[z_eq]   [0   -sin ε   cos ε]   [z_ecl]
```

### 6.3 Body-Local Coordinates

Used for:
- Surface rendering (texture mapping)
- Terrain generation (future)
- Atmospheric effects

**Origin:** Body center  
**Y-axis:** Rotation axis  
**X-axis:** Points toward vernal equinox in body frame  
**Z-axis:** Completes right-handed system

### 6.4 Floating-Origin System

The universe is **big**. Jupiter is 5.2 AU from the Sun. The nearest star is 270,000 AU away. Naive f64 → f32 conversion would lose precision for distant objects.

**Solution:** Floating origin relative to camera.

```rust
pub struct FloatingOrigin {
    /// World-space origin offset (f64)
    pub origin: Vec3d,
    /// Render-space positions (f32, relative to origin)
    pub render_offsets: HashMap<Entity, Vec3f>,
}

impl FloatingOrigin {
    pub fn update(&mut self, camera_pos: Vec3d) {
        // Snap origin to camera (with dead zone)
        let diff = camera_pos - self.origin;
        if diff.length() > 100.0 { // AU threshold
            self.origin = camera_pos;
        }

        // Compute f32 offsets for all objects
        for (entity, pos) in self.positions.iter() {
            let offset = *pos - self.origin;
            self.render_offsets.insert(*entity, offset.as_vec3());
        }
    }
}
```

### 6.5 f64 → f32 Conversion

**The precision problem:**
- f64 has ~15 significant digits
- f32 has ~7 significant digits
- If position is 100 AU and we need mm precision: 100 AU = 1.5e13 m, mm = 1e-3 m → need 16 digits → f64 required

**Solution:** Store in f64, convert to f32 relative to camera:

```rust
impl Position {
    /// Convert to render-space f32, relative to floating origin
    pub fn to_render_space(&self, origin: Vec3d) -> Vec3f {
        let offset = self.coords - origin;
        // This is safe because offset is typically < 100 AU
        // which fits comfortably in f32 precision
        Vec3f::new(offset.x as f32, offset.y as f32, offset.z as f32)
    }
}
```

### 6.6 Coordinate System Summary

| Frame | Precision | Units | Used For |
|-------|-----------|-------|----------|
| Heliocentric Ecliptic | f64 | AU, yr | Physics, ECS |
| Equatorial | f64 | AU, yr | Rotation, stars |
| Body-Local | f64 | m | Surface rendering |
| Floating Origin | f64→f32 | AU→m | GPU rendering |
| Screen | f32 | pixels | UI |

---

## 7. Threading Model

### 7.1 Thread Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    THREAD ARCHITECTURE                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────┐                                       │
│  │   MAIN THREAD    │ ← winit event loop, input handling    │
│  │                  │ ← UI updates (egui)                   │
│  │                  │ ← ECS schedule orchestration          │
│  └────────┬─────────┘                                       │
│           │                                                  │
│           │ spawn async tasks                               │
│           │                                                  │
│  ┌────────▼─────────┐     ┌──────────────────┐             │
│  │  RENDER THREAD   │     │  PHYSICS THREAD  │             │
│  │  (async wgpu)    │     │  (rayon pool)    │             │
│  │                  │     │                  │             │
│  │  - Device ops    │     │  - N-body sim    │             │
│  │  - Encoder       │     │  - Integration   │             │
│  │  - Submit        │     │  - Collision     │             │
│  └──────────────────┘     └──────────────────┘             │
│                                                              │
│  ┌──────────────────┐                                       │
│  │  ASSET THREAD    │ ← async file I/O                     │
│  │  (tokio async)   │ ← Texture loading                    │
│  │                  │ ← Shader compilation                  │
│  └──────────────────┘                                       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 7.2 Main Thread

- **winit event loop:** Receives window events, dispatches to handlers
- **Input handling:** Processes keyboard/mouse/scroll events
- **UI updates:** Runs egui logic, draws panels
- **System orchestration:** Runs ECS schedule, coordinates other threads

### 7.3 Render Thread (Async)

wgpu operations are async. We spawn a dedicated task for GPU work:

```rust
async fn render_frame(
    render_context: Arc<Mutex<RenderContext>>,
    ecs_world: Arc<Mutex<World>>,
) {
    let ctx = render_context.lock().await;
    let world = ecs_world.lock().await;

    // Build command encoder
    let mut encoder = ctx.device.create_command_encoder(...);

    // Record render passes
    record_opaque_pass(&mut encoder, &ctx, &world);
    record_atmosphere_pass(&mut encoder, &ctx, &world);
    record_post_processing(&mut encoder, &ctx);
    record_ui_pass(&mut encoder, &ctx);

    // Submit
    ctx.queue.submit(std::iter::once(encoder.finish()));

    // Present
    output.present();
}
```

### 7.4 Physics Thread (rayon)

N-body simulation is embarrassingly parallel. We use rayon's thread pool:

```rust
use rayon::prelude::*;

fn parallel_gravity(
    positions: &[Vec3d],
    masses: &[f64],
    softening: f64,
) -> Vec<Vec3d> {
    let n = positions.len();
    let mut accelerations = vec![Vec3d::ZERO; n];

    // Parallel force calculation
    accelerations.par_iter_mut().enumerate().for_each(|(i, acc)| {
        for j in 0..n {
            if i == j { continue; }
            let r = positions[j] - positions[i];
            let dist_sq = r.length_squared() + softening * softening;
            let force = constants::G * masses[i] * masses[j] / dist_sq;
            *acc += force * r / (dist_sq.sqrt() * masses[i]);
        }
    });

    accelerations
}
```

### 7.5 System Ordering and Synchronization

```
Frame Timeline:
───────────────

t=0: Input poll (main thread)
t=1: Time step calculation (main thread)
t=2: Physics update (rayon thread pool)
     ├── Gravity calculation (parallel)
     ├── Orbital propagation (sequential per body)
     └── Integration (parallel)
t=3: ECS flush (main thread, wait for physics)
t=4: Camera update (main thread)
t=5: Uniform buffer writes (main thread)
t=6: Render pass recording (async render task)
t=7: UI render (async render task)
t=8: Submit (async render task)
t=9: Present (async render task)
```

**Synchronization points:**
- Physics → ECS: Barrier (wait for all physics tasks to complete)
- ECS → Render: Mutex lock on World
- Render → Present: async await on GPU completion

---

## 8. Resource Management

### 8.1 GPU Buffer Pool

```rust
pub struct BufferPool {
    /// Available vertex buffers
    vertex_buffers: Vec<wgpu::Buffer>,
    /// Available index buffers
    index_buffers: Vec<wgpu::Buffer>,
    /// Available uniform buffers
    uniform_buffers: Vec<wgpu::Buffer>,
    /// Total allocated bytes
    total_allocated: usize,
}

impl BufferPool {
    pub fn acquire_vertex_buffer(&mut self, device: &wgpu::Device, size: usize) -> wgpu::Buffer {
        self.vertex_buffers.pop().unwrap_or_else(|| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Pooled Vertex Buffer"),
                size: size as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        })
    }

    pub fn release(&mut self, buffer: wgpu::Buffer) {
        self.total_allocated -= buffer.size() as usize;
        // Return to pool for reuse
        self.vertex_buffers.push(buffer);
    }
}
```

### 8.2 Mesh Caching

```rust
pub struct MeshCache {
    /// Cache key: (mesh type, resolution)
    meshes: HashMap<MeshKey, wgpu::Buffer>,
    /// GPU memory usage
    memory_usage: usize,
}

#[derive(Hash, Eq, PartialEq)]
enum MeshKey {
    UvSphere { sectors: u32, stacks: u32 },
    Icosphere { subdivisions: u32 },
    Billboard,
    Custom(String),
}

impl MeshCache {
    pub fn get_or_create(
        &mut self,
        key: MeshKey,
        device: &wgpu::Device,
    ) -> &wgpu::Buffer {
        self.meshes.entry(key.clone()).or_insert_with(|| {
            let (vertices, indices) = match &key {
                MeshKey::UvSphere { sectors, stacks } => {
                    uv_sphere(1.0, *sectors, *stacks)
                }
                MeshKey::Billboard => billboard_quad(1.0),
                _ => unimplemented!(),
            };

            let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Mesh Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertices),
                usage: BufferUsages::VERTEX,
            });

            self.memory_usage += vertex_buffer.size() as usize;
            vertex_buffer
        })
    }
}
```

### 8.3 Texture Management

```rust
pub struct TextureManager {
    /// Named textures (e.g., "earth_day", "earth_night")
    textures: HashMap<String, wgpu::Texture>,
    /// Sampler cache
    samplers: HashMap<SamplerKey, wgpu::Sampler>,
}

impl TextureManager {
    pub async fn load_texture(
        &mut self,
        name: &str,
        path: &Path,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bytes = tokio::fs::read(path).await?;
        let image = image::load_from_memory(&bytes)?;
        let rgba = image.to_rgba8();

        let size = Extent3d {
            width: rgba.width(),
            height: rgba.height(),
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&TextureDescriptor {
            label: Some(name),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: ImageAspect::All,
            },
            &rgba,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * rgba.width()),
                rows_per_image: Some(rgba.height()),
            },
            size,
        );

        self.textures.insert(name.to_string(), texture);
        Ok(())
    }
}
```

### 8.4 Uniform Buffer Updates Per Frame

**Strategy:** Write-once-per-frame for camera, write-on-change for objects.

```rust
pub fn prepare_uniforms(
    camera_query: Query<(&Camera, &Position)>,
    object_query: Query<(Entity, &Position, &Radius, &Renderable, &VisualProperties)>,
    time_state: Res<TimeState>,
    mut camera_uniforms: ResMut<CameraUniforms>,
    mut object_uniforms: ResMut<ObjectUniforms>,
) {
    // Camera uniforms (every frame)
    for (camera, pos) in camera_query.iter() {
        let view = Mat4d::look_at(pos.coords, camera.target_pos, Vec3d::Y);
        let projection = Mat4d::perspective_rh(
            camera.fov.to_radians() as f64,
            aspect_ratio,
            camera.near,
            camera.far,
        );

        camera_uniforms.update(&CameraUniforms {
            view: view.to_cols_array(),
            projection: projection.to_cols_array(),
            view_position: [pos.coords.x as f32, pos.coords.y as f32, pos.coords.z as f32, 1.0],
            time: time_state.current_time as f32,
            ..default()
        });
    }

    // Object uniforms (when changed)
    for (entity, pos, radius, renderable, visual) in object_query.iter() {
        if uniforms_changed(entity) {
            let model = Mat4f::from_translation(pos.to_render_space(origin))
                * Mat4f::from_scale(Vec3f::splat(radius.0 as f32));

            object_uniforms.update(entity, &ObjectUniforms {
                model: model.to_cols_array(),
                base_color: [visual.base_color[0], visual.base_color[1], visual.base_color[2], 1.0],
                emissive: [0.0, 0.0, 0.0, visual.emissive_strength],
                roughness_metallic: [visual.roughness, 0.0, 0.0, 0.0],
                flags: match renderable {
                    Renderable::Planet { has_atmosphere } => if *has_atmosphere { 1 } else { 0 },
                    Renderable::Star { .. } => 2,
                    _ => 0,
                },
                ..default()
            });
        }
    }
}
```

### 8.5 Memory Budget

| Resource | Budget | Notes |
|----------|--------|-------|
| Vertex buffers | 512 MB | UV spheres, billboards |
| Index buffers | 256 MB | Index data |
| Uniform buffers | 64 MB | Camera + objects |
| Textures | 1 GB | Planet textures (if any) |
| Depth buffer | 32 MB | 4K depth |
| Total GPU | ~2 GB | Conservative estimate |

**CPU memory:**
| Resource | Budget | Notes |
|----------|--------|-------|
| ECS World | 128 MB | Components, entities |
| Physics state | 64 MB | Positions, velocities, accelerations |
| Scene data | 16 MB | Orbital elements, constants |
| Total CPU | ~208 MB | For 100,000 bodies |

---

## 9. Module Dependencies

### 9.1 ASCII Dependency Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                    COSMOGON DEPENDENCY GRAPH                     │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│                        ┌──────────────┐                         │
│                        │ cosmogon_app │                         │
│                        │  (top-level) │                         │
│                        └──────┬───────┘                         │
│                               │                                  │
│              ┌────────────────┼────────────────┐                │
│              │                │                │                │
│              ▼                ▼                ▼                │
│     ┌──────────────┐ ┌──────────────┐ ┌──────────────┐         │
│     │ cosmogon_ui  │ │cosmogon_scene│ │cosmogon_render│        │
│     │   (egui)     │ │ (solar sys)  │ │   (wgpu)     │        │
│     └──────┬───────┘ └──────┬───────┘ └──────┬───────┘         │
│            │                │                │                  │
│            │                ▼                │                  │
│            │         ┌──────────────┐        │                  │
│            └────────►│cosmogon_ecs  │◄───────┘                  │
│                      │  (components │                           │
│                      │   + systems) │                           │
│                      └──────┬───────┘                           │
│                             │                                   │
│                             ▼                                   │
│                      ┌──────────────┐                           │
│                      │cosmogon_core │                           │
│                      │   (math,     │                           │
│                      │  constants)  │                           │
│                      └──────────────┘                           │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ DEPENDENCY RULES:                                         │   │
│  │                                                           │   │
│  │ 1. cosmogon_core has NO internal dependencies             │   │
│  │ 2. cosmogon_ecs depends ONLY on cosmogon_core             │   │
│  │ 3. cosmogon_physics depends on core + ecs                 │   │
│  │ 4. cosmogon_render depends on core + ecs                  │   │
│  │ 5. cosmogon_scene depends on core + ecs                   │   │
│  │ 6. cosmogon_ui depends on core + ecs                      │   │
│  │ 7. cosmogon_app depends on ALL crates (orchestrator)      │   │
│  │                                                           │   │
│  │ No circular dependencies. No diamond dependencies.        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 9.2 Dependency Matrix

| Crate | core | ecs | physics | render | scene | ui | app |
|-------|------|-----|---------|--------|-------|----|-----|
| **core** | — | — | — | — | — | — | — |
| **ecs** | ✓ | — | — | — | — | — | — |
| **physics** | ✓ | ✓ | — | — | — | — | — |
| **render** | ✓ | ✓ | — | — | — | — | — |
| **scene** | ✓ | ✓ | — | — | — | — | — |
| **ui** | ✓ | ✓ | — | — | — | — | — |
| **app** | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | — |

### 9.3 Why This Dependency Structure?

**cosmogon_core is the foundation:**
- Math types and constants are used everywhere
- No ECS dependency means it can be used in tests, benchmarks, and tools independently

**cosmogon_ecs is the data layer:**
- Defines components and resources that all systems use
- Depends only on core for math types
- No rendering, physics, or UI dependencies

**The "sibling" crates (physics, render, scene, ui) are independent:**
- They all depend on core + ecs but NOT on each other
- Physics doesn't know about rendering
- Rendering doesn't know about physics
- They communicate through ECS components/resources

**cosmogon_app is the orchestrator:**
- Depends on everything because it wires systems together
- Creates the ECS schedule
- Manages the main loop

---

## 10. Data Flow

### 10.1 Frame Lifecycle

```
┌─────────────────────────────────────────────────────────────────┐
│                    FRAME LIFECYCLE                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. INPUT POLL                                                   │
│     ├── winit sends WindowEvent                                 │
│     ├── Keyboard events → InputState.keys_pressed               │
│     ├── Mouse events → InputState.mouse_delta                   │
│     └── Scroll events → InputState.scroll_delta                 │
│                                                                  │
│  2. TIME STEP CALCULATION                                        │
│     ├── Read frame_delta from winit                             │
│     ├── Scale by time_scale                                     │
│     ├── Add to accumulator                                      │
│     └── Compute number of physics steps                         │
│                                                                  │
│  3. PHYSICS UPDATE                                               │
│     ├── [rayon] Gravity calculation (parallel N-body)           │
│     ├── Keplerian orbital propagation                           │
│     ├── Numerical integration (RK4 or Verlet)                   │
│     └── Collision detection (future)                            │
│                                                                  │
│  4. ECS FLUSH                                                    │
│     ├── Apply all component mutations                           │
│     ├── Process entity spawning/despawning                      │
│     └── Sync ECS state                                          │
│                                                                  │
│  5. CAMERA UPDATE                                                │
│     ├── Read InputState → update Camera component               │
│     ├── Compute view/projection matrices                        │
│     ├── Update floating origin                                   │
│     └── Compute f32 render offsets                              │
│                                                                  │
│  6. UNIFORM BUFFER WRITES                                        │
│     ├── Camera uniforms → GPU buffer                            │
│     ├── Light uniforms → GPU buffer                             │
│     ├── Object uniforms → GPU buffer (per visible object)       │
│     └── Time uniforms → GPU buffer                              │
│                                                                  │
│  7. RENDER PASS RECORDING                                        │
│     ├── Begin command encoder                                   │
│     ├── Opaque pass: depth test, draw planets/stars             │
│     ├── Atmosphere pass: additive blend, scattering shader      │
│     └── Post-processing: bloom, tone mapping                    │
│                                                                  │
│  8. UI RENDER                                                    │
│     ├── egui begin frame                                        │
│     ├── Draw inspector panel                                    │
│     ├── Draw time control panel                                 │
│     ├── Draw camera control panel                               │
│     ├── Draw debug overlay                                      │
│     └── egui end frame → render to swapchain                    │
│                                                                  │
│  9. SUBMIT                                                       │
│     ├── Finalize command encoder                                │
│     ├── Submit to wgpu queue                                    │
│     └── GPU executes commands                                   │
│                                                                  │
│  10. PRESENT                                                     │
│      ├── Swap chain presents frame                              │
│      ├── VSync wait (if enabled)                                │
│      └── Next frame begins                                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 10.2 Data Flow Diagram

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Input   │────►│   Time   │────►│ Physics  │────►│   ECS    │
│  State   │     │  State   │     │          │     │  World   │
└──────────┘     └──────────┘     └──────────┘     └──────────┘
                                              │              │
                                              │              │
                                              ▼              ▼
                                     ┌──────────┐   ┌──────────┐
                                     │ Position │   │ Camera   │
                                     │ Velocity │   │ State    │
                                     │ Mass     │   │          │
                                     └──────────┘   └──────────┘
                                              │              │
                                              │              │
                                              ▼              ▼
                                     ┌──────────────────────────┐
                                     │   Uniform Buffer Writes  │
                                     │   (Camera + Objects)     │
                                     └──────────┬───────────────┘
                                                │
                                                ▼
                                     ┌──────────────────────────┐
                                     │   Render Pass Recording  │
                                     │   (wgpu encoder)         │
                                     └──────────┬───────────────┘
                                                │
                                                ▼
                                     ┌──────────────────────────┐
                                     │        Submit            │
                                     │   (queue.submit)         │
                                     └──────────┬───────────────┘
                                                │
                                                ▼
                                     ┌──────────────────────────┐
                                     │        Present           │
                                     │   (swapchain.present)    │
                                     └──────────────────────────┘
```

### 10.3 Per-Frame Data Volume

| Data | Size | Frequency | Direction |
|------|------|-----------|-----------|
| Input events | ~1 KB | Per event | winit → InputState |
| TimeState | 64 B | Per frame | TimeState → Physics |
| Position array | 8 N bytes | Per physics step | Physics → ECS |
| Velocity array | 8 N bytes | Per physics step | Physics → ECS |
| Camera uniforms | 128 B | Per frame | Camera → GPU |
| Object uniforms | 96 B × visible | Per frame | ECS → GPU |
| Light uniforms | 64 B | Per frame | Light → GPU |
| Vertex data | Variable | On mesh change | CPU → GPU |

---

## 11. Future Extensibility

### 11.1 Plugin System for New Physics Modules

```rust
pub trait PhysicsPlugin {
    fn name(&self) -> &str;
    fn systems(&self) -> Vec<SystemType>;
    fn register_resources(&self, world: &mut World);
}

pub struct GeneralRelativityPlugin;
impl PhysicsPlugin for GeneralRelativityPlugin {
    fn name(&self) -> &str { "general_relativity" }
    fn systems(&self) -> Vec<SystemType> {
        vec![
            SystemType::GravityCorrection,  // GR corrections to Newtonian gravity
        ]
    }
}

pub struct TidalForcePlugin;
impl PhysicsPlugin for TidalForcePlugin {
    fn name(&self) -> &str { "tidal_forces" }
    fn systems(&self) -> Vec<SystemType> {
        vec![
            SystemType::TidalCalculation,
            SystemType::TidalHeating,
        ]
    }
}

// App registers plugins:
app.add_physics_plugin(Box::new(GeneralRelativityPlugin));
app.add_physics_plugin(Box::new(TidalForcePlugin));
```

### 11.2 Shader Hot-Reloading

```rust
pub struct ShaderReloader {
    /// Watched shader files
    watchers: HashMap<PathBuf, SystemTime>,
    /// Compiled shader cache
    cache: HashMap<PathBuf, wgpu::ShaderModule>,
}

impl ShaderReloader {
    pub fn watch(&mut self, path: PathBuf) {
        let metadata = std::fs::metadata(&path).unwrap();
        self.watchers.insert(path, metadata.modified().unwrap());
    }

    pub fn check_for_changes(&mut self, device: &wgpu::Device) -> Vec<PathBuf> {
        let mut changed = Vec::new();

        for (path, last_modified) in &self.watchers {
            let metadata = std::fs::metadata(path).unwrap();
            if metadata.modified().unwrap() > *last_modified {
                // Recompile shader
                let source = std::fs::read_to_string(path).unwrap();
                let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some(path.to_str().unwrap()),
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                });

                self.cache.insert(path.clone(), module);
                self.watchers.insert(path.clone(), metadata.modified().unwrap());
                changed.push(path.clone());
            }
        }

        changed
    }
}
```

### 11.3 Scene File Format (RON)

```rust
// scene.ron — RON (Rusty Object Notation)
(
    name: "Solar System",
    bodies: [
        (
            name: "Earth",
            mass: 5.972e24,
            radius: 6.371e6,
            orbital: Some((
                semi_major_axis: 1.0,
                eccentricity: 0.0167,
                inclination: 0.0,
                longitude_ascending: -11.26,
                argument_perihelion: 114.21,
                mean_anomaly: 357.52,
            )),
            renderable: Planet(has_atmosphere: true),
            texture: "earth_day.png",
        ),
        // ... more bodies
    ],
    time_scale: 86400.0,  // 1 day per second
    camera: (
        target: "Earth",
        distance: 0.01,  // AU
        fov: 60.0,
    ),
)
```

### 11.4 Save/Load State

```rust
#[derive(Serialize, Deserialize)]
pub struct SimulationState {
    pub time: f64,
    pub bodies: Vec<BodyState>,
    pub camera: CameraSnapshot,
}

#[derive(Serialize, Deserialize)]
pub struct BodyState {
    pub name: String,
    pub position: [f64; 3],
    pub velocity: [f64; 3],
    pub mass: f64,
}

impl SimulationState {
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let file = File::create(path)?;
        let mut serializer = ron::Serializer::new(file, None)?;
        self.serialize(&mut serializer)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        let state = ron::de::from_reader(file)?;
        Ok(state)
    }
}
```

### 11.5 Network Sync (Future)

```rust
pub struct NetworkSyncPlugin {
    /// Server connection
    connection: Option<Connection>,
    /// State snapshots for interpolation
    snapshot_buffer: VecDeque<SimulationState>,
    /// Entity mapping (server → client)
    entity_map: HashMap<u64, Entity>,
}

impl NetworkSyncPlugin {
    /// Client-side prediction + server reconciliation
    pub fn sync_state(&mut self, world: &mut World) {
        if let Some(server_state) = self.connection.as_mut()
            .and_then(|c| c.receive_state())
        {
            // Interpolate between snapshots
            let interpolated = self.interpolate_snapshots(
                &self.snapshot_buffer,
                server_state.timestamp,
            );

            // Apply to local world (with entity mapping)
            for body in &interpolated.bodies {
                if let Some(&entity) = self.entity_map.get(&body.id) {
                    *world.get_mut::<Position>(entity).unwrap() = Position {
                        coords: Vec3d::from(body.position),
                    };
                }
            }
        }
    }
}
```

### 11.6 GPU Compute Pipeline for Particles

```rust
pub struct ParticleSystem {
    /// Particle buffer (GPU-side)
    buffer: wgpu::Buffer,
    /// Compute pipeline for particle update
    compute_pipeline: wgpu::ComputePipeline,
    /// Bind group
    bind_group: wgpu::BindGroup,
}

impl ParticleSystem {
    pub fn update(&self, encoder: &mut wgpu::CommandEncoder, dt: f32) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("Particle Update"),
        });

        pass.set_pipeline(&self.compute_pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.dispatch_workgroups(
            (PARTICLE_COUNT / 256) as u32,
            1,
            1,
        );
    }
}

// particle_update.wgsl
@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: SimParams;

struct Particle {
    position: vec3<f32>,
    velocity: vec3<f32>,
    life: f32,
    size: f32,
}

@compute @workgroup_size(256)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= params.particle_count) { return; }

    var p = particles[i];

    // Update position
    p.position += p.velocity * params.dt;

    // Apply gravity toward center
    let r = length(p.position);
    let grav = -params.gravity / (r * r + 1.0);
    p.velocity += normalize(-p.position) * grav * params.dt;

    // Decrease life
    p.life -= params.dt;

    // Reset if dead
    if (p.life <= 0.0) {
        p.position = vec3<f32>(0.0);
        p.velocity = (rand3(id) - 0.5) * params.initial_speed;
        p.life = params.max_life;
    }

    particles[i] = p;
}
```

---

## Appendix A: Glossary

| Term | Definition |
|------|------------|
| **AU** | Astronomical Unit — average Earth-Sun distance (~150M km) |
| **N-body** | Simulation of N objects interacting gravitationally |
| **Keplerian** | Based on Kepler's laws of orbital motion |
| **Symplectic** | Integration method that preserves energy over long times |
| **Softening** | Parameter to prevent force divergence at r=0 |
| **Barnes-Hut** | O(N log N) algorithm for gravitational N-body |
| **Ecliptic** | Plane of Earth's orbit around the Sun |
| **Epoch** | Reference time for orbital elements |
| **Julian Date** | Days since noon Universal Time, January 1, 4713 BC |

## Appendix B: Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Frame time | < 16.67 ms | 60 FPS minimum |
| Physics step | < 8 ms | 120 Hz physics |
| Entity count | 100,000+ | Solar system + asteroids |
| Render calls | < 1,000 | Instanced rendering |
| GPU memory | < 2 GB | Consumer hardware |
| CPU memory | < 512 MB | 100K bodies |

## Appendix C: Reference Material

- **Kepler's Equation:** Newton-Raphson solver (converges in ~5 iterations for e < 0.9)
- **N-body complexity:** Direct O(N²), Barnes-Hut O(N log N), Fast Multipole O(N)
- **Atmospheric scattering:** Bruneton & Neyret 2008, Hillaire 2020
- **Tone mapping:** ACES Filmic (Narkowicz 2015)
- **Orbital elements:** Standish 1992 (J2000.0 epoch)

---

*Last updated: 2026-07-20*  
*Status: Living document — evolves with the codebase*
