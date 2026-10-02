//! User settings and save-file locations.

use std::path::PathBuf;

use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphicsPreset {
    Low,
    Medium,
    High,
    Ultra,
}

impl GraphicsPreset {
    pub const ALL: [GraphicsPreset; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }
    /// Width of the most detailed planet texture (height is half).
    pub fn max_texture_width(self) -> u32 {
        match self {
            Self::Low => 512,
            Self::Medium => 1024,
            Self::High => 2048,
            Self::Ultra => 4096,
        }
    }
    pub fn sphere_segments(self) -> (u32, u32) {
        match self {
            Self::Low => (48, 24),
            Self::Medium => (96, 48),
            Self::High => (160, 80),
            Self::Ultra => (256, 128),
        }
    }
    pub fn bloom(self) -> bool {
        self != Self::Low
    }
    pub fn msaa(self) -> u32 {
        match self {
            Self::Low => 1,
            Self::Medium => 2,
            _ => 4,
        }
    }
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct UserSettings {
    pub graphics: GraphicsPreset,
    pub ui_scale: f32,
    pub autosave_minutes: f32,
    /// Slow down automatically when a milestone happens.
    pub auto_slow: bool,
    pub show_orbits: bool,
    pub show_labels: bool,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self { graphics: GraphicsPreset::High, ui_scale: 1.0, autosave_minutes: 5.0, auto_slow: true, show_orbits: true, show_labels: true }
    }
}

pub fn data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("Cosmogon")
}

pub fn saves_dir() -> PathBuf {
    data_dir().join("saves")
}

fn settings_path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("Cosmogon").join("settings.toml")
}

impl UserSettings {
    pub fn load() -> Self {
        std::fs::read_to_string(settings_path()).ok().and_then(|s| toml::from_str(&s).ok()).unwrap_or_default()
    }
    pub fn save(&self) {
        let p = settings_path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Ok(s) = toml::to_string_pretty(self) {
            let _ = std::fs::write(p, s);
        }
    }
}

#[derive(Clone, Debug)]
pub struct SaveEntry {
    pub path: PathBuf,
    pub header: cosmogon_sim::save::SaveHeader,
}

/// Saves, newest first.
pub fn list_saves() -> Vec<SaveEntry> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(saves_dir()) {
        for e in rd.flatten() {
            let path = e.path();
            if path.extension().and_then(|x| x.to_str()) != Some(cosmogon_sim::save::EXTENSION) {
                continue;
            }
            // Headers sit at the start of the file; reading it whole is fine at current sizes.
            if let Ok(s) = std::fs::read_to_string(&path) {
                if let Ok(header) = cosmogon_sim::save::read_header(&s) {
                    out.push(SaveEntry { path, header });
                }
            }
        }
    }
    out.sort_by(|a, b| b.header.saved_at_unix.cmp(&a.header.saved_at_unix));
    out
}
