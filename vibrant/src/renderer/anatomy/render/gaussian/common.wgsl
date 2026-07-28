const VMM_SIZE   : u32 = 8u;
const EPSILON    : f32 = 1e-6;
const INV_TWO_PI : f32 = 0.15915494309189535;
const LOG2_E     : f32 = 1.4426950408889634;
const U_MIN      : f32 = 1.001;    // κ_max ≈ 1000
const U_MAX      : f32 = 100.0;    // κ_min ≈ 0.03 — indistinguishable from uniform

fn vmf(v: vec4<f32>, omega: vec3<f32>) -> f32 {
    let q  = inverseSqrt(dot(v.xyz, v.xyz) + 1e-24);   // 1/|s|
    let u  = clamp((v.w + 1e-8) * q, U_MIN, U_MAX);    // w/|s| ∈ [1, ∞)
    let u2 = u * u;

    let k  = (3.0 * u2 - 1.0) / (u2 * u - u);          // Banerjee, reparameterized
    let kl = k * LOG2_E;

    let e = exp2(fma(kl * q, dot(v.xyz, omega), -kl)); // e^{κ(μ·ω − 1)}
    return INV_TWO_PI * k * e / (1.0 - exp2(-2.0 * kl));
}

// Fitted analytic approximation of ∫_hemisphere(N) L(ω)⟨N·ω⟩dω for a single
// SG/vMF lobe L(ω) = phi·exp(κ·(μ·ω − 1)), where `v` = (Σwᵢωᵢ, Σwᵢ) is the same
// sufficient statistic consumed by vmf() — κ and μ are recovered from it directly
// instead of requiring the caller to normalize/derive them up front.
// See: https://therealmjp.github.io/posts/sg-series-part-3-diffuse-lighting-from-an-sg-light-source/
//
// The published fit diverges for κ ≲ 1 (it targets tight specular-like lobes), so the
// lobe shape is evaluated at a clamped sharpness and blended towards the exact
// isotropic-lobe irradiance (π·phi) as κ → 0. Near-isotropic/degenerate statistics
// (v.xyz ≈ 0) fall out of the same κ-clamp used by vmf(), so no extra branch is needed.
fn sg_irradiance_fitted(phi: vec3<f32>, v: vec4<f32>, normal: vec3<f32>) -> vec3<f32> {
    let q  = inverseSqrt(dot(v.xyz, v.xyz) + 1e-24);   // 1/|s|
    let u  = clamp((v.w + 1e-8) * q, U_MIN, U_MAX);    // w/|s| ∈ [1, ∞)
    let u2 = u * u;

    let kappa  = (3.0 * u2 - 1.0) / (u2 * u - u);      // Banerjee, reparameterized
    let lambda = max(kappa, 1.0);

    let mu_dot_n = clamp(dot(v.xyz, normal) * q, -1.0, 1.0);

    let c0 = 0.36;
    let c1 = 1.0 / (4.0 * c0);

    let eml  = exp2(-lambda * LOG2_E);
    let em2l = eml * eml;
    let rl   = 1.0 / lambda;

    let scale = 1.0 + 2.0 * em2l - rl;
    let bias  = (eml - em2l) * rl - em2l;

    let x  = sqrt(max(1.0 - scale, 0.0));
    let x0 = c0 * mu_dot_n;
    let x1 = c1 * x;

    let n = x0 + x1;

    var y = clamp(mu_dot_n, 0.0, 1.0);
    if (abs(x0) <= x1) {
        y = n * n / max(x, EPSILON);
    }

    let result = scale * y + bias;

    // Exact SG integral over the sphere is 2π·phi·(1 − e^{−2λ})/λ; em2l is already
    // available from scale/bias above, so this reuses it instead of a third exp call.
    let fitted = result * (2.0 * PI * phi * (1.0 - em2l) / lambda);

    return mix(PI * phi, fitted, clamp(kappa, 0.0, 1.0));
}
