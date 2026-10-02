use bevy_ecs::prelude::*;
use std::collections::HashSet;

/// Keyboard key identifiers (subset relevant to camera / UI controls).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    W,
    A,
    S,
    D,
    Q,
    E,
    Space,
    LShift,
    LControl,
    Escape,
    Tab,
    F1,
    F2,
    F3,
    F4,
}

/// Current state of the mouse each frame.
#[derive(Resource, Debug, Clone, Default)]
pub struct MouseState {
    /// Cursor position in window-normalized coordinates.
    pub position: (f32, f32),
    /// Mouse movement delta since last frame.
    pub delta: (f32, f32),
    /// Left button held down.
    pub left_pressed: bool,
    /// Right button held down.
    pub right_pressed: bool,
    /// Middle button held down.
    pub middle_pressed: bool,
}

/// Current state of the keyboard each frame.
#[derive(Resource, Debug, Clone, Default)]
pub struct KeyboardState {
    /// Keys currently held down.
    pub pressed: HashSet<KeyCode>,
}

/// Scroll wheel delta for the current frame.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct ScrollDelta(pub f32);
