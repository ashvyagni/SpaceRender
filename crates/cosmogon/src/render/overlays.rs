//! Line overlays drawn with gizmos: orbits of the focused system and expanding radio
//! spheres of civilizations that broadcast.

use bevy::math::DVec3;
use bevy::prelude::*;
use cosmogon_sim::astro::SPEED_OF_LIGHT;

use super::{to_render, ViewInfo};
use crate::camera::CameraRig;
use crate::persistence::UserSettings;
use crate::sim::Sim;
use crate::state::{AppState, Frame};

pub struct OverlayPlugin;

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (draw_orbits, draw_radio_spheres).after(super::apply_origin).in_set(Frame::Apply).run_if(in_state(AppState::Observing).and(resource_exists::<Sim>)));
    }
}

fn draw_orbits(mut gizmos: Gizmos, sim: Res<Sim>, view: Res<ViewInfo>, rig: Res<CameraRig>, settings: Res<UserSettings>) {
    if !settings.show_orbits {
        return;
    }
    let u = &sim.universe;
    let t = u.time;
    let Some(focus) = rig.focus else { return };
    let sys = u.system(focus.system());
    let star = to_render(sys.position);
    let cam_dist = (view.origin - star).length();
    for (i, b) in sys.bodies.iter().enumerate() {
        let parent_pos = match b.parent {
            Some(p) => to_render(sys.body_position(p as usize, t)),
            None => star,
        };
        // Moons' orbits only when close enough for them to be distinguishable.
        let extent = b.orbit.apoapsis();
        let parent_dist = (view.origin - parent_pos).length();
        if b.parent.is_some() && parent_dist > extent * 60.0 {
            continue;
        }
        if b.parent.is_none() && cam_dist > extent * 4000.0 {
            continue;
        }
        let selected = sim.selected == Some(crate::sim::Target::Body(cosmogon_sim::BodyRef { system: sys.id, body: i as u32 }));
        let color = if selected {
            Color::srgba(1.0, 0.78, 0.35, 0.9)
        } else if b.parent.is_some() {
            Color::srgba(0.55, 0.65, 0.8, 0.22)
        } else {
            Color::srgba(0.55, 0.7, 0.95, 0.3)
        };
        let points: Vec<Vec3> = b.orbit.path(256).into_iter().map(|p| (parent_pos + to_render(p) - view.origin).as_vec3()).collect();
        gizmos.linestrip(points, color);
    }
}

fn draw_radio_spheres(mut gizmos: Gizmos, sim: Res<Sim>, view: Res<ViewInfo>) {
    let u = &sim.universe;
    for c in &u.civs {
        let Some(since) = c.radio_since else { continue };
        let radius = SPEED_OF_LIGHT * (u.time - since);
        let center: DVec3 = to_render(u.system(c.system).position);
        if radius <= 0.0 || (view.origin - center).length() < radius * 0.02 {
            continue;
        }
        let rel = (center - view.origin).as_vec3();
        let iso = Isometry3d::from_translation(rel);
        gizmos.sphere(iso, radius as f32, Color::srgba(0.4, 0.9, 1.0, 0.25)).resolution(48);
    }
}
