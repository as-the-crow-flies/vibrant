use material;

// One fraction per dispatch: flags every block (dilated by one) where a sample
// may classify to non-zero density. Material independent apart from the LUT.
@group(0) @binding(0) var<storage, read_write> OCCUPANCY: array<atomic<u32>>;
@group(0) @binding(1) var LUT: texture_2d_array<f32>;
// (physical volume size, fraction count)
@group(0) @binding(2) var<uniform> GRID: vec4<u32>;

@group(1) @binding(0) var FRACTION: texture_3d<f32>;
@group(1) @binding(3) var<uniform> MATERIAL: Material;
@group(1) @binding(4) var COLORMAP: texture_3d<f32>;

const BLOCK: u32 = 4u;

@compute
@workgroup_size(4, 4, 4)
fn main(@builtin(global_invocation_id) block: vec3<u32>) {
    let size = GRID.xyz;
    let blocks = (size + BLOCK - 1u) / BLOCK;
    if (any(block >= blocks)) { return; }

    // Every texel a cubic (or trilinear) sample inside this block can read.
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

    if (!is_dense(low, high)) { return; }

    let last = vec3<i32>(blocks) - 1;

    for (var dz = -1; dz <= 1; dz++) {
        for (var dy = -1; dy <= 1; dy++) {
            for (var dx = -1; dx <= 1; dx++) {
                let b = vec3<i32>(block) + vec3<i32>(dx, dy, dz);
                if (any(b < vec3<i32>(0)) || any(b > last)) { continue; }

                let index = u32(b.x) + blocks.x * (u32(b.y) + blocks.y * u32(b.z));
                atomicStore(&OCCUPANCY[index], 1u);
            }
        }
    }
}

// Whether the LUT density is non-zero anywhere in [low, high], after inversion
// (exact zeros stay zero, so a range touching 0 inverts to {0} ∪ [1 - high, 1]).
fn is_dense(low: f32, high: f32) -> bool {
    var lo = low;
    var hi = high;
    var zero = false;

    if (bool(MATERIAL.inverted)) {
        zero = low <= 0.0;
        lo = 1.0 - high;
        hi = select(1.0 - low, 1.0, zero);
    }

    if (zero && density(0u) > 0.0) { return true; }

    let scale = f32(textureDimensions(LUT).x - 1u);
    let first = u32(floor(saturate(lo) * scale));
    let last = u32(ceil(saturate(hi) * scale));

    for (var bin = first; bin <= last; bin++) {
        if (density(bin) > 0.0) { return true; }
    }

    return false;
}

fn density(bin: u32) -> f32 {
    return textureLoad(LUT, vec2<u32>(bin, 0u), 1, 0).w;
}
