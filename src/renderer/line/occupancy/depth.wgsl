use math;

// Packed per-voxel u32: segment count in the low 16 bits, occupancy depth
// d = -ln(1 - c) in the high 16 bits as 5.11 fixed point. Summing depths
// combines coverages as 1 - Π(1 - c), so overlapping tubes aren't double-counted.
const DEPTH_SCALE: f32 = 2048.0;
const DEPTH_SCALE_INV: f32 = 1.0 / DEPTH_SCALE;

// Voxels stop taking depth at 1 - e^-8 coverage; per-segment terms are capped
// at 4, so racing adds stay well below the 32 range and never wrap.
const DEPTH_SATURATED: u32 = u32(8.0 * DEPTH_SCALE) << U16_SHIFT;
const COVERAGE_MAX: f32 = 0.98168436; // 1 - e^-4

// Dithered rounding keeps faint tube edges unbiased.
fn encode_depth(coverage: f32, dither: f32) -> u32 {
    let depth = -log(1.0 - clamp(coverage, 0.0, COVERAGE_MAX));
    return u32(depth * DEPTH_SCALE + dither) << U16_SHIFT;
}

fn decode_fraction(encoded: u32) -> f32 {
    return 1.0 - exp(-DEPTH_SCALE_INV * f32(encoded >> U16_SHIFT));
}

fn dither(a: u32, b: u32) -> f32 {
    return f32(pcg(a ^ pcg(b)) >> 8u) * (1.0 / 16777216.0);
}

fn pcg(v: u32) -> u32 {
    let state = v * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}
