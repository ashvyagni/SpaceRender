use winit::dpi::{LogicalSize, PhysicalSize};

pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
    pub vsync: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            title: "Cosmogon".to_string(),
            vsync: true,
        }
    }
}

impl WindowConfig {
    pub fn physical_size(&self) -> PhysicalSize<u32> {
        PhysicalSize::new(self.width, self.height)
    }

    pub fn logical_size(&self) -> LogicalSize<f64> {
        LogicalSize::new(self.width as f64, self.height as f64)
    }
}
