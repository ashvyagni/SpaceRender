use bevy_ecs::schedule::SystemSet;

/// System sets that define execution ordering for the simulation pipeline.
///
/// ## Pipeline order
///
/// ```text
/// Input → TimeUpdate → OrbitalUpdate → Physics → TransformSync
///      → CameraUpdate → RenderPrep → Render → Ui
/// ```
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimulationSet {
    /// Poll and ingest user input events.
    Input,
    /// Advance simulation clocks and time accelerations.
    TimeUpdate,
    /// Propagate Keplerian orbits and update orbital velocities.
    OrbitalUpdate,
    /// Run gravity, collisions, and force integrations.
    Physics,
    /// Synchronize derived transforms from position / rotation / scale.
    TransformSync,
    /// Update camera position, orientation, and projection.
    CameraUpdate,
    /// Prepare render primitives, culling, LOD selection.
    RenderPrep,
    /// Submit draw calls to the GPU backend.
    Render,
    /// Immediate-mode UI overlays and HUD.
    Ui,
}
