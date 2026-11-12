// Per-pixel noise, white or spatiotemporal blue per `ENVIRONMENT.noise`.
// Roots declare `ENVIRONMENT` and `BLUE_NOISE: texture_3d<f32>`.
use random;

const U24_TO_UNIT: f32 = 5.960464477539063e-8;

// Four independent [0, 1) values. Distinct `dimension`s are uncorrelated.
fn noise(pixel: vec2<u32>, dimension: u32) -> vec4<f32> {
    if (ENVIRONMENT.noise == 0u) {
        return white_noise(vec3<u32>(pixel, ENVIRONMENT.sample ^ (dimension << 24u)));
    }

    // One slice per sample. Each screen tile, dimension and pass through the
    // slices reads the texture shifted and in one of its 8 orientations, so the
    // tiles don't repeat; within a pass a pixel's sequence stays blue in time.
    let size = textureDimensions(BLUE_NOISE);
    let tile = pixel / size.xy;
    let hash = pcg3d(vec3<u32>(ENVIRONMENT.sample / size.z, dimension, tile.x | (tile.y << 16u)));

    let local = pixel % size.xy;
    let mirrored = select(local, size.xy - 1u - local, (vec2<u32>(hash.z) & vec2<u32>(1u, 2u)) != vec2<u32>(0u));
    let oriented = select(mirrored, mirrored.yx, (hash.z & 4u) != 0u);
    let texel = vec3<u32>((oriented + hash.xy) % size.xy, ENVIRONMENT.sample % size.z);

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
