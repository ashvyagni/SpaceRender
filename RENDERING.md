# Rendering

Bevy 0.18 (wgpu: Metal / Vulkan / DX12) with HDR, AgX tonemapping, bloom, MSAA and a procedural skybox.

## Precision across scales
* Simulation positions are `f64` metres. Each visual entity carries `WorldPos(DVec3)`.
* Each frame the camera rig computes its own `f64` position; `ViewInfo.origin` is set to it and every
  `Transform.translation = (WorldPos − origin) as f32`. The camera itself sits at the origin.
* Bevy uses an infinite reverse-Z perspective, so depth precision is good from the near plane to 10²² m;
  the near plane follows altitude (5% of the distance to the surface, clamped to [0.5 m, 10⁹ m]).
* Result: stable, jitter-free views from a few hundred metres above a planet to tens of light-years.
* Frames: simulation X/Y orbital plane with +Z pole; render (x, y, z) = (x, z, −y). Bevy's UV-sphere mesh uses
  +Z as its pole with the same longitude convention as the simulation's terrain, so textures map 1:1.

## Planets
Custom `PlanetMaterial` (WGSL in `crates/cosmogon/src/render/shaders/planet.wgsl`), lit by **each planet's own
star direction** passed as a uniform — not by engine lights — so every system is lit correctly at any distance.
It combines Lambert lighting with a soft twilight terminator, ocean glint from a water mask, a drifting cloud
layer, limb scattering tint, a faint airglow floor and **night-side city lights**. A separate additive
`AtmosphereMaterial` shell adds the limb halo with a warm sunset tint. Illumination is compressed
(`flux^0.28`) so outer planets stay visible: an artistic exposure until auto-exposure lands.

## Surfaces (interim, pre-Phase 3)
Textures are baked on background threads from the simulation's terrain/biome functions at screen-size-driven
levels (256 px → max preset width up to 4096 px), largest-on-screen first, ≤ 3 bakes in flight. A body is
**re-baked when its simulated environment changes** (oceans, ice, temperature, vegetation): you can watch a
world green up or freeze over. Gas giants get banded turbulence; icy moons cracked ice; airless worlds rock and
craters; thick hazes (Venus, Titan) a cloud deck. Biome colours blend continuously from temperature and moisture.

## Civilizations
City lights are baked from settlements (core + population-scaled sprawl) and lit transport corridors, refreshed
every 1.5 s while visible, and shown only on the night side. Overlays (egui, occluded by nearby planets) draw
settlements, roads/rail/sea lanes, satellites, colonies, probes and star/planet markers. Gizmos draw orbits of
the focused system and expanding radio spheres.

## Stars and sky
Stars are unlit HDR spheres scaled to stay a few pixels wide at any distance; bloom makes them glow. The skybox
is a procedural cubemap (galactic band with dust lanes plus ~100k point stars); the simulated neighbourhood stars
are real objects on top of it.

## Graphics presets
Low / Medium / High / Ultra change sphere tessellation, maximum texture size, bloom and MSAA.
They never change simulation results.

## Verification
`cosmogon --new sol --seed 4 --advance-years 200300 --focus Earth --capture out.png --quit` renders a frame to a
PNG from the GPU (no OS screen recording needed); used to check every visual change in this milestone.
