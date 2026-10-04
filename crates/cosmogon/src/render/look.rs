//! Appearance model: how a world *looks*, derived from what the simulation knows about it.
//!
//! Every body gets a [`Look`]: a visual style chosen from its physical state (kind,
//! temperature, atmosphere, geology, water) — or, for the real Solar System, from what
//! spacecraft have shown us. The same look colours the baked globe texture, the close-up
//! terrain and the shader's fine detail, so a world looks consistent from any distance.
//!
//! Colours for real worlds are hand-tuned approximations of true-colour imagery
//! (Voyager, Galileo, Cassini, MESSENGER, Juno, New Horizons); procedural worlds use the
//! same models with parameters from physics (giant-planet palettes follow Sudarsky et al.
//! 2000's temperature classes). Visual only: nothing here feeds back into the simulation.

use cosmogon_sim::astro::{Body, BodyKind};
use cosmogon_sim::noise::{fbm3, gradient3, ridged3};
use cosmogon_sim::planet::terrain::{smoothstep, SurfaceContext};
use cosmogon_sim::rng::mix;

use super::bake::{mix3, surface_color};

type Rgb = [f32; 3];

/// A latitude band of a giant planet. `dark` > 0 is a belt, < 0 a bright zone.
#[derive(Clone, Debug)]
pub struct Band {
    /// Centre latitude (degrees) and half-width (degrees).
    pub lat: f32,
    pub half: f32,
    pub dark: f32,
    /// Optional colour shift (towards this colour by `dark.abs()`).
    pub tint: Option<Rgb>,
}

/// A long-lived vortex (Great Red Spot, white ovals, dark spots).
#[derive(Clone, Debug)]
pub struct Storm {
    pub lat: f64,
    pub lon: f64,
    /// Angular half-size in latitude (radians); longitude extent is `stretch` times larger.
    pub size: f64,
    pub stretch: f64,
    pub color: Rgb,
    /// Swirl strength (radians of rotation at the centre).
    pub swirl: f64,
}

#[derive(Clone, Debug)]
pub struct Giant {
    pub zone: Rgb,
    pub belt: Rgb,
    pub polar: Rgb,
    pub bands: Vec<Band>,
    pub storms: Vec<Storm>,
    /// Strength of turbulent eddies along band edges (0..1).
    pub turbulence: f32,
    /// Overall band contrast multiplier.
    pub contrast: f32,
    /// Saturn's north-polar hexagon.
    pub hexagon: bool,
    /// Latitude (degrees) beyond which the banding gives way to mottled polar regions.
    pub polar_lat: f32,
}

#[derive(Clone, Debug)]
pub struct Cratered {
    pub bright: Rgb,
    pub dark: Rgb,
    /// Fraction of the surface covered by dark smooth plains (maria).
    pub maria: f32,
    /// Strength of bright ray systems around fresh craters.
    pub rays: f32,
    pub ray_color: Rgb,
}

#[derive(Clone, Debug)]
pub enum Style {
    /// The general terrain/biome model (Earth, Mars, the Moon, living and temperate worlds).
    Terran,
    /// Airless, impact-dominated surfaces (Mercury, Callisto, most small moons).
    Cratered(Cratered),
    /// Tidally heated sulphur volcanism (Io).
    Volcanic,
    /// Young fractured ice shell over an ocean (Europa).
    IceShell { stain: Rgb },
    /// Ancient dark terrain cut by bright grooved lanes (Ganymede).
    GroovedIce,
    /// Fresh ice with active south-polar fissures (Enceladus).
    TigerStripes,
    /// Organic haze over a surface of dunes and hydrocarbon lakes (Titan).
    Haze { haze: Rgb },
    /// Opaque cloud deck over a hot surface (Venus, super-Earths with thick envelopes).
    CloudDeck { tint: Rgb, streaks: f32 },
    /// Surface hot enough to melt rock (close-in exoplanets).
    Lava { melt: f32 },
    /// Hydrogen/helium or ice giant atmosphere.
    Giant(Box<Giant>),
}

/// The visual description of one body.
#[derive(Clone, Debug)]
pub struct Look {
    pub style: Style,
    /// Short human description shown in the inspector ("Sudarsky class I", "lava world").
    pub label: String,
    /// Colour of self-emitted light (lava, hot spots, thermal glow), linear RGB.
    pub emission: Rgb,
    /// Thermal glow of the whole night side (hot giants), 0..1.
    pub night_glow: f32,
}

/// One texel of appearance.
#[derive(Clone, Copy, Debug)]
pub struct Texel {
    pub rgb: Rgb,
    pub water: bool,
    /// Self-emission intensity (0..1), coloured by [`Look::emission`].
    pub emit: f32,
}

impl Texel {
    fn of(rgb: Rgb) -> Self {
        Self { rgb, water: false, emit: 0.0 }
    }
}

fn scale(d: [f64; 3], k: f64) -> [f64; 3] {
    [d[0] * k, d[1] * k, d[2] * k]
}

fn sm(e0: f64, e1: f64, x: f64) -> f32 {
    smoothstep(e0, e1, x) as f32
}

fn mul(c: Rgb, k: f32) -> Rgb {
    [c[0] * k, c[1] * k, c[2] * k]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn normalize(a: [f64; 3]) -> [f64; 3] {
    let l = dot(a, a).sqrt().max(1e-12);
    [a[0] / l, a[1] / l, a[2] / l]
}

fn hash01(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

/// Unit vector from a hash (uniform on the sphere).
fn hash_dir(h: u64) -> [f64; 3] {
    let z = 2.0 * hash01(h) - 1.0;
    let a = std::f64::consts::TAU * hash01(mix(h, 0x51));
    let r = (1.0 - z * z).max(0.0).sqrt();
    [r * a.cos(), r * a.sin(), z]
}

fn lat_lon(d: [f64; 3]) -> (f64, f64) {
    (d[2].clamp(-1.0, 1.0).asin(), d[1].atan2(d[0]))
}

/// Approximate sRGB colour of a blackbody (normalised to its brightest channel).
pub fn blackbody(t: f64) -> Rgb {
    // Tanner Helland's fit, valid ~1000–40000 K.
    let t = (t / 100.0).clamp(10.0, 400.0);
    let r = if t <= 66.0 { 255.0 } else { 329.698_727_446 * (t - 60.0).powf(-0.133_204_759_2) };
    let g = if t <= 66.0 { 99.470_802_586_1 * t.ln() - 161.119_568_166_1 } else { 288.122_169_528_3 * (t - 60.0).powf(-0.075_514_849_2) };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_731_223_1 * (t - 10.0).ln() - 305.044_792_730_7
    };
    let c = [r, g, b].map(|x: f64| (x.clamp(0.0, 255.0) / 255.0) as f32);
    let m = c[0].max(c[1]).max(c[2]).max(1e-3);
    c.map(|x| x / m)
}

fn srgb_to_linear(c: Rgb) -> Rgb {
    c.map(|x| if x <= 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4) })
}

// ── Impact craters ───────────────────────────────────────────────────────

/// What the crater field contributes at a point.
#[derive(Clone, Copy, Debug, Default)]
pub struct CraterSample {
    /// Relief in units of crater depth: −1 at the floor, positive on rims and ejecta.
    pub height: f64,
    /// Bright ejecta blanket (0..1).
    pub ejecta: f64,
    /// Bright ray systems of fresh craters (0..1).
    pub ray: f64,
    /// Inside a crater floor (0..1) — floors are often darker or flooded.
    pub floor: f64,
}

/// Craters from a jittered 3D lattice at frequency `freq`: each cell may hold one crater.
/// `rays` enables ray systems for the freshest craters (expensive, so only at low frequency).
pub fn crater_field(seed: u64, d: [f64; 3], freq: f64, density: f64, rays: bool) -> CraterSample {
    let p = scale(d, freq);
    let c = [p[0].floor() as i64, p[1].floor() as i64, p[2].floor() as i64];
    let mut out = CraterSample::default();
    let reach = if rays { 2 } else { 1 };
    for dz in -reach..=reach {
        for dy in -reach..=reach {
            for dx in -reach..=reach {
                let cell = [c[0] + dx, c[1] + dy, c[2] + dz];
                let h = mix(mix(mix(seed, cell[0] as u64), cell[1] as u64), cell[2] as u64);
                if hash01(h) > density {
                    continue;
                }
                let center = [cell[0] as f64 + hash01(mix(h, 1)), cell[1] as f64 + hash01(mix(h, 2)), cell[2] as f64 + hash01(mix(h, 3))];
                let radius = 0.12 + 0.38 * hash01(mix(h, 4)).powi(2);
                let off = [p[0] - center[0], p[1] - center[1], p[2] - center[2]];
                let dist = dot(off, off).sqrt() / radius;
                let fresh = hash01(mix(h, 5));
                if dist < 1.0 {
                    out.height += -(1.0 - dist * dist) + 0.35 * dist.powi(6);
                    out.floor = out.floor.max(1.0 - dist);
                } else if dist < 2.2 {
                    let t = (dist - 1.0) / 1.2;
                    out.height += 0.35 * (1.0 - t).powi(3);
                    out.ejecta = out.ejecta.max((1.0 - t).powi(2) * (0.3 + 0.7 * fresh));
                }
                if rays && fresh > 0.82 && dist < 9.0 && dist > 0.8 {
                    // Streaks radiating from the centre: noise in the azimuth around it.
                    let n = normalize(center);
                    let rel = [off[0] - n[0] * dot(off, n), off[1] - n[1] * dot(off, n), off[2] - n[2] * dot(off, n)];
                    let a = rel[1].atan2(rel[0]) + 0.7 * rel[2].atan2(rel[0].hypot(rel[1]));
                    let streak = (gradient3(h, a.cos() * 6.0, a.sin() * 6.0, 0.5) * 0.5 + 0.5).powi(4) * 3.0;
                    let fade = (1.0 - (dist - 0.8) / 8.2).max(0.0).powf(1.5);
                    out.ray = out.ray.max((streak * fade * (fresh - 0.82) / 0.18).min(1.0));
                }
            }
        }
    }
    out
}

/// Several crater scales combined, for colour (cheap enough for texture baking).
fn craters(seed: u64, d: [f64; 3], density: f64) -> CraterSample {
    let a = crater_field(seed ^ 0xC4A1, d, 5.0, density * 0.8, true);
    let b = crater_field(seed ^ 0xC4A2, d, 16.0, density, false);
    let c = crater_field(seed ^ 0xC4A3, d, 55.0, density, false);
    CraterSample {
        height: a.height + 0.5 * b.height + 0.25 * c.height,
        ejecta: a.ejecta.max(b.ejecta * 0.7).max(c.ejecta * 0.4),
        ray: a.ray,
        floor: a.floor.max(b.floor),
    }
}

// ── Choosing a look ─────────────────────────────────────────────────────

fn band(lat: f32, half: f32, dark: f32) -> Band {
    Band { lat, half, dark, tint: None }
}

fn jupiter() -> Giant {
    let brown = Some([0.60, 0.40, 0.26]);
    let mut bands = vec![
        Band { lat: 0.0, half: 6.5, dark: -0.6, tint: Some([0.95, 0.86, 0.66]) }, // EZ
        Band { lat: 12.5, half: 5.5, dark: 1.0, tint: brown },                   // NEB
        band(21.0, 3.0, -0.8),                                                   // NTrZ
        band(27.5, 3.5, 0.55),                                                   // NTB
        band(33.0, 2.0, -0.6),                                                   // NTZ
        band(37.0, 2.0, 0.35),                                                   // NNTB
        band(41.0, 2.0, -0.4),
        band(44.5, 1.6, 0.3),
        Band { lat: -13.5, half: 6.5, dark: 0.95, tint: brown },                 // SEB
        band(-23.5, 3.5, -0.85),                                                 // STrZ
        band(-30.5, 3.5, 0.6),                                                   // STB
        band(-36.0, 2.0, -0.6),                                                  // STZ
        band(-40.0, 2.0, 0.4),                                                   // SSTB
        band(-44.0, 1.6, -0.4),
    ];
    bands.push(band(47.5, 1.5, 0.25));
    bands.push(band(-47.5, 1.5, 0.25));
    let oval = [0.94, 0.92, 0.88];
    Giant {
        zone: [0.94, 0.90, 0.82],
        belt: [0.66, 0.53, 0.40],
        polar: [0.58, 0.56, 0.54],
        bands,
        storms: vec![
            // The Great Red Spot (~22°S); its longitude drifts, so this one is representative.
            Storm { lat: -22.3f64.to_radians(), lon: 0.9, size: 5.6f64.to_radians(), stretch: 1.55, color: [0.80, 0.45, 0.30], swirl: 2.4 },
            Storm { lat: -33.5f64.to_radians(), lon: 2.6, size: 1.8f64.to_radians(), stretch: 1.4, color: oval, swirl: 1.8 }, // Oval BA region
            Storm { lat: -40.0f64.to_radians(), lon: -1.2, size: 1.1f64.to_radians(), stretch: 1.3, color: oval, swirl: 1.5 },
            Storm { lat: -40.5f64.to_radians(), lon: 1.9, size: 0.9f64.to_radians(), stretch: 1.3, color: oval, swirl: 1.5 },
            Storm { lat: 41.0f64.to_radians(), lon: -2.4, size: 0.9f64.to_radians(), stretch: 1.3, color: oval, swirl: -1.5 },
            Storm { lat: 18.0f64.to_radians(), lon: 2.2, size: 0.8f64.to_radians(), stretch: 2.4, color: [0.55, 0.38, 0.30], swirl: -1.0 }, // brown barge
        ],
        turbulence: 1.0,
        contrast: 1.0,
        hexagon: false,
        polar_lat: 50.0,
    }
}

fn saturn() -> Giant {
    let mut bands = vec![Band { lat: 0.0, half: 9.0, dark: -0.7, tint: Some([0.96, 0.90, 0.72]) }];
    // Many narrow, low-contrast belts and zones.
    let mut lat = 9.0f32;
    let mut k = 0;
    while lat < 72.0 {
        let half = 2.0 + 1.6 * ((k * 7 % 5) as f32 / 4.0);
        let dark = if k % 2 == 0 { 0.55 } else { -0.45 };
        bands.push(band(lat + half, half, dark));
        bands.push(band(-(lat + half), half, dark * 0.9));
        lat += half * 2.0;
        k += 1;
    }
    Giant {
        zone: [0.93, 0.86, 0.68],
        belt: [0.80, 0.69, 0.50],
        polar: [0.50, 0.58, 0.64],
        bands,
        storms: vec![Storm { lat: 42.0f64.to_radians(), lon: 1.0, size: 0.6f64.to_radians(), stretch: 1.4, color: [0.96, 0.94, 0.88], swirl: 1.0 }],
        turbulence: 0.35,
        contrast: 0.55,
        hexagon: true,
        polar_lat: 72.0,
    }
}

fn uranus() -> Giant {
    Giant {
        zone: [0.72, 0.89, 0.91],
        belt: [0.62, 0.82, 0.86],
        polar: [0.80, 0.93, 0.94],
        bands: vec![band(-25.0, 8.0, 0.25), band(25.0, 8.0, 0.25), band(-45.0, 6.0, -0.3), band(45.0, 6.0, -0.3)],
        storms: vec![Storm { lat: 30.0f64.to_radians(), lon: 0.5, size: 0.7f64.to_radians(), stretch: 2.0, color: [0.92, 0.97, 0.98], swirl: 0.3 }],
        turbulence: 0.12,
        contrast: 0.35,
        hexagon: false,
        polar_lat: 60.0,
    }
}

fn neptune() -> Giant {
    Giant {
        zone: [0.34, 0.52, 0.93],
        belt: [0.25, 0.40, 0.82],
        polar: [0.28, 0.42, 0.80],
        bands: vec![band(-62.0, 6.0, 0.8), band(-28.0, 5.0, 0.35), band(-8.0, 7.0, -0.35), band(25.0, 9.0, 0.3), band(50.0, 6.0, -0.25)],
        storms: vec![
            // A dark vortex with bright companion clouds, as seen by Voyager 2 and Hubble.
            Storm { lat: -20.0f64.to_radians(), lon: 0.4, size: 4.0f64.to_radians(), stretch: 1.7, color: [0.12, 0.20, 0.48], swirl: 1.4 },
            Storm { lat: -16.0f64.to_radians(), lon: 0.45, size: 1.6f64.to_radians(), stretch: 3.0, color: [0.95, 0.97, 1.0], swirl: 0.2 },
            Storm { lat: -42.0f64.to_radians(), lon: 2.3, size: 1.2f64.to_radians(), stretch: 3.5, color: [0.92, 0.95, 1.0], swirl: 0.2 },
            Storm { lat: -55.0f64.to_radians(), lon: -1.5, size: 1.5f64.to_radians(), stretch: 1.4, color: [0.15, 0.24, 0.55], swirl: 1.0 },
        ],
        turbulence: 0.4,
        contrast: 0.6,
        hexagon: false,
        polar_lat: 70.0,
    }
}

/// Sudarsky class palette anchors (zone, belt) by effective temperature (K).
fn sudarsky(t: f64) -> (Rgb, Rgb, &'static str) {
    const ANCHORS: [(f64, Rgb, Rgb, &str); 6] = [
        (120.0, [0.93, 0.87, 0.72], [0.66, 0.52, 0.38], "Sudarsky class I — ammonia clouds"),
        (260.0, [0.97, 0.97, 0.95], [0.80, 0.82, 0.86], "Sudarsky class II — water clouds"),
        (550.0, [0.38, 0.55, 0.88], [0.27, 0.42, 0.78], "Sudarsky class III — cloudless"),
        (1000.0, [0.30, 0.28, 0.36], [0.20, 0.18, 0.26], "Sudarsky class IV — alkali metals"),
        (1500.0, [0.58, 0.52, 0.50], [0.42, 0.34, 0.32], "Sudarsky class V — silicate clouds"),
        (2600.0, [0.62, 0.42, 0.34], [0.45, 0.25, 0.18], "Sudarsky class V — silicate clouds"),
    ];
    let lt = t.max(1.0).ln();
    for w in ANCHORS.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if t <= b.0 {
            let k = (((lt - a.0.ln()) / (b.0.ln() - a.0.ln())).clamp(0.0, 1.0)) as f32;
            // Switch label at the midpoint.
            let label = if k < 0.5 { a.3 } else { b.3 };
            return (mix3(a.1, b.1, k), mix3(a.2, b.2, k), label);
        }
    }
    let last = ANCHORS[ANCHORS.len() - 1];
    (last.1, last.2, last.3)
}

fn procedural_giant(body: &Body) -> (Giant, String) {
    let seed = body.terrain_seed;
    let r = |k: u64| hash01(mix(seed, k));
    let ice = body.kind == BodyKind::IceGiant;
    let t = body.temperature;
    let (mut zone, mut belt, label) = sudarsky(t);
    let mut label = label.to_string();
    if ice && t < 300.0 {
        // Methane absorbs red light: cyan to deep blue with more methane.
        let methane = r(1) as f32;
        zone = mix3([0.68, 0.88, 0.92], [0.36, 0.55, 0.94], methane);
        belt = mix3([0.58, 0.80, 0.86], [0.26, 0.42, 0.84], methane);
        label = "Ice giant — methane-tinted hydrogen".into();
    }
    // Per-planet hue variation so no two giants share a palette.
    let jitter = |c: Rgb, k: u64| -> Rgb {
        let s = 0.12;
        [c[0] * (1.0 + s * (r(k) as f32 - 0.5)), c[1] * (1.0 + s * (r(k + 1) as f32 - 0.5)), c[2] * (1.0 + s * (r(k + 2) as f32 - 0.5))]
    };
    zone = jitter(zone, 10);
    belt = jitter(belt, 20);
    let polar = mix3(belt, [0.55, 0.58, 0.62], 0.4);

    // Faster rotators have more, narrower bands (Rhines scale).
    let rot_h = (body.rotation_period.abs() / 3600.0).max(4.0);
    let hot = t > 800.0;
    let per_hemi = if hot { 1 + (r(30) * 2.0) as usize } else if ice { 2 + (r(30) * 3.0) as usize } else { ((5.0 + 6.0 * r(31)) * (10.0 / rot_h).sqrt().clamp(0.7, 1.4)) as usize };
    let polar_lat = (45.0 + 25.0 * r(32)) as f32;
    let mut bands = vec![Band { lat: 0.0, half: (4.0 + 6.0 * r(33)) as f32, dark: -0.6, tint: None }];
    for hemi in [1.0f32, -1.0] {
        let mut lat = bands[0].half;
        for k in 0..per_hemi {
            let remaining = polar_lat - lat;
            if remaining < 2.0 {
                break;
            }
            let half = (remaining / (per_hemi - k) as f32 * 0.5 * (0.6 + 0.8 * r(40 + k as u64 + hemi as i64 as u64 * 100) as f32)).max(1.0);
            let dark = if k % 2 == 0 { 0.5 + 0.5 * r(60 + k as u64) as f32 } else { -(0.3 + 0.5 * r(70 + k as u64) as f32) };
            bands.push(band(hemi * (lat + half), half, dark));
            lat += half * 2.0;
        }
    }
    let n_storms = (r(80) * if ice { 2.5 } else { 5.0 }) as usize;
    let mut storms = Vec::new();
    for k in 0..n_storms {
        let h = mix(seed, 900 + k as u64);
        let big = k == 0 && hash01(h) < 0.5;
        let size = if big { 3.0 + 4.0 * hash01(mix(h, 1)) } else { 0.6 + 1.2 * hash01(mix(h, 1)) };
        let color = match (hash01(mix(h, 2)) * 3.0) as u32 {
            0 => mix3(belt, [0.75, 0.35, 0.25], 0.6),
            1 => mix3(belt, [0.05, 0.08, 0.2], 0.5),
            _ => mix3(zone, [1.0, 1.0, 1.0], 0.6),
        };
        storms.push(Storm {
            lat: ((hash01(mix(h, 3)) - 0.5) * 100.0).to_radians(),
            lon: hash01(mix(h, 4)) * std::f64::consts::TAU,
            size: size.to_radians(),
            stretch: 1.3 + hash01(mix(h, 5)),
            color,
            swirl: (1.0 + 1.5 * hash01(mix(h, 6))) * if hash01(mix(h, 7)) < 0.5 { 1.0 } else { -1.0 },
        });
    }
    (
        Giant {
            zone,
            belt,
            polar,
            bands,
            storms,
            turbulence: if ice { 0.3 } else if hot { 0.45 } else { 0.6 + 0.5 * r(90) as f32 },
            contrast: if ice { 0.4 } else if hot { 0.35 } else { 0.6 + 0.5 * r(91) as f32 },
            hexagon: false,
            polar_lat,
        },
        label,
    )
}

impl Look {
    pub fn of(body: &Body) -> Look {
        let plain = |style: Style, label: &str| Look { style, label: label.into(), emission: [1.0, 0.45, 0.15], night_glow: 0.0 };
        let mercury = Cratered { bright: [0.64, 0.61, 0.57], dark: [0.42, 0.40, 0.38], maria: 0.25, rays: 0.8, ray_color: [0.82, 0.80, 0.77] };
        if body.real {
            match body.name.as_str() {
                "Earth" | "Moon" | "Mars" => return plain(Style::Terran, "Measured relief / mapped terrain"),
                "Mercury" => return plain(Style::Cratered(mercury), "Cratered plains and smooth volcanic plains"),
                "Venus" => return plain(Style::CloudDeck { tint: [0.93, 0.86, 0.66], streaks: 0.6 }, "Sulphuric-acid cloud deck"),
                "Jupiter" => return plain(Style::Giant(Box::new(jupiter())), "Ammonia cloud belts and zones"),
                "Saturn" => return plain(Style::Giant(Box::new(saturn())), "Ammonia haze, polar hexagon"),
                "Uranus" => return plain(Style::Giant(Box::new(uranus())), "Methane-tinted haze"),
                "Neptune" => return plain(Style::Giant(Box::new(neptune())), "Methane atmosphere, dark vortices"),
                "Io" => return Look { style: Style::Volcanic, label: "Active sulphur volcanism".into(), emission: [1.0, 0.38, 0.10], night_glow: 0.0 },
                "Europa" => return plain(Style::IceShell { stain: [0.58, 0.36, 0.22] }, "Fractured ice shell (lineae)"),
                "Ganymede" => return plain(Style::GroovedIce, "Dark cratered and bright grooved terrain"),
                "Callisto" => return plain(Style::Cratered(Cratered { bright: [0.44, 0.40, 0.35], dark: [0.30, 0.27, 0.24], maria: 0.0, rays: 1.0, ray_color: [0.85, 0.83, 0.80] }), "Saturated ancient craters"),
                "Titan" => return plain(Style::Haze { haze: [0.82, 0.58, 0.28] }, "Organic haze over dunes and lakes"),
                "Enceladus" => return plain(Style::TigerStripes, "Fresh ice, south-polar fissures"),
                _ => {}
            }
        }
        if body.kind.is_stellar() {
            return plain(Style::Terran, body.kind.label());
        }
        let t = body.temperature;
        let atmo = &body.atmosphere;
        match body.kind {
            BodyKind::GasGiant | BodyKind::IceGiant => {
                let (g, label) = procedural_giant(body);
                let glow = ((t - 900.0) / 1200.0).clamp(0.0, 1.0) as f32;
                Look { style: Style::Giant(Box::new(g)), label, emission: srgb_to_linear(blackbody(t.max(1000.0))), night_glow: glow }
            }
            _ if t > 1000.0 => {
                let melt = ((t - 1000.0) / 1000.0).clamp(0.1, 0.95) as f32;
                Look { style: Style::Lava { melt }, label: if melt > 0.6 { "Magma-ocean world".into() } else { "Lava world".into() }, emission: srgb_to_linear(blackbody(t.max(1200.0))), night_glow: 0.0 }
            }
            _ if atmo.h2he > 0.5 && atmo.pressure_bar > 10.0 => plain(Style::CloudDeck { tint: [0.70, 0.80, 0.90], streaks: 0.3 }, "Mini-Neptune — hydrogen envelope"),
            _ if atmo.pressure_bar > 20.0 => plain(Style::CloudDeck { tint: [0.93, 0.86, 0.66], streaks: 0.5 }, "Thick cloud deck (Venus-like)"),
            _ if atmo.ch4 > 0.02 && atmo.pressure_bar > 0.3 => plain(Style::Haze { haze: [0.82, 0.58, 0.28] }, "Organic haze (Titan-like)"),
            BodyKind::Rocky if body.geology > 3.0 && atmo.pressure_bar < 0.01 => Look { style: Style::Volcanic, label: "Tidally heated volcanism".into(), emission: [1.0, 0.38, 0.10], night_glow: 0.0 },
            BodyKind::Icy if body.hydro.subsurface_ocean && body.radius < 400e3 => plain(Style::TigerStripes, "Cryovolcanic ice"),
            BodyKind::Icy if body.hydro.subsurface_ocean => plain(Style::IceShell { stain: mix3([0.58, 0.36, 0.22], [0.35, 0.40, 0.55], hash01(body.terrain_seed) as f32) }, "Fractured ice shell over an ocean"),
            BodyKind::Icy if body.geology > 0.3 => plain(Style::GroovedIce, "Tectonically resurfaced ice"),
            BodyKind::Icy if atmo.pressure_bar < 0.01 => {
                let dark = hash01(mix(body.terrain_seed, 7)) as f32;
                plain(
                    Style::Cratered(Cratered { bright: mix3([0.86, 0.85, 0.82], [0.50, 0.46, 0.40], dark), dark: mix3([0.70, 0.68, 0.64], [0.32, 0.29, 0.25], dark), maria: 0.0, rays: 0.8, ray_color: [0.92, 0.92, 0.90] }),
                    "Cratered ice",
                )
            }
            BodyKind::Rocky if atmo.pressure_bar < 0.01 && body.hydro.ocean_fraction <= 0.0 => {
                let c = body.color;
                plain(
                    Style::Cratered(Cratered { bright: mix3(c, [0.62, 0.60, 0.57], 0.5), dark: mix3(mul(c, 0.6), [0.30, 0.29, 0.28], 0.5), maria: (0.35 * hash01(mix(body.terrain_seed, 9))) as f32, rays: 0.7, ray_color: [0.80, 0.78, 0.75] }),
                    "Airless cratered world",
                )
            }
            _ => plain(Style::Terran, "Terrain and climate model"),
        }
    }

    /// Whether the globe shows clouds or haze rather than the ground (close-up terrain must
    /// then wait until the camera is below the cloud tops).
    pub fn opaque_atmosphere(&self) -> bool {
        matches!(self.style, Style::CloudDeck { .. } | Style::Haze { .. } | Style::Giant(_))
    }

    /// Altitude (m) of the visible cloud tops for opaque atmospheres.
    pub fn cloud_top(&self, body: &Body) -> f64 {
        match self.style {
            Style::CloudDeck { .. } => 65_000.0,
            Style::Haze { .. } => 200_000.0 * (1.35 / body.gravity_g().max(0.05)).min(3.0) * 0.5,
            _ => 0.0,
        }
    }

    /// Appearance of the globe (what an orbiting camera sees) at unit direction `d`.
    pub fn globe(&self, body: &Body, ctx: &SurfaceContext, d: [f64; 3]) -> Texel {
        let seed = body.terrain_seed;
        match &self.style {
            Style::Giant(g) => giant(g, seed, d),
            Style::CloudDeck { tint, streaks } => Texel::of(cloud_deck(*tint, *streaks, seed, d)),
            Style::Haze { haze } => {
                // Mostly featureless haze; a hint of the darkest dune fields shows through.
                let ground = self.ground(body, ctx, d);
                let n = fbm3(seed ^ 0x4A2E, scale(d, 3.0), 4, 2.0, 0.5) as f32;
                let (lat, _) = lat_lon(d);
                // Seasonal polar hood: the winter pole is darker and greyer.
                let hood = sm(0.9, 1.25, lat) * 0.25;
                let base = mix3(*haze, [0.60, 0.50, 0.38], hood);
                let c = mix3(base, ground.rgb, 0.10);
                Texel::of(mul(c, 1.0 + 0.04 * n))
            }
            _ => self.ground(body, ctx, d),
        }
    }

    /// The ground itself (what close-up terrain shows, beneath any clouds).
    pub fn ground(&self, body: &Body, ctx: &SurfaceContext, d: [f64; 3]) -> Texel {
        let seed = body.terrain_seed;
        match &self.style {
            Style::Terran | Style::Giant(_) => {
                let (rgb, water) = surface_color(body.kind, ctx, body.color, seed, d);
                Texel { rgb, water, emit: 0.0 }
            }
            Style::Cratered(c) => Texel::of(cratered(c, seed, d)),
            Style::Volcanic => volcanic(seed, d),
            Style::IceShell { stain } => Texel::of(ice_shell(*stain, seed, d)),
            Style::GroovedIce => Texel::of(grooved_ice(seed, d)),
            Style::TigerStripes => Texel::of(tiger_stripes(seed, d)),
            Style::Haze { .. } => titan_ground(ctx, seed, d),
            Style::CloudDeck { .. } => Texel::of(basalt_plains(seed, d)),
            Style::Lava { melt } => lava(*melt, ctx, seed, d),
        }
    }

    /// Extra close-up relief (terrain units) specific to the style, on top of the
    /// authoritative terrain and generic detail.
    pub fn relief(&self, seed: u64, d: [f64; 3]) -> f64 {
        match &self.style {
            Style::Cratered(_) => {
                let c = craters(seed, d, 0.6);
                0.45 * c.height
            }
            Style::IceShell { .. } => {
                // Double ridges along lineae.
                let l = lineae(seed, d);
                0.05 * l.ridge
            }
            Style::GroovedIce => {
                let g = grooves(seed, d);
                0.03 * g.0 + 0.2 * crater_field(seed ^ 0xC4A2, d, 16.0, 0.4, false).height
            }
            Style::Volcanic => {
                let p = paterae(seed, d);
                -0.08 * p.caldera + 0.05 * fbm3(seed ^ 0x10, scale(d, 30.0), 4, 2.0, 0.5)
            }
            Style::Lava { .. } => 0.04 * ridged3(seed ^ 0x1A7A, scale(d, 20.0), 4),
            _ => 0.0,
        }
    }
}

// ── Giants ──────────────────────────────────────────────────────────────

/// Rotate `d` about unit axis `a` by angle `ang`.
fn rotate(d: [f64; 3], a: [f64; 3], ang: f64) -> [f64; 3] {
    let (s, c) = ang.sin_cos();
    let k = dot(a, d);
    let cross = [a[1] * d[2] - a[2] * d[1], a[2] * d[0] - a[0] * d[2], a[0] * d[1] - a[1] * d[0]];
    [d[0] * c + cross[0] * s + a[0] * k * (1.0 - c), d[1] * c + cross[1] * s + a[1] * k * (1.0 - c), d[2] * c + cross[2] * s + a[2] * k * (1.0 - c)]
}

fn storm_dir(s: &Storm) -> [f64; 3] {
    [s.lat.cos() * s.lon.cos(), s.lat.cos() * s.lon.sin(), s.lat.sin()]
}

/// Elliptical distance from a storm centre in units of its size (1 = edge).
fn storm_distance(s: &Storm, d: [f64; 3]) -> f64 {
    let (lat, lon) = lat_lon(d);
    let mut dl = lon - s.lon;
    dl = (dl + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI;
    let x = dl * s.lat.cos() / s.stretch;
    let y = lat - s.lat;
    (x * x + y * y).sqrt() / s.size
}

fn giant(g: &Giant, seed: u64, d0: [f64; 3]) -> Texel {
    // Vortices twist the flow around them before anything else is sampled.
    let mut d = d0;
    for s in &g.storms {
        let r = storm_distance(s, d);
        if r < 2.5 {
            let w = (1.0 - r / 2.5).powi(2);
            d = rotate(d, storm_dir(s), s.swirl * w);
        }
    }
    // Zonal flow: noise stretched along longitude (compressed in z), relaxing to isotropic
    // towards the poles where "along longitude" stops meaning anything.
    let iso = (1.0 - d[2] * d[2]).powi(2);
    let zk = |k: f64| 1.0 + (k - 1.0) * iso;
    let q = [d[0] * 3.0, d[1] * 3.0, d[2] * 3.0 * zk(3.0)];
    let warp = fbm3(seed ^ 0x61, q, 5, 2.1, 0.55);
    let eddy_q = [d[0] * 10.0 + 0.8 * warp, d[1] * 10.0 - 0.8 * warp, d[2] * 10.0 * zk(2.2)];
    let eddies = fbm3(seed ^ 0x62, eddy_q, 6, 2.2, 0.55);
    let lat = (d[2].clamp(-1.0, 1.0).asin()).to_degrees() as f32 + g.turbulence * (2.2 * warp as f32 + 0.8 * eddies as f32);
    let abs_lat = lat.abs();

    // Belt/zone profile.
    let mut dark = 0.0f32;
    let mut tint: Option<(Rgb, f32)> = None;
    for b in &g.bands {
        let x = (lat - b.lat) / b.half;
        let w = (-x * x * 1.6).exp();
        dark += b.dark * w;
        if let Some(t) = b.tint {
            if w > 0.3 {
                tint = Some((t, w));
            }
        }
    }
    let dark = (dark * g.contrast).clamp(-1.0, 1.0);
    let mut c = if dark > 0.0 { mix3(g.zone, g.belt, dark) } else { mix3(g.zone, mix3(g.zone, [1.0, 1.0, 0.98], 0.35), -dark) };
    if let Some((t, w)) = tint {
        c = mix3(c, t, 0.5 * w * dark.abs().max(0.3));
    }
    // Turbulent texture: brighter plumes and darker eddies, strongest at band edges.
    let edge = 1.0 - dark.abs();
    let fine = fbm3(seed ^ 0x63, [d[0] * 34.0 + warp, d[1] * 34.0, d[2] * 34.0 * zk(1.8)], 4, 2.0, 0.5) as f32;
    c = mul(c, 1.0 + g.turbulence * (0.10 * eddies as f32 * (0.4 + edge) + 0.05 * fine));

    // Polar regions: mottled, darker, with small vortices.
    let polar = sm(g.polar_lat as f64 - 6.0, g.polar_lat as f64 + 4.0, abs_lat as f64);
    if polar > 0.0 {
        let cells = fbm3(seed ^ 0x64, scale(d, 14.0), 4, 2.0, 0.5) as f32;
        let p = mul(g.polar, 1.0 + 0.15 * cells);
        c = mix3(c, p, polar * 0.85);
    }
    if g.hexagon && d0[2] > 0.0 {
        // Saturn's hexagonal jet near 78°N.
        let a = d0[1].atan2(d0[0]).rem_euclid(std::f64::consts::FRAC_PI_3) - std::f64::consts::FRAC_PI_6;
        let colat = d0[2].clamp(-1.0, 1.0).acos();
        let hex_r = 12.5f64.to_radians() / a.cos();
        let inside = sm(hex_r + 0.01, hex_r - 0.01, colat);
        c = mix3(c, [0.42, 0.50, 0.56], inside * 0.7);
        let rim = (-(colat - hex_r).powi(2) / 0.00004).exp() as f32;
        c = mix3(c, [0.88, 0.84, 0.72], rim * 0.5);
        let eye = sm(0.06, 0.02, colat);
        c = mix3(c, [0.30, 0.34, 0.38], eye);
    }
    // Storms.
    for s in &g.storms {
        let r = storm_distance(s, d0);
        if r < 1.3 {
            let inside = sm(1.05, 0.75, r);
            let swirl_tex = (fbm3(seed ^ 0x65, scale(d, 60.0), 3, 2.0, 0.5) * 0.5 + 0.5) as f32;
            let core = mix3(s.color, mul(s.color, 0.85 + 0.3 * swirl_tex), 0.6);
            c = mix3(c, core, inside);
            // A pale collar around large vortices (the GRS's "hollow").
            let collar = (-(r - 1.12).powi(2) / 0.005).exp() as f32;
            c = mix3(c, mix3(g.zone, [1.0, 1.0, 1.0], 0.3), collar * 0.5);
        }
    }
    // Emission marks the hottest (deepest, cloud-free) regions for glowing giants.
    // Tidally locked hot giants glow most around a hot spot shifted east of the substellar
    // point by their winds (as measured for HD 189733 b); the body frame's +x faces the star.
    let hotspot = [0.94f64, 0.34, 0.0];
    let day = (dot(d0, hotspot) * 0.5 + 0.5) as f32;
    let emit = ((0.25 + 0.75 * day * day) * (0.7 + 0.3 * dark.max(0.0) + 0.2 * eddies as f32)).clamp(0.0, 1.0);
    Texel { rgb: c, water: false, emit }
}

// ── Cloud decks and hazes ─────────────────────────────────────────────

fn cloud_deck(tint: Rgb, streaks: f32, seed: u64, d: [f64; 3]) -> Rgb {
    let (lat, lon) = lat_lon(d);
    // Super-rotating clouds form chevrons (the "Y" seen in ultraviolet), pointing west.
    let chevron = lon + 1.8 * lat.abs();
    let warp = fbm3(seed ^ 0x71, [d[0] * 2.0, d[1] * 2.0, d[2] * 6.0], 4, 2.0, 0.5);
    let bands = ((chevron * 2.0 + warp * 2.5).sin() * 0.5 + 0.5) as f32;
    let fine = fbm3(seed ^ 0x72, [d[0] * 8.0 + warp, d[1] * 8.0, d[2] * 20.0], 5, 2.1, 0.5) as f32;
    let polar = sm(1.0, 1.35, lat.abs());
    let k = 1.0 - streaks * (0.06 * bands + 0.04 * fine) - 0.05 * polar;
    let c = mix3(tint, [tint[0] * 0.92, tint[1] * 0.88, tint[2] * 0.8], bands * 0.4 * streaks);
    mul(c, k)
}

fn basalt_plains(seed: u64, d: [f64; 3]) -> Rgb {
    let flows = fbm3(seed ^ 0xBA5A, scale(d, 6.0), 6, 2.1, 0.55) as f32;
    let fine = fbm3(seed ^ 0xBA5B, scale(d, 80.0), 4, 2.0, 0.5) as f32;
    let tesserae = ridged3(seed ^ 0xBA5C, scale(d, 4.0), 4) as f32;
    let c = mix3([0.30, 0.26, 0.22], [0.48, 0.40, 0.32], 0.5 + 0.5 * flows);
    let c = mix3(c, [0.56, 0.50, 0.44], tesserae.powf(3.0) * 0.6);
    mul(c, 1.0 + 0.12 * fine)
}

fn titan_ground(ctx: &SurfaceContext, seed: u64, d: [f64; 3]) -> Texel {
    let s = ctx.sample(d);
    let (lat, lon) = lat_lon(d);
    // Equatorial linear dune seas (dark organic sand), bright icy highlands, polar lakes.
    let dunes_mask = sm(0.55, 0.3, lat.abs()) * sm(-0.1, 0.2, fbm3(seed ^ 0xD0, scale(d, 2.0), 4, 2.0, 0.5));
    let dune_lines = ((lon * 160.0 + lat * 20.0 + 3.0 * fbm3(seed ^ 0xD1, scale(d, 8.0), 3, 2.0, 0.5)).sin() * 0.5 + 0.5) as f32;
    let highland = sm(0.15, 0.5, s.height);
    let mut c = mix3([0.46, 0.38, 0.26], [0.70, 0.62, 0.48], highland);
    c = mix3(c, mix3([0.20, 0.15, 0.10], [0.28, 0.22, 0.15], dune_lines), dunes_mask * (1.0 - highland));
    let lake_noise = fbm3(seed ^ 0xD2, scale(d, 10.0), 5, 2.0, 0.55);
    let lake = lat.abs() > 1.05 && lake_noise > 0.15 - (lat.abs() - 1.05) * 0.6 && s.height < 0.2;
    if lake {
        return Texel { rgb: [0.05, 0.05, 0.06], water: true, emit: 0.0 };
    }
    Texel::of(c)
}

// ── Airless and icy worlds ─────────────────────────────────────────────

fn cratered(c: &Cratered, seed: u64, d: [f64; 3]) -> Rgb {
    let maria_n = fbm3(seed ^ 0x3A, scale(d, 1.3), 5, 2.0, 0.5);
    let maria = if c.maria > 0.0 { sm(0.35 - c.maria as f64, 0.45 - c.maria as f64, maria_n) } else { 0.0 };
    let albedo = fbm3(seed ^ 0x3B, scale(d, 3.0), 5, 2.0, 0.5) as f32;
    let cr = craters(seed, d, if maria > 0.5 { 0.35 } else { 0.7 });
    let mut col = mix3(c.bright, c.dark, maria);
    col = mul(col, 1.0 + 0.12 * albedo);
    col = mul(col, 1.0 - 0.08 * cr.floor as f32 + 0.06 * cr.height.max(0.0) as f32);
    col = mix3(col, c.ray_color, (cr.ejecta as f32 * 0.35 + cr.ray as f32 * 0.7) * c.rays);
    let fine = fbm3(seed ^ 0x3C, scale(d, 120.0), 3, 2.0, 0.5) as f32;
    mul(col, 1.0 + 0.05 * fine)
}

struct Paterae {
    caldera: f64,
    plume: f64,
    lava: f64,
}

/// Io's volcanic depressions: dark calderas, some active (glowing) with red plume rings.
fn paterae(seed: u64, d: [f64; 3]) -> Paterae {
    let freq = 9.0;
    let p = scale(d, freq);
    let c = [p[0].floor() as i64, p[1].floor() as i64, p[2].floor() as i64];
    let mut out = Paterae { caldera: 0.0, plume: 0.0, lava: 0.0 };
    for dz in -1..=1 {
        for dy in -1..=1 {
            for dx in -1..=1 {
                let cell = [c[0] + dx, c[1] + dy, c[2] + dz];
                let h = mix(mix(mix(seed ^ 0x10, cell[0] as u64), cell[1] as u64), cell[2] as u64);
                if hash01(h) > 0.7 {
                    continue;
                }
                let center = [cell[0] as f64 + hash01(mix(h, 1)), cell[1] as f64 + hash01(mix(h, 2)), cell[2] as f64 + hash01(mix(h, 3))];
                let off = [p[0] - center[0], p[1] - center[1], p[2] - center[2]];
                let r = 0.02 + 0.05 * hash01(mix(h, 4));
                let raw = dot(off, off).sqrt() / r;
                let active = hash01(mix(h, 5));
                if raw > if active > 0.88 { 10.0 } else { 1.6 } {
                    continue;
                }
                // Irregular outline.
                let wob = 1.0 + 0.35 * gradient3(h, off[0] * 6.0, off[1] * 6.0, off[2] * 6.0);
                let dist = raw / wob;
                out.caldera = out.caldera.max(smoothstep(1.1, 0.8, dist));
                if active > 0.75 && dist < 0.9 {
                    out.lava = out.lava.max(smoothstep(0.9, 0.3, dist) * (0.5 + 0.5 * gradient3(h, off[0] * 30.0, off[1] * 30.0, off[2] * 30.0)).max(0.0));
                }
                if active > 0.88 {
                    // Pele-type plume deposit: a red ring ~6 caldera radii across.
                    let ring = (-(dist / 7.0 - 1.0).powi(2) / 0.04).exp();
                    out.plume = out.plume.max(ring * (0.6 + 0.4 * gradient3(h ^ 9, off[0] * 4.0, off[1] * 4.0, off[2] * 4.0)));
                }
            }
        }
    }
    out
}

fn volcanic(seed: u64, d: [f64; 3]) -> Texel {
    let (lat, _) = lat_lon(d);
    let plains = fbm3(seed ^ 0x501, scale(d, 3.0), 6, 2.1, 0.55);
    let frost = fbm3(seed ^ 0x502, scale(d, 5.0), 5, 2.0, 0.5);
    let fine = fbm3(seed ^ 0x503, scale(d, 50.0), 4, 2.0, 0.5) as f32;
    let mut c = mix3([0.90, 0.82, 0.40], [0.86, 0.60, 0.28], sm(-0.1, 0.4, plains));
    c = mix3(c, [0.93, 0.92, 0.84], sm(0.15, 0.45, frost) * 0.7);
    // Reddish-brown polar regions (radiation-processed sulphur).
    c = mix3(c, [0.56, 0.42, 0.30], sm(0.6, 1.2, lat.abs()) * 0.8);
    let p = paterae(seed, d);
    c = mix3(c, [0.82, 0.32, 0.16], p.plume as f32 * 0.75);
    c = mix3(c, [0.30, 0.22, 0.14], p.caldera as f32 * 0.75);
    c = mul(c, 1.0 + 0.08 * fine);
    Texel { rgb: c, water: false, emit: (p.lava * 1.2).min(1.0) as f32 }
}

pub struct Lineae {
    /// Stain intensity (dark reddish lines).
    pub stain: f64,
    /// Ridge relief (double ridge with central trough).
    pub ridge: f64,
}

/// Europa's lineae: arcs of great and small circles, fading in and out.
pub fn lineae(seed: u64, d: [f64; 3]) -> Lineae {
    let mut out = Lineae { stain: 0.0, ridge: 0.0 };
    let wob = fbm3(seed ^ 0xE1, scale(d, 6.0), 3, 2.0, 0.5) * 0.004;
    for k in 0..48u64 {
        let h = mix(seed ^ 0xE0, k);
        let n = hash_dir(h);
        let offset = (hash01(mix(h, 1)) - 0.5) * 0.6;
        let x = dot(d, n) - offset + wob;
        let width = 0.0015 + 0.004 * hash01(mix(h, 2)).powi(2);
        if x.abs() > width * 6.0 {
            continue;
        }
        // Only arcs, not full circles.
        let seg = fbm3(h, scale(d, 1.5 + 2.0 * hash01(mix(h, 3))), 2, 2.0, 0.5);
        let present = smoothstep(-0.05, 0.15, seg);
        if present <= 0.0 {
            continue;
        }
        let t = x / width;
        out.stain = out.stain.max((-(t * t) * 0.35).exp() * present);
        // Twin ridges either side of a central trough.
        let ridge = (-((t.abs() - 1.0).powi(2)) * 2.0).exp() - 0.6 * (-(t * t) * 3.0).exp();
        out.ridge += ridge * present;
    }
    // Fine background crisscross.
    let fine = ridged3(seed ^ 0xE2, scale(d, 40.0), 3).powi(8);
    out.stain = out.stain.max(fine * 0.5);
    out
}

fn ice_shell(stain: Rgb, seed: u64, d: [f64; 3]) -> Rgb {
    let l = lineae(seed, d);
    // Chaos terrain: mottled, darker brownish regions of disrupted crust.
    let chaos_mask = sm(0.15, 0.35, fbm3(seed ^ 0xE3, scale(d, 2.5), 5, 2.0, 0.5));
    let blocks = fbm3(seed ^ 0xE4, scale(d, 30.0), 4, 2.0, 0.5) as f32;
    let (lat, _) = lat_lon(d);
    let mut c = mix3([0.90, 0.87, 0.80], [0.82, 0.76, 0.66], sm(0.0, 0.8, 1.0 - lat.abs()) * 0.5);
    c = mix3(c, mul(stain, 1.15 + 0.3 * blocks), chaos_mask * 0.6);
    c = mix3(c, stain, l.stain as f32 * 0.75);
    c = mix3(c, [0.95, 0.94, 0.90], (l.ridge.max(0.0) * 0.15) as f32);
    let cr = crater_field(seed ^ 0xC4A1, d, 5.0, 0.08, true);
    mix3(c, [0.95, 0.95, 0.93], (cr.ray * 0.6 + cr.ejecta * 0.3) as f32)
}

/// Ganymede-style grooves: (relief, bright-terrain mask).
fn grooves(seed: u64, d: [f64; 3]) -> (f64, f64) {
    let region = fbm3(seed ^ 0x6A, scale(d, 1.8), 5, 2.0, 0.5);
    let bright = smoothstep(-0.05, 0.08, region);
    // Lanes of parallel grooves; orientation changes from lane to lane.
    let lane = fbm3(seed ^ 0x6B, scale(d, 4.0), 2, 2.0, 0.5);
    let ang = lane * 6.0;
    let axis = normalize([ang.cos(), ang.sin(), 0.4 * (ang * 1.7).sin()]);
    let g = (dot(d, axis) * 900.0 + 4.0 * fbm3(seed ^ 0x6C, scale(d, 12.0), 3, 2.0, 0.5)).sin();
    (g * bright, bright)
}

fn grooved_ice(seed: u64, d: [f64; 3]) -> Rgb {
    let (g, bright) = grooves(seed, d);
    let (lat, _) = lat_lon(d);
    let dark_terrain = mix3([0.40, 0.36, 0.31], [0.48, 0.43, 0.37], (fbm3(seed ^ 0x6D, scale(d, 6.0), 4, 2.0, 0.5) * 0.5 + 0.5) as f32);
    let bright_terrain = mul([0.76, 0.73, 0.68], 1.0 + 0.06 * g as f32);
    let mut c = mix3(dark_terrain, bright_terrain, bright as f32);
    // Polar frost caps.
    c = mix3(c, [0.86, 0.86, 0.86], sm(0.7, 1.0, lat.abs()) * 0.6);
    let cr = craters(seed, d, 0.5);
    mix3(c, [0.90, 0.89, 0.86], (cr.ray * 0.8 + cr.ejecta * 0.4) as f32)
}

fn tiger_stripes(seed: u64, d: [f64; 3]) -> Rgb {
    let (lat, lon) = lat_lon(d);
    let mut c: Rgb = [0.95, 0.96, 0.97];
    // Old cratered northern plains, fractured mid-latitudes.
    let cr = crater_field(seed ^ 0xC4A2, d, 18.0, if lat > 0.3 { 0.6 } else { 0.15 }, false);
    c = mul(c, 1.0 - 0.06 * cr.floor as f32);
    let fract = ridged3(seed ^ 0xEC, scale(d, 14.0), 4).powi(6) as f32;
    c = mix3(c, [0.70, 0.80, 0.86], fract * 0.35);
    // Four parallel "tiger stripe" sulci around the south pole, ~35 km apart.
    if lat < -1.15 {
        let x = (lat + std::f64::consts::FRAC_PI_2) * (lon * 0.7).cos() + 0.04 * fbm3(seed ^ 0xED, scale(d, 10.0), 3, 2.0, 0.5);
        let stripes = ((x * 70.0).sin().abs()).powf(0.15);
        let s = (1.0 - stripes) as f32 * sm(-1.15, -1.3, lat);
        c = mix3(c, [0.45, 0.62, 0.72], s * 0.9);
    }
    c
}

fn lava(melt: f32, ctx: &SurfaceContext, seed: u64, d: [f64; 3]) -> Texel {
    let s = ctx.sample(d);
    let cracks = ridged3(seed ^ 0x1A7A, scale(d, 9.0), 5).powi(10);
    let crust_n = fbm3(seed ^ 0x1A7B, scale(d, 20.0), 4, 2.0, 0.5) as f32;
    // Low ground and the substellar side of locked worlds are molten.
    let sub = if ctx.tidally_locked { (d[0] * 0.5 + 0.5) as f32 } else { 0.5 };
    let molten_level = (melt - 0.5) as f64 * 1.2 + (sub as f64 - 0.5) * 0.8;
    let sea = smoothstep(molten_level + 0.05, molten_level - 0.05, s.height) as f32;
    let crust = mix3([0.08, 0.07, 0.065], [0.18, 0.15, 0.13], 0.5 + 0.5 * crust_n);
    let magma = [0.35, 0.10, 0.03];
    let c = mix3(crust, magma, sea);
    let emit = (sea + cracks as f32 * (0.4 + 0.6 * melt)).min(1.0);
    Texel { rgb: c, water: false, emit }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmogon_sim::astro::sol;

    fn sol_look(name: &str) -> (Look, Body) {
        let sys = sol::sol_system();
        let b = sys.bodies[sys.find_body(name).unwrap()].clone();
        (Look::of(&b), b)
    }

    #[test]
    fn real_worlds_get_their_own_styles() {
        assert!(matches!(sol_look("Io").0.style, Style::Volcanic));
        assert!(matches!(sol_look("Europa").0.style, Style::IceShell { .. }));
        assert!(matches!(sol_look("Jupiter").0.style, Style::Giant(_)));
        assert!(matches!(sol_look("Venus").0.style, Style::CloudDeck { .. }));
        assert!(matches!(sol_look("Earth").0.style, Style::Terran));
    }

    #[test]
    fn giants_have_contrast_and_the_red_spot() {
        let (look, body) = sol_look("Jupiter");
        let ctx = SurfaceContext::new(&body, false);
        let lum = |d: [f64; 3]| {
            let t = look.globe(&body, &ctx, d);
            t.rgb[0] + t.rgb[1] + t.rgb[2]
        };
        let dir = |lat: f64, lon: f64| [lat.to_radians().cos() * lon.to_radians().cos(), lat.to_radians().cos() * lon.to_radians().sin(), lat.to_radians().sin()];
        // NEB (dark) vs EZ/NTrZ (bright), averaged over longitude.
        let avg = |lat: f64| (0..36).map(|k| lum(dir(lat, k as f64 * 10.0))).sum::<f32>() / 36.0;
        assert!(avg(13.0) < avg(21.0) - 0.15, "NEB {} NTrZ {}", avg(13.0), avg(21.0));
        // The GRS is redder than its surroundings.
        let grs = look.globe(&body, &ctx, dir(-22.3, 0.9f64.to_degrees()));
        assert!(grs.rgb[0] > grs.rgb[2] * 1.4, "{:?}", grs.rgb);
    }

    #[test]
    fn hot_giants_glow_and_cold_ones_do_not() {
        let mut b = sol_look("Jupiter").1;
        b.real = false;
        b.temperature = 1800.0;
        let hot = Look::of(&b);
        assert!(hot.night_glow > 0.5 && hot.label.contains("class V"));
        b.temperature = 140.0;
        let cold = Look::of(&b);
        assert_eq!(cold.night_glow, 0.0);
        assert!(cold.label.contains("class I"));
    }

    #[test]
    fn lava_worlds_emit() {
        let mut b = sol_look("Mercury").1;
        b.real = false;
        b.temperature = 1600.0;
        let look = Look::of(&b);
        assert!(matches!(look.style, Style::Lava { .. }));
        let ctx = SurfaceContext::new(&b, false);
        let emitting = cosmogon_sim::planet::terrain::fibonacci_sphere(500).filter(|d| look.globe(&b, &ctx, *d).emit > 0.3).count();
        assert!(emitting > 50, "{emitting}");
    }

    #[test]
    fn blackbody_is_red_when_cool_and_white_when_hot() {
        let c = blackbody(1200.0);
        assert!(c[0] > c[2] * 3.0);
        let s = blackbody(6500.0);
        assert!((s[0] - s[2]).abs() < 0.1);
    }
}

#[cfg(test)]
mod timing {
    #[test]
    #[ignore]
    fn bake_times() {
        let sys = cosmogon_sim::astro::sol::sol_system();
        for b in &sys.bodies {
            let t = std::time::Instant::now();
            let _ = super::super::bake::bake_surface(b, b.name == "Earth", 2048);
            println!("{:10} {:6.2} s", b.name, t.elapsed().as_secs_f64());
        }
    }
}
