//! Procedural background sky: a distant starfield and galactic band baked into a cubemap.
//! (The nearby star systems of the simulation are real objects drawn separately.)

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension};
use cosmogon_sim::noise::fbm3;
use cosmogon_sim::rng::Rng;

/// Face order +X, -X, +Y, -Y, +Z, -Z (wgpu cube convention).
fn face_dir(face: usize, u: f32, v: f32) -> Vec3 {
    let (a, b) = (2.0 * u - 1.0, 2.0 * v - 1.0);
    match face {
        0 => Vec3::new(1.0, -b, -a),
        1 => Vec3::new(-1.0, -b, a),
        2 => Vec3::new(a, 1.0, b),
        3 => Vec3::new(a, -1.0, -b),
        4 => Vec3::new(a, -b, 1.0),
        _ => Vec3::new(-a, -b, -1.0),
    }
    .normalize()
}

fn dir_to_face(d: Vec3) -> (usize, f32, f32) {
    let a = d.abs();
    let (face, ma, uc, vc) = if a.x >= a.y && a.x >= a.z {
        if d.x > 0.0 { (0, a.x, -d.z, -d.y) } else { (1, a.x, d.z, -d.y) }
    } else if a.y >= a.z {
        if d.y > 0.0 { (2, a.y, d.x, d.z) } else { (3, a.y, d.x, -d.z) }
    } else if d.z > 0.0 {
        (4, a.z, d.x, -d.y)
    } else {
        (5, a.z, -d.x, -d.y)
    };
    (face, (uc / ma + 1.0) * 0.5, (vc / ma + 1.0) * 0.5)
}

pub fn build_starfield(size: u32, seed: u64) -> Image {
    let n = size as usize;
    let mut faces = vec![[0.0f32; 3]; n * n * 6];
    let band_normal = Vec3::new(0.25, 0.9, 0.35).normalize();

    // Diffuse galactic band with dust lanes.
    for (f, face) in faces.chunks_mut(n * n).enumerate() {
        for y in 0..n {
            for x in 0..n {
                let d = face_dir(f, (x as f32 + 0.5) / size as f32, (y as f32 + 0.5) / size as f32);
                let lat = d.dot(band_normal);
                let p = [d.x as f64 * 3.0, d.y as f64 * 3.0, d.z as f64 * 3.0];
                let cloud = (fbm3(seed, p, 5, 2.0, 0.55) as f32 * 0.5 + 0.5).powf(1.5);
                let dust = (fbm3(seed ^ 7, [p[0] * 2.0, p[1] * 2.0, p[2] * 2.0], 4, 2.0, 0.5) as f32).max(0.0);
                let band = (-(lat * lat) / 0.02).exp() * cloud * (1.0 - 0.8 * dust);
                let glow = band * 0.05 + (-(lat * lat) / 0.15).exp() * 0.006;
                face[y * n + x] = [glow * 0.9, glow * 0.85, glow * 1.0];
            }
        }
    }

    // Point stars, denser along the band, with blackbody-ish tints.
    let mut rng = Rng::new(seed);
    for _ in 0..(size as usize * size as usize / 9) {
        let z = rng.range(-1.0, 1.0) as f32;
        let th = rng.range(0.0, std::f64::consts::TAU) as f32;
        let r = (1.0 - z * z).sqrt();
        let d = Vec3::new(r * th.cos(), z, r * th.sin());
        let lat = d.dot(band_normal).abs();
        if rng.f64() as f32 > 0.25 + 0.75 * (-lat * lat / 0.05).exp() {
            continue;
        }
        let mag = rng.f64().powf(7.0) as f32;
        let b = 0.05 + mag * 1.8;
        let t = rng.f64() as f32;
        let tint = if t < 0.15 { [0.7, 0.8, 1.0] } else if t < 0.7 { [1.0, 0.97, 0.92] } else { [1.0, 0.8, 0.6] };
        let (f, u, v) = dir_to_face(d);
        let (x, y) = ((u * size as f32) as usize, (v * size as f32) as usize);
        if x < n && y < n {
            let px = &mut faces[f * n * n + y * n + x];
            for c in 0..3 {
                px[c] += tint[c] * b;
            }
        }
    }

    let mut data = Vec::with_capacity(n * n * 6 * 4);
    for px in &faces {
        for c in px {
            // Mild tone curve then sRGB-ish gamma.
            let v = (c / (1.0 + c)).powf(1.0 / 2.2);
            data.push((v.clamp(0.0, 1.0) * 255.0) as u8);
        }
        data.push(255);
    }
    let mut image = Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 6 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_view_descriptor = Some(TextureViewDescriptor { dimension: Some(TextureViewDimension::Cube), ..default() });
    image
}

#[derive(Resource)]
pub struct SkyHandle(pub Handle<Image>);

pub fn setup_sky(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let handle = images.add(build_starfield(1024, 0xC05_0607));
    commands.insert_resource(SkyHandle(handle));
}
