@group(0) @binding(0) var RADIANCE_OUT: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(1) var TRANSMISSION_OUT: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(2) var RADIANCE_IN: texture_3d<f32>;
@group(0) @binding(3) var IRRADIANCE: texture_3d<f32>;
@group(0) @binding(4) var IMPORTANCE: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(5) var<uniform> CASCADE: u32;

@group(1) @binding(0) var ABSORPTION: texture_3d<f32>;
@group(1) @binding(1) var SCATTERING: texture_3d<f32>;
@group(1) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(1) @binding(3) var GRADIENT: texture_3d<f32>;
@group(1) @binding(4) var SAMPLER: sampler;
@group(1) @binding(5) var<uniform> TRANSFORM: mat4x4<f32>;
@group(1) @binding(6) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(2) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(3) @binding(0) var HDRI: texture_2d<f32>;
@group(3) @binding(1) var HDRI_SAMPLER: sampler;
@group(3) @binding(2) var<uniform> HDRI_SETTINGS: HdriSettings;

const STEP_SIZE: f32 = 0.5;

const INTERVAL = array<f32, 11>(0.0, 1.0, 3.0, 7.0, 15.0, 31.0, 63.0, 127.0, 255.0, 511.0, 1023.0);

var<private> SCALE: u32;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    SCALE = textureDimensions(EXTINCTION).x / textureDimensions(IRRADIANCE).x;

    if (any(voxel >= textureDimensions(TRANSMISSION_OUT))) { return; }

    let dim_full = textureDimensions(IRRADIANCE);
    let dim = dim_full >> vec3<u32>(CASCADE);

    // Cascade Dimensions
    let probes = max(dim, vec3<u32>(1u));
    let samples = 1u << CASCADE; // Sqrt of samples

    // Thread Index
    let probe = voxel % probes;
    let sample = (voxel.xy / probes.xy) % samples;

    // Index to Ray
    let origin = (vec3<f32>(probe) + 0.5) * f32(1u << CASCADE);

    // Ray to Radiance & Importance
    var radiance_average = vec3<f32>(0.0);
    var transmission_average = vec3<f32>(0.0);
    var importance = vec4<f32>(0.0);

    for (var sx=0u; sx<2; sx++) {
        for (var sy=0u; sy<2; sy++) {
            let smp = 2 * sample + vec2<u32>(sx, sy);

            let uv = 2.0 * (vec2<f32>(smp) + 0.5) / f32(2 * samples) - 1.0;
            let direction = clarberg_equal_area_sphere(uv);

            let transmission = trace(origin, direction, INTERVAL[CASCADE], INTERVAL[CASCADE + 1]);
            transmission_average += 0.25 * transmission;

            var radiance: vec3<f32>;
            if (CASCADE == 8) { // Final Cascade
                radiance = transmission * hdri(direction, 2 * samples);
            } else { // Other Cascades
                radiance = transmission * merge(probes, probe, smp);
            }

            radiance_average += 0.25 * radiance;
            importance[2 * sy + sx] = brightness(radiance);
        }
    }

    textureStore(TRANSMISSION_OUT, voxel, pack_rgb(transmission_average));
    textureStore(RADIANCE_OUT, voxel, pack_rgb(radiance_average));

    let importance_voxel = dim_full * vec3<u32>(CASCADE / 3, CASCADE % 3, 0) + voxel;
    textureStore(IMPORTANCE, importance_voxel, normalize_max(importance));
}

fn merge(probes: vec3<u32>, probe: vec3<u32>, sample: vec2<u32>) -> vec3<f32> {
    let probes_in = max(probes >> vec3<u32>(1u), vec3<u32>(1u));

    let probe_position = clamp(
        vec3<f32>(0.5) * (vec3<f32>(probe) + vec3<f32>(0.5)),
        vec3<f32>(0.5),
        vec3<f32>(probes_in) - vec3<f32>(0.5));

    let sample_position = vec3<f32>(vec3<u32>(sample * probes_in.xy, 0u));
    let position = (sample_position + probe_position) / vec3<f32>(textureDimensions(RADIANCE_IN));

    return unpack_rgb(textureSampleLevel(RADIANCE_IN, SAMPLER, position, 0.0));
}

fn trace(origin: vec3<f32>, direction: vec3<f32>, t0: f32, t1: f32) -> vec3<f32> {
    var transmission = vec3<f32>(1.0);

    let dim = vec3<f32>(textureDimensions(IRRADIANCE));
    let origin_sample = origin / dim;
    let direction_sample = direction / dim;

    let scale = length(TRANSFORM[0].xyz) * dim.x; // voxels/mm
    let step_size = STEP_SIZE / f32(SCALE);

    for (var t=t0; t<t1; t+=step_size) {
        let sample = origin_sample + direction_sample * t;

        let extinction = unpack_rgb(textureSampleLevel(EXTINCTION, SAMPLER, sample, 0.0));

        transmission *= exp(-extinction * step_size / scale);
        if (all(transmission < vec3<f32>(1e-3)) || any(abs(sample - 0.5) > vec3<f32>(0.5))) { break; }
    }

    return transmission;
}

fn hdri(direction: vec3<f32>, N: u32) -> vec3<f32> {
    let hdri_dim = vec2<f32>(textureDimensions(HDRI));
    let lod = 0.5 * log2(hdri_dim.x * hdri_dim.y / f32(N * N));

    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world.xzy, HDRI_SETTINGS.rotation);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, lod).rgb;

    return radiance;
}

fn normalize_max(v: vec4<f32>) -> vec4<f32> {
    return v / max(max(v.x, v.y), max(v.z, v.w));
}

fn brightness(rgb: vec3<f32>) -> f32 {
    return dot(rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
}
