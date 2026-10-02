//! Object inspector panel — shows detailed information about the selected
//! celestial body.

use bevy_ecs::prelude::*;
use cosmogon_core::units;
use cosmogon_ecs::components::celestial::*;
use cosmogon_ecs::components::motion::*;
use cosmogon_ecs::components::orbital::*;
use cosmogon_ecs::components::physics::*;
use cosmogon_ecs::components::transform::*;
use cosmogon_ecs::resources::render_state::SelectedObject;
use egui::Ui;

/// Stateless inspector panel. Call [`InspectorPanel::show`] each frame with the
/// current selection and a query over all celestial entities.
pub struct InspectorPanel;

impl InspectorPanel {
    /// Draw the inspector into `ui` for the currently `selected` entity.
    ///
    /// `query` should cover every celestial-body entity so that the selected
    /// entity's components can be fetched by id.
    pub fn show(
        &self,
        ui: &mut Ui,
        selected: &SelectedObject,
        query: &Query<(
            &BodyName,
            Option<&Mass>,
            Option<&Radius>,
            Option<&OrbitalElements>,
            Option<&Position>,
            Option<&Velocity>,
            Option<&Star>,
            Option<&Planet>,
            Option<&Moon>,
        )>,
    ) {
        ui.heading("Object Inspector");
        ui.separator();

        match &selected.0 {
            Some(entity) => {
                if let Ok((name, mass, radius, orbital, position, velocity, star, planet, moon)) =
                    query.get(*entity)
                {
                    ui.label(format!("Name: {}", name.0));

                    if let Some(mass) = mass {
                        ui.label(format!("Mass: {:.2e} kg", mass.0));
                        ui.label(format!(
                            "Mass: {:.4} Solar masses",
                            units::kg_to_solar_masses(mass.0)
                        ));
                    }

                    if let Some(radius) = radius {
                        ui.label(format!("Radius: {:.2} km", units::meters_to_km(radius.0)));
                    }

                    if let Some(orbital) = orbital {
                        ui.label(format!(
                            "Semi-major axis: {:.4} AU",
                            units::meters_to_au(orbital.semi_major_axis)
                        ));
                        ui.label(format!("Eccentricity: {:.4}", orbital.eccentricity));
                        ui.label(format!(
                            "Inclination: {:.2}\u{00B0}",
                            orbital.inclination.to_degrees()
                        ));
                        ui.label(format!(
                            "Arg. perihelion: {:.2}\u{00B0}",
                            orbital.argument_perihelion.to_degrees()
                        ));
                        ui.label(format!(
                            "Long. ascending: {:.2}\u{00B0}",
                            orbital.longitude_ascending.to_degrees()
                        ));
                    }

                    if let Some(pos) = position {
                        let dist = pos.coords.length();
                        ui.label(format!("Distance from origin: {:.4} AU", units::meters_to_au(dist)));
                        ui.label(format!(
                            "Distance from origin: {:.0} km",
                            units::meters_to_km(dist)
                        ));
                        ui.label(format!(
                            "Position (m): ({:.0}, {:.0}, {:.0})",
                            pos.coords.x, pos.coords.y, pos.coords.z
                        ));
                    }

                    if let Some(vel) = velocity {
                        let speed = vel.linear.length();
                        ui.label(format!("Speed: {:.2} km/s", speed * 1e-3));
                    }

                    // Body type tag
                    if let Some(star) = star {
                        ui.label(format!(
                            "Type: Star (class {}, {:.0} K)",
                            star.spectral_class, star.temperature
                        ));
                    } else if let Some(planet) = planet {
                        let rings = if planet.has_rings { " [rings]" } else { "" };
                        ui.label(format!("Type: Planet{rings}"));
                    } else if moon.is_some() {
                        ui.label("Type: Moon");
                    }
                } else {
                    ui.colored_label(egui::Color32::YELLOW, "Selected entity not found in query");
                }
            }
            None => {
                ui.label("No object selected");
                ui.label("Click on a celestial body to inspect it");
            }
        }
    }
}
