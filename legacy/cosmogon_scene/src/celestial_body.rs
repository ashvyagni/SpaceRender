use bevy_ecs::prelude::*;
use cosmogon_ecs::components::celestial::*;
use cosmogon_ecs::components::motion::Velocity;
use cosmogon_ecs::components::orbital::OrbitalElements;
use cosmogon_ecs::components::physics::*;
use cosmogon_ecs::components::renderable::*;
use cosmogon_ecs::components::transform::*;

/// Complete definition of a celestial body for spawning.
///
/// Holds all physical, visual, and orbital properties needed to construct
/// an ECS entity representing a celestial body in the simulation.
#[derive(Debug, Clone)]
pub struct CelestialBodyDef {
    /// Human-readable name (e.g. "Earth", "Jupiter").
    pub name: String,
    /// Classification of the body.
    pub body_type: BodyType,
    /// Mass in kilograms.
    pub mass: f64,
    /// Equatorial radius in meters.
    pub radius: f64,
    /// RGBA color as `[r, g, b, a]` with channels in `[0.0, 1.0]`.
    pub color: [f32; 4],
    /// Keplerian orbital elements (None for the central star).
    pub orbital: Option<OrbitalElements>,
    /// Sidereal rotation period in seconds (None for tidally locked to parent).
    pub rotation_period: Option<f64>,
    /// Axial tilt in radians relative to the orbital plane.
    pub axial_tilt: Option<f64>,
    /// Whether this body has a ring system.
    pub has_rings: bool,
    /// Inner radius of the ring system in meters (from body center).
    pub ring_inner_radius: Option<f64>,
    /// Outer radius of the ring system in meters (from body center).
    pub ring_outer_radius: Option<f64>,
    /// Atmospheric scattering parameters (None for airless bodies).
    pub atmosphere: Option<AtmosphereDef>,
    /// Additional data for stars (None for non-stellar bodies).
    pub star_data: Option<StarData>,
}

/// Classification of a celestial body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BodyType {
    /// A luminous star.
    Star,
    /// A planet orbiting a star.
    Planet,
    /// A natural satellite orbiting a planet or dwarf planet.
    Moon,
    /// A small rocky body (asteroid or comet).
    Asteroid,
    /// A dwarf planet (e.g. Pluto, Ceres).
    DwarfPlanet,
}

/// Atmospheric scattering parameters for a planet.
///
/// Uses a simplified single-scattering Rayleigh + Mie model.
#[derive(Debug, Clone, Copy)]
pub struct AtmosphereDef {
    /// Rayleigh scattering coefficients `[r, g, b]` at sea level (m⁻¹).
    pub rayleigh_scattering: [f32; 3],
    /// Rayleigh scale height in meters.
    pub rayleigh_scale_height: f32,
    /// Mie scattering coefficient (isotropic, same for all channels) in m⁻¹.
    pub mie_scattering: [f32; 3],
    /// Mie scale height in meters.
    pub mie_scale_height: f32,
    /// Distance from the surface to the top of the atmosphere in meters.
    pub atmosphere_height: f64,
}

/// Additional physical data for stars.
#[derive(Debug, Clone, Copy)]
pub struct StarData {
    /// Effective surface temperature in Kelvin.
    pub temperature: f64,
    /// Luminosity in solar luminosities (L☉).
    pub luminosity: f64,
    /// Harvard spectral classification character (O, B, A, F, G, K, M).
    pub spectral_class: char,
}

impl CelestialBodyDef {
    /// Spawn this body as an ECS entity with all appropriate components.
    ///
    /// Returns the [`Entity`] id of the newly created body.
    pub fn spawn(self, commands: &mut Commands) -> Entity {
        let mut entity = commands.spawn_empty();

        // -- Marker and name --
        entity.insert((CelestialBody, BodyName::new(&self.name)));

        // -- Body-type marker --
        match self.body_type {
            BodyType::Star => {
                let sd = self.star_data.unwrap_or(StarData {
                    temperature: 5778.0,
                    luminosity: 1.0,
                    spectral_class: 'G',
                });
                entity.insert(Star {
                    spectral_class: sd.spectral_class,
                    temperature: sd.temperature,
                });
                entity.insert(EmissiveColor {
                    color: Color::new(
                        self.color[0],
                        self.color[1],
                        self.color[2],
                        self.color[3],
                    ),
                    intensity: sd.luminosity as f32,
                });
            }
            BodyType::Planet => {
                entity.insert(Planet {
                    has_rings: self.has_rings,
                });
            }
            BodyType::Moon => {
                entity.insert(Moon);
            }
            BodyType::Asteroid => {
                entity.insert(Asteroid);
            }
            BodyType::DwarfPlanet => {
                entity.insert(Planet {
                    has_rings: self.has_rings,
                });
            }
        }

        // -- Physics --
        let volume = (4.0 / 3.0) * std::f64::consts::PI * self.radius.powi(3);
        let density = self.mass / volume;
        let surface_gravity =
            cosmogon_core::constants::G * self.mass / self.radius.powi(2);
        let escape_velocity =
            (2.0 * cosmogon_core::constants::G * self.mass / self.radius).sqrt();

        entity.insert((
            Mass(self.mass),
            Radius(self.radius),
            Density(density),
            SurfaceGravity(surface_gravity),
            EscapeVelocity(escape_velocity),
        ));

        // -- Renderable --
        entity.insert((
            RenderMesh::Sphere {
                subdivisions: 64,
            },
            Color::new(
                self.color[0],
                self.color[1],
                self.color[2],
                self.color[3],
            ),
        ));

        // -- Transform (start at origin; orbital system will move it) --
        entity.insert(TransformBundle::default());

        // -- Orbital elements --
        if let Some(orbital) = self.orbital {
            entity.insert(orbital);
            entity.insert(Velocity::default());
        }

        // -- Rotation --
        // (rotation is applied via the motion system from rotation_period)

        entity.id()
    }
}

impl Default for CelestialBodyDef {
    fn default() -> Self {
        Self {
            name: String::new(),
            body_type: BodyType::Planet,
            mass: 0.0,
            radius: 0.0,
            color: [0.5, 0.5, 0.5, 1.0],
            orbital: None,
            rotation_period: None,
            axial_tilt: None,
            has_rings: false,
            ring_inner_radius: None,
            ring_outer_radius: None,
            atmosphere: None,
            star_data: None,
        }
    }
}
