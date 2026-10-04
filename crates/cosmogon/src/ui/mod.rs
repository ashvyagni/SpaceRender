//! User interface (egui). The UI only *reads* simulation state and issues commands
//! (select, focus, speed, save…); it never mutates the simulation model directly.

mod charts;
mod dock;
pub mod home;
mod hud;
mod inspect;
mod markers;
mod tools;
pub mod tour;
pub mod units;

pub use home::MenuScreen;

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
pub const PANEL: egui::Color32 = egui::Color32::from_rgb(9, 12, 20);

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
    Physics,
    Environment,
    Life,
    Civilization,
    History,
    Data,
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
    pub form: home::NewForm,
    pub saves: Vec<crate::persistence::SaveEntry>,
    pub sandboxes: Vec<crate::persistence::SandboxEntry>,
    pub rename: Option<(String, String)>,
    pub confirm_delete: Option<String>,
    pub open_checkpoints: Option<String>,
    pub intro_started: Option<f64>,
    pub intro_skipped: bool,
    pub tech_filter_blocked: bool,
    pub share_domain: u8,
    /// Sandbox tool windows.
    pub create_open: bool,
    pub create: tools::CreateForm,
    pub physics_open: bool,
    pub palette_open: bool,
    pub palette_query: String,
    pub palette_index: usize,
    pub clone_dialog: Option<String>,
    pub save_as_dialog: Option<String>,
    pub checkpoint_label: String,
    pub impulse: [f64; 3],
    /// Set by the photo button / P; forwarded to `capture::Photo`.
    pub photo_requested: bool,
    /// A photo is being taken: draw nothing but the universe.
    pub photo_mode: bool,
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
            form: home::NewForm::default(),
            saves: Vec::new(),
            sandboxes: Vec::new(),
            rename: None,
            confirm_delete: None,
            open_checkpoints: None,
            intro_started: None,
            intro_skipped: false,
            tech_filter_blocked: false,
            share_domain: 0,
            create_open: false,
            create: tools::CreateForm::default(),
            physics_open: false,
            palette_open: false,
            palette_query: String::new(),
            palette_index: 0,
            clone_dialog: None,
            save_as_dialog: None,
            checkpoint_label: String::new(),
            impulse: [0.0; 3],
            photo_requested: false,
            photo_mode: false,
        }
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .add_systems(Startup, init_ui_state)
            .add_systems(EguiPrimaryContextPass, apply_theme)
            .add_systems(EguiPrimaryContextPass, (home::main_menu).after(apply_theme).run_if(in_state(AppState::MainMenu)))
            .add_systems(EguiPrimaryContextPass, (home::generating).after(apply_theme).run_if(in_state(AppState::Generating)))
            .add_systems(
                EguiPrimaryContextPass,
                (markers::draw_markers, hud::keyboard, hud::top_bar, hud::bottom_bar, dock::tool_dock, inspect::left_panel, inspect::right_panel, tools::tool_windows, hud::toasts, hud::overlays_windows, tour::tour_card)
                    .chain()
                    .after(apply_theme)
                    .run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)),
            )
            .add_plugins(home::MenuScenePlugin);
    }
}

fn init_ui_state(mut ui: ResMut<UiState>, args: Res<crate::args::Args>) {
    ui.hidden = args.hide_ui;
    ui.debug = args.debug;
    if args.select_civ {
        ui.body_tab = BodyTab::Civilization;
    }
    ui.menu = match args.menu.as_deref() {
        Some("new") => MenuScreen::NewSandbox,
        Some("load") => MenuScreen::Load,
        Some("scenarios") => MenuScreen::Scenarios,
        Some("settings") => MenuScreen::Settings,
        Some("credits") => MenuScreen::Credits,
        _ => MenuScreen::Home,
    };
    if ui.menu == MenuScreen::Load {
        ui.sandboxes = crate::persistence::list_sandboxes();
        ui.saves = crate::persistence::list_saves();
    }
    if args.menu.is_some() {
        ui.intro_skipped = true;
    }
    match args.panel.as_deref() {
        Some("create") => ui.create_open = true,
        Some("physics") => ui.physics_open = true,
        Some("palette") => ui.palette_open = true,
        _ => {}
    }
    // Automated captures and dev starts skip the intro.
    if args.capture.is_some() || args.new.is_some() || args.load.is_some() {
        ui.intro_skipped = true;
    }
}

/// Icon glyphs (Phosphor, MIT) — use inside any text.
pub use egui_phosphor::regular as icon;

/// Inter for text (SIL Open Font License), Phosphor for icons, egui's fonts as fallbacks.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("inter".into(), std::sync::Arc::new(egui::FontData::from_static(include_bytes!("../../assets/fonts/Inter-Regular.ttf"))));
    if let Some(f) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        f.insert(0, "inter".into());
    }
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    ctx.set_fonts(fonts);
}

fn apply_theme(mut contexts: EguiContexts, mut done: Local<Option<f32>>, settings: Res<crate::persistence::UserSettings>) -> Result {
    let ctx = contexts.ctx_mut()?;
    if *done == Some(settings.ui_scale) {
        return Ok(());
    }
    if done.is_none() {
        install_fonts(ctx);
    }
    *done = Some(settings.ui_scale);
    ctx.set_zoom_factor(settings.ui_scale);
    let mut style = (*ctx.style()).clone();
    use egui::{FontFamily::*, FontId, TextStyle};
    style.text_styles = [
        (TextStyle::Heading, FontId::new(19.0, Proportional)),
        (TextStyle::Body, FontId::new(13.0, Proportional)),
        (TextStyle::Monospace, FontId::new(12.5, Monospace)),
        (TextStyle::Button, FontId::new(13.0, Proportional)),
        (TextStyle::Small, FontId::new(10.5, Proportional)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = egui::Color32::from_rgba_premultiplied(11, 14, 24, 242);
    v.window_stroke = egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(36, 44, 62));
    v.window_shadow = egui::epaint::Shadow { offset: [0, 8], blur: 28, spread: 0, color: egui::Color32::from_black_alpha(140) };
    v.popup_shadow = egui::epaint::Shadow { offset: [0, 6], blur: 18, spread: 0, color: egui::Color32::from_black_alpha(120) };
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
