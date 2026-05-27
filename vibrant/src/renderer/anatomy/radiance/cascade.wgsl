@group(0) @binding(2) var EXTINCTION: texture_3d<f32>;
@group(0) @binding(3) var GRADIENT: texture_3d<f32>;
@group(0) @binding(4) var SAMPLER: sampler;
@group(0) @binding(5) var<uniform> TRANSFORM: mat4x4<f32>;
@group(0) @binding(6) var<uniform> TRANSFORM_INVERSE: mat4x4<f32>;

@group(1) @binding(0) var<uniform> ENVIRONMENT: Environment;

@group(2) @binding(0) var CASCADE_OUT_RADIANCE: texture_storage_3d<rgba8unorm, write>;
@group(2) @binding(1) var CASCADE_OUT_TRANSMISSION: texture_storage_3d<rgba8unorm, write>;
@group(2) @binding(2) var CASCADE_IN: texture_3d<f32>;
@group(2) @binding(3) var CASCADE_SAMPLER: sampler;
@group(2) @binding(4) var<uniform> CASCADE_INDEX: u32;

@group(3) @binding(0) var HDRI: texture_2d<f32>;
@group(3) @binding(1) var HDRI_SAMPLER: sampler;
@group(3) @binding(2) var<uniform> HDRI_SETTINGS: HdriSettings;

const MAX_CASCADE: u32 = 5u;
const STEP_SIZE: f32 = 0.5;

var<private> SOURCE_DIM: vec3<u32>;
var<private> SOURCE_DIM_INV: vec3<f32>;

var<private> CASCADE_OUT_DIM: vec3<u32>;
var<private> CASCADE_IN_DIM: vec3<u32>;
var<private> CASCADE_OUT_DIRECTION_DIM: vec3<u32>;
var<private> CASCADE_IN_DIRECTION_DIM: vec3<u32>;

var<private> DIRECTION_COUNT: u32;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) texel: vec3<u32>) {
    CASCADE_OUT_DIM = textureDimensions(CASCADE_OUT_RADIANCE) / vec3<u32>(1, 1, 6);
    CASCADE_IN_DIM = textureDimensions(CASCADE_IN) / vec3<u32>(1, 1, 6);

    if (any(texel >= CASCADE_OUT_DIM)) { return; }

    SOURCE_DIM = textureDimensions(EXTINCTION);
    SOURCE_DIM_INV = 1.0 / vec3<f32>(SOURCE_DIM);

    CASCADE_OUT_DIRECTION_DIM = CASCADE_OUT_DIM >> vec3<u32>(CASCADE_INDEX, CASCADE_INDEX, 0u);
    CASCADE_IN_DIRECTION_DIM = CASCADE_OUT_DIRECTION_DIM >> vec3<u32>(1u);

    DIRECTION_COUNT = 1u << CASCADE_INDEX;

    let voxel = texel % CASCADE_OUT_DIRECTION_DIM;
    let uv = (vec3<f32>(voxel) + 0.5) * f32(2u << CASCADE_INDEX) / vec3<f32>(SOURCE_DIM);
    let origin = uv * vec3<f32>(SOURCE_DIM);

    // Compute Ray intervals as Harmonic Series (and their cumulative sum)
    let interval = f32(1u << (CASCADE_INDEX)); // 1 2 4 8 16 32 ...
    let t0 = interval - 1.0; // 0 1 3 7 15 31 ...
    let t1 = t0 + interval; // 1 3 7 15 31 63 ...

    let direction = texel.xy / CASCADE_OUT_DIRECTION_DIM.xy;

    for (var face=0u; face < 6u; face++) {
        let offset = texel + vec3<u32>(0, 0, face * CASCADE_OUT_DIM.z);

        let result = radiance(voxel, origin, face, direction, t0, t1);
        textureStore(CASCADE_OUT_RADIANCE, offset, vec4<f32>(result.radiance, 0.0));
        textureStore(CASCADE_OUT_TRANSMISSION, offset, vec4<f32>(result.transmission, 0.0));
    }
}

struct RadianceInterval {
    radiance: vec3<f32>,
    transmission: vec3<f32>
}

fn radiance(
    voxel: vec3<u32>,
    origin: vec3<f32>,
    face: u32,
    direction_index: vec2<u32>,
    t0: f32,
    t1: f32) -> RadianceInterval
{
    let delta = 1.0 / f32(DIRECTION_COUNT);
    let uv = (vec2<f32>(direction_index) + 0.5) * delta;
    let direction = cubemap_decode(CubeCoordinates(face, uv));

    if (CASCADE_INDEX < MAX_CASCADE) {
        let ri = radiance_interval(origin, direction, t0, t1);
        let cascade =
            cascade_radiance(voxel, face, 2 * direction_index + vec2<u32>(0, 0)) +
            cascade_radiance(voxel, face, 2 * direction_index + vec2<u32>(0, 1)) +
            cascade_radiance(voxel, face, 2 * direction_index + vec2<u32>(1, 0)) +
            cascade_radiance(voxel, face, 2 * direction_index + vec2<u32>(1, 1));

        let radiance = (ri.radiance + ri.transmission * 0.25 * cascade);
        let transmission = ri.transmission;

        return RadianceInterval(radiance, transmission);
    }

    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world, HDRI_SETTINGS.rotation);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, 0.0).rgb;

    return RadianceInterval(radiance, vec3<f32>(1.0, 1.0, 1.0));
}

fn cascade_radiance(voxel: vec3<u32>, face: u32, direction: vec2<u32>) -> vec3<f32> {
    let origin = vec3<u32>(
        direction * CASCADE_IN_DIRECTION_DIM.xy,
        face * CASCADE_IN_DIM.z
    );

    let position = vec3<f32>(origin) + 0.5 * (vec3<f32>(voxel) + 0.5);
    let sample = position / vec3<f32>(textureDimensions(CASCADE_IN));
    return textureSampleLevel(CASCADE_IN, CASCADE_SAMPLER, sample, 0.0).rgb;
}

fn radiance_interval(origin: vec3<f32>, direction: vec3<f32>, t0: f32, t1: f32) -> RadianceInterval {
    let dim_inv = 1.0 / vec3<f32>(textureDimensions(EXTINCTION));
    let phase_function = 1.0 / (4.0 * PI);

    var transmission = vec3<f32>(1.0);
    var radiance = vec3<f32>(0.0);

    for (var t=t0; t<t1; t+=STEP_SIZE) {
        let position_voxel_space = origin + direction * t;
        let sample = position_voxel_space * dim_inv;

        if (any(sample < vec3<f32>(0.0)) || any(sample > vec3<f32>(1.0))) { break; }

        let extinction = unpack_rgb(textureSampleLevel(EXTINCTION, SAMPLER, sample, 0.0));

        transmission *= exp(-STEP_SIZE * extinction); // TODO: should be in mm
    }

    return RadianceInterval(radiance, transmission);
}
