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

struct Segment {
    position: vec3<f32>,
    radius: f32
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
    segment: Segment,
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

const U6_MAX: u32 = 63;
const U6_MAX_f32: f32 = f32(U6_MAX);
const U6_MAX_INV: f32 = 1.0 / U6_MAX_f32;

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

fn block_index_2d(voxel: vec2<u32>, dim: vec2<u32>, log2_block: u32) -> u32 {
    // block size = 2^log2_block
    let block_size: u32 = 1u << log2_block;
    let block_mask: u32 = block_size - 1u;

    // Which block this voxel belongs to (fast division by block_size)
    let block_id = voxel >> vec2<u32>(log2_block, log2_block);

    // Local offset inside the block (fast modulus by block_size)
    let local_id = voxel & vec2<u32>(block_mask, block_mask);

    // Number of blocks per dimension (ceil division)
    let blocks_per_dim = (dim + vec2<u32>(block_mask, block_mask)) >> vec2<u32>(log2_block, log2_block);

    // Flatten block index in row-major order
    let block_linear = block_id.y * blocks_per_dim.x + block_id.x;

    // Flatten local index inside the block
    let local_linear = (local_id.y << log2_block) + local_id.x;

    // Each block stores (block_size^2) elements = 1 << (2 * log2_block)
    return (block_linear << (log2_block * 2u)) + local_linear;
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

fn unpack_vertex_scale(v: vec4<f32>, scale: f32) -> Vertex {
    var vertex = unpack_vertex(v);
    vertex.xyz = (vertex.xyz + 0.5) * scale;
    return vertex;
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


// https://iquilezles.org/articles/intersectors
fn cylinder_intersection(ro: vec3<f32>, rd: vec3<f32>, cb: vec3<f32>, ca: vec3<f32>, cr: f32) -> f32 {
    let oc = ro - cb;
    let card = dot(ca,rd);
    let caoc = dot(ca,oc);
    let a = 1.0 - card*card;
    let b = dot( oc, rd) - caoc*card;
    let c = dot( oc, oc) - caoc*caoc - cr*cr;
    let h = b*b - a*c;

    return select(1E6, (-b - sqrt(h)) / a, h >= 0.0);
}

// https://iquilezles.org/articles/intersectors
fn sphere_intersection(ro: vec3<f32>, rd: vec3<f32>, ce: vec3<f32>, ra: f32) -> f32 {
    let oc = ro - ce;
    let b = dot(oc, rd);
    let c = dot(oc, oc) - ra*ra;
    let h = b*b - c;

    return select(1E6, -b - sqrt(h), h > 0.0);
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

    let normal = normalize((pa - height * delta) / radius);

    let use_original_normal = (is_start && height == 0.0) || (is_end && height == 1.0);
    let normal_smooth = select(orthonormalize(normal, tangent), normal, use_original_normal);
    let diffuse = lambert(normal_smooth, environment.light);

    let ambient = 1.0 - textureSampleLevel(occlusion_ambient, occlusion_sampler, position + 0.5, 0.0).x;
    let directional = 1.0 - textureSampleLevel(occlusion_directional, occlusion_sampler, position + 0.5, 0.0).x;

    let factor = mix(1.0, mix(ambient, diffuse * directional,
        environment.settings.direct_light),
        environment.settings.lighting);

    let rgb = factor * mix(vec3<f32>(1.0), abs(tangent).xzy, environment.settings.tangent_color);
    let a = environment.settings.alpha * mix(v0.alpha, v1.alpha, height);

    return vec4<f32>(rgb, a);
}

fn should_be_clipped(v0: Vertex, v1: Vertex, position: vec3<f32>) -> bool {
    return dot(position - v0.xyz, v0.clip) < 0.0 || dot(v1.xyz - position, v1.clip) < 0.0;
}

fn morton_encode(p: vec3<u32>) -> u32 {
    let xx = expand_bits(p.x);
    let yy = expand_bits(p.y) << 1u;
    let zz = expand_bits(p.z) << 2u;
    return xx | yy | zz;
}

fn morton_decode(code: u32) -> vec3<u32> {
    let x = compact_bits(code);
    let y = compact_bits(code >> 1u);
    let z = compact_bits(code >> 2u);
    return vec3<u32>(x, y, z);
}

const MORTON_MAGIC_BITS: array<u32, 5> = array<u32, 5>(0x000003ff, 0x30000ff, 0x0300f00f, 0x30c30c3, 0x9249249);

fn expand_bits(v: u32) -> u32 {
    var x = v & MORTON_MAGIC_BITS[0];
    x = (x | (x << 16u)) & MORTON_MAGIC_BITS[1];
    x = (x | (x << 8u))  & MORTON_MAGIC_BITS[2];
    x = (x | (x << 4u))  & MORTON_MAGIC_BITS[3];
    x = (x | (x << 2u))  & MORTON_MAGIC_BITS[4];
    return x;
}

fn compact_bits(v: u32) -> u32 {
    var x = v & MORTON_MAGIC_BITS[4];
    x = (x ^ (x >> 2u))  & MORTON_MAGIC_BITS[3];
    x = (x ^ (x >> 4u))  & MORTON_MAGIC_BITS[2];
    x = (x ^ (x >> 8u))  & MORTON_MAGIC_BITS[1];
    x = (x ^ (x >> 16u)) & MORTON_MAGIC_BITS[0];
    return x;
}

struct VoxelSegment {
    v0: Vertex,
    v1: Vertex
}

fn encode_segment_vertex(voxel: vec3<u32>, vertex: vec3<f32>, axis: u32, clip: vec3<f32>) -> u32 {
    let local = saturate(vertex - vec3<f32>(voxel));

    let face_bits = (1u << (axis + 1u)) + u32(local[axis] > 0.5);

    let axes = select(select(
        vec2<u32>(0, 1),
        vec2<u32>(0, 2), axis == 1),
        vec2<u32>(1, 2), axis == 0);

    let axis_0_bits = encode_segment_axis(local[axes[0]]);
    let axis_1_bits = encode_segment_axis(local[axes[1]]);

    let vertex_quantized = (face_bits << 12u) | (axis_0_bits << 6u) | axis_1_bits;

    return (vertex_quantized << 16) | pack_normal(clip);
}

fn decode_segment_vertex(voxel: vec3<u32>, vertex_clip: u32, scale: f32) -> Vertex {
    let vertex = vertex_clip >> 16u;

    let face_bits = vertex >> 12u;

    let axis_0 = decode_vertex_axis(vertex >> 6u);
    let axis_1 = decode_vertex_axis(vertex);
    let axis_2 = f32(face_bits & 1u);

    let direction_0 = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0), bool((face_bits >> 1u) & 1u));
    let direction_1 = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 1.0, 0.0), bool((face_bits >> 3u) & 1u));
    let direction_2 = vec3<f32>(1.0) - direction_0 - direction_1;

    let xyz = vec3<f32>(voxel) +
        direction_0 * axis_0 +
        direction_1 * axis_1 +
        direction_2 * axis_2;

    let clip = unpack_normal(vertex_clip & U16_MAX);

    return Vertex(xyz * scale - 0.5, clip, 1.0);
}

fn encode_segment_axis(local: f32) -> u32 {
    return clamp(u32(local * U6_MAX_f32), 0u, U6_MAX);
}

fn decode_vertex_axis(axis: u32) -> f32 {
    return f32(axis & U6_MAX) * U6_MAX_INV;
}

const INSERTION_SORT_SIZE: u32 = 32;

fn insertion_sort_insert(hits: ptr<function, array<u32, INSERTION_SORT_SIZE>>, hit_count: u32, value: u32) {
    let hit_count_clamped = min(hit_count, INSERTION_SORT_SIZE);
    let idx = insertion_sort_insertion_index(hits, hit_count_clamped, value);

    for (var i = hit_count_clamped; i > idx; i--) {
        (*hits)[i] = (*hits)[i - 1u];
    }

    (*hits)[idx] = value;
}

fn insertion_sort_insertion_index(hits: ptr<function, array<u32, INSERTION_SORT_SIZE>>, hit_count: u32, value: u32) -> u32 {
    var lo: u32 = 0u;
    var hi: u32 = hit_count;

    while (lo < hi) {
        let mid: u32 = (lo + hi) / 2u;
        if ((*hits)[mid] < value) {
            lo = mid + 1u;
        } else {
            hi = mid;
        }
    }

    return lo;
}

fn linear_to_srgb(c: f32) -> f32 {
    return select(
        c * 12.92,
        1.055 * pow(c, 1.0 / 2.4) - 0.055,
        c > 0.0031308
    );
}

fn linear_to_srgb_vec3(c: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        linear_to_srgb(c.r),
        linear_to_srgb(c.g),
        linear_to_srgb(c.b)
    );
}

fn linear_to_srgb_rgba(c: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(linear_to_srgb_vec3(c.rgb), c.a);
}

fn pack_normal(n: vec3<f32>) -> u32 {
    if (all(n == vec3<f32>())) { return 0u; }

    // project the normal onto the octahedron
    var p = n.xy / (abs(n.x) + abs(n.y) + abs(n.z));
    if (n.z < 0.0) {
        p = (1.0 - abs(p.yx)) * sign(p);
    }

    // remap from [-1,1] to [0,1] and quantize to 0–255
    let enc = clamp(p * 0.5 + 0.5, vec2<f32>(0.0), vec2<f32>(1.0));
    let xi = u32(round(enc.x * 255.0));
    let yi = u32(round(enc.y * 255.0));

    // pack into a single 16-bit value (low byte = x, high byte = y)
    return (yi << 8u) | xi;
}

fn unpack_normal(packed: u32) -> vec3<f32> {
    if (packed == 0u) { return vec3<f32>(); }

    // extract bytes and remap to [-1,1]
    let x = f32(packed & 0xFFu) / 255.0 * 2.0 - 1.0;
    let y = f32((packed >> 8u) & 0xFFu) / 255.0 * 2.0 - 1.0;
    var v = vec3<f32>(x, y, 1.0 - abs(x) - abs(y));

    // fold back into the sphere if we were in the lower hemisphere
    if (v.z < 0.0) {
        v = vec3<f32>((1.0 - abs(v.yx)) * sign(v.xy), v.z);
    }

    return normalize(v);
}
