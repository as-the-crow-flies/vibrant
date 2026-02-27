@group(0) @binding(0) var ABSORPTION: texture_3d<f32>;
@group(0) @binding(1) var SCATTERING: texture_3d<f32>;
@group(0) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(0) @binding(3) var GRADIENT: texture_3d<f32>;
@group(0) @binding(4) var SAMPLER: sampler;
@group(0) @binding(5) var<uniform> TRANSFORM: mat4x4<f32>;

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

    let origin = (TRANSFORM * vec4<f32>(near, 1.0)).xyz;
    let direction = (TRANSFORM * vec4<f32>(normalize(far - near), 0.0)).xyz;

    let hit = intersectAABB(origin, direction);

    if (hit.x > hit.y) {
        return vec4<f32>(1.0, 1.0, 1.0, 0.0);
    }

    let t0 = max(hit.x, 0.0) + hash(pixel.xy + fract(ENVIRONMENT.time));
    let t1 = hit.y;

    return raymarch(vec2<f32>(pixel.xy), origin, direction, t0, t1);
}

fn raymarch(pixel: vec2<f32>, origin: vec3<f32>, direction: vec3<f32>, t0: f32, t1: f32) -> vec4<f32> {
    let light_direction = (TRANSFORM * vec4<f32>(ENVIRONMENT.light, 0.0)).xyz;

    var transmittance = vec3<f32>(1.0);
    var color = vec3<f32>(0.0);

    let phase_function = 1.0 / (4.0 * PI);

    var step = ENVIRONMENT.settings.radius;

    let direction_norm = normalize(direction);

    for (var t = t0; t < t1; t += step) {
        let sample = origin + direction * t;

        let material = sample_material(sample);
        let extinction = step * material.extinction;

        if (any(extinction < vec3<f32>(1E-5))) { continue; }

        let gradient = sample_gradient(sample);
        let gradient_norm = gradient.xyz / (gradient.a + 1E-5);

        let normal_offset = 0.01 * gradient_norm * ENVIRONMENT.settings.lighting;
        let light = ENVIRONMENT.settings.ambient_light * sample_ambient(sample);
        let light_sample = light * material.scattering * phase_function;

        let transmittance_in_step = 1.0 - exp(-extinction);

        // let gradient_color = normalize(abs(gradient.xyz) + 1E-5);
        // color += gradient_color * transmittance * transmittance_in_step;

        color += transmittance * transmittance_in_step * light_sample;

        transmittance *= exp(-extinction);

        // if (all(transmittance <= vec3<f32>(0.01))) { break; }
    }

    return vec4<f32>(transmittance + color.rgb, 1.0);
}

fn sample_ambient(origin: vec3<f32>) -> vec3<f32> {
    return textureSampleLevel(IRRADIANCE, SAMPLER, origin, 0.0).rgb;
}

fn minimum(v: vec3<f32>) -> f32 {
    return min(min(v.x, v.y), v.z);
}

struct Material {
    absorption: vec3<f32>,
    scattering: vec3<f32>,
    extinction: vec3<f32>,
};

fn sample_material(sample: vec3<f32>) -> Material {
    let absorption = tex(ABSORPTION, sample).rgb * ENVIRONMENT.settings.alpha;
    let scattering = tex(SCATTERING, sample).rgb * ENVIRONMENT.settings.alpha;
    let extinction = absorption + scattering;

    return Material(absorption, scattering, extinction);
}

fn sample_gradient(sample: vec3<f32>) -> vec4<f32> {
    let gradient_raw = tex(GRADIENT, sample);
    return vec4<f32>(2.0 * gradient_raw.xyz - 1.0, gradient_raw.a);
}

struct ColorGradient {
    color: vec3<f32>,
    gradient: vec3<f32>
}

fn sample_extinction(position: vec3<f32>) -> ColorGradient {
    let extinction = textureSampleLevel(EXTINCTION, SAMPLER, position, 0.0).rgb;

    let sample = position * vec3<f32>(1.0, 1.0, 0.333333);

    let dx = textureSampleLevel(GRADIENT, SAMPLER, sample + vec3<f32>(0.0, 0.0, 0.000000), 0.0).rgb;
    let dy = textureSampleLevel(GRADIENT, SAMPLER, sample + vec3<f32>(0.0, 0.0, 0.333333), 0.0).rgb;
    let dz = textureSampleLevel(GRADIENT, SAMPLER, sample + vec3<f32>(0.0, 0.0, 0.666666), 0.0).rgb;

    let l = fract(position * vec3<f32>(textureDimensions(EXTINCTION)));

    let color = extinction
        + (1.0 - l.x) * l.x * dx
        + (1.0 - l.y) * l.y * dy
        + (1.0 - l.z) * l.z * dz;

    let gradient = dx + dy + dz;

    if (ENVIRONMENT.settings.lighting > 0.5) { return ColorGradient(extinction, gradient); }

    return ColorGradient(color, gradient);
}

fn unproject(v: vec3<f32>) -> vec3<f32> {
    let t = ENVIRONMENT.camera.projection_inverse * vec4<f32>(v, 1.0);
    return t.xyz / t.w;
}

fn intersectAABB(origin: vec3<f32>, direction: vec3<f32>) -> vec2<f32> {
    let boxMin = vec3<f32>(0.0);
    let boxMax = vec3<f32>(1.0);

    let invDir = 1.0 / direction;

    let t1 = (boxMin - origin) * invDir;
    let t2 = (boxMax - origin) * invDir;

    let tMinVec = min(t1, t2);
    let tMaxVec = max(t1, t2);

    let tMin = max(max(tMinVec.x, tMinVec.y), tMinVec.z);
    let tMax = min(min(tMaxVec.x, tMaxVec.y), tMaxVec.z);

    return vec2<f32>(tMin, tMax);
}

fn adaptive_step(density: f32, gradient: f32, min_step: f32, max_step: f32) -> f32 {
    let eps = 1e-5;
    let step = min(0.5 / (gradient + eps), 0.5 / (density + eps));
    return clamp(step, min_step, max_step);
}

fn tex(tex: texture_3d<f32>, coords: vec3<f32>) -> vec4<f32> {
    return textureSampleLevel(tex, SAMPLER, coords, 0.0);
}
