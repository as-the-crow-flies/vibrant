@group(0) @binding(0) var RADIANCE_OUT: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(1) var TRANSMISSION_OUT: texture_storage_3d<rgba8unorm, write>;
@group(0) @binding(2) var RADIANCE_IN: texture_3d<f32>;
@group(0) @binding(3) var IRRADIANCE: texture_3d<f32>;
@group(0) @binding(4) var<uniform> CASCADE: u32;

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

    // Cascade Dimensions
    let probes = max(textureDimensions(IRRADIANCE) >> vec3<u32>(CASCADE), vec3<u32>(1u));
    let samples = 3u << CASCADE; // Sqrt of samples
    let samples_2 = samples * samples;

    // Thread Index
    let probe = voxel % probes;
    let sample = (voxel.xy / probes.xy) % samples;

    // Index to Ray
    let origin = (vec3<f32>(probe) + 0.5) * f32(1u << CASCADE);
    let uv = 2.0 * (vec2<f32>(sample)) / f32(samples) - 1.0;
    let direction = octahedron_decode(uv);

    // Ray to Radiance
    let transmission = trace(origin, direction, INTERVAL[CASCADE], INTERVAL[CASCADE + 1]);
    var radiance = vec3<f32>(0.0);

    if (CASCADE == 9) { // Final Cascade
        radiance = hdri(direction, samples_2);
    } else { // Other Cascades
        let probes_in = max(probes >> vec3<u32>(1u), vec3<u32>(1u));
        let samples_in = samples << 1u;

        let sample_in = sample << vec2<u32>(1u);

        let probe_position = clamp(
            vec3<f32>(0.5) * (vec3<f32>(probe) + vec3<f32>(0.5)),
            vec3<f32>(0.5),
            vec3<f32>(probes_in) - vec3<f32>(0.5));

        for (var sub_x = 0u; sub_x < 2u; sub_x++) {
            for (var sub_y = 0u; sub_y < 2u; sub_y++) {
                let sample_position = vec3<f32>(vec3<u32>((sample_in + vec2<u32>(sub_x, sub_y)) * probes_in.xy, 0u));
                let position = (sample_position + probe_position) / vec3<f32>(textureDimensions(RADIANCE_IN));

                radiance += 0.25 * unpack_rgb(textureSampleLevel(RADIANCE_IN, SAMPLER, position, 0.0));
            }
        }
    }

    textureStore(TRANSMISSION_OUT, voxel, pack_rgb(transmission));
    textureStore(RADIANCE_OUT, voxel, pack_rgb(transmission * radiance));
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
    let lod = 0.5 * log2(hdri_dim.x * hdri_dim.y / f32(N));

    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world.xzy, HDRI_SETTINGS.rotation);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, lod).rgb;

    return radiance;
}
