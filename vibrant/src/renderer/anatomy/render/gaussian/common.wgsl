const VMM_SIZE   : u32 = 16u;
const INTERVAL = array<f32, 7>(1.0, 3.0, 7.0, 15.0, 31.0, 63.0, 127.0);

fn vmf(v: vec4<f32>, omega: mat4x3<f32>) -> vec4<f32> {
    let q  = inverseSqrt(dot(v.xyz, v.xyz) + 1e-24);   // 1/|s|
    let u  = clamp((v.w + 1e-8) * q, U_MIN, U_MAX);    // w/|s| ∈ [1, ∞)
    let u2 = u * u;

    let k  = (3.0 * u2 - 1.0) / (u2 * u - u);          // Banerjee, reparameterized
    let kl = k * LOG2_E;

    let e    = exp2(fma(vec4<f32>(kl * q), v.xyz * omega, vec4<f32>(-kl))); // e^{κ(μ·ωᵢ − 1)}
    let norm = INV_TWO_PI * k / (1.0 - exp2(-2.0 * kl));
    return norm * e;
}

fn hdri(direction: vec3<f32>, N: u32) -> vec3<f32> {
    let hdri_dim = vec2<f32>(textureDimensions(HDRI));
    let lod = 0.5 * log2(hdri_dim.x * hdri_dim.y / f32(N));

    let direction_world = normalize((TRANSFORM_INVERSE * vec4<f32>(direction, 0.0)).xyz);
    let sample = equirectangular(direction_world.xzy, HDRI_SETTINGS.rotation);
    let radiance = HDRI_SETTINGS.strength * textureSampleLevel(HDRI, HDRI_SAMPLER, sample, lod).rgb;

    return radiance;
}

fn grid(dim: vec3<u32>) -> vec3<u32> {
    return max(dim / vec3<u32>(4, 4, 2), vec3<u32>(1));
}

fn lobe(k: u32) -> vec3<u32> {
    return vec3<u32>(k & 3, (k >> 2) & 3, (k >> 4) & 1);
}

// Lays samples out in 2x2 blocks instead of pure Hammersley: block = i/4 is
// placed via Hammersley (still low-discrepancy across blocks), and sub = i%4
// picks one of 4 tightly-packed sub-positions within that block's cell. Callers
// that request 4 consecutive i's (as cascade.wgsl's main() does per thread) get
// back 4 directions that are close together instead of scattered across the
// sphere -- needed wherever those 4 are later averaged into one ray.
fn get_direction(i: u32, N: u32) -> vec3<f32> {
    let block = i / 4u;
    let sub = i % 4u;
    let blocks = N / 4u;

    let base = hammersley(block, blocks);
    let cell = 1.0 / f32(blocks);
    let sub_offset = vec2<f32>(f32(sub & 1u), f32(sub >> 1u)) - 0.5;

    let uv = base + sub_offset * cell * 0.5;
    return octahedron_decode(2.0 * uv - 1.0);
}

fn max_norm(v: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(v.rgb / max(v.w, EPSILON), 1.0);
}

fn sum(v: vec4<f32>) -> f32 {
    return v.x + v.y + v.z + v.w;
}
