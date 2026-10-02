use cosmogon_ecs::resources::input_state::*;

pub struct InputHandler {
    // Input state is handled via ECS resources
}

impl InputHandler {
    pub fn new() -> Self {
        Self {}
    }

    pub fn reset_frame(&self, mouse: &mut MouseState, scroll: &mut ScrollDelta) {
        mouse.delta = (0.0, 0.0);
        scroll.0 = 0.0;
    }
}
