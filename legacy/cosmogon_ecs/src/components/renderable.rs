use bevy_ecs::prelude::*;

/// Mesh type used for rendering a celestial body.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderMesh {
    /// UV sphere with a given subdivision count.
    Sphere { subdivisions: u32 },
    /// Camera-facing billboard (used for distant stars, particles).
    Billboard,
    /// Custom mesh handle by id.
    Custom(u64),
}

/// RGBA color.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red channel [0.0 – 1.0].
    pub r: f32,
    /// Green channel [0.0 – 1.0].
    pub g: f32,
    /// Blue channel [0.0 – 1.0].
    pub b: f32,
    /// Alpha channel [0.0 – 1.0].
    pub a: f32,
}

impl Color {
    /// Create a new RGBA color.
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque RGB color.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// White, fully opaque.
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);

    /// Black, fully opaque.
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);

    /// Convert to `[r, g, b, a]` array.
    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

/// Emissive (self-lit) color with intensity.
#[derive(Component, Debug, Clone, Copy)]
pub struct EmissiveColor {
    /// Emissive color.
    pub color: Color,
    /// Intensity multiplier (HDR).
    pub intensity: f32,
}

impl Default for EmissiveColor {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
        }
    }
}

/// Handle to a GPU-resident mesh.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshId(pub u64);

/// Handle to a GPU-resident material.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialId(pub u64);
