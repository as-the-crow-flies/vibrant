@group(0) @binding(0) var POSITION: texture_2d<f32>;
@group(0) @binding(1) var NORMAL: texture_2d<f32>;
@group(0) @binding(2) var TANGENT: texture_2d<f32>;

@group(1) @binding(0) var DENSITY: texture_3d<f32>;
@group(1) @binding(1) var SAMPLER: sampler;

@group(2) @binding(0) var<uniform> TRACTOGRAM_TO_WORLD: mat4x4<f32>;
@group(2) @binding(1) var<uniform> WORLD_TO_TRACTOGRAM: mat4x4<f32>;

@group(3) @binding(0) var<uniform> ENVIRONMENT: Environment;

const PI: f32 = 3.1415926535897932;
const PHI = 1.6180339887498948482045868;

@vertex
fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4<f32>(2.0 * vec2<f32>(f32((index & 1) == 0), f32((index & 2) == 0)) - 1.0, 0.0, 1.0);
}

@fragment
fn fragment(@builtin(position) uv: vec4<f32>) -> @location(0) vec4<f32> {
    let camera = ENVIRONMENT.camera.transform[3].xyz;
    let light = normalize(ENVIRONMENT.light);

    let position = textureLoad(POSITION, vec2<u32>(uv.xy), 0);
    let normal = textureLoad(NORMAL, vec2<u32>(uv.xy), 0) * 2.0 - 1.0;
    let tangent = textureLoad(TANGENT, vec2<u32>(uv.xy), 0) * 2.0 - 1.0;

    if (position.w == 0.0) { discard; }

    let radius = length(TRACTOGRAM_TO_WORLD * vec4<f32>(ENVIRONMENT.settings.streamline_radius, 0.0, 0.0, 0.0));

    let view = normalize(camera - position.xyz);

    let lighting = ambient(position.xyz, radius) + direct(position.xyz, light, radius) *
        select(lambert(normal.xyz, light), stalling(tangent.xyz, light), normal.w < 0.5);

    let tangent_object_space = normalize(TRACTOGRAM_TO_WORLD * vec4<f32>(tangent.xyz, 0.0));

    let color = mix(vec3<f32>(1.0), abs(tangent_object_space.xyz), ENVIRONMENT.settings.gradient_factor);
    let illumination = mix(1.0, lighting, ENVIRONMENT.settings.shading_level);

    return vec4<f32>(color * illumination, 1.0);
}

fn lambert(normal: vec3<f32>, light: vec3<f32>) -> f32 {
    return max(0.0, dot(normal, light));
}

// Stalling et al. 1997 - Fast Display of Illuminated Field Lines
fn stalling(tangent: vec3<f32>, light: vec3<f32>) -> f32 {
    return max(0.0, sqrt(1.0 - pow(dot(light, tangent), 2.0)));
}

fn direct(position: vec3<f32>, light: vec3<f32>, radius: f32) -> f32 {
    if (ENVIRONMENT.settings.direct_light == 0.0) { return 0.0; }

    let step = (1.0 / f32(textureDimensions(DENSITY).x)) * light;
    let start = position + 0.5 + radius * light;

    var occlusion = 0.0;
    var level = 0.5;

    for (var sample = start; in_domain(sample); sample += step) {
        occlusion += (1.0 - occlusion) * textureSampleLevel(DENSITY, SAMPLER, sample, level).x;
    }

    return ENVIRONMENT.settings.direct_light * max(0.0, 1.0 - occlusion);
}

fn ambient(position: vec3<f32>, radius: f32) -> f32 {
    if (ENVIRONMENT.settings.direct_light == 1.0) { return 0.0; }

    let N_SAMPLES = f32(ENVIRONMENT.settings.ambient_occlusion_samples);
    let TAN_CONE_ANGLE = tan(sqrt(4.0 * PI / N_SAMPLES));
    let DIM = f32(textureDimensions(DENSITY).x);
    let distance_start = 1.0 / DIM + radius;

    var total_occlusion = 0.0;

    for (var i = 0.0; i < N_SAMPLES; i += 1.0) {
        let direction = fibonacci_sphere(N_SAMPLES, i);

        var occlusion = 0.0;
        var size = 0.0;

        for (var distance = distance_start; distance < 1.0; distance *= 2.0) {
            let sample = position + direction * distance + 0.5;

            if (!in_domain(sample) || occlusion > 0.99) { break; }

            let level = log2(TAN_CONE_ANGLE * distance * DIM);
            occlusion += (1.0 - occlusion) * textureSampleLevel(DENSITY, SAMPLER, sample, level).x;
        }

        total_occlusion += occlusion;
    }

    return (1.0 - ENVIRONMENT.settings.direct_light) * (1.0 - (total_occlusion / N_SAMPLES));
}

fn in_domain(v: vec3<f32>) -> bool {
    return all(v >= vec3<f32>(0.0)) && all(v <= vec3<f32>(1.0));
}

// Fibonacci Sphere Sampling in WGSL
fn fibonacci_sphere(n: f32, i: f32) -> vec3<f32> {
    let theta = 2.0 * PI * i / PHI;
    let z = 1.0 - (2.0 * i + 1.0) / n;
    let radius = sqrt(1.0 - z * z);

    let x = radius * cos(theta);
    let y = radius * sin(theta);

    return vec3<f32>(x, y, z);
}
