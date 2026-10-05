const VMM_SIZE   : u32 = #VMM_SIZE;
const INTERVAL = array<f32, 7>(1.0, 3.0, 7.0, 15.0, 31.0, 63.0, 127.0);

const EM_ITERATIONS_MAX: u32 = 100u;
const EM_ITERATIONS_MIN: u32 = 1u;
const EM_CONVERGENCE: f32 = 0.05;

fn vmf(v: vec4<f32>, omega: Directions) -> Weights {
    let q  = inverseSqrt(dot(v.xyz, v.xyz) + 1e-24);   // 1/|s|
    let u  = clamp((v.w + 1e-8) * q, U_MIN, U_MAX);    // w/|s| ∈ [1, ∞)
    let u2 = u * u;

    let k  = (3.0 * u2 - 1.0) / (u2 * u - u);          // Banerjee, reparameterized
    let kl = k * LOG2_E;

    let e    = exp2(fma(Weights(kl * q), project(v.xyz, omega), Weights(-kl))); // e^{κ(μ·ωᵢ − 1)}
    let norm = INV_TWO_PI * k / (1.0 - exp2(-2.0 * kl));
    return norm * e;
}

fn grid(dim: vec3<u32>) -> vec3<u32> {
    return max(dim / vec3<u32>(4, 4, 2), vec3<u32>(1));
}

fn lobe(k: u32) -> vec3<u32> {
    return vec3<u32>(k & 3, (k >> 2) & 3, (k >> 4) & 1);
}

fn get_direction(i: u32, N: u32) -> vec3<f32> {
    return octahedron_decode(2.0 * hammersley(i, N) - 1.0);
}

fn max_norm(v: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(v.rgb / max(v.w, EPSILON), 1.0);
}

fn radiance_weight(radiance: vec3<f32>) -> f32 {
    return log(1.0 + length(radiance));
}
