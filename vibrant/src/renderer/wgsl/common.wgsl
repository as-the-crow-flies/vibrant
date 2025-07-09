struct Settings {
    streamline_radius: f32,
    direct_light: f32,
    culling_threshold: f32,
    alpha: f32,
    level: f32,
    layer: u32,
    skip: u32,
    quality: u32,
    smoothing: f32
}

struct Camera {
    transform: mat4x4<f32>,
    projection: mat4x4<f32>,
    projection_inverse: mat4x4<f32>,
    near: f32,
    far: f32,
}

struct Environment {
    surface: vec2<u32>,
    volume: u32,
    occlusion: u32,
    memory_: vec3<u32>,
    memory: u32,
    camera: Camera,
    light: vec3<f32>,
    light_: f32,
    settings: Settings
}

const U24_MAX: u32 = 16777215;
const U24_MAX_f32: f32 = f32(U24_MAX);
const U24_MAX_INV: f32 = 1.0 / f32(U24_MAX);

const U16_MAX: u32 = 65535;
const U16_MAX_f32: f32 = f32(U16_MAX);
const U16_MAX_INV: f32 = 1.0 / f32(U16_MAX);

const U14_SHIFT: u32 = 14;
const U14_MAX: u32 = 16383;

const U8_MAX: u32 = 255;
const U8_MAX_f32: f32 = f32(U8_MAX);
const U8_MAX_INV: f32 = 0.003921568627;

const BLOCK_BITS: u32 = 3u;
const BLOCK_SIZE: u32 = 8u;
const BLOCK_SIZE_2: u32 = BLOCK_SIZE * BLOCK_SIZE;
const BLOCK_SIZE_3: u32 = BLOCK_SIZE_2 * BLOCK_SIZE;

fn block_index(voxel: vec3<u32>, dim: vec3<u32>) -> u32 {
    // Number of blocks along each axis
    let blocks = dim >> vec3<u32>(BLOCK_BITS);

    // Block coordinate of this voxel
    let block_coord = voxel >> vec3<u32>(BLOCK_BITS);

    // Local coordinate within the block
    let local_coord = voxel & vec3<u32>(BLOCK_SIZE - 1);

    // Linear index of the block
    let block_index =
        block_coord.z * blocks.x * blocks.y +
        block_coord.y * blocks.x +
        block_coord.x;

    // Linear index within the block
    let local_index =
        local_coord.z * BLOCK_SIZE_2 +
        local_coord.y * BLOCK_SIZE +
        local_coord.x;

    // Total linear index
    return block_index * BLOCK_SIZE_3 + local_index;
}

fn transform(m: mat4x4<f32>, v: vec4<f32>) -> vec3<f32> {
    return (m * vec4<f32>(v.xyz, 1.0)).xyz;
}

fn div_ceil(a: u32, b: u32) -> u32 {
    return (a + b - 1) / b;
}

fn length2(a: vec3<f32>) -> f32 {
    return dot(a, a);
}

// Stalling et al. 1997 - Fast Display of Illuminated Field Lines
fn stalling(tangent: vec3<f32>, light: vec3<f32>) -> f32 {
    return max(0.0, sqrt(1.0 - pow(dot(light, tangent), 2.0)));
}

fn lambert(normal: vec3<f32>, light: vec3<f32>) -> f32 {
    return max(0.0, dot(normal, light));
}

// Constants
const GAMMA = 0.5;
const GAMMA_INV = 1.0 / GAMMA;

fn precision_encode(x: f32) -> f32 {
    return saturate(pow(saturate(x), GAMMA));
}

fn precision_decode(x: f32) -> f32 {
    return saturate(pow(saturate(x), GAMMA_INV));
}

var<workgroup> WORKGROUP_EXCLUSIVE_ADD: array<u32, 32>;
fn workgroupExclusiveAdd(value: u32, local: u32, subgroup: u32, subgroup_size: u32) -> u32 {
    let subgroup_id = local / subgroup_size;
    let subgroup_cumsum = subgroupExclusiveAdd(value);

    if (subgroup == subgroup_size - 1) {
        WORKGROUP_EXCLUSIVE_ADD[subgroup_id] = subgroup_cumsum + value;
    }

    workgroupBarrier();

    if (local < 32) {
        WORKGROUP_EXCLUSIVE_ADD[local] = subgroupExclusiveAdd(WORKGROUP_EXCLUSIVE_ADD[local]);
    }

    workgroupBarrier();

    return WORKGROUP_EXCLUSIVE_ADD[subgroup_id] + subgroup_cumsum;
}
