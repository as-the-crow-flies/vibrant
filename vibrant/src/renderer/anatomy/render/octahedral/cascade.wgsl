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

const INTERVAL = array<f32, 7>(0.0, 1.0, 3.0, 7.0, 15.0, 31.0, 63.0);

var<private> SCALE: u32;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) voxel: vec3<u32>) {
    SCALE = textureDimensions(EXTINCTION).x / textureDimensions(IRRADIANCE).x;

    let probes = textureDimensions(IRRADIANCE) >> vec3<u32>(CASCADE);
    let subdivisions = 1u << CASCADE; // Sqrt of subdivisions
    let subdivisions_2 = subdivisions * subdivisions;

    // Cascade Index: Octant -> Subdivision -> Probe
    let octant = voxel / (probes * vec3<u32>(subdivisions, subdivisions, 1u));
    let subdivision = (voxel.xy / probes.xy) % subdivisions;
    let probe = voxel % probes;

    if (any(octant > vec3<u32>(1u))) { return; }

    // Index to Ray
    let origin = vec3<f32>(probe * (1u << CASCADE)) + 0.5 * vec3<f32>(f32(1u << CASCADE));
    let direction = octahedron(octant, subdivision, subdivisions);

    // Ray to Radiance
    let transmission = trace(origin, direction, INTERVAL[CASCADE], INTERVAL[CASCADE + 1]);
    var radiance = vec3<f32>(0.0);

    if (CASCADE == 5) { // Final Cascade
        radiance = hdri(direction, subdivisions_2);
    } else { // Other Cascades
        let probes_in = probes >> vec3<u32>(1u);
        let subdivisions_in = subdivisions << 1u;

        let dim_in = vec3<f32>(2u * probes_in * vec3<u32>(subdivisions_in, subdivisions_in, 1u));

        let subdivision_in = subdivision << vec2<u32>(1u);

        let octant_position = vec3<f32>(octant * probes_in * vec3<u32>(subdivisions_in, subdivisions_in, 1u));
        let probe_position = clamp(
            vec3<f32>(0.5) * (vec3<f32>(probe) + vec3<f32>(0.5)),
            vec3<f32>(0.5),
            vec3<f32>(probes_in) - vec3<f32>(0.5));

        for (var sub_x = 0u; sub_x < 2u; sub_x++) {
            for (var sub_y = 0u; sub_y < 2u; sub_y++) {
                let subdivision_position = vec3<f32>(vec3<u32>((subdivision_in + vec2<u32>(sub_x, sub_y)) * probes_in.xy, 0u));

                let position = octant_position + subdivision_position + probe_position;
                let sample = position / vec3<f32>(textureDimensions(RADIANCE_IN));

                radiance += 0.25 * unpack_rgb(textureSampleLevel(RADIANCE_IN, SAMPLER, sample, 0.0));
            }
        }
    }

    textureStore(TRANSMISSION_OUT, voxel, pack_rgb(transmission));
    textureStore(RADIANCE_OUT, voxel, pack_rgb(transmission * radiance));
}

fn trace(origin: vec3<f32>, direction: vec3<f32>, t0: f32, t1: f32) -> vec3<f32> {
    var transmission = vec3<f32>(1.0);

    if (HDRI_SETTINGS.specular < 0.5) { return transmission; }

    let dim = vec3<f32>(textureDimensions(IRRADIANCE));
    let origin_sample = origin / dim;
    let direction_sample = direction / dim;

    let scale = length(TRANSFORM[0].xyz) * dim.x; // voxels/mm
    let step_size = STEP_SIZE / f32(SCALE);

    for (var t=t0; t<t1; t+=step_size) {
        let sample = origin_sample + direction_sample * t;

        let extinction = unpack_rgb(textureSampleLevel(EXTINCTION, SAMPLER, sample, 0.0));

        transmission *= exp(-extinction * step_size / scale);
    }

    return transmission;
}

fn visibility_bias(probe: vec3<f32>) -> vec3<f32> {
    let probe_min = vec3<u32>(probe);

    let mip = CASCADE + 1u + countTrailingZeros(SCALE);

    let e000 = length(unpack_rgb(textureLoad(EXTINCTION, probe_min + vec3<u32>(0, 0, 0), mip)));
    let e001 = length(unpack_rgb(textureLoad(EXTINCTION, probe_min + vec3<u32>(0, 0, 1), mip)));
    let e010 = length(unpack_rgb(textureLoad(EXTINCTION, probe_min + vec3<u32>(0, 1, 0), mip)));
    let e100 = length(unpack_rgb(textureLoad(EXTINCTION, probe_min + vec3<u32>(1, 0, 0), mip)));
    let e011 = length(unpack_rgb(textureLoad(EXTINCTION, probe_min + vec3<u32>(0, 1, 1), mip)));
    let e101 = length(unpack_rgb(textureLoad(EXTINCTION, probe_min + vec3<u32>(1, 0, 1), mip)));
    let e110 = length(unpack_rgb(textureLoad(EXTINCTION, probe_min + vec3<u32>(1, 1, 0), mip)));
    let e111 = length(unpack_rgb(textureLoad(EXTINCTION, probe_min + vec3<u32>(1, 1, 1), mip)));

    let sum_e = e000 + e001 + e010 + e100 + e011 + e101 + e110 + e111;
    if (sum_e == 0.0) { return probe; }

    let norm = 1.0 / sum_e;

    let bias =
        vec3<f32>(-0.5, -0.5, -0.5) * (1.0 - e000 * norm) +
        vec3<f32>(-0.5, -0.5,  0.5) * (1.0 - e001 * norm) +
        vec3<f32>(-0.5,  0.5, -0.5) * (1.0 - e010 * norm) +
        vec3<f32>( 0.5, -0.5, -0.5) * (1.0 - e100 * norm) +
        vec3<f32>(-0.5,  0.5,  0.5) * (1.0 - e011 * norm) +
        vec3<f32>( 0.5, -0.5,  0.5) * (1.0 - e101 * norm) +
        vec3<f32>( 0.5,  0.5, -0.5) * (1.0 - e110 * norm) +
        vec3<f32>( 0.5,  0.5,  0.5) * (1.0 - e111 * norm);

    return probe + bias * ENVIRONMENT.settings.alpha;
}

fn octahedron(octant: vec3<u32>, subdivision: vec2<u32>, count: u32) -> vec3<f32> {
    var a = vec3<f32>(select(1.0, -1.0, octant.x != 0u), 0.0, 0.0);
    var b = vec3<f32>(0.0, select(1.0, -1.0, octant.y != 0u), 0.0);
    var c = vec3<f32>(0.0, 0.0, select(1.0, -1.0, octant.z != 0u));

    var half = count >> 1u;
    while (half > 0u) {
        let bit = countTrailingZeros(half);
        let tri = ((subdivision.x >> bit) & 1u) | (((subdivision.y >> bit) & 1u) << 1u);
        half >>= 1u;

        let mab = 0.5 * (a + b);
        let mbc = 0.5 * (b + c);
        let mca = 0.5 * (c + a);

        a = select(select(mab, a,   tri == 1u), mca, tri == 3u);
        b = select(select(mbc, mab, tri == 1u), b,   tri == 2u);
        c = select(select(mca, mbc, tri == 2u), c,   tri == 3u);
    }

    return normalize(a + b + c);
}

fn hdri(direction: vec3<f32>, N: u32) -> vec3<f32> {
    let hdri_dim = vec2<f32>(textureDimensions(HDRI));
    let lod = 0.5 * log2(hdri_dim.x * hdri_dim.y / f32(N));

    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world.xzy, HDRI_SETTINGS.rotation);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, lod).rgb;

    return radiance;
}
