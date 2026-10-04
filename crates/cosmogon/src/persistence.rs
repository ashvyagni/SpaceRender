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
    /// Preferred display units (indices into `ui::units` tables).
    pub mass_unit: usize,
    pub distance_unit: usize,
    pub speed_unit: usize,
    pub temperature_unit: usize,
    /// Advanced mode shows every quantity, state vectors and provenance details.
    pub advanced: bool,
    pub show_trails: bool,
    pub show_predictions: bool,
    pub show_velocity: bool,
    /// Intro animation on start.
    pub intro: bool,
    /// Master volume for the soundtrack and effects (0 = silent).
    pub sound_volume: f32,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            graphics: GraphicsPreset::High,
            ui_scale: 1.0,
            autosave_minutes: 5.0,
            auto_slow: true,
            show_orbits: true,
            show_labels: true,
            mass_unit: 1,
            distance_unit: 1,
            speed_unit: 1,
            temperature_unit: 0,
            advanced: false,
            show_trails: true,
            show_predictions: true,
            show_velocity: false,
            intro: true,
            sound_volume: 0.6,
        }
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

// ── Sandbox documents (docs/SAVE_FORMAT.md) ─────────────────────────────────

use std::collections::BTreeMap;

use cosmogon_sim::Universe;

pub const MANIFEST_FORMAT: &str = "cosmogon-sandbox";
pub const MANIFEST_VERSION: u32 = 1;

/// The offline profile. Accounts (later) map onto profiles without changing save layout.
pub fn profile_dir() -> PathBuf {
    data_dir().join("profiles").join("local")
}

pub fn sandboxes_dir() -> PathBuf {
    profile_dir().join("sandboxes")
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Profile {
    pub id: String,
    pub display_name: String,
    pub created_unix: u64,
}

pub fn now_unix() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Load the local profile, creating it on first run.
pub fn ensure_profile() -> Profile {
    let path = profile_dir().join("profile.json");
    if let Some(p) = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str(&s).ok()) {
        return p;
    }
    let p = Profile { id: "local".into(), display_name: "Local profile".into(), created_unix: now_unix() };
    let _ = std::fs::create_dir_all(profile_dir());
    if let Ok(s) = serde_json::to_string_pretty(&p) {
        let _ = std::fs::write(path, s);
    }
    p
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Origin {
    /// "dataset:<id>@<version>" or "legacy-save:<file>".
    pub cloned_from: Option<String>,
    pub parent_sandbox: Option<String>,
    pub parent_checkpoint: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CheckpointInfo {
    pub file: String,
    pub label: String,
    pub sim_date: String,
    pub created_unix: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PhysicsSummary {
    pub model: String,
    pub preset: String,
    pub relativity: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub created_unix: u64,
    pub modified_unix: u64,
    pub template: String,
    #[serde(default)]
    pub origin: Origin,
    #[serde(default)]
    pub sim_date: String,
    #[serde(default)]
    pub sim_time: f64,
    #[serde(default)]
    pub seed: u64,
    #[serde(default)]
    pub app_version: String,
    #[serde(default)]
    pub physics_engine: String,
    #[serde(default)]
    pub data_versions: BTreeMap<String, String>,
    #[serde(default)]
    pub physics: PhysicsSummary,
    #[serde(default)]
    pub systems_enabled: BTreeMap<String, bool>,
    #[serde(default)]
    pub checkpoints: Vec<CheckpointInfo>,
    #[serde(default)]
    pub bookmarks: Vec<String>,
    #[serde(default)]
    pub edits: usize,
    pub owner: String,
}

pub fn new_sandbox_id() -> String {
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let rnd = cosmogon_sim::rng::mix(ms as u64, std::process::id() as u64) & 0x00FF_FFFF;
    format!("sb-{ms}-{rnd:06x}")
}

impl Manifest {
    pub fn new(name: &str, description: &str, template: &str, origin: Origin) -> Self {
        let now = now_unix();
        Self {
            format: MANIFEST_FORMAT.into(),
            version: MANIFEST_VERSION,
            id: new_sandbox_id(),
            name: name.into(),
            description: description.into(),
            created_unix: now,
            modified_unix: now,
            template: template.into(),
            origin,
            sim_date: String::new(),
            sim_time: 0.0,
            seed: 0,
            app_version: env!("CARGO_PKG_VERSION").into(),
            physics_engine: cosmogon_sim::save::PHYSICS_ENGINE.into(),
            data_versions: BTreeMap::new(),
            physics: PhysicsSummary::default(),
            systems_enabled: BTreeMap::new(),
            checkpoints: Vec::new(),
            bookmarks: Vec::new(),
            edits: 0,
            owner: ensure_profile().id,
        }
    }

    pub fn dir(&self) -> PathBuf {
        sandboxes_dir().join(&self.id)
    }

    pub fn thumbnail_path(&self) -> PathBuf {
        self.dir().join("thumbnail.png")
    }

    /// Refresh the descriptive fields from the current universe.
    pub fn update_from(&mut self, u: &Universe) {
        self.modified_unix = now_unix();
        self.sim_time = u.time;
        self.sim_date = if u.gregorian() { cosmogon_sim::time::format_datetime(u.time) } else { u.date_label() };
        self.seed = u.settings.seed;
        self.app_version = env!("CARGO_PKG_VERSION").into();
        self.physics_engine = cosmogon_sim::save::PHYSICS_ENGINE.into();
        self.data_versions = cosmogon_sim::save::data_versions(u);
        let home = &u.systems[0];
        self.physics = match &home.dynamics {
            Some(d) => PhysicsSummary { model: "N-body".into(), preset: d.settings.preset.label().into(), relativity: d.settings.relativity },
            None => PhysicsSummary { model: "Kepler".into(), preset: "—".into(), relativity: false },
        };
        self.systems_enabled = [("climate".to_string(), true), ("life".to_string(), u.settings.systems.life), ("civilization".to_string(), u.settings.systems.civilization)].into_iter().collect();
        self.edits = u.edits.len();
    }

    pub fn write(&self) -> Result<(), String> {
        let dir = self.dir();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let tmp = dir.join("manifest.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(tmp, dir.join("manifest.json")).map_err(|e| e.to_string())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveSlot {
    State,
    Autosave,
}

impl SaveSlot {
    fn file(self) -> &'static str {
        match self {
            Self::State => "state.cosmo",
            Self::Autosave => "autosave.cosmo",
        }
    }
}

/// Write the universe into a sandbox (and its manifest).
pub fn save_sandbox(m: &mut Manifest, u: &Universe, slot: SaveSlot) -> Result<PathBuf, String> {
    m.update_from(u);
    let path = m.dir().join(slot.file());
    cosmogon_sim::save::write_file(&path, u, &m.name).map_err(|e| e.to_string())?;
    if slot == SaveSlot::State {
        // An explicit save supersedes the autosave.
        let _ = std::fs::remove_file(m.dir().join(SaveSlot::Autosave.file()));
    }
    m.write()?;
    Ok(path)
}

#[derive(Clone, Debug)]
pub struct SandboxEntry {
    pub manifest: Manifest,
    pub has_thumbnail: bool,
}

/// All sandboxes, most recently modified first.
pub fn list_sandboxes() -> Vec<SandboxEntry> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(sandboxes_dir()) {
        for e in rd.flatten() {
            let dir = e.path();
            let Some(m) = std::fs::read_to_string(dir.join("manifest.json")).ok().and_then(|s| serde_json::from_str::<Manifest>(&s).ok()) else { continue };
            if m.format != MANIFEST_FORMAT {
                continue;
            }
            let has_thumbnail = dir.join("thumbnail.png").exists();
            out.push(SandboxEntry { manifest: m, has_thumbnail });
        }
    }
    out.sort_by(|a, b| b.manifest.modified_unix.cmp(&a.manifest.modified_unix));
    out
}

/// The newest state of a sandbox: the autosave if it is newer than the explicit save.
pub fn newest_state_file(m: &Manifest) -> Option<PathBuf> {
    let modified = |p: &PathBuf| std::fs::metadata(p).and_then(|m| m.modified()).ok();
    let state = m.dir().join(SaveSlot::State.file());
    let auto = m.dir().join(SaveSlot::Autosave.file());
    match (state.exists(), auto.exists()) {
        (true, true) => Some(if modified(&auto) > modified(&state) { auto } else { state }),
        (true, false) => Some(state),
        (false, true) => Some(auto),
        _ => None,
    }
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()))?;
        } else {
            std::fs::copy(&p, to.join(e.file_name()))?;
        }
    }
    Ok(())
}

/// Copy a sandbox under a new id and name (a branch of the original).
pub fn duplicate_sandbox(m: &Manifest, name: &str) -> Result<Manifest, String> {
    let mut copy = m.clone();
    copy.id = new_sandbox_id();
    copy.name = name.into();
    copy.created_unix = now_unix();
    copy.modified_unix = copy.created_unix;
    copy.origin.parent_sandbox = Some(m.id.clone());
    copy_dir(&m.dir(), &copy.dir()).map_err(|e| e.to_string())?;
    copy.write()?;
    Ok(copy)
}

pub fn rename_sandbox(m: &mut Manifest, name: &str) -> Result<(), String> {
    m.name = name.into();
    m.write()
}

/// Delete a sandbox and all its files (the UI asks for confirmation first).
pub fn delete_sandbox(m: &Manifest) -> Result<(), String> {
    let dir = m.dir();
    // Never delete anything outside the sandboxes directory.
    if !dir.starts_with(sandboxes_dir()) || m.id.is_empty() || m.id.contains(['/', '\\', '.']) {
        return Err("refusing to delete an unexpected path".into());
    }
    std::fs::remove_dir_all(dir).map_err(|e| e.to_string())
}

pub fn create_checkpoint(m: &mut Manifest, u: &Universe, label: &str) -> Result<(), String> {
    let now = now_unix();
    let file = format!("checkpoints/{now}-{}.cosmo", m.checkpoints.len());
    cosmogon_sim::save::write_file(&m.dir().join(&file), u, label).map_err(|e| e.to_string())?;
    let date = if u.gregorian() { cosmogon_sim::time::format_datetime(u.time) } else { u.date_label() };
    m.checkpoints.push(CheckpointInfo { file, label: label.into(), sim_date: date, created_unix: now });
    m.write()
}

pub fn load_universe_file(path: &std::path::Path) -> Result<Universe, String> {
    cosmogon_sim::save::read_file(path).map_err(|e| e.to_string())
}

/// Downscale an RGBA8 image (box filter) and write it as PNG.
pub fn write_thumbnail(path: &std::path::Path, rgba: &[u8], w: u32, h: u32) -> Result<(), String> {
    let (tw, th) = (480u32, (480 * h / w.max(1)).max(1));
    let mut out = vec![0u8; (tw * th * 4) as usize];
    for y in 0..th {
        for x in 0..tw {
            let (x0, x1) = (x * w / tw, ((x + 1) * w / tw).max(x * w / tw + 1));
            let (y0, y1) = (y * h / th, ((y + 1) * h / th).max(y * h / th + 1));
            let mut acc = [0u32; 4];
            let mut n = 0;
            for sy in y0..y1.min(h) {
                for sx in x0..x1.min(w) {
                    let i = ((sy * w + sx) * 4) as usize;
                    for c in 0..4 {
                        acc[c] += rgba[i + c] as u32;
                    }
                    n += 1;
                }
            }
            let o = ((y * tw + x) * 4) as usize;
            for c in 0..4 {
                out[o + c] = (acc[c] / n.max(1)) as u8;
            }
            out[o + 3] = 255;
        }
    }
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    image::save_buffer(path, &out, tw, th, image::ColorType::Rgba8).map_err(|e| e.to_string())
}
