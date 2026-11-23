// Per-pixel noise, white or spatiotemporal blue per `ENVIRONMENT.noise`.
// Roots declare `ENVIRONMENT` and `BLUE_NOISE: texture_3d<f32>`.
use random;

// Must match `BlueNoise`
const BLUE_NOISE_SIZE: u32 = 22u;
const BLUE_NOISE_DEPTH: u32 = 32u;
const BLUE_NOISE_VARIANTS: u32 = 4u;
const U24_TO_UNIT: f32 = 5.960464477539063e-8;

// Four independent [0, 1) values. Distinct `dimension`s are uncorrelated.
fn noise(pixel: vec2<u32>, dimension: u32) -> vec4<f32> {
    if (ENVIRONMENT.noise == 0u) {
        return white_noise(vec3<u32>(pixel, ENVIRONMENT.sample ^ (dimension << 24u)));
    }

    // One slice per sample. The screen is a checkerboard of cells, each showing
    // one of its colour's variants by hash; the variants are baked to stay blue
    // across cell edges (blue_noise.py). Each dimension and pass through the
    // slices shifts the board and rehashes it.
    let hash = pcg3d(vec3<u32>(ENVIRONMENT.sample / BLUE_NOISE_DEPTH, dimension, 0u));

    let p = pixel + (hash.xy & vec2<u32>(0xffffu));
    let cell = p / BLUE_NOISE_SIZE;
    let colour = (cell.x + cell.y) & 1u;
    let variant = pcg3d(vec3<u32>(cell, hash.z)).x % BLUE_NOISE_VARIANTS;
    let layer = (colour * BLUE_NOISE_VARIANTS + variant) * BLUE_NOISE_DEPTH + ENVIRONMENT.sample % BLUE_NOISE_DEPTH;
    let texel = vec3<u32>(p % BLUE_NOISE_SIZE, layer);

    return (textureLoad(BLUE_NOISE, texel, 0) * 255.0 + 0.5) / 256.0;
}

fn white_noise(seed: vec3<u32>) -> vec4<f32> {
    let a = pcg3d(seed);
    let b = pcg3d(a);
    // Top 24 bits, so the float never rounds up to 1.
    return vec4<f32>(vec4<u32>(a, b.x) >> vec4<u32>(8u)) * U24_TO_UNIT;
}

// Uniform [0, 1) → tent on [-1, 1] by inverse CDF.
fn tent(u: vec3<f32>) -> vec3<f32> {
    let lower = sqrt(2.0 * u) - 1.0;
    let upper = 1.0 - sqrt(2.0 * (1.0 - u));
    return select(upper, lower, u < vec3<f32>(0.5));
}
