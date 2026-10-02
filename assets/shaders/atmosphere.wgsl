struct CameraUniform {
    view: mat4x4<f32>,
    projection: mat4x4<f32>,
    view_pos: vec4<f32>,
    time: f32,
};

struct AtmosphereUniform {
    planet_radius: f32,
    atmosphere_radius: f32,
    rayleigh_scattering: vec3<f32>,
    rayleigh_scale_height: f32,
    mie_scattering: vec3<f32>,
    mie_scale_height: f32,
    sun_direction: vec4<f32>,
    camera_position: vec4<f32>,
    planet_center: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(1) @binding(0) var<uniform> atmosphere: AtmosphereUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) view_ray: vec3<f32>,
};

const PI: f32 = 3.14159265359;
const NUM_SAMPLES: i32 = 16;
const NUM_LIGHT_SAMPLES: i32 = 8;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_position = in.position;
    out.view_ray = in.position - camera.view_pos.xyz;
    out.clip_position = camera.projection * camera.view * vec4<f32>(in.position, 1.0);
    return out;
}

// Ray-sphere intersection: returns (t_near, t_far)
fn ray_sphere_intersect(ray_origin: vec3<f32>, ray_dir: vec3<f32>, center: vec3<f32>, radius: f32) -> vec2<f32> {
    let oc = ray_origin - center;
    let a = dot(ray_dir, ray_dir);
    let b = 2.0 * dot(oc, ray_dir);
    let c = dot(oc, oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;

    if discriminant < 0.0 {
        return vec2<f32>(-1.0, -1.0);
    }

    let sqrt_disc = sqrt(discriminant);
    return vec2<f32>(-b - sqrt_disc, -b + sqrt_disc) / (2.0 * a);
}

fn rayleigh_phase(cos_theta: f32) -> f32 {
    return 3.0 / (16.0 * PI) * (1.0 + cos_theta * cos_theta);
}

fn mie_phase(cos_theta: f32, g: f32) -> f32 {
    let g2 = g * g;
    let num = 3.0 * (1.0 - g2) * (1.0 + cos_theta * cos_theta);
    let den = (2.0 + g2) * pow(1.0 + g2 - 2.0 * g * cos_theta, 1.5);
    return num / (4.0 * PI * den);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ray_origin = camera.view_pos.xyz;
    let ray_dir = normalize(in.view_ray);

    let planet_center = atmosphere.planet_center.xyz;
    let planet_radius = atmosphere.planet_radius;
    let atmo_radius = atmosphere.atmosphere_radius;

    // Intersect with atmosphere shell
    let atmo_hit = ray_sphere_intersect(ray_origin, ray_dir, planet_center, atmo_radius);
    if atmo_hit.x < 0.0 {
        discard;
    }

    // Check if ray hits the planet surface (occlusion)
    let planet_hit = ray_sphere_intersect(ray_origin, ray_dir, planet_center, planet_radius);

    let ray_length = select(atmo_hit.y, planet_hit.x, planet_hit.x > 0.0 && planet_hit.x < atmo_hit.y);

    let sun_dir = normalize(atmosphere.sun_direction.xyz);

    var total_rayleigh = vec3<f32>(0.0, 0.0, 0.0);
    var total_mie = vec3<f32>(0.0, 0.0, 0.0);

    let step_size = ray_length / f32(NUM_SAMPLES);
    var optical_depth_rayleigh = 0.0;
    var optical_depth_mie = 0.0;

    var t = atmo_hit.x;
    for (var i = 0; i < NUM_SAMPLES; i++) {
        let sample_pos = ray_origin + ray_dir * (t + step_size * 0.5);

        let height = length(sample_pos - planet_center) - planet_radius;

        // Density falloff
        let rayleigh_density = exp(-height / atmosphere.rayleigh_scale_height);
        let mie_density = exp(-height / atmosphere.mie_scale_height);

        optical_depth_rayleigh += rayleigh_density * step_size;
        optical_depth_mie += mie_density * step_size;

        // Light sampling: accumulate scattering along sun ray
        var light_optical_rayleigh = 0.0;
        var light_optical_mie = 0.0;
        let light_hit = ray_sphere_intersect(sample_pos, sun_dir, planet_center, atmo_radius);

        if light_hit.x >= 0.0 {
            let light_step = light_hit.y / f32(NUM_LIGHT_SAMPLES);
            var light_t = light_hit.x;

            for (var j = 0; j < NUM_LIGHT_SAMPLES; j++) {
                let light_sample = sample_pos + sun_dir * (light_t + light_step * 0.5);
                let light_height = length(light_sample - planet_center) - planet_radius;
                light_optical_rayleigh += exp(-light_height / atmosphere.rayleigh_scale_height) * light_step;
                light_optical_mie += exp(-light_height / atmosphere.mie_scale_height) * light_step;
                light_t += light_step;
            }
        }

        let attenuation_rayleigh = exp(-atmosphere.rayleigh_scattering * (optical_depth_rayleigh + light_optical_rayleigh));
        let attenuation_mie = exp(-atmosphere.mie_scattering * (optical_depth_mie + light_optical_mie));

        total_rayleigh += rayleigh_density * attenuation_rayleigh * step_size;
        total_mie += mie_density * attenuation_mie * step_size;

        t += step_size;
    }

    // Scattering
    let cos_theta = dot(ray_dir, sun_dir);
    let rayleigh = total_rayleigh * atmosphere.rayleigh_scattering * rayleigh_phase(cos_theta);
    let mie = total_mie * atmosphere.mie_scattering * mie_phase(cos_theta, 0.758);

    // Sun color (slightly warm white)
    let sun_color = vec3<f32>(1.0, 0.95, 0.8);

    var color = (rayleigh + mie) * sun_color;
    let alpha = min(length(color) * 5.0, 0.95);

    return vec4<f32>(color, alpha);
}
