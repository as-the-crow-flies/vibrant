@group(0) @binding(0) var ABSORPTION: texture_3d<f32>;
@group(0) @binding(1) var SCATTERING: texture_3d<f32>;
@group(0) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(0) @binding(3) var SAMPLER: sampler;
@group(0) @binding(4) var<uniform> TRANSFORM: mat4x4<f32>;

@group(1) @binding(0) var IRRADIANCE: texture_3d<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(
        select(-1.0, 1.0, bool(index & 1)),
        select(-1.0, 1.0, bool(index & 2)),
        0.0,
        1.0
    );
}

@fragment
fn fragment(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = vec2<f32>(1.0, -1.0) * (pixel.xy / vec2<f32>(ENVIRONMENT.surface) * 2.0 - 1.0);

    let near = unproject(vec3<f32>(uv.xy, 0.0));
    let far = unproject(vec3<f32>(uv.xy, 1.0));

    let origin = TRANSFORM * vec4<f32>(near, 1.0);
    let direction = TRANSFORM * vec4<f32>(normalize(far - near), 0.0);

    return raymarch(vec2<f32>(pixel.xy), origin.xyz, direction.xyz);
}

fn raymarch(pixel: vec2<f32>, origin: vec3<f32>, direction: vec3<f32>) -> vec4<f32> {
    let light_direction = (TRANSFORM * vec4<f32>(ENVIRONMENT.light, 0.0)).xyz;

    var transmission = vec3<f32>(1.0);
    var ambient_light_scattering = vec3<f32>(0.0);
    var direct_light_scattering = vec3<f32>(0.0);

    let phase_function = 1.0 / (4.0 * PI);

    let step = 1.0;

    for (var t = 100.0; t < 400.0; t += step) {
        let position = origin + direction * (t + step * hash(pixel * step));

        let material = sample_material(position, 0.0);

        transmission *= exp(-step * material.extinction);

        ambient_light_scattering += ambient(position, 0.0) *
            step *
            phase_function *
            material.scattering *
            transmission;

        if (all(transmission <= vec3<f32>(0.01))) { break; }
    }

    return vec4<f32>(
        transmission +
        ENVIRONMENT.settings.ambient_light * ambient_light_scattering +
        ENVIRONMENT.settings.direct_light * direct_light_scattering, 1.0);
}

fn ambient(origin: vec3<f32>, level: f32) -> vec3<f32> {
    let irradiance = textureSampleLevel(IRRADIANCE, SAMPLER, origin, level).rgb;
    return irradiance;
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}

struct Material {
    absorption: vec3<f32>,
    scattering: vec3<f32>,
    extinction: vec3<f32>,
};

fn sample_material(position: vec3<f32>, level: f32) -> Material {
    let absorption = textureSampleLevel(ABSORPTION, SAMPLER, position, level).rgb * ENVIRONMENT.settings.alpha;
    let scattering = textureSampleLevel(SCATTERING, SAMPLER, position, level).rgb * ENVIRONMENT.settings.alpha;
    let extinction = absorption + scattering;

    return Material(absorption, scattering, extinction);
}

fn unproject(v: vec3<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}
