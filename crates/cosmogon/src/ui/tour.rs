//! Guided tours: short, narrated walks through the program for first-time players.
//!
//! A tour opens an ordinary sandbox (optionally with an experiment) and shows a card with
//! one step at a time. Each step can move the camera, set the clock and select something;
//! the player can still do anything in between. Nothing is pre-recorded: what you see is
//! the live simulation.

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use super::{icon, UiState, ACCENT, ACCENT_DIM, MUTED, PANEL, TEXT};
use crate::camera::{find_target, CameraRig};
use crate::sim::Sim;

pub struct TourStep {
    pub title: &'static str,
    pub body: &'static str,
    /// Fly to this object (name) …
    pub focus: Option<&'static str>,
    /// … ending this many of its radii away.
    pub distance: Option<f64>,
    /// Clock speed index (`cosmogon_sim::time::SPEEDS`); `None` keeps the current one.
    pub speed: Option<usize>,
    /// Open the inspector on the focused object.
    pub select: bool,
}

pub struct Tour {
    pub id: &'static str,
    pub title: &'static str,
    pub blurb: &'static str,
    /// Experiment the tour's sandbox starts with (`cosmogon_sim::sandbox::WHAT_IFS`).
    pub what_if: Option<&'static str>,
    pub steps: &'static [TourStep],
}

const fn step(title: &'static str, body: &'static str, focus: Option<&'static str>, distance: Option<f64>, speed: Option<usize>, select: bool) -> TourStep {
    TourStep { title, body, focus, distance, speed, select }
}

pub const TOURS: &[Tour] = &[
    Tour {
        id: "solar_system",
        title: "From Earth to the cosmic web",
        blurb: "Meet the real Solar System, then pull back past the Milky Way to the cosmic web. Three minutes.",
        what_if: None,
        steps: &[
            step("Welcome to Cosmogon", "This is the real Solar System, placed where NASA/JPL says it is today. Drag to orbit the view, scroll to zoom, and click anything to inspect it. Press Next when you're ready.", Some("Sun"), None, Some(0), false),
            step("Earth", "Eight billion people. The coloured dots are cities, tinted by country; the panel on the right follows humanity's technology, energy and space programme. The bright specks around the planet are satellites.", Some("Earth"), Some(3.2), Some(1), true),
            step("The Moon", "Every world, moon and star here has measured relief or a procedurally generated surface — zoom all the way down to walk its craters.", Some("Moon"), Some(3.0), None, true),
            step("Jupiter", "Time runs at one hour per second now: watch the bands drift and the Galilean moons move. Use the slider at the bottom left to go faster — up to a hundred million years per second.", Some("Jupiter"), Some(5.0), Some(2), true),
            step("Saturn", "Its rings are thousands of ringlets with real gaps. Moons and rings cast shadows on the planet.", Some("Saturn"), Some(5.0), None, true),
            step("The whole system", "Pulling back: the orbits of all eight planets. Every one of them moves under real gravity — and you can change it.", Some("Sun"), Some(2.0e4), Some(3), false),
            step("Where stars are born", "1,344 light-years away: the Orion Nebula, a cloud of glowing hydrogen lit by the four young Trapezium stars. Run time at a million years per second and it forms new stars — the most massive will explode within a few million years.", Some("Theta1 Orionis C"), Some(5.0e7), Some(0), false),
            step("The Milky Way", "150,000 light-years out. The Sun sits in a spiral arm 26,700 light-years from the centre, where a black hole four million times the Sun's mass hides behind the bar. The two small smudges are the Magellanic Clouds.", Some("Sun"), Some(2.0e12), Some(0), false),
            step("The cosmic web", "Further still: the Local Group and thousands of galaxies strung along filaments, with quasars blazing in the far distance.", Some("Sun"), Some(5.0e13), None, false),
            step("Your turn", "Head home with the search bar (try “Earth”). Then pick the Throw tool in the dock, choose an object on the shelf, and drag in space to fling it. Experiments has curated what-ifs to start from.", Some("Earth"), Some(4.0), None, false),
        ],
    },
    Tour {
        id: "black_hole",
        title: "A black hole passes through",
        blurb: "A 10 M☉ black hole falls into the Solar System. See light bend around it — and what it does to the planets.",
        what_if: Some("black_hole_flyby"),
        steps: &[
            step("The Wanderer", "A black hole of ten solar masses, three hundred thousand times the Sun's, is falling in from 60 AU at 15 km/s. Its horizon is only 60 km across, but look at how it bends the starlight behind it.", Some("Wanderer"), Some(4.0e5), Some(0), true),
            step("Gravitational lensing", "Light grazing the hole is bent into an Einstein ring; the black disc is the shadow of the event horizon, two and a half times wider than the horizon itself. No accretion disk: this hole has nothing to feed on.", Some("Wanderer"), Some(6.0e4), None, false),
            step("Speeding up", "Time now runs at a year per second. The hole needs about two decades to arrive. Watch the orbit lines: they redraw as the planets feel its pull.", Some("Sun"), Some(9.0e3), Some(5), false),
            step("Aftermath", "Inspect the planets as it passes — orbits stretched, flung outward, or captured. The Chronicle (left panel) records every ejection and collision. Nothing is scripted: each run can differ.", Some("Sun"), Some(9.0e3), None, false),
            step("Your turn", "Pick Throw in the dock, choose a black hole on the shelf and drag anywhere to throw your own. Or press Experiments for more what-ifs.", None, None, None, false),
        ],
    },
    Tour {
        id: "dying_sun",
        title: "The death of the Sun",
        blurb: "7.6 billion years from now: the Sun is a red giant swelling towards Earth's orbit. Watch it end as a white dwarf.",
        what_if: Some("sun_red_giant"),
        steps: &[
            step("A red giant", "The Sun has burnt the hydrogen in its core. Its outer layers have swollen to over a hundred times today's size, and it shines two thousand times brighter, with a cool, red surface.", Some("Sun"), None, Some(0), true),
            step("Mercury and Venus", "The inner planets are inside the reach of the growing star. As the Sun loses mass its grip weakens and the surviving orbits widen — a race between the two.", Some("Sun"), Some(250.0), None, false),
            step("Earth", "Oceans long gone, the surface molten. Whether Earth is swallowed depends on how much mass the Sun sheds first.", Some("Earth"), Some(4.0), None, true),
            step("Fast forward", "A million years per second. The Sun will puff off its envelope as a glowing planetary nebula and leave a white dwarf the size of Earth.", Some("Sun"), Some(400.0), Some(11), false),
            step("Your turn", "Select the Sun and use the inspector to set its age or mass — or try “What if the Sun were a dying supergiant?” in Experiments for a supernova.", None, None, None, false),
        ],
    },
    Tour {
        id: "halley",
        title: "Halley's Comet",
        blurb: "Comet 1P/Halley on its real orbit, a month before perihelion. Its tails grow as it nears the Sun.",
        what_if: Some("halley"),
        steps: &[
            step("1P/Halley", "A 15-km nucleus of ice and dust on a 76-year orbit that runs backwards around the Sun. Sunlight is boiling its ices away.", Some("Halley"), Some(1.5e7), Some(0), true),
            step("Two tails", "The straight blue ion tail is gas blown directly away from the Sun by the solar wind. The broad, curved yellow tail is dust left behind along the orbit.", Some("Halley"), Some(6.0e6), None, false),
            step("Perihelion", "One day per second: the comet whips round the Sun at 55 km/s and its tails swing with it, always pointing away from the Sun.", Some("Halley"), Some(2.5e7), Some(3), false),
            step("Your turn", "Throw a comet of your own from the shelf in the dock — any icy body warmed by a star grows tails.", None, None, None, false),
        ],
    },
];

/// The tour being shown.
#[derive(Resource)]
pub struct ActiveTour {
    pub tour: usize,
    pub step: usize,
    /// The step whose camera/clock moves have been applied.
    pub applied: Option<usize>,
}

pub fn tour_by_id(id: &str) -> Option<usize> {
    TOURS.iter().position(|t| t.id == id)
}

/// Apply the current step's moves once, and draw the card.
pub fn tour_card(mut commands: Commands, mut contexts: EguiContexts, tour: Option<ResMut<ActiveTour>>, mut sim: ResMut<Sim>, mut rig: ResMut<CameraRig>, ui_state: Res<UiState>) -> Result {
    let Some(mut tour) = tour else { return Ok(()) };
    let t = &TOURS[tour.tour];
    let n = t.steps.len();
    tour.step = tour.step.min(n - 1);
    let s = &t.steps[tour.step];
    if tour.applied != Some(tour.step) {
        tour.applied = Some(tour.step);
        if let Some(name) = s.focus {
            if let Some(target) = find_target(&sim, name) {
                let radii = s.distance;
                rig.focus_on(target, &sim, radii);
                if s.select {
                    sim.selected = Some(target);
                }
            }
        }
        if let Some(v) = s.speed {
            sim.speed = v.min(cosmogon_sim::time::SPEEDS.len() - 1);
            sim.paused = false;
        }
    }
    if ui_state.hidden {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let mut close = false;
    // Bottom-left of the free 3D view (between the side panels), clear of the tool dock.
    let free = ctx.available_rect();
    egui::Area::new(egui::Id::new("tour")).pivot(egui::Align2::LEFT_BOTTOM).fixed_pos(egui::pos2(free.left() + 16.0, free.bottom() - 120.0)).show(ctx, |ui| {
        egui::Frame::new()
            .fill(PANEL.gamma_multiply(0.97))
            .stroke(egui::Stroke::new(1.0_f32, ACCENT_DIM))
            .corner_radius(12)
            .inner_margin(16)
            .shadow(egui::epaint::Shadow { offset: [0, 6], blur: 24, spread: 0, color: egui::Color32::from_black_alpha(140) })
            .show(ui, |ui| {
                ui.set_width(380.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon::COMPASS).size(15.0).color(ACCENT));
                    ui.label(egui::RichText::new(t.title.to_uppercase()).size(10.5).color(ACCENT).extra_letter_spacing(1.2));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new(egui::RichText::new(icon::X).color(MUTED)).frame(false)).on_hover_text("End tour").clicked() {
                            close = true;
                        }
                    });
                });
                ui.add_space(4.0);
                ui.label(egui::RichText::new(s.title).size(18.0).strong().color(TEXT));
                ui.add_space(2.0);
                ui.label(egui::RichText::new(s.body).size(13.0).color(TEXT.gamma_multiply(0.92)));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    // Progress pips.
                    for i in 0..n {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(if i == tour.step { 16.0 } else { 6.0 }, 6.0), egui::Sense::hover());
                        ui.painter().rect_filled(r, 3.0, if i <= tour.step { ACCENT } else { egui::Color32::from_rgb(45, 52, 70) });
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let last = tour.step + 1 == n;
                        let next = egui::Button::new(egui::RichText::new(if last { "Finish".to_string() } else { format!("Next {}", icon::ARROW_RIGHT) }).color(egui::Color32::from_rgb(20, 16, 8)).strong())
                            .fill(ACCENT)
                            .corner_radius(8)
                            .min_size(egui::vec2(84.0, 30.0));
                        if ui.add(next).clicked() {
                            if last {
                                close = true;
                            } else {
                                tour.step += 1;
                            }
                        }
                        if tour.step > 0 && ui.add(egui::Button::new(egui::RichText::new(icon::ARROW_LEFT).color(MUTED)).corner_radius(8).min_size(egui::vec2(34.0, 30.0))).on_hover_text("Back").clicked() {
                            tour.step -= 1;
                        }
                    });
                });
            });
    });
    if close {
        commands.remove_resource::<ActiveTour>();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tours_use_real_experiments_and_valid_speeds() {
        for t in TOURS {
            if let Some(w) = t.what_if {
                assert!(cosmogon_sim::sandbox::WHAT_IFS.iter().any(|x| x.id == w), "{}: unknown experiment {w}", t.id);
            }
            assert!(!t.steps.is_empty());
            for s in t.steps {
                if let Some(v) = s.speed {
                    assert!(v < cosmogon_sim::time::SPEEDS.len(), "{}: speed {v}", t.id);
                }
            }
        }
    }

    #[test]
    fn tour_targets_exist_in_their_sandbox() {
        for t in TOURS {
            let mut u = cosmogon_sim::Universe::new(cosmogon_sim::UniverseSettings { seed: 1, scenario: cosmogon_sim::Scenario::SolarSystemLab, ..Default::default() });
            if let Some(w) = t.what_if {
                cosmogon_sim::sandbox::apply_what_if(&mut u, w).unwrap();
            }
            for s in t.steps {
                if let Some(name) = s.focus {
                    let found = u.systems.iter().any(|sys| sys.star.name.eq_ignore_ascii_case(name) || sys.name.eq_ignore_ascii_case(name) || sys.find_body(name).is_some());
                    assert!(found, "{}: no object called {name}", t.id);
                }
            }
        }
    }
}
