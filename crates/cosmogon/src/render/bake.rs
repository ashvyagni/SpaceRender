//! CPU texture baking from authoritative simulation state.
//!
//! Every texel is coloured by calling the *same* terrain/biome functions the simulation
//! uses, so what you see is what the simulation reasons about. Baking runs on background
//! threads and is parallelised over rows. This is the interim surface pipeline until the
//! quadtree terrain LOD of Phase 3 (see ROADMAP.md).

use cosmogon_sim::astro::{Body, BodyKind};
use cosmogon_sim::civ::settlements::LinkKind;
use cosmogon_sim::civ::Civilization;
use cosmogon_sim::noise::{fbm3, ridged3};
use cosmogon_sim::planet::terrain::{dir_from_lat_lon, Biome, SurfaceContext};

pub struct BakedSurface {
    pub width: u32,
    pub height: u32,
    /// sRGB RGBA8; alpha = water mask (for specular glints).
    pub albedo: Vec<u8>,
    /// R8 cloud opacity.
    pub clouds: Vec<u8>,
}

fn srgb(c: [f32; 3]) -> [u8; 3] {
    c.map(|x| (x.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}

fn biome_color(b: Biome) -> [f32; 3] {
    match b {
        Biome::Ocean => [0.03, 0.09, 0.22],
        Biome::SeaIce => [0.80, 0.86, 0.92],
        Biome::IceSheet => [0.92, 0.95, 0.98],
        Biome::Tundra => [0.46, 0.45, 0.38],
        Biome::Taiga => [0.16, 0.27, 0.17],
        Biome::TemperateForest => [0.16, 0.33, 0.13],
        Biome::Grassland => [0.42, 0.48, 0.23],
        Biome::Desert => [0.80, 0.68, 0.45],
        Biome::Savanna => [0.58, 0.52, 0.28],
        Biome::Rainforest => [0.08, 0.26, 0.08],
        Biome::Mountain => [0.42, 0.38, 0.33],
        Biome::Barren => [0.5, 0.45, 0.4],
    }
}

/// Run `f(row, out_row)` over all rows in parallel.
fn par_rows<F>(width: u32, height: u32, channels: usize, f: F) -> Vec<u8>
where
    F: Fn(u32, &mut [u8]) + Sync,
{
    let row_len = width as usize * channels;
    let mut buf = vec![0u8; row_len * height as usize];
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 12);
    let rows_per = (height as usize).div_ceil(threads);
    std::thread::scope(|scope| {
        for (chunk_idx, chunk) in buf.chunks_mut(row_len * rows_per).enumerate() {
            let f = &f;
            scope.spawn(move || {
                for (k, row) in chunk.chunks_mut(row_len).enumerate() {
                    f((chunk_idx * rows_per + k) as u32, row);
                }
            });
        }
    });
    buf
}

/// Direction for texel centre (x, y) of an equirectangular map that matches Bevy's UV
/// sphere (+Z pole, u = longitude / 2π, v = colatitude / π).
fn texel_dir(x: u32, y: u32, w: u32, h: u32) -> [f64; 3] {
    let lon = (x as f64 + 0.5) / w as f64 * std::f64::consts::TAU;
    let lat = std::f64::consts::FRAC_PI_2 - (y as f64 + 0.5) / h as f64 * std::f64::consts::PI;
    dir_from_lat_lon(lat, lon)
}

pub fn bake_surface(body: &Body, vegetated: bool, width: u32) -> BakedSurface {
    let width = width.max(16);
    let height = width / 2;
    let seed = body.terrain_seed;
    let ctx = SurfaceContext::new(body, vegetated);
    let base = body.color;
    let thick_haze = body.atmosphere.pressure_bar > 20.0 && body.kind.has_surface();
    let titan_haze = body.atmosphere.ch4 > 0.02 && body.kind.has_surface() && body.atmosphere.pressure_bar > 0.5;

    let albedo = par_rows(width, height, 4, |y, row| {
        for x in 0..width {
            let d = texel_dir(x, y, width, height);
            let (rgb, water): ([f32; 3], bool) = match body.kind {
                BodyKind::GasGiant | BodyKind::IceGiant => {
                    let ice = body.kind == BodyKind::IceGiant;
                    let turb = fbm3(seed, [d[0] * 3.0, d[1] * 3.0, d[2] * 3.0], 5, 2.0, 0.55);
                    let lat = d[2] + 0.08 * turb;
                    let bands = if ice { 4.0 } else { 11.0 };
                    let b = ((lat * bands * std::f64::consts::PI).sin() * 0.5 + 0.5) as f32;
                    let fine = fbm3(seed ^ 9, [d[0] * 12.0, d[1] * 12.0, d[2] * 40.0], 4, 2.0, 0.5) as f32;
                    let light = mix3(base, [1.0, 0.97, 0.9], 0.45);
                    let dark = [base[0] * 0.65, base[1] * 0.58, base[2] * 0.52];
                    let k = if ice { 0.25 } else { 1.0 };
                    let c = mix3(base, if b > 0.5 { light } else { dark }, ((b - 0.5).abs() * 2.0) * k + fine * 0.15);
                    (c, false)
                }
                _ if thick_haze || titan_haze => {
                    let n = fbm3(seed, [d[0] * 2.0, d[1] * 2.0, d[2] * 6.0], 5, 2.0, 0.5) as f32;
                    let deck = if titan_haze { [0.78, 0.58, 0.28] } else { [0.93, 0.85, 0.62] };
                    (mix3(deck, [deck[0] * 0.85, deck[1] * 0.8, deck[2] * 0.7], 0.5 + n), false)
                }
                BodyKind::Icy => {
                    let cracks = ridged3(seed, [d[0] * 4.0, d[1] * 4.0, d[2] * 4.0], 5) as f32;
                    let mottle = fbm3(seed, [d[0] * 2.0, d[1] * 2.0, d[2] * 2.0], 4, 2.0, 0.5) as f32;
                    let c = mix3(base, [base[0] * 0.55, base[1] * 0.45, base[2] * 0.35], cracks.powf(6.0) * 0.8 + mottle * 0.2);
                    (c, false)
                }
                BodyKind::Rocky => {
                    let s = ctx.sample(d);
                    let detail = fbm3(seed ^ 0x5151, [d[0] * 9.0, d[1] * 9.0, d[2] * 9.0], 4, 2.0, 0.5) as f32;
                    let sm = |e0: f64, e1: f64, x: f64| cosmogon_sim::planet::terrain::smoothstep(e0, e1, x) as f32;
                    let mut c = match s.biome {
                        Biome::Ocean | Biome::SeaIce => {
                            let shallow = (1.0 + s.height / 0.12).clamp(0.0, 1.0) as f32;
                            let water = mix3([0.02, 0.07, 0.19], [0.05, 0.25, 0.38], shallow * shallow);
                            // Sea ice fades in continuously with temperature.
                            mix3(water, [0.82, 0.87, 0.93], sm(273.0, 266.0, s.temperature))
                        }
                        Biome::Barren => {
                            let h = (s.height as f32 * 0.5 + 0.5).clamp(0.0, 1.0);
                            let crater = ridged3(seed ^ 0xC4A7, [d[0] * 6.0, d[1] * 6.0, d[2] * 6.0], 3) as f32;
                            let rock = mix3([base[0] * 0.55, base[1] * 0.5, base[2] * 0.45], [base[0] * 1.15, base[1] * 1.1, base[2] * 1.05], h + detail * 0.3 - crater.powf(8.0) * 0.4);
                            mix3(rock, [0.93, 0.95, 0.98], sm(262.0, 250.0, s.temperature))
                        }
                        _ if !ctx.vegetated => biome_color(s.biome),
                        _ => {
                            // Continuous blend from the same fields that define biomes, so
                            // transitions are gradual rather than pixel-stepped.
                            let m = s.moisture;
                            let warm = sm(270.0, 300.0, s.temperature);
                            let dry = mix3([0.80, 0.68, 0.45], [0.58, 0.52, 0.28], sm(0.15, 0.35, m));
                            let green = mix3([0.42, 0.48, 0.23], mix3([0.16, 0.33, 0.13], [0.08, 0.26, 0.08], warm), sm(0.45, 0.7, m));
                            let mut land = mix3(dry, green, sm(0.25, 0.5, m));
                            land = mix3([0.46, 0.45, 0.38], land, sm(262.0, 280.0, s.temperature));
                            land = mix3(land, [0.42, 0.38, 0.33], sm(0.4, 0.65, s.height));
                            mix3(land, [0.93, 0.95, 0.98], sm(262.0, 250.0, s.temperature).max(if s.height > 0.5 { sm(272.0, 262.0, s.temperature) } else { 0.0 }))
                        }
                    };
                    if s.biome == Biome::Mountain && !ctx.vegetated && s.temperature < 268.0 {
                        c = mix3(c, [0.95, 0.96, 0.98], 0.8);
                    }
                    let shade = 1.0 + detail * 0.18 + (s.height.max(0.0) as f32) * 0.1;
                    ([c[0] * shade, c[1] * shade, c[2] * shade], s.biome == Biome::Ocean)
                }
            };
            let px = srgb(rgb);
            let i = x as usize * 4;
            row[i..i + 3].copy_from_slice(&px);
            row[i + 3] = if water { 255 } else { 0 };
        }
    });

    // Cloud cover scales with surface water and atmosphere.
    let coverage = if body.kind.has_surface() && body.atmosphere.pressure_bar > 0.05 && body.hydro.ocean_fraction > 0.0 && !thick_haze {
        (0.25 + 0.45 * body.hydro.ocean_fraction) as f32
    } else {
        0.0
    };
    let clouds = if coverage > 0.0 {
        par_rows(width, height, 1, |y, row| {
            for x in 0..width {
                let d = texel_dir(x, y, width, height);
                // Domain-warped noise gives swirling storm-like structure.
                let w = fbm3(seed ^ 0xC10D, [d[0] * 2.0, d[1] * 2.0, d[2] * 2.0], 3, 2.0, 0.5);
                let n = fbm3(seed ^ 0xC10E, [d[0] * 3.0 + w, d[1] * 3.0 - w, d[2] * 5.0], 6, 2.1, 0.55) as f32;
                let lat_band = (1.0 - (d[2] as f32).abs()).powf(0.4);
                let v = ((n * 1.6 + coverage - 0.55) * 2.2 * lat_band).clamp(0.0, 1.0);
                row[x as usize] = (v * 255.0) as u8;
            }
        })
    } else {
        vec![0u8; (width * height) as usize]
    };

    BakedSurface { width, height, albedo, clouds }
}

/// Night-side lights from a civilization's settlements and transport links (R8).
pub fn bake_lights(civ: &Civilization, width: u32) -> Vec<u8> {
    let height = width / 2;
    let mut buf = vec![0f32; (width * height) as usize];
    let electric = civ.flags.contains("electric_light");
    let scale = if electric { 1.0 } else { 0.12 };
    let mut splat = |lat: f64, lon: f64, radius_px: f32, intensity: f32| {
        let cx = (lon.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU * width as f64) as f32;
        let cy = ((std::f64::consts::FRAC_PI_2 - lat) / std::f64::consts::PI * height as f64) as f32;
        // Equirectangular stretch near the poles.
        let stretch = (1.0 / lat.cos().abs().max(0.15)) as f32;
        let rx = (radius_px * stretch).ceil() as i32;
        let ry = radius_px.ceil() as i32;
        for dy in -ry..=ry {
            let y = cy as i32 + dy;
            if y < 0 || y >= height as i32 {
                continue;
            }
            for dx in -rx..=rx {
                let x = (cx as i32 + dx).rem_euclid(width as i32);
                let fx = dx as f32 / stretch;
                let r2 = (fx * fx + (dy * dy) as f32) / (radius_px * radius_px).max(0.25);
                if r2 < 1.0 {
                    buf[(y as u32 * width + x as u32) as usize] += intensity * (1.0 - r2).powi(2);
                }
            }
        }
    };
    let px_per_rad = width as f32 / std::f32::consts::TAU;
    for (si, s) in civ.sites.iter().enumerate().filter(|(_, s)| s.active() && s.population > 50.0) {
        // Sprawl: many small points clustered around the centre, more and wider for bigger
        // settlements, so lights read as urban fabric rather than single blobs.
        let pop = s.population;
        let lp = pop.log10() as f32;
        let core = ((lp - 2.0) / 5.0).clamp(0.05, 1.4) * scale;
        splat(s.lat, s.lon, (1.0 + 0.9 * (lp - 4.0)).max(0.8), core);
        let count = ((pop / 5_000.0).sqrt() as usize).clamp(3, 160);
        let spread = (lp - 3.0).max(0.4) * 0.007;
        let mut h = cosmogon_sim::rng::mix(si as u64 + 1, civ.id as u64);
        for _ in 0..count {
            h = cosmogon_sim::rng::mix(h, 0x9E37);
            let a = (h & 0xFFFF) as f32 / 65535.0 * std::f32::consts::TAU;
            let r = ((h >> 16) & 0xFFFF) as f32 / 65535.0;
            let d = spread * r * r * 1.6;
            let (dlat, dlon) = (d * a.sin(), d * a.cos() / (s.lat.cos().abs() as f32).max(0.2));
            splat(s.lat + dlat as f64, s.lon + dlon as f64, 1.1, core * 0.35 * (1.0 - r * 0.6));
        }
        let _ = px_per_rad;
    }
    // Lit corridors along roads and railways.
    if electric {
        for l in civ.links.iter().filter(|l| l.kind != LinkKind::Sea) {
            let (a, b) = (&civ.sites[l.a as usize], &civ.sites[l.b as usize]);
            let (da, db) = (a.dir(), b.dir());
            for k in 1..48 {
                let t = k as f64 / 48.0;
                let p = [da[0] + (db[0] - da[0]) * t, da[1] + (db[1] - da[1]) * t, da[2] + (db[2] - da[2]) * t];
                let n = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
                let (lat, lon) = cosmogon_sim::planet::terrain::lat_lon_from_dir([p[0] / n, p[1] / n, p[2] / n]);
                splat(lat, lon, 0.7, 0.07);
            }
        }
    }
    buf.into_iter().map(|v| (v.min(1.0) * 255.0) as u8).collect()
}

/// Radial ring texture (RGBA, 256×1, alpha = opacity).
pub fn bake_rings(seed: u64, opacity: f64, tint: [f32; 3]) -> Vec<u8> {
    let mut out = Vec::with_capacity(256 * 4);
    for i in 0..256 {
        let r = i as f64 / 255.0;
        let bands = fbm3(seed, [r * 40.0, 0.5, 0.5], 4, 2.2, 0.6) * 0.5 + 0.5;
        let gap = if (0.62..0.67).contains(&r) { 0.1 } else { 1.0 };
        let edge = (r * 12.0).min(1.0) * ((1.0 - r) * 20.0).min(1.0);
        let a = (bands * gap * edge * opacity).clamp(0.0, 1.0);
        let c = mix3(tint, [0.95, 0.92, 0.85], bands as f32);
        out.extend_from_slice(&srgb(c));
        out.push((a * 255.0) as u8);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmogon_sim::astro::sol;

    #[test]
    fn earth_texture_has_oceans_and_land() {
        let sys = sol::sol_system();
        let earth = &sys.bodies[sys.find_body("Earth").unwrap()];
        let baked = bake_surface(earth, true, 128);
        assert_eq!(baked.albedo.len(), 128 * 64 * 4);
        let water = baked.albedo.chunks(4).filter(|p| p[3] == 255).count() as f64 / (128.0 * 64.0);
        assert!((0.5..0.85).contains(&water), "{water}");
        assert!(baked.clouds.iter().any(|c| *c > 0));
    }
}
