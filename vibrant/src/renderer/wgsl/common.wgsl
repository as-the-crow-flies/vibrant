struct Settings {
    radius: f32,
    lighting: f32,
    direct_light: f32,
    tangent_color: f32,
    shadows: f32,
    alpha: f32,
    level: f32,
    smoothing: f32,
    culling: u32,
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
    _memory: u32,
    camera: Camera,
    light: vec3<f32>,
    light_: f32,
    settings: Settings
}

const U32_MAX: u32 = 4294967295;
const U32_MAX_f32: f32 = f32(U32_MAX);
const U32_MAX_INV: f32 = 1.0 / U32_MAX_f32;

const U24_MAX: u32 = 16777215;
const U24_MAX_f32: f32 = f32(U24_MAX);
const U24_MAX_INV: f32 = 1.0 / U24_MAX_f32;

const U16_SHIFT: u32 = 16;
const U16_MAX: u32 = 65535;
const U16_MAX_f32: f32 = f32(U16_MAX);
const U16_MAX_INV: f32 = 1.0 / U16_MAX_f32;

const U14_SHIFT: u32 = 14;
const U14_MAX: u32 = 16383;
const U14_MAX_f32: f32 = f32(U14_MAX);
const U14_MAX_INV: f32 = 1.0 / U14_MAX_f32;

const U12_MAX: u32 = 1024;
const U12_MAX_f32: f32 = f32(U12_MAX);
const U12_MAX_INV: f32 = 1.0 / U12_MAX_f32;

const U8_MAX: u32 = 255;
const U8_MAX_f32: f32 = f32(U8_MAX);
const U8_MAX_INV: f32 = 1.0 / U8_MAX_f32;

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

fn pack_clip_alpha(clip_alpha: vec4<f32>) -> f32 {
    let clip = pack4x8snorm(vec4<f32>(clip_alpha.xyz, 0.0));
    let alpha = pack4x8unorm(vec4<f32>(vec3<f32>(0.0), clip_alpha.a));

    return bitcast<f32>(clip | alpha);
}

fn unpack_clip_alpha(f: f32) -> vec4<f32> {
    let u = bitcast<u32>(f);

    let clip = unpack4x8snorm(u);
    let alpha = unpack4x8unorm(u);

    return vec4<f32>(clip.xyz, alpha.a);
}

struct Vertex {
    xyz: vec3<f32>,
    clip: vec3<f32>,
    alpha: f32
}

fn unpack_vertex(v: vec4<f32>) -> Vertex {
    let clip_alpha = unpack_clip_alpha(v.a);
    return Vertex(v.xyz, clip_alpha.xyz, clip_alpha.a);
}

// https://iquilezles.org/articles/intersectors
// https://www.shadertoy.com/view/Xt3SzX
fn capsule_intersection(ro: vec3<f32>, rd: vec3<f32>, pa: vec3<f32>, pb: vec3<f32>, r: f32) -> f32
{
    let ba = pb - pa;
    let oa = ro - pa;

    let baba = dot(ba,ba);
    let bard = dot(ba,rd);
    let baoa = dot(ba,oa);
    let rdoa = dot(rd,oa);
    let oaoa = dot(oa,oa);

    var a = baba      - bard*bard;
    var b = baba*rdoa - baoa*bard;
    var c = baba*oaoa - baoa*baoa - r*r*baba;
    var h = b*b - a*c;

    if (h>=0.0) {
        let t = (-b - sqrt(h)) / a;
        let y = baoa + t*bard;

        // body
        if(y > 0.0 && y < baba) { return t; }

        // caps
        let oc = select(ro - pb, oa, y <= 0.0);

        b = dot(rd, oc);
        c = dot(oc, oc) - r*r;
        h = b*b - c;
        if (h > 0.0) { return -b - sqrt(h); }
    }

    return 1E6;
}

// https://iquilezles.org/articles/distfunctions/
fn capsule_sdf(p: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
  let pa = p - a;
  let ba = b - a;
  let h = saturate(dot(pa,ba) / dot(ba,ba));
  return length(pa - ba*h) - r;
}

// https://www.shadertoy.com/view/MlGczG
fn capsule_normal(pos: vec3<f32>, a: vec3<f32>, b: vec3<f32>, r: f32) -> vec3<f32> {
    let ba = b - a;
    let pa = pos - a;
    let h = saturate(dot(pa, ba) / dot(ba, ba));
    return (pa - h*ba) / r;
}

fn orthonormalize(normal: vec3<f32>, tangent: vec3<f32>) -> vec3<f32> {
    return normalize(normal - dot(normal, tangent) * tangent);
}

fn shade(
    v0: Vertex,
    v1: Vertex,
    radius: f32,
    position: vec3<f32>,
    direction: vec3<f32>,
    sample: vec3<f32>,
    environment: Environment,
    occlusion_ambient: texture_3d<f32>,
    occlusion_directional: texture_3d<f32>,
    occlusion_sampler: sampler
) -> vec4<f32> {
    let delta = v1.xyz - v0.xyz;
    let pa = position - v0.xyz;
    let height = saturate(dot(pa, delta) / dot(delta, delta));

    let is_start = all(v0.clip == vec3<f32>());
    let is_end = all(v1.clip == vec3<f32>());

    let delta_norm = normalize(delta);
    let tangent = normalize(mix(
        select(v0.clip, delta_norm, is_start),
        select(v1.clip, delta_norm, is_end),
        height
    ));

    let normal = (pa - height * delta) / radius;

    let use_original_normal = (is_start && height == 0.0) || (is_end && height == 1.0);
    let normal_smooth = select(orthonormalize(normal, tangent), normal, use_original_normal);
    let diffuse = lambert(normal_smooth, environment.light);

    let ambient = 1.0 - textureSampleLevel(occlusion_ambient, occlusion_sampler, sample, 0.0).x;
    let directional = 1.0 - textureSampleLevel(occlusion_directional, occlusion_sampler, sample, 0.0).x;
    // let shadow = anyhit(sample, environment.light);
    let shadow = 1.0;

    let factor = mix(1.0, mix(ambient, diffuse * min(directional, shadow),
        environment.settings.direct_light),
        environment.settings.lighting);

    let rgb = factor * mix(vec3<f32>(1.0), abs(tangent), environment.settings.tangent_color);
    let a = environment.settings.alpha * mix(v0.alpha, v1.alpha, height);

    return vec4<f32>(rgb, a);
}

fn should_be_clipped(v0: Vertex, v1: Vertex, position: vec3<f32>) -> bool {
    return dot(position - v0.xyz, v0.clip) < 0.0 || dot(v1.xyz - position, v1.clip) < 0.0;
}
