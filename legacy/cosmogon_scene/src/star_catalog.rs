use cosmogon_core::math::Vec3d;

/// A single entry in the star catalog.
#[derive(Debug, Clone)]
pub struct StarEntry {
    /// Common name of the star.
    pub name: String,
    /// Position in light-years relative to the Sun.
    pub position: Vec3d,
    /// Apparent visual magnitude (lower = brighter).
    pub magnitude: f64,
    /// Harvard spectral classification character (O, B, A, F, G, K, M).
    pub spectral_class: char,
    /// Effective surface temperature in Kelvin.
    pub temperature: f64,
}

/// Return a catalog of ~50 well-known stars with real positions and data.
///
/// Positions are in light-years from the Sun. Distances are approximate
/// Hipparcos / Gaia values. Spectral types and temperatures are from
/// the Bright Star Catalogue (BSC5) and SIMBAD.
pub fn basic_star_catalog() -> Vec<StarEntry> {
    vec![
        // Nearby stars
        StarEntry { name: "Alpha Centauri A".into(), position: Vec3d::new(4.37, -1.42, -2.71), magnitude: -0.27, spectral_class: 'G', temperature: 5790.0 },
        StarEntry { name: "Alpha Centauri B".into(), position: Vec3d::new(4.37, -1.42, -2.71), magnitude: 1.33, spectral_class: 'K', temperature: 5260.0 },
        StarEntry { name: "Barnard's Star".into(), position: Vec3d::new(5.96, 0.32, -1.82), magnitude: 9.51, spectral_class: 'M', temperature: 3134.0 },
        StarEntry { name: "Wolf 359".into(), position: Vec3d::new(7.86, 0.28, -1.33), magnitude: 13.44, spectral_class: 'M', temperature: 2800.0 },
        StarEntry { name: "Lalande 21185".into(), position: Vec3d::new(8.31, 3.15, -0.83), magnitude: 7.47, spectral_class: 'M', temperature: 3828.0 },
        StarEntry { name: "Sirius A".into(), position: Vec3d::new(8.60, -0.32, -1.23), magnitude: -1.46, spectral_class: 'A', temperature: 9940.0 },
        StarEntry { name: "Luyten's Star".into(), position: Vec3d::new(12.36, 1.04, -0.68), magnitude: 9.86, spectral_class: 'M', temperature: 3380.0 },
        StarEntry { name: "Ross 154".into(), position: Vec3d::new(9.69, 0.39, -0.46), magnitude: 10.43, spectral_class: 'M', temperature: 3340.0 },
        StarEntry { name: "Ross 248".into(), position: Vec3d::new(10.32, 0.52, -0.15), magnitude: 12.29, spectral_class: 'M', temperature: 2799.0 },
        StarEntry { name: "Epsilon Eridani".into(), position: Vec3d::new(10.48, -0.11, -1.57), magnitude: 3.72, spectral_class: 'K', temperature: 5098.0 },

        // Bright stars visible from Earth
        StarEntry { name: "Vega".into(), position: Vec3d::new(25.04, 19.52, -4.66), magnitude: 0.03, spectral_class: 'A', temperature: 9602.0 },
        StarEntry { name: "Capella Aa".into(), position: Vec3d::new(42.21, -13.63, -20.18), magnitude: 0.08, spectral_class: 'G', temperature: 4940.0 },
        StarEntry { name: "Arcturus".into(), position: Vec3d::new(36.70, 8.98, 14.18), magnitude: -0.05, spectral_class: 'K', temperature: 4286.0 },
        StarEntry { name: "Procyon A".into(), position: Vec3d::new  (11.46, -0.83, -3.14), magnitude: 0.34, spectral_class: 'F', temperature: 6530.0 },
        StarEntry { name: "Achernar".into(), position: Vec3d::new(139.36, -41.33, -73.60), magnitude: 0.46, spectral_class: 'B', temperature: 14500.0 },
        StarEntry { name: "Betelgeuse".into(), position: Vec3d::new(547.93, -105.42, -307.59), magnitude: 0.42, spectral_class: 'M', temperature: 3600.0 },
        StarEntry { name: "Hadar".into(), position: Vec3d::new(525.05, -105.83, -224.80), magnitude: 0.61, spectral_class: 'B', temperature: 25000.0 },
        StarEntry { name: "Rigel".into(), position: Vec3d::new(862.76, -162.51, -446.78), magnitude: 0.13, spectral_class: 'B', temperature: 12100.0 },
        StarEntry { name: "Proxima Centauri".into(), position: Vec3d::new(4.24, -1.34, -2.57), magnitude: 11.13, spectral_class: 'M', temperature: 3042.0 },
        StarEntry { name: "Altair".into(), position: Vec3d::new(16.81, 10.15, -1.46), magnitude: 0.77, spectral_class: 'A', temperature: 8570.0 },
        StarEntry { name: "Aldebaran".into(), position: Vec3d::new(21.51, -6.58, -13.60), magnitude: 0.85, spectral_class: 'K', temperature: 3910.0 },
        StarEntry { name: "Antares".into(), position: Vec3d::new(550.26, -113.91, -344.00), magnitude: 1.06, spectral_class: 'M', temperature: 3660.0 },
        StarEntry { name: "Spica".into(), position: Vec3d::new(250.43, -45.38, -102.94), magnitude: 0.97, spectral_class: 'B', temperature: 25300.0 },
        StarEntry { name: "Pollux".into(), position: Vec3d::new(33.75, 4.11, -12.74), magnitude: 1.14, spectral_class: 'K', temperature: 4666.0 },
        StarEntry { name: "Fomalhaut".into(), position: Vec3d::new(25.13, -1.38, -4.17), magnitude: 1.16, spectral_class: 'A', temperature: 8590.0 },
        StarEntry { name: "Deneb".into(), position: Vec3d::new(2615.43, -193.90, -698.17), magnitude: 1.25, spectral_class: 'A', temperature: 8525.0 },
        StarEntry { name: "Mimosa".into(), position: Vec3d::new(280.43, -68.09, -121.81), magnitude: 1.25, spectral_class: 'B', temperature: 27000.0 },
        StarEntry { name: "Regulus".into(), position: Vec3d::new(77.51, -18.83, -41.44), magnitude: 1.36, spectral_class: 'B', temperature: 12460.0 },
        StarEntry { name: "Adhara".into(), position: Vec3d::new(431.47, -97.23, -240.02), magnitude: 1.50, spectral_class: 'B', temperature: 22800.0 },
        StarEntry { name: "Castor A".into(), position: Vec3d::new(50.97, 3.33, -22.12), magnitude: 1.58, spectral_class: 'A', temperature: 9740.0 },
        StarEntry { name: "Shaula".into(), position: Vec3d::new(571.73, -135.68, -262.23), magnitude: 1.62, spectral_class: 'B', temperature: 25000.0 },
        StarEntry { name: "Bellatrix".into(), position: Vec3d::new(252.52, -46.00, -130.60), magnitude: 1.64, spectral_class: 'B', temperature: 21800.0 },
        StarEntry { name: "Elnath".into(), position: Vec3d::new(131.26, -5.35, -70.90), magnitude: 1.65, spectral_class: 'B', temperature: 22400.0 },
        StarEntry { name: "Alnilam".into(), position: Vec3d::new(2000.74, -305.80, -887.84), magnitude: 1.69, spectral_class: 'B', temperature: 27500.0 },
        StarEntry { name: "Alioth".into(), position: Vec3d::new(80.87, 18.30, -34.21), magnitude: 1.77, spectral_class: 'A', temperature: 9020.0 },
        StarEntry { name: "Dubhe".into(), position: Vec3d::new(124.38, 12.53, -66.57), magnitude: 1.79, spectral_class: 'K', temperature: 4660.0 },
        StarEntry { name: "Mirfak".into(), position: Vec3d::new(510.12, 15.24, -246.99), magnitude: 1.80, spectral_class: 'F', temperature: 6570.0 },
        StarEntry { name: "Kaus Australis".into(), position: Vec3d::new(143.18, -23.73, -93.48), magnitude: 1.85, spectral_class: 'B', temperature: 9800.0 },
        StarEntry { name: "Wezen".into(), position: Vec3d::new(1798.68, -357.30, -795.30), magnitude: 1.84, spectral_class: 'F', temperature: 5400.0 },
        StarEntry { name: "Alkaid".into(), position: Vec3d::new(100.88, 11.75, -44.43), magnitude: 1.86, spectral_class: 'B', temperature: 15540.0 },
        StarEntry { name: "Sargas".into(), position: Vec3d::new(272.24, -67.09, -162.53), magnitude: 1.87, spectral_class: 'F', temperature: 7268.0 },
        StarEntry { name: "Avior".into(), position: Vec3d::new(631.72, -110.41, -290.34), magnitude: 1.86, spectral_class: 'K', temperature: 4430.0 },
        StarEntry { name: "Alphard".into(), position: Vec3d::new(176.63, -21.83, -85.23), magnitude: 1.98, spectral_class: 'K', temperature: 4105.0 },
        StarEntry { name: "Polaris".into(), position: Vec3d::new(433.55, 58.35, -237.23), magnitude: 1.98, spectral_class: 'F', temperature: 6015.0 },
        StarEntry { name: "Algieba".into(), position: Vec3d::new(126.04, 11.56, -67.51), magnitude: 2.08, spectral_class: 'K', temperature: 4490.0 },
        StarEntry { name: "Zubeneschamali".into(), position: Vec3d::new(185.13, -20.99, -91.17), magnitude: 2.61, spectral_class: 'B', temperature: 12300.0 },
        StarEntry { name: "Sabik".into(), position: Vec3d::new(84.13, -7.44, -37.64), magnitude: 2.43, spectral_class: 'A', temperature: 8760.0 },
        StarEntry { name: "Acrux".into(), position: Vec3d::new(321.37, -86.75, -175.56), magnitude: 0.76, spectral_class: 'B', temperature: 28000.0 },
        StarEntry { name: "Mizar A".into(), position: Vec3d::new(78.43, 10.45, -34.76), magnitude: 2.27, spectral_class: 'A', temperature: 9000.0 },
        StarEntry { name: "Saiph".into(), position: Vec3d::new(650.40, -114.12, -310.86), magnitude: 2.09, spectral_class: 'B', temperature: 26500.0 },
    ]
}

/// Map a stellar effective temperature to an approximate RGB color.
///
/// Uses a simplified blackbody-to-RGB mapping. The resulting color is
/// suitable for star rendering (emissive, no albedo).
///
/// | Class | Temperature      | Approx. Color |
/// |-------|------------------|---------------|
/// | O     | ≥ 30 000 K       | Blue-violet   |
/// | B     | 10 000 – 30 000 K| Blue-white    |
/// | A     | 7 500 – 10 000 K | White         |
/// | F     | 6 000 – 7 500 K  | Yellow-white  |
/// | G     | 5 200 – 6 000 K  | Yellow        |
/// | K     | 3 700 – 5 200 K  | Orange        |
/// | M     | < 3 700 K        | Red           |
pub fn star_color(temperature: f64) -> [f32; 3] {
    // Attempt at a perceptually reasonable mapping.
    // Based on "Tanner Helland – Approximating Colors of Stars"
    // (simplified, not physically exact).
    let temp = temperature.clamp(1000.0, 40000.0);
    let t = temp / 100.0;

    // Red channel
    let r = if t <= 66.0 {
        1.0
    } else {
        let r = 329.698727446 * (t - 60.0).powf(-0.1332047592);
        (r / 255.0).clamp(0.0, 1.0) as f32
    };

    // Green channel
    let g = if t <= 66.0 {
        let g = 99.4708025861 * t.ln() - 161.1195681661;
        (g / 255.0).clamp(0.0, 1.0) as f32
    } else {
        let g = 288.1221695283 * (t - 60.0).powf(-0.0755148492);
        (g / 255.0).clamp(0.0, 1.0) as f32
    };

    // Blue channel
    let b = if t >= 66.0 {
        1.0
    } else if t <= 19.0 {
        0.0
    } else {
        let b = 138.5177312231 * (t - 10.0).ln() - 305.0447927307;
        (b / 255.0).clamp(0.0, 1.0) as f32
    };

    [r, g, b]
}
