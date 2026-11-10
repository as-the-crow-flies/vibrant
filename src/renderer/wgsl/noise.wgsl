// Per-pixel noise, white or spatiotemporal blue per `ENVIRONMENT.noise`.
// Roots declare `ENVIRONMENT` and `BLUE_NOISE: texture_3d<f32>`.
use random;

// R4 sequence (Roberts): per-step rotation, low discrepancy along the ray.
const R4 = vec4<f32>(0.8566748838545029, 0.733891856627126, 0.6287067210378086, 0.53859725722361);

const U24_TO_UNIT: f32 = 5.960464477539063e-8;

// Four independent [0, 1) values. Distinct `dimension`s are uncorrelated.
fn noise(pixel: vec2<u32>, dimension: u32) -> vec4<f32> {
    if (ENVIRONMENT.noise == 0u) {
        return white_noise(vec3<u32>(pixel, ENVIRONMENT.sample ^ (dimension << 24u)));
    }

    // One slice per sample; each dimension and each pass through the slices
    // reads a randomly shifted tile.
    let size = textureDimensions(BLUE_NOISE);
    let shift = pcg3d(vec3<u32>(ENVIRONMENT.sample / size.z, dimension, 0x5bd1e995u)).xy;
    let texel = vec3<u32>((pixel + shift) % size.xy, ENVIRONMENT.sample % size.z);

    return (textureLoad(BLUE_NOISE, texel, 0) * 255.0 + 0.5) / 256.0;
}

// `u` for ray step `step`: rotated (blue) or redrawn (white).
fn advance(u: vec4<f32>, step: u32) -> vec4<f32> {
    if (ENVIRONMENT.noise == 0u) {
        return white_noise(bitcast<vec3<u32>>(u.xyz) ^ vec3<u32>(step));
    }

    return fract(u + R4 * f32(step));
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
