/// Application configuration
pub struct AppConfig {
    pub window_title: String,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
    pub msaa: bool,
    pub hdr: bool,
    pub bloom: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window_title: "Cosmogon — Universe Simulator".to_string(),
            width: 1920,
            height: 1080,
            vsync: true,
            msaa: true,
            hdr: true,
            bloom: true,
        }
    }
}
