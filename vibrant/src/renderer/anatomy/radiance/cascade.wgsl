@group(2) @binding( 0) var IRRADIANCE: texture_3d<f32>;
@group(2) @binding( 1) var CASCADE_SAMPLER: sampler;
@group(2) @binding( 2) var<uniform> CASCADE_INDEX: u32;

@group(2) @binding( 3) var CASCADE_IN_POS_X: texture_3d<f32>;
@group(2) @binding( 4) var CASCADE_IN_POS_Y: texture_3d<f32>;
@group(2) @binding( 5) var CASCADE_IN_POS_Z: texture_3d<f32>;
@group(2) @binding( 6) var CASCADE_IN_NEG_X: texture_3d<f32>;
@group(2) @binding( 7) var CASCADE_IN_NEG_Y: texture_3d<f32>;
@group(2) @binding( 8) var CASCADE_IN_NEG_Z: texture_3d<f32>;

@group(2) @binding( 9) var CASCADE_OUT_POS_X: texture_storage_3d<rgba32float, write>;
@group(2) @binding(10) var CASCADE_OUT_POS_Y: texture_storage_3d<rgba32float, write>;
@group(2) @binding(11) var CASCADE_OUT_POS_Z: texture_storage_3d<rgba32float, write>;
@group(2) @binding(12) var CASCADE_OUT_NEG_X: texture_storage_3d<rgba32float, write>;
@group(2) @binding(13) var CASCADE_OUT_NEG_Y: texture_storage_3d<rgba32float, write>;
@group(2) @binding(14) var CASCADE_OUT_NEG_Z: texture_storage_3d<rgba32float, write>;

var<private> SOURCE_DIM: vec3<u32>;
var<private> SOURCE_DIM_INV: vec3<f32>;

var<private> CASCADE_OUT_DIM: vec3<u32>;
var<private> CASCADE_IN_DIM: vec3<u32>;

var<private> DIRECTION_COUNT: u32;

const MAX_CASCADE: u32 = 5u;

const FRAME_POS_X: mat3x3<f32> = mat3x3<f32>(vec3<f32>( 1.0, 0.0, 0.0), vec3<f32>( 0.0, 1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0));
const FRAME_POS_Y: mat3x3<f32> = mat3x3<f32>(vec3<f32>( 0.0, 1.0, 0.0), vec3<f32>( 1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0));
const FRAME_POS_Z: mat3x3<f32> = mat3x3<f32>(vec3<f32>( 0.0, 0.0, 1.0), vec3<f32>( 1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0));
const FRAME_NEG_X: mat3x3<f32> = mat3x3<f32>(vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>( 0.0,-1.0, 0.0), vec3<f32>(0.0, 0.0,-1.0));
const FRAME_NEG_Y: mat3x3<f32> = mat3x3<f32>(vec3<f32>( 0.0,-1.0, 0.0), vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0,-1.0));
const FRAME_NEG_Z: mat3x3<f32> = mat3x3<f32>(vec3<f32>( 0.0, 0.0,-1.0), vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(0.0,-1.0, 0.0));

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) texel: vec3<u32>) {
    if (any(texel >= textureDimensions(CASCADE_OUT_POS_X))) { return; }

    SOURCE_DIM = textureDimensions(ABSORPTION);
    SOURCE_DIM_INV = 1.0 / vec3<f32>(SOURCE_DIM);

    CASCADE_OUT_DIM = textureDimensions(CASCADE_OUT_POS_X) >> vec3<u32>(CASCADE_INDEX, CASCADE_INDEX, 0u);
    CASCADE_IN_DIM = CASCADE_OUT_DIM >> vec3<u32>(1u);

    DIRECTION_COUNT = 2u << CASCADE_INDEX;

    let voxel = texel % CASCADE_OUT_DIM;
    let uv = (vec3<f32>(voxel) + 0.5) * f32(1u << CASCADE_INDEX) / vec3<f32>(SOURCE_DIM);
    let origin = uv * vec3<f32>(SOURCE_DIM);

    // Compute Ray intervals as Harmonic Series (and their cumulative sum)
    // let interval = f32(1 << (2 * CASCADE_INDEX)); // 1 4 16 64 128 256 ...
    // let t0 = (1.0 - interval) / -3.0; // 0 1 5 21 85 213 ...
    // let t1 = t0 + interval; // 1 5 21 85 213 ...

    // Compute Ray intervals as Harmonic Series (and their cumulative sum)
    let interval = f32(1u << (CASCADE_INDEX)); // 1 2 4 8 16 32 ...
    let t0 = interval - 1.0; // 0 1 3 7 15 31 ...
    let t1 = t0 + interval; // 1 3 7 15 31 63 ...

    let direction = 2u * (texel.xy / CASCADE_OUT_DIM.xy);

    textureStore(CASCADE_OUT_POS_X, texel, radiance(CASCADE_IN_POS_X, FRAME_POS_X, voxel, origin, direction, t0, t1));
    textureStore(CASCADE_OUT_NEG_X, texel, radiance(CASCADE_IN_NEG_X, FRAME_NEG_X, voxel, origin, direction, t0, t1));
    textureStore(CASCADE_OUT_POS_Y, texel, radiance(CASCADE_IN_POS_Y, FRAME_POS_Y, voxel, origin, direction, t0, t1));
    textureStore(CASCADE_OUT_NEG_Y, texel, radiance(CASCADE_IN_NEG_Y, FRAME_NEG_Y, voxel, origin, direction, t0, t1));
    textureStore(CASCADE_OUT_POS_Z, texel, radiance(CASCADE_IN_POS_Z, FRAME_POS_Z, voxel, origin, direction, t0, t1));
    textureStore(CASCADE_OUT_NEG_Z, texel, radiance(CASCADE_IN_NEG_Z, FRAME_NEG_Z, voxel, origin, direction, t0, t1));
}

fn radiance(
    cascade_in: texture_3d<f32>,
    frame: mat3x3<f32>,
    voxel: vec3<u32>,
    origin: vec3<f32>,
    direction_index: vec2<u32>,
    t0: f32,
    t1: f32) -> vec4<f32>
{
    var radiance = vec3<f32>(0.0, 0.0, 0.0);

    if (CASCADE_INDEX < MAX_CASCADE) {
        for (var i=0u; i<2u; i++) {
            for (var j=0u; j<2u; j++) {
                let index = direction_index + vec2<u32>(i,j);
                let da = index_to_direction(index, frame);
                let ri = radiance_interval(origin, da.xyz, t0, t1);
                let cascade = cascade_radiance(cascade_in, voxel, index);

                radiance += da.a * (ri.radiance + ri.transmission * cascade);
            }
        }
    } else if (CASCADE_INDEX == MAX_CASCADE) {
        radiance = vec3<f32>(1.0);
    }

    return vec4<f32>(radiance, 0.0);
}

fn cascade_radiance(cascade: texture_3d<f32>, voxel: vec3<u32>, direction: vec2<u32>) -> vec3<f32> {
    let origin = CASCADE_IN_DIM * vec3<u32>(direction, 0u);
    let position = vec3<f32>(origin) + 0.5 * (vec3<f32>(voxel) + 0.5);
    let sample = position / vec3<f32>(textureDimensions(cascade));
    return textureSampleLevel(cascade, CASCADE_SAMPLER, sample, 0.0).rgb;
}

fn index_to_direction(index: vec2<u32>, frame: mat3x3<f32>) -> vec4<f32> {
    let delta = 2.0 / f32(DIRECTION_COUNT);
    let face = -1.0 + (vec2<f32>(index) + 0.5) * vec2<f32>(delta);
    let area = (delta * delta) / pow(1.0 + dot(face, face), 1.5) * (4.0 * PI);

    let direction = normalize(frame[0] + frame[1] * face.x + frame[2] * face.y);

    return vec4<f32>(direction, area);
}

fn radiance_interval(origin: vec3<f32>, direction: vec3<f32>, t0: f32, t1: f32) -> RadianceInterval {
    let dim_inv = 1.0 / vec3<f32>(textureDimensions(EXTINCTION));
    let phase_function = 1.0 / (4.0 * PI);

    var transmission = vec3<f32>(1.0);
    var radiance = vec3<f32>(0.0);

    var outside = 0.0;

    for (var t=t0; t<t1; t+=STEP_SIZE) {
        let position_voxel_space = origin + direction * t;
        let sample = position_voxel_space * dim_inv;

        // let irradiance = textureSampleLevel(IRRADIANCE, SAMPLER, sample, 0.0).rgb;
        // let scattering = textureSampleLevel(SCATTERING, SAMPLER, sample, 0.0).rgb;

        // In-Scattering
        // radiance += irradiance * phase_function * scattering * transmission;

        let extinction = textureSampleLevel(EXTINCTION, SAMPLER, sample, 0.0).rgb;

        transmission *= exp(-STEP_SIZE * extinction); // TODO: should be in mm
    }

    return RadianceInterval(radiance, transmission);
}
