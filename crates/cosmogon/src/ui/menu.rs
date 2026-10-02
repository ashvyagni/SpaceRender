//! Main menu, universe creation, save browser, settings and the loading screen.

use bevy::math::DVec3;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy_egui::{egui, EguiContexts};
use cosmogon_sim::universe::LIFE_PRESETS;
use cosmogon_sim::{Scenario, UniverseSettings};

use super::{UiState, ACCENT, MUTED, TEXT};
use crate::persistence::{list_saves, GraphicsPreset, UserSettings};
use crate::render::materials::{AtmosphereMaterial, AtmosphereUniform, PlanetMaterial, PlanetUniform};
use crate::render::{atmosphere_look, bake, solid_image, SharedMeshes, ViewInfo};
use crate::sim::{begin_load, begin_new, MenuMessage, PendingUniverse};
use crate::state::AppState;

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

fn big_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    ui.add_sized([240.0, 38.0], egui::Button::new(egui::RichText::new(text).size(16.0)))
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
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let ui_s = &mut *ui_state;
    egui::SidePanel::left("menu")
        .exact_width(420.0)
        .resizable(false)
        .frame(egui::Frame::new().fill(egui::Color32::from_rgba_premultiplied(5, 7, 12, 215)).inner_margin(egui::Margin::symmetric(44, 40)))
        .show(ctx, |ui| {
            ui.add_space(30.0);
            ui.label(egui::RichText::new("COSMOGON").size(46.0).color(TEXT).strong().extra_letter_spacing(8.0));
            ui.label(egui::RichText::new("a living universe").size(16.0).color(ACCENT).italics());
            ui.add_space(36.0);
            match ui_s.menu {
                super::MenuScreen::Home => {
                    if big_button(ui, "New universe").clicked() {
                        ui_s.menu = super::MenuScreen::NewUniverse;
                    }
                    ui.add_space(6.0);
                    let saves_exist = !list_saves().is_empty();
                    if ui.add_enabled(saves_exist, egui::Button::new(egui::RichText::new("Continue").size(16.0)).min_size(egui::vec2(240.0, 38.0))).clicked() {
                        if let Some(latest) = list_saves().into_iter().next() {
                            begin_load(&mut commands, &mut next, latest.path);
                        }
                    }
                    ui.add_space(6.0);
                    if big_button(ui, "Load").clicked() {
                        ui_s.saves = list_saves();
                        ui_s.menu = super::MenuScreen::Load;
                    }
                    ui.add_space(6.0);
                    if big_button(ui, "Settings").clicked() {
                        ui_s.menu = super::MenuScreen::Settings;
                    }
                    ui.add_space(6.0);
                    if big_button(ui, "Quit").clicked() {
                        exit.write(AppExit::Success);
                    }
                    if let Some(m) = &msg.0 {
                        ui.add_space(20.0);
                        ui.colored_label(super::DANGER, m);
                    }
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        ui.label(egui::RichText::new(format!("v{} · prototype milestone 1", env!("CARGO_PKG_VERSION"))).size(11.0).color(MUTED));
                    });
                }
                super::MenuScreen::NewUniverse => new_universe(ui, ui_s, &mut commands, &mut next, &mut msg),
                super::MenuScreen::Load => {
                    ui.heading("Load universe");
                    ui.add_space(8.0);
                    egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| {
                        if ui_s.saves.is_empty() {
                            ui.label(egui::RichText::new("No saved universes yet.").color(MUTED));
                        }
                        for s in ui_s.saves.clone() {
                            egui::Frame::group(ui.style()).show(ui, |ui| {
                                ui.set_width(310.0);
                                ui.label(egui::RichText::new(&s.header.name).strong());
                                ui.label(egui::RichText::new(format!("{} · {} · seed {}", s.header.date, s.header.scenario, s.header.seed)).size(11.5).color(MUTED));
                                ui.label(egui::RichText::new(format!("{} living civilization(s) · {}", s.header.civilizations, s.path.file_name().unwrap_or_default().to_string_lossy())).size(11.5).color(MUTED));
                                if ui.button("Load").clicked() {
                                    begin_load(&mut commands, &mut next, s.path.clone());
                                }
                            });
                        }
                    });
                    ui.add_space(10.0);
                    if ui.button("← Back").clicked() {
                        ui_s.menu = super::MenuScreen::Home;
                    }
                }
                super::MenuScreen::Settings => {
                    settings_ui(ui, &mut settings);
                    ui.add_space(10.0);
                    if ui.button("← Back").clicked() {
                        settings.save();
                        ui_s.menu = super::MenuScreen::Home;
                    }
                }
            }
        });
    Ok(())
}

fn new_universe(ui: &mut egui::Ui, s: &mut UiState, commands: &mut Commands, next: &mut NextState<AppState>, msg: &mut MenuMessage) {
    ui.heading("New universe");
    ui.add_space(8.0);
    for sc in Scenario::ALL {
        let selected = s.new_universe.scenario == sc;
        let frame = egui::Frame::group(ui.style()).stroke(egui::Stroke::new(1.0_f32, if selected { ACCENT } else { egui::Color32::from_rgb(40, 50, 70) }));
        let r = frame
            .show(ui, |ui| {
                ui.set_width(310.0);
                ui.label(egui::RichText::new(sc.label()).strong().color(if selected { ACCENT } else { TEXT }));
                ui.label(egui::RichText::new(sc.description()).size(11.5).color(MUTED));
            })
            .response
            .interact(egui::Sense::click());
        if r.clicked() {
            s.new_universe.scenario = sc;
        }
    }
    ui.add_space(10.0);
    egui::Grid::new("newu").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        ui.label("Seed");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut s.new_universe.seed).speed(1.0));
            if ui.button("🎲").on_hover_text("Random seed").clicked() {
                s.new_universe.seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64 % 1_000_000).unwrap_or(1);
            }
        });
        ui.end_row();
        ui.label("Star systems");
        ui.add(egui::Slider::new(&mut s.new_universe.system_count, 4..=64));
        ui.end_row();
        ui.label("Life");
        egui::ComboBox::from_id_salt("life").selected_text(LIFE_PRESETS[s.life_preset].0).show_ui(ui, |ui| {
            for (i, p) in LIFE_PRESETS.iter().enumerate() {
                ui.selectable_value(&mut s.life_preset, i, p.0).on_hover_text(p.3);
            }
        });
        ui.end_row();
        ui.label("Technology pace");
        ui.add(egui::Slider::new(&mut s.new_universe.tech_rate, 0.25..=4.0).logarithmic(true));
        ui.end_row();
        ui.label("Resources");
        ui.add(egui::Slider::new(&mut s.new_universe.resource_abundance, 0.25..=4.0).logarithmic(true));
        ui.end_row();
    });
    ui.label(egui::RichText::new(LIFE_PRESETS[s.life_preset].3).size(11.5).color(MUTED));
    ui.add_space(14.0);
    ui.horizontal(|ui| {
        if ui.button("← Back").clicked() {
            s.menu = super::MenuScreen::Home;
        }
        if ui.add(egui::Button::new(egui::RichText::new("Create universe").strong()).fill(egui::Color32::from_rgb(90, 66, 24))).clicked() {
            let (_, life, intel, _) = LIFE_PRESETS[s.life_preset];
            let mut settings = s.new_universe.clone();
            settings.life_rate = life;
            settings.intelligence_rate = intel;
            msg.0 = None;
            begin_new(commands, next, settings, 0.0);
        }
    });
}

pub fn settings_ui(ui: &mut egui::Ui, settings: &mut UserSettings) {
    ui.heading("Settings");
    ui.add_space(8.0);
    egui::Grid::new("settings").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
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
        ui.label("Autosave (minutes)");
        ui.add(egui::Slider::new(&mut settings.autosave_minutes, 0.0..=30.0));
        ui.end_row();
        ui.label("Slow down for milestones");
        ui.checkbox(&mut settings.auto_slow, "");
        ui.end_row();
    });
    ui.label(egui::RichText::new("Graphics quality never changes simulation results.").size(11.0).color(MUTED));
}

pub fn generating(mut contexts: EguiContexts, pending: Option<Res<PendingUniverse>>, time: Res<Time>) -> Result {
    let ctx = contexts.ctx_mut()?;
    let label = pending.map(|p| p.label.clone()).unwrap_or_default();
    egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::from_rgba_premultiplied(0, 0, 0, 120))).show(ctx, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.4);
            ui.label(egui::RichText::new("COSMOGON").size(30.0).strong().extra_letter_spacing(6.0));
            ui.add_space(10.0);
            ui.add(egui::Spinner::new().size(26.0).color(ACCENT));
            ui.add_space(10.0);
            ui.label(egui::RichText::new(label).color(MUTED));
            let dots = ".".repeat((time.elapsed_secs() * 2.0) as usize % 4);
            ui.label(egui::RichText::new(format!("stars ignite, planets cool, chemistry begins{dots}")).size(11.5).color(MUTED).italics());
        });
    });
    Ok(())
}
