//! User interface (egui). The UI only *reads* simulation state and issues commands
//! (select, focus, speed, save…); it never mutates the simulation model directly.

mod charts;
mod hud;
mod inspect;
mod markers;
mod menu;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

use crate::sim::Sim;
use crate::state::AppState;

pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(232, 176, 75);
pub const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(150, 112, 50);
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(214, 220, 232);
pub const MUTED: egui::Color32 = egui::Color32::from_rgb(130, 140, 160);
pub const LIFE: egui::Color32 = egui::Color32::from_rgb(110, 210, 140);
pub const CIV: egui::Color32 = egui::Color32::from_rgb(255, 196, 92);
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(235, 100, 90);
pub const PANEL: egui::Color32 = egui::Color32::from_rgba_premultiplied(9, 12, 20, 232);

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftTab {
    #[default]
    Systems,
    Chronicle,
    Civilizations,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum BodyTab {
    #[default]
    Overview,
    Orbit,
    Environment,
    Life,
    Civilization,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum MenuScreen {
    #[default]
    Home,
    NewUniverse,
    Load,
    Settings,
}

#[derive(Resource)]
pub struct UiState {
    pub show_left: bool,
    pub show_right: bool,
    pub hidden: bool,
    pub debug: bool,
    pub help: bool,
    pub pause_menu: bool,
    pub left_tab: LeftTab,
    pub body_tab: BodyTab,
    pub search: String,
    pub chronicle_min_importance: u8,
    pub menu: MenuScreen,
    pub new_universe: cosmogon_sim::UniverseSettings,
    pub life_preset: usize,
    pub saves: Vec<crate::persistence::SaveEntry>,
    pub tech_filter_blocked: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_left: true,
            show_right: true,
            hidden: false,
            debug: false,
            help: false,
            pause_menu: false,
            left_tab: LeftTab::Systems,
            body_tab: BodyTab::Overview,
            search: String::new(),
            chronicle_min_importance: 4,
            menu: MenuScreen::Home,
            new_universe: cosmogon_sim::UniverseSettings { seed: 2026, scenario: cosmogon_sim::Scenario::GardenWorld, ..Default::default() },
            life_preset: 0,
            saves: Vec::new(),
            tech_filter_blocked: false,
        }
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .add_systems(Startup, init_ui_state)
            .add_systems(EguiPrimaryContextPass, apply_theme)
            .add_systems(EguiPrimaryContextPass, (menu::main_menu).after(apply_theme).run_if(in_state(AppState::MainMenu)))
            .add_systems(EguiPrimaryContextPass, (menu::generating).after(apply_theme).run_if(in_state(AppState::Generating)))
            .add_systems(
                EguiPrimaryContextPass,
                (markers::draw_markers, hud::keyboard, hud::top_bar, hud::bottom_bar, inspect::left_panel, inspect::right_panel, hud::toasts, hud::overlays_windows)
                    .chain()
                    .after(apply_theme)
                    .run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)),
            )
            .add_plugins(menu::MenuScenePlugin);
    }
}

fn init_ui_state(mut ui: ResMut<UiState>, args: Res<crate::args::Args>) {
    ui.hidden = args.hide_ui;
    ui.debug = args.debug;
    if args.select_civ {
        ui.body_tab = BodyTab::Civilization;
    }
}

fn apply_theme(mut contexts: EguiContexts, mut done: Local<Option<f32>>, settings: Res<crate::persistence::UserSettings>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if *done == Some(settings.ui_scale) {
        return Ok(());
    }
    *done = Some(settings.ui_scale);
    ctx.set_zoom_factor(settings.ui_scale);
    let mut style = (*ctx.style()).clone();
    use egui::{FontFamily::*, FontId, TextStyle};
    style.text_styles = [
        (TextStyle::Heading, FontId::new(19.0, Proportional)),
        (TextStyle::Body, FontId::new(13.5, Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, Monospace)),
        (TextStyle::Button, FontId::new(13.5, Proportional)),
        (TextStyle::Small, FontId::new(11.0, Proportional)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = egui::Color32::from_rgba_premultiplied(12, 16, 27, 245);
    v.window_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(40, 50, 70));
    v.extreme_bg_color = egui::Color32::from_rgb(6, 8, 14);
    v.faint_bg_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 6);
    v.selection.bg_fill = egui::Color32::from_rgb(92, 70, 32);
    v.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    v.hyperlink_color = ACCENT;
    v.override_text_color = Some(TEXT);
    let r = egui::CornerRadius::same(6);
    v.window_corner_radius = egui::CornerRadius::same(10);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = r;
    }
    v.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(24, 31, 48);
    v.widgets.inactive.bg_fill = egui::Color32::from_rgb(24, 31, 48);
    v.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(38, 48, 72);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, ACCENT_DIM);
    v.widgets.active.weak_bg_fill = egui::Color32::from_rgb(70, 56, 30);
    ctx.set_style(style);
    Ok(())
}

/// Section heading in the house style.
pub fn heading(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(egui::RichText::new(text.to_uppercase()).size(11.0).color(ACCENT).strong());
}

pub fn kv(ui: &mut egui::Ui, k: &str, v: impl Into<egui::WidgetText>) {
    ui.label(egui::RichText::new(k).color(MUTED));
    ui.label(v);
    ui.end_row();
}

/// Format a large count compactly: 1.2 M, 3.4 bn.
pub fn compact(n: f64) -> String {
    let a = n.abs();
    if a >= 1e12 {
        format!("{:.2} tn", n / 1e12)
    } else if a >= 1e9 {
        format!("{:.2} bn", n / 1e9)
    } else if a >= 1e6 {
        format!("{:.2} M", n / 1e6)
    } else if a >= 1e4 {
        format!("{:.1} k", n / 1e3)
    } else {
        format!("{n:.0}")
    }
}

pub fn power(w: f64) -> String {
    let a = w.abs();
    if a >= 1e15 {
        format!("{:.1} PW", w / 1e15)
    } else if a >= 1e12 {
        format!("{:.1} TW", w / 1e12)
    } else if a >= 1e9 {
        format!("{:.1} GW", w / 1e9)
    } else if a >= 1e6 {
        format!("{:.1} MW", w / 1e6)
    } else {
        format!("{:.0} kW", w / 1e3)
    }
}

pub fn distance(m: f64) -> String {
    use cosmogon_sim::astro::{AU, LIGHT_YEAR};
    if m >= 0.05 * LIGHT_YEAR {
        format!("{:.2} ly", m / LIGHT_YEAR)
    } else if m >= 0.01 * AU {
        format!("{:.3} AU", m / AU)
    } else if m >= 1e4 {
        format!("{} km", cosmogon_sim::time::group_digits(m / 1000.0))
    } else {
        format!("{m:.0} m")
    }
}
