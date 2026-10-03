//! The application shell: cinematic intro, HOME, the new-sandbox workflow, the sandbox
//! library (load, duplicate, rename, delete, checkpoints), the real-universe view,
//! curated scenarios, settings and credits. See docs/UI_SYSTEM.md.

use std::collections::HashMap;

use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy_egui::{egui, EguiContexts, EguiTextureHandle};
use cosmogon_sim::astro::dynamics::PhysicsPreset;
use cosmogon_sim::sandbox::WHAT_IFS;
use cosmogon_sim::universe::{EnabledSystems, LIFE_PRESETS};
use cosmogon_sim::{Scenario, UniverseSettings};

use super::{UiState, ACCENT, ACCENT_DIM, DANGER, MUTED, TEXT};
use crate::persistence::{self, list_saves, list_sandboxes, GraphicsPreset, Manifest, SandboxEntry, UserSettings};
use crate::render::materials::{AtmosphereMaterial, AtmosphereUniform, PlanetMaterial, PlanetUniform};
use crate::render::{atmosphere_look, bake, solid_image, SharedMeshes, ViewInfo};
use crate::sim::{begin_load_legacy, begin_new, begin_open_sandbox, begin_reference, MenuMessage, NewSandbox, PendingUniverse};
use crate::state::AppState;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum MenuScreen {
    #[default]
    Home,
    NewSandbox,
    Load,
    Scenarios,
    Settings,
    Credits,
}

/// Sandbox templates (docs/SANDBOX_VISION.md).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Template {
    #[default]
    SolarSystemLab,
    DawnOfHumanity,
    HabitableWorld,
    ProceduralSystem,
    ProceduralNeighbourhood,
    EmptySystem,
    Custom,
}

impl Template {
    pub const ALL: [Template; 7] = [Self::SolarSystemLab, Self::DawnOfHumanity, Self::HabitableWorld, Self::ProceduralSystem, Self::ProceduralNeighbourhood, Self::EmptySystem, Self::Custom];
    fn title(self) -> &'static str {
        match self {
            Self::SolarSystemLab => "Solar System Lab",
            Self::DawnOfHumanity => "Dawn of Humanity",
            Self::HabitableWorld => "Habitable World Experiment",
            Self::ProceduralSystem => "Procedural Star System",
            Self::ProceduralNeighbourhood => "Stellar Neighbourhood",
            Self::EmptySystem => "Empty System",
            Self::Custom => "Custom",
        }
    }
    fn tag(self) -> &'static str {
        match self {
            Self::SolarSystemLab => "REAL DATA · N-BODY",
            Self::DawnOfHumanity => "REAL SOLAR SYSTEM · CIVILIZATION",
            Self::HabitableWorld => "LIFE · CIVILIZATION",
            Self::ProceduralSystem => "PROCEDURAL",
            Self::ProceduralNeighbourhood => "PROCEDURAL · MANY STARS",
            Self::EmptySystem => "BUILD YOUR OWN",
            Self::Custom => "EXPERT",
        }
    }
    fn description(self) -> &'static str {
        match self {
            Self::SolarSystemLab => "Today's Solar System from NASA/JPL Horizons state vectors (1 Jan 2026), with dynamic N-body gravity. Earth has its present biosphere.",
            Self::DawnOfHumanity => "The real Solar System 200,000 years ago with early humans. Their history — technology, nations, wars — unfolds from here.",
            Self::HabitableWorld => "A star with an Earth-like world already rich in complex life. Will intelligence emerge?",
            Self::ProceduralSystem => "One physically plausible star system generated from a seed.",
            Self::ProceduralNeighbourhood => "Dozens of procedurally generated star systems with realistic statistics. (A full galaxy arrives in milestone S5.)",
            Self::EmptySystem => "A Sun-like star alone. Add planets, moons and asteroids yourself.",
            Self::Custom => "Every option: scenario, seed, number of stars, life and technology rates, physics.",
        }
    }
    fn scenario(self) -> Scenario {
        match self {
            Self::SolarSystemLab => Scenario::SolarSystemLab,
            Self::DawnOfHumanity => Scenario::Sol,
            Self::HabitableWorld => Scenario::GardenWorld,
            Self::ProceduralSystem => Scenario::StarSystem,
            Self::ProceduralNeighbourhood | Self::Custom => Scenario::Neighbourhood,
            Self::EmptySystem => Scenario::EmptySystem,
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::SolarSystemLab => "solar_system_lab",
            Self::DawnOfHumanity => "dawn_of_humanity",
            Self::HabitableWorld => "habitable_world",
            Self::ProceduralSystem => "procedural_system",
            Self::ProceduralNeighbourhood => "procedural_neighbourhood",
            Self::EmptySystem => "empty_system",
            Self::Custom => "custom",
        }
    }
    /// Dynamic gravity on by default where users are expected to edit right away.
    fn default_nbody(self) -> bool {
        matches!(self, Self::SolarSystemLab | Self::ProceduralSystem | Self::EmptySystem)
    }
    fn uses_seed(self) -> bool {
        !matches!(self, Self::SolarSystemLab | Self::EmptySystem)
    }
}

/// The new-sandbox form.
#[derive(Clone, Debug)]
pub struct NewForm {
    pub template: Template,
    pub name: String,
    pub description: String,
    pub seed: u64,
    pub nbody: bool,
    pub preset: PhysicsPreset,
    pub life: bool,
    pub civilization: bool,
    pub life_preset: usize,
    pub tech_rate: f64,
    pub resources: f64,
    pub systems: u32,
    pub scenario: Scenario,
}

impl Default for NewForm {
    fn default() -> Self {
        let mut f = Self {
            template: Template::SolarSystemLab,
            name: String::new(),
            description: String::new(),
            seed: 2026,
            nbody: true,
            preset: PhysicsPreset::Balanced,
            life: true,
            civilization: true,
            life_preset: 0,
            tech_rate: 1.0,
            resources: 1.0,
            systems: 24,
            scenario: Scenario::SolarSystemLab,
        };
        f.pick(Template::SolarSystemLab);
        f
    }
}

impl NewForm {
    fn pick(&mut self, t: Template) {
        self.template = t;
        self.name = t.title().to_string();
        self.nbody = t.default_nbody();
        self.scenario = t.scenario();
    }
}

/// Thumbnails loaded into egui, by sandbox id.
#[derive(Default)]
pub struct Thumbs(pub HashMap<String, Option<(Handle<Image>, egui::TextureId)>>);

#[derive(Component)]
struct MenuVisual;

pub struct MenuScenePlugin;

impl Plugin for MenuScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), spawn_showcase)
            .add_systems(OnEnter(AppState::Observing), despawn_showcase)
            .add_systems(Update, place_showcase.run_if(not(in_state(AppState::Observing))));
    }
}

/// A freshly generated Earth-like world (never the same twice) behind the menu.
fn spawn_showcase(
    mut commands: Commands,
    existing: Query<(), With<MenuVisual>>,
    shared: Res<SharedMeshes>,
    mut planet_mats: ResMut<Assets<PlanetMaterial>>,
    mut atmo_mats: ResMut<Assets<AtmosphereMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    if !existing.is_empty() {
        return;
    }
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(7);
    let settings = UniverseSettings { seed, scenario: Scenario::GardenWorld, ..Default::default() };
    let sys = cosmogon_sim::astro::generate::generate_system(&settings, 0, cosmogon_sim::Vec3d::ZERO, true);
    let Some(body) = sys.bodies.iter().find(|b| b.kind == cosmogon_sim::astro::BodyKind::Rocky && b.hydro.ocean_fraction > 0.2).or(sys.bodies.first()) else { return };
    let mut body = body.clone();
    body.radius = 6.371e6;
    let baked = bake::bake_surface(&body, true, 2048);
    let mk = |data: Vec<u8>, f| {
        let mut img = Image::new(
            bevy::render::render_resource::Extent3d { width: baked.width, height: baked.height, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2,
            data,
            f,
            bevy::asset::RenderAssetUsages::RENDER_WORLD,
        );
        img.sampler = bevy::image::ImageSampler::linear();
        img
    };
    let albedo = images.add(mk(baked.albedo, TextureFormat::Rgba8UnormSrgb));
    let clouds = images.add(mk(baked.clouds, TextureFormat::R8Unorm));
    let lights = images.add(solid_image([0, 0, 0, 0], TextureFormat::R8Unorm));
    let sun = Vec4::new(-0.55, 0.3, 0.8, 1.5);
    let (ac, astr) = atmosphere_look(&body).unwrap_or(([0.3, 0.55, 1.0], 0.8));
    let m = planet_mats.add(PlanetMaterial {
        u: PlanetUniform { sun: Vec3::new(sun.x, sun.y, sun.z).normalize().extend(sun.w), sun_color: Vec4::ONE, atmo: Vec4::new(ac[0], ac[1], ac[2], astr), params: Vec4::new(0.0, 0.85, 0.0, 1.0) },
        albedo,
        clouds,
        lights,
    });
    let a = atmo_mats.add(AtmosphereMaterial { u: AtmosphereUniform { sun: Vec3::new(sun.x, sun.y, sun.z).normalize().extend(sun.w), color: Vec4::new(ac[0], ac[1], ac[2], astr) } });
    commands
        .spawn((Mesh3d(shared.sphere.clone()), MeshMaterial3d(m), Transform::from_scale(Vec3::splat(6.371e6)), MenuVisual))
        .with_children(|p| {
            p.spawn((Mesh3d(shared.sphere.clone()), MeshMaterial3d(a), Transform::from_scale(Vec3::splat(1.025))));
        });
}

fn place_showcase(view: Res<ViewInfo>, time: Res<Time>, mut q: Query<&mut Transform, With<MenuVisual>>) {
    for mut tf in &mut q {
        tf.translation = (DVec3::ZERO - view.origin).as_vec3();
        tf.rotation = Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2) * Quat::from_rotation_x(0.4) * Quat::from_rotation_z(time.elapsed_secs() * 0.02);
    }
}

fn despawn_showcase(mut commands: Commands, q: Query<Entity, With<MenuVisual>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

fn nav_button(ui: &mut egui::Ui, text: &str, selected: bool, enabled: bool) -> egui::Response {
    let color = if !enabled { MUTED.gamma_multiply(0.6) } else if selected { ACCENT } else { TEXT };
    let r = ui.add_enabled(enabled, egui::Button::new(egui::RichText::new(text).size(15.0).color(color).extra_letter_spacing(2.0)).frame(false).min_size(egui::vec2(260.0, 34.0)));
    if selected {
        let rect = r.rect;
        ui.painter().line_segment([rect.left_bottom() + egui::vec2(0.0, -4.0), rect.left_bottom() + egui::vec2(36.0, -4.0)], egui::Stroke::new(2.0_f32, ACCENT));
    }
    r
}

fn panel_frame() -> egui::Frame {
    egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(6, 8, 14, 236)).inner_margin(egui::Margin::symmetric(36, 30))
}

fn card(ui: &mut egui::Ui, selected: bool, add: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    let stroke = if selected { egui::Stroke::new(1.5_f32, ACCENT) } else { egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(38, 46, 64)) };
    egui::Frame::new()
        .fill(if selected { egui::Color32::from_rgb(26, 22, 14) } else { egui::Color32::from_rgb(13, 17, 27) })
        .stroke(stroke)
        .corner_radius(8)
        .inner_margin(14)
        .show(ui, add)
        .response
        .interact(egui::Sense::click())
}

fn primary_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add(egui::Button::new(egui::RichText::new(text).strong().size(15.0).color(egui::Color32::from_rgb(20, 14, 4))).fill(ACCENT).min_size(egui::vec2(180.0, 36.0)))
}

fn ago(unix: u64) -> String {
    let d = persistence::now_unix().saturating_sub(unix);
    match d {
        0..=59 => "just now".into(),
        60..=3599 => format!("{} min ago", d / 60),
        3600..=86_399 => format!("{} h ago", d / 3600),
        _ => format!("{} days ago", d / 86_400),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn main_menu(
    mut contexts: EguiContexts,
    mut ui_state: ResMut<UiState>,
    mut commands: Commands,
    mut next: ResMut<NextState<AppState>>,
    mut settings: ResMut<UserSettings>,
    mut msg: ResMut<MenuMessage>,
    mut exit: MessageWriter<AppExit>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time>,
    mut thumbs: Local<Thumbs>,
) -> Result {
    // Thumbnails need the egui texture registry before the context is borrowed.
    if ui_state.menu == MenuScreen::Load {
        let wanted: Vec<SandboxEntry> = ui_state.sandboxes.iter().filter(|e| e.has_thumbnail && !thumbs.0.contains_key(&e.manifest.id)).cloned().collect();
        for e in wanted {
            let tex = std::fs::read(e.manifest.thumbnail_path()).ok().and_then(|bytes| image::load_from_memory(&bytes).ok()).map(|img| {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let image = Image::new(
                    bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                    bevy::render::render_resource::TextureDimension::D2,
                    rgba.into_raw(),
                    TextureFormat::Rgba8UnormSrgb,
                    bevy::asset::RenderAssetUsages::RENDER_WORLD | bevy::asset::RenderAssetUsages::MAIN_WORLD,
                );
                let handle = images.add(image);
                let id = contexts.add_image(EguiTextureHandle::Strong(handle.clone()));
                (handle, id)
            });
            thumbs.0.insert(e.manifest.id.clone(), tex);
        }
    }
    let now = time.elapsed_secs_f64();
    let ctx = contexts.ctx_mut()?;
    let s = &mut *ui_state;

    // Cinematic intro: the title fades in over the planet; any click skips it.
    let intro_len = 2.6;
    let start = *s.intro_started.get_or_insert(now);
    let in_intro = settings.intro && now - start < intro_len && !s.intro_skipped;
    if in_intro {
        if ctx.input(|i| i.pointer.any_click() || i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter)) {
            s.intro_skipped = true;
        }
        let k = ((now - start) / intro_len) as f32;
        let alpha = (k / 0.35).min(1.0) * (1.0 - ((k - 0.8) / 0.2).clamp(0.0, 1.0));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, (200.0 * (1.0 - k)) as u8))).show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.38);
                ui.label(egui::RichText::new("COSMOGON").size(64.0).strong().extra_letter_spacing(18.0).color(TEXT.gamma_multiply(alpha)));
                ui.add_space(8.0);
                ui.label(egui::RichText::new("a universe that remembers").size(17.0).italics().color(ACCENT.gamma_multiply(alpha)));
            });
        });
        ctx.request_repaint();
        return Ok(());
    }

    // ── Navigation column ──
    egui::SidePanel::left("nav").exact_width(330.0).resizable(false).frame(panel_frame()).show(ctx, |ui| {
        ui.add_space(24.0);
        ui.label(egui::RichText::new("COSMOGON").size(34.0).color(TEXT).strong().extra_letter_spacing(5.0));
        ui.label(egui::RichText::new("universe sandbox").size(14.0).color(ACCENT).italics());
        ui.add_space(40.0);
        let latest = list_sandboxes().into_iter().next();
        let items: [(&str, Option<MenuScreen>, bool); 9] = [
            ("NEW SANDBOX", Some(MenuScreen::NewSandbox), true),
            ("CONTINUE", None, latest.is_some()),
            ("LOAD SANDBOX", Some(MenuScreen::Load), true),
            ("REAL UNIVERSE", None, true),
            ("SCENARIOS", Some(MenuScreen::Scenarios), true),
            ("OBJECT LAB", None, false),
            ("SETTINGS", Some(MenuScreen::Settings), true),
            ("CREDITS", Some(MenuScreen::Credits), true),
            ("QUIT", None, true),
        ];
        for (label, screen, enabled) in items {
            let selected = screen == Some(s.menu) && s.menu != MenuScreen::Home;
            let r = nav_button(ui, label, selected, enabled);
            let r = if label == "OBJECT LAB" { r.on_disabled_hover_text("Design and save object blueprints — milestone S3. Create objects inside any sandbox today.") } else { r };
            let r = if label == "CONTINUE" { r.on_hover_text(latest.as_ref().map(|l| format!("{} · {}", l.manifest.name, l.manifest.sim_date)).unwrap_or_default()) } else { r };
            if r.clicked() {
                msg.0 = None;
                match label {
                    "CONTINUE" => {
                        if let Some(l) = &latest {
                            begin_open_sandbox(&mut commands, &mut next, l.manifest.clone(), None);
                        }
                    }
                    "REAL UNIVERSE" => begin_reference(&mut commands, &mut next),
                    "QUIT" => {
                        exit.write(AppExit::Success);
                    }
                    _ => {
                        if let Some(sc) = screen {
                            s.menu = if s.menu == sc { MenuScreen::Home } else { sc };
                            if sc == MenuScreen::Load {
                                s.sandboxes = list_sandboxes();
                                s.saves = list_saves();
                            }
                        }
                    }
                }
            }
            ui.add_space(2.0);
        }
        if let Some(m) = &msg.0 {
            ui.add_space(16.0);
            ui.colored_label(DANGER, m);
        }
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
            ui.label(egui::RichText::new(format!("v{} · sandbox foundation", env!("CARGO_PKG_VERSION"))).size(11.0).color(MUTED));
            if let Some(l) = &latest {
                ui.label(egui::RichText::new(format!("Last: {} — {}", l.manifest.name, ago(l.manifest.modified_unix))).size(11.0).color(MUTED));
            }
        });
    });

    if s.menu == MenuScreen::Home {
        return Ok(());
    }

    // ── Content panel ──
    egui::SidePanel::right("content").exact_width(700.0).resizable(false).frame(panel_frame()).show(ctx, |ui| {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match s.menu {
            MenuScreen::NewSandbox => new_sandbox(ui, s, &mut commands, &mut next),
            MenuScreen::Load => load_screen(ui, s, &thumbs, &mut commands, &mut next),
            MenuScreen::Scenarios => scenarios(ui, s, &mut commands, &mut next),
            MenuScreen::Settings => {
                settings_ui(ui, &mut settings);
                ui.add_space(12.0);
                if ui.button("Save settings").clicked() {
                    settings.save();
                }
            }
            MenuScreen::Credits => credits(ui),
            MenuScreen::Home => {}
        });
    });
    Ok(())
}

fn heading(ui: &mut egui::Ui, title: &str, sub: &str) {
    ui.label(egui::RichText::new(title).size(26.0).strong().color(TEXT));
    ui.label(egui::RichText::new(sub).size(13.0).color(MUTED));
    ui.add_space(16.0);
}

fn new_sandbox(ui: &mut egui::Ui, s: &mut UiState, commands: &mut Commands, next: &mut NextState<AppState>) {
    heading(ui, "New sandbox", "Choose where your experiment begins. Everything can be changed once it is running.");
    egui::Grid::new("templates").num_columns(2).spacing([12.0, 12.0]).show(ui, |ui| {
        for (i, t) in Template::ALL.iter().enumerate() {
            let selected = s.form.template == *t;
            let r = card(ui, selected, |ui| {
                ui.set_width(286.0);
                ui.set_height(92.0);
                ui.label(egui::RichText::new(t.tag()).size(9.5).color(if selected { ACCENT } else { ACCENT_DIM }).strong().extra_letter_spacing(1.5));
                ui.label(egui::RichText::new(t.title()).size(16.0).strong().color(TEXT));
                ui.label(egui::RichText::new(t.description()).size(11.5).color(MUTED));
            });
            if r.clicked() {
                s.form.pick(*t);
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
    ui.add_space(18.0);
    ui.separator();
    ui.add_space(10.0);
    let f = &mut s.form;
    egui::Grid::new("form").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
        ui.label(egui::RichText::new("Name").color(MUTED));
        ui.add(egui::TextEdit::singleline(&mut f.name).desired_width(420.0));
        ui.end_row();
        ui.label(egui::RichText::new("Description").color(MUTED));
        ui.add(egui::TextEdit::multiline(&mut f.description).desired_rows(2).desired_width(420.0).hint_text("What are you testing?"));
        ui.end_row();
        if f.template.uses_seed() {
            ui.label(egui::RichText::new("Seed").color(MUTED));
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut f.seed).speed(1.0));
                if ui.button("Random").clicked() {
                    f.seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64 % 1_000_000).unwrap_or(1);
                }
            });
            ui.end_row();
        }
        ui.label(egui::RichText::new("Gravity").color(MUTED));
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut f.nbody, false, "Fixed orbits (Kepler)");
                ui.selectable_value(&mut f.nbody, true, "Dynamic (N-body)");
            });
            if f.nbody {
                egui::ComboBox::from_id_salt("preset").selected_text(f.preset.label()).show_ui(ui, |ui| {
                    for p in PhysicsPreset::ALL {
                        ui.selectable_value(&mut f.preset, p, p.label()).on_hover_text(p.description());
                    }
                });
                ui.label(egui::RichText::new(f.preset.description()).size(11.0).color(MUTED));
            } else {
                ui.label(egui::RichText::new("Analytic orbits: exact and fast at any speed, but bodies do not pull on each other. Editing anything switches that system to N-body.").size(11.0).color(MUTED));
            }
        });
        ui.end_row();
        ui.label(egui::RichText::new("Simulate").color(MUTED));
        ui.horizontal(|ui| {
            ui.checkbox(&mut f.life, "Life");
            ui.add_enabled(f.life, egui::Checkbox::new(&mut f.civilization, "Civilizations"));
        });
        ui.end_row();
        if f.template == Template::Custom {
            ui.label(egui::RichText::new("Scenario").color(MUTED));
            egui::ComboBox::from_id_salt("scn").selected_text(f.scenario.label()).show_ui(ui, |ui| {
                for sc in Scenario::ALL {
                    ui.selectable_value(&mut f.scenario, sc, sc.label());
                }
            });
            ui.end_row();
            ui.label(egui::RichText::new("Star systems").color(MUTED));
            ui.add(egui::Slider::new(&mut f.systems, 4..=64));
            ui.end_row();
            ui.label(egui::RichText::new("Life").color(MUTED));
            egui::ComboBox::from_id_salt("life").selected_text(LIFE_PRESETS[f.life_preset].0).show_ui(ui, |ui| {
                for (i, p) in LIFE_PRESETS.iter().enumerate() {
                    ui.selectable_value(&mut f.life_preset, i, p.0).on_hover_text(p.3);
                }
            });
            ui.end_row();
            ui.label(egui::RichText::new("Technology pace").color(MUTED));
            ui.add(egui::Slider::new(&mut f.tech_rate, 0.25..=4.0).logarithmic(true));
            ui.end_row();
            ui.label(egui::RichText::new("Resources").color(MUTED));
            ui.add(egui::Slider::new(&mut f.resources, 0.25..=4.0).logarithmic(true));
            ui.end_row();
        }
    });
    ui.add_space(16.0);
    ui.horizontal(|ui| {
        if primary_button(ui, "Create sandbox").clicked() {
            let f = s.form.clone();
            let (_, life, intel, _) = LIFE_PRESETS[f.life_preset];
            let settings = UniverseSettings {
                seed: f.seed,
                scenario: f.scenario,
                system_count: f.systems,
                life_rate: life,
                intelligence_rate: intel,
                tech_rate: f.tech_rate,
                resource_abundance: f.resources,
                physics: f.nbody.then(|| f.preset.settings()),
                systems: EnabledSystems { life: f.life, civilization: f.life && f.civilization },
            };
            let name = if f.name.trim().is_empty() { f.template.title().to_string() } else { f.name.trim().to_string() };
            begin_new(commands, next, NewSandbox { settings, advance_years: 0.0, name, description: f.description.clone(), template: f.template.id().into(), what_if: None, persist: true });
        }
        if ui.button("Cancel").clicked() {
            s.menu = MenuScreen::Home;
        }
    });
}

fn load_screen(ui: &mut egui::Ui, s: &mut UiState, thumbs: &Thumbs, commands: &mut Commands, next: &mut NextState<AppState>) {
    heading(ui, "Your sandboxes", "Every experiment is saved in its own folder with autosaves and checkpoints.");
    if s.sandboxes.is_empty() {
        ui.label(egui::RichText::new("No sandboxes yet. Create one with NEW SANDBOX.").color(MUTED));
    }
    let mut refresh = false;
    for e in s.sandboxes.clone() {
        let m = &e.manifest;
        egui::Frame::new().fill(egui::Color32::from_rgb(13, 17, 27)).stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(38, 46, 64))).corner_radius(8).inner_margin(12).show(ui, |ui| {
            ui.set_width(620.0);
            ui.horizontal(|ui| {
                let size = egui::vec2(176.0, 99.0);
                match thumbs.0.get(&m.id).and_then(|t| t.as_ref()) {
                    Some((_, id)) => {
                        ui.add(egui::Image::new(egui::load::SizedTexture::new(*id, size)).corner_radius(4));
                    }
                    None => {
                        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
                        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_rgb(20, 26, 40));
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "no preview", egui::FontId::proportional(11.0), MUTED);
                    }
                }
                ui.vertical(|ui| {
                    if s.rename.as_ref().is_some_and(|(id, _)| id == &m.id) {
                        let (_, buf) = s.rename.as_mut().unwrap();
                        let r = ui.add(egui::TextEdit::singleline(buf).desired_width(300.0));
                        if r.lost_focus() {
                            let mut mm = m.clone();
                            let name = buf.trim().to_string();
                            if !name.is_empty() {
                                let _ = persistence::rename_sandbox(&mut mm, &name);
                            }
                            s.rename = None;
                            refresh = true;
                        }
                    } else {
                        ui.label(egui::RichText::new(&m.name).size(16.0).strong().color(TEXT));
                    }
                    if !m.description.is_empty() {
                        ui.label(egui::RichText::new(&m.description).size(11.5).color(MUTED));
                    }
                    ui.label(egui::RichText::new(format!("{} · {} · {} gravity{} · edited {}", m.sim_date, m.template.replace('_', " "), m.physics.model, if m.edits > 0 { format!(" · {} changes", m.edits) } else { String::new() }, ago(m.modified_unix))).size(11.0).color(MUTED));
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::new(egui::RichText::new("Open").strong()).fill(egui::Color32::from_rgb(70, 54, 24))).clicked() {
                            begin_open_sandbox(commands, next, m.clone(), None);
                        }
                        if ui.button("Duplicate").on_hover_text("A separate copy you can take in another direction (a branch).").clicked() {
                            let _ = persistence::duplicate_sandbox(m, &format!("{} (copy)", m.name));
                            refresh = true;
                        }
                        if ui.button("Rename").clicked() {
                            s.rename = Some((m.id.clone(), m.name.clone()));
                        }
                        if !m.checkpoints.is_empty() && ui.button(format!("Checkpoints ({})", m.checkpoints.len())).clicked() {
                            s.open_checkpoints = if s.open_checkpoints.as_deref() == Some(&m.id) { None } else { Some(m.id.clone()) };
                        }
                        if s.confirm_delete.as_deref() == Some(&m.id) {
                            if ui.add(egui::Button::new(egui::RichText::new("Delete permanently").color(DANGER))).clicked() {
                                let _ = persistence::delete_sandbox(m);
                                s.confirm_delete = None;
                                refresh = true;
                            }
                            if ui.button("Keep").clicked() {
                                s.confirm_delete = None;
                            }
                        } else if ui.button("Delete…").clicked() {
                            s.confirm_delete = Some(m.id.clone());
                        }
                    });
                    if s.open_checkpoints.as_deref() == Some(&m.id) {
                        for c in m.checkpoints.iter().rev() {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("· {} — {}", c.label, c.sim_date)).size(11.5));
                                if ui.small_button("Open").clicked() {
                                    begin_open_sandbox(commands, next, m.clone(), Some(m.dir().join(&c.file)));
                                }
                            });
                        }
                    }
                });
            });
        });
        ui.add_space(8.0);
    }
    if !s.saves.is_empty() {
        ui.add_space(14.0);
        ui.label(egui::RichText::new("FROM EARLIER VERSIONS").size(11.0).color(ACCENT).strong());
        for sv in s.saves.clone() {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(format!("{} — {} · {}", sv.header.name, sv.header.scenario, sv.header.date)).size(12.0));
                if ui.small_button("Import").on_hover_text("Opens it; it becomes a sandbox the first time you save.").clicked() {
                    begin_load_legacy(commands, next, sv.path.clone());
                }
            });
        }
    }
    if refresh {
        s.sandboxes = list_sandboxes();
    }
}

fn scenarios(ui: &mut egui::Ui, s: &mut UiState, commands: &mut Commands, next: &mut NextState<AppState>) {
    heading(ui, "Scenarios", "Curated experiments. Each is an ordinary sandbox set up with the same tools you have — nothing is scripted.");
    for w in WHAT_IFS {
        let r = card(ui, false, |ui| {
            ui.set_width(610.0);
            ui.label(egui::RichText::new(w.title).size(15.0).strong().color(TEXT));
            ui.label(egui::RichText::new(w.description).size(11.5).color(MUTED));
        });
        if r.on_hover_text("Click to create this sandbox").clicked() {
            let settings = UniverseSettings { seed: 1, scenario: Scenario::SolarSystemLab, physics: Some(PhysicsPreset::Balanced.settings()), ..Default::default() };
            begin_new(commands, next, NewSandbox { settings, advance_years: 0.0, name: w.title.trim_end_matches('?').to_string(), description: w.description.into(), template: "solar_system_lab".into(), what_if: Some(w.id.into()), persist: true });
        }
        ui.add_space(6.0);
    }
    ui.add_space(10.0);
    ui.label(egui::RichText::new("STORY STARTS").size(11.0).color(ACCENT).strong());
    for t in [Template::DawnOfHumanity, Template::HabitableWorld, Template::ProceduralNeighbourhood] {
        let r = card(ui, false, |ui| {
            ui.set_width(610.0);
            ui.label(egui::RichText::new(t.title()).size(15.0).strong().color(TEXT));
            ui.label(egui::RichText::new(t.description()).size(11.5).color(MUTED));
        });
        if r.clicked() {
            s.form.pick(t);
            s.menu = MenuScreen::NewSandbox;
        }
        ui.add_space(6.0);
    }
}

fn credits(ui: &mut egui::Ui) {
    heading(ui, "Credits", "Cosmogon is free software (MIT or Apache-2.0).");
    for (title, body) in [
        ("Solar System data", "NASA/JPL Horizons On-Line Ephemeris System — planetary ephemeris DE441 and JPL satellite solutions. Physical parameters: NASA Goddard planetary fact sheets."),
        ("Earth relief", "NOAA National Centers for Environmental Information — ETOPO5 global relief (public domain)."),
        ("Physical constants", "IAU 2012/2015 resolutions; CODATA 2018."),
        ("Science", "Impact scaling after Collins, Melosh & Marcus (2005); climate effects after Toon et al. (1997) and Brugger et al. (2017); integrator after Yoshida (1990). Full references in docs/PHYSICS_ENGINE.md."),
        ("Engine", "Built with Rust, Bevy, wgpu and egui."),
    ] {
        ui.label(egui::RichText::new(title.to_uppercase()).size(11.0).color(ACCENT).strong());
        ui.label(egui::RichText::new(body).size(12.5).color(TEXT));
        ui.add_space(10.0);
    }
}

pub fn settings_ui(ui: &mut egui::Ui, settings: &mut UserSettings) {
    ui.label(egui::RichText::new("Settings").size(22.0).strong());
    ui.add_space(8.0);
    egui::Grid::new("settings").num_columns(2).spacing([14.0, 9.0]).show(ui, |ui| {
        ui.label("Graphics");
        ui.horizontal(|ui| {
            for g in GraphicsPreset::ALL {
                ui.selectable_value(&mut settings.graphics, g, g.label());
            }
        });
        ui.end_row();
        ui.label("Interface scale");
        ui.add(egui::Slider::new(&mut settings.ui_scale, 0.75..=1.75));
        ui.end_row();
        ui.label("Interface mode");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut settings.advanced, false, "Simple");
            ui.selectable_value(&mut settings.advanced, true, "Advanced");
        });
        ui.end_row();
        use super::units::{units, Quantity};
        for (label, q, value) in [("Mass unit", Quantity::Mass, &mut settings.mass_unit), ("Distance unit", Quantity::Length, &mut settings.distance_unit), ("Speed unit", Quantity::Speed, &mut settings.speed_unit), ("Temperature unit", Quantity::Temperature, &mut settings.temperature_unit)] {
            ui.label(label);
            egui::ComboBox::from_id_salt(label).selected_text(units(q)[(*value).min(units(q).len() - 1)].label).show_ui(ui, |ui| {
                for (i, u) in units(q).iter().enumerate() {
                    ui.selectable_value(value, i, u.label);
                }
            });
            ui.end_row();
        }
        ui.label("Autosave (minutes)");
        ui.add(egui::Slider::new(&mut settings.autosave_minutes, 0.0..=30.0));
        ui.end_row();
        ui.label("Slow down for milestones");
        ui.checkbox(&mut settings.auto_slow, "");
        ui.end_row();
        ui.label("Intro animation");
        ui.checkbox(&mut settings.intro, "");
        ui.end_row();
    });
    ui.label(egui::RichText::new("Graphics quality never changes simulation results.").size(11.0).color(MUTED));
}

pub fn generating(mut contexts: EguiContexts, pending: Option<Res<PendingUniverse>>, time: Res<Time>) -> Result {
    let ctx = contexts.ctx_mut()?;
    let label = pending.map(|p| p.label.clone()).unwrap_or_default();
    egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 150))).show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.4);
            ui.label(egui::RichText::new("COSMOGON").size(32.0).strong().extra_letter_spacing(10.0));
            ui.add_space(14.0);
            ui.add(egui::Spinner::new().size(26.0).color(ACCENT));
            ui.add_space(12.0);
            ui.label(egui::RichText::new(label).color(MUTED));
            let dots = ".".repeat((time.elapsed_secs() * 2.0) as usize % 4);
            ui.label(egui::RichText::new(format!("stars ignite, planets cool, chemistry begins{dots}")).size(11.5).color(MUTED).italics());
        });
    });
    Ok(())
}

/// Manifest helper for the pause menu's "Save as" (a new sandbox from the current state).
pub fn save_as(m: &Manifest, name: &str) -> Manifest {
    let mut copy = Manifest::new(name, &m.description, &m.template, m.origin.clone());
    copy.origin.parent_sandbox = Some(m.id.clone());
    copy
}
