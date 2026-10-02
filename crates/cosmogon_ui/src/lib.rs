//! Cosmogon UI crate — egui-based panels for the universe simulator.
//!
//! Provides an object inspector, time controls, camera settings, debug overlay,
//! and ECS systems to drive them.

pub mod camera_panel;
pub mod debug_overlay;
pub mod inspector;
pub mod systems;
pub mod time_controls;

use egui_wgpu::Renderer as EguiWgpuRenderer;
use egui_winit::State as EguiWinitState;

/// Combined egui integration state holding the winit input handler and the wgpu
/// paint renderer.
pub struct EguiContext {
    /// egui ↔ winit integration (raw input, platform output, clipboard).
    pub winit_state: EguiWinitState,
    /// egui ↔ wgpu paint renderer (textures, draw lists, screen-sized buffers).
    pub wgpu_renderer: EguiWgpuRenderer,
}
