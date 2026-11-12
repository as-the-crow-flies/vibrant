use material;

// One fraction per dispatch: adds the fraction's largest LUT absorption and
// scattering, over every channel and every value a sample inside the block can
// read, to the block's majorant. Masks and crop only attenuate, so the sum
// bounds the classified extinction. Non-empty blocks flag their coarse cell.
@group(0) @binding(0) var<storage, read_write> MAJORANT: array<vec2<f32>>;
@group(0) @binding(1) var LUT: texture_2d_array<f32>;
// (physical volume size, fraction count)
@group(0) @binding(2) var<uniform> GRID: vec4<u32>;
@group(0) @binding(3) var<storage, read_write> COARSE_CELLS: array<atomic<u32>>;

@group(1) @binding(0) var FRACTION: texture_3d<f32>;
@group(1) @binding(3) var<uniform> MATERIAL: Material;
@group(1) @binding(4) var COLORMAP: texture_3d<f32>;

const BLOCK: u32 = 4u;
// Blocks per coarse cell, per axis.
const COARSE: u32 = 8u;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) block: vec3<u32>) {
    let size = GRID.xyz;
    let blocks = (size + BLOCK - 1u) / BLOCK;
    if (any(block >= blocks)) { return; }

    // Every texel a cubic (or trilinear) sample inside this block can read.
    // B-spline weights are non-negative, so samples stay within their range.
    let texels = vec3<f32>(textureDimensions(FRACTION));
    let lo_uv = vec3<f32>(block * BLOCK) / vec3<f32>(size);
    let hi_uv = vec3<f32>(min((block + 1u) * BLOCK, size)) / vec3<f32>(size);
    let lo = vec3<i32>(clamp(floor(lo_uv * texels - 0.5) - 1.0, vec3<f32>(0.0), texels - 1.0));
    let hi = vec3<i32>(clamp(floor(hi_uv * texels - 0.5) + 2.0, vec3<f32>(0.0), texels - 1.0));

    var low = 1.0;
    var high = 0.0;

    for (var z = lo.z; z <= hi.z; z++) {
        for (var y = lo.y; y <= hi.y; y++) {
            for (var x = lo.x; x <= hi.x; x++) {
                let f = textureLoad(FRACTION, vec3<i32>(x, y, z), 0).x;
                low = min(low, f);
                high = max(high, f);
            }
        }
    }

    let bound = majorant(low, high);
    if (all(bound == vec2<f32>(0.0))) { return; }

    // Dispatches run in order and each block has one invocation, so the sum
    // needs no atomics; the coarse flag is shared by many blocks.
    let index = block.x + blocks.x * (block.y + blocks.y * block.z);
    MAJORANT[index] += bound;

    let cells = (blocks + COARSE - 1u) / COARSE;
    let cell = block / COARSE;
    atomicStore(&COARSE_CELLS[cell.x + cells.x * (cell.y + cells.y * cell.z)], 1u);
}

// Largest (absorption, scattering) the LUT holds for fractions in [low, high],
// after inversion (exact zeros stay zero, so a range touching 0 inverts to
// {0} ∪ [1 - high, 1]). Linear LUT filtering stays within the bins either side.
fn majorant(low: f32, high: f32) -> vec2<f32> {
    var lo = low;
    var hi = high;
    var result = vec2<f32>(0.0);

    if (bool(MATERIAL.inverted)) {
        if (low <= 0.0) { result = bin(0u); }
        lo = 1.0 - high;
        hi = 1.0 - low;
    }

    let scale = f32(textureDimensions(LUT).x - 1u);
    let first = u32(floor(saturate(lo) * scale));
    let last = u32(ceil(saturate(hi) * scale));

    for (var b = first; b <= last; b++) { result = max(result, bin(b)); }

    return result;
}

fn bin(b: u32) -> vec2<f32> {
    let absorption = textureLoad(LUT, vec2<u32>(b, 0u), 0, 0).rgb;
    let scattering = textureLoad(LUT, vec2<u32>(b, 0u), 1, 0).rgb;
    return vec2<f32>(max_channel(absorption), max_channel(scattering));
}

fn max_channel(v: vec3<f32>) -> f32 {
    return max(v.x, max(v.y, v.z));
}
