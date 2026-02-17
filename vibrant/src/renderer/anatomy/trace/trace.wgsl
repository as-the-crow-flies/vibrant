@group(0) @binding(0) var ABSORPTION_TRANSMISSION: texture_3d<f32>;
@group(0) @binding(1) var SCATTERING_ROUGHNESS: texture_3d<f32>;
@group(0) @binding(2) var SAMPLER: sampler;
@group(0) @binding(3) var<uniform> TRANSFORM: mat4x4<f32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

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
    var direct_light_scattering = vec3<f32>(0.0);

    let phase_function = 1.0 / (4.0 * PI);

    let step = 1.0;

    for (var t = 0.0; t < 500.0; t += step) {
        let position = origin + direction * t;

        let material = sample_material(position, 0.0);

        transmission *= exp(-step * material.extinction);

        if (any(material.scattering > vec3<f32>(0.0))) {
            direct_light_scattering += direct(position, light_direction) *
                phase_function *
                material.scattering *
                transmission;
        }

        if (all(transmission <= vec3<f32>(0.001))) { break; }
    }

    return vec4<f32>(
        transmission +
        ENVIRONMENT.settings.direct_light * direct_light_scattering, 1.0);
}

fn direct(origin: vec3<f32>, direction: vec3<f32>) -> vec3<f32> {
    let step = 2.0;
    var optical_depth = vec3<f32>(0.0);

    for (var t = step; t < 100.0; t += step) {
        let position = origin + direction * t;

        let material = sample_material(position, 0.0);

        optical_depth += step * material.extinction;

        if (all(optical_depth >= vec3<f32>(3.0))) { break; }
    }

    return exp(-optical_depth);
}

fn ambient(origin: vec3<f32>) -> vec3<f32> {
    let dim = minimum(vec3<f32>(textureDimensions(ABSORPTION_TRANSMISSION)));

    var total = vec3<f32>(0.0);

    for (var i=0u; i<12; i++) {
        var direction = (TRANSFORM * vec4<f32>(ICOSAHEDRON[i], 0.0)).xyz;

        var optical_depth = vec3<f32>(0.0);

        for (var distance = 1.0; distance < dim; distance *= 2.0) {
            let position = origin + direction * distance;
            let level = log2(TAN_CONE_ANGLE * distance);

            let material = sample_material(position, level);

            optical_depth += material.extinction;
        }

        total += optical_depth;
    }

    return exp(-total * ONE_OVER_TWELVE);
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
    let absorption = textureSampleLevel(ABSORPTION_TRANSMISSION, SAMPLER, position, level).rgb;
    let scattering = textureSampleLevel(SCATTERING_ROUGHNESS, SAMPLER, position, level).rgb;
    let extinction = absorption + scattering;

    return Material(absorption, scattering, extinction);
}

fn unproject(v: vec3<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}
